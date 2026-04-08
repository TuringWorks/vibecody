/*!
 * BDD tests for CircuitBreaker using Cucumber.
 *
 * Run with:
 *   cargo test --test circuit_breaker_bdd
 */

use cucumber::{World, given, then, when};
use vibe_ai::agent::{AgentHealthState, CircuitBreaker};
use vibe_ai::tools::{ToolCall, ToolResult};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
#[world(init = Self::new)]
pub struct CbWorld {
    cb: CircuitBreaker,
}

impl CbWorld {
    fn new() -> Self {
        Self {
            cb: CircuitBreaker::default(),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn ok(tool: &str) -> ToolResult {
    ToolResult { tool_name: tool.to_string(), output: "ok".to_string(), success: true, truncated: false }
}

fn err_msg(tool: &str, msg: &str) -> ToolResult {
    ToolResult { tool_name: tool.to_string(), output: msg.to_string(), success: false, truncated: false }
}

fn think() -> ToolCall {
    ToolCall::Think { thought: "pondering".into() }
}

fn write_file() -> ToolCall {
    ToolCall::WriteFile { path: "out.rs".into(), content: "fn f(){}".into() }
}

fn bash(cmd: &str) -> ToolCall {
    ToolCall::Bash { command: cmd.into() }
}

// ── Given steps ──────────────────────────────────────────────────────────────

#[given("a fresh CircuitBreaker with default thresholds")]
fn fresh_default(world: &mut CbWorld) {
    world.cb = CircuitBreaker::default();
}

#[given(expr = "a CircuitBreaker with stall_threshold {int}")]
fn with_stall_threshold(world: &mut CbWorld, threshold: u32) {
    world.cb = CircuitBreaker { stall_threshold: threshold, ..Default::default() };
}

#[given(expr = "a CircuitBreaker with spin_threshold {int} and stall_threshold {int}")]
fn with_spin_and_stall(world: &mut CbWorld, spin: u32, stall: u32) {
    world.cb = CircuitBreaker { spin_threshold: spin, stall_threshold: stall, ..Default::default() };
}

#[given(expr = "a CircuitBreaker with stall_threshold {int} and max_rotations {int}")]
fn with_stall_and_max_rotations(world: &mut CbWorld, stall: u32, max_rot: u32) {
    world.cb = CircuitBreaker { stall_threshold: stall, max_rotations: max_rot, ..Default::default() };
}

#[given(expr = "a CircuitBreaker with stall_threshold {int} and degradation_pct {float}")]
fn with_stall_and_degradation(world: &mut CbWorld, stall: u32, pct: f64) {
    world.cb = CircuitBreaker { stall_threshold: stall, degradation_pct: pct, ..Default::default() };
}

#[given(expr = "a health state of {string}")]
fn given_health_state(world: &mut CbWorld, state: String) {
    world.cb.state = parse_state(&state);
}

// ── When steps ───────────────────────────────────────────────────────────────

#[when(expr = "I record {int} Think steps with success")]
fn record_think_ok(world: &mut CbWorld, count: usize) {
    let tc = think();
    for _ in 0..count {
        world.cb.record_step(&tc, &ok("think"), 100);
    }
}

#[when(expr = "I record {int} WriteFile step with success")]
fn record_write_one(world: &mut CbWorld, count: usize) {
    let tc = write_file();
    for _ in 0..count {
        world.cb.record_step(&tc, &ok("write_file"), 100);
    }
}

#[when(expr = "I record {int} Bash step with success")]
fn record_bash_one(world: &mut CbWorld, count: usize) {
    let tc = bash("echo ok");
    for _ in 0..count {
        world.cb.record_step(&tc, &ok("bash"), 100);
    }
}

#[when(expr = "I record {int} Bash steps with failure")]
fn record_bash_fail(world: &mut CbWorld, count: usize) {
    let tc = bash("cargo build");
    let er = err_msg("bash", "error: build failed");
    for _ in 0..count {
        world.cb.record_step(&tc, &er, 50);
    }
}

#[when(expr = "I record {int} identical Bash error steps")]
fn record_identical_bash_errors(world: &mut CbWorld, count: usize) {
    let tc = bash("cargo test");
    let er = err_msg("bash", "error[E0308]: mismatched types");
    for _ in 0..count {
        world.cb.record_step(&tc, &er, 100);
    }
}

#[when(expr = "I record {int} distinct Bash error steps")]
fn record_distinct_bash_errors(world: &mut CbWorld, count: usize) {
    let tc = bash("cargo check");
    for i in 0..count {
        let er = err_msg("bash", &format!("unique error #{i}"));
        world.cb.record_step(&tc, &er, 100);
    }
}

#[when(expr = "I record {int} Bash steps with success and output size {int}")]
fn record_bash_with_output(world: &mut CbWorld, count: usize, size: usize) {
    let tc = bash("ls");
    for _ in 0..count {
        world.cb.record_step(&tc, &ok("bash"), size);
    }
}

#[when(expr = "I record {int} Think step with success")]
fn record_think_one(world: &mut CbWorld, count: usize) {
    let tc = think();
    for _ in 0..count {
        world.cb.record_step(&tc, &ok("think"), 100);
    }
}

// ── Then steps ───────────────────────────────────────────────────────────────

#[then(expr = "the health state should be {string}")]
fn assert_state(world: &mut CbWorld, expected: String) {
    let want = parse_state(&expected);
    assert_eq!(
        world.cb.state, want,
        "expected {:?} but got {:?}", want, world.cb.state
    );
}

#[then(expr = "the stall counter should be {int}")]
fn assert_stall_counter(world: &mut CbWorld, expected: u32) {
    assert_eq!(
        world.cb.steps_since_file_change, expected,
        "stall counter: expected {} got {}", expected, world.cb.steps_since_file_change
    );
}

#[then("the error hashes should be empty")]
fn assert_error_hashes_empty(world: &mut CbWorld) {
    assert!(
        world.cb.recent_error_hashes.is_empty(),
        "expected empty error hashes but got {:?}", world.cb.recent_error_hashes
    );
}

#[then(expr = "the approach_rotations should be {int}")]
fn assert_rotations(world: &mut CbWorld, expected: u32) {
    assert_eq!(
        world.cb.approach_rotations, expected,
        "approach_rotations: expected {} got {}", expected, world.cb.approach_rotations
    );
}

#[then(expr = "the rotation hint should contain {string}")]
fn assert_rotation_hint_contains(world: &mut CbWorld, fragment: String) {
    let hint = world.cb.rotation_hint();
    assert!(
        hint.contains(&fragment),
        "rotation hint {:?} does not contain {:?}", hint, fragment
    );
}

#[then(expr = "its display string should be {string}")]
fn assert_display_string(world: &mut CbWorld, expected: String) {
    assert_eq!(
        world.cb.state.to_string(), expected,
        "display: expected {:?} got {:?}", expected, world.cb.state.to_string()
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_state(s: &str) -> AgentHealthState {
    match s {
        "PROGRESS"  => AgentHealthState::Progress,
        "STALLED"   => AgentHealthState::Stalled,
        "SPINNING"  => AgentHealthState::Spinning,
        "DEGRADED"  => AgentHealthState::Degraded,
        "BLOCKED"   => AgentHealthState::Blocked,
        other       => panic!("Unknown AgentHealthState: {other}"),
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    CbWorld::run("tests/features/circuit_breaker.feature").await;
}
