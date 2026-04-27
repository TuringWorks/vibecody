//! BDD: IMDSv2 faker — drives the AWS metadata-service dance against
//! a real server bound on a loopback port.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use vibe_broker::{
    AwsCredentials, ImdsHandle, ImdsServer, InMemorySecretStore, SecretStore,
    policy::SecretRef,
};

#[derive(Default, World)]
pub struct IWorld {
    rt: Option<Arc<Runtime>>,
    server_addr: Option<std::net::SocketAddr>,
    server_handle: Option<ImdsHandle>,
    role_name: String,
    secret_key: String,
    imds_token: Option<String>,
    response_status: Option<u16>,
    response_body: Vec<u8>,
}

impl std::fmt::Debug for IWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IWorld")
            .field("addr", &self.server_addr)
            .finish()
    }
}

impl IWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

fn build_secrets(key: &str) -> Arc<dyn SecretStore> {
    let s = Arc::new(InMemorySecretStore::new());
    s.set_aws(
        key,
        AwsCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            session_token: Some("FwoGZXIvYXdzELP".into()),
            region: "us-east-1".into(),
            service: "s3".into(),
        },
    );
    s
}

#[given("an IMDS faker bound to a loopback address")]
fn boot_imds_default(world: &mut IWorld) {
    let role = "vibe-broker-role".to_string();
    let key = "@workspace.aws_default".to_string();
    let secrets = build_secrets(&key);
    let server = ImdsServer::new(role.clone(), SecretRef(key.clone()), secrets);
    let rt = world.rt();
    let handle = rt.block_on(async move { server.start("127.0.0.1:0").await.unwrap() });
    world.server_addr = Some(handle.addr);
    world.server_handle = Some(handle);
    world.role_name = role;
    world.secret_key = key;
}

#[given(expr = "an IMDS faker bound to a loopback address with role {string} and creds at {string}")]
fn boot_imds_named(world: &mut IWorld, role: String, key: String) {
    let secrets = build_secrets(&key);
    let server = ImdsServer::new(role.clone(), SecretRef(key.clone()), secrets);
    let rt = world.rt();
    let handle = rt.block_on(async move { server.start("127.0.0.1:0").await.unwrap() });
    world.server_addr = Some(handle.addr);
    world.server_handle = Some(handle);
    world.role_name = role;
    world.secret_key = key;
}

#[given("I have an IMDS token from the faker")]
fn fetch_token(world: &mut IWorld) {
    let addr = world.server_addr.unwrap();
    let rt = world.rt();
    let body = rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        let req = "PUT /latest/api/token HTTP/1.1\r\n\
                   Host: 169.254.169.254\r\n\
                   X-aws-ec2-metadata-token-ttl-seconds: 21600\r\n\
                   Connection: close\r\n\
                   \r\n";
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
        buf
    });
    let split = body.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let token = String::from_utf8_lossy(&body[split + 4..]).trim().to_owned();
    world.imds_token = Some(token);
}

fn raw_request(world: &mut IWorld, method: &str, path: &str, token: Option<&str>) {
    let addr = world.server_addr.unwrap();
    let token_header = match token {
        Some(t) => format!("X-aws-ec2-metadata-token: {t}\r\n"),
        None => String::new(),
    };
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 169.254.169.254\r\n{token_header}Connection: close\r\n\r\n"
    );
    let rt = world.rt();
    let buf = rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
        buf
    });
    parse_response(world, &buf);
}

fn parse_response(world: &mut IWorld, buf: &[u8]) {
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..split]);
    if let Some(line) = head.lines().next() {
        let parts: Vec<_> = line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            world.response_status = parts[1].parse().ok();
        }
    }
    world.response_body = if split + 4 <= buf.len() {
        buf[split + 4..].to_vec()
    } else {
        Vec::new()
    };
}

#[when(expr = "I PUT {string} with header {string}")]
fn put_with_header(world: &mut IWorld, path: String, header: String) {
    let addr = world.server_addr.unwrap();
    let req = format!(
        "PUT {path} HTTP/1.1\r\nHost: 169.254.169.254\r\n{header}\r\nConnection: close\r\n\r\n"
    );
    let rt = world.rt();
    let buf = rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
        buf
    });
    parse_response(world, &buf);
}

#[when(expr = "I GET {string} with the IMDS token")]
fn get_with_token(world: &mut IWorld, path: String) {
    let token = world.imds_token.clone();
    raw_request(world, "GET", &path, token.as_deref());
}

#[when(expr = "I GET {string} without an IMDS token")]
fn get_without_token(world: &mut IWorld, path: String) {
    raw_request(world, "GET", &path, None);
}

#[then(expr = "the IMDS response status is {int}")]
fn status_is(world: &mut IWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected),
        "body was: {:?}", String::from_utf8_lossy(&world.response_body));
}

#[then("the IMDS response body is non-empty")]
fn body_non_empty(world: &mut IWorld) {
    assert!(!world.response_body.is_empty());
}

#[then(expr = "the IMDS response body equals {string}")]
fn body_equals(world: &mut IWorld, expected: String) {
    assert_eq!(String::from_utf8_lossy(&world.response_body).trim(), expected);
}

#[then(expr = "the IMDS response body contains {string}")]
fn body_contains(world: &mut IWorld, needle: String) {
    let text = String::from_utf8_lossy(&world.response_body);
    assert!(text.contains(&needle), "body was: {text}");
}

fn main() {
    futures::executor::block_on(IWorld::run("tests/features/broker_imds.feature"));
}
