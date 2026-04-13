/*!
 * BDD tests for the remote dispatch queue using Cucumber.
 * Run with: cargo test --test dispatch_remote_bdd
 */
use cucumber::{given, then, when, World};
use std::collections::HashMap;
use vibecli_cli::dispatch_remote::{DispatchQueue, JobStatus};

#[derive(Debug, Default, World)]
pub struct DrWorld {
    queue: DispatchQueue,
    /// most recently enqueued job id
    last_id: String,
    dequeued_prompt: Option<String>,
    /// maps prompt string → job id (so feature steps can reference by prompt)
    prompt_to_id: HashMap<String, String>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("an empty dispatch queue")]
fn empty_queue(world: &mut DrWorld) {
    world.queue = DispatchQueue::new();
    world.prompt_to_id.clear();
    world.last_id.clear();
    world.dequeued_prompt = None;
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I enqueue a job with prompt {string} at time {int}")]
fn enqueue_job(world: &mut DrWorld, prompt: String, now: u64) {
    let id = world.queue.enqueue(&prompt, now);
    world.last_id = id.clone();
    world.prompt_to_id.insert(prompt, id);
}

#[when(expr = "I enqueue a job with prompt {string} at priority {int} at time {int}")]
fn enqueue_priority(world: &mut DrWorld, prompt: String, priority: u8, now: u64) {
    let id = world.queue.enqueue_with_priority(&prompt, priority, now);
    world.last_id = id.clone();
    world.prompt_to_id.insert(prompt, id);
}

#[when("I mark the job as running")]
fn mark_running(world: &mut DrWorld) {
    let id = world.last_id.clone();
    world.queue.mark_running(&id);
}

#[when(expr = "I mark the job as completed with output {string}")]
fn mark_completed(world: &mut DrWorld, output: String) {
    let id = world.last_id.clone();
    world.queue.mark_completed(&id, &output);
}

#[when("I dequeue the next job")]
fn dequeue_next(world: &mut DrWorld) {
    if let Some(job) = world.queue.dequeue_next() {
        world.dequeued_prompt = Some(job.prompt);
    }
}

/// Mark running by looking up the job whose prompt matches `prompt_key`.
#[when(expr = "I mark job {string} as running")]
fn mark_named_running(world: &mut DrWorld, prompt_key: String) {
    let id = world
        .prompt_to_id
        .get(&prompt_key)
        .cloned()
        .unwrap_or_else(|| prompt_key.clone());
    world.queue.mark_running(&id);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("polling the job should return status Queued")]
fn assert_queued(world: &mut DrWorld) {
    let id = world.last_id.clone();
    let r = world.queue.poll(&id).unwrap();
    assert_eq!(r.status, JobStatus::Queued, "expected Queued");
}

#[then("polling the job should return status Completed")]
fn assert_completed(world: &mut DrWorld) {
    let id = world.last_id.clone();
    let r = world.queue.poll(&id).unwrap();
    assert!(matches!(r.status, JobStatus::Completed(_)), "expected Completed, got {:?}", r.status);
}

#[then(expr = "dequeuing the next job should return prompt {string}")]
fn assert_dequeued_prompt(world: &mut DrWorld, expected: String) {
    if let Some(job) = world.queue.dequeue_next() {
        assert_eq!(job.prompt, expected, "wrong prompt dequeued");
    } else {
        panic!("no job was dequeued");
    }
}

#[then(expr = "the pending count should be {int}")]
fn assert_pending(world: &mut DrWorld, expected: usize) {
    assert_eq!(world.queue.pending_count(), expected, "pending count mismatch");
}

fn main() {
    futures::executor::block_on(DrWorld::run("tests/features/dispatch_remote.feature"));
}
