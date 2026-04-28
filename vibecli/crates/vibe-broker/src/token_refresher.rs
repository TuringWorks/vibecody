//! Daemon-side token refresher.
//!
//! Periodic task that calls each registered minter, takes the resulting
//! `MintedToken`, and stuffs it into the `InMemorySecretStore` that the
//! broker's hot path reads from. The refresh loop is what closes the gap
//! between async OAuth IO and the broker's sync `SecretStore::resolve_*`
//! calls — the broker never blocks on a token mint.
//!
//! Production wiring: the daemon constructs a `TokenRefresher` per
//! workspace, registers each cloud profile from config, and calls
//! `start()` once at startup. The returned `RefreshHandle` is dropped at
//! shutdown to stop the loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::policy::SecretRef;
use crate::secrets::{AzureAccessToken, GcpAccessToken, InMemorySecretStore};
use crate::token_mint::TokenMinter;

#[derive(Clone)]
enum CloudKind {
    Gcp,
    Azure,
}

struct Registration {
    kind: CloudKind,
    minter: Arc<dyn TokenMinter>,
}

#[derive(Default)]
struct RefresherState {
    profiles: HashMap<String, Registration>,
    /// Number of times the refresher minted across all profiles. Tests
    /// assert the count plateaus after `stop()`.
    mint_count: u64,
}

pub struct TokenRefresher {
    secrets: Arc<InMemorySecretStore>,
    interval: Duration,
    state: Arc<Mutex<RefresherState>>,
}

pub struct RefreshHandle {
    join: JoinHandle<()>,
    state: Arc<Mutex<RefresherState>>,
}

impl RefreshHandle {
    /// Abort the refresh loop. In-flight mints may still complete.
    pub fn abort(&self) {
        self.join.abort();
    }

    /// Snapshot the cumulative mint count (across all profiles, all
    /// ticks). Used in tests to assert the loop has stopped.
    pub async fn mint_count(&self) -> u64 {
        self.state.lock().await.mint_count
    }
}

impl Drop for RefreshHandle {
    fn drop(&mut self) {
        self.join.abort();
    }
}

impl TokenRefresher {
    pub fn new(secrets: Arc<InMemorySecretStore>, interval: Duration) -> Self {
        TokenRefresher {
            secrets,
            interval,
            state: Arc::new(Mutex::new(RefresherState::default())),
        }
    }

    pub async fn register_gcp(&self, secret_ref: SecretRef, minter: Arc<dyn TokenMinter>) {
        self.state.lock().await.profiles.insert(
            secret_ref.0,
            Registration {
                kind: CloudKind::Gcp,
                minter,
            },
        );
    }

    pub async fn register_azure(&self, secret_ref: SecretRef, minter: Arc<dyn TokenMinter>) {
        self.state.lock().await.profiles.insert(
            secret_ref.0,
            Registration {
                kind: CloudKind::Azure,
                minter,
            },
        );
    }

    /// Snapshot how many mints have completed across all profiles.
    pub async fn mint_count(&self) -> u64 {
        self.state.lock().await.mint_count
    }

    /// Start the refresh loop. The first refresh fires immediately, then
    /// every `interval`.
    pub fn start(self) -> RefreshHandle {
        let secrets = self.secrets.clone();
        let interval = self.interval;
        let state = self.state.clone();
        let join = tokio::spawn(async move {
            loop {
                let snapshot: Vec<(String, Registration)> = {
                    let s = state.lock().await;
                    s.profiles
                        .iter()
                        .map(|(k, r)| {
                            (
                                k.clone(),
                                Registration {
                                    kind: r.kind.clone(),
                                    minter: r.minter.clone(),
                                },
                            )
                        })
                        .collect()
                };
                for (key, reg) in &snapshot {
                    match reg.minter.mint().await {
                        Ok(token) => {
                            match reg.kind {
                                CloudKind::Gcp => secrets.set_gcp(
                                    key.clone(),
                                    GcpAccessToken {
                                        token: token.access_token,
                                    },
                                ),
                                CloudKind::Azure => secrets.set_azure(
                                    key.clone(),
                                    AzureAccessToken {
                                        token: token.access_token,
                                    },
                                ),
                            }
                            state.lock().await.mint_count += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "token_refresher: mint for {key} failed: {e}"
                            );
                        }
                    }
                }
                tokio::time::sleep(interval).await;
            }
        });
        RefreshHandle { join, state: self.state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_mint::{MintError, MintedToken};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU64, Ordering};

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

    #[tokio::test]
    async fn first_tick_populates_store() {
        let secrets = Arc::new(InMemorySecretStore::new());
        let refresher =
            TokenRefresher::new(secrets.clone(), Duration::from_millis(50));
        let calls = Arc::new(AtomicU64::new(0));
        refresher
            .register_azure(
                SecretRef("@workspace.azure_default".into()),
                Arc::new(StubMinter {
                    token: "stub-azure".into(),
                    calls: calls.clone(),
                }),
            )
            .await;
        let handle = refresher.start();
        // Wait long enough for the first tick.
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            if let Some(t) = secrets.resolve_azure(&SecretRef("@workspace.azure_default".into()))
            {
                assert_eq!(t.token, "stub-azure");
                handle.abort();
                return;
            }
        }
        panic!("token never landed in store");
    }
}
