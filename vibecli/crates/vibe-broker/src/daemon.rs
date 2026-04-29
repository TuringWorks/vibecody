//! BrokerDaemon — assembles the broker, IMDS faker, and token refresher
//! from a `BrokerConfig` and runs them under one tokio runtime.
//!
//! The daemon entry point is what an operator invokes via
//! `vibecli broker start --config /etc/vibe/broker.toml`. It returns a
//! `DaemonHandle` that exposes the bound listener address(es) and aborts
//! the full stack on drop or explicit shutdown.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::accept::{BoundAddr, Broker, BrokerHandle};
use crate::audit::{AuditSink, JsonlFileAuditSink, NullAuditSink};
use crate::config::{BrokerConfig, ListenerKind};
use crate::imds::{ImdsHandle, ImdsServer};
use crate::policy::{Policy, SecretRef};
use crate::secrets::InMemorySecretStore;
use crate::ssrf::SsrfGuard;
use crate::tls::BrokerCa;
use crate::token_mint::{AzureClientCredentialsMinter, GcpServiceAccountMinter};
use crate::token_refresher::{RefreshHandle, TokenRefresher};

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("config: {0}")]
    Config(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("policy load: {0}")]
    Policy(String),
    #[error("tls: {0}")]
    Tls(String),
}

pub struct DaemonHandle {
    broker: BrokerHandle,
    imds: Option<ImdsHandle>,
    refresher: Option<RefreshHandle>,
    /// Public so callers can share it with sandbox spawn paths that need
    /// to inject `AWS_EC2_METADATA_SERVICE_ENDPOINT` etc.
    pub secrets: Arc<InMemorySecretStore>,
}

impl DaemonHandle {
    /// Address the broker accepted on. UDS callers get the path; TCP
    /// callers get a SocketAddr-shaped string.
    pub fn broker_addr(&self) -> &BoundAddr {
        &self.broker.addr
    }

    /// Address the IMDS faker bound to (`None` if no `[broker.imds]`
    /// section was configured).
    pub fn imds_addr(&self) -> Option<&std::net::SocketAddr> {
        self.imds.as_ref().map(|h| &h.addr)
    }

    /// Stop the accept loops. In-flight connections may still complete.
    pub fn shutdown(self) {
        self.broker.abort();
        if let Some(i) = &self.imds {
            i.abort();
        }
        if let Some(r) = &self.refresher {
            r.abort();
        }
    }
}

pub struct BrokerDaemon;

