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
    display_name: String,
}

impl CopilotProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Copilot ({})", config.model);
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(Mutex::new(None)),
            display_name,
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
    fn name(&self) -> &str { &self.display_name }

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

        // Buffer SSE lines across chunk boundaries — HTTP chunks may split mid-line
        let mut line_buf = String::new();
        let stream = resp.bytes_stream().map(move |chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            line_buf.push_str(&text);
            let mut content = String::new();
            while let Some(nl) = line_buf.find('\n') {
                let line = line_buf[..nl].trim_end_matches('\r').to_string();
                line_buf = line_buf[nl + 1..].to_string();
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
        assert_eq!(p.name(), "Copilot (gpt-4o)");
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

    #[test]
    fn build_messages_maps_roles() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "usr".into() },
            Message { role: MessageRole::Assistant, content: "ast".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn build_messages_appends_context_to_last_user() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, Some("file.rs contents".into()));
        assert!(result[0].content.contains("Context:"));
        assert!(result[0].content.contains("file.rs contents"));
        assert!(result[0].content.contains("hello"));
    }

    #[test]
    fn build_messages_no_context_leaves_content_unchanged() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "world".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result[0].content, "world");
    }

    #[test]
    fn chat_request_serializes_stream_field() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
        assert_eq!(json["model"], "gpt-4o");
        // temperature and max_tokens should be absent (skip_serializing_if)
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn chat_request_includes_optional_fields() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![],
            temperature: Some(0.75),
            max_tokens: Some(1024),
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["temperature"], 0.75);
        assert_eq!(json["max_tokens"], 1024);
    }

    // ── stream response deserialization ─────────────────────────────────

    #[test]
    fn stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"streamed token"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "streamed token");
    }

    #[test]
    fn stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn stream_response_deser_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
    }

    // ── build_messages edge cases ───────────────────────────────────────

    #[test]
    fn build_messages_empty_input() {
        let p = CopilotProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_input_with_context() {
        let p = CopilotProvider::new(test_config());
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_context_only_affects_last_user() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "first".into() },
            Message { role: MessageRole::Assistant, content: "mid".into() },
            Message { role: MessageRole::User, content: "second".into() },
        ];
        let result = p.build_messages(&messages, Some("bg info".into()));
        // First user message unchanged
        assert_eq!(result[0].content, "first");
        // Last user message gets context prepended
        assert!(result[2].content.starts_with("Context:\nbg info"));
        assert!(result[2].content.contains("User: second"));
    }

    #[test]
    fn build_messages_context_skipped_when_last_is_assistant() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "q".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = p.build_messages(&messages, Some("ignored ctx".into()));
        // Last message is assistant, so context is NOT injected
        assert_eq!(result[1].content, "a");
        assert_eq!(result[0].content, "q");
    }

    // ── chat response deserialization edge cases ────────────────────────

    #[test]
    fn chat_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn chat_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"a1"}},{"message":{"role":"assistant","content":"a2"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "a2");
    }

    // ── chat message roundtrip ──────────────────────────────────────────

    #[test]
    fn chat_message_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "hello world".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── copilot token response edge cases ───────────────────────────────

    #[test]
    fn copilot_token_response_deser_no_expires() {
        let json = r#"{"token":"ghu_xyz"}"#;
        let resp: CopilotTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.token, "ghu_xyz");
        assert!(resp.expires_at.is_none());
    }

    #[test]
    fn copilot_token_boundary_expired() {
        // Token that expires exactly now should be considered expired (60s buffer)
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let token = CopilotToken { token: "tok".into(), expires_at: now + 30 };
        // expires_at is within the 60s buffer, so should be expired
        assert!(token.is_expired());
    }

    #[test]
    fn copilot_token_far_future_not_expired() {
        let token = CopilotToken {
            token: "tok".into(),
            expires_at: u64::MAX / 2, // very far future
        };
        assert!(!token.is_expired());
    }

    // ── provider config preserved ───────────────────────────────────────

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "claude-3.5-sonnet".into();
        cfg.temperature = Some(0.5);
        cfg.max_tokens = Some(2048);
        let p = CopilotProvider::new(cfg);
        assert_eq!(p.config.model, "claude-3.5-sonnet");
        assert_eq!(p.config.temperature, Some(0.5));
        assert_eq!(p.config.max_tokens, Some(2048));
    }

    // ── constants validation ────────────────────────────────────────────

    #[test]
    fn copilot_base_url_ends_with_domain() {
        assert!(COPILOT_BASE_URL.starts_with("https://"));
        assert!(!COPILOT_BASE_URL.ends_with('/'));
    }

    #[test]
    fn copilot_token_url_is_https() {
        assert!(COPILOT_TOKEN_URL.starts_with("https://"));
    }

    // ── additional edge case tests ──────────────────────────────────────

    #[test]
    fn chat_message_unicode_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "café résumé naïve".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg2.content, "café résumé naïve");
    }

    #[test]
    fn chat_request_multiple_messages_serialized() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "sys".into() },
                ChatMessage { role: "user".into(), content: "q".into() },
                ChatMessage { role: "assistant".into(), content: "a".into() },
            ],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["messages"].as_array().unwrap().len(), 3);
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][2]["role"], "assistant");
    }

    #[test]
    fn chat_usage_deser() {
        let json = r#"{"prompt_tokens":150,"completion_tokens":75}"#;
        let usage: ChatUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 150);
        assert_eq!(usage.completion_tokens, 75);
    }

    #[test]
    fn stream_response_deser_missing_content_field() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn copilot_token_response_deser_with_extra_fields() {
        // API may return extra fields; deserialization should still work
        let json = r#"{"token":"ghu_tok","expires_at":1700000000,"org_enforcement":"none"}"#;
        let resp: CopilotTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.token, "ghu_tok");
        assert_eq!(resp.expires_at, Some(1700000000));
    }

    #[test]
    fn github_token_from_config() {
        let mut cfg = test_config();
        cfg.api_key = Some("ghp_testtoken123".into());
        let p = CopilotProvider::new(cfg);
        assert_eq!(p.github_token(), "ghp_testtoken123");
    }

    #[test]
    fn github_token_empty_when_no_key_and_no_env() {
        // With no api_key and (probably) no GITHUB_TOKEN env var in test
        let p = CopilotProvider::new(test_config());
        // It should either be empty or come from env; just verify it doesn't panic
        let _ = p.github_token();
    }

    #[test]
    fn chat_response_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
        assert!(resp.usage.is_none());
    }

    #[test]
    fn build_messages_single_system_message_context_not_injected() {
        let p = CopilotProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "sys prompt".into() },
        ];
        let result = p.build_messages(&messages, Some("ctx data".into()));
        // Last message is system, not user, so context should NOT be injected
        assert_eq!(result[0].content, "sys prompt");
    }
}
