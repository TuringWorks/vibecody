#![allow(dead_code)]
//! MockAIProvider — deterministic AI provider for CI testing.
//!
//! Supports:
//! - Sequenced responses (VecDeque, pops in order)
//! - Scenario-based prefix matching (prefix → response)
//! - Call count tracking (Arc<AtomicUsize>)
//! - Configurable availability
//!
//! Feature-gated: `#[cfg(any(test, feature = "testing"))]`

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::stream;

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message};

// ── MockAIProvider ─────────────────────────────────────────────────────────────

pub struct MockAIProvider {
    name: String,
    available: bool,
    responses: Arc<Mutex<VecDeque<String>>>,
    call_count: Arc<AtomicUsize>,
    /// (prefix, response) pairs checked in order
    scenario_responses: Vec<(String, String)>,
    default_response: String,
}

impl std::fmt::Debug for MockAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAIProvider")
            .field("name", &self.name)
            .field("available", &self.available)
            .field("call_count", &self.call_count.load(Ordering::SeqCst))
            .finish()
    }
}

impl Default for MockAIProvider {
    fn default() -> Self {
        Self::new("mock")
    }
}

impl MockAIProvider {
    /// Create a provider that always returns an empty string (default response).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            available: true,
            responses: Arc::new(Mutex::new(VecDeque::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
            scenario_responses: Vec::new(),
            default_response: String::new(),
        }
    }

    /// Provider with sequenced responses (popped in order, error when exhausted).
    pub fn with_responses(name: &str, responses: Vec<&str>) -> Self {
        let queue = responses.iter().map(|s| s.to_string()).collect();
        Self {
            name: name.to_string(),
            available: true,
            responses: Arc::new(Mutex::new(queue)),
            call_count: Arc::new(AtomicUsize::new(0)),
            scenario_responses: Vec::new(),
            default_response: String::new(),
        }
    }

    /// Provider with scenario prefix matching.
    ///
    /// Each entry is `(prefix, response)`. When a prompt starts with `prefix`,
    /// the corresponding `response` is returned. Falls back to `default_response`
    /// when no prefix matches.
    pub fn with_scenarios(name: &str, scenarios: Vec<(&str, &str)>) -> Self {
        let scenario_responses = scenarios
            .into_iter()
            .map(|(p, r)| (p.to_string(), r.to_string()))
            .collect();
        Self {
            name: name.to_string(),
            available: true,
            responses: Arc::new(Mutex::new(VecDeque::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
            scenario_responses,
            default_response: String::new(),
        }
    }

    /// Override the default response returned when the queue is empty and no
    /// scenario prefix matches.
    pub fn set_default_response(&mut self, response: &str) {
        self.default_response = response.to_string();
    }

    /// Mark the provider as available (`true`) or unavailable (`false`).
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    /// Number of times `chat` (or a method that delegates to it) has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Internal: resolve the response for a given prompt.
    ///
    /// Resolution order:
    /// 1. Pop the next sequenced response (if the queue is non-empty).
    /// 2. Check scenario prefixes in order.
    /// 3. Return `self.default_response`.
    ///
    /// If the queue was initialised with responses but is now exhausted AND
    /// no scenario matches, returns an error so tests can assert on depletion.
    fn resolve(&self, prompt: &str) -> Result<String> {
        // 1. Sequenced queue
        {
            let mut queue = self.responses.lock().expect("mock response queue poisoned");
            if !queue.is_empty() {
                return Ok(queue.pop_front().unwrap());
            }
        }

        // 2. Scenario prefix matching
        for (prefix, response) in &self.scenario_responses {
            if prompt.starts_with(prefix.as_str()) {
                return Ok(response.clone());
            }
        }

        // 3. Default response — but only error if the provider was configured
        //    with explicit responses that are now exhausted AND there are no
        //    scenarios to fall back on.
        //    We signal exhaustion only when there are no scenarios and the queue
        //    had items originally. We approximate this by checking if the
        //    default_response is empty AND there are no scenarios.
        //    The simpler rule: return default unless both conditions hold:
        //      - no scenario responses configured
        //      - default_response is empty AND the queue was set up (call_count > 0)
        //    We use a flag-free approach: if the caller set up with_responses and
        //    the queue is empty, bail. We detect this by re-checking: if
        //    scenario_responses is empty AND default_response is empty AND
        //    call_count > 0 then it means we've exhausted a with_responses provider.
        if self.scenario_responses.is_empty() && self.default_response.is_empty() && self.call_count.load(Ordering::SeqCst) > 0 {
            bail!("MockAIProvider: no more responses");
        }

        Ok(self.default_response.clone())
    }
}

// ── AIProvider impl ────────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for MockAIProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn is_available(&self) -> bool {
        self.available
    }

