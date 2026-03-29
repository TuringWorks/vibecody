//! Ollama AI provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
    /// Base64-encoded images for vision models (Qwen2-VL, GLM-4V, LLaVA, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessage>,
    #[allow(dead_code)]
    done: bool,
}

/// Ollama AI provider
pub struct OllamaProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    base_url: String,
    display_name: String,
    /// Resolved API key: explicit config/env key, or `None` (no auth sent).
    api_key: Option<String>,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// API key resolution: `config.api_key` first, then `OLLAMA_API_KEY` env var.
    /// If neither is set, no auth header is sent (standard Ollama needs no auth).
    pub fn new(config: ProviderConfig) -> Self {
        let raw_url = config
            .api_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        // Normalize: OLLAMA_HOST env var is often set without a scheme
        let base_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            raw_url
        } else {
            format!("http://{}", raw_url)
        };

        let display_name = format!("Ollama ({})", config.model);

        // Resolve API key: explicit config → env var → None (no auth)
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OLLAMA_API_KEY").ok());

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url,
            display_name,
            api_key,
        }
    }

    fn build_prompt(&self, context: &CodeContext) -> String {
        format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        )
    }

    /// Build a POST request, adding Bearer auth only when an API key is configured.
    fn auth_post(&self, url: String) -> reqwest::RequestBuilder {
        let req = self.client.post(url);
        match &self.api_key {
            Some(key) => req.header("Authorization", format!("Bearer {}", key)),
            None => req,
        }
    }

    /// Build a GET request, adding Bearer auth only when an API key is configured.
    fn auth_get(&self, url: String) -> reqwest::RequestBuilder {
        let req = self.client.get(url);
        match &self.api_key {
            Some(key) => req.header("Authorization", format!("Bearer {}", key)),
            None => req,
        }
    }

    fn build_options(&self) -> Option<OllamaOptions> {
        if self.config.temperature.is_some() || self.config.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            })
        } else {
            None
        }
    }

    /// List available Ollama models that support chat.
    ///
    /// Fetches all models from `/api/tags`, then probes each with a minimal
    /// `/api/chat` request to filter out completion-only models (e.g. codellama)
    /// and embedding models (e.g. nomic-embed-text).
    ///
    /// Auth is sent only when `OLLAMA_API_KEY` is set.
    pub async fn list_models(base_url: Option<String>) -> Result<Vec<String>> {
        let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let api_key = std::env::var("OLLAMA_API_KEY").ok();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut req = client.get(format!("{}/api/tags", base_url));
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let response = req
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        #[derive(Deserialize)]
        struct ModelListResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
            #[serde(default)]
            details: ModelDetails,
        }

        #[derive(Deserialize, Default)]
        struct ModelDetails {
            #[serde(default)]
            family: String,
        }

        let list: ModelListResponse = response
            .json()
            .await
            .context("Failed to parse model list")?;

        // Quick filter: skip known embedding-only model families
        let embedding_families = ["nomic-bert", "bert", "all-minilm"];
        let candidates: Vec<String> = list.models
            .into_iter()
            .filter(|m| !embedding_families.contains(&m.details.family.as_str()))
            .map(|m| m.name)
            .collect();

        // Probe each candidate with /api/chat to confirm chat support.
        // Use a short timeout per probe — cloud models respond quickly,
        // local models may take a moment but the error is instant.
        let probe_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut chat_models = Vec::new();
        for model in candidates {
            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "hi"}],
                "stream": false,
                "options": {"num_predict": 1}
            });
            let mut probe_req = probe_client
                .post(format!("{}/api/chat", base_url));
            if let Some(ref key) = api_key {
                probe_req = probe_req.header("Authorization", format!("Bearer {}", key));
            }
            match probe_req
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    // Models that don't support chat return {"error":"...does not support chat"}
                    if !text.contains("does not support chat") {
                        chat_models.push(model);
                    }
                }
                Err(_) => {
                    // Network error — include model anyway; error will surface at chat time
                    chat_models.push(model);
                }
            }
        }

        Ok(chat_models)
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        // Try to ping the Ollama API
        self.auth_get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = self.build_prompt(context);
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(CompletionResponse {
            text: ollama_response.response,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = self.build_prompt(context);
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let response: OllamaResponse = serde_json::from_slice(&chunk)?;
                Ok(response.response)
            })
            .boxed();

        Ok(completion_stream)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
                images: None,
            })
            .collect();

        // Inject context into the last user message if available
        if let Some(ctx) = context {
            if let Some(last_msg) = ollama_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        let status = response.status();
        let body_text = response.text().await.context("Failed to read response body")?;
        


        if !status.is_success() {
            anyhow::bail!("Ollama API error: {}", body_text);
        }

        let ollama_response: OllamaChatResponse = serde_json::from_str(&body_text)
            .context(format!("Failed to parse Ollama chat response: {}", body_text))?;

        Ok(ollama_response.message.map(|m| m.content).unwrap_or_default())
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
                images: None,
            })
            .collect();

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        let stream = response.bytes_stream();

        let completion_stream = stream
            .map(|chunk| -> Result<String, anyhow::Error> {
                let chunk = chunk?;
                let text = std::str::from_utf8(&chunk)?;
                let mut result = String::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    if let Ok(response) = serde_json::from_str::<OllamaChatResponse>(line) {
                        if let Some(msg) = response.message {
                            result.push_str(&msg.content);
                        }
                    }
                }
                Ok(result)
            })
            .boxed();

        Ok(completion_stream)
    }

    fn supports_vision(&self) -> bool {
        // Ollama vision support depends on the model. Common vision models:
        // qwen2-vl, qwen2.5-vl, glm-4v, llava, bakllava, moondream, deepseek-vl
        // We return true and let the model handle it — Ollama will error if the
        // model doesn't support images, which is better than silently dropping them.
        true
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[crate::provider::ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
                images: None,
            })
            .collect();

        // Inject context into the last user message if available
        if let Some(ctx) = context {
            if let Some(last_msg) = ollama_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }

        // Attach images to the last user message (Ollama expects base64 strings)
        if !images.is_empty() {
            if let Some(last_user) = ollama_messages.iter_mut().rev().find(|m| m.role == "user") {
                last_user.images = Some(
                    images.iter().map(|img| img.base64.clone()).collect()
                );
            }
        }

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send vision request to Ollama")?;

        let status = response.status();
        let body_text = response.text().await.context("Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("Ollama vision API error: {}", body_text);
        }

        let ollama_response: OllamaChatResponse = serde_json::from_str(&body_text)
            .context(format!("Failed to parse Ollama vision response: {}", body_text))?;

        Ok(ollama_response.message.map(|m| m.content).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);

        let context = CodeContext {
            language: "rust".to_string(),
            file_path: None,
            prefix: "fn main() {\n    ".to_string(),
            suffix: "\n}".to_string(),
            additional_context: vec![],
        };

        let prompt = provider.build_prompt(&context);
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("fn main()"));
    }

    // ── build_options ────────────────────────────────────────────────────

    #[test]
    fn build_options_none_when_no_config() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);
        assert!(provider.build_options().is_none());
    }

    #[test]
    fn build_options_some_when_temperature_set() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_temperature(0.5);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options();
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert!((opts.temperature.unwrap() - 0.5).abs() < 0.001);
        assert!(opts.num_predict.is_none());
    }

    #[test]
    fn build_options_some_when_max_tokens_set() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_max_tokens(256);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options();
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert!(opts.temperature.is_none());
        assert_eq!(opts.num_predict, Some(256));
    }

    #[test]
    fn build_options_both_set() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_temperature(0.9)
            .with_max_tokens(1024);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options().unwrap();
        assert!((opts.temperature.unwrap() - 0.9).abs() < 0.001);
        assert_eq!(opts.num_predict, Some(1024));
    }

    // ── URL normalization in new() ───────────────────────────────────────

    #[test]
    fn url_default_when_none() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    #[test]
    fn url_preserves_http_prefix() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("http://myhost:11434".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://myhost:11434");
    }

    #[test]
    fn url_preserves_https_prefix() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("https://ollama.example.com".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "https://ollama.example.com");
    }

    #[test]
    fn url_prepends_http_when_no_scheme() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("192.168.1.100:11434".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://192.168.1.100:11434");
    }

    #[test]
    fn url_prepends_http_for_hostname_only() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("ollama-server".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://ollama-server");
    }

    // ── display name ────────────────────────────────────────────────────

    #[test]
    fn display_name_includes_model() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3.1:8b".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.name(), "Ollama (llama3.1:8b)");
    }

    // ── request serde with skip_serializing_if ──────────────────────────

    #[test]
    fn ollama_request_omits_none_options() {
        let req = OllamaRequest {
            model: "codellama".to_string(),
            prompt: "test".to_string(),
            stream: false,
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("options"), "options should be omitted when None");
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"prompt\""));
        assert!(json.contains("\"stream\""));
    }

    #[test]
    fn ollama_request_includes_options_when_some() {
        let req = OllamaRequest {
            model: "codellama".to_string(),
            prompt: "test".to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.7),
                num_predict: Some(100),
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"options\""));
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"num_predict\""));
    }

    #[test]
    fn ollama_options_omits_none_fields() {
        let opts = OllamaOptions {
            temperature: None,
            num_predict: Some(512),
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(!json.contains("temperature"), "temperature should be omitted when None");
        assert!(json.contains("\"num_predict\":512"));
    }

    #[test]
    fn ollama_options_omits_both_none_fields() {
        let opts = OllamaOptions {
            temperature: None,
            num_predict: None,
        };
        let json = serde_json::to_string(&opts).unwrap();
        // Should be an empty object
        assert_eq!(json, "{}");
    }

    #[test]
    fn ollama_chat_request_omits_none_options() {
        let req = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
                images: None,
            }],
            stream: false,
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("options"));
    }

    #[test]
    fn ollama_response_deser() {
        let json = r#"{"response":"Hello world","done":true}"#;
        let resp: OllamaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "Hello world");
        assert!(resp.done);
    }

    #[test]
    fn ollama_chat_response_deser() {
        let json = r#"{"message":{"role":"assistant","content":"reply"},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let msg = resp.message.unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "reply");
        assert!(resp.done);
    }

    #[test]
    fn ollama_chat_response_done_without_message() {
        let json = r#"{"done":true,"total_duration":123456}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.message.is_none());
        assert!(resp.done);
    }

    // ── API key resolution ─────────────────────────────────────────────

    #[test]
    fn api_key_uses_config_when_set() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string())
            .with_api_key("my-secret-key".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, Some("my-secret-key".to_string()));
    }

    #[test]
    fn no_api_key_when_unconfigured() {
        // Without config key or OLLAMA_API_KEY env, api_key should be None
        // (no auth sent to vanilla Ollama).
        // Note: this test may see Some if OLLAMA_API_KEY is set in the environment.
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string());
        let provider = OllamaProvider::new(config);
        if std::env::var("OLLAMA_API_KEY").is_err() {
            assert_eq!(provider.api_key, None);
        }
    }

    #[test]
    fn config_api_key_takes_precedence() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string())
            .with_api_key("config-key".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, Some("config-key".to_string()));
    }
}
