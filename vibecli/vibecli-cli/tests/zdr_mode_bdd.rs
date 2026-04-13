/*!
 * BDD tests for zdr_mode using Cucumber 0.20.
 * Run with: cargo test --test zdr_mode_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::zdr_mode::{
    ZdrCompliance, ZdrPolicy, ZdrSession, scrub_pii,
};

// ─── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct ZdrWorld {
    policy: Option<ZdrPolicy>,
    session: Option<ZdrSession>,
    text_input: String,
    text_output: String,
    message_count: usize,
}

// ─── Given ────────────────────────────────────────────────────────────────────

#[given("a strict ZDR policy")]
fn given_strict_policy(world: &mut ZdrWorld) {
    world.policy = Some(ZdrPolicy::strict());
}

#[given("a permissive ZDR policy")]
fn given_permissive_policy(world: &mut ZdrWorld) {
    world.policy = Some(ZdrPolicy::permissive());
}

#[given("a ZDR session with strict policy")]
fn given_strict_session(world: &mut ZdrWorld) {
    world.session = Some(ZdrSession::new(ZdrPolicy::strict()));
    world.message_count = 0;
}

#[given(expr = "the text {string}")]
fn given_text(world: &mut ZdrWorld, text: String) {
    world.text_input = text.replace("\\\"", "\"");
}

// ─── When ─────────────────────────────────────────────────────────────────────

#[when(expr = "I add a {string} message {string}")]
fn when_add_message(world: &mut ZdrWorld, role: String, content: String) {
    let session = world.session.as_mut().expect("session not initialised");
    session.add_message(role, content, None);
    world.message_count = session.message_count();
}

#[when("I apply PII scrubbing")]
fn when_apply_pii(world: &mut ZdrWorld) {
    world.text_output = scrub_pii(&world.text_input);
}

#[when("I clear the session")]
fn when_clear_session(world: &mut ZdrWorld) {
    let session = world.session.as_mut().expect("session not initialised");
    session.clear();
    world.message_count = session.message_count();
}

// ─── Then ─────────────────────────────────────────────────────────────────────

#[then("the policy should be ZDR compliant")]
fn then_compliant(world: &mut ZdrWorld) {
    let policy = world.policy.as_ref().expect("policy not set");
    assert!(policy.is_zdr_compliant(), "expected policy to be ZDR compliant");
}

#[then("the policy should not be ZDR compliant")]
fn then_not_compliant(world: &mut ZdrWorld) {
    let policy = world.policy.as_ref().expect("policy not set");
    assert!(!policy.is_zdr_compliant(), "expected policy to NOT be ZDR compliant");
}

#[then(expr = "there should be at least {int} compliance violation")]
fn then_violation_count(world: &mut ZdrWorld, min: usize) {
    let policy = world.policy.as_ref().expect("policy not set");
    let compliance = ZdrCompliance::check(policy);
    assert!(
        compliance.violation_count() >= min,
        "expected >= {min} violations, got {}",
        compliance.violation_count()
    );
}

#[then(expr = "the built request should contain {int} messages")]
fn then_request_message_count(world: &mut ZdrWorld, expected: usize) {
    let session = world.session.as_ref().expect("session not initialised");
    let req = session.build_request();
    assert_eq!(
        req.messages.len(),
        expected,
        "expected {expected} messages in request, got {}",
        req.messages.len()
    );
}

#[then(expr = "the output should not contain {string}")]
fn then_output_not_contain(world: &mut ZdrWorld, fragment: String) {
    let fragment = fragment.replace("\\\"", "\"");
    assert!(
        !world.text_output.contains(&fragment),
        "output should NOT contain {:?}, but got: {:?}",
        fragment,
        world.text_output
    );
}

#[then(expr = "the output should contain {string}")]
fn then_output_contain(world: &mut ZdrWorld, fragment: String) {
    let fragment = fragment.replace("\\\"", "\"");
    assert!(
        world.text_output.contains(&fragment),
        "output should contain {:?}, but got: {:?}",
        fragment,
        world.text_output
    );
}

#[then(expr = "the session should have {int} messages")]
fn then_session_message_count(world: &mut ZdrWorld, expected: usize) {
    assert_eq!(
        world.message_count, expected,
        "expected {expected} messages, got {}",
        world.message_count
    );
}

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(ZdrWorld::run("tests/features/zdr_mode.feature"));
}
