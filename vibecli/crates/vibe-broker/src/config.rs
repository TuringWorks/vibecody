//! TOML configuration for the vibe-broker daemon.
//!
//! The daemon reads one config file and produces a ready-to-start
//! triad: `Broker` (the egress proxy), `ImdsServer` (cloud-credential
//! faker), and `TokenRefresher` (background OAuth minter). Tests
//! construct these manually; production callers use `BrokerConfig`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrokerConfig {
    pub broker: BrokerSection,
    #[serde(default)]
    pub policy: PolicySection,
    #[serde(default)]
    pub refresher: Option<RefresherSection>,
    /// `[[azure]]` arrays of cloud profiles (Service Principal /
    /// client_credentials). Each entry has a SecretRef key the broker
    /// looks up via `SecretStore::resolve_azure`.
    #[serde(default, rename = "azure")]
    pub azure: Vec<AzureProfile>,
    /// `[[gcp]]` arrays of cloud profiles (service-account JSON key).
    #[serde(default, rename = "gcp")]
    pub gcp: Vec<GcpProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrokerSection {
    /// `tcp` or `uds`. Exactly one of `listen_tcp` / `listen_uds` must
    /// be set; the parser surfaces a structured error if both or
    /// neither appear.
    #[serde(default)]
    pub listen_tcp: Option<String>,
    #[serde(default)]
    pub listen_uds: Option<PathBuf>,
    #[serde(default = "default_policy_id")]
    pub policy_id: String,
    /// When set, the broker mints a per-broker root CA + leaf certs in
    /// this directory (mode 0700). Required for HTTPS interception
    /// (B1.7+).
    #[serde(default)]
    pub tls_ca_dir: Option<PathBuf>,
    /// When set, allowed requests are forwarded upstream rather than
    /// returned as the stub 200. Production callers want this on.
    #[serde(default)]
    pub forward_upstream: bool,
    /// Audit sink configuration (slice B5.3).
    #[serde(default)]
    pub audit: Option<AuditSection>,
    /// IMDS faker (slice B3). Off when absent.
    #[serde(default)]
    pub imds: Option<ImdsSection>,
}

