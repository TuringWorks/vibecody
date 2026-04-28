//! BDD: TokenRefresher — uses an Arc<AtomicU64> counting StubMinter so
//! the test can assert exact mint counts without running a real OAuth
//! HTTP stub on every scenario.

use async_trait::async_trait;
use cucumber::{World, given, then, when};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Runtime;
use vibe_broker::{
    InMemorySecretStore, MintError, MintedToken, RefreshHandle, SecretStore, TokenMinter,
    TokenRefresher,
    policy::SecretRef,
};

struct StubMinter {
    token: String,
    calls: Arc<AtomicU64>,
}

#[async_trait]
impl TokenMinter for StubMinter {
    async fn mint(&self) -> Result<MintedToken, MintError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(MintedToken::from_expires_in(self.token.clone(), 3600))
    }
}

#[derive(Default, World)]
pub struct RWorld {
    rt: Option<Arc<Runtime>>,
    secrets: Option<Arc<InMemorySecretStore>>,
    refresher: Option<TokenRefresher>,
    handle: Option<RefreshHandle>,
    stub_calls: Arc<AtomicU64>,
    stub_token: String,
    plateau_after: u64,
}

impl std::fmt::Debug for RWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RWorld")
            .field(
                "stub_calls",
                &self.stub_calls.load(Ordering::SeqCst),
            )
            .finish()
    }
}

impl RWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

#[given(expr = "a stub Azure OAuth endpoint returning access_token {string} with expires_in {int}")]
fn stub_endpoint(world: &mut RWorld, token: String, _expires: u64) {
    world.stub_token = token;
    world.stub_calls = Arc::new(AtomicU64::new(0));
}

#[given("an InMemorySecretStore")]
fn fresh_store(world: &mut RWorld) {
    world.secrets = Some(Arc::new(InMemorySecretStore::new()));
}

#[given(expr = "a TokenRefresher with {int}ms interval")]
fn fresh_refresher(world: &mut RWorld, interval_ms: u64) {
    let secrets = world.secrets.clone().unwrap();
    let r = TokenRefresher::new(secrets, Duration::from_millis(interval_ms));
    world.refresher = Some(r);
}

#[when(expr = "I register the Azure profile {string} against the stub")]
fn register(world: &mut RWorld, key: String) {
    let calls = world.stub_calls.clone();
    let token = world.stub_token.clone();
    let rt = world.rt();
    let r = world.refresher.as_ref().unwrap();
    rt.block_on(async {
        r.register_azure(
            SecretRef(key),
            Arc::new(StubMinter { token, calls }),
        )
        .await;
    });
}

#[when("I start the refresher")]
fn start(world: &mut RWorld) {
    let rt = world.rt();
    let r = world.refresher.take().unwrap();
    let handle = rt.block_on(async move { r.start() });
    world.handle = Some(handle);
}

#[when("I wait for the first refresh")]
fn wait_first(world: &mut RWorld) {
    let calls = world.stub_calls.clone();
    let rt = world.rt();
    rt.block_on(async move {
        for _ in 0..200 {
            if calls.load(Ordering::SeqCst) >= 1 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("first refresh never fired");
    });
}

#[when("I stop the refresher")]
fn stop(world: &mut RWorld) {
    let h = world.handle.as_ref().unwrap();
    h.abort();
    // Record the count at stop time. We allow a small grace window
    // for any in-flight mint to land before snapshotting.
    let calls = world.stub_calls.clone();
    let rt = world.rt();
    rt.block_on(async move { tokio::time::sleep(Duration::from_millis(80)).await });
    world.plateau_after = calls.load(Ordering::SeqCst);
}

#[then(expr = "the SecretStore has an Azure token at {string} equal to {string}")]
fn store_has_token(world: &mut RWorld, key: String, expected: String) {
    let s = world.secrets.as_ref().unwrap();
    let token = s.resolve_azure(&SecretRef(key)).expect("token present");
    assert_eq!(token.token, expected);
}

#[then("the underlying mint count plateaus")]
fn plateaus(world: &mut RWorld) {
    let calls = world.stub_calls.clone();
    let rt = world.rt();
    let after = world.plateau_after;
    let later = rt.block_on(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        calls.load(Ordering::SeqCst)
    });
    assert!(
        later <= after,
        "mint count grew after stop: was {after}, now {later}"
    );
}

fn main() {
    futures::executor::block_on(RWorld::run(
        "tests/features/broker_token_refresher.feature",
    ));
}
