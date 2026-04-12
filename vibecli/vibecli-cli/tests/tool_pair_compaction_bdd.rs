/*!
 * BDD tests for tool_pair_compaction using Cucumber.
 * Run with: cargo test --test tool_pair_compaction_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::tool_pair_compaction::{
    CompactionConfig, CompactionEngine, CompactionSummary, SimpleMessage, SimpleMessageRole,
};

fn user(c: &str) -> SimpleMessage {
    SimpleMessage::new(SimpleMessageRole::User, c)
}
fn assistant(c: &str) -> SimpleMessage {
    SimpleMessage::new(SimpleMessageRole::Assistant, c)
}
fn system(c: &str) -> SimpleMessage {
    SimpleMessage::new(SimpleMessageRole::System, c)
}
fn tool_use(n: &str) -> SimpleMessage {
    SimpleMessage::new(SimpleMessageRole::ToolUse, n)
}
fn tool_result(c: &str) -> SimpleMessage {
    SimpleMessage::new(SimpleMessageRole::ToolResult, c)
}

#[derive(Debug, Default, World)]
pub struct TpcWorld {
    messages: Vec<SimpleMessage>,
    raw_boundary: usize,
    safe_boundary: usize,
    summary: Option<CompactionSummary>,
    compacted: Vec<SimpleMessage>,
    continuation: Option<SimpleMessage>,
}

#[given("a conversation ending with a ToolUse at position 9 and ToolResult at position 10")]
fn conv_with_tool_pair(world: &mut TpcWorld) {
    world.messages = (0..9).map(|_| user("q")).collect();
    world.messages.push(tool_use("read_file")); // 9
    world.messages.push(tool_result("content")); // 10
}

#[given(expr = "a raw compaction boundary of {int}")]
fn set_raw_boundary(world: &mut TpcWorld, b: usize) {
    world.raw_boundary = b;
}

#[given(expr = "a conversation with {int} user, {int} assistant, and {int} system message")]
fn conv_with_counts(world: &mut TpcWorld, u: usize, a: usize, s: usize) {
    world.messages.clear();
    for _ in 0..u {
        world.messages.push(user("q"));
    }
    for _ in 0..a {
        world.messages.push(assistant("a"));
    }
    for _ in 0..s {
        world.messages.push(system("sys"));
    }
}

#[given(expr = "a conversation with {int} user messages")]
fn conv_with_n_users(world: &mut TpcWorld, n: usize) {
    world.messages = (0..n).map(|i| user(&format!("request {i}"))).collect();
}

#[given("a conversation with interleaved ToolUse and ToolResult messages")]
fn conv_with_tool_pairs(world: &mut TpcWorld) {
    world.messages = vec![
        user("u1"),
        user("u2"),
        user("u3"),
        tool_use("bash"),
        tool_result("ok"),
    ];
}

#[given(expr = "a compaction summary with {int} user and {int} assistant message")]
fn summary_with_counts(world: &mut TpcWorld, u: usize, a: usize) {
    world.summary =
        Some(CompactionSummary { user_count: u, assistant_count: a, ..Default::default() });
}

#[when("I find the safe boundary")]
fn find_boundary(world: &mut TpcWorld) {
    world.safe_boundary =
        CompactionEngine::find_safe_boundary(&world.messages, world.raw_boundary);
}

#[when("I generate a compaction summary")]
fn gen_summary(world: &mut TpcWorld) {
    world.summary = Some(CompactionEngine::summarize(&world.messages));
}

#[when("I create the synthetic continuation")]
fn create_continuation(world: &mut TpcWorld) {
    let s = world.summary.clone().unwrap_or_default();
    world.continuation = Some(CompactionEngine::synthetic_continuation(&s));
}

#[when(expr = "I compact the conversation with keep_recent {int}")]
fn compact_conv(world: &mut TpcWorld, keep: usize) {
    let engine =
        CompactionEngine::new(CompactionConfig { keep_recent: keep, ..Default::default() });
    world.compacted = engine.compact(&world.messages);
}

#[then(expr = "it should be {int}")]
fn check_boundary(world: &mut TpcWorld, expected: usize) {
    assert_eq!(world.safe_boundary, expected);
}

#[then(expr = "user_count should be {int}")]
fn check_user_count(world: &mut TpcWorld, expected: usize) {
    assert_eq!(world.summary.as_ref().unwrap().user_count, expected);
}

#[then(expr = "assistant_count should be {int}")]
fn check_asst_count(world: &mut TpcWorld, expected: usize) {
    assert_eq!(world.summary.as_ref().unwrap().assistant_count, expected);
}

#[then("last_user_requests should contain exactly 3 entries")]
fn check_last_requests(world: &mut TpcWorld) {
    assert_eq!(world.summary.as_ref().unwrap().last_user_requests.len(), 3);
}

#[then(expr = "its role should be {string}")]
fn check_role(world: &mut TpcWorld, expected: String) {
    let role = world.continuation.as_ref().unwrap().role.to_string();
    assert_eq!(role, expected);
}

#[then("no ToolUse message should be followed by a non-ToolResult message")]
fn check_no_orphans(world: &mut TpcWorld) {
    let msgs = &world.compacted;
    for pair in msgs.windows(2) {
        if pair[0].role == SimpleMessageRole::ToolUse {
            assert_eq!(
                pair[1].role,
                SimpleMessageRole::ToolResult,
                "ToolUse was not followed by ToolResult"
            );
        }
    }
}

fn main() {
    futures::executor::block_on(
        TpcWorld::run("tests/features/tool_pair_compaction.feature"),
    );
}
