//! BDD: AuditSummary primitive — drives synthetic events into a builder
//! / a JSONL file, then asserts the rolled-up shape recap will read.

use cucumber::{World, given, then, when};
use std::path::PathBuf;
use tempfile::TempDir;
use vibe_broker::{
    AuditEvent, AuditSummary, EgressOutcome, JsonlFileAuditSink,
    audit::baseline_egress_request,
};

#[derive(Default, World)]
pub struct SWorld {
    events: Vec<AuditEvent>,
    summary: Option<AuditSummary>,
    sink_dir: Option<TempDir>,
    sink_path: Option<PathBuf>,
    sink: Option<JsonlFileAuditSink>,
}

impl std::fmt::Debug for SWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SWorld")
            .field("events", &self.events.len())
            .field("has_summary", &self.summary.is_some())
            .finish()
    }
}

fn outcome_from(s: &str) -> EgressOutcome {
    match s {
        "Ok" | "ok" => EgressOutcome::Ok,
        "PolicyDenied" | "policy_denied" => EgressOutcome::PolicyDenied,
        "SsrfBlocked" | "ssrf_blocked" => EgressOutcome::SsrfBlocked,
        "BodyOversized" | "body_oversized" => EgressOutcome::BodyOversized,
        "TlsError" | "tls_error" => EgressOutcome::TlsError,
        "Timeout" | "timeout" => EgressOutcome::Timeout,
        "UpstreamError" | "upstream_error" => EgressOutcome::UpstreamError,
        other => panic!("unknown outcome: {other}"),
    }
}

#[given("a fresh AuditSummary builder")]
fn fresh_builder(world: &mut SWorld) {
    world.events.clear();
    world.summary = None;
}

#[given("a fresh JSONL audit sink at a temp path")]
fn fresh_sink(world: &mut SWorld) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");
    world.sink = Some(JsonlFileAuditSink::open(&path).unwrap());
    world.sink_path = Some(path);
    world.sink_dir = Some(dir);
}

#[when(expr = "I summarize {int} events")]
fn summarize_n(world: &mut SWorld, _n: usize) {
    world.summary = Some(AuditSummary::from_events(&world.events));
}

#[when(expr = "I record an Ok event for host {string} with bytes_request {int} and bytes_response {int}")]
fn record_ok_with_bytes(world: &mut SWorld, host: String, req: u64, resp: u64) {
    let mut e = baseline_egress_request("native", "skill:test", "GET", &host, "/x");
    e.outcome = EgressOutcome::Ok;
    e.bytes_request = req;
    e.bytes_response = resp;
    world.events.push(e);
}

#[when(expr = "I record an Ok event with inject {string} for host {string}")]
fn record_ok_with_inject(world: &mut SWorld, inject: String, host: String) {
    let mut e = baseline_egress_request("native", "skill:test", "GET", &host, "/x");
    e.outcome = EgressOutcome::Ok;
    e.inject = inject;
    world.events.push(e);
}

#[when(expr = "I record a {word} event for host {string}")]
#[when(expr = "I record an {word} event for host {string}")]
fn record_named(world: &mut SWorld, outcome: String, host: String) {
    let mut e = baseline_egress_request("native", "skill:test", "GET", &host, "/x");
    e.outcome = outcome_from(&outcome);
    world.events.push(e);
}

#[when("I summarize the recorded events")]
fn summarize_recorded(world: &mut SWorld) {
    world.summary = Some(AuditSummary::from_events(&world.events));
}

#[when(expr = "I record an Ok event for host {string} with bytes_request {int} and bytes_response {int} to the sink")]
fn record_to_sink(world: &mut SWorld, host: String, req: u64, resp: u64) {
    let mut e = baseline_egress_request("native", "skill:test", "GET", &host, "/x");
    e.outcome = EgressOutcome::Ok;
    e.bytes_request = req;
    e.bytes_response = resp;
    use vibe_broker::AuditSink;
    world.sink.as_ref().unwrap().record(e);
}

#[when(expr = "I record a {word} event for host {string} to the sink")]
#[when(expr = "I record an {word} event for host {string} to the sink")]
fn record_named_to_sink(world: &mut SWorld, outcome: String, host: String) {
    let mut e = baseline_egress_request("native", "skill:test", "GET", &host, "/x");
    e.outcome = outcome_from(&outcome);
    use vibe_broker::AuditSink;
    world.sink.as_ref().unwrap().record(e);
}

#[when("I summarize the JSONL file")]
fn summarize_file(world: &mut SWorld) {
    // Drop the writer first so its BufWriter flushes.
    drop(world.sink.take());
    let path = world.sink_path.clone().unwrap();
    world.summary = Some(AuditSummary::from_jsonl_file(&path).unwrap());
}

fn s(world: &SWorld) -> &AuditSummary {
    world.summary.as_ref().expect("summary built")
}

#[then(expr = "the summary total_requests is {int}")]
fn total(world: &mut SWorld, expected: u64) {
    assert_eq!(s(world).total_requests, expected);
}

#[then(expr = "the summary by_outcome has {int} entries")]
fn by_outcome_count(world: &mut SWorld, expected: usize) {
    assert_eq!(s(world).by_outcome.len(), expected);
}

#[then(expr = "the summary by_host has {int} entries")]
fn by_host_count(world: &mut SWorld, expected: usize) {
    assert_eq!(s(world).by_host.len(), expected);
}

#[then(expr = "the summary by_outcome {word} count is {int}")]
fn by_outcome_named(world: &mut SWorld, name: String, expected: u64) {
    let key = match name.as_str() {
        "ok" | "Ok" => "ok",
        "policy_denied" => "policy_denied",
        "ssrf_blocked" => "ssrf_blocked",
        "body_oversized" => "body_oversized",
        "tls_error" => "tls_error",
        "timeout" => "timeout",
        "upstream_error" => "upstream_error",
        other => panic!("unknown outcome key: {other}"),
    };
    assert_eq!(s(world).by_outcome.get(key).copied().unwrap_or(0), expected,
        "outcome {key} expected {expected} got {:?}", s(world).by_outcome.get(key));
}

#[then(expr = "the summary by_host {string} count is {int}")]
fn by_host_named(world: &mut SWorld, host: String, expected: u64) {
    assert_eq!(s(world).by_host.get(&host).copied().unwrap_or(0), expected);
}

#[then(expr = "the summary by_inject {string} count is {int}")]
fn by_inject_named(world: &mut SWorld, inject: String, expected: u64) {
    assert_eq!(s(world).by_inject.get(&inject).copied().unwrap_or(0), expected);
}

#[then(expr = "the summary bytes_request_total is {int}")]
fn bytes_req(world: &mut SWorld, expected: u64) {
    assert_eq!(s(world).bytes_request_total, expected);
}

#[then(expr = "the summary bytes_response_total is {int}")]
fn bytes_resp(world: &mut SWorld, expected: u64) {
    assert_eq!(s(world).bytes_response_total, expected);
}

fn main() {
    futures::executor::block_on(SWorld::run(
        "tests/features/broker_audit_summary.feature",
    ));
}
