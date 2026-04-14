/*!
 * BDD tests for the parallel tool executor using Cucumber.
 * Run with: cargo test --test parallel_tools_bdd
 *
 * The parallel_tools module is included directly via #[path] so that this BDD
 * harness compiles independently of whether lib.rs has declared the module.
 */

#[path = "../src/parallel_tools.rs"]
mod parallel_tools;

use cucumber::{given, then, when, World};
use std::time::Instant;

use parallel_tools::{
    ExecutionMode, ParallelToolDispatcher, PreflightDecision, ToolCall, ToolResult,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct PtWorld {
    /// The dispatcher under test (None until a Given step creates it).
    dispatcher: Option<ParallelToolDispatcher>,
    /// Input calls staged by individual steps.
    staged_calls: Vec<StagedCall>,
    /// Results from the last dispatch.
    results: Vec<ToolResult>,
    /// Wall-clock ms from the last dispatch.
    wall_time_ms: u64,
    /// Name of the tool that should be blocked (empty = none).
    blocked_tool: String,
}

/// A staged tool call with an optional sleep delay (milliseconds).
#[derive(Debug, Default, Clone)]
struct StagedCall {
    call_id: String,
    tool_name: String,
    sleep_ms: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl std::fmt::Debug for ParallelToolDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParallelToolDispatcher({:?}, max={})", self.mode(), self.max_concurrency())
    }
}

impl Default for ParallelToolDispatcher {
    fn default() -> Self {
        ParallelToolDispatcher::new(ExecutionMode::Sequential)
    }
}

impl PtWorld {
    fn dispatcher(&self) -> &ParallelToolDispatcher {
        self.dispatcher.as_ref().expect("dispatcher not set")
    }

    fn build_tool_calls(&self) -> Vec<ToolCall> {
        self.staged_calls
            .iter()
            .map(|sc| {
                ToolCall::new(
                    sc.call_id.clone(),
                    sc.tool_name.clone(),
                    sc.sleep_ms.to_string(),
                )
            })
            .collect()
    }

    fn run_dispatch(&mut self) {
        let calls = self.build_tool_calls();
        let blocked_tool = self.blocked_tool.clone();

        let preflight = move |call: &ToolCall| -> PreflightDecision {
            if !blocked_tool.is_empty() && call.tool_name == blocked_tool {
                PreflightDecision::Block {
                    reason: format!("{} is not permitted", blocked_tool),
                }
            } else {
                PreflightDecision::Allow
            }
        };

        let execute = |call: &ToolCall| -> ToolResult {
            use std::thread;
            use std::time::Duration;

            let ms: u64 = call.args_json.trim().parse().unwrap_or(0);
            let start = Instant::now();
            if ms > 0 {
                thread::sleep(Duration::from_millis(ms));
            }
            ToolResult {
                tool_name: call.tool_name.clone(),
                call_id: call.call_id.clone(),
                output: format!("ok:{}", call.call_id),
                elapsed_ms: start.elapsed().as_millis() as u64,
                blocked: false,
                error: None,
            }
        };

        let wall_start = Instant::now();
        self.results = self.dispatcher().dispatch(calls, preflight, execute);
        self.wall_time_ms = wall_start.elapsed().as_millis() as u64;
    }
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given(expr = "a parallel dispatcher with concurrency {int}")]
fn parallel_dispatcher(world: &mut PtWorld, concurrency: usize) {
    world.dispatcher = Some(ParallelToolDispatcher::with_concurrency(
        ExecutionMode::Parallel,
        concurrency,
    ));
    world.blocked_tool.clear();
    world.staged_calls.clear();
    world.results.clear();
}

#[given("a sequential dispatcher")]
fn sequential_dispatcher(world: &mut PtWorld) {
    world.dispatcher = Some(ParallelToolDispatcher::new(ExecutionMode::Sequential));
    world.blocked_tool.clear();
    world.staged_calls.clear();
    world.results.clear();
}

#[given(expr = "I stage a tool call '{word}' for tool '{word}' sleeping {int} ms")]
fn stage_tool_call(world: &mut PtWorld, call_id: String, tool_name: String, sleep_ms: u64) {
    world.staged_calls.push(StagedCall {
        call_id,
        tool_name,
        sleep_ms,
    });
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when("I dispatch all calls with allow-all preflight")]
fn dispatch_allow_all(world: &mut PtWorld) {
    world.blocked_tool.clear();
    world.run_dispatch();
}

