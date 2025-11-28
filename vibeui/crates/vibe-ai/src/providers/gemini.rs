//! Google Gemini provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;

/// Gemini provider
pub struct GeminiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Gemini"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, _context: &CodeContext) -> Result<CompletionResponse> {
        anyhow::bail!("Gemini provider not yet implemented")
    }

    async fn stream_complete(&self, _context: &CodeContext) -> Result<CompletionStream> {
        anyhow::bail!("Gemini provider not yet implemented")
    }

    async fn chat(&self, _messages: &[Message], _context: Option<String>) -> Result<String> {
        anyhow::bail!("Gemini provider not yet implemented")
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<CompletionStream> {
        anyhow::bail!("Gemini provider not yet implemented")
    }
}
