/*!
 * BDD tests for watch_session_relay — compact Watch payloads and replay prevention.
 * Run with: cargo test --test watch_session_relay_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::watch_session_relay::{
    to_watch_event_json, to_watch_message, to_watch_summary,
    truncate, MessageRowView, NonceRegistry, SessionRowView,
    WatchSandboxStatus,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct RelayWorld {
    input_str: String,
    truncated: String,
    max_len: usize,
    payload: serde_json::Value,
    event_kind: String,
    event_delta: Option<String>,
    event_tool: Option<String>,
    event_status: Option<String>,
    event_error: Option<String>,
    event_step: Option<u32>,
    nonce_reg: Option<NonceRegistry>,
    current_ts: u64,
    record_error: Option<String>,
    last_nonce: String,
    message_preview: String,
    message_count: u32,
    watch_message_content: String,
    // session summary
    session_id: String,
    messages: Vec<(String, String, u64)>, // (role, content, created_at)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ── Given steps ──────────────────────────────────────────────────────────────

#[given(expr = "a string {string} of length {int}")]
fn given_string(_world: &mut RelayWorld, _s: String, _len: u32) {
    // length is just documentation; the string IS the test input
    _world.input_str = _s;
}

#[given(expr = "a string of {int} repeated {string} characters")]
fn given_repeated_string(world: &mut RelayWorld, count: u32, ch: String) {
    let c = ch.chars().next().unwrap_or('a');
    world.input_str = std::iter::repeat(c).take(count as usize).collect();
}

#[given(expr = "an SSE payload with type {string} and text {string}")]
fn given_sse_delta(world: &mut RelayWorld, typ: String, text: String) {
    world.payload = serde_json::json!({ "type": typ, "text": text });
}

#[given(expr = "an SSE payload with type {string} and name {string} and step {word}")]
fn given_sse_tool_start(world: &mut RelayWorld, typ: String, name: String, step_str: String) {
    let step: u32 = step_str.parse().unwrap_or(0);
    world.payload = serde_json::json!({ "type": typ, "name": name, "step": step });
}

#[given(expr = "an SSE payload with type {string} and name {string} and success true")]
fn given_sse_tool_end_ok(world: &mut RelayWorld, typ: String, name: String) {
    world.payload = serde_json::json!({ "type": typ, "name": name, "success": true });
}

#[given(expr = "an SSE payload with type {string} and name {string} and success false")]
fn given_sse_tool_end_fail(world: &mut RelayWorld, typ: String, name: String) {
    world.payload = serde_json::json!({ "type": typ, "name": name, "success": false });
}

#[given(expr = "an SSE payload with type {string} and status {string}")]
fn given_sse_done(world: &mut RelayWorld, typ: String, status: String) {
    world.payload = serde_json::json!({ "type": typ, "status": status });
}

#[given(expr = "an SSE payload with type {string} and a {int}-character message")]
fn given_sse_error(world: &mut RelayWorld, typ: String, len: u32) {
    let msg: String = "e".repeat(len as usize);
    world.payload = serde_json::json!({ "type": typ, "message": msg });
}

#[given(expr = "an SSE payload with type {string}")]
fn given_sse_unknown(world: &mut RelayWorld, typ: String) {
    world.payload = serde_json::json!({ "type": typ });
}

#[given("a NonceRegistry")]
fn given_nonce_registry(world: &mut RelayWorld) {
    world.nonce_reg = Some(NonceRegistry::new());
}

#[given("the current timestamp")]
fn given_current_ts(world: &mut RelayWorld) {
    world.current_ts = now_unix();
}

#[given(expr = "a session with 3 messages: assistant {string} and second user {string}")]
fn given_session_messages(world: &mut RelayWorld, asst_msg: String, user_msg2: String) {
    world.messages = vec![
        ("user".into(), "hello".into(), 1_700_000_001),
        ("assistant".into(), asst_msg, 1_700_000_002),
        ("user".into(), user_msg2, 1_700_000_003),
    ];
    world.session_id = "sess-bdd-1".into();
}

#[given(expr = "a message row with {int} characters of content")]
fn given_long_message(world: &mut RelayWorld, len: u32) {
    world.input_str = "x".repeat(len as usize);
}

// ── When steps ───────────────────────────────────────────────────────────────

#[when(expr = "I truncate it to {int} characters")]
fn when_truncate(world: &mut RelayWorld, max: u32) {
    world.max_len = max as usize;
    world.truncated = truncate(&world.input_str, max as usize);
}

#[when("I convert it to a WatchAgentEvent")]
fn when_convert_event(world: &mut RelayWorld) {
    let ev = to_watch_event_json(&world.payload);
    world.event_kind = ev.kind;
    world.event_delta = ev.delta;
    world.event_tool = ev.tool;
    world.event_status = ev.status;
    world.event_error = ev.error;
    world.event_step = ev.step;
}

#[when(expr = "I record nonce {string}")]
fn when_record_nonce(world: &mut RelayWorld, nonce: String) {
    let ts = world.current_ts;
    let result = world.nonce_reg.as_ref().unwrap().check_and_record(&nonce, ts);
    world.record_error = result.err().map(|e| e.to_string());
    world.last_nonce = nonce;
}

#[when(expr = "I record nonces {string}, {string}, and {string}")]
fn when_record_three_nonces(world: &mut RelayWorld, n1: String, n2: String, n3: String) {
    let ts = world.current_ts;
    let reg = world.nonce_reg.as_ref().unwrap();
    for n in [&n1, &n2, &n3] {
        reg.check_and_record(n, ts).unwrap();
    }
}

#[when("I record a nonce with a timestamp 60 seconds in the past")]
fn when_record_stale_nonce(world: &mut RelayWorld) {
    let stale_ts = now_unix().saturating_sub(60);
    let result = world.nonce_reg.as_ref().unwrap()
        .check_and_record("stale-nonce", stale_ts);
    world.record_error = result.err().map(|e| e.to_string());
}

#[when("I compute the WatchSessionSummary")]
fn when_compute_summary(world: &mut RelayWorld) {
    let session = SessionRowView {
        id: &world.session_id,
        task: "BDD test session",
        status: "running",
        provider: "claude",
        model: "claude-opus-4-6",
        step_count: 2,
        started_at: 1_700_000_000,
    };
    let msg_views: Vec<MessageRowView<'_>> = world.messages.iter().map(|(role, content, ts)| {
        MessageRowView { id: 0, role, content, created_at: *ts }
    }).collect();
    let summary = to_watch_summary(&session, &msg_views);
    world.message_preview = summary.last_message_preview;
    world.message_count = summary.message_count;
}

#[when("I convert it to a WatchMessage")]
fn when_convert_watch_message(world: &mut RelayWorld) {
    let row = MessageRowView {
        id: 1,
        role: "assistant",
        content: &world.input_str,
        created_at: 0,
    };
    let wm = to_watch_message(&row);
    world.watch_message_content = wm.content;
}

// ── Then steps ───────────────────────────────────────────────────────────────

#[then(expr = "the result should equal {string}")]
fn then_result_equals(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.truncated, expected);
}

#[then(expr = "the result should be exactly {int} characters")]
fn then_result_length(world: &mut RelayWorld, len: u32) {
    assert_eq!(world.truncated.chars().count(), len as usize);
}

#[then(expr = "the result should end with {string}")]
fn then_result_ends_with(world: &mut RelayWorld, suffix: String) {
    let c = suffix.chars().next().unwrap();
    assert!(world.truncated.ends_with(c),
        "expected to end with '{}', got: {}", suffix, world.truncated);
}

#[then(expr = "the event kind should be {string}")]
fn then_event_kind(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.event_kind, expected);
}

#[then(expr = "the event delta should be {string}")]
fn then_event_delta(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.event_delta.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the event tool should be {string}")]
fn then_event_tool(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.event_tool.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the event step should be {int}")]
fn then_event_step(world: &mut RelayWorld, expected: u32) {
    assert_eq!(world.event_step, Some(expected));
}

#[then(expr = "the event status should be {string}")]
fn then_event_status(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.event_status.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the event error should be at most {int} characters")]
fn then_event_error_max_len(world: &mut RelayWorld, max: u32) {
    let err = world.event_error.as_ref().expect("expected error field");
    assert!(err.chars().count() <= max as usize,
        "error too long: {} chars", err.chars().count());
}

#[then(expr = "the event error should end with {string}")]
fn then_event_error_ends(world: &mut RelayWorld, suffix: String) {
    let c = suffix.chars().next().unwrap();
    let err = world.event_error.as_ref().unwrap();
    assert!(err.ends_with(c), "expected error to end with '{}'", suffix);
}

#[then(expr = "recording the same nonce again should fail with {string}")]
fn then_replay_fails(world: &mut RelayWorld, needle: String) {
    // Try recording the same nonce a second time — must fail
    let ts = world.current_ts;
    let nonce = world.last_nonce.clone();
    let result = world.nonce_reg.as_ref().unwrap().check_and_record(&nonce, ts);
    let err = result.err().expect("expected replay error");
    assert!(err.to_string().contains(&needle),
        "expected '{}' in: {}", needle, err);
}

#[then("all three should be accepted")]
fn then_three_accepted(world: &mut RelayWorld) {
    assert!(world.record_error.is_none(), "expected no error for distinct nonces");
}

#[then(expr = "it should fail with {string}")]
fn then_fails_with(world: &mut RelayWorld, needle: String) {
    let err = world.record_error.as_ref().expect("expected error");
    assert!(err.contains(&needle), "expected '{}' in: {}", needle, err);
}

#[then(expr = "the last_message_preview should be {string}")]
fn then_preview(world: &mut RelayWorld, expected: String) {
    assert_eq!(world.message_preview, expected);
}

#[then(expr = "the message_count should be {int}")]
fn then_message_count(world: &mut RelayWorld, expected: u32) {
    assert_eq!(world.message_count, expected);
}

#[then(expr = "the content should be at most {int} characters")]
fn then_content_max(world: &mut RelayWorld, max: u32) {
    assert!(world.watch_message_content.chars().count() <= max as usize);
}

#[then("the content should end with \"…\"")]
fn then_content_ellipsis(world: &mut RelayWorld) {
    assert!(world.watch_message_content.ends_with('…'));
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(RelayWorld::run(
        "tests/features/watch_session_relay.feature",
    ));
}