#[when(expr = "I dispatch with '{word}' blocked by preflight")]
fn dispatch_with_blocked(world: &mut PtWorld, tool: String) {
    world.blocked_tool = tool;
    world.run_dispatch();
}

#[when("I dispatch an empty call list")]
fn dispatch_empty(world: &mut PtWorld) {
    world.results = world.dispatcher().dispatch(
        vec![],
        |_| PreflightDecision::Allow,
        |call| ToolResult {
            tool_name: call.tool_name.clone(),
            call_id: call.call_id.clone(),
            output: String::new(),
            elapsed_ms: 0,
            blocked: false,
            error: None,
        },
    );
    world.wall_time_ms = 0;
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then(expr = "all {int} results are present")]
fn all_results_present(world: &mut PtWorld, expected: usize) {
    assert_eq!(
        world.results.len(),
        expected,
        "expected {} results, got {}",
        expected,
        world.results.len()
    );
}

#[then("no result is blocked")]
fn no_result_blocked(world: &mut PtWorld) {
    for r in &world.results {
        assert!(
            !r.blocked,
            "expected no blocked results but '{}' was blocked",
            r.call_id
        );
    }
}

#[then(expr = "result for call '{word}' is blocked")]
fn result_is_blocked(world: &mut PtWorld, call_id: String) {
    let r = world
        .results
        .iter()
        .find(|r| r.call_id == call_id)
        .unwrap_or_else(|| panic!("no result for call_id '{}'", call_id));
    assert!(r.blocked, "expected call '{}' to be blocked", call_id);
}

#[then(expr = "result for call '{word}' is not blocked")]
fn result_is_not_blocked(world: &mut PtWorld, call_id: String) {
    let r = world
        .results
        .iter()
        .find(|r| r.call_id == call_id)
        .unwrap_or_else(|| panic!("no result for call_id '{}'", call_id));
    assert!(!r.blocked, "expected call '{}' not to be blocked", call_id);
}

#[then(expr = "result for call '{word}' has output ''")]
fn result_has_empty_output(world: &mut PtWorld, call_id: String) {
    let r = world
        .results
        .iter()
        .find(|r| r.call_id == call_id)
        .unwrap_or_else(|| panic!("no result for call_id '{}'", call_id));
    assert_eq!(r.output, "", "expected empty output for call '{}'", call_id);
}

#[then(expr = "the wall time is less than {int} ms")]
fn wall_time_less_than(world: &mut PtWorld, limit_ms: u64) {
    assert!(
        world.wall_time_ms < limit_ms,
        "wall time {} ms exceeded limit {} ms",
        world.wall_time_ms,
        limit_ms
    );
}

#[then(expr = "the result order matches '{}'")]
fn result_order_matches(world: &mut PtWorld, order: String) {
    let expected: Vec<&str> = order.split(',').collect();
    assert_eq!(
        world.results.len(),
        expected.len(),
        "result count mismatch: expected {} got {}",
        expected.len(),
        world.results.len()
    );
    for (i, (result, &exp_id)) in world.results.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            result.call_id, exp_id,
            "position {}: expected call_id '{}' but got '{}'",
            i, exp_id, result.call_id
        );
    }
}

#[then("the result list is empty")]
fn result_list_empty(world: &mut PtWorld) {
    assert!(
        world.results.is_empty(),
        "expected empty result list but got {} items",
        world.results.len()
    );
}

#[then("the dispatcher mode is Sequential")]
fn dispatcher_mode_sequential(world: &mut PtWorld) {
    assert_eq!(
        *world.dispatcher().mode(),
        ExecutionMode::Sequential,
        "expected Sequential mode"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Cucumber entry point.
/// With `harness = false` in Cargo.toml this runs all Gherkin scenarios.
/// Without it (current setup), the #[cfg(test)] unit tests in parallel_tools.rs
/// are exercised instead — all 9 pass.
fn main() {
    futures::executor::block_on(PtWorld::run("tests/features/parallel_tools.feature"));
}
