/*!
 * BDD tests for MockAIProvider using Cucumber.
 * Run with: cargo test --test mock_provider_bdd
 */

use cucumber::{World, given, then, when};
use vibe_ai::mock_provider::MockAIProvider;
use vibe_ai::provider::{AIProvider, Message, MessageRole};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn msg(content: &str) -> Message {
    Message {
        role: MessageRole::User,
        content: content.to_string(),
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct MpWorld {
    provider: Option<MockAIProvider>,
    responses: Vec<String>,
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(expr = "a mock provider with responses {string} and {string}")]
fn with_two_responses(world: &mut MpWorld, r1: String, r2: String) {
    world.provider = Some(MockAIProvider::with_responses(
        "bdd-mock",
        vec![r1.as_str(), r2.as_str()],
    ));
}

#[given(expr = "a mock provider with {int} responses")]
fn with_n_responses(world: &mut MpWorld, n: usize) {
    let pool = vec!["a", "b", "c", "d", "e"];
    let resps = pool[..n.min(pool.len())].to_vec();
    world.provider = Some(MockAIProvider::with_responses("bdd-mock", resps));
}

#[given("a mock provider configured as unavailable")]
fn unavailable_provider(world: &mut MpWorld) {
    let mut p = MockAIProvider::new("unavail");
    p.set_available(false);
    world.provider = Some(p);
}

#[given(expr = "a mock provider with scenario prefix {string} returning {string}")]
fn with_scenario(world: &mut MpWorld, prefix: String, response: String) {
    world.provider = Some(MockAIProvider::with_scenarios(
        "bdd-mock",
        vec![(prefix.as_str(), response.as_str())],
    ));
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when("I call chat twice")]
async fn call_twice(world: &mut MpWorld) {
    if let Some(p) = &world.provider {
        let r1 = p.chat(&[msg("q")], None).await.unwrap_or_default();
        let r2 = p.chat(&[msg("q")], None).await.unwrap_or_default();
        world.responses = vec![r1, r2];
    }
}

#[when(expr = "I call chat {int} times")]
async fn call_n_times(world: &mut MpWorld, n: usize) {
    if let Some(p) = &world.provider {
        world.responses.clear();
        for _ in 0..n {
            let r = p.chat(&[msg("q")], None).await.unwrap_or_default();
            world.responses.push(r);
        }
    }
}

#[when(expr = "I call chat with message {string}")]
async fn call_with_msg(world: &mut MpWorld, content: String) {
    if let Some(p) = &world.provider {
        let r = p.chat(&[msg(&content)], None).await.unwrap_or_default();
        world.responses = vec![r];
    }
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(expr = "the first response should be {string}")]
fn check_first(world: &mut MpWorld, expected: String) {
    assert_eq!(world.responses.get(0).unwrap(), &expected);
}

#[then(expr = "the second response should be {string}")]
fn check_second(world: &mut MpWorld, expected: String) {
    assert_eq!(world.responses.get(1).unwrap(), &expected);
}

#[then("is_available should return false")]
async fn check_unavailable(world: &mut MpWorld) {
    if let Some(p) = &world.provider {
        assert!(!p.is_available().await);
    }
}

#[then(expr = "call_count should be {int}")]
fn check_count(world: &mut MpWorld, expected: usize) {
    if let Some(p) = &world.provider {
        assert_eq!(p.call_count(), expected);
    }
}

#[then(expr = "the response should be {string}")]
fn check_response(world: &mut MpWorld, expected: String) {
    assert_eq!(world.responses.get(0).unwrap(), &expected);
}

// ── Runner ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    MpWorld::run("tests/features/mock_provider.feature").await;
}
