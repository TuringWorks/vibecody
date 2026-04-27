//! BDD: audit emission on the plain-HTTP path. Sends raw HTTP through
//! the broker, then asserts what landed in the MemoryAuditSink.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use vibe_broker::{
    AuditEvent, AwsCredentials, BoundAddr, Broker, BrokerHandle, EgressOutcome, ImdsHandle,
    ImdsServer, InMemorySecretStore, MemoryAuditSink, Policy, SecretStore, SsrfGuard,
    policy::{DefaultRule, SecretRef},
};

#[derive(Default, World)]
pub struct AWorld {
    rt: Option<Arc<Runtime>>,
    audit: Option<Arc<MemoryAuditSink>>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<BrokerHandle>,
    imds_addr: Option<std::net::SocketAddr>,
    imds_handle: Option<ImdsHandle>,
}

impl std::fmt::Debug for AWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AWorld").finish()
    }
}

impl AWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

fn install_broker(world: &mut AWorld, broker: Broker) {
    let rt = world.rt();
    let handle = rt.block_on(async move { broker.start_tcp("127.0.0.1:0").await.unwrap() });
    if let BoundAddr::Tcp(addr) = handle.addr.clone() {
        world.broker_addr = Some(addr);
    }
    world.broker_handle = Some(handle);
}

#[given("a broker with an in-memory audit sink and empty policy")]
fn empty_policy(world: &mut AWorld) {
    let sink = Arc::new(MemoryAuditSink::new());
    world.audit = Some(sink.clone());
    let broker = Broker::new(
        Policy {
            default: DefaultRule::Deny,
            rule: vec![],
        },
        SsrfGuard::new(),
    )
    .with_audit_sink(sink);
    install_broker(world, broker);
}

#[given(expr = "a broker with an in-memory audit sink and a rule allowing {string} methods {string}")]
fn one_rule(world: &mut AWorld, host: String, methods: String) {
    let sink = Arc::new(MemoryAuditSink::new());
    world.audit = Some(sink.clone());
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = [{}]
match.require_tls = false
"#,
        methods
            .split(',')
            .map(|m| format!("\"{}\"", m.trim()))
            .collect::<Vec<_>>()
            .join(", "),
    );
    let policy = Policy::parse_toml(&toml).unwrap();
    let broker = Broker::new(policy, SsrfGuard::new()).with_audit_sink(sink);
    install_broker(world, broker);
}

#[when(expr = "I send {string} through the broker")]
fn send_request(world: &mut AWorld, req: String) {
    let mut parts = req.splitn(2, ' ');
    let method = parts.next().unwrap();
    let url = parts.next().unwrap();
    let parsed = url::Url::parse(url).unwrap();
    let host = parsed.host_str().unwrap().to_owned();
    let path_q = format!(
        "{}{}",
        parsed.path(),
        parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
    );
    let raw = format!(
        "{method} {path_q} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    );
    drive(world, raw);
}

#[when(expr = "I send raw bytes {string} to the broker")]
fn send_raw(world: &mut AWorld, raw: String) {
    let bytes = raw.replace("\\r", "\r").replace("\\n", "\n");
    drive(world, bytes);
}

fn drive(world: &mut AWorld, raw: String) {
    let addr = world.broker_addr.unwrap();
    let rt = world.rt();
    rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(raw.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
    });
}

#[then(expr = "the audit sink recorded {int} event")]
#[then(expr = "the audit sink recorded {int} events")]
fn count(world: &mut AWorld, expected: usize) {
    let actual = world.audit.as_ref().unwrap().len();
    assert_eq!(actual, expected,
        "events: {:?}", world.audit.as_ref().unwrap().events());
}

fn event_at(world: &AWorld, idx: usize) -> AuditEvent {
    world.audit.as_ref().unwrap().events()[idx].clone()
}

#[then(expr = "the audit event {int} outcome is {string}")]
fn outcome_is(world: &mut AWorld, idx: usize, expected: String) {
    let e = event_at(world, idx);
    let want = match expected.as_str() {
        "ok" => EgressOutcome::Ok,
        "policy_denied" => EgressOutcome::PolicyDenied,
        "ssrf_blocked" => EgressOutcome::SsrfBlocked,
        "body_oversized" => EgressOutcome::BodyOversized,
        "tls_error" => EgressOutcome::TlsError,
        "timeout" => EgressOutcome::Timeout,
        "upstream_error" => EgressOutcome::UpstreamError,
        other => panic!("unknown outcome: {other}"),
    };
    assert_eq!(e.outcome, want);
}

#[then(expr = "the audit event {int} host is {string}")]
fn host_is(world: &mut AWorld, idx: usize, expected: String) {
    assert_eq!(event_at(world, idx).host, expected);
}

#[then(expr = "the audit event {int} method is {string}")]
fn method_is(world: &mut AWorld, idx: usize, expected: String) {
    assert_eq!(event_at(world, idx).method, expected);
}

#[then(expr = "the audit event {int} matched_rule_index is {int}")]
fn matched_rule_idx(world: &mut AWorld, idx: usize, expected: usize) {
    assert_eq!(event_at(world, idx).matched_rule_index, Some(expected));
}

#[given(expr = "an IMDS faker with audit sink, role {string}, creds at {string}")]
fn boot_imds_with_audit(world: &mut AWorld, role: String, key: String) {
    let sink = Arc::new(MemoryAuditSink::new());
    world.audit = Some(sink.clone());
    let secrets = Arc::new(InMemorySecretStore::new());
    secrets.set_aws(
        key.clone(),
        AwsCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
            region: "us-east-1".into(),
            service: "s3".into(),
        },
    );
    let store = secrets as Arc<dyn SecretStore>;
    let server = ImdsServer::new(role, SecretRef(key), store).with_audit_sink(sink);
    let rt = world.rt();
    let handle = rt.block_on(async move { server.start("127.0.0.1:0").await.unwrap() });
    world.imds_addr = Some(handle.addr);
    world.imds_handle = Some(handle);
}

#[when(expr = "I PUT {string} against the IMDS faker")]
fn imds_put(world: &mut AWorld, path: String) {
    let addr = world.imds_addr.unwrap();
    let req = format!(
        "PUT {path} HTTP/1.1\r\nHost: 169.254.169.254\r\nx-aws-ec2-metadata-token-ttl-seconds: 21600\r\nConnection: close\r\n\r\n"
    );
    let rt = world.rt();
    rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
    });
}

#[when(expr = "I GET {string} against the IMDS faker without a token")]
fn imds_get_no_token(world: &mut AWorld, path: String) {
    let addr = world.imds_addr.unwrap();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: 169.254.169.254\r\nConnection: close\r\n\r\n"
    );
    let rt = world.rt();
    rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
    });
}

fn main() {
    futures::executor::block_on(AWorld::run("tests/features/broker_audit.feature"));
}
