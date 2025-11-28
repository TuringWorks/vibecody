//! OpenAI provider implementation (ChatGPT, Codex)

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::Result;
use async_trait::async_trait;

/// OpenAI provider
pub struct OpenAIProvider {
    config: ProviderConfig,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, _context: &CodeContext) -> Result<CompletionResponse> {
        anyhow::bail!("OpenAI provider not yet implemented")
    }

    async fn stream_complete(&self, _context: &CodeContext) -> Result<CompletionStream> {
        anyhow::bail!("OpenAI provider not yet implemented")
    }

    async fn chat(&self, _messages: &[Message], _context: Option<String>) -> Result<String> {
        anyhow::bail!("OpenAI provider not yet implemented")
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<CompletionStream> {
        anyhow::bail!("OpenAI provider not yet implemented")
    }
}
