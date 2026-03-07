//! AI provider abstraction layer

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::Stream;
use anyhow::Result;

/// Code context for AI completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// The programming language
    pub language: String,
    /// File path
    pub file_path: Option<String>,
    /// Code before the cursor
    pub prefix: String,
    /// Code after the cursor
    pub suffix: String,
    /// Additional context (e.g., imports, related files)
    pub additional_context: Vec<String>,
}

/// An image attached to a chat message (for vision-capable providers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    /// Base64-encoded image bytes.
    pub base64: String,
    /// MIME type: `"image/png"`, `"image/jpeg"`, `"image/gif"`, `"image/webp"`.
    pub media_type: String,
}

impl ImageAttachment {
    /// Read an image file from disk and base64-encode it.
    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut bytes)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let media_type = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };
        Ok(Self {
            base64: base64_encode(&bytes),
            media_type: media_type.to_string(),
        })
    }
}

/// Simple base64 encoder (no external crate required — standard alphabet).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 0x3F) as usize] as char } else { '=' });
    }
    out
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    /// Return the lowercase API string for this role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// Token usage statistics returned by a provider response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }

    /// Accumulate another usage record into this one.
    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
    }

    /// Estimated cost in USD based on provider name and model string.
    pub fn estimated_cost_usd(&self, provider: &str, model: &str) -> f64 {
        let (input_price, output_price): (f64, f64) = match (provider, model) {
            (_, m) if m.contains("claude-opus-4") => (15.0 / 1_000_000.0, 75.0 / 1_000_000.0),
            (_, m) if m.contains("claude-sonnet-4") => (3.0 / 1_000_000.0, 15.0 / 1_000_000.0),
            (_, m) if m.contains("claude-haiku-4") => (0.8 / 1_000_000.0, 4.0 / 1_000_000.0),
            (_, m) if m.contains("gpt-4o") => (2.5 / 1_000_000.0, 10.0 / 1_000_000.0),
            (_, m) if m.contains("gpt-4-turbo") => (10.0 / 1_000_000.0, 30.0 / 1_000_000.0),
            (_, m) if m.contains("gpt-3.5") => (0.5 / 1_000_000.0, 1.5 / 1_000_000.0),
            _ => (0.0, 0.0), // Ollama / local providers = free
        };
        self.prompt_tokens as f64 * input_price + self.completion_tokens as f64 * output_price
    }
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    /// Token usage if the provider reported it (None for local models like Ollama).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// Stream of completion chunks
pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

/// AI provider trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available/configured
    async fn is_available(&self) -> bool;

    /// Generate a code completion
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;

    /// Generate a streaming code completion
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;

    /// Chat with the provider
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;

    /// Stream chat response
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;

    /// Chat, returning a full `CompletionResponse` with token usage.
    /// Default implementation wraps `chat()` with no usage info.
    /// Cloud providers (Claude, OpenAI) should override this to return usage.
    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let text = self.chat(messages, context).await?;
        Ok(CompletionResponse {
            text,
            model: self.name().to_string(),
            usage: None,
        })
    }

    /// Chat with optional image attachments (vision).
    /// Default implementation ignores images and falls back to `chat`.
    /// Vision-capable providers (Claude, OpenAI) should override this.
    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        // Default: ignore images, use text-only chat.
        let _ = images;
        self.chat(messages, context).await
    }

    /// Returns true if this provider supports vision (image) inputs.
    fn supports_vision(&self) -> bool {
        false
    }
}

/// AI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    /// Path to a helper script that emits a fresh API key on stdout.
    /// E.g. `~/.vibecli/get-key.sh claude`
    /// If set, this overrides `api_key` when the script exits 0.
    #[serde(default)]
    pub api_key_helper: Option<String>,
    /// Extended thinking budget in tokens (Claude only).
    /// When set, passes `"thinking": {"type":"enabled","budget_tokens":N}` to the API.
    #[serde(default)]
    pub thinking_budget_tokens: Option<u32>,
}

