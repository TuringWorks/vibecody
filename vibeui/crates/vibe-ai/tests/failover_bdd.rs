/*!
 * BDD tests for FailoverProvider using Cucumber.
 *
 * Run with:
 *   cargo test --test failover_bdd -p vibe-ai
 */

use cucumber::{World, given, then, when};
use std::sync::Arc;
use std::time::Instant;
use vibe_ai::providers::FailoverProvider;
use vibe_ai::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, MessageRole};
use vibe_ai::resilience::{ProviderHealthTracker, ProviderCallOutcome, FailureCategory};
use anyhow::Result;
use async_trait::async_trait;

// ── Mock providers ─────────────────────────────────────────────────────────────

struct AlwaysFailProvider { name_: String }
impl AlwaysFailProvider {
    fn new(name: &str) -> Self { Self { name_: name.to_string() } }
}
#[async_trait]
impl AIProvider for AlwaysFailProvider {
    fn name(&self) -> &str { &self.name_ }
    async fn is_available(&self) -> bool { false }
    async fn complete(&self, _: &CodeContext) -> Result<CompletionResponse> { anyhow::bail!("mock") }
    async fn stream_complete(&self, _: &CodeContext) -> Result<CompletionStream> { anyhow::bail!("mock") }
    async fn chat(&self, _: &[Message], _: Option<String>) -> Result<String> { anyhow::bail!("mock") }
    async fn stream_chat(&self, _: &[Message]) -> Result<CompletionStream> { anyhow::bail!("mock") }
}

struct AlwaysSucceedProvider { label: String }
impl AlwaysSucceedProvider {
    fn new(label: &str) -> Self { Self { label: label.to_string() } }
}
#[async_trait]
impl AIProvider for AlwaysSucceedProvider {
    fn name(&self) -> &str { &self.label }
    async fn is_available(&self) -> bool { true }
    async fn complete(&self, _: &CodeContext) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: format!("{}-complete", self.label),
            model: self.label.clone(),
            usage: None,
        })
    }
    async fn stream_complete(&self, _: &CodeContext) -> Result<CompletionStream> { anyhow::bail!("not impl") }
    async fn chat(&self, _: &[Message], _: Option<String>) -> Result<String> {
        Ok(format!("{}-chat", self.label))
    }
    async fn stream_chat(&self, _: &[Message]) -> Result<CompletionStream> { anyhow::bail!("not impl") }
    async fn chat_response(&self, _: &[Message], _: Option<String>) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: format!("{}-chat-response", self.label),
            model: self.label.clone(),
            usage: None,
        })
    }
}

struct UnavailableProvider;
#[async_trait]
impl AIProvider for UnavailableProvider {
    fn name(&self) -> &str { "Unavailable" }
    async fn is_available(&self) -> bool { false }
    async fn complete(&self, _: &CodeContext) -> Result<CompletionResponse> { anyhow::bail!("unavailable") }
    async fn stream_complete(&self, _: &CodeContext) -> Result<CompletionStream> { anyhow::bail!("unavailable") }
    async fn chat(&self, _: &[Message], _: Option<String>) -> Result<String> { anyhow::bail!("unavailable") }
    async fn stream_chat(&self, _: &[Message]) -> Result<CompletionStream> { anyhow::bail!("unavailable") }
}

struct AvailableProvider;
#[async_trait]
impl AIProvider for AvailableProvider {
    fn name(&self) -> &str { "Available" }
    async fn is_available(&self) -> bool { true }
    async fn complete(&self, _: &CodeContext) -> Result<CompletionResponse> { anyhow::bail!("no impl") }
    async fn stream_complete(&self, _: &CodeContext) -> Result<CompletionStream> { anyhow::bail!("no impl") }
    async fn chat(&self, _: &[Message], _: Option<String>) -> Result<String> { anyhow::bail!("no impl") }
    async fn stream_chat(&self, _: &[Message]) -> Result<CompletionStream> { anyhow::bail!("no impl") }
}

// ── World ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
#[world(init = Self::new)]
pub struct FailoverWorld {
    failover: Option<FailoverProvider>,
    tracker: Option<Arc<ProviderHealthTracker>>,
    last_chat_result: Option<Result<String>>,
    last_complete_result: Option<Result<CompletionResponse>>,
    last_chat_response_result: Option<Result<CompletionResponse>>,
    available_result: Option<bool>,
}

impl FailoverWorld {
    fn new() -> Self { Self::default() }

    fn msgs() -> Vec<Message> {
        vec![Message { role: MessageRole::User, content: "hi".into() }]
    }

