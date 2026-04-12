/*!
 * BDD tests for trust_resolution using Cucumber.
 * Run with: cargo test --test trust_resolution_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::trust_resolution::{
    ContentSource, ContentTrustResolver, TrustDecision, TrustEvent, TrustLevel, TrustPolicy,
    TrustResolver,
};
use std::path::PathBuf;

// ── World for content-source BDD scenarios ────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct TrWorld {
    workspace: String,
    path: String,
    decision: Option<TrustDecision>,
    // policy resolver state
    resolver: Option<TrustResolver>,
    resolved_policy: Option<TrustPolicy>,
    prompt_result: Option<bool>,
}

fn content_resolver_for(workspace: &str) -> ContentTrustResolver {
    ContentTrustResolver::new(workspace)
}

fn make_policy_resolver() -> TrustResolver {
    TrustResolver::new(PathBuf::from("/tmp/test-trust.json"))
}

// ── Content-source steps ──────────────────────────────────────────────────────

#[given(expr = "a workspace at {string}")]
fn set_workspace(world: &mut TrWorld, ws: String) {
    world.workspace = ws;
}

#[given(expr = "a local file at {string}")]
fn set_path(world: &mut TrWorld, path: String) {
    world.path = path;
}

#[when("I resolve trust for the file")]
fn resolve_file(world: &mut TrWorld) {
    let r = content_resolver_for(&world.workspace);
    world.decision = Some(r.resolve(ContentSource::LocalFile { path: world.path.clone() }));
}

#[when("I resolve trust for a remote URL")]
fn resolve_remote(world: &mut TrWorld) {
    let r = content_resolver_for(&world.workspace);
    world.decision = Some(r.resolve(ContentSource::RemoteUrl { url: "https://example.com/x".into() }));
}

#[when("I resolve trust for agent-generated content")]
fn resolve_agent(world: &mut TrWorld) {
    let r = content_resolver_for(&world.workspace);
    world.decision = Some(r.resolve(ContentSource::AgentGenerated));
}

#[then(expr = "the trust level should be {string}")]
fn check_level(world: &mut TrWorld, expected: String) {
    let level = world.decision.as_ref().unwrap().level.to_string();
    assert_eq!(level, expected, "trust level mismatch");
}

#[then("execution should be allowed")]
fn check_exec_allowed(world: &mut TrWorld) {
    assert!(world.decision.as_ref().unwrap().allow_execution);
}

#[then("execution should not be allowed")]
fn check_exec_denied(world: &mut TrWorld) {
    assert!(!world.decision.as_ref().unwrap().allow_execution);
}

#[then("the decision should be trusted")]
fn check_trusted(world: &mut TrWorld) {
    assert!(world.decision.as_ref().unwrap().is_trusted());
}

#[then("the decision should not be trusted")]
fn check_not_trusted(world: &mut TrWorld) {
    assert!(!world.decision.as_ref().unwrap().is_trusted());
}

// ── Policy resolver steps (TrustPolicy / TrustResolver) ──────────────────────

#[given(expr = "text {string}")]
fn set_text(world: &mut TrWorld, text: String) {
    world.prompt_result = Some(TrustResolver::is_trust_prompt(&text));
}

#[given(expr = "an allowed path {string}")]
fn add_allowed(world: &mut TrWorld, path: String) {
    let r = world.resolver.get_or_insert_with(make_policy_resolver);
    r.add_allowed(&path);
}

#[given(expr = "a denied path {string}")]
fn add_denied(world: &mut TrWorld, path: String) {
    let r = world.resolver.get_or_insert_with(make_policy_resolver);
    r.add_denied(&path);
}

#[given("a trust resolver")]
fn new_resolver(world: &mut TrWorld) {
    world.resolver = Some(make_policy_resolver());
}

#[when(expr = "I resolve {string}")]
fn resolve_path(world: &mut TrWorld, path: String) {
    let r = world.resolver.get_or_insert_with(make_policy_resolver);
    world.resolved_policy = Some(r.resolve(&path));
}

#[when(expr = "I record {int} trust events")]
fn record_events(world: &mut TrWorld, n: usize) {
    let r = world.resolver.get_or_insert_with(make_policy_resolver);
    for i in 0..n {
        r.record_event(TrustEvent::new(
            &format!("/path/{i}"),
            TrustPolicy::AutoTrust,
            "test",
        ));
    }
}

#[then("is_trust_prompt should return true")]
fn check_prompt_true(world: &mut TrWorld) {
    assert!(world.prompt_result.unwrap_or(false));
}

#[then(expr = "the policy should be {string}")]
fn check_policy(world: &mut TrWorld, expected: String) {
    let policy = world.resolved_policy.as_ref().unwrap().to_string();
    assert_eq!(policy, expected);
}

#[then(expr = "the policy should not be {string}")]
fn check_policy_not(world: &mut TrWorld, not_expected: String) {
    let policy = world.resolved_policy.as_ref().unwrap().to_string();
    assert_ne!(policy, not_expected);
}

#[then(expr = "the event log should contain {int} entries")]
fn check_event_count(world: &mut TrWorld, expected: usize) {
    assert_eq!(world.resolver.as_ref().unwrap().events.len(), expected);
}

fn main() {
    futures::executor::block_on(TrWorld::run("tests/features/trust_resolution.feature"));
}
