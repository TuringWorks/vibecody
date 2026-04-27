//! Per-skill / per-agent egress policy DSL.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Policy {
    #[serde(default = "default_deny")]
    pub default: DefaultRule,
    #[serde(default)]
    pub rule: Vec<Rule>,
}

fn default_deny() -> DefaultRule {
    DefaultRule::Deny
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DefaultRule {
    Deny,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    #[serde(rename = "match")]
    pub match_: RuleMatch,
    #[serde(default)]
    pub inject: Inject,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleMatch {
    pub host: String,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub path_pattern: Option<String>,
    #[serde(default = "default_true")]
    pub require_tls: bool,
    #[serde(default)]
    pub require_user_consent: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Inject {
    #[default]
    None,
    Bearer { key: SecretRef },
    Basic { user: SecretRef, pass: SecretRef },
    #[serde(rename = "aws-sigv4")]
    AwsSigV4 { profile: SecretRef },
    #[serde(rename = "gcp-iam")]
    GcpIam { service_account: SecretRef },
    #[serde(rename = "azure-msi")]
    AzureMsi { client_id: SecretRef },
    HeaderTemplate { name: String, value_template: String },
}

impl Inject {
    pub fn type_name(&self) -> &'static str {
        match self {
            Inject::None => "None",
            Inject::Bearer { .. } => "Bearer",
            Inject::Basic { .. } => "Basic",
            Inject::AwsSigV4 { .. } => "AwsSigV4",
            Inject::GcpIam { .. } => "GcpIam",
            Inject::AzureMsi { .. } => "AzureMsi",
            Inject::HeaderTemplate { .. } => "HeaderTemplate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(transparent)]
pub struct SecretRef(pub String);

impl SecretRef {
    pub fn new(s: impl Into<String>) -> Self {
        SecretRef(s.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Limits {
    #[serde(default)]
    pub max_request_body: Option<String>,
    #[serde(default)]
    pub max_response_body: Option<String>,
    #[serde(default)]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision<'a> {
    Allow {
        rule_index: usize,
        inject: &'a Inject,
    },
    Deny,
}

#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub method: &'a str,
    pub url: &'a str,
}

impl Policy {
    pub fn parse_toml(text: &str) -> Result<Self, super::BrokerError> {
        toml::from_str(text).map_err(|e| super::BrokerError::PolicyParse(e.to_string()))
    }

    pub fn match_request(&self, req: &Request<'_>) -> Decision<'_> {
        let parsed = match url::Url::parse(req.url) {
            Ok(u) => u,
            Err(_) => return Decision::Deny,
        };
        let host = match parsed.host_str() {
            Some(h) => h.to_owned(),
            None => return Decision::Deny,
        };
        let path = parsed.path();
        let scheme = parsed.scheme();
        for (i, rule) in self.rule.iter().enumerate() {
            if rule.match_.require_tls && scheme != "https" {
                continue;
            }
            if !host_glob_match(&rule.match_.host, &host) {
                continue;
            }
            if !rule.match_.methods.is_empty() {
                let m_upper = req.method.to_ascii_uppercase();
                if !rule.match_.methods.iter().any(|m| m.eq_ignore_ascii_case(&m_upper)) {
                    continue;
                }
            }
            if let Some(prefix) = &rule.match_.path_prefix {
                if !path.starts_with(prefix.as_str()) {
                    continue;
                }
            }
            return Decision::Allow {
                rule_index: i,
                inject: &rule.inject,
            };
        }
        Decision::Deny
    }
}

/// Simple glob matcher supporting only `*` as a leading wildcard.
/// `*.openai.com` matches `api.openai.com` and `openai.com`.
/// Exact host strings match literally.
pub fn host_glob_match(glob: &str, host: &str) -> bool {
    if let Some(suffix) = glob.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else if glob == "*" {
        true
    } else {
        glob.eq_ignore_ascii_case(host)
    }
}

impl FromStr for Policy {
    type Err = super::BrokerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Policy::parse_toml(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_policy_denies() {
        let p = Policy {
            default: DefaultRule::Deny,
            rule: vec![],
        };
        let d = p.match_request(&Request {
            method: "GET",
            url: "https://api.openai.com/v1",
        });
        assert!(matches!(d, Decision::Deny));
    }

    #[test]
    fn host_glob_matches_subdomain() {
        assert!(host_glob_match("*.openai.com", "api.openai.com"));
        assert!(host_glob_match("*.openai.com", "openai.com"));
        assert!(!host_glob_match("*.openai.com", "openai.evil.com"));
    }

    #[test]
    fn host_glob_exact() {
        assert!(host_glob_match("api.github.com", "api.github.com"));
        assert!(!host_glob_match("api.github.com", "api.gitlab.com"));
    }

    #[test]
    fn parse_toml_round_trip() {
        let toml = r#"
default = "deny"

[[rule]]
match.host = "api.example.com"
match.methods = ["GET"]
inject = { type = "bearer", key = "@profile.example_key" }
"#;
        let p = Policy::parse_toml(toml).unwrap();
        assert_eq!(p.rule.len(), 1);
        assert_eq!(p.rule[0].match_.host, "api.example.com");
    }

    #[test]
    fn aws_sigv4_inject_parses() {
        let toml = r#"
default = "deny"

[[rule]]
match.host = "*.amazonaws.com"
match.methods = ["GET", "POST"]
inject = { type = "aws-sigv4", profile = "@workspace.aws_default" }
"#;
        let p = Policy::parse_toml(toml).unwrap();
        assert_eq!(p.rule[0].inject.type_name(), "AwsSigV4");
    }

    #[test]
    fn method_filter_denies_other() {
        let toml = r#"
default = "deny"

[[rule]]
match.host = "*.openai.com"
match.methods = ["GET"]
inject = { type = "bearer", key = "@profile.openai_key" }
"#;
        let p = Policy::parse_toml(toml).unwrap();
        let d = p.match_request(&Request {
            method: "POST",
            url: "https://api.openai.com/v1",
        });
        assert!(matches!(d, Decision::Deny));
    }

    #[test]
    fn path_prefix_narrows() {
        let toml = r#"
default = "deny"

[[rule]]
match.host = "api.github.com"
match.methods = ["GET"]
match.path_prefix = "/repos/me/myrepo/"
inject = { type = "bearer", key = "@workspace.github_token" }
"#;
        let p = Policy::parse_toml(toml).unwrap();
        let allow = p.match_request(&Request {
            method: "GET",
            url: "https://api.github.com/repos/me/myrepo/issues",
        });
        let deny = p.match_request(&Request {
            method: "GET",
            url: "https://api.github.com/repos/other/other/issues",
        });
        assert!(matches!(allow, Decision::Allow { .. }));
        assert!(matches!(deny, Decision::Deny));
    }
}
