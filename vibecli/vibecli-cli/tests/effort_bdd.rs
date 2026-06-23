/*!
 * BDD coverage for the per-request effort knob (gap C5).
 * Run with: cargo test --test effort_bdd
 *
 * `Effort` lives in vibe-ai; vibecli depends on it, so the harness drives the
 * real type the providers and the daemon use.
 */
use cucumber::{given, then, when, World};
use vibe_ai::provider::Effort;

#[derive(Debug, Default, World)]
#[allow(dead_code)]
pub struct EffortWorld {
    input: String,
    parsed: Option<Option<Effort>>,
}

fn effort(label: &str) -> Effort {
    Effort::parse(label).expect("known effort label")
}

// ── Given ─────────────────────────────────────────────────────────────────

#[given(expr = "the effort string {string}")]
fn set_input(w: &mut EffortWorld, s: String) {
    w.input = s;
}

// ── When ──────────────────────────────────────────────────────────────────

#[when("I parse the effort")]
fn do_parse(w: &mut EffortWorld) {
    w.parsed = Some(Effort::parse(&w.input));
}

// ── Then ──────────────────────────────────────────────────────────────────

#[then(expr = "the effort is {string}")]
fn effort_is(w: &mut EffortWorld, expected: String) {
    let parsed = w.parsed.unwrap().expect("should parse");
    assert_eq!(parsed.as_str(), expected);
}

#[then("the effort does not parse")]
fn effort_unparsed(w: &mut EffortWorld) {
    assert_eq!(w.parsed.unwrap(), None);
}

#[then(expr = "the Claude budget for {string} is none")]
fn claude_budget_none(_w: &mut EffortWorld, label: String) {
    assert_eq!(effort(&label).claude_thinking_budget(), None);
}

#[then(expr = "the Claude budget for {string} exceeds the budget for {string}")]
fn claude_budget_exceeds(_w: &mut EffortWorld, hi: String, lo: String) {
    let a = effort(&hi).claude_thinking_budget().unwrap();
    let b = effort(&lo).claude_thinking_budget().unwrap();
    assert!(a > b, "{hi}={a} should exceed {lo}={b}");
}

#[then(expr = "the OpenAI reasoning effort for {string} is {string}")]
fn openai_reasoning(_w: &mut EffortWorld, label: String, expected: String) {
    assert_eq!(effort(&label).openai_reasoning_effort(), expected);
}

#[then(expr = "the Gemini budget for {string} is {int}")]
fn gemini_budget_is(_w: &mut EffortWorld, label: String, expected: i32) {
    assert_eq!(effort(&label).gemini_thinking_budget(), expected);
}

#[then(expr = "the Gemini budget for {string} exceeds the budget for {string}")]
fn gemini_budget_exceeds(_w: &mut EffortWorld, hi: String, lo: String) {
    let a = effort(&hi).gemini_thinking_budget();
    let b = effort(&lo).gemini_thinking_budget();
    assert!(a > b, "{hi}={a} should exceed {lo}={b}");
}

fn main() {
    futures::executor::block_on(EffortWorld::run("tests/features/effort.feature"));
}
