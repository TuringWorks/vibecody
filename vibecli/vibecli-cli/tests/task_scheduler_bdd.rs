/*!
 * BDD tests for the cron/interval Scheduler and priority TaskScheduler.
 * Run with: cargo test --test task_scheduler_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::task_scheduler::{
    CronTask, Schedule, ScheduledTask, Scheduler, TaskPriority, TaskScheduler,
};

#[derive(Debug, Default, World)]
pub struct TsWorld {
    scheduler: Scheduler,
    priority_scheduler: TaskScheduler,
    ticked_ids: Vec<String>,
    due_ids: Vec<String>,
    popped: Option<ScheduledTask>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "a scheduler with an interval task {string} every {int} seconds starting at time {int}")]
fn interval_task(world: &mut TsWorld, id: String, secs: u64, now: u64) {
    let task = CronTask::new(id.clone(), id, "run", Schedule::Interval { secs }, now);
    world.scheduler.add(task);
}

#[given(expr = "a scheduler with a one-time task {string} at time {int} created at time {int}")]
fn once_task(world: &mut TsWorld, id: String, at: u64, now: u64) {
    let task = CronTask::new(id.clone(), id, "run", Schedule::Once { at_secs: at }, now);
    world.scheduler.add(task);
}

#[given(expr = "a priority scheduler with a Low task {string} and a High task {string}")]
fn two_priority_tasks(world: &mut TsWorld, low: String, high: String) {
    world.priority_scheduler = TaskScheduler::new();
    world.priority_scheduler.push(ScheduledTask::new(low, "low", TaskPriority::Low));
    world.priority_scheduler.push(ScheduledTask::new(high, "high", TaskPriority::High));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I tick the scheduler at time {int}")]
fn tick_at(world: &mut TsWorld, now: u64) {
    world.ticked_ids = world.scheduler.tick(now);
}

#[when(expr = "I check due tasks at time {int}")]
fn check_due(world: &mut TsWorld, now: u64) {
    world.due_ids = world.scheduler.due_tasks(now).iter().map(|t| t.id.clone()).collect();
}

#[when(expr = "I remove task {string}")]
fn remove_task(world: &mut TsWorld, id: String) {
    world.scheduler.remove(&id);
}

#[when(expr = "I pop a priority task at time {int}")]
fn pop_priority(world: &mut TsWorld, now: u64) {
    world.popped = world.priority_scheduler.pop_ready(now);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(expr = "the ticked ids should include {string}")]
fn assert_ticked(world: &mut TsWorld, id: String) {
    assert!(world.ticked_ids.contains(&id), "expected {:?} in ticked: {:?}", id, world.ticked_ids);
}

#[then("no tasks should be due")]
fn assert_none_due(world: &mut TsWorld) {
    assert!(world.due_ids.is_empty(), "expected no due tasks, got: {:?}", world.due_ids);
}

#[then(expr = "the due task should be {string}")]
fn assert_due_task(world: &mut TsWorld, id: String) {
    assert!(world.due_ids.contains(&id), "expected {:?} in due: {:?}", id, world.due_ids);
}

#[then(expr = "the scheduler should have {int} tasks")]
fn assert_task_count(world: &mut TsWorld, expected: usize) {
    assert_eq!(world.scheduler.task_count(), expected, "task count mismatch");
}

#[then(expr = "the popped task id should be {string}")]
fn assert_popped_id(world: &mut TsWorld, expected: String) {
    let t = world.popped.as_ref().expect("expected a popped task");
    assert_eq!(t.id, expected, "popped id mismatch");
}

fn main() {
    futures::executor::block_on(TsWorld::run("tests/features/task_scheduler.feature"));
}
