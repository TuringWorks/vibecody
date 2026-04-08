//! Failover provider — tries multiple providers in sequence.
//!
//! When the primary provider fails, automatically falls through to
//! the next provider in the chain until one succeeds.
//!
//! With an optional `ProviderHealthTracker`, providers are sorted by
//! health score (antifragility: the system learns which providers are
//! reliable and prefers them).

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message};
use crate::resilience::{ProviderCallOutcome, ProviderHealthTracker, classify_error};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

/// A provider that wraps multiple providers and tries each in sequence.
/// If the first provider fails, it falls through to the next, and so on.
///
/// When a `ProviderHealthTracker` is attached, providers are tried in
/// health-score order (healthiest first) instead of fixed chain order.
pub struct FailoverProvider {
    chain: Vec<Arc<dyn AIProvider>>,
    name: String,
    health_tracker: Option<Arc<ProviderHealthTracker>>,
}

impl std::fmt::Debug for FailoverProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailoverProvider")
            .field("name", &self.name)
            .field("chain_len", &self.chain.len())
            .finish()
    }
}

impl FailoverProvider {
    /// Create a failover provider with fixed ordering (original behavior).
    pub fn new(chain: Vec<Arc<dyn AIProvider>>) -> Self {
        let name = if chain.is_empty() {
            "Failover(empty)".to_string()
        } else {
            format!("Failover({})", chain.iter().map(|p| p.name().to_string()).collect::<Vec<_>>().join(" -> "))
        };
        Self { chain, name, health_tracker: None }
    }

    /// Create a failover provider with health-aware dynamic ordering.
    /// Providers are tried in order of health score (highest first).
    pub fn with_health_tracker(chain: Vec<Arc<dyn AIProvider>>, tracker: Arc<ProviderHealthTracker>) -> Self {
        let name = if chain.is_empty() {
            "Failover(empty)".to_string()
        } else {
            format!("Failover({})", chain.iter().map(|p| p.name().to_string()).collect::<Vec<_>>().join(" -> "))
        };
        Self { chain, name, health_tracker: Some(tracker) }
    }

    /// Get the providers sorted by health score if a tracker is attached,
    /// otherwise return them in the original chain order.
    fn ordered_chain(&self) -> Vec<Arc<dyn AIProvider>> {
        match &self.health_tracker {
            None => self.chain.clone(),
            Some(tracker) => {
                let names: Vec<String> = self.chain.iter().map(|p| p.name().to_string()).collect();
                let ranked = tracker.ranked_providers(&names);
                let mut ordered = Vec::with_capacity(self.chain.len());
                for name in &ranked {
                    if let Some(provider) = self.chain.iter().find(|p| p.name() == name) {
                        ordered.push(provider.clone());
                    }
                }
                ordered
            }
        }
    }

