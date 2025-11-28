//! Claude AI provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;

/// Claude AI provider (Anthropic)
pub struct ClaudeProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, _context: &CodeContext) -> Result<CompletionResponse> {
        // TODO: Implement Claude API integration
        anyhow::bail!("Claude provider not yet implemented")
    }

    async fn stream_complete(&self, _context: &CodeContext) -> Result<CompletionStream> {
        anyhow::bail!("Claude provider not yet implemented")
    }

    async fn chat(&self, _messages: &[Message], _context: Option<String>) -> Result<String> {
        anyhow::bail!("Claude provider not yet implemented")
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<CompletionStream> {
        anyhow::bail!("Claude provider not yet implemented")
    }
}