impl ProviderConfig {
    pub fn new(provider_type: String, model: String) -> Self {
        Self {
            provider_type,
            api_key: None,
            api_url: None,
            model,
            max_tokens: None,
            temperature: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_api_url(mut self, api_url: String) -> Self {
        self.api_url = Some(api_url);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Resolve the API key: run `api_key_helper` script if configured;
    /// fall back to the static `api_key` field.
    pub async fn resolve_api_key(&self) -> Option<String> {
        if let Some(helper) = &self.api_key_helper {
            // Expand leading `~`
            let helper_path = if helper.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}{}", home, &helper[1..])
            } else {
                helper.clone()
            };
            // Split into program + args
            let parts: Vec<&str> = helper_path.splitn(2, ' ').collect();
            let (prog, args) = if parts.len() == 2 {
                (parts[0], parts[1].split_whitespace().collect::<Vec<_>>())
            } else {
                (parts[0], vec![])
            };
            match tokio::process::Command::new(prog)
                .args(&args)
                .output()
                .await
            {
                Ok(out) if out.status.success() => {
                    let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !key.is_empty() {
                        return Some(key);
                    }
                }
                _ => {} // fall through to static key
            }
        }
        self.api_key.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TokenUsage ───────────────────────────────────────────────────────

    #[test]
    fn token_usage_total() {
        let u = TokenUsage { prompt_tokens: 100, completion_tokens: 50 };
        assert_eq!(u.total(), 150);
    }

    #[test]
    fn token_usage_total_zero() {
        let u = TokenUsage::default();
        assert_eq!(u.total(), 0);
    }

    #[test]
    fn token_usage_add() {
        let mut a = TokenUsage { prompt_tokens: 10, completion_tokens: 20 };
        let b = TokenUsage { prompt_tokens: 5, completion_tokens: 15 };
        a.add(&b);
        assert_eq!(a.prompt_tokens, 15);
        assert_eq!(a.completion_tokens, 35);
        assert_eq!(a.total(), 50);
    }

    #[test]
    fn estimated_cost_claude_opus() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("claude", "claude-opus-4-20250514");
        // 15.0 + 75.0 = 90.0
        assert!((cost - 90.0).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_claude_sonnet() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("claude", "claude-sonnet-4-20250514");
        // 3.0 + 15.0 = 18.0
        assert!((cost - 18.0).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_claude_haiku() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("claude", "claude-haiku-4-20250101");
        // 0.8 + 4.0 = 4.8
        assert!((cost - 4.8).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_gpt4o() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("openai", "gpt-4o-2024-08-06");
        // 2.5 + 10.0 = 12.5
        assert!((cost - 12.5).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_gpt4_turbo() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("openai", "gpt-4-turbo-preview");
        // 10.0 + 30.0 = 40.0
        assert!((cost - 40.0).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_gpt35() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("openai", "gpt-3.5-turbo");
        // 0.5 + 1.5 = 2.0
        assert!((cost - 2.0).abs() < 0.01, "got {}", cost);
    }

    #[test]
    fn estimated_cost_ollama_free() {
        let u = TokenUsage { prompt_tokens: 1_000_000, completion_tokens: 1_000_000 };
        let cost = u.estimated_cost_usd("ollama", "llama3.1");
        assert!((cost - 0.0).abs() < 0.001, "local model should be free, got {}", cost);
    }

    #[test]
    fn estimated_cost_unknown_provider_free() {
        let u = TokenUsage { prompt_tokens: 500, completion_tokens: 500 };
        let cost = u.estimated_cost_usd("custom", "my-model");
        assert!((cost - 0.0).abs() < 0.001);
    }

    // ── ProviderConfig ───────────────────────────────────────────────────

    #[test]
    fn provider_config_new() {
        let cfg = ProviderConfig::new("claude".into(), "claude-sonnet-4".into());
        assert_eq!(cfg.provider_type, "claude");
        assert_eq!(cfg.model, "claude-sonnet-4");
        assert!(cfg.api_key.is_none());
        assert!(cfg.api_url.is_none());
        assert!(cfg.max_tokens.is_none());
        assert!(cfg.temperature.is_none());
    }

    #[test]
    fn provider_config_builder_chain() {
        let cfg = ProviderConfig::new("openai".into(), "gpt-4o".into())
            .with_api_key("sk-test".into())
            .with_api_url("https://api.openai.com".into())
            .with_max_tokens(4096)
            .with_temperature(0.7);
        assert_eq!(cfg.api_key.as_deref(), Some("sk-test"));
        assert_eq!(cfg.api_url.as_deref(), Some("https://api.openai.com"));
        assert_eq!(cfg.max_tokens, Some(4096));
        assert!((cfg.temperature.unwrap() - 0.7).abs() < 0.001);
    }

    #[test]
    fn provider_config_serialization_roundtrip() {
        let cfg = ProviderConfig::new("gemini".into(), "gemini-pro".into())
            .with_api_key("test-key".into());
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: ProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.provider_type, "gemini");
        assert_eq!(decoded.model, "gemini-pro");
        assert_eq!(decoded.api_key.as_deref(), Some("test-key"));
    }

    // ── base64_encode ────────────────────────────────────────────────────

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_hello() {
        // "Hello" = SGVsbG8=
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn base64_encode_padding() {
        // "Hi" = SGk=  (2 bytes → 1 padding)
        assert_eq!(base64_encode(b"Hi"), "SGk=");
        // "A" = QQ==  (1 byte → 2 padding)
        assert_eq!(base64_encode(b"A"), "QQ==");
        // "Hel" = SGVs (3 bytes → no padding)
        assert_eq!(base64_encode(b"Hel"), "SGVs");
    }

    // ── Message / MessageRole ────────────────────────────────────────────

    #[test]
    fn message_role_serde() {
        let msg = Message { role: MessageRole::User, content: "test".into() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"user\""));
        let decoded: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.role, MessageRole::User);
    }

    #[test]
    fn message_role_system() {
        let json = r#"{"role":"system","content":"sys"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, MessageRole::System);
    }

    // ── CompletionResponse ───────────────────────────────────────────────

    #[test]
    fn completion_response_with_usage() {
        let resp = CompletionResponse {
            text: "hello".into(),
            model: "test".into(),
            usage: Some(TokenUsage { prompt_tokens: 10, completion_tokens: 5 }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"usage\""));
        let decoded: CompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.usage.unwrap().total(), 15);
    }

    #[test]
    fn completion_response_without_usage() {
        let json = r#"{"text":"hi","model":"m"}"#;
        let resp: CompletionResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
    }

    // ── MessageRole::as_str ──────────────────────────────────────────────

    #[test]
    fn message_role_as_str_all_variants() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
    }

    // ── CodeContext serde roundtrip ───────────────────────────────────────

    #[test]
    fn code_context_serde_roundtrip() {
        let ctx = CodeContext {
            language: "rust".to_string(),
            file_path: Some("src/main.rs".to_string()),
            prefix: "fn main() {".to_string(),
            suffix: "}".to_string(),
            additional_context: vec!["use std::io;".to_string()],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: CodeContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.language, "rust");
        assert_eq!(back.file_path.as_deref(), Some("src/main.rs"));
        assert_eq!(back.prefix, "fn main() {");
        assert_eq!(back.suffix, "}");
        assert_eq!(back.additional_context.len(), 1);
    }

    #[test]
    fn code_context_no_file_path() {
        let ctx = CodeContext {
            language: "python".into(),
            file_path: None,
            prefix: "def f():".into(),
            suffix: "".into(),
            additional_context: vec![],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: CodeContext = serde_json::from_str(&json).unwrap();
        assert!(back.file_path.is_none());
    }

    // ── ImageAttachment serde roundtrip ──────────────────────────────────

    #[test]
    fn image_attachment_serde_roundtrip() {
        let img = ImageAttachment {
            base64: "dGVzdA==".to_string(),
            media_type: "image/png".to_string(),
        };
        let json = serde_json::to_string(&img).unwrap();
        let back: ImageAttachment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.base64, "dGVzdA==");
        assert_eq!(back.media_type, "image/png");
    }

    // ── TokenUsage serde roundtrip ───────────────────────────────────────

    #[test]
    fn token_usage_serde_roundtrip() {
        let usage = TokenUsage { prompt_tokens: 100, completion_tokens: 50 };
        let json = serde_json::to_string(&usage).unwrap();
        let back: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.prompt_tokens, 100);
        assert_eq!(back.completion_tokens, 50);
    }

    // ── ProviderConfig default ───────────────────────────────────────────

    #[test]
    fn provider_config_default() {
        let cfg = ProviderConfig::default();
        assert_eq!(cfg.provider_type, "");
        assert_eq!(cfg.model, "");
        assert!(cfg.api_key.is_none());
        assert!(cfg.api_url.is_none());
        assert!(cfg.max_tokens.is_none());
        assert!(cfg.temperature.is_none());
        assert!(cfg.api_key_helper.is_none());
        assert!(cfg.thinking_budget_tokens.is_none());
    }

    // ── base64_encode larger input ───────────────────────────────────────

    #[test]
    fn base64_encode_longer_input() {
        // "Hello, World!" = SGVsbG8sIFdvcmxkIQ==
        assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn base64_encode_binary_data() {
        let data: Vec<u8> = (0..=255).collect();
        let encoded = base64_encode(&data);
        // Just verify it doesn't panic and has expected length
        assert_eq!(encoded.len(), (256_usize).div_ceil(3) * 4);
    }
}
