/*!
 * BDD tests for hook_abort using Cucumber.
 * Run with: cargo test --test hook_abort_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::hook_abort::{
    AbortSignal, HookAbortController, HookOutput, HookParser, HookProgressEvent,
};

#[derive(Debug, Default, World)]
pub struct HaWorld {
    // HookParser fields
    exit_code: i32,
    stdout: String,
    output: Option<HookOutput>,
    outputs: Vec<HookOutput>,
    aggregate: Option<HookOutput>,
    // AbortSignal / HookAbortController fields
    signal: Option<AbortSignal>,
    clone_signal: Option<AbortSignal>,
    controller: Option<HookAbortController>,
    received_events: Vec<String>,
}

// ── HookParser step definitions ───────────────────────────────────────────────

#[given(expr = "a hook exit code of {int}")]
fn set_exit(world: &mut HaWorld, code: i32) {
    world.exit_code = code;
}

#[given(expr = "hook stdout {string}")]
fn set_stdout(world: &mut HaWorld, stdout: String) {
    world.stdout = stdout;
}

#[given("an allow output")]
fn add_allow(world: &mut HaWorld) {
    world.outputs.push(HookOutput::allow());
}

#[given(expr = "a blocking output with reason {string}")]
fn add_block(world: &mut HaWorld, reason: String) {
    world.outputs.push(HookOutput::block(reason));
}

#[when("I parse the hook output")]
fn parse(world: &mut HaWorld) {
    world.output = Some(HookParser::parse(world.exit_code, &world.stdout));
}

#[when("I aggregate the outputs")]
fn aggregate(world: &mut HaWorld) {
    world.aggregate = Some(HookParser::aggregate(&world.outputs));
}

#[then("the hook should be blocking")]
fn check_blocking(world: &mut HaWorld) {
    let out = world.output.as_ref().or(world.aggregate.as_ref()).unwrap();
    assert!(out.is_blocking(), "expected blocking but was not");
}

#[then("the hook should not be blocking")]
fn check_not_blocking(world: &mut HaWorld) {
    let out = world.output.as_ref().or(world.aggregate.as_ref()).unwrap();
    assert!(!out.is_blocking(), "expected non-blocking but was blocking");
}

#[then(expr = "the message should contain {string}")]
fn check_message(world: &mut HaWorld, needle: String) {
    let out = world.output.as_ref().or(world.aggregate.as_ref()).unwrap();
    let msg = out.message.as_deref().unwrap_or("");
    assert!(msg.contains(&needle), "message '{}' does not contain '{}'", msg, needle);
}

// ── AbortSignal step definitions ──────────────────────────────────────────────

#[given("a new AbortSignal")]
fn new_signal(world: &mut HaWorld) {
    world.signal = Some(AbortSignal::new());
}

#[given("a new AbortSignal with a clone")]
fn signal_with_clone(world: &mut HaWorld) {
    let s = AbortSignal::new();
    world.clone_signal = Some(s.clone_signal());
    world.signal = Some(s);
}

#[given("a HookAbortController")]
fn new_controller(world: &mut HaWorld) {
    world.controller = Some(HookAbortController::new());
}

#[when("I abort the original signal")]
fn abort_signal(world: &mut HaWorld) {
    world.signal.as_ref().unwrap().abort();
}

#[when(expr = "I emit Started, Running, and Completed events for hook {string}")]
fn emit_events(world: &mut HaWorld, hook_name: String) {
    if let Some(ctrl) = world.controller.as_mut() {
        let rx = ctrl.take_receiver().unwrap();
        ctrl.emit(HookProgressEvent::Started { hook_name: hook_name.clone() });
        ctrl.emit(HookProgressEvent::Running { hook_name: hook_name.clone(), elapsed_ms: 50 });
        ctrl.emit(HookProgressEvent::Completed { hook_name: hook_name.clone(), success: true });
        world.received_events = rx.try_iter().map(|e| e.to_string()).collect();
    }
}

#[then("is_aborted should return false")]
fn check_not_aborted(world: &mut HaWorld) {
    assert!(!world.signal.as_ref().unwrap().is_aborted());
}

#[then("the clone should also be aborted")]
fn check_clone_aborted(world: &mut HaWorld) {
    assert!(world.clone_signal.as_ref().unwrap().is_aborted());
}

#[then(expr = "{int} events should be received on the channel")]
fn check_event_count(world: &mut HaWorld, expected: usize) {
    assert_eq!(
        world.received_events.len(),
        expected,
        "received: {:?}",
        world.received_events
    );
}

fn main() {
    futures::executor::block_on(HaWorld::run("tests/features/hook_abort.feature"));
}
