/*!
 * BDD tests for rpc_mode using Cucumber.
 * Run with: cargo test --test rpc_mode_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::rpc_mode::{MemoryTransport, RpcFrame, RpcReader};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct RpcWorld {
    /// The frame under test (serialise scenario).
    frame: Option<RpcFrame>,
    /// The serialised JSONL line.
    line: String,
    /// Result of parsing a JSONL line.
    parse_result: Option<Result<RpcFrame, String>>,
    /// Frames collected by an RpcReader.
    collected: Vec<RpcFrame>,
    /// The shared MemoryTransport.
    transport: Option<std::sync::Arc<MemoryTransport>>,
    /// Frames popped from the transport outbound buffer.
    popped: Vec<RpcFrame>,
}

// ---------------------------------------------------------------------------
// Scenario: Serialised frame ends with LF not CRLF
// ---------------------------------------------------------------------------

#[given(expr = "a token_delta frame with text {string}")]
fn given_token_delta(world: &mut RpcWorld, text: String) {
    world.frame = Some(RpcFrame::token_delta(&text));
}

#[when("I serialise the frame to a JSONL line")]
fn when_serialise(world: &mut RpcWorld) {
    world.line = world.frame.as_ref().expect("frame not set").to_jsonl();
}

#[then("the line ends with LF")]
fn then_ends_with_lf(world: &mut RpcWorld) {
    assert!(
        world.line.ends_with('\n'),
        "expected LF at end, got: {:?}",
        world.line
    );
}

#[then("the line does not end with CRLF")]
fn then_not_crlf(world: &mut RpcWorld) {
    assert!(
        !world.line.ends_with("\r\n"),
        "line ends with CRLF: {:?}",
        world.line
    );
}

#[then(expr = "the line contains the text {string}")]
fn then_contains_text(world: &mut RpcWorld, text: String) {
    assert!(
        world.line.contains(&text),
        "line {:?} does not contain {:?}",
        world.line,
        text
    );
}

// ---------------------------------------------------------------------------
// Scenario: Deserialise a well-formed JSONL line
// ---------------------------------------------------------------------------

#[given(expr = "a JSONL line with type {string} and id {string}")]
fn given_jsonl_line(world: &mut RpcWorld, msg_type: String, id: String) {
    world.line = format!("{{\"type\":\"{msg_type}\",\"id\":\"{id}\"}}\n");
}

#[when("I parse the line into an RPC frame")]
fn when_parse(world: &mut RpcWorld) {
    world.parse_result = Some(RpcFrame::from_line(&world.line));
}

#[then(expr = "the frame type is {string}")]
fn then_frame_type(world: &mut RpcWorld, expected: String) {
    let frame = world
        .parse_result
        .as_ref()
        .expect("no parse result")
        .as_ref()
        .expect("parse failed");
    assert_eq!(frame.msg_type, expected);
}

#[then(expr = "the frame field {string} is {string}")]
fn then_frame_field(world: &mut RpcWorld, field: String, expected: String) {
    let frame = world
        .parse_result
        .as_ref()
        .expect("no parse result")
        .as_ref()
        .expect("parse failed");
    assert_eq!(
        frame.get_str(&field),
        Some(expected.as_str()),
        "field {:?} mismatch",
        field
    );
}

// ---------------------------------------------------------------------------
// Scenario: Missing type field returns an error
// ---------------------------------------------------------------------------

#[given("a JSONL line without a \"type\" field")]
fn given_no_type_field(world: &mut RpcWorld) {
    world.line = r#"{"content":"no type here"}"#.to_string();
}

#[when("I attempt to parse the line")]
fn when_attempt_parse(world: &mut RpcWorld) {
    world.parse_result = Some(RpcFrame::from_line(&world.line));
}

#[then(expr = "parsing fails with an error containing {string}")]
fn then_parse_err_contains(world: &mut RpcWorld, fragment: String) {
    let result = world.parse_result.as_ref().expect("no parse result");
    assert!(result.is_err(), "expected Err, got Ok");
    let msg = result.as_ref().unwrap_err();
    assert!(
        msg.contains(&fragment),
        "error {:?} does not contain {:?}",
        msg,
        fragment
    );
}

// ---------------------------------------------------------------------------
// Scenario: RpcReader collects multiple frames
// ---------------------------------------------------------------------------

#[given("a byte stream containing 3 valid JSONL frames")]
fn given_three_frames(world: &mut RpcWorld) {
    let input = concat!(
        "{\"type\":\"send_message\",\"content\":\"hi\"}\n",
        "{\"type\":\"interrupt\"}\n",
        "{\"type\":\"shutdown\"}\n",
    );
    let cursor = std::io::BufReader::new(std::io::Cursor::new(input));
    let mut reader = RpcReader::new(cursor);
    world.collected = reader.collect_frames();
}

#[when("I collect all frames with an RpcReader")]
fn when_collect(_world: &mut RpcWorld) {
    // Collection already performed in the Given step above.
}

#[then(expr = "I receive exactly {int} frames")]
fn then_frame_count(world: &mut RpcWorld, count: usize) {
    assert_eq!(
        world.collected.len(),
        count,
        "expected {count} frames, got {}",
        world.collected.len()
    );
}

#[then(expr = "the first frame type is {string}")]
fn then_first_type(world: &mut RpcWorld, expected: String) {
    assert_eq!(world.collected[0].msg_type, expected);
}

#[then(expr = "the second frame type is {string}")]
fn then_second_type(world: &mut RpcWorld, expected: String) {
    assert_eq!(world.collected[1].msg_type, expected);
}

#[then(expr = "the third frame type is {string}")]
fn then_third_type(world: &mut RpcWorld, expected: String) {
    assert_eq!(world.collected[2].msg_type, expected);
}

// ---------------------------------------------------------------------------
// Scenario: Writer and MemoryTransport roundtrip
// ---------------------------------------------------------------------------

#[given("an empty MemoryTransport")]
fn given_empty_transport(world: &mut RpcWorld) {
    world.transport = Some(std::sync::Arc::new(MemoryTransport::new()));
}

#[when(expr = "I write a pong frame with id {string} through the transport writer")]
fn when_write_pong(world: &mut RpcWorld, id: String) {
    let transport = world.transport.as_ref().expect("no transport");
    let mut writer = transport.writer();
    writer
        .send(&RpcFrame::pong(&id))
        .expect("send failed");
}

#[when("I flush the transport writer")]
fn when_flush_writer(world: &mut RpcWorld) {
    // The MemoryWrite adapter flushes complete lines immediately on write; a
    // separate flush is a no-op but we call it to match the scenario wording.
    let transport = world.transport.as_ref().expect("no transport");
    let mut writer = transport.writer();
    writer.flush().expect("flush failed");
}

#[then(expr = "the transport outbound count is {int}")]
fn then_outbound_count(world: &mut RpcWorld, expected: usize) {
    let transport = world.transport.as_ref().expect("no transport");
    assert_eq!(transport.outbound_count(), expected);
}

#[then(expr = "the first popped outbound frame has type {string}")]
fn then_first_popped_type(world: &mut RpcWorld, expected: String) {
    let transport = world.transport.as_ref().expect("no transport");
    world.popped = transport.pop_outbound();
    assert!(
        !world.popped.is_empty(),
        "no outbound frames to pop"
    );
    assert_eq!(world.popped[0].msg_type, expected);
}

#[then(expr = "the first popped outbound frame field {string} is {string}")]
fn then_first_popped_field(world: &mut RpcWorld, field: String, expected: String) {
    assert!(
        !world.popped.is_empty(),
        "popped frames not yet populated — run the 'first popped outbound frame has type' step first"
    );
    assert_eq!(
        world.popped[0].get_str(&field),
        Some(expected.as_str()),
        "field {:?} mismatch",
        field
    );
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    RpcWorld::run("tests/features/rpc_mode.feature").await;
}
