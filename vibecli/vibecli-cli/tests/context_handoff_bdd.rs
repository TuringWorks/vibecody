/*!
 * BDD tests for context_handoff using Cucumber.
 * Run with: cargo test --test context_handoff_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::context_handoff::{
    HandoffContext, HandoffHistory, HandoffMessage, HandoffReason, ToolDefinition,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct ChWorld {
    /// The primary context being built or tested.
    ctx: Option<HandoffContext>,
    /// Serialized JSON of `ctx`.
    serialized: Option<String>,
    /// The restored context after deserialization.
    restored: Option<HandoffContext>,
    /// A cloned context re-targeted at a different provider.
    routed: Option<HandoffContext>,
    /// The handoff history under test.
    history: HandoffHistory,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a context from provider {string} with system prompt {string}")]
fn given_ctx_with_system(world: &mut ChWorld, provider: String, system: String) {
    world.ctx = Some(HandoffContext::new(provider).with_system(system));
}

#[given(expr = "a context from provider {string} with no system prompt")]
fn given_ctx_no_system(world: &mut ChWorld, provider: String) {
    world.ctx = Some(HandoffContext::new(provider));
}

#[given(expr = "a user message {string}")]
fn given_user_message(world: &mut ChWorld, text: String) {
    world
        .ctx
        .as_mut()
        .expect("context not initialised")
        .push_message(HandoffMessage::user(text));
}

#[given(expr = "an assistant message {string}")]
fn given_assistant_message(world: &mut ChWorld, text: String) {
    world
        .ctx
        .as_mut()
        .expect("context not initialised")
        .push_message(HandoffMessage::assistant(text));
}

#[given(expr = "a tool named {string} described as {string} with parameters {string}")]
fn given_tool(world: &mut ChWorld, name: String, desc: String, params: String) {
    world
        .ctx
        .as_mut()
        .expect("context not initialised")
        .push_tool(ToolDefinition::new(name, desc, params));
}

#[given(expr = "{int} user messages prefixed {string}")]
fn given_n_user_messages(world: &mut ChWorld, count: usize, prefix: String) {
    let ctx = world.ctx.as_mut().expect("context not initialised");
    for i in 0..count {
        ctx.push_message(HandoffMessage::user(format!("{}{}", prefix, i)));
    }
}

#[given("an empty handoff history")]
fn given_empty_history(world: &mut ChWorld) {
    world.history = HandoffHistory::new();
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I serialize the context")]
fn when_serialize(world: &mut ChWorld) {
    let json = world
        .ctx
        .as_ref()
        .expect("context not initialised")
        .serialize()
        .expect("serialize failed");
    world.serialized = Some(json);
}

#[when("I deserialize the context")]
fn when_deserialize(world: &mut ChWorld) {
    let json = world
        .serialized
        .as_deref()
        .expect("no serialized JSON to deserialize");
    let restored = HandoffContext::deserialize(json).expect("deserialize failed");
    world.restored = Some(restored);
}

#[when(expr = "I trim the context to a budget of {int} token")]
fn when_trim(world: &mut ChWorld, budget: usize) {
    let trimmed = world
        .ctx
        .as_ref()
        .expect("context not initialised")
        .trim_to_token_budget(budget);
    world.ctx = Some(trimmed);
}

#[when(expr = "I route the context to provider {string}")]
fn when_route(world: &mut ChWorld, target: String) {
    let routed = world
        .ctx
        .as_ref()
        .expect("context not initialised")
        .for_provider(&target);
    world.routed = Some(routed);
}

#[when(expr = "I record a handoff from {string} to {string} for reason {string} at message {int}")]
fn when_record_handoff(
    world: &mut ChWorld,
    from: String,
    to: String,
    reason: String,
    msg_count: usize,
) {
    let r = match reason.as_str() {
        "cost_routing" => HandoffReason::CostRouting,
        "fallback" => HandoffReason::Fallback,
        "capability_gap" => HandoffReason::CapabilityGap,
        _ => HandoffReason::UserRequested,
    };
    world.history.record(&from, &to, r, msg_count);
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the restored source provider is {string}")]
fn then_restored_source(world: &mut ChWorld, expected: String) {
    let r = world.restored.as_ref().expect("no restored context");
    assert_eq!(r.source_provider, expected);
}

#[then(expr = "the restored message count is {int}")]
fn then_restored_msg_count(world: &mut ChWorld, expected: usize) {
    let r = world.restored.as_ref().expect("no restored context");
    assert_eq!(r.message_count(), expected);
}

#[then(expr = "the restored tool count is {int}")]
fn then_restored_tool_count(world: &mut ChWorld, expected: usize) {
    let r = world.restored.as_ref().expect("no restored context");
    assert_eq!(r.tools.len(), expected);
}

#[then(expr = "the restored system prompt is {string}")]
fn then_restored_system_prompt(world: &mut ChWorld, expected: String) {
    let r = world.restored.as_ref().expect("no restored context");
    assert_eq!(r.system_prompt.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the message count is less than {int}")]
fn then_message_count_lt(world: &mut ChWorld, limit: usize) {
    let ctx = world.ctx.as_ref().expect("context not initialised");
    assert!(
        ctx.message_count() < limit,
        "expected message count < {} but got {}",
        limit,
        ctx.message_count()
    );
}

#[then(expr = "the last remaining message starts with {string}")]
fn then_last_message_starts_with(world: &mut ChWorld, prefix: String) {
    let ctx = world.ctx.as_ref().expect("context not initialised");
    if let Some(last) = ctx.messages.last() {
        assert!(
            last.text_content().starts_with(&prefix),
            "last message '{}' does not start with '{}'",
            last.text_content(),
            prefix
        );
    }
    // If no messages remain after trimming that is also valid.
}

#[then(expr = "the routed context has target provider {string}")]
fn then_routed_target(world: &mut ChWorld, expected: String) {
    let r = world.routed.as_ref().expect("no routed context");
    assert_eq!(r.target_provider.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the source provider is still {string}")]
fn then_source_unchanged(world: &mut ChWorld, expected: String) {
    let ctx = world.ctx.as_ref().expect("context not initialised");
    assert_eq!(ctx.source_provider, expected);
    assert!(
        ctx.target_provider.is_none(),
        "original context should have no target provider"
    );
}

#[then(expr = "the routed context has {int} messages")]
fn then_routed_msg_count(world: &mut ChWorld, expected: usize) {
    let r = world.routed.as_ref().expect("no routed context");
    assert_eq!(r.message_count(), expected);
}

#[then(expr = "the history count is {int}")]
fn then_history_count(world: &mut ChWorld, expected: usize) {
    assert_eq!(world.history.count(), expected);
}

#[then(expr = "the last handoff destination is {string}")]
fn then_last_destination(world: &mut ChWorld, expected: String) {
    let last = world.history.last().expect("history is empty");
    assert_eq!(last.to_provider, expected);
}

#[then(expr = "the providers used are {string}")]
fn then_providers_used(world: &mut ChWorld, expected_csv: String) {
    let expected: Vec<String> = expected_csv.split(',').map(str::to_owned).collect();
    let actual = world.history.providers_used();
    assert_eq!(actual, expected);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(ChWorld::run(
        "tests/features/context_handoff.feature",
    ));
}
