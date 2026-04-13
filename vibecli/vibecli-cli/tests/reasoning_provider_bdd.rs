/*!
 * BDD tests for reasoning_provider using Cucumber.
 * Run with: cargo test --test reasoning_provider_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::reasoning_provider::{
    build_reasoning_response, parse_thinking_blocks, strip_thinking_from,
    token_budget_for_complexity, ModelTier, ReasoningBudget, ReasoningConfig, ReasoningResponse,
    ThinkingBlock,
};

#[derive(Debug, Default, World)]
pub struct RpWorld {
    raw: String,
    blocks: Vec<ThinkingBlock>,
    stripped: String,
    complexity: u8,
    budget: Option<ReasoningBudget>,
    strip_thinking: bool,
    response: Option<ReasoningResponse>,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "the raw response {string}")]
fn set_raw(world: &mut RpWorld, raw: String) {
    world.raw = raw;
}

#[given(expr = "a complexity level of {int}")]
fn set_complexity(world: &mut RpWorld, c: u8) {
    world.complexity = c;
}

#[given("strip thinking is enabled")]
fn enable_strip(world: &mut RpWorld) {
    world.strip_thinking = true;
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I parse thinking blocks")]
fn do_parse(world: &mut RpWorld) {
    world.blocks = parse_thinking_blocks(&world.raw);
}

#[when("I strip thinking blocks")]
fn do_strip(world: &mut RpWorld) {
    world.stripped = strip_thinking_from(&world.raw);
}

#[when("I compute the token budget")]
fn do_budget(world: &mut RpWorld) {
    world.budget = Some(token_budget_for_complexity(world.complexity));
}

#[when("I build the reasoning response")]
fn do_build(world: &mut RpWorld) {
    let config = ReasoningConfig::new(ModelTier::Reasoning)
        .with_strip_thinking(world.strip_thinking);
    world.response = Some(build_reasoning_response(&world.raw, &config));
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "there should be {int} thinking block")]
fn check_block_count(world: &mut RpWorld, expected: usize) {
    assert_eq!(world.blocks.len(), expected);
}

#[then(expr = "the first block content should be {string}")]
fn check_first_block(world: &mut RpWorld, expected: String) {
    assert_eq!(world.blocks[0].content, expected);
}

#[then(expr = "the result should contain {string}")]
fn check_contains(world: &mut RpWorld, expected: String) {
    assert!(
        world.stripped.contains(&expected),
        "stripped {:?} does not contain {:?}",
        world.stripped,
        expected
    );
}

#[then(expr = "the result should not contain {string}")]
fn check_not_contains(world: &mut RpWorld, expected: String) {
    assert!(
        !world.stripped.contains(&expected),
        "stripped {:?} should not contain {:?}",
        world.stripped,
        expected
    );
}

#[then(expr = "the thinking token budget should be {int}")]
fn check_budget(world: &mut RpWorld, expected: u32) {
    let b = world.budget.as_ref().expect("budget not computed");
    assert_eq!(b.max_thinking_tokens, expected);
}

#[then(expr = "the response field should not contain {string}")]
fn check_resp_not_contains(world: &mut RpWorld, expected: String) {
    let resp = world.response.as_ref().expect("response not built");
    assert!(
        !resp.response.contains(&expected),
        "response {:?} should not contain {:?}",
        resp.response,
        expected
    );
}

#[then(expr = "the response field should contain {string}")]
fn check_resp_contains(world: &mut RpWorld, expected: String) {
    let resp = world.response.as_ref().expect("response not built");
    assert!(
        resp.response.contains(&expected),
        "response {:?} does not contain {:?}",
        resp.response,
        expected
    );
}

fn main() {
    futures::executor::block_on(RpWorld::run(
        "tests/features/reasoning_provider.feature",
    ));
}
