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

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
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
}
