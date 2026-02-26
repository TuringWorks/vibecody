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

        let confidence = estimate_confidence(&response.text);
        Ok(Completion {
            text: response.text,
            provider: provider.name().to_string(),
            confidence,
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

/// Estimate completion confidence from the returned text.
///
/// Heuristics (all contribute additively, capped at 1.0):
/// - Non-empty response: base 0.5
/// - Response length ≥ 10 chars: +0.2
/// - Response length ≥ 50 chars: +0.1 (substantial suggestion)
/// - No uncertainty markers ("I don't know", "I'm not sure", etc.): +0.1
/// - Ends with a syntactically plausible token (`;`, `}`, `)`, `\n`): +0.1
fn estimate_confidence(text: &str) -> f32 {
    if text.trim().is_empty() {
        return 0.0;
    }
    let mut score: f32 = 0.5;
    let len = text.len();
    if len >= 10  { score += 0.2; }
    if len >= 50  { score += 0.1; }

    let lower = text.to_lowercase();
    let uncertain = ["i don't know", "i'm not sure", "i cannot", "i can't",
                     "i am not sure", "unable to", "not available"];
    if !uncertain.iter().any(|u| lower.contains(u)) {
        score += 0.1;
    }

    if text.trim_end().ends_with(|c| matches!(c, ';' | '}' | ')' | '\n' | ',')) {
        score += 0.1;
    }

    score.min(1.0)
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