impl BrokerDaemon {
    /// Assemble + start the daemon stack. Async because the broker's
    /// `start_tcp` / `start_uds` and the IMDS server's `start` are async
    /// (they bind sockets); the refresher's `start()` is sync but spawns
    /// onto the current tokio runtime.
    pub async fn start(config: BrokerConfig) -> Result<DaemonHandle, DaemonError> {
        // ---- 1. Policy ----------------------------------------------
        let policy = match &config.policy.path {
            Some(p) => load_policy(p)?,
            None => Policy {
                default: crate::policy::DefaultRule::Deny,
                rule: vec![],
            },
        };

        // ---- 2. TLS CA ----------------------------------------------
        let tls_ca = match &config.broker.tls_ca_dir {
            Some(dir) => Some(Arc::new(
                BrokerCa::load_or_generate(dir).map_err(|e| DaemonError::Tls(e.to_string()))?,
            )),
            None => None,
        };

        // ---- 3. Audit sink ------------------------------------------
        let audit: Arc<dyn AuditSink> = match &config.broker.audit {
            Some(a) => Arc::new(
                JsonlFileAuditSink::open(&a.jsonl_path)
                    .map_err(|e| DaemonError::Io(e))?,
            ),
            None => Arc::new(NullAuditSink),
        };

        // ---- 4. SecretStore + TokenRefresher ------------------------
        let secrets = Arc::new(InMemorySecretStore::new());
        let refresher_handle = if !config.azure.is_empty() || !config.gcp.is_empty() {
            let interval = config
                .refresher
                .as_ref()
                .map(|r| Duration::from_secs(r.interval_secs))
                .unwrap_or(Duration::from_secs(300));
            let refresher = TokenRefresher::new(secrets.clone(), interval);
            for prof in &config.azure {
                let minter = AzureClientCredentialsMinter::new(
                    &prof.tenant,
                    &prof.client_id,
                    &prof.client_secret,
                    &prof.scope,
                );
                let minter = match &prof.endpoint {
                    Some(ep) => minter.with_endpoint(ep),
                    None => minter,
                };
                refresher
                    .register_azure(SecretRef(prof.secret_ref.clone()), Arc::new(minter))
                    .await;
            }
            for prof in &config.gcp {
                let key_pem = std::fs::read_to_string(&prof.private_key_pem_path)
                    .map_err(|e| DaemonError::Io(e))?;
                let minter = GcpServiceAccountMinter::new(
                    &prof.client_email,
                    key_pem,
                    &prof.scope,
                );
                let minter = match &prof.endpoint {
                    Some(ep) => minter.with_endpoint(ep),
                    None => minter,
                };
                refresher
                    .register_gcp(SecretRef(prof.secret_ref.clone()), Arc::new(minter))
                    .await;
            }
            Some(refresher.start())
        } else {
            None
        };

        // ---- 5. Broker ----------------------------------------------
        let mut broker = Broker::new(policy, SsrfGuard::new())
            .with_secret_store(secrets.clone())
            .with_audit_sink(audit.clone())
            .with_policy_id(config.broker.policy_id.clone());
        if config.broker.forward_upstream {
            broker = broker.with_upstream();
        }
        if let Some(ca) = tls_ca {
            broker = broker.with_tls_ca(ca);
        }

        let broker_handle = match config.listener_kind() {
            ListenerKind::Tcp => {
                let addr = config.broker.listen_tcp.as_ref().unwrap();
                broker
                    .start_tcp(addr)
                    .await
                    .map_err(DaemonError::Io)?
            }
            ListenerKind::Uds => {
                #[cfg(unix)]
                {
                    let path = config.broker.listen_uds.as_ref().unwrap();
                    broker
                        .start_uds(path)
                        .await
                        .map_err(DaemonError::Io)?
                }
                #[cfg(not(unix))]
                {
                    return Err(DaemonError::Config(
                        "UDS listener requested on non-Unix host".into(),
                    ));
                }
            }
        };

        // ---- 6. IMDS server ----------------------------------------
        let imds_handle = match &config.broker.imds {
            Some(i) => {
                let server = ImdsServer::new(
                    &i.role_name,
                    SecretRef(i.secret_ref.clone()),
                    secrets.clone() as Arc<dyn crate::secrets::SecretStore>,
                )
                .with_audit_sink(audit.clone());
                Some(server.start(&i.listen_tcp).await.map_err(DaemonError::Io)?)
            }
            None => None,
        };

        Ok(DaemonHandle {
            broker: broker_handle,
            imds: imds_handle,
            refresher: refresher_handle,
            secrets,
        })
    }
}

fn load_policy(path: &Path) -> Result<Policy, DaemonError> {
    let text = std::fs::read_to_string(path).map_err(DaemonError::Io)?;
    Policy::parse_toml(&text).map_err(|e| DaemonError::Policy(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_policy_when_path_absent() {
        let cfg_toml = r#"
[broker]
listen_tcp = "127.0.0.1:0"
"#;
        let cfg = BrokerConfig::from_toml_str(cfg_toml).unwrap();
        let h = BrokerDaemon::start(cfg).await.unwrap();
        match h.broker_addr() {
            BoundAddr::Tcp(addr) => assert!(addr.port() != 0),
            _ => panic!("expected TCP"),
        }
        h.shutdown();
    }
}
