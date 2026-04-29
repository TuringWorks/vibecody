//! BDD: BrokerDaemon end-to-end. Writes config + policy files, calls
//! the entry point, asserts the bound listener responds and the audit
//! file gets populated.

use cucumber::{World, given, then, when};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use vibe_broker::{
    BoundAddr, BrokerConfig, BrokerDaemon, DaemonHandle, SecretStore, policy::SecretRef,
};

#[derive(Default, World)]
pub struct DWorld {
    rt: Option<Arc<Runtime>>,
    dir: Option<TempDir>,
    config_path: Option<PathBuf>,
    daemon: Option<DaemonHandle>,
    broker_addr: Option<std::net::SocketAddr>,
    imds_addr: Option<std::net::SocketAddr>,
    response_status: Option<u16>,
    audit_path: Option<PathBuf>,
    azure_token: Option<String>,
    stub_addr: Option<std::net::SocketAddr>,
    stub_handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for DWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DWorld")
            .field("broker_addr", &self.broker_addr)
            .finish()
    }
}

impl DWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
    fn dir_path(&self) -> &std::path::Path {
        self.dir.as_ref().unwrap().path()
    }
}

#[given("a temp dir")]
fn temp_dir(world: &mut DWorld) {
    world.dir = Some(tempfile::tempdir().unwrap());
}

#[given(expr = "a policy file in the temp dir with one rule for {string}")]
fn write_policy(world: &mut DWorld, host: String) {
    let path = world.dir_path().join("egress.toml");
    let body = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = ["GET"]
match.require_tls = false
"#
    );
    std::fs::write(&path, body).unwrap();
}

#[given("a broker config in the temp dir with TCP listener and the policy + audit JSONL")]
fn write_full_config(world: &mut DWorld) {
    let policy_path = world.dir_path().join("egress.toml");
    let audit_path = world.dir_path().join("audit.jsonl");
    world.audit_path = Some(audit_path.clone());
    let config_path = world.dir_path().join("broker.toml");
    let body = format!(
        r#"
[broker]
listen_tcp = "127.0.0.1:0"
policy_id = "skill:test"

[broker.audit]
jsonl_path = "{}"

[policy]
path = "{}"
"#,
        audit_path.display(),
        policy_path.display()
    );
    std::fs::write(&config_path, body).unwrap();
    world.config_path = Some(config_path);
}

#[given(expr = "a broker config in the temp dir with TCP listener, no policy, IMDS section bound to {word}")]
fn write_imds_config(world: &mut DWorld, addr: String) {
    let config_path = world.dir_path().join("broker.toml");
    // Always use a free TCP port (ignore the cosmetic `127.0.0.1:0`
    // marker in the feature text).
    let _ = addr;
    let body = r#"
[broker]
listen_tcp = "127.0.0.1:0"

[broker.imds]
role_name = "vibe-broker-role"
secret_ref = "@workspace.aws_default"
listen_tcp = "127.0.0.1:0"
"#;
    std::fs::write(&config_path, body).unwrap();
    world.config_path = Some(config_path);
}

#[given(expr = "a stub Azure OAuth endpoint returning access_token {string} with expires_in {int}")]
fn stub_endpoint(world: &mut DWorld, token: String, expires_in: u64) {
    let body = format!(
        r#"{{"access_token":"{token}","token_type":"Bearer","expires_in":{expires_in}}}"#
    );
    let rt = world.rt();
    let (addr, handle) = rt.block_on(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body_clone = body.clone();
        let h = tokio::spawn(async move {
            loop {
                let (mut s, _) = match listener.accept().await {
                    Ok(p) => p,
                    Err(_) => break,
                };
                let body = body_clone.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let mut total = 0;
                    loop {
                        let n = match s.read(&mut buf[total..]).await {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => return,
                        };
                        total += n;
                        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                        if total == buf.len() {
                            break;
                        }
                    }
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = s.write_all(resp.as_bytes()).await;
                });
            }
        });
        (addr, h)
    });
    world.stub_addr = Some(addr);
    world.stub_handle = Some(handle);
}

