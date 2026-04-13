/*!
 * BDD tests for the priority-queue task scheduler using Cucumber.
 * Run with: cargo test --test task_scheduler_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::task_scheduler::{ScheduledTask, TaskPriority, TaskScheduler};

#[derive(Debug, Default, World)]
pub struct TsWorld {
    scheduler: TaskScheduler,
    popped: Option<ScheduledTask>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "a scheduler with a Low task {string} and a High task {string}")]
fn two_priority_tasks(world: &mut TsWorld, low: String, high: String) {
    world.scheduler = TaskScheduler::new();
    world.scheduler.push(ScheduledTask::new(low, "low", TaskPriority::Low));
    world.scheduler.push(ScheduledTask::new(high, "high", TaskPriority::High));
}

#[given(expr = "a scheduler with a Normal task {string} scheduled at time {int}")]
fn future_task(world: &mut TsWorld, id: String, ts: u64) {
    world.scheduler = TaskScheduler::new();
    world.scheduler.push(ScheduledTask::new(id, "label", TaskPriority::Normal).with_run_after(ts));
}

#[given(expr = "a scheduler with a Normal task {string} and no delay")]
fn immediate_task(world: &mut TsWorld, id: String) {
    world.scheduler = TaskScheduler::new();
    world.scheduler.push(ScheduledTask::new(id, "label", TaskPriority::Normal));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I pop a ready task at time {int}")]
fn pop_ready(world: &mut TsWorld, now: u64) {
    world.popped = world.scheduler.pop_ready(now);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(expr = "the task id should be {string}")]
fn assert_task_id(world: &mut TsWorld, expected: String) {
    let task = world.popped.as_ref().expect("expected a task but got None");
    assert_eq!(task.id, expected, "task id mismatch");
}

#[then("no task should be ready")]
fn assert_no_task(world: &mut TsWorld) {
    assert!(world.popped.is_none(), "expected no task but got {:?}", world.popped);
}

#[then("the scheduler should be empty")]
fn assert_empty(world: &mut TsWorld) {
    assert!(world.scheduler.is_empty(), "expected scheduler to be empty");
}

fn main() {
    futures::executor::block_on(TsWorld::run("tests/features/task_scheduler.feature"));
}
