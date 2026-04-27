//! Egress broker — out-of-sandbox HTTP forward proxy with policy,
//! credential injection, and audit. See
//! `docs/design/sandbox-tiers/02-egress-broker.md`.
//!
//! v1 ships the policy DSL parser, host/method matching, SSRF guard, and
//! audit-event types. Hyper-based accept loop, rustls handshake, IMDS
//! faker, and SigV4/Bearer injection runtime arrive in slices B1.4+.

pub mod accept;
pub mod audit;
pub mod forward;
pub mod imds;
pub mod mitm;
pub mod policy;
pub mod secrets;
pub mod ssrf;
pub mod tls;

pub use accept::{BoundAddr, Broker, BrokerHandle};
pub use audit::{
    AuditEvent, AuditSink, EgressOutcome, MemoryAuditSink, NullAuditSink,
    baseline_egress_request,
};
pub use forward::{ForwardError, ForwardRequest, ForwardResponse, forward_plain_http};
pub use imds::{ImdsHandle, ImdsServer};
pub use mitm::{InspectContext, MitmError, default_upstream_roots, run_mitm};
pub use policy::{Decision, Inject, Policy, Rule, RuleMatch, SecretRef};
pub use secrets::{
    AwsCredentials, AzureAccessToken, EmptySecretStore, GcpAccessToken, InMemorySecretStore,
    SecretStore,
};
pub use ssrf::{SsrfGuard, SsrfVerdict};
pub use tls::{BrokerCa, LeafCert, TlsError};

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("policy parse error: {0}")]
    PolicyParse(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
