//! AI-powered code completion engine

use crate::provider::{AIProvider, CodeContext};
use anyhow::Result;
use std::sync::Arc;

/// Completion suggestion
#[derive(Debug, Clone)]
pub struct Completion {
    pub text: String,
    pub provider: String,
    pub confidence: f32,
}

/// Code completion engine
pub struct CompletionEngine {
    providers: Vec<Arc<dyn AIProvider>>,
    active_provider_index: usize,
}

impl CompletionEngine {
    /// Create a new completion engine
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_provider_index: 0,
        }
    }

    /// Add an AI provider
    pub fn add_provider(&mut self, provider: Arc<dyn AIProvider>) {
        self.providers.push(provider);
    }

    /// Set the active provider by index
    pub fn set_active_provider(&mut self, index: usize) -> Result<()> {
        if index >= self.providers.len() {
            anyhow::bail!("Provider index out of bounds");
        }
        self.active_provider_index = index;
        Ok(())
    }

    /// Get the active provider
    pub fn active_provider(&self) -> Option<&Arc<dyn AIProvider>> {
        self.providers.get(self.active_provider_index)
    }

    /// Get all providers
    pub fn providers(&self) -> &[Arc<dyn AIProvider>] {
        &self.providers
    }

    /// Request a code completion
    pub async fn complete(&self, context: &CodeContext) -> Result<Completion> {
        let provider = self
            .active_provider()
            .ok_or_else(|| anyhow::anyhow!("No active provider"))?;

        if !provider.is_available().await {
            anyhow::bail!("Provider {} is not available", provider.name());
        }

        let response = provider.complete(context).await?;

        Ok(Completion {
            text: response.text,
            provider: provider.name().to_string(),
            confidence: 0.8, // TODO: Calculate confidence based on model
        })
    }

    /// Request a streaming code completion
    pub async fn stream_complete(
        &self,
        context: &CodeContext,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        let provider = self
            .active_provider()
            .ok_or_else(|| anyhow::anyhow!("No active provider"))?;

        if !provider.is_available().await {
            anyhow::bail!("Provider {} is not available", provider.name());
        }

        provider.stream_complete(context).await
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let engine = CompletionEngine::new();
        assert_eq!(engine.providers().len(), 0);
        assert!(engine.active_provider().is_none());
    }
}