    /// Record a call outcome to the health tracker (if attached).
    fn record_outcome(&self, provider_name: &str, success: bool, latency: std::time::Duration, error: Option<&str>) {
        if let Some(tracker) = &self.health_tracker {
            tracker.record(ProviderCallOutcome {
                provider_name: provider_name.to_string(),
                success,
                latency,
                timestamp: Instant::now(),
                error_category: error.map(classify_error),
            });
        }
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
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.complete(context).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} complete failed: {s}, trying next", p.name()); last_err = e; }
            }
        }
        Err(last_err)
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.stream_complete(context).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} stream_complete failed: {s}, trying next", p.name()); last_err = e; }
            }
        }
        Err(last_err)
    }

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.chat_response(messages, context.clone()).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} chat_response failed: {s}, trying next", p.name()); last_err = e; }
            }
        }
        Err(last_err)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.chat(messages, context.clone()).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} chat failed: {s}, trying next", p.name()); last_err = e; }
            }
        }
        Err(last_err)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.stream_chat(messages).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} stream_chat failed: {s}, trying next", p.name()); last_err = e; }
            }
        }
        Err(last_err)
    }

    async fn chat_with_images(&self, messages: &[Message], images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        let mut last_err = anyhow::anyhow!("No providers in failover chain");
        for p in self.ordered_chain() {
            let start = Instant::now();
            match p.chat_with_images(messages, images, context.clone()).await {
                Ok(v) => { self.record_outcome(p.name(), true, start.elapsed(), None); return Ok(v); }
                Err(e) => { let s = e.to_string(); self.record_outcome(p.name(), false, start.elapsed(), Some(&s)); tracing::warn!("[failover] {} chat_with_images failed: {s}, trying next", p.name()); last_err = e; }
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
        let name = "Failover(A -> B -> C)";
        let p = FailoverProvider { chain: vec![], name: name.to_string(), health_tracker: None };
        assert_eq!(p.name(), "Failover(A -> B -> C)");
    }

    /// A minimal mock provider for testing.
    struct MockProvider {
        mock_name: String,
    }

    impl MockProvider {
        fn new(name: &str) -> Self {
            Self { mock_name: name.to_string() }
        }
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str { &self.mock_name }
        async fn is_available(&self) -> bool { false }
        async fn complete(&self, _ctx: &CodeContext) -> Result<CompletionResponse> {
            anyhow::bail!("mock")
        }
        async fn stream_complete(&self, _ctx: &CodeContext) -> Result<CompletionStream> {
            anyhow::bail!("mock")
        }
        async fn chat(&self, _msgs: &[Message], _ctx: Option<String>) -> Result<String> {
            anyhow::bail!("mock")
        }
        async fn stream_chat(&self, _msgs: &[Message]) -> Result<CompletionStream> {
            anyhow::bail!("mock")
        }
    }

    #[test]
    fn name_with_single_provider() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Claude")),
        ];
        let p = FailoverProvider::new(chain);
        assert_eq!(p.name(), "Failover(Claude)");
    }

    #[test]
    fn name_with_two_providers() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Claude")),
            Arc::new(MockProvider::new("Ollama")),
        ];
        let p = FailoverProvider::new(chain);
        assert_eq!(p.name(), "Failover(Claude -> Ollama)");
    }

    #[test]
    fn name_with_three_providers() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("OpenAI")),
            Arc::new(MockProvider::new("Groq")),
            Arc::new(MockProvider::new("Grok")),
        ];
        let p = FailoverProvider::new(chain);
        assert_eq!(p.name(), "Failover(OpenAI -> Groq -> Grok)");
    }

    // ── empty chain error behavior ───────────────────────────────────────

    #[tokio::test]
    async fn empty_chain_complete_errors() {
        let p = FailoverProvider::new(vec![]);
        let ctx = CodeContext {
            language: "rust".into(),
            file_path: None,
            prefix: "".into(),
            suffix: "".into(),
            additional_context: vec![],
        };
        let result = p.complete(&ctx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No providers"), "expected 'No providers' error, got: {}", err_msg);
    }

    #[tokio::test]
    async fn empty_chain_chat_errors() {
        let p = FailoverProvider::new(vec![]);
        let msgs = vec![Message {
            role: crate::provider::MessageRole::User,
            content: "hello".into(),
        }];
        let result = p.chat(&msgs, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No providers"));
    }

    #[tokio::test]
    async fn empty_chain_stream_complete_errors() {
        let p = FailoverProvider::new(vec![]);
        let ctx = CodeContext {
            language: "py".into(),
            file_path: None,
            prefix: "".into(),
            suffix: "".into(),
            additional_context: vec![],
        };
        assert!(p.stream_complete(&ctx).await.is_err());
    }

    #[tokio::test]
    async fn empty_chain_stream_chat_errors() {
        let p = FailoverProvider::new(vec![]);
        let msgs = vec![Message {
            role: crate::provider::MessageRole::User,
            content: "hi".into(),
        }];
        assert!(p.stream_chat(&msgs).await.is_err());
    }

    #[tokio::test]
    async fn empty_chain_chat_response_errors() {
        let p = FailoverProvider::new(vec![]);
        let msgs = vec![Message {
            role: crate::provider::MessageRole::User,
            content: "hi".into(),
        }];
        assert!(p.chat_response(&msgs, None).await.is_err());
    }

    #[tokio::test]
    async fn empty_chain_chat_with_images_errors() {
        let p = FailoverProvider::new(vec![]);
        let msgs = vec![Message {
            role: crate::provider::MessageRole::User,
            content: "describe".into(),
        }];
        assert!(p.chat_with_images(&msgs, &[], None).await.is_err());
    }

    // ── is_available with providers ──────────────────────────────────────

    struct AvailableProvider;

    #[async_trait]
    impl AIProvider for AvailableProvider {
        fn name(&self) -> &str { "Available" }
        async fn is_available(&self) -> bool { true }
        async fn complete(&self, _ctx: &CodeContext) -> Result<CompletionResponse> {
            anyhow::bail!("not implemented")
        }
        async fn stream_complete(&self, _ctx: &CodeContext) -> Result<CompletionStream> {
            anyhow::bail!("not implemented")
        }
        async fn chat(&self, _msgs: &[Message], _ctx: Option<String>) -> Result<String> {
            anyhow::bail!("not implemented")
        }
        async fn stream_chat(&self, _msgs: &[Message]) -> Result<CompletionStream> {
            anyhow::bail!("not implemented")
        }
    }

    #[tokio::test]
    async fn is_available_true_if_any_provider_available() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Offline")),
            Arc::new(AvailableProvider),
        ];
        let p = FailoverProvider::new(chain);
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn is_available_false_if_all_unavailable() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("A")),
            Arc::new(MockProvider::new("B")),
        ];
        let p = FailoverProvider::new(chain);
        assert!(!p.is_available().await);
    }

    // ── fallback behavior ───────────────────────────────────────────────

    struct SuccessProvider {
        label: String,
    }

    impl SuccessProvider {
        fn new(label: &str) -> Self {
            Self { label: label.to_string() }
        }
    }

    #[async_trait]
    impl AIProvider for SuccessProvider {
        fn name(&self) -> &str { &self.label }
        async fn is_available(&self) -> bool { true }
        async fn complete(&self, _ctx: &CodeContext) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                text: format!("{}-complete", self.label),
                model: self.label.clone(),
                usage: None,
            })
        }
        async fn stream_complete(&self, _ctx: &CodeContext) -> Result<CompletionStream> {
            anyhow::bail!("not implemented")
        }
        async fn chat(&self, _msgs: &[Message], _ctx: Option<String>) -> Result<String> {
            Ok(format!("{}-chat", self.label))
        }
        async fn stream_chat(&self, _msgs: &[Message]) -> Result<CompletionStream> {
            anyhow::bail!("not implemented")
        }
        async fn chat_response(&self, _msgs: &[Message], _ctx: Option<String>) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                text: format!("{}-chat-response", self.label),
                model: self.label.clone(),
                usage: None,
            })
        }
    }

    #[tokio::test]
    async fn chat_falls_through_to_second_provider() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Failing")),
            Arc::new(SuccessProvider::new("Backup")),
        ];
        let p = FailoverProvider::new(chain);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let result = p.chat(&msgs, None).await.unwrap();
        assert_eq!(result, "Backup-chat");
    }

    #[tokio::test]
    async fn complete_falls_through_to_second_provider() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Failing")),
            Arc::new(SuccessProvider::new("Backup")),
        ];
        let p = FailoverProvider::new(chain);
        let ctx = CodeContext {
            language: "rust".into(),
            file_path: None,
            prefix: "fn ".into(),
            suffix: "".into(),
            additional_context: vec![],
        };
        let result = p.complete(&ctx).await.unwrap();
        assert_eq!(result.text, "Backup-complete");
        assert_eq!(result.model, "Backup");
    }

    #[tokio::test]
    async fn chat_response_falls_through() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Bad")),
            Arc::new(SuccessProvider::new("Good")),
        ];
        let p = FailoverProvider::new(chain);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "test".into() }];
        let result = p.chat_response(&msgs, None).await.unwrap();
        assert_eq!(result.text, "Good-chat-response");
    }

    #[tokio::test]
    async fn first_provider_succeeds_no_fallthrough() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(SuccessProvider::new("Primary")),
            Arc::new(SuccessProvider::new("Secondary")),
        ];
        let p = FailoverProvider::new(chain);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let result = p.chat(&msgs, None).await.unwrap();
        assert_eq!(result, "Primary-chat");
    }

    #[tokio::test]
    async fn all_providers_fail_returns_last_error() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("A")),
            Arc::new(MockProvider::new("B")),
        ];
        let p = FailoverProvider::new(chain);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let err = p.chat(&msgs, None).await.unwrap_err();
        assert_eq!(err.to_string(), "mock");
    }

    #[test]
    fn name_with_many_providers() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("A")),
            Arc::new(MockProvider::new("B")),
            Arc::new(MockProvider::new("C")),
            Arc::new(MockProvider::new("D")),
        ];
        let p = FailoverProvider::new(chain);
        assert_eq!(p.name(), "Failover(A -> B -> C -> D)");
    }

    #[tokio::test]
    async fn is_available_true_when_first_available() {
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(AvailableProvider),
            Arc::new(MockProvider::new("Offline")),
        ];
        let p = FailoverProvider::new(chain);
        assert!(p.is_available().await);
    }

    // ── health-aware failover tests ─────────────────────────────────────

    #[tokio::test]
    async fn health_aware_failover_prefers_healthy_provider() {
        let tracker = Arc::new(ProviderHealthTracker::new(50, std::time::Duration::from_secs(600)));

        // Record "Backup" as healthy, "Primary" as unhealthy
        for _ in 0..5 {
            tracker.record(ProviderCallOutcome {
                provider_name: "Backup".to_string(),
                success: true,
                latency: std::time::Duration::from_millis(100),
                timestamp: Instant::now(),
                error_category: None,
            });
            tracker.record(ProviderCallOutcome {
                provider_name: "Primary".to_string(),
                success: false,
                latency: std::time::Duration::from_millis(5000),
                timestamp: Instant::now(),
                error_category: Some(crate::resilience::FailureCategory::ServerError),
            });
        }

        // Both succeed, but health tracker should prefer "Backup"
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(SuccessProvider::new("Primary")),
            Arc::new(SuccessProvider::new("Backup")),
        ];
        let p = FailoverProvider::with_health_tracker(chain, tracker);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let result = p.chat(&msgs, None).await.unwrap();
        // Should try "Backup" first since it has higher health score
        assert_eq!(result, "Backup-chat");
    }

    #[tokio::test]
    async fn health_tracker_records_outcomes_on_call() {
        let tracker = Arc::new(ProviderHealthTracker::new(50, std::time::Duration::from_secs(600)));
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(MockProvider::new("Failing")),
            Arc::new(SuccessProvider::new("Working")),
        ];
        let p = FailoverProvider::with_health_tracker(chain, tracker.clone());
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let _ = p.chat(&msgs, None).await;

        // Tracker should have recorded both outcomes
        let failing_health = tracker.health("Failing");
        assert_eq!(failing_health.total_calls, 1);
        assert_eq!(failing_health.recent_failures, 1);

        let working_health = tracker.health("Working");
        assert_eq!(working_health.total_calls, 1);
        assert_eq!(working_health.recent_failures, 0);
    }

    #[tokio::test]
    async fn without_tracker_uses_original_order() {
        // Backward compat: FailoverProvider::new() with no tracker
        let chain: Vec<Arc<dyn AIProvider>> = vec![
            Arc::new(SuccessProvider::new("First")),
            Arc::new(SuccessProvider::new("Second")),
        ];
        let p = FailoverProvider::new(chain);
        let msgs = vec![Message { role: crate::provider::MessageRole::User, content: "hi".into() }];
        let result = p.chat(&msgs, None).await.unwrap();
        assert_eq!(result, "First-chat"); // always tries first in chain
    }
}
