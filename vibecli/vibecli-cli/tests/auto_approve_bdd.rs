/*!
 * BDD tests for auto_approve using Cucumber 0.20.
 * Run with: cargo test --test auto_approve_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::auto_approve::{ApprovalConfig, ApprovalDecision, ApprovalScore, AutoApprover};

#[derive(Debug, Default, World)]
pub struct ApprovalWorld {
    tool_name: String,
    input: String,
    always_allow: Vec<String>,
    result: Option<ApprovalScore>,
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given(expr = "a tool named {string} with input {string}")]
fn set_tool(world: &mut ApprovalWorld, tool: String, input: String) {
    world.tool_name = tool;
    world.input = input.replace("\\\"", "\"");
}

#[given(expr = "{string} is in the always_allow list")]
fn add_always_allow(world: &mut ApprovalWorld, tool: String) {
    world.always_allow.push(tool);
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when("I evaluate the approval")]
fn evaluate(world: &mut ApprovalWorld) {
    let config = ApprovalConfig {
        auto_approve_threshold: 0.2,
        auto_deny_threshold: 0.8,
        always_allow: world.always_allow.clone(),
        always_deny: vec![],
    };
    let approver = AutoApprover::new(config);
    world.result = Some(approver.evaluate(&world.tool_name, &world.input));
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then(expr = "the decision should be {string}")]
fn check_decision(world: &mut ApprovalWorld, expected: String) {
    let result = world.result.as_ref().unwrap();
    let expected_decision = match expected.as_str() {
        "AutoApprove" => ApprovalDecision::AutoApprove,
        "AskUser" => ApprovalDecision::AskUser,
        "AutoDeny" => ApprovalDecision::AutoDeny,
        other => panic!("Unknown decision: {}", other),
    };
    assert_eq!(
        result.decision, expected_decision,
        "Expected {:?} but got {:?} (score={:.3}, rationale={})",
        expected_decision, result.decision, result.score, result.rationale
    );
}

#[then(expr = "the decision should not be {string}")]
fn check_decision_not(world: &mut ApprovalWorld, unexpected: String) {
    let result = world.result.as_ref().unwrap();
    let unexpected_decision = match unexpected.as_str() {
        "AutoApprove" => ApprovalDecision::AutoApprove,
        "AskUser" => ApprovalDecision::AskUser,
        "AutoDeny" => ApprovalDecision::AutoDeny,
        other => panic!("Unknown decision: {}", other),
    };
    assert_ne!(
        result.decision, unexpected_decision,
        "Expected decision to not be {:?} but it was (score={:.3})",
        unexpected_decision, result.score
    );
}

#[then(expr = "the score should be below {float}")]
fn check_score_below(world: &mut ApprovalWorld, threshold: f32) {
    let result = world.result.as_ref().unwrap();
    assert!(
        result.score < threshold,
        "Expected score < {} but got {}",
        threshold,
        result.score
    );
}

#[then(expr = "the score should be above {float}")]
fn check_score_above(world: &mut ApprovalWorld, threshold: f32) {
    let result = world.result.as_ref().unwrap();
    assert!(
        result.score > threshold,
        "Expected score > {} but got {}",
        threshold,
        result.score
    );
}

fn main() {
    futures::executor::block_on(ApprovalWorld::run("tests/features/auto_approve.feature"));
}