#[given(expr = "a broker config in the temp dir with TCP listener, refresher {int}ms, azure profile pointed at the stub")]
fn write_refresher_config(world: &mut DWorld, interval_ms: u64) {
    let stub = world.stub_addr.unwrap();
    let endpoint = format!("http://{}", stub);
    // Refresher::interval_secs is whole seconds in the config; use 1
    // and override mid-test if needed. For the BDD we want quick
    // ticks, so we set interval_secs to 0 (which clamps to 0 sec
    // sleep — effectively immediate re-mint on every loop).
    let secs = (interval_ms / 1000).max(1);
    let _ = secs;
    let config_path = world.dir_path().join("broker.toml");
    let body = format!(
        r#"
[broker]
listen_tcp = "127.0.0.1:0"

[refresher]
interval_secs = 1

[[azure]]
secret_ref = "@workspace.azure_default"
tenant = "tenant42"
client_id = "abc"
client_secret = "shh"
scope = "default"
endpoint = "{endpoint}"
"#
    );
    std::fs::write(&config_path, body).unwrap();
    world.config_path = Some(config_path);
}

#[when("I start the daemon from the config")]
fn start_daemon(world: &mut DWorld) {
    let path = world.config_path.clone().unwrap();
    let rt = world.rt();
    let h = rt.block_on(async move {
        let cfg = BrokerConfig::from_path(&path).unwrap();
        BrokerDaemon::start(cfg).await.unwrap()
    });
    if let BoundAddr::Tcp(addr) = h.broker_addr() {
        world.broker_addr = Some(*addr);
    }
    world.imds_addr = h.imds_addr().copied();
    world.daemon = Some(h);
}

#[then("the daemon listener address is a real bound port")]
fn listener_bound(world: &mut DWorld) {
    let addr = world.broker_addr.expect("broker bound");
    assert!(addr.port() != 0);
}

#[then("the daemon IMDS address is a real bound port")]
fn imds_bound(world: &mut DWorld) {
    let addr = world.imds_addr.expect("imds bound");
    assert!(addr.port() != 0);
}

#[when(expr = "I send {string} through the daemon")]
fn send_request(world: &mut DWorld, req: String) {
    let mut parts = req.splitn(2, ' ');
    let method = parts.next().unwrap();
    let url = parts.next().unwrap();
    let parsed = url::Url::parse(url).unwrap();
    let host = parsed.host_str().unwrap().to_owned();
    let path = format!(
        "{}{}",
        parsed.path(),
        parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
    );
    let raw = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    let addr = world.broker_addr.unwrap();
    let rt = world.rt();
    let resp = rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(raw.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf).await;
        buf
    });
    let split = resp.windows(2).position(|w| w == b"\r\n").unwrap_or(resp.len());
    let line = String::from_utf8_lossy(&resp[..split]);
    if let Some(p) = line.split_whitespace().nth(1) {
        world.response_status = p.parse().ok();
    }
}

#[then(expr = "the daemon response status is {int}")]
fn status_is(world: &mut DWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected));
}

#[when("I shut the daemon down")]
fn shutdown_daemon(world: &mut DWorld) {
    let h = world.daemon.take().unwrap();
    h.shutdown();
    // Audit JSONL flushes are best-effort; let any pending writes settle.
    let rt = world.rt();
    rt.block_on(async move { tokio::time::sleep(Duration::from_millis(50)).await });
}

#[then(expr = "the audit JSONL file has at least {int} line")]
#[then(expr = "the audit JSONL file has at least {int} lines")]
fn audit_lines(world: &mut DWorld, min: usize) {
    let path = world.audit_path.clone().unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    let count = contents.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(count >= min, "audit file had {count} lines: {contents}");
}

#[when(expr = "I wait up to {int} seconds for the SecretStore to have an Azure token at {string}")]
fn wait_for_token(world: &mut DWorld, max_secs: u64, key: String) {
    let secrets = world.daemon.as_ref().unwrap().secrets.clone();
    let rt = world.rt();
    let token = rt.block_on(async move {
        for _ in 0..(max_secs * 20) {
            if let Some(t) = secrets.resolve_azure(&SecretRef(key.clone())) {
                return Some(t.token);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    });
    world.azure_token = token;
}

#[then(expr = "the SecretStore Azure token equals {string}")]
fn token_eq(world: &mut DWorld, expected: String) {
    assert_eq!(world.azure_token.as_deref(), Some(expected.as_str()));
}

fn main() {
    futures::executor::block_on(DWorld::run(
        "tests/features/broker_daemon.feature",
    ));
}
