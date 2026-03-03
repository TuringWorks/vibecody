//! GitHub Copilot provider — OpenAI-compatible endpoint with GitHub token exchange.
//!
//! Auth: Set GITHUB_TOKEN (classic or fine-grained PAT with copilot scope).
//! The provider automatically exchanges the GitHub token for a short-lived Copilot
//! API token (valid ~30 minutes) and refreshes it as needed.
//!
//! Supported models: gpt-4o (default), gpt-3.5-turbo, claude-3.5-sonnet
//! The model you get depends on your GitHub Copilot plan.

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream,
    ImageAttachment, Message, MessageRole, ProviderConfig, TokenUsage,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const COPILOT_TOKEN_URL: &str =
    "https://api.github.com/copilot_internal/v2/token";
const COPILOT_BASE_URL: &str = "https://api.githubcopilot.com";

// ── Copilot token exchange ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    /// Unix timestamp when this token expires
    expires_at: Option<u64>,
}

#[derive(Debug, Clone)]
struct CopilotToken {
    token: String,
    expires_at: u64,
}

impl CopilotToken {
    fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 60 >= self.expires_at // refresh 60s before expiry
    }
}

// ── OpenAI-compatible request/response ───────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

// ── Provider ──────────────────────────────────────────────────────────────────

pub struct CopilotProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    token_cache: Arc<Mutex<Option<CopilotToken>>>,
}

impl CopilotProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn github_token(&self) -> String {
        self.config.api_key.clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .unwrap_or_default()
    }

    /// Exchange GitHub token for a short-lived Copilot API token.
    async fn get_copilot_token(&self) -> Result<String> {
        let mut cache = self.token_cache.lock().await;

        // Return cached token if still valid
        if let Some(ref t) = *cache {
            if !t.is_expired() {
                return Ok(t.token.clone());
            }
        }

        let gh_token = self.github_token();
        if gh_token.is_empty() {
            bail!("GITHUB_TOKEN not set (required for GitHub Copilot provider)");
        }

        let resp = self.client
            .get(COPILOT_TOKEN_URL)
            .header("Authorization", format!("token {}", gh_token))
            .header("Accept", "application/json")
            .header("User-Agent", "vibecli/0.1")
            .send()
            .await
            .context("Failed to reach GitHub Copilot token endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("GitHub Copilot token exchange failed ({}): {}", status, body);
        }

        let token_resp: CopilotTokenResponse = resp.json().await
            .context("Failed to parse Copilot token response")?;

        let expires_at = token_resp.expires_at.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() + 1800 // default 30 min
        });

        let token = token_resp.token.clone();
        *cache = Some(CopilotToken { token: token_resp.token, expires_at });
        Ok(token)
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<ChatMessage> {
        let mut result: Vec<ChatMessage> = messages.iter().map(|m| ChatMessage {
            role: match m.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            }.to_string(),
            content: m.content.clone(),
        }).collect();

        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\nUser: {}", ctx, last.content);
                }
            }
        }
        result
    }
}

#[async_trait]
impl AIProvider for CopilotProvider {
    fn name(&self) -> &str { "Copilot" }

    async fn is_available(&self) -> bool {
        !self.github_token().is_empty()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message { role: MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: MessageRole::User, content: prompt },
        ];
        self.chat_response(&messages, None).await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message { role: MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: MessageRole::User, content: prompt },
        ];
        self.stream_chat(&messages).await
    }

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let copilot_token = self.get_copilot_token().await?;
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let url = format!("{}/chat/completions", COPILOT_BASE_URL);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", copilot_token))
            .header("Copilot-Integration-Id", "vscode-chat")
            .header("Editor-Version", "vscode/1.85.0")
            .header("User-Agent", "vibecli/0.1")
            .json(&request)
            .send()
            .await
            .context("Copilot request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Copilot API error {}: {}", status, body);
        }

        let body: ChatResponse = resp.json().await.context("Failed to parse Copilot response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
        });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let copilot_token = self.get_copilot_token().await?;
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let url = format!("{}/chat/completions", COPILOT_BASE_URL);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", copilot_token))
            .header("Copilot-Integration-Id", "vscode-chat")
            .header("Editor-Version", "vscode/1.85.0")
            .header("User-Agent", "vibecli/0.1")
            .json(&request)
            .send()
            .await
            .context("Copilot stream request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Copilot API error {}: {}", status, body);
        }

        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<StreamResponse>(data) {
                        if let Some(c) = r.choices.first().and_then(|ch| ch.delta.content.as_ref()) {
                            content.push_str(c);
                        }
                    }
                }
            }
            Ok(content)
        }).boxed();
        Ok(stream)
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

