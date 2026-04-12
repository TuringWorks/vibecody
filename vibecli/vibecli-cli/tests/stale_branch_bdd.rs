/*!
 * BDD tests for stale_branch using Cucumber.
 * Run with: cargo test --test stale_branch_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::stale_branch::{
    BranchFreshness, StaleBranchConfig, FreshnessPolicyDetector,
};

#[derive(Debug, Default, World)]
pub struct SbWorld {
    commits_behind: usize,
    activity_secs: u64,
    freshness: Option<BranchFreshness>,
}

fn make_detector() -> FreshnessPolicyDetector {
    FreshnessPolicyDetector::new(StaleBranchConfig::default())
}

#[given(expr = "a branch {int} commits behind and active {int} hour ago")]
fn branch_behind_active_hours(world: &mut SbWorld, behind: usize, hours: u64) {
    world.commits_behind = behind;
    world.activity_secs = hours * 3600;
}

#[given(expr = "a branch {int} commits behind and active {int} days ago")]
fn branch_behind_active_days(world: &mut SbWorld, behind: usize, days: u64) {
    world.commits_behind = behind;
    world.activity_secs = days * 24 * 3600;
}

#[given(expr = "a branch {int} commits behind main")]
fn branch_behind(world: &mut SbWorld, behind: usize) {
    world.commits_behind = behind;
    world.activity_secs = 3600; // 1 hour (recent)
}

#[when("I assess freshness")]
fn assess(world: &mut SbWorld) {
    let d = make_detector();
    world.freshness = Some(d.assess(
        "feature",
        "main",
        world.commits_behind,
        0,
        world.activity_secs,
    ));
}

#[then(expr = "the state should be {string}")]
fn check_state(world: &mut SbWorld, expected: String) {
    let state = world.freshness.as_ref().unwrap().state.to_string();
    assert_eq!(state, expected);
}

#[then(expr = "the missing_fixes_message should contain {string}")]
fn check_msg(world: &mut SbWorld, needle: String) {
    let msg = &world.freshness.as_ref().unwrap().missing_fixes_message;
    assert!(
        msg.contains(&needle),
        "message '{}' does not contain '{}'",
        msg,
        needle
    );
}

fn main() {
    futures::executor::block_on(SbWorld::run("tests/features/stale_branch.feature"));
}
