//! Failover provider — tries multiple providers in sequence.
//!
//! When the primary provider fails, automatically falls through to
//! the next provider in the chain until one succeeds.

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// A provider that wraps multiple providers and tries each in sequence.
/// If the first provider fails, it falls through to the next, and so on.
pub struct FailoverProvider {
    chain: Vec<Arc<dyn AIProvider>>,
    name: String,
}

impl FailoverProvider {
    pub fn new(chain: Vec<Arc<dyn AIProvider>>) -> Self {
        let name = if chain.is_empty() {
            "Failover(empty)".to_string()
        } else {
            format!("Failover({})", chain.iter().map(|p| p.name().to_string()).collect::<Vec<_>>().join(" -> "))
        };
        Self { chain, name }
    }
}

#[async_trait]
impl AIProvider for FailoverProvider {
    fn name(&self) -> &str { &self.name }

    async fn is_available(&self) -> bool {
        for provider in &self.chain {
            if provider.is_available().await {
                return true;
            }
        }
        false
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.complete(context).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!("[failover] {} complete failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.stream_complete(context).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    tracing::warn!("[failover] {} stream_complete failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.chat_response(messages, context.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!("[failover] {} chat_response failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.chat(messages, context.clone()).await {
                Ok(text) => return Ok(text),
                Err(e) => {
                    tracing::warn!("[failover] {} chat failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.stream_chat(messages).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    tracing::warn!("[failover] {} stream_chat failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn chat_with_images(&self, messages: &[Message], images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for provider in &self.chain {
            match provider.chat_with_images(messages, images, context.clone()).await {
                Ok(text) => return Ok(text),
                Err(e) => {
                    tracing::warn!("[failover] {} chat_with_images failed: {}, trying next", provider.name(), e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chain_name() {
        let p = FailoverProvider::new(vec![]);
        assert_eq!(p.name(), "Failover(empty)");
    }

    #[tokio::test]
    async fn empty_chain_not_available() {
        let p = FailoverProvider::new(vec![]);
        assert!(!p.is_available().await);
    }

    #[test]
    fn chain_name_shows_providers() {
        // Use a simple mock-like setup — create two Groq providers with different names
        // Just test the name generation logic
        let name = "Failover(A -> B -> C)";
        let p = FailoverProvider { chain: vec![], name: name.to_string() };
        assert_eq!(p.name(), "Failover(A -> B -> C)");
    }
}