// ── Device Flow helper (for one-time GITHUB_TOKEN setup) ─────────────────────

/// Run the GitHub Device Flow to obtain a personal access token interactively.
/// The token is printed to stdout; the user saves it as GITHUB_TOKEN.
/// Client ID is the public GitHub CLI app ID (no secret needed for device flow).
pub async fn run_device_flow() -> Result<String> {
    use serde_json::Value;

    const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98"; // GitHub CLI public app
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Step 1: request device code
    let device_resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "read:user copilot")])
        .send().await?;
    if !device_resp.status().is_success() {
        let status = device_resp.status();
        let body = device_resp.text().await.unwrap_or_default();
        bail!("GitHub device code request failed ({}): {}", status, body);
    }
    let resp = device_resp.json::<Value>().await?;

    let device_code = resp["device_code"].as_str().context("no device_code")?.to_string();
    let user_code = resp["user_code"].as_str().unwrap_or("???");
    let verify_url = resp["verification_uri"].as_str().unwrap_or("https://github.com/login/device");
    let interval_secs = resp["interval"].as_u64().unwrap_or(5);

    eprintln!("\n🔑 GitHub Copilot OAuth");
    eprintln!("  1. Open: {}", verify_url);
    eprintln!("  2. Enter code: {}", user_code);
    eprintln!("  3. Waiting for authorization...\n");

    // Step 2: poll for access token
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;

        let poll_resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", &device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send().await?;
        if !poll_resp.status().is_success() {
            let status = poll_resp.status();
            let body = poll_resp.text().await.unwrap_or_default();
            bail!("GitHub OAuth poll failed ({}): {}", status, body);
        }
        let poll = poll_resp.json::<Value>().await?;

        if let Some(token) = poll["access_token"].as_str() {
            return Ok(token.to_string());
        }

        let error = poll["error"].as_str().unwrap_or("unknown");
        match error {
            "authorization_pending" | "slow_down" => continue,
            "expired_token" => bail!("Device code expired. Re-run to try again."),
            "access_denied" => bail!("Authorization denied by user."),
            other => bail!("OAuth error: {}", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "copilot".into(),
            api_key: None,
            api_url: None,
            model: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_copilot() {
        let p = CopilotProvider::new(test_config());
        assert_eq!(p.name(), "Copilot");
    }

    #[test]
    fn copilot_token_expired() {
        let token = CopilotToken { token: "tok".into(), expires_at: 0 };
        assert!(token.is_expired());
    }

    #[test]
    fn copilot_token_not_expired() {
        let token = CopilotToken {
            token: "tok".into(),
            expires_at: u64::MAX,
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn copilot_constants() {
        assert!(COPILOT_TOKEN_URL.contains("github.com"));
        assert!(COPILOT_BASE_URL.contains("githubcopilot.com"));
    }

    #[test]
    fn copilot_token_response_deser() {
        let json = r#"{"token":"ghu_abc123","expires_at":1700000000}"#;
        let resp: CopilotTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.token, "ghu_abc123");
        assert_eq!(resp.expires_at, Some(1700000000));
    }

    #[test]
    fn chat_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hi");
    }
}
