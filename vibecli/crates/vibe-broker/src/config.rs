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

/// Which socket the broker will bind, carrying the address it needs.
///
/// The variants carry their value rather than being bare tags. As tags, the
/// kind was inferred with `if listen_tcp.is_some() { Tcp } else { Uds }` —
/// which returns `Uds` whenever TCP is unset, *without checking `listen_uds`
/// is set at all*. The caller then unwrapped it. `validate()` rejects that
/// combination, but only on the file-load path, so any `BrokerConfig` built as
/// a struct literal or via `Default` panicked at daemon startup. Carrying the
/// value makes the empty case unrepresentable instead of merely documented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerKind<'a> {
    Tcp(&'a str),
    Uds(&'a std::path::Path),
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
        let parsed: Self = toml::from_str(text).map_err(|e| ConfigError::Toml(e.to_string()))?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_toml_str(&text)
    }

    /// Which listener to bind, together with its address.
    ///
    /// Fallible because "exactly one of TCP / UDS" is a property of the
    /// *config*, not of the type, and a config can reach here without passing
    /// through `from_toml_str`. Returning the address with the variant means
    /// the caller has nothing left to unwrap.
    pub fn listener_kind(&self) -> Result<ListenerKind<'_>, ConfigError> {
        match (&self.broker.listen_tcp, &self.broker.listen_uds) {
            (Some(addr), None) => Ok(ListenerKind::Tcp(addr)),
            (None, Some(path)) => Ok(ListenerKind::Uds(path)),
            (Some(_), Some(_)) => Err(ConfigError::Invalid(
                "specify exactly one of broker.listen_tcp or broker.listen_uds".into(),
            )),
            (None, None) => Err(ConfigError::Invalid(
                "broker.listen_tcp or broker.listen_uds must be set".into(),
            )),
        }
    }

    /// The configured listener address, or empty when the config names none.
    pub fn listener_address(&self) -> String {
        match self.listener_kind() {
            Ok(ListenerKind::Tcp(addr)) => addr.to_string(),
            Ok(ListenerKind::Uds(path)) => path.to_string_lossy().into_owned(),
            Err(_) => String::new(),
        }
    }

    /// Validation is now exactly "can we name a listener?", so the rule lives
    /// in one place instead of being duplicated here and in `listener_kind`.
    fn validate(&self) -> Result<(), ConfigError> {
        self.listener_kind().map(|_| ())
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
        assert_eq!(
            cfg.listener_kind().unwrap(),
            ListenerKind::Tcp("127.0.0.1:8080")
        );
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
        assert_eq!(
            cfg.listener_kind().unwrap(),
            ListenerKind::Uds(std::path::Path::new("/run/vibe-broker.sock"))
        );
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

    /// The hole that made `listener_kind` fallible. A config that never went
    /// through `from_toml_str` — a struct literal in a caller, or `Default` —
    /// skips `validate`, and the old bare-tag version answered `Uds` for this
    /// purely because TCP was unset. The daemon then unwrapped `listen_uds`
    /// and panicked at startup. It must be an error, not a variant.
    #[test]
    fn a_config_naming_no_listener_is_an_error_even_without_validate() {
        let cfg = BrokerConfig {
            broker: BrokerSection {
                listen_tcp: None,
                listen_uds: None,
                policy_id: "x".into(),
                tls_ca_dir: None,
                forward_upstream: false,
                audit: None,
                imds: None,
            },
            policy: PolicySection::default(),
            refresher: None,
            azure: Vec::new(),
            gcp: Vec::new(),
        };

        assert!(matches!(cfg.listener_kind(), Err(ConfigError::Invalid(_))));
        assert_eq!(cfg.listener_address(), "");
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
        assert!(matches!(cfg.listener_kind(), Ok(ListenerKind::Uds(_))));
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
