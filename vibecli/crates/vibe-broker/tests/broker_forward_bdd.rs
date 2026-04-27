//! BDD: broker forwards an allowed request to a real (test-local) upstream.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use vibe_broker::{Broker, Policy, SsrfGuard};

#[derive(Default, World)]
pub struct FWorld {
    rt: Option<Arc<Runtime>>,
    upstream_addr: Option<std::net::SocketAddr>,
    upstream_handle: Option<JoinHandle<()>>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<JoinHandle<()>>,
    response_status: Option<u16>,
    response_headers: Vec<(String, String)>,
    response_body: Vec<u8>,
}

impl std::fmt::Debug for FWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FWorld").finish()
    }
}

impl FWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

async fn start_stub_upstream(
    response: Option<(u16, &'static str)>,
    hang: bool,
) -> (std::net::SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => break,
            };
            let response = response;
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                if hang {
                    // hold the connection open without responding
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    return;
                }
                if let Some((code, body)) = response {
                    let resp = format!(
                        "HTTP/1.1 {code} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                }
            });
        }
    });
    (addr, handle)
}

#[given(expr = "a stub upstream that replies with {string} body {string}")]
fn stub_replies(world: &mut FWorld, status_phrase: String, body: String) {
    let rt = world.rt();
    let body_static: &'static str = Box::leak(body.into_boxed_str());
    let code: u16 = status_phrase
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let (addr, handle) = rt.block_on(start_stub_upstream(Some((code, body_static)), false));
    world.upstream_addr = Some(addr);
    world.upstream_handle = Some(handle);
}

#[given("a stub upstream that hangs forever")]
fn stub_hangs(world: &mut FWorld) {
    let rt = world.rt();
    let (addr, handle) = rt.block_on(start_stub_upstream(None, true));
    world.upstream_addr = Some(addr);
    world.upstream_handle = Some(handle);
}

fn build_policy_for(host: &str) -> Policy {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = ["GET"]
match.require_tls = false
"#
    );
    Policy::parse_toml(&toml).unwrap()
}

fn ssrf_allowing_stub(host: &str) -> SsrfGuard {
    SsrfGuard::new().with_allow_host(host)
}

#[given("a running broker with upstream forwarding and a rule for the stub host")]
fn broker_forwarding(world: &mut FWorld) {
    let upstream = world.upstream_addr.unwrap();
    let host = upstream.ip().to_string();
    let policy = build_policy_for(&host);
    let broker = Broker::new(policy, ssrf_allowing_stub(&host)).with_upstream();
    let rt = world.rt();
    let (addr, handle) = rt.block_on(async move {
        broker.start("127.0.0.1:0").await.unwrap()
    });
    world.broker_addr = Some(addr);
    world.broker_handle = Some(handle);
}

#[given(expr = "a running broker with upstream forwarding timeout {int} ms and a rule for the stub host")]
fn broker_forwarding_timeout(world: &mut FWorld, ms: u64) {
    let upstream = world.upstream_addr.unwrap();
    let host = upstream.ip().to_string();
    let policy = build_policy_for(&host);
    let broker = Broker::new(policy, ssrf_allowing_stub(&host))
        .with_upstream()
        .with_upstream_timeout(Duration::from_millis(ms));
    let rt = world.rt();
    let (addr, handle) = rt.block_on(async move {
        broker.start("127.0.0.1:0").await.unwrap()
    });
    world.broker_addr = Some(addr);
    world.broker_handle = Some(handle);
}

#[when(expr = "I send {string} through the broker to the stub")]
fn send(world: &mut FWorld, req_line: String) {
    let mut parts = req_line.splitn(2, ' ');
    let method = parts.next().unwrap();
    let path = parts.next().unwrap();
    let upstream = world.upstream_addr.unwrap();
    let host_hdr = format!("{}:{}", upstream.ip(), upstream.port());
    let raw = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host_hdr}\r\nConnection: close\r\n\r\n"
    );
    let broker_addr = world.broker_addr.unwrap();
    let rt = world.rt();
    let resp = rt.block_on(async move {
        let mut stream = TcpStream::connect(broker_addr).await.unwrap();
        stream.write_all(raw.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        buf
    });
    parse_into(world, &resp);
}

fn parse_into(world: &mut FWorld, resp: &[u8]) {
    let split = resp.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(resp.len());
    let head_text = String::from_utf8_lossy(&resp[..split]);
    let body = if split + 4 <= resp.len() {
        resp[split + 4..].to_vec()
    } else {
        Vec::new()
    };
    let mut lines = head_text.split("\r\n");
    if let Some(status_line) = lines.next() {
        let parts: Vec<_> = status_line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            world.response_status = parts[1].parse().ok();
        }
    }
    world.response_headers.clear();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            world
                .response_headers
                .push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
    world.response_body = body;
}

#[then(expr = "the broker response status is {int}")]
fn status_is(world: &mut FWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected),
        "headers: {:?}", world.response_headers);
}

#[then(expr = "the broker response header {string} is {string}")]
fn header_is(world: &mut FWorld, name: String, value: String) {
    let lower = name.to_ascii_lowercase();
    let actual = world
        .response_headers
        .iter()
        .find(|(n, _)| n == &lower)
        .map(|(_, v)| v.as_str());
    assert_eq!(actual, Some(value.as_str()), "headers: {:?}", world.response_headers);
}

#[then(expr = "the broker response body equals {string}")]
fn body_equals(world: &mut FWorld, expected: String) {
    assert_eq!(String::from_utf8_lossy(&world.response_body), expected);
}

fn main() {
    futures::executor::block_on(FWorld::run(
        "tests/features/broker_forward.feature",
    ));
}
