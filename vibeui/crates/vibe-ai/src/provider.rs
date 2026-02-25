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
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
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
