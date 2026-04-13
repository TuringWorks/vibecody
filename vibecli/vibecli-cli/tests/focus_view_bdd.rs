/*!
 * BDD tests for Focus View session management using Cucumber.
 * Run with: cargo test --test focus_view_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::focus_view::{FocusConfig, FocusManager, NotificationLevel};

#[derive(Debug, Default, World)]
pub struct FvWorld {
    manager: FocusManager,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("the focus manager is idle")]
fn idle_manager(world: &mut FvWorld) {
    world.manager = FocusManager::new();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I enter deep focus at time {int}")]
fn enter_deep(world: &mut FvWorld, now: u64) {
    world.manager.enter_focus(FocusConfig::default_deep(), now);
}

#[when(expr = "I exit focus at time {int}")]
fn exit_focus(world: &mut FvWorld, now: u64) {
    world.manager.exit_focus(now);
}

#[when(expr = "I record {int} distractions")]
fn record_distractions(world: &mut FvWorld, count: u32) {
    for _ in 0..count {
        world.manager.record_distraction();
    }
}

#[when(expr = "I enter focus with auto-exit {int} seconds at time {int}")]
fn enter_auto_exit(world: &mut FvWorld, secs: u64, now: u64) {
    let mut cfg = FocusConfig::default_deep();
    cfg.auto_exit_after_secs = Some(secs);
    world.manager.enter_focus(cfg, now);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the manager should be in focus")]
fn assert_in_focus(world: &mut FvWorld) {
    assert!(world.manager.is_in_focus(), "expected manager to be in focus");
}

#[then("the manager should not be in focus")]
fn assert_not_in_focus(world: &mut FvWorld) {
    assert!(!world.manager.is_in_focus(), "expected manager to NOT be in focus");
}

#[then(expr = "the session count should be {int}")]
fn assert_session_count(world: &mut FvWorld, expected: usize) {
    assert_eq!(world.manager.session_count(), expected, "session count mismatch");
}

#[then(expr = "the active distraction count should be {int}")]
fn assert_distraction_count(world: &mut FvWorld, expected: u32) {
    let count = world.manager.active.as_ref().map(|s| s.distraction_count).unwrap_or(0);
    assert_eq!(count, expected, "distraction count mismatch");
}

#[then(expr = "auto-exit at time {int} should be true")]
fn assert_auto_exit_true(world: &mut FvWorld, now: u64) {
    assert!(world.manager.should_auto_exit(now), "expected auto-exit at {}", now);
}

#[then(expr = "auto-exit at time {int} should be false")]
fn assert_auto_exit_false(world: &mut FvWorld, now: u64) {
    assert!(!world.manager.should_auto_exit(now), "expected NO auto-exit at {}", now);
}

#[then("Silent should be less than Minimal")]
fn silent_lt_minimal(_world: &mut FvWorld) {
    assert!(NotificationLevel::Silent < NotificationLevel::Minimal);
}

#[then("Minimal should be less than Normal")]
fn minimal_lt_normal(_world: &mut FvWorld) {
    assert!(NotificationLevel::Minimal < NotificationLevel::Normal);
}

fn main() {
    futures::executor::block_on(FvWorld::run("tests/features/focus_view.feature"));
}