fn default_policy_id() -> String {
    "broker".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PolicySection {
    /// Path to a TOML file containing the policy DSL (rules, etc).
    /// When absent, the broker uses an empty policy (deny all).
    #[serde(default)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditSection {
    /// Path of the JSONL audit log. Parent dirs are created on open.
    pub jsonl_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImdsSection {
    /// IMDS role name surfaced via `/security-credentials/`.
    pub role_name: String,
    /// SecretRef the IMDS faker looks up to get AwsCredentials.
    pub secret_ref: String,
    /// Where the faker binds. Operators alias 169.254.169.254 to this.
    pub listen_tcp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefresherSection {
    /// Refresh interval in seconds. The first tick fires immediately on
    /// `start()`; subsequent ticks fire every `interval_secs`.
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AzureProfile {
    pub secret_ref: String,
    pub tenant: String,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
    /// Optional override for the OAuth endpoint (production defaults
    /// to https://login.microsoftonline.com).
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcpProfile {
    pub secret_ref: String,
    pub client_email: String,
    /// Path to the service-account private key (PKCS#8 PEM).
    pub private_key_pem_path: PathBuf,
    pub scope: String,
    /// Optional override for the OAuth endpoint (production defaults
    /// to https://oauth2.googleapis.com).
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerKind {
    Tcp,
    Uds,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("toml parse: {0}")]
    Toml(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid: {0}")]
    Invalid(String),
}

impl BrokerConfig {
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        let parsed: Self =
            toml::from_str(text).map_err(|e| ConfigError::Toml(e.to_string()))?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_toml_str(&text)
    }

    /// Inspect the listener configuration. The validator already
    /// guarantees exactly one of TCP / UDS is set, so unwrapping is
    /// safe in production.
    pub fn listener_kind(&self) -> ListenerKind {
        if self.broker.listen_tcp.is_some() {
            ListenerKind::Tcp
        } else {
            ListenerKind::Uds
        }
    }

    pub fn listener_address(&self) -> String {
        if let Some(addr) = &self.broker.listen_tcp {
            addr.clone()
        } else if let Some(path) = &self.broker.listen_uds {
            path.to_string_lossy().into_owned()
        } else {
            String::new()
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let has_tcp = self.broker.listen_tcp.is_some();
        let has_uds = self.broker.listen_uds.is_some();
        match (has_tcp, has_uds) {
            (true, true) => Err(ConfigError::Invalid(
                "specify exactly one of broker.listen_tcp or broker.listen_uds".into(),
            )),
            (false, false) => Err(ConfigError::Invalid(
                "broker.listen_tcp or broker.listen_uds must be set".into(),
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_tcp_config() {
        let toml = r#"
[broker]
listen_tcp = "127.0.0.1:8080"
policy_id = "skill:test"
"#;
        let cfg = BrokerConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.listener_kind(), ListenerKind::Tcp);
        assert_eq!(cfg.listener_address(), "127.0.0.1:8080");
        assert_eq!(cfg.broker.policy_id, "skill:test");
    }

    #[test]
    fn parses_minimal_uds_config() {
        let toml = r#"
[broker]
listen_uds = "/run/vibe-broker.sock"
"#;
        let cfg = BrokerConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.listener_kind(), ListenerKind::Uds);
        assert_eq!(cfg.listener_address(), "/run/vibe-broker.sock");
        assert_eq!(cfg.broker.policy_id, "broker");
    }

    #[test]
    fn rejects_both_listeners() {
        let toml = r#"
[broker]
listen_tcp = "127.0.0.1:8080"
listen_uds = "/run/x.sock"
"#;
        let err = BrokerConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_neither_listener() {
        let toml = r#"
[broker]
policy_id = "x"
"#;
        let err = BrokerConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn parses_full_config_with_imds_and_audit() {
        let toml = r#"
[broker]
listen_uds = "/run/vibe-broker.sock"
tls_ca_dir = "/var/run/vibe-ca"
forward_upstream = true

[broker.audit]
jsonl_path = "/var/log/vibe-audit.jsonl"

[broker.imds]
role_name = "vibe-broker-role"
secret_ref = "@workspace.aws_default"
listen_tcp = "127.0.0.1:8181"

[refresher]
interval_secs = 300

[[azure]]
secret_ref = "@workspace.azure_default"
tenant = "tenant42"
client_id = "client42"
client_secret = "secret42"
scope = "https://graph.microsoft.com/.default"

[[gcp]]
secret_ref = "@workspace.gcp_default"
client_email = "sa@example.iam.gserviceaccount.com"
private_key_pem_path = "/etc/vibe/gcp-key.pem"
scope = "https://www.googleapis.com/auth/cloud-platform"
"#;
        let cfg = BrokerConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.listener_kind(), ListenerKind::Uds);
        assert_eq!(
            cfg.broker.tls_ca_dir.as_deref(),
            Some(std::path::Path::new("/var/run/vibe-ca"))
        );
        assert!(cfg.broker.forward_upstream);
        assert_eq!(
            cfg.broker.audit.as_ref().unwrap().jsonl_path,
            std::path::Path::new("/var/log/vibe-audit.jsonl")
        );
        let imds = cfg.broker.imds.as_ref().unwrap();
        assert_eq!(imds.role_name, "vibe-broker-role");
        assert_eq!(imds.listen_tcp, "127.0.0.1:8181");
        assert_eq!(cfg.refresher.as_ref().unwrap().interval_secs, 300);
        assert_eq!(cfg.azure.len(), 1);
        assert_eq!(cfg.azure[0].tenant, "tenant42");
        assert_eq!(cfg.gcp.len(), 1);
        assert_eq!(
            cfg.gcp[0].client_email,
            "sa@example.iam.gserviceaccount.com"
        );
    }

    #[test]
    fn malformed_toml_returns_error() {
        let toml = "this is not = valid = toml syntax]]]";
        let err = BrokerConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ConfigError::Toml(_)));
    }
}
