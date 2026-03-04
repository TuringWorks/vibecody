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

    if text.trim_end().ends_with([';', '}', ')', '\n', ',']) {
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

    #[test]
    fn engine_default() {
        let engine = CompletionEngine::default();
        assert!(engine.active_provider().is_none());
    }

    #[test]
    fn set_active_provider_out_of_bounds() {
        let mut engine = CompletionEngine::new();
        let result = engine.set_active_provider(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));
    }

    // ── estimate_confidence tests ────────────────────────────────────────────

    #[test]
    fn confidence_empty_is_zero() {
        assert_eq!(estimate_confidence(""), 0.0);
        assert_eq!(estimate_confidence("   "), 0.0);
    }

    #[test]
    fn confidence_short_text_base() {
        // Non-empty, <10 chars, no syntactic ending → 0.5 + 0.1 (no uncertainty) = 0.6
        let score = estimate_confidence("hi");
        assert!((score - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_medium_text() {
        // >=10 chars: 0.5 + 0.2 + 0.1 = 0.8
        let score = estimate_confidence("some code here!!");
        assert!((score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_long_text() {
        // >=50 chars: 0.5 + 0.2 + 0.1 + 0.1 = 0.9
        let text = "a".repeat(50);
        let score = estimate_confidence(&text);
        assert!((score - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_syntactic_ending_semicolon() {
        // 10+ chars ending with ; → 0.5 + 0.2 + 0.1 + 0.1 = 0.9
        let score = estimate_confidence("let x = 42;");
        assert!((score - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_syntactic_ending_brace() {
        let score = estimate_confidence("fn main() }");
        assert!((score - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_syntactic_ending_paren() {
        let score = estimate_confidence("foo.bar())");
        assert!((score - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_syntactic_ending_comma() {
        let score = estimate_confidence("item_one,");
        assert!(score > 0.5);
    }

    #[test]
    fn confidence_capped_at_one() {
        // Long text with syntactic ending → would be >1.0 without cap
        let text = "a".repeat(60) + ";";
        let score = estimate_confidence(&text);
        assert!((score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_uncertainty_markers_lower_score() {
        let certain = estimate_confidence("Here is the answer code;");
        let uncertain = estimate_confidence("I don't know the answer;");
        assert!(uncertain < certain);
    }

    #[test]
    fn confidence_uncertainty_im_not_sure() {
        let score = estimate_confidence("I'm not sure about this");
        // No uncertainty bonus: 0.5 + 0.2 = 0.7
        assert!((score - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_uncertainty_unable_to() {
        // "unable to determine the result" = 30 chars → 0.5 + 0.2 (>=10) + 0.0 (uncertainty marker) = 0.7
        let score = estimate_confidence("unable to determine the result");
        assert!((score - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_uncertainty_case_insensitive() {
        let upper = estimate_confidence("I DON'T KNOW what to do here");
        let lower = estimate_confidence("i don't know what to do here");
        // Both should have the same score (case-insensitive check)
        assert!((upper - lower).abs() < f32::EPSILON);
    }

    // ── Completion struct ────────────────────────────────────────────────────

    #[test]
    fn completion_struct_fields() {
        let c = Completion {
            text: "fn foo()".to_string(),
            provider: "ollama".to_string(),
            confidence: 0.85,
        };
        assert_eq!(c.text, "fn foo()");
        assert_eq!(c.provider, "ollama");
        assert!((c.confidence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn completion_clone() {
        let c = Completion {
            text: "test".to_string(),
            provider: "test".to_string(),
            confidence: 0.5,
        };
        let c2 = c.clone();
        assert_eq!(c.text, c2.text);
    }
}
