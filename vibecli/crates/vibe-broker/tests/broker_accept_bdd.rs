//! End-to-end BDD: spin up a real broker on a localhost port and send
//! HTTP through it.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use vibe_broker::{Broker, BrokerHandle, Policy, SsrfGuard, policy::DefaultRule};

#[derive(Default, World)]
pub struct AWorld {
    rt: Option<Arc<Runtime>>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<BrokerHandle>,
    response_status: Option<u16>,
    response_headers: Vec<(String, String)>,
    raw_response: Vec<u8>,
}

impl std::fmt::Debug for AWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AWorld")
            .field("addr", &self.broker_addr)
            .finish()
    }
}

impl AWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }

    fn start_broker(&mut self, policy: Policy) {
        let rt = self.rt();
        let broker = Broker::new(policy, SsrfGuard::new());
        let handle = rt.block_on(async move { broker.start_tcp("127.0.0.1:0").await.unwrap() });
        match handle.addr.clone() {
            vibe_broker::BoundAddr::Tcp(addr) => self.broker_addr = Some(addr),
            other => panic!("expected TCP, got {other:?}"),
        }
        self.broker_handle = Some(handle);
    }
}

#[given("a running broker with empty policy")]
fn empty_policy(world: &mut AWorld) {
    world.start_broker(Policy {
        default: DefaultRule::Deny,
        rule: vec![],
    });
}

#[given(expr = "a running broker with a rule allowing {string} methods {string}")]
fn one_rule(world: &mut AWorld, host: String, methods: String) {
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
    let policy = Policy::parse_toml(&toml).expect("policy parses");
    world.start_broker(policy);
}

#[when(expr = "I send {string} through the broker")]
fn send_request(world: &mut AWorld, req: String) {
    let mut parts = req.splitn(2, ' ');
    let method = parts.next().unwrap().to_owned();
    let url = parts.next().unwrap().to_owned();
    let parsed = url::Url::parse(&url).expect("url parses");
    let host = parsed.host_str().unwrap().to_owned();
    let path_and_query = format!(
        "{}{}",
        parsed.path(),
        parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
    );

    let addr = world.broker_addr.expect("broker started");
    let rt = world.rt();
    let raw = format!(
        "{method} {path_and_query} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    );

    let resp = rt.block_on(async move {
        let mut stream = TcpStream::connect(addr).await.expect("connect");
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.expect("read");
        buf
    });
    parse_response_into_world(world, &resp);
}

#[when(expr = "I send raw bytes {string} to the broker")]
fn send_raw(world: &mut AWorld, raw: String) {
    let bytes = unescape(&raw).into_bytes();
    let addr = world.broker_addr.expect("broker started");
    let rt = world.rt();
    let resp = rt.block_on(async move {
        let mut stream = TcpStream::connect(addr).await.expect("connect");
        stream.write_all(&bytes).await.expect("write");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.expect("read");
        buf
    });
    world.raw_response = resp.clone();
    parse_response_into_world(world, &resp);
}

fn unescape(s: &str) -> String {
    s.replace("\\r", "\r").replace("\\n", "\n")
}

fn parse_response_into_world(world: &mut AWorld, resp: &[u8]) {
    let text = String::from_utf8_lossy(resp);
    let mut lines = text.split("\r\n");
    if let Some(status_line) = lines.next() {
        let parts: Vec<_> = status_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            if let Ok(code) = parts[1].parse::<u16>() {
                world.response_status = Some(code);
            }
        }
    }
    world.response_headers.clear();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            world
                .response_headers
                .push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
        }
    }
    world.raw_response = resp.to_vec();
}

#[then(expr = "the broker response status is {int}")]
fn status_is(world: &mut AWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected),
        "raw response was: {:?}", String::from_utf8_lossy(&world.raw_response));
}

#[then(expr = "the broker response header {string} is {string}")]
fn header_is(world: &mut AWorld, name: String, value: String) {
    let lower = name.to_ascii_lowercase();
    let actual = world
        .response_headers
        .iter()
        .find(|(n, _)| n == &lower)
        .map(|(_, v)| v.as_str());
    assert_eq!(actual, Some(value.as_str()), "headers: {:?}", world.response_headers);
}

#[then(expr = "the broker raw response starts with {string}")]
fn raw_starts_with(world: &mut AWorld, expected: String) {
    let text = String::from_utf8_lossy(&world.raw_response);
    assert!(
        text.starts_with(&expected),
        "got: {:?}",
        text.chars().take(40).collect::<String>()
    );
}

fn main() {
    futures::executor::block_on(AWorld::run(
        "tests/features/broker_accept.feature",
    ));
}
