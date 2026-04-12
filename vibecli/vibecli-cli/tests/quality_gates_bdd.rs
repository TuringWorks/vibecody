/*!
 * BDD tests for quality_gates using Cucumber.
 * Run with: cargo test --test quality_gates_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::quality_gates::{CheckResults, GreenContract, GreenOutcome, QualityLevel};

#[derive(Debug, Default, World)]
pub struct QgWorld {
    contract: Option<GreenContract>,
    results: CheckResults,
    outcome: Option<GreenOutcome>,
}

#[given("a MergeReady contract")]
fn merge_ready_contract(world: &mut QgWorld) {
    world.contract = Some(GreenContract::new(QualityLevel::MergeReady));
    world.results = CheckResults::all_passing();
}

#[given("a TargetedTests contract")]
fn targeted_tests_contract(world: &mut QgWorld) {
    world.contract = Some(GreenContract::new(QualityLevel::TargetedTests));
    world.results = CheckResults::all_passing();
}

#[given("all checks are passing")]
fn all_passing(world: &mut QgWorld) {
    world.results = CheckResults::all_passing();
}

#[given("tests_passed is false")]
fn tests_fail(world: &mut QgWorld) {
    world.results.tests_passed = false;
}

#[when("I evaluate the contract")]
fn evaluate(world: &mut QgWorld) {
    if let Some(contract) = &world.contract {
        world.outcome = Some(contract.evaluate(&world.results));
    }
}

#[then("MergeReady should satisfy TargetedTests")]
fn merge_satisfies_targeted(_world: &mut QgWorld) {
    assert!(QualityLevel::MergeReady.satisfies(&QualityLevel::TargetedTests));
}

#[then("TargetedTests should not satisfy Package")]
fn targeted_not_satisfy_package(_world: &mut QgWorld) {
    assert!(!QualityLevel::TargetedTests.satisfies(&QualityLevel::Package));
}

#[then(expr = "the outcome should be {string}")]
fn check_outcome_str(world: &mut QgWorld, expected: String) {
    let outcome = world.outcome.as_ref().unwrap();
    let s = match outcome {
        GreenOutcome::Pass => "pass".to_string(),
        GreenOutcome::Fail(r) => format!("fail: {r}"),
    };
    assert!(s.starts_with(&expected), "expected '{}' but got '{}'", expected, s);
}

#[then(expr = "the outcome should contain {string}")]
fn check_outcome_contains(world: &mut QgWorld, needle: String) {
    let outcome = world.outcome.as_ref().unwrap();
    let s = match outcome {
        GreenOutcome::Pass => "pass".to_string(),
        GreenOutcome::Fail(r) => r.clone(),
    };
    assert!(s.contains(&needle), "expected '{}' to contain '{}'", s, needle);
}

fn main() {
    futures::executor::block_on(QgWorld::run("tests/features/quality_gates.feature"));
}