    fn ctx() -> CodeContext {
        CodeContext { language: "rust".into(), file_path: None, prefix: "fn ".into(), suffix: "".into(), additional_context: vec![] }
    }
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(expr = "a failover chain with {int} providers")]
fn empty_chain(world: &mut FailoverWorld, count: usize) {
    assert_eq!(count, 0, "only 0-provider scenario is covered by this step");
    world.failover = Some(FailoverProvider::new(vec![]));
}

#[given(expr = "a failover chain with providers {string}")]
fn chain_from_names(world: &mut FailoverWorld, names_json: String) {
    // Parse ["A","B"] → names
    let names: Vec<String> = names_json
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let chain: Vec<Arc<dyn AIProvider>> = names.iter()
        .map(|n| Arc::new(AlwaysSucceedProvider::new(n)) as Arc<dyn AIProvider>)
        .collect();
    world.failover = Some(FailoverProvider::new(chain));
}

#[given("a failover chain where all providers are unavailable")]
fn all_unavailable(world: &mut FailoverWorld) {
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(UnavailableProvider),
        Arc::new(UnavailableProvider),
    ];
    world.failover = Some(FailoverProvider::new(chain));
}

#[given("a failover chain where only the second provider is available")]
fn second_available(world: &mut FailoverWorld) {
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(UnavailableProvider),
        Arc::new(AvailableProvider),
    ];
    world.failover = Some(FailoverProvider::new(chain));
}

#[given(expr = "a failover chain where the first provider fails and the second succeeds as {string}")]
fn first_fails_second_succeeds(world: &mut FailoverWorld, backup_name: String) {
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(AlwaysFailProvider::new("Failing")),
        Arc::new(AlwaysSucceedProvider::new(&backup_name)),
    ];
    world.failover = Some(FailoverProvider::new(chain));
}

#[given(expr = "a failover chain where the first provider succeeds as {string} and second as {string}")]
fn both_succeed(world: &mut FailoverWorld, first: String, second: String) {
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(AlwaysSucceedProvider::new(&first)),
        Arc::new(AlwaysSucceedProvider::new(&second)),
    ];
    world.failover = Some(FailoverProvider::new(chain));
}

#[given("a failover chain where all providers fail with message \"mock\"")]
fn all_fail(world: &mut FailoverWorld) {
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(AlwaysFailProvider::new("A")),
        Arc::new(AlwaysFailProvider::new("B")),
    ];
    world.failover = Some(FailoverProvider::new(chain));
}

#[given(expr = "a failover chain with providers {string} and no health tracker")]
fn chain_no_tracker(world: &mut FailoverWorld, names_json: String) {
    chain_from_names(world, names_json);
}

#[given(expr = "a failover chain with providers {string} and a health tracker")]
fn chain_with_tracker(world: &mut FailoverWorld, names_json: String) {
    let names: Vec<String> = names_json
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let tracker = Arc::new(ProviderHealthTracker::new(50, std::time::Duration::from_secs(600)));
    let chain: Vec<Arc<dyn AIProvider>> = names.iter()
        .map(|n| Arc::new(AlwaysSucceedProvider::new(n)) as Arc<dyn AIProvider>)
        .collect();
    world.failover = Some(FailoverProvider::with_health_tracker(chain, tracker.clone()));
    world.tracker = Some(tracker);
}

#[given(expr = "{string} has {int} successful calls recorded and {string} has {int} failed calls")]
fn record_health(world: &mut FailoverWorld, healthy: String, ok_count: usize, unhealthy: String, fail_count: usize) {
    let tracker = world.tracker.as_ref().expect("tracker must be set first");
    for _ in 0..ok_count {
        tracker.record(ProviderCallOutcome {
            provider_name: healthy.clone(),
            success: true,
            latency: std::time::Duration::from_millis(100),
            timestamp: Instant::now(),
            error_category: None,
        });
    }
    for _ in 0..fail_count {
        tracker.record(ProviderCallOutcome {
            provider_name: unhealthy.clone(),
            success: false,
            latency: std::time::Duration::from_millis(3000),
            timestamp: Instant::now(),
            error_category: Some(FailureCategory::ServerError),
        });
    }
}

#[given(expr = "when both providers succeed and chat is called")]
async fn both_succeed_chat(_world: &mut FailoverWorld) {
    // intentional no-op; handled by the When step
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when("chat is called")]
async fn call_chat(world: &mut FailoverWorld) {
    let fp = world.failover.take().expect("failover not set");
    world.last_chat_result = Some(fp.chat(&FailoverWorld::msgs(), None).await);
    world.failover = Some(fp);
}

