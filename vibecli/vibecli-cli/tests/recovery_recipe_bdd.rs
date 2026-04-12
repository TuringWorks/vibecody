/*!
 * BDD tests for recovery_recipe using Cucumber.
 * Run with: cargo test --test recovery_recipe_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::recovery_recipe::{FailureScenario, RecoveryOutcome, RecoveryRegistry};

#[derive(Debug, Default, World)]
pub struct RrWorld {
    registry: Option<RecoveryRegistry>,
    last_outcome: Option<RecoveryOutcome>,
}

fn scenario_from(s: &str) -> FailureScenario {
    match s {
        "provider_timeout"       => FailureScenario::ProviderTimeout,
        "tool_permission_denied" => FailureScenario::ToolPermissionDenied,
        "session_corrupted"      => FailureScenario::SessionCorrupted,
        "compaction_failed"      => FailureScenario::CompactionFailed,
        "subagent_crash"         => FailureScenario::SubagentCrash,
        "workspace_conflict"     => FailureScenario::WorkspaceConflict,
        "mcp_server_down"        => FailureScenario::MCPServerDown,
        _                        => FailureScenario::ProviderTimeout,
    }
}

#[given("a fresh recovery registry")]
fn fresh_registry(world: &mut RrWorld) {
    world.registry = Some(RecoveryRegistry::new());
}

#[when(expr = "I execute auto-recovery for {string}")]
fn execute_recovery(world: &mut RrWorld, scenario_str: String) {
    if let Some(reg) = world.registry.as_mut() {
        let scenario = scenario_from(&scenario_str);
        world.last_outcome = Some(reg.execute_auto_recovery(&scenario));
    }
}

#[then("it should contain 7 recipes")]
fn check_7_recipes(world: &mut RrWorld) {
    assert_eq!(world.registry.as_ref().unwrap().recipe_count(), 7);
}

#[then(expr = "the outcome should be {string}")]
fn check_outcome(world: &mut RrWorld, expected: String) {
    let outcome = world.last_outcome.as_ref().unwrap().to_string();
    assert_eq!(outcome, expected);
}

#[then(expr = "{int} recovery event should be recorded")]
fn check_event_count(world: &mut RrWorld, expected: usize) {
    assert_eq!(world.registry.as_ref().unwrap().events().len(), expected);
}

#[then("every recipe's first step should have automatic true")]
fn check_first_steps(world: &mut RrWorld) {
    let reg = world.registry.as_ref().unwrap();
    for scenario in &[
        FailureScenario::ProviderTimeout,
        FailureScenario::ToolPermissionDenied,
        FailureScenario::SessionCorrupted,
        FailureScenario::CompactionFailed,
        FailureScenario::SubagentCrash,
        FailureScenario::WorkspaceConflict,
        FailureScenario::MCPServerDown,
    ] {
        let recipe = reg.get_recipe(scenario).unwrap();
        assert!(recipe.steps[0].automatic, "{:?} first step not automatic", scenario);
    }
}

fn main() {
    futures::executor::block_on(RrWorld::run("tests/features/recovery_recipe.feature"));
}
