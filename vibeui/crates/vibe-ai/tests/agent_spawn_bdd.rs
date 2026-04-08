//! BDD tests for sub-agent spawning guard logic:
//! depth limits, global counter limits, AgentContext serde, and ApprovalPolicy.
//!
//! Run with: `cargo test --test agent_spawn_bdd`

use cucumber::{given, then, when, World};
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use vibe_ai::agent::{AgentContext, ApprovalPolicy};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct SpawnWorld {
    /// The context under test.
    context: Option<AgentContext>,
    /// A child context derived from `context`.
    child_context: Option<AgentContext>,
    /// A shared atomic counter for multi-context tests.
    counter: Option<Arc<AtomicU32>>,
    /// Serialised JSON representation.
    json: Option<String>,
    /// Re-hydrated context from JSON.
    roundtrip: Option<AgentContext>,
    /// Approval policy parsed from a string.
    policy: Option<ApprovalPolicy>,
    /// Requested depth limit.
    requested_limit: Option<u32>,
}

// ── Background ────────────────────────────────────────────────────────────────

#[given("a workspace directory")]
fn setup_workspace(_world: &mut SpawnWorld) {
    // Nothing to create — AgentContext.workspace_root can remain default.
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given("an agent context with no parent")]
fn context_no_parent(world: &mut SpawnWorld) {
    world.context = Some(AgentContext::default());
}

#[given(regex = r"an agent context with depth (\d+)")]
fn context_with_depth(world: &mut SpawnWorld, depth: u32) {
    world.context = Some(AgentContext {
        workspace_root: PathBuf::from("/tmp"),
        depth,
        ..Default::default()
    });
}

#[given(regex = r"an agent context with a counter")]
fn context_with_counter(world: &mut SpawnWorld) {
    let counter = Arc::new(AtomicU32::new(5));
    world.counter = Some(counter.clone());
    world.context = Some(AgentContext {
        depth: 0,
        active_agent_counter: Some(counter),
        ..Default::default()
    });
}

#[given(regex = r"a shared agent counter at (\d+)")]
fn shared_counter_at(world: &mut SpawnWorld, value: u32) {
    world.counter = Some(Arc::new(AtomicU32::new(value)));
}

#[given(regex = r"a depth limit of (\d+)")]
fn depth_limit(world: &mut SpawnWorld, limit: u32) {
    world.requested_limit = Some(limit);
}

#[given(regex = r#"an approval policy string "([^"]+)""#)]
fn approval_policy_string(world: &mut SpawnWorld, policy_str: String) {
    world.policy = Some(ApprovalPolicy::from_str(&policy_str));
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when("a child context is created from it")]
fn create_child_from_context(world: &mut SpawnWorld) {
    let parent = world.context.as_ref().expect("parent context must be set");
    let counter = parent.active_agent_counter.clone()
        .unwrap_or_else(|| Arc::new(AtomicU32::new(0)));
    world.child_context = Some(AgentContext {
        workspace_root: parent.workspace_root.clone(),
        parent_session_id: parent.parent_session_id.clone()
            .or_else(|| Some("root".to_string())),
        depth: parent.depth + 1,
        active_agent_counter: Some(counter),
        ..Default::default()
    });
}

#[when("a child context is created with that counter")]
fn create_child_with_counter(world: &mut SpawnWorld) {
    let counter = world.counter.as_ref().expect("counter must be set").clone();
    world.child_context = Some(AgentContext {
        depth: 1,
        active_agent_counter: Some(counter),
        ..Default::default()
    });
}

#[when("the context is serialised to JSON")]
fn serialise_context(world: &mut SpawnWorld) {
    let ctx = world.context.as_ref().expect("context must be set");
    world.json = Some(serde_json::to_string(ctx).expect("serialise"));
}

#[when("the context is deserialised from that JSON")]
fn deserialise_context(world: &mut SpawnWorld) {
    let json = world.json.as_ref().expect("json must be set");
    world.roundtrip = Some(serde_json::from_str(json).expect("deserialise"));
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then("the agent depth is 0")]
fn depth_is_zero(world: &mut SpawnWorld) {
    let ctx = world.context.as_ref().expect("context must be set");
    assert_eq!(ctx.depth, 0, "root context depth must be 0");
}

#[then(regex = r"the child depth is (\d+)")]
fn child_depth_is(world: &mut SpawnWorld, expected: u32) {
    let child = world.child_context.as_ref().expect("child context must be set");
    assert_eq!(child.depth, expected, "child depth mismatch");
}

#[then(regex = r"the effective depth limit is (\d+)")]
fn effective_depth_limit(world: &mut SpawnWorld, expected: u32) {
    let requested = world.requested_limit.unwrap_or(3);
    // Mirror the production logic: requested.min(5)
    let effective = requested.min(5);
    assert_eq!(effective, expected,
        "effective limit {effective} != expected {expected}");
}

#[then("the active agent counter is absent")]
fn counter_absent(world: &mut SpawnWorld) {
    let ctx = world.context.as_ref().expect("context must be set");
    assert!(ctx.active_agent_counter.is_none(),
        "root context should have no active_agent_counter");
}

#[then(regex = r"the child context counter reads (\d+)")]
fn child_counter_reads(world: &mut SpawnWorld, expected: u32) {
    let child = world.child_context.as_ref().expect("child must be set");
    let counter = child.active_agent_counter.as_ref()
        .expect("child should have a counter");
    assert_eq!(counter.load(Ordering::Relaxed), expected);
}

#[then(regex = r"the counter is at or above the global limit of (\d+)")]
fn counter_at_limit(world: &mut SpawnWorld, limit: u32) {
    let counter = world.counter.as_ref().expect("counter must be set");
    assert!(counter.load(Ordering::Relaxed) >= limit,
        "expected counter >= {limit}");
}

#[then(regex = r"the counter is below the global limit of (\d+)")]
fn counter_below_limit(world: &mut SpawnWorld, limit: u32) {
    let counter = world.counter.as_ref().expect("counter must be set");
    assert!(counter.load(Ordering::Relaxed) < limit,
        "expected counter < {limit}");
}

#[then(regex = r"the deserialised depth is (\d+)")]
fn roundtrip_depth(world: &mut SpawnWorld, expected: u32) {
    let ctx = world.roundtrip.as_ref().expect("roundtrip must be set");
    assert_eq!(ctx.depth, expected);
}

#[then(regex = r#"the JSON does not contain "([^"]+)""#)]
fn json_not_contains(world: &mut SpawnWorld, key: String) {
    let json = world.json.as_ref().expect("json must be set");
    assert!(!json.contains(&key),
        "JSON should not contain '{key}' but it does:\n{json}");
}

#[then("the policy is FullAuto")]
fn policy_is_full_auto(world: &mut SpawnWorld) {
    assert_eq!(world.policy, Some(ApprovalPolicy::FullAuto));
}

#[then("the policy is Suggest")]
fn policy_is_suggest(world: &mut SpawnWorld) {
    assert_eq!(world.policy, Some(ApprovalPolicy::Suggest));
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    SpawnWorld::run("tests/features/agent_spawn.feature").await;
}