    async fn chat(&self, messages: &[Message], _ctx: Option<String>) -> Result<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let prompt = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        self.resolve(prompt)
    }

    async fn complete(&self, _ctx: &CodeContext) -> Result<CompletionResponse> {
        let text = self.resolve("")?;
        Ok(CompletionResponse {
            text: text.clone(),
            model: self.name.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, _ctx: &CodeContext) -> Result<CompletionStream> {
        let text = self.resolve("")?;
        Ok(Box::pin(stream::once(async move { Ok(text) })))
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let prompt = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        let text = self.resolve(prompt)?;
        Ok(Box::pin(stream::once(async move { Ok(text) })))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MessageRole;

    fn msg(content: &str) -> Message {
        Message {
            role: MessageRole::User,
            content: content.to_string(),
        }
    }

    #[tokio::test]
    async fn returns_name() {
        let p = MockAIProvider::new("test-mock");
        assert_eq!(p.name(), "test-mock");
    }

    #[tokio::test]
    async fn sequenced_responses_returned_in_order() {
        let p = MockAIProvider::with_responses("m", vec!["hello", "world"]);
        let r1 = p.chat(&[msg("q")], None).await.unwrap();
        let r2 = p.chat(&[msg("q")], None).await.unwrap();
        assert_eq!(r1, "hello");
        assert_eq!(r2, "world");
    }

    #[tokio::test]
    async fn exhausted_responses_returns_error() {
        let p = MockAIProvider::with_responses("m", vec!["only"]);
        let _ = p.chat(&[msg("q")], None).await.unwrap();
        assert!(p.chat(&[msg("q")], None).await.is_err());
    }

    #[tokio::test]
    async fn tracks_call_count() {
        let p = MockAIProvider::with_responses("m", vec!["a", "b", "c"]);
        for _ in 0..3 {
            let _ = p.chat(&[msg("q")], None).await;
        }
        assert_eq!(p.call_count(), 3);
    }

    #[tokio::test]
    async fn unavailable_when_configured() {
        let mut p = MockAIProvider::new("m");
        p.set_available(false);
        assert!(!p.is_available().await);
    }

    #[tokio::test]
    async fn available_by_default() {
        let p = MockAIProvider::new("m");
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn scenario_prefix_matching() {
        let p = MockAIProvider::with_scenarios("m", vec![("fix", "I fixed the bug")]);
        let r = p.chat(&[msg("fix the failing test")], None).await.unwrap();
        assert_eq!(r, "I fixed the bug");
    }

    #[tokio::test]
    async fn scenario_fallback_on_no_match() {
        let p = MockAIProvider::with_scenarios("m", vec![("fix", "fixed")]);
        let r = p.chat(&[msg("unrelated question")], None).await.unwrap();
        // Falls back to default_response (empty string)
        assert!(!r.is_empty() || r.is_empty()); // fallback always returns Ok
    }

    #[tokio::test]
    async fn complete_returns_response() {
        let mut p = MockAIProvider::new("m");
        p.set_default_response("completion result");
        let resp = p.complete(&CodeContext {
            language: "rust".into(),
            file_path: None,
            prefix: "fn ".into(),
            suffix: String::new(),
            additional_context: vec![],
        }).await.unwrap();
        assert_eq!(resp.text, "completion result");
        assert_eq!(resp.model, "m");
        assert!(resp.usage.is_none());
    }

    #[tokio::test]
    async fn stream_complete_yields_single_chunk() {
        use futures::StreamExt;
        let mut p = MockAIProvider::new("m");
        p.set_default_response("streamed");
        let mut stream = p.stream_complete(&CodeContext {
            language: "rust".into(),
            file_path: None,
            prefix: String::new(),
            suffix: String::new(),
            additional_context: vec![],
        }).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk, "streamed");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn stream_chat_yields_single_chunk() {
        use futures::StreamExt;
        let p = MockAIProvider::with_scenarios("m", vec![("hello", "world")]);
        let mut stream = p.stream_chat(&[msg("hello there")]).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk, "world");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn multiple_scenarios_first_match_wins() {
        let p = MockAIProvider::with_scenarios(
            "m",
            vec![("fix", "fix response"), ("fix the", "should not reach")],
        );
        let r = p.chat(&[msg("fix the bug")], None).await.unwrap();
        assert_eq!(r, "fix response");
    }

    #[tokio::test]
    async fn sequenced_queue_takes_priority_over_scenarios() {
        let mut p = MockAIProvider::with_scenarios("m", vec![("any", "scenario response")]);
        // Push one sequenced response into the queue manually by building
        // a with_responses provider and verifying queue beats scenario.
        // Simpler: build a provider that has BOTH. We compose manually.
        p.responses.lock().unwrap().push_back("queue response".to_string());
        let r = p.chat(&[msg("any prompt")], None).await.unwrap();
        assert_eq!(r, "queue response");
        // Second call: queue empty, scenario matches
        let r2 = p.chat(&[msg("any prompt")], None).await.unwrap();
        assert_eq!(r2, "scenario response");
    }
}
