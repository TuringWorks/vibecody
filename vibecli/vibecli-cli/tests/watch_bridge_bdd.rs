/*!
 * BDD tests for watch_bridge — Axum router state, response structures, replay.
 * Run with: cargo test --test watch_bridge_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::watch_bridge::{WatchBridgeState, WatchEventStreams};
use vibecli_cli::watch_session_relay::{
    NonceRegistry, WatchDispatchRequest, WatchDispatchResponse, WatchSandboxControlRequest,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct BridgeWorld {
    streams: Option<WatchEventStreams>,
    dispatch_response: Option<WatchDispatchResponse>,
    dispatch_request: Option<WatchDispatchRequest>,
    sandbox_req: Option<WatchSandboxControlRequest>,
    json: String,
    nonce_reg: Option<NonceRegistry>,
    current_ts: u64,
    record_error: Option<String>,
    last_nonce: String,
    size_of_state: usize,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ── Given steps ──────────────────────────────────────────────────────────────

#[given("a new WatchEventStreams map")]
fn given_new_streams(world: &mut BridgeWorld) {
    world.streams = Some(std::sync::Arc::new(std::sync::Mutex::new(
        std::collections::HashMap::new(),
    )));
}

#[given(expr = "a WatchDispatchResponse with session_id {string}")]
fn given_dispatch_response_session(world: &mut BridgeWorld, session_id: String) {
    world.dispatch_response = Some(WatchDispatchResponse {
        session_id: session_id.clone(),
        message_id: 1,
        streaming_url: format!("/watch/stream/{}", session_id),
    });
}

#[given(expr = "a WatchDispatchResponse with session_id {string} and message_id {int}")]
fn given_dispatch_response_full(world: &mut BridgeWorld, session_id: String, msg_id: i64) {
    world.dispatch_response = Some(WatchDispatchResponse {
        session_id: session_id.clone(),
        message_id: msg_id,
        streaming_url: format!("/watch/stream/{}", session_id),
    });
}

#[given("a NonceRegistry used by the bridge")]
fn given_nonce_registry(world: &mut BridgeWorld) {
    world.nonce_reg = Some(NonceRegistry::new());
}

#[given("the current Unix timestamp")]
fn given_current_ts(world: &mut BridgeWorld) {
    world.current_ts = now_unix();
}

#[given(expr = "a WatchSandboxControlRequest with action {string}")]
fn given_sandbox_req(world: &mut BridgeWorld, action: String) {
    world.sandbox_req = Some(WatchSandboxControlRequest {
        action,
        nonce: "nonce-001".into(),
        timestamp: 1_700_000_000,
    });
}

#[given("a WatchDispatchRequest with no session_id")]
fn given_dispatch_no_session(world: &mut BridgeWorld) {
    world.dispatch_request = Some(WatchDispatchRequest {
        session_id: None,
        content: "Start new session".into(),
        provider: None,
        nonce: "n1".into(),
        timestamp: 1_700_000_000,
    });
}

#[given(expr = "a WatchDispatchRequest with session_id {string}")]
fn given_dispatch_with_session(world: &mut BridgeWorld, session_id: String) {
    world.dispatch_request = Some(WatchDispatchRequest {
        session_id: Some(session_id),
        content: "Continue".into(),
        provider: None,
        nonce: "n2".into(),
        timestamp: 1_700_000_001,
    });
}

// ── When steps ───────────────────────────────────────────────────────────────

#[when("I serialise the response")]
fn when_serialise_response(world: &mut BridgeWorld) {
    let resp = world.dispatch_response.as_ref().unwrap();
    world.json = serde_json::to_string(resp).unwrap();
}

#[when(expr = "I insert a broadcast sender for session {string}")]
fn when_insert_sender(world: &mut BridgeWorld, session_id: String) {
    let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
    let mut map = world.streams.as_ref().unwrap().lock().unwrap();
    map.insert(session_id, tx);
}

#[when(expr = "I record nonce {string}")]
fn when_record_nonce(world: &mut BridgeWorld, nonce: String) {
    let ts = world.current_ts;
    let result = world.nonce_reg.as_ref().unwrap().check_and_record(&nonce, ts);
    world.record_error = result.err().map(|e| e.to_string());
    world.last_nonce = nonce;
}

#[when("I serialise it to JSON")]
fn when_serialise_sandbox(world: &mut BridgeWorld) {
    let req = world.sandbox_req.as_ref().unwrap();
    world.json = serde_json::to_string(req).unwrap();
}

#[when("I serialise and deserialise the request")]
fn when_serde_request(world: &mut BridgeWorld) {
    let req = world.dispatch_request.as_ref().unwrap();
    let json = serde_json::to_string(req).unwrap();
    world.dispatch_request = Some(serde_json::from_str(&json).unwrap());
}

#[when("I check the size of WatchBridgeState")]
fn when_check_size(world: &mut BridgeWorld) {
    world.size_of_state = std::mem::size_of::<WatchBridgeState>();
}

// ── Then steps ───────────────────────────────────────────────────────────────

#[then(expr = "the streaming_url should contain {string}")]
fn then_streaming_url_contains(world: &mut BridgeWorld, needle: String) {
    let resp = world.dispatch_response.as_ref().unwrap();
    assert!(resp.streaming_url.contains(&needle),
        "streaming_url '{}' does not contain '{}'", resp.streaming_url, needle);
}

#[then(expr = "the streaming_url should start with {string}")]
fn then_streaming_url_starts_with(world: &mut BridgeWorld, prefix: String) {
    let resp = world.dispatch_response.as_ref().unwrap();
    assert!(resp.streaming_url.starts_with(&prefix),
        "streaming_url '{}' does not start with '{}'", resp.streaming_url, prefix);
}

#[then("the map should be empty")]
fn then_map_empty(world: &mut BridgeWorld) {
    let map = world.streams.as_ref().unwrap().lock().unwrap();
    assert!(map.is_empty());
}

#[then(expr = "the map should contain key {string}")]
fn then_map_has_key(world: &mut BridgeWorld, key: String) {
    let map = world.streams.as_ref().unwrap().lock().unwrap();
    assert!(map.contains_key(&key), "map does not contain key '{}'", key);
}

#[then(expr = "the map size should be {int}")]
fn then_map_size(world: &mut BridgeWorld, expected: u32) {
    let map = world.streams.as_ref().unwrap().lock().unwrap();
    assert_eq!(map.len(), expected as usize);
}

#[then(expr = "recording the same nonce again should fail with {string}")]
fn then_replay_fails(world: &mut BridgeWorld, needle: String) {
    // Try recording the same nonce a second time — must fail
    let ts = world.current_ts;
    let nonce = world.last_nonce.clone();
    let result = world.nonce_reg.as_ref().unwrap().check_and_record(&nonce, ts);
    let err = result.err().expect("expected replay error");
    assert!(err.to_string().contains(&needle),
        "expected '{}' in: {}", needle, err);
}

#[then("both should succeed")]
fn then_both_succeed(world: &mut BridgeWorld) {
    assert!(world.record_error.is_none(), "expected no error: {:?}", world.record_error);
}

#[then(expr = "the JSON should contain {string}")]
fn then_json_contains(world: &mut BridgeWorld, needle: String) {
    assert!(world.json.contains(&needle),
        "JSON '{}' does not contain '{}'", world.json, needle);
}

#[then(expr = "deserialising should produce action {string}")]
fn then_deserialise_action(world: &mut BridgeWorld, expected: String) {
    let back: WatchSandboxControlRequest = serde_json::from_str(&world.json).unwrap();
    assert_eq!(back.action, expected);
}

#[then("session_id should be null")]
fn then_session_id_null(world: &mut BridgeWorld) {
    assert!(world.dispatch_request.as_ref().unwrap().session_id.is_none());
}

#[then(expr = "session_id should be {string}")]
fn then_session_id_value(world: &mut BridgeWorld, expected: String) {
    assert_eq!(world.dispatch_request.as_ref().unwrap().session_id.as_deref(),
        Some(expected.as_str()));
}

#[then("the size should be greater than zero")]
fn then_size_nonzero(world: &mut BridgeWorld) {
    assert!(world.size_of_state > 0);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(BridgeWorld::run(
        "tests/features/watch_bridge.feature",
    ));
}
