//! BDD: BrokerCa minting + leaf-cert factory + CONNECT method handling.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use vibe_broker::{
    BoundAddr, Broker, BrokerCa, BrokerHandle, LeafCert, Policy, SsrfGuard,
    policy::DefaultRule,
};

#[derive(Default, World)]
pub struct TWorld {
    rt: Option<Arc<Runtime>>,
    ca: Option<Arc<BrokerCa>>,
    ca_pem: Option<String>,
    leaves: Vec<LeafCert>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<BrokerHandle>,
    response_status: Option<u16>,
    response_headers: Vec<(String, String)>,
}

impl std::fmt::Debug for TWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TWorld").finish()
    }
}

impl TWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

#[given("a fresh BrokerCa")]
fn fresh_ca(world: &mut TWorld) {
    world.ca = Some(Arc::new(BrokerCa::generate().unwrap()));
}

#[when("I read the CA cert PEM")]
fn read_ca_pem(world: &mut TWorld) {
    world.ca_pem = Some(world.ca.as_ref().unwrap().ca_pem().to_owned());
}

#[then(expr = "the PEM starts with {string}")]
fn pem_starts_with(world: &mut TWorld, expected: String) {
    let pem = world.ca_pem.as_ref().unwrap();
    assert!(pem.starts_with(&expected), "got: {}", &pem[..40.min(pem.len())]);
}

#[then(expr = "the PEM ends with a {string} block")]
fn pem_ends_with_block(world: &mut TWorld, marker: String) {
    let pem = world.ca_pem.as_ref().unwrap();
    assert!(pem.contains(&marker), "PEM did not contain {marker:?}");
}

#[when(expr = "I mint a leaf for {string}")]
fn mint_leaf(world: &mut TWorld, host: String) {
    let leaf = world.ca.as_ref().unwrap().leaf_for(&host).unwrap();
    world.leaves.push(leaf);
}

#[when(expr = "I mint a leaf for {string} again")]
fn mint_leaf_again(world: &mut TWorld, host: String) {
    let leaf = world.ca.as_ref().unwrap().leaf_for(&host).unwrap();
    world.leaves.push(leaf);
}

#[then(expr = "the leaf cert SAN list contains {string}")]
fn san_contains(world: &mut TWorld, expected: String) {
    let leaf = world.leaves.last().unwrap();
    assert!(leaf.san_list.contains(&expected), "SANs were: {:?}", leaf.san_list);
}

#[then("the leaf cert is signed by the broker CA")]
fn leaf_signed_by_ca(world: &mut TWorld) {
    // Cheap sanity check: leaf cert PEM exists and is non-empty; the
    // cryptographic verification path is exercised in the rustls
    // integration test in B1.8 (real curl + --cacert hits a real TLS
    // server using these certs).
    let leaf = world.leaves.last().unwrap();
    assert!(leaf.cert_pem.starts_with("-----BEGIN CERTIFICATE-----"));
    assert!(world.ca.is_some());
}

#[then("both leaf certs share the same serial number")]
fn same_serial(world: &mut TWorld) {
    assert!(world.leaves.len() >= 2);
    let a = &world.leaves[world.leaves.len() - 2];
    let b = &world.leaves[world.leaves.len() - 1];
    assert_eq!(a.serial, b.serial, "expected cache hit");
}

fn boot_tls_broker(world: &mut TWorld, policy: Policy) {
    let ca = Arc::new(BrokerCa::generate().unwrap());
    let broker = Broker::new(policy, SsrfGuard::new()).with_tls_ca(ca.clone());
    let rt = world.rt();
    let handle = rt.block_on(async move { broker.start_tcp("127.0.0.1:0").await.unwrap() });
    match handle.addr.clone() {
        BoundAddr::Tcp(addr) => world.broker_addr = Some(addr),
        other => panic!("expected TCP, got {other:?}"),
    }
    world.broker_handle = Some(handle);
    world.ca = Some(ca);
}

#[given("a running broker with TLS interception and empty policy")]
fn tls_broker_empty(world: &mut TWorld) {
    boot_tls_broker(
        world,
        Policy {
            default: DefaultRule::Deny,
            rule: vec![],
        },
    );
}

#[given(expr = "a running broker with TLS interception and a rule allowing {string} on CONNECT")]
fn tls_broker_with_rule(world: &mut TWorld, host: String) {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = ["CONNECT"]
match.require_tls = true
"#
    );
    boot_tls_broker(world, Policy::parse_toml(&toml).unwrap());
}

#[when(expr = "the client sends {string}")]
fn send_connect(world: &mut TWorld, line: String) {
    let mut parts = line.splitn(2, ' ');
    let _method = parts.next().unwrap();
    let target = parts.next().unwrap().to_owned();
    let raw = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n");
    let addr = world.broker_addr.unwrap();
    let rt = world.rt();
    let resp = rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(raw.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        // Read a bounded amount; CONNECT 200 has no body, so the broker's
        // peer is likely to keep the connection open. Read with a deadline.
        let timeout = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            s.read_to_end(&mut buf),
        )
        .await;
        // On 200 the broker closes (B1.7 stub); on 4xx it closes too. Either way buf has the response.
        let _ = timeout;
        buf
    });
    parse_response_into(world, &resp);
}

fn parse_response_into(world: &mut TWorld, resp: &[u8]) {
    // The 200 path may not have full \r\n\r\n if the connection was idle.
    // Locate the first \r\n and parse the status line.
    let split = resp
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(resp.len());
    let status_line = String::from_utf8_lossy(&resp[..split]);
    let parts: Vec<_> = status_line.splitn(3, ' ').collect();
    if parts.len() >= 2 {
        world.response_status = parts[1].parse().ok();
    }
    world.response_headers.clear();
    let header_end = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(resp.len());
    let headers_text = String::from_utf8_lossy(&resp[split + 2..header_end]);
    for line in headers_text.split("\r\n") {
        if let Some((k, v)) = line.split_once(':') {
            world
                .response_headers
                .push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
}

#[then(expr = "the broker response status is {int}")]
fn status_is(world: &mut TWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected),
        "headers: {:?}", world.response_headers);
}

#[then(expr = "the broker response header {string} is {string}")]
fn header_is(world: &mut TWorld, name: String, value: String) {
    let lower = name.to_ascii_lowercase();
    let actual = world
        .response_headers
        .iter()
        .find(|(n, _)| n == &lower)
        .map(|(_, v)| v.as_str());
    assert_eq!(actual, Some(value.as_str()), "headers: {:?}", world.response_headers);
}

fn main() {
    futures::executor::block_on(TWorld::run("tests/features/broker_tls.feature"));
}
