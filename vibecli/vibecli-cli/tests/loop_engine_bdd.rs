/*!
 * BDD coverage for the /loop engine (gap C1).
 * Run with: cargo test --test loop_engine_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::loop_engine::{parse_loop_args, LoopDecision, LoopJob, LoopMode, LoopSpec};

#[derive(Debug, Default, World)]
#[allow(dead_code)]
pub struct LoopWorld {
    arg: String,
    parsed: Option<Result<LoopSpec, String>>,
    job: Option<LoopJob>,
    decision: Option<LoopDecision>,
}

// ── Given ─────────────────────────────────────────────────────────────────

#[given(expr = "the loop argument {string}")]
fn set_arg(w: &mut LoopWorld, arg: String) {
    w.arg = arg;
}

#[given(expr = "a self-paced job from {string}")]
fn self_paced_job(w: &mut LoopWorld, arg: String) {
    let spec = parse_loop_args(&arg).expect("self-paced spec parses");
    w.job = Some(LoopJob::new("loop-test".into(), spec, 0));
}

#[given(expr = "a self-paced job from {string} with max duration {int} seconds")]
fn self_paced_job_with_budget(w: &mut LoopWorld, arg: String, secs: u64) {
    let mut spec = parse_loop_args(&arg).expect("self-paced spec parses");
    spec.max_duration_secs = secs;
    w.job = Some(LoopJob::new("loop-test".into(), spec, 0));
}

#[given(expr = "the job has run {int} iterations")]
fn set_iterations(w: &mut LoopWorld, n: u32) {
    w.job.as_mut().unwrap().iterations_done = n;
}

// ── When ──────────────────────────────────────────────────────────────────

#[when("I parse the loop arguments")]
fn do_parse(w: &mut LoopWorld) {
    w.parsed = Some(parse_loop_args(&w.arg));
}

#[when(expr = "the validator reports done at elapsed {int} seconds")]
fn validator_done(w: &mut LoopWorld, elapsed: u64) {
    let job = w.job.as_ref().unwrap();
    w.decision = Some(job.decide_next(true, elapsed));
}

#[when(expr = "the validator reports not-done at elapsed {int} seconds")]
fn validator_not_done(w: &mut LoopWorld, elapsed: u64) {
    let job = w.job.as_ref().unwrap();
    w.decision = Some(job.decide_next(false, elapsed));
}

// ── Then ──────────────────────────────────────────────────────────────────

#[then("parsing succeeds")]
fn parse_ok(w: &mut LoopWorld) {
    assert!(
        matches!(w.parsed, Some(Ok(_))),
        "expected Ok, got {:?}",
        w.parsed
    );
}

#[then("parsing fails")]
fn parse_err(w: &mut LoopWorld) {
    assert!(
        matches!(w.parsed, Some(Err(_))),
        "expected Err, got {:?}",
        w.parsed
    );
}

#[then(expr = "the mode is recurring with interval {int} seconds")]
fn mode_recurring(w: &mut LoopWorld, secs: u64) {
    let spec = w.parsed.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        spec.mode,
        LoopMode::Recurring {
            interval_secs: secs
        }
    );
}

#[then("the mode is self-paced")]
fn mode_self_paced(w: &mut LoopWorld) {
    let spec = w.parsed.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(spec.mode, LoopMode::SelfPaced);
}

#[then(expr = "the prompt is {string}")]
fn prompt_is(w: &mut LoopWorld, expected: String) {
    let spec = w.parsed.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(spec.prompt, expected);
}

#[then("the decision is stop-done")]
fn decision_done(w: &mut LoopWorld) {
    assert_eq!(w.decision, Some(LoopDecision::StopDone));
}

#[then("the decision is stop-max-iter")]
fn decision_max_iter(w: &mut LoopWorld) {
    assert_eq!(w.decision, Some(LoopDecision::StopMaxIter));
}

#[then("the decision is stop-expired")]
fn decision_expired(w: &mut LoopWorld) {
    assert_eq!(w.decision, Some(LoopDecision::StopExpired));
}

fn main() {
    futures::executor::block_on(LoopWorld::run("tests/features/loop_engine.feature"));
}
