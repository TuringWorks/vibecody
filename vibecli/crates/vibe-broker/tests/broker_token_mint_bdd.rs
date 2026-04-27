//! BDD: token-mint flows. Stand up a tiny stub OAuth endpoint inside
//! the test, point the minter at it, assert minted-token shape and
//! caching behaviour.

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use vibe_broker::{
    AzureClientCredentialsMinter, CachedMinter, MintedToken, TokenMinter,
};

#[derive(Default, World)]
pub struct TWorld {
    rt: Option<Arc<Runtime>>,
    stub_addr: Option<std::net::SocketAddr>,
    stub_handle: Option<JoinHandle<()>>,
    azure_minter: Option<AzureClientCredentialsMinter>,
    cached_minter: Option<Arc<CachedMinter<AzureClientCredentialsMinter>>>,
    minted: Option<MintedToken>,
}

impl std::fmt::Debug for TWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TWorld")
            .field("stub_addr", &self.stub_addr)
            .finish()
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

/// Spawn a tiny TCP-level HTTP server that returns a fixed
/// `{"access_token":..., "token_type":"Bearer", "expires_in":...}`
/// JSON for any POST. We can't use rustls + a self-signed cert here
/// because reqwest's default trust pool wouldn't accept it; instead the
/// minter is pointed at a plain http:// stub via `with_endpoint`.
async fn start_stub_http(
    body: String,
) -> (std::net::SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => break,
            };
            let body_clone = body.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let mut total = 0;
                // Read until end of request headers.
                loop {
                    let n = match stream.read(&mut buf[total..]).await {
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
                // Parse Content-Length and read body.
                let head = String::from_utf8_lossy(&buf[..total]);
                let cl = head
                    .lines()
                    .find_map(|l| {
                        let lower = l.to_ascii_lowercase();
                        lower
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                let head_end = head.find("\r\n\r\n").map(|i| i + 4).unwrap_or(total);
                let already_read = total.saturating_sub(head_end);
                let mut remaining = cl.saturating_sub(already_read);
                while remaining > 0 {
                    let want = remaining.min(buf.len());
                    let n = match stream.read(&mut buf[..want]).await {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    remaining -= n;
                }
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_clone}",
                    body_clone.len()
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });
    (addr, handle)
}

#[given(expr = "a stub Azure OAuth endpoint at \\/tenant42\\/oauth2\\/v2.0\\/token returning access_token {string} with expires_in {int}")]
fn stub_endpoint(world: &mut TWorld, token: String, expires_in: u64) {
    let body = format!(
        r#"{{"access_token":"{token}","token_type":"Bearer","expires_in":{expires_in}}}"#
    );
    let rt = world.rt();
    let (addr, handle) = rt.block_on(start_stub_http(body));
    world.stub_addr = Some(addr);
    world.stub_handle = Some(handle);
}

#[given(expr = "an AzureClientCredentialsMinter pointing at the stub with tenant {string} client_id {string} client_secret {string} scope {string}")]
fn make_azure_minter(
    world: &mut TWorld,
    tenant: String,
    client_id: String,
    client_secret: String,
    scope: String,
) {
    let addr = world.stub_addr.unwrap();
    let endpoint = format!("http://{}", addr);
    let m = AzureClientCredentialsMinter::new(tenant, client_id, client_secret, scope)
        .with_endpoint(endpoint);
    world.azure_minter = Some(m);
}

#[given(expr = "a CachedMinter wrapping it with a {int}-second refresh buffer")]
fn wrap_cached(world: &mut TWorld, buffer_secs: u64) {
    let inner = world.azure_minter.take().unwrap();
    let cached = CachedMinter::new(inner, std::time::Duration::from_secs(buffer_secs));
    world.cached_minter = Some(Arc::new(cached));
}

#[when("I mint via the Azure minter")]
fn mint_azure(world: &mut TWorld) {
    let m = world.azure_minter.clone().unwrap();
    let rt = world.rt();
    let token = rt.block_on(async move { m.mint().await.unwrap() });
    world.minted = Some(token);
}

#[when("I mint via the cached minter")]
fn mint_cached(world: &mut TWorld) {
    let m = world.cached_minter.clone().unwrap();
    let rt = world.rt();
    let token = rt.block_on(async move { m.mint().await.unwrap() });
    world.minted = Some(token);
}

#[then(expr = "the minted access_token is {string}")]
fn token_is(world: &mut TWorld, expected: String) {
    let t = world.minted.as_ref().unwrap();
    assert_eq!(t.access_token, expected);
}

#[then(expr = "the minted token expires at least {int} seconds from now")]
fn expires_at_least(world: &mut TWorld, expected: u64) {
    let t = world.minted.as_ref().unwrap();
    let r = t.seconds_remaining();
    assert!(r >= expected, "remaining was {r}, expected >= {expected}");
}

#[then(expr = "the cached minter underlying mint count is {int}")]
fn cached_call_count(world: &mut TWorld, expected: u64) {
    let m = world.cached_minter.as_ref().unwrap();
    assert_eq!(m.underlying_call_count(), expected);
}

fn main() {
    futures::executor::block_on(TWorld::run(
        "tests/features/broker_token_mint.feature",
    ));
}
