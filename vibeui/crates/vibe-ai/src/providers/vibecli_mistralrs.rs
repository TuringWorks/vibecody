//! VibeCLI in-process mistralrs provider.
//!
//! Speaks the Ollama HTTP wire format against the local vibecli daemon
//! (default `http://localhost:7878`) but pins the backend to mistralrs by
//! sending `X-VibeCLI-Backend: mistralrs` on every request. That header
//! beats the daemon's `VIBECLI_DEFAULT_BACKEND` and any per-model pin —
//! see [`crate::inference_routes`] in vibecli-cli.
//!
//! Auth: bearer token from `~/.vibecli/daemon.token`. Re-read on every
//! request so a daemon restart (which rotates the token) doesn't require
//! re-launching the host process. If the file is missing we send no auth
//! and let the daemon return 401 with a message the user can act on.

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "http://localhost:7878";
const BACKEND_HEADER: &str = "x-vibecli-backend";
const BACKEND_VALUE: &str = "mistralrs";

#[derive(Debug, Serialize)]
struct ChatRequestBody {
    model: String,
    messages: Vec<ChatMessageBody>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageBody {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseFrame {
    message: Option<ChatMessageBody>,
    #[allow(dead_code)]
    done: bool,
}

pub struct VibeCliMistralRsProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    base_url: String,
    display_name: String,
}

impl VibeCliMistralRsProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let raw_url = config
            .api_url
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let base_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            raw_url
        } else {
            format!("http://{}", raw_url)
        };
        let display_name = format!("VibeCLI mistralrs ({})", config.model);

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .connect_timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url,
            display_name,
        }
    }

    /// Token resolution: explicit `config.api_key`, then env `VIBECLI_DAEMON_TOKEN`,
    /// then `~/.vibecli/daemon.token`. Re-read each call so daemon restarts that
    /// rotate the file are picked up without restarting the host.
    fn current_token(&self) -> Option<String> {
        if let Some(k) = &self.config.api_key {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        if let Ok(env) = std::env::var("VIBECLI_DAEMON_TOKEN") {
            if !env.is_empty() {
                return Some(env);
            }
        }
        let home = std::env::var("HOME").ok().filter(|s| !s.is_empty())?;
        let path = std::path::PathBuf::from(home)
            .join(".vibecli")
            .join("daemon.token");
        std::fs::read_to_string(&path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn auth_post(&self, url: String) -> reqwest::RequestBuilder {
        let mut req = self.client.post(url).header(BACKEND_HEADER, BACKEND_VALUE);
        if let Some(tok) = self.current_token() {
            req = req.header("Authorization", format!("Bearer {}", tok));
        }
        req
    }

    fn auth_get(&self, url: String) -> reqwest::RequestBuilder {
        let mut req = self.client.get(url).header(BACKEND_HEADER, BACKEND_VALUE);
        if let Some(tok) = self.current_token() {
            req = req.header("Authorization", format!("Bearer {}", tok));
        }
        req
    }

    fn build_options(&self) -> Option<ChatOptions> {
        Some(ChatOptions {
            temperature: self.config.temperature,
            num_predict: self.config.max_tokens.or(Some(16_384)),
        })
    }

    fn map_messages(messages: &[Message], context: Option<String>) -> Vec<ChatMessageBody> {
        let mut out: Vec<ChatMessageBody> = messages
            .iter()
            .map(|m| ChatMessageBody {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
            })
            .collect();
        if let Some(ctx) = context {
            if let Some(last) = out.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\nUser: {}", ctx, last.content);
                }
            }
        }
        out
    }

    /// List models known to the daemon's mistralrs backend.
    /// The daemon's `list_models` only reports *already-loaded* models, so
    /// expect this to be empty until the user has hit `/api/pull` or made a
    /// chat request that lazy-loads a model. Callers should fall back to a
    /// static list (see `useModelRegistry.ts`) for the initial dropdown.
    pub async fn list_models(base_url: Option<String>) -> Result<Vec<String>> {
        let base_url = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let token = std::env::var("VIBECLI_DAEMON_TOKEN").ok().or_else(|| {
            let home = std::env::var("HOME").ok().filter(|s| !s.is_empty())?;
            std::fs::read_to_string(
                std::path::PathBuf::from(home)
                    .join(".vibecli")
                    .join("daemon.token"),
            )
            .ok()
            .map(|s| s.trim().to_string())
        });

        let mut req = client
            .get(format!("{}/api/tags", base_url))
            .header(BACKEND_HEADER, BACKEND_VALUE);
        if let Some(tok) = token {
            req = req.header("Authorization", format!("Bearer {}", tok));
        }
        let response = req
            .send()
            .await
            .context("Failed to connect to vibecli daemon")?;

        #[derive(Deserialize)]
        struct ModelListResponse {
            models: Vec<ModelInfo>,
        }
        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let list: ModelListResponse = response
            .json()
            .await
            .context("Failed to parse vibecli /api/tags response")?;
        Ok(list.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl AIProvider for VibeCliMistralRsProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        self.auth_get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        // Route completions through chat — mistralrs's chat template is
        // model-specific and produces better output than raw `/api/generate`
        // for instruct-tuned models like Qwen.
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];
        let text = self.chat(&messages, None).await?;
        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        // Same rationale as `complete` — re-use the streaming chat path.
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];
        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let body = ChatRequestBody {
            model: self.config.model.clone(),
            messages: Self::map_messages(messages, context),
            stream: false,
            options: self.build_options(),
        };
        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send chat request to vibecli daemon")?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read vibecli response body")?;
        if !status.is_success() {
            anyhow::bail!("vibecli mistralrs API error ({}): {}", status, body_text);
        }
        let frame: ChatResponseFrame = serde_json::from_str(&body_text).context(format!(
            "Failed to parse vibecli chat response: {}",
            body_text
        ))?;
        Ok(frame.message.map(|m| m.content).unwrap_or_default())
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let body = ChatRequestBody {
            model: self.config.model.clone(),
            messages: Self::map_messages(messages, None),
            stream: true,
            options: self.build_options(),
        };
        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send streaming chat request to vibecli daemon")?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("vibecli mistralrs API error ({}): {}", status, error_text);
        }

        // Daemon emits NDJSON frames matching `ChatResponseFrame`. Buffer
        // partial UTF-8 / partial JSON lines so chunk boundaries that split
        // mid-frame don't lose tokens.
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let stream = response.bytes_stream();
        let completion_stream = stream
            .map(move |chunk| -> Result<String> {
                let chunk = chunk?;
                let mut guard = buf.lock().unwrap_or_else(|e| e.into_inner());
                guard.extend_from_slice(&chunk);
                let valid_up_to = match std::str::from_utf8(&guard) {
                    Ok(_) => guard.len(),
                    Err(e) => e.valid_up_to(),
                };
                if valid_up_to == 0 {
                    return Ok(String::new());
                }
                let text = String::from_utf8_lossy(&guard[..valid_up_to]).into_owned();
                let remainder = guard[valid_up_to..].to_vec();
                *guard = remainder;

                let mut result = String::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<ChatResponseFrame>(line) {
                        Ok(frame) => {
                            if let Some(msg) = frame.message {
                                result.push_str(&msg.content);
                            }
                        }
                        Err(_) => {
                            let mut guard2 = buf.lock().unwrap_or_else(|e| e.into_inner());
                            let mut leftover = line.as_bytes().to_vec();
                            leftover.push(b'\n');
                            leftover.extend_from_slice(&guard2);
                            *guard2 = leftover;
                        }
                    }
                }
                Ok(result)
            })
            .boxed();
        Ok(completion_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_default_when_none() {
        let cfg = ProviderConfig::new(
            "vibecli-mistralrs".to_string(),
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
        );
        let p = VibeCliMistralRsProvider::new(cfg);
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn url_preserves_explicit_scheme() {
        let cfg = ProviderConfig::new(
            "vibecli-mistralrs".to_string(),
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
        )
        .with_api_url("http://10.0.0.5:7878".to_string());
        let p = VibeCliMistralRsProvider::new(cfg);
        assert_eq!(p.base_url, "http://10.0.0.5:7878");
    }

    #[test]
    fn url_prepends_http_when_no_scheme() {
        let cfg = ProviderConfig::new(
            "vibecli-mistralrs".to_string(),
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
        )
        .with_api_url("box.local:7878".to_string());
        let p = VibeCliMistralRsProvider::new(cfg);
        assert_eq!(p.base_url, "http://box.local:7878");
    }

    #[test]
    fn display_name_includes_model() {
        let cfg = ProviderConfig::new(
            "vibecli-mistralrs".to_string(),
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
        );
        let p = VibeCliMistralRsProvider::new(cfg);
        assert_eq!(p.name(), "VibeCLI mistralrs (Qwen/Qwen2.5-0.5B-Instruct)");
    }

    #[test]
    fn token_prefers_config_api_key() {
        let cfg = ProviderConfig::new(
            "vibecli-mistralrs".to_string(),
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
        )
        .with_api_key("explicit-token".to_string());
        let p = VibeCliMistralRsProvider::new(cfg);
        assert_eq!(p.current_token(), Some("explicit-token".to_string()));
    }
}