#[when("complete is called")]
async fn call_complete(world: &mut FailoverWorld) {
    let fp = world.failover.take().expect("failover not set");
    world.last_complete_result = Some(fp.complete(&FailoverWorld::ctx()).await);
    world.failover = Some(fp);
}

#[when("chat_response is called")]
async fn call_chat_response(world: &mut FailoverWorld) {
    let fp = world.failover.take().expect("failover not set");
    world.last_chat_response_result = Some(fp.chat_response(&FailoverWorld::msgs(), None).await);
    world.failover = Some(fp);
}

#[when("both providers succeed and chat is called")]
async fn both_succeed_and_call_chat(world: &mut FailoverWorld) {
    call_chat(world).await;
}

#[when("the first provider fails and the second succeeds on a chat call")]
async fn first_fail_second_succeed_chat(world: &mut FailoverWorld) {
    let tracker = world.tracker.as_ref().cloned().expect("tracker must be set");
    let chain: Vec<Arc<dyn AIProvider>> = vec![
        Arc::new(AlwaysFailProvider::new("Failing")),
        Arc::new(AlwaysSucceedProvider::new("Working")),
    ];
    let fp = FailoverProvider::with_health_tracker(chain, tracker);
    world.failover = Some(fp);
    call_chat(world).await;
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(expr = "the provider name should be {string}")]
fn assert_name(world: &mut FailoverWorld, expected: String) {
    let name = world.failover.as_ref().expect("failover not set").name().to_string();
    assert_eq!(name, expected, "name mismatch");
}

#[then("is_available should return false")]
async fn assert_not_available(world: &mut FailoverWorld) {
    let available = world.failover.as_ref().expect("failover not set").is_available().await;
    assert!(!available, "expected not available");
}

#[then("is_available should return true")]
async fn assert_available(world: &mut FailoverWorld) {
    let available = world.failover.as_ref().expect("failover not set").is_available().await;
    assert!(available, "expected available");
}

#[then(expr = "it should return an error containing {string}")]
fn assert_error_contains(world: &mut FailoverWorld, fragment: String) {
    let result = world.last_chat_result.as_ref().expect("no chat result");
    assert!(result.is_err(), "expected error but got ok");
    assert!(
        result.as_ref().unwrap_err().to_string().contains(&fragment),
        "error {:?} does not contain {:?}",
        result.as_ref().unwrap_err().to_string(),
        fragment
    );
}

#[then(expr = "the response should contain {string}")]
fn assert_response_contains(world: &mut FailoverWorld, fragment: String) {
    let result = world.last_chat_result.as_ref().expect("no chat result");
    assert!(result.is_ok(), "expected ok but got {:?}", result);
    assert!(
        result.as_ref().unwrap().contains(&fragment),
        "response {:?} does not contain {:?}",
        result.as_ref().unwrap(),
        fragment
    );
}

#[then(expr = "the completion text should contain {string}")]
fn assert_completion_contains(world: &mut FailoverWorld, fragment: String) {
    let result = world.last_complete_result.as_ref().expect("no complete result");
    assert!(result.is_ok(), "expected ok but got {:?}", result);
    assert!(
        result.as_ref().unwrap().text.contains(&fragment),
        "text {:?} does not contain {:?}",
        result.as_ref().unwrap().text,
        fragment
    );
}

#[then(expr = "the response text should contain {string}")]
fn assert_chat_response_contains(world: &mut FailoverWorld, fragment: String) {
    let result = world.last_chat_response_result.as_ref().expect("no chat_response result");
    assert!(result.is_ok(), "expected ok but got {:?}", result);
    assert!(
        result.as_ref().unwrap().text.contains(&fragment),
        "text {:?} does not contain {:?}",
        result.as_ref().unwrap().text,
        fragment
    );
}

#[then(expr = "the tracker should record {int} failure for {string}")]
fn assert_tracker_failures(world: &mut FailoverWorld, count: usize, provider: String) {
    let tracker = world.tracker.as_ref().expect("no tracker");
    let health = tracker.health(&provider);
    assert_eq!(
        health.recent_failures, count,
        "{provider} recent_failures: expected {count} got {}", health.recent_failures
    );
}

#[then(expr = "the tracker should record {int} success for {string}")]
fn assert_tracker_successes(world: &mut FailoverWorld, count: usize, provider: String) {
    let tracker = world.tracker.as_ref().expect("no tracker");
    let health = tracker.health(&provider);
    let successes = health.total_calls - health.recent_failures;
    assert_eq!(
        successes, count,
        "{provider} successes: expected {count} got {successes}"
    );
}

// ── Runner ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    FailoverWorld::run("tests/features/failover.feature").await;
}
