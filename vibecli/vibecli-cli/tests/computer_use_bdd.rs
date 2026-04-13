/*!
 * BDD tests for Computer Use visual self-testing using Cucumber.
 * Run with: cargo test --test computer_use_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::computer_use::{VisualAssertion, VisualTestSession};

#[derive(Debug, Default, World)]
pub struct CuWorld {
    session: VisualTestSession,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a computer use session")]
fn new_session(world: &mut CuWorld) {
    world.session = VisualTestSession {
        id: "bdd-session".to_string(),
        url: "http://localhost:3000".to_string(),
        steps: vec![],
        passed: false,
        started_at: 0,
        finished_at: None,
    };
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I record a passing assertion {string}")]
fn record_passing(world: &mut CuWorld, label: String) {
    world.session.steps.push(vibecli_cli::computer_use::VisualTestStep {
        action: format!("assert: {}", label),
        screenshot: None,
        assertion: Some(VisualAssertion {
            screenshot_path: String::new(),
            assertion: label,
            passed: true,
            confidence: 0.99,
            details: "ok".to_string(),
        }),
    });
}

#[when(expr = "I record a failing assertion {string}")]
fn record_failing(world: &mut CuWorld, label: String) {
    world.session.steps.push(vibecli_cli::computer_use::VisualTestStep {
        action: format!("assert: {}", label),
        screenshot: None,
        assertion: Some(VisualAssertion {
            screenshot_path: String::new(),
            assertion: label,
            passed: false,
            confidence: 0.30,
            details: "failed".to_string(),
        }),
    });
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(expr = "the session should have {int} assertion")]
fn assert_count(world: &mut CuWorld, count: usize) {
    assert_eq!(world.session.steps.len(), count, "step count mismatch");
}

#[then("the last assertion should have passed")]
fn last_passed(world: &mut CuWorld) {
    let a = world.session.steps.last().unwrap().assertion.as_ref().unwrap();
    assert!(a.passed, "expected last assertion to pass");
}

#[then("the last assertion should not have passed")]
fn last_failed(world: &mut CuWorld) {
    let a = world.session.steps.last().unwrap().assertion.as_ref().unwrap();
    assert!(!a.passed, "expected last assertion to fail");
}

#[then("the overall result should be pass")]
fn overall_pass(world: &mut CuWorld) {
    let all_pass = world.session.steps.iter()
        .filter_map(|s| s.assertion.as_ref())
        .all(|a| a.passed);
    assert!(all_pass, "expected all assertions to pass");
}

#[then("the overall result should be fail")]
fn overall_fail(world: &mut CuWorld) {
    let any_fail = world.session.steps.iter()
        .filter_map(|s| s.assertion.as_ref())
        .any(|a| !a.passed);
    assert!(any_fail, "expected at least one failing assertion");
}

fn main() {
    futures::executor::block_on(CuWorld::run("tests/features/computer_use.feature"));
}
