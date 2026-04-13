/*!
 * BDD tests for github_action using Cucumber.
 * Run with: cargo test --test github_action_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::github_action::{ActionGenerator, ActionTrigger, WorkflowConfig};

#[derive(Debug, Default, World)]
pub struct GaWorld {
    workflow: Option<WorkflowConfig>,
    yaml: Option<String>,
    action_yml: Option<String>,
    warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a PR review workflow")]
fn given_pr_review(world: &mut GaWorld) {
    world.workflow = Some(ActionGenerator::pr_review_workflow());
}

#[given("an issue handler workflow")]
fn given_issue_handler(world: &mut GaWorld) {
    world.workflow = Some(ActionGenerator::issue_to_task_workflow());
}

#[given("an empty workflow")]
fn given_empty_workflow(world: &mut GaWorld) {
    world.workflow = Some(WorkflowConfig {
        name: "Empty".to_string(),
        triggers: vec![ActionTrigger::PullRequest],
        jobs: vec![],
    });
}

#[given("the generated action.yml content")]
fn given_action_yml(world: &mut GaWorld) {
    world.action_yml = Some(ActionGenerator::generate_action_yml());
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I validate the workflow")]
fn when_validate(world: &mut GaWorld) {
    if let Some(ref wf) = world.workflow {
        world.warnings = wf.validate();
    }
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the workflow should have trigger {string}")]
fn then_has_trigger(world: &mut GaWorld, trigger_name: String) {
    let wf = world.workflow.as_ref().unwrap();
    let found = wf.triggers.iter().any(|t| match (t, trigger_name.as_str()) {
        (ActionTrigger::PullRequest, "pull_request") => true,
        (ActionTrigger::IssueComment { .. }, "issue_comment") => true,
        (ActionTrigger::Push { .. }, "push") => true,
        (ActionTrigger::WorkflowDispatch, "workflow_dispatch") => true,
        _ => false,
    });
    assert!(found, "No trigger '{}' in workflow", trigger_name);
}

#[then(expr = "the workflow should have {int} job")]
fn then_job_count(world: &mut GaWorld, count: usize) {
    let wf = world.workflow.as_ref().unwrap();
    assert_eq!(wf.job_count(), count, "Job count mismatch");
}

#[then(expr = "the workflow YAML should contain {string}")]
fn then_yaml_contains(world: &mut GaWorld, expected: String) {
    let wf = world.workflow.as_ref().unwrap();
    let yaml = world.yaml.get_or_insert_with(|| wf.to_yaml());
    assert!(
        yaml.contains(&*expected),
        "YAML does not contain '{}'\nYAML:\n{}",
        expected,
        yaml
    );
}

#[then("validation should return at least 1 warning")]
fn then_has_warnings(world: &mut GaWorld) {
    let wf = world.workflow.as_ref().unwrap();
    let warnings = wf.validate();
    assert!(!warnings.is_empty(), "Expected at least one validation warning");
}

#[then(expr = "at least one warning should mention {string}")]
fn then_warning_mentions(world: &mut GaWorld, keyword: String) {
    let wf = world.workflow.as_ref().unwrap();
    let warnings = wf.validate();
    let found = warnings.iter().any(|w| w.contains(&*keyword));
    assert!(
        found,
        "No warning mentions '{}'. Warnings: {:?}",
        keyword, warnings
    );
}

#[then(expr = "the content should contain {string}")]
fn then_content_contains(world: &mut GaWorld, expected: String) {
    let content = world.action_yml.as_ref().unwrap();
    assert!(
        content.contains(&*expected),
        "action.yml does not contain '{}'\nContent:\n{}",
        expected,
        content
    );
}

fn main() {
    futures::executor::block_on(GaWorld::run("tests/features/github_action.feature"));
}
