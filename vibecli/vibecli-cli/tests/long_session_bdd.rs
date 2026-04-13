/*!
 * BDD tests for extended session management using Cucumber.
 * Run with: cargo test --test long_session_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::long_session::{
    ContinuationDecision, SessionBudget, SessionManager, SessionState,
};

#[derive(Debug, Default, World)]
pub struct LsWorld {
    state: Option<SessionState>,
    decision: Option<ContinuationDecision>,
    remaining: Option<SessionBudget>,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "a new session started at time {int}")]
fn new_session(world: &mut LsWorld, start: u64) {
    world.state = Some(SessionState::new("bdd-session", start));
}

#[given(expr = "the session has used {int} tokens in {int} turn")]
fn used_tokens(world: &mut LsWorld, tokens: u64, _turns: u32) {
    if let Some(s) = world.state.as_mut() {
        s.record_turn(tokens, 0);
    }
}

#[given(expr = "the session has used {int} tokens in {int} turns")]
fn used_tokens_plural(world: &mut LsWorld, tokens: u64, _turns: u32) {
    if let Some(s) = world.state.as_mut() {
        s.record_turn(tokens, 0);
    }
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I check the decision at time {int}")]
fn check_decision(world: &mut LsWorld, now: u64) {
    let mgr = SessionManager::with_defaults();
    if let Some(s) = &world.state {
        world.decision = Some(mgr.decide(s, now));
    }
}

#[when(expr = "I compute budget remaining at time {int}")]
fn compute_remaining(world: &mut LsWorld, now: u64) {
    let mgr = SessionManager::with_defaults();
    if let Some(s) = &world.state {
        world.remaining = Some(mgr.budget_remaining(s, now));
    }
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the decision should be Continue")]
fn assert_continue(world: &mut LsWorld) {
    assert_eq!(
        world.decision.as_ref().unwrap(),
        &ContinuationDecision::Continue,
        "expected Continue"
    );
}

#[then("the decision should be CompactAndContinue")]
fn assert_compact(world: &mut LsWorld) {
    assert_eq!(
        world.decision.as_ref().unwrap(),
        &ContinuationDecision::CompactAndContinue,
        "expected CompactAndContinue"
    );
}

#[then("the decision should be Halt")]
fn assert_halt(world: &mut LsWorld) {
    assert!(
        matches!(world.decision.as_ref().unwrap(), ContinuationDecision::Halt(_)),
        "expected Halt, got {:?}",
        world.decision
    );
}

#[then(expr = "remaining tokens should be {int}")]
fn assert_remaining_tokens(world: &mut LsWorld, expected: u64) {
    let r = world.remaining.as_ref().unwrap();
    assert_eq!(r.max_tokens, expected, "remaining tokens mismatch");
}

fn main() {
    futures::executor::block_on(LsWorld::run("tests/features/long_session.feature"));
}
