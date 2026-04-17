/*!
 * BDD tests for stream_tool_args using Cucumber.
 * Run with: cargo test --test stream_tool_args_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::stream_tool_args::{
    PartialParseResult, StreamingToolCallManager, ToolArgAccumulator, ToolCallDelta,
    render_partial_hint,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct StaWorld {
    /// Single-accumulator scenarios.
    accumulator: Option<ToolArgAccumulator>,
    /// Last PartialParseResult from a push.
    last_result: Option<PartialParseResult>,
    /// Result of finalize().
    finalize_result: Option<Result<serde_json::Value, String>>,
    /// Manager for multi-call scenarios.
    manager: Option<StreamingToolCallManager>,
    /// Completion results keyed by call id (stored as Vec for ordering).
    completion_results: Vec<(String, Result<serde_json::Value, String>)>,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a new accumulator for call {string} and tool {string}")]
fn new_accumulator(world: &mut StaWorld, call_id: String, tool_name: String) {
    world.accumulator = Some(ToolArgAccumulator::new(call_id, tool_name));
    world.last_result = None;
    world.finalize_result = None;
}

#[given("a new streaming tool call manager")]
fn new_manager(world: &mut StaWorld) {
    world.manager = Some(StreamingToolCallManager::new());
    world.completion_results.clear();
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when(expr = "I push the fragment {string}")]
fn push_fragment(world: &mut StaWorld, fragment: String) {
    let acc = world.accumulator.as_mut().expect("accumulator not initialised");
    world.last_result = Some(acc.push(&fragment));
}

#[when(expr = "I send a delta for call {string} tool {string} with fragment {string}")]
fn send_delta(world: &mut StaWorld, call_id: String, tool_name: String, fragment: String) {
    let mgr = world.manager.as_mut().expect("manager not initialised");
    let delta = ToolCallDelta::new(call_id, tool_name, fragment, 0);
    mgr.on_delta(delta);
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the buffer should contain {string}")]
fn buffer_contains(world: &mut StaWorld, needle: String) {
    let acc = world.accumulator.as_ref().expect("no accumulator");
    assert!(
        acc.buffer().contains(&needle),
        "buffer '{}' does not contain '{}'",
        acc.buffer(),
        needle
    );
}

#[then(expr = "the sequence should be {int}")]
fn sequence_is(world: &mut StaWorld, expected: u32) {
    let acc = world.accumulator.as_ref().expect("no accumulator");
    assert_eq!(
        acc.sequence(),
        expected,
        "sequence mismatch: got {}, expected {}",
        acc.sequence(),
        expected
    );
}

#[then(expr = "the extractable keys should include {string}")]
fn keys_include(world: &mut StaWorld, key: String) {
    let result = world.last_result.as_ref().expect("no parse result");
    assert!(
        result.extractable_keys.contains(&key),
        "extractable_keys {:?} does not contain '{}'",
        result.extractable_keys,
        key
    );
}

#[then(expr = "the extractable keys should not include {string}")]
fn keys_exclude(world: &mut StaWorld, key: String) {
    let result = world.last_result.as_ref().expect("no parse result");
    assert!(
        !result.extractable_keys.contains(&key),
        "extractable_keys {:?} unexpectedly contains '{}'",
        result.extractable_keys,
        key
    );
}

#[then("finalizing should succeed")]
fn finalize_ok(world: &mut StaWorld) {
    let acc = world.accumulator.as_ref().expect("no accumulator");
    let result = acc.finalize();
    assert!(
        result.is_ok(),
        "finalize failed: {:?}",
        result.err()
    );
    world.finalize_result = Some(result);
}

#[then(expr = "the finalized value at {string} should be {string}")]
fn finalized_value_at(world: &mut StaWorld, key: String, expected: String) {
    // If finalize_result is not yet populated, run finalize now.
    if world.finalize_result.is_none() {
        let acc = world.accumulator.as_ref().expect("no accumulator");
        world.finalize_result = Some(acc.finalize());
    }
    let result = world
        .finalize_result
        .as_ref()
        .expect("no finalize result")
        .as_ref()
        .expect("finalize failed");
    let actual = result
        .get(&key)
        .and_then(|v| v.as_str())
        .expect("key not found or not a string");
    assert_eq!(actual, expected, "finalized[{}] mismatch", key);
}

#[then("the hint should be a FilePath hint")]
fn hint_is_file_path(world: &mut StaWorld) {
    let result = world.last_result.as_ref().expect("no parse result");
    assert!(
        result.has_file_path(),
        "expected FilePath hint, got {:?}",
        result.hint
    );
}

#[then(expr = "the hint file path should be {string}")]
fn hint_file_path_value(world: &mut StaWorld, expected: String) {
    let result = world.last_result.as_ref().expect("no parse result");
    let actual = result.file_path().expect("no file path in hint");
    assert_eq!(actual, expected, "file path hint mismatch");
}

#[then(expr = "the rendered hint for tool {string} should contain {string}")]
fn rendered_hint_contains(world: &mut StaWorld, tool_name: String, needle: String) {
    let result = world.last_result.as_ref().expect("no parse result");
    let rendered = render_partial_hint(result, &tool_name);
    assert!(
        rendered.contains(&needle),
        "rendered '{}' does not contain '{}'",
        rendered,
        needle
    );
}

#[then(expr = "the manager should have {int} active calls")]
fn manager_active_count(world: &mut StaWorld, expected: usize) {
    let mgr = world.manager.as_ref().expect("no manager");
    let count = mgr.active_calls().len();
    assert_eq!(count, expected, "active call count mismatch: got {}", count);
}

#[then(expr = "completing call {string} should succeed")]
fn completing_call_ok(world: &mut StaWorld, call_id: String) {
    let mgr = world.manager.as_mut().expect("no manager");
    let result = mgr.on_complete(&call_id);
    assert!(
        result.is_ok(),
        "on_complete('{}') failed: {:?}",
        call_id,
        result.err()
    );
    world.completion_results.push((call_id, result));
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(StaWorld::run(
        "tests/features/stream_tool_args.feature",
    ));
}
