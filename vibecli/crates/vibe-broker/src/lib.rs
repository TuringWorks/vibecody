//! Egress broker — out-of-sandbox HTTP forward proxy with policy,
//! credential injection, and audit. See
//! `docs/design/sandbox-tiers/02-egress-broker.md`.
//!
//! v1 ships the policy DSL parser, host/method matching, SSRF guard, and
//! audit-event types. Hyper-based accept loop, rustls handshake, IMDS
//! faker, and SigV4/Bearer injection runtime arrive in slices B1.4+.

pub mod audit;
pub mod policy;
pub mod ssrf;

pub use audit::{AuditEvent, EgressOutcome};
pub use policy::{Decision, Inject, Policy, Rule, RuleMatch, SecretRef};
pub use ssrf::{SsrfGuard, SsrfVerdict};

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("policy parse error: {0}")]
    PolicyParse(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
