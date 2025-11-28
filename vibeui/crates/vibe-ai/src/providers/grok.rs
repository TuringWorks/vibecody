//! xAI Grok provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;

/// Grok provider
pub struct GrokProvider {
    config: ProviderConfig,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl GrokProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for GrokProvider {
    fn name(&self) -> &str {
        "Grok"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, _context: &CodeContext) -> Result<CompletionResponse> {
        anyhow::bail!("Grok provider not yet implemented")
    }

    async fn stream_complete(&self, _context: &CodeContext) -> Result<CompletionStream> {
        anyhow::bail!("Grok provider not yet implemented")
    }

    async fn chat(&self, _messages: &[Message], _context: Option<String>) -> Result<String> {
        anyhow::bail!("Grok provider not yet implemented")
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<CompletionStream> {
        anyhow::bail!("Grok provider not yet implemented")
    }
}
