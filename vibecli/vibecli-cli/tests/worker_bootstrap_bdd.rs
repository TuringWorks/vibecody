/*!
 * BDD tests for worker_bootstrap (six-state lifecycle) using Cucumber.
 * Run with: cargo test --test worker_bootstrap_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::worker_bootstrap::{WorkerLifecycle, WorkerState};

#[derive(Debug, Default, World)]
pub struct WbWorld {
    worker: Option<WorkerLifecycle>,
    transition_error: Option<String>,
    readiness: Option<bool>,
}

fn state_from(s: &str) -> WorkerState {
    match s {
        "spawning"         => WorkerState::Spawning,
        "trust_required"   => WorkerState::TrustRequired,
        "ready_for_prompt" => WorkerState::ReadyForPrompt,
        "running"          => WorkerState::Running,
        "finished"         => WorkerState::Finished,
        "failed"           => WorkerState::Failed,
        _                  => WorkerState::Spawning,
    }
}

#[given(expr = "a new worker {string} with task {string}")]
fn new_worker(world: &mut WbWorld, id: String, task: String) {
    world.worker = Some(WorkerLifecycle::new(&id, &task, "/project"));
}

#[given(expr = "a worker that has reached {string} state")]
fn worker_in_state(world: &mut WbWorld, state_str: String) {
    let mut w = WorkerLifecycle::new("w", "task", "/");
    let target = state_from(&state_str);
    let path = match target {
        WorkerState::Finished => vec![
            WorkerState::ReadyForPrompt,
            WorkerState::Running,
            WorkerState::Finished,
        ],
        WorkerState::Running => vec![
            WorkerState::ReadyForPrompt,
            WorkerState::Running,
        ],
        _ => vec![target.clone()],
    };
    for s in path {
        let _ = w.transition(s);
    }
    world.worker = Some(w);
}

#[given(expr = "output line {string}")]
fn output_line(world: &mut WbWorld, line: String) {
    world.readiness = Some(WorkerLifecycle::detect_readiness(&line));
}

#[when(expr = "I transition to {string}")]
fn do_transition(world: &mut WbWorld, state_str: String) {
    if let Some(w) = world.worker.as_mut() {
        match w.transition(state_from(&state_str)) {
            Ok(_)  => world.transition_error = None,
            Err(e) => world.transition_error = Some(e),
        }
    }
}

#[when(expr = "I try to transition to {string}")]
fn try_transition(world: &mut WbWorld, state_str: String) {
    do_transition(world, state_str);
}

#[then(expr = "its state should be {string}")]
fn check_state(world: &mut WbWorld, expected: String) {
    let state = world.worker.as_ref().unwrap().state.to_string();
    assert_eq!(state, expected);
}

#[then(expr = "the state should be {string}")]
fn check_state_after(world: &mut WbWorld, expected: String) {
    check_state(world, expected);
}

#[then("the transition should fail with an error")]
fn check_fail(world: &mut WbWorld) {
    assert!(world.transition_error.is_some(), "expected transition to fail");
}

#[then("detect_readiness should return false")]
fn check_not_ready(world: &mut WbWorld) {
    assert!(!world.readiness.unwrap_or(true));
}

#[then("detect_readiness should return true")]
fn check_ready(world: &mut WbWorld) {
    assert!(world.readiness.unwrap_or(false));
}

fn main() {
    futures::executor::block_on(WbWorld::run(
        "tests/features/worker_bootstrap.feature",
    ));
}
