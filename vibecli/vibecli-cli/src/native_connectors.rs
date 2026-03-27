//! Native integration connectors for 20 third-party services.
//!
//! Gap 14 — Auto-discovers connectors from project config files, manages
//! health checks, webhook ingestion, and connector lifecycle.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported third-party connector types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectorType {
    Stripe,
    Figma,
    Notion,
    Jira,
    Slack,
    PagerDuty,
    Datadog,
    Sentry,
    LaunchDarkly,
    Vercel,
    Netlify,
    Supabase,
    Firebase,
    Aws,
    Gcp,
    Azure,
    GitHub,
    GitLab,
    Linear,
    Confluence,
}

impl ConnectorType {
    pub fn all() -> Vec<ConnectorType> {
        vec![
            Self::Stripe, Self::Figma, Self::Notion, Self::Jira, Self::Slack,
            Self::PagerDuty, Self::Datadog, Self::Sentry, Self::LaunchDarkly,
            Self::Vercel, Self::Netlify, Self::Supabase, Self::Firebase,
            Self::Aws, Self::Gcp, Self::Azure, Self::GitHub, Self::GitLab,
            Self::Linear, Self::Confluence,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Stripe => "stripe",
            Self::Figma => "figma",
            Self::Notion => "notion",
            Self::Jira => "jira",
            Self::Slack => "slack",
            Self::PagerDuty => "pagerduty",
            Self::Datadog => "datadog",
            Self::Sentry => "sentry",
            Self::LaunchDarkly => "launchdarkly",
            Self::Vercel => "vercel",
            Self::Netlify => "netlify",
            Self::Supabase => "supabase",
            Self::Firebase => "firebase",
            Self::Aws => "aws",
            Self::Gcp => "gcp",
            Self::Azure => "azure",
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
            Self::Linear => "linear",
            Self::Confluence => "confluence",
        }
    }

    pub fn default_base_url(&self) -> &str {
        match self {
            Self::Stripe => "https://api.stripe.com",
            Self::Figma => "https://api.figma.com",
            Self::Notion => "https://api.notion.com",
            Self::Jira => "https://your-domain.atlassian.net",
            Self::Slack => "https://slack.com/api",
            Self::PagerDuty => "https://api.pagerduty.com",
            Self::Datadog => "https://api.datadoghq.com",
            Self::Sentry => "https://sentry.io/api",
            Self::LaunchDarkly => "https://app.launchdarkly.com/api",
            Self::Vercel => "https://api.vercel.com",
            Self::Netlify => "https://api.netlify.com",
            Self::Supabase => "https://api.supabase.com",
            Self::Firebase => "https://firebase.googleapis.com",
            Self::Aws => "https://amazonaws.com",
            Self::Gcp => "https://cloud.google.com",
            Self::Azure => "https://management.azure.com",
            Self::GitHub => "https://api.github.com",
            Self::GitLab => "https://gitlab.com/api/v4",
            Self::Linear => "https://api.linear.app",
            Self::Confluence => "https://your-domain.atlassian.net/wiki",
        }
    }
}

/// OAuth configuration for connectors that need it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
}

/// Configuration for a single connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub connector_type: ConnectorType,
    pub api_key: Option<String>,
    pub base_url: String,
    pub oauth_config: Option<OAuthConfig>,
    pub enabled: bool,
    pub custom_headers: HashMap<String, String>,
}

impl ConnectorConfig {
    pub fn new(connector_type: ConnectorType) -> Self {
        let base_url = connector_type.default_base_url().to_string();
        Self {
            connector_type,
            api_key: None,
            base_url,
            oauth_config: None,
            enabled: true,
            custom_headers: HashMap::new(),
        }
    }

    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }
}

/// Current status of a connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectorStatus {
    Connected,
    Disconnected,
    AuthRequired,
    Error(String),
}

/// A live connector instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInstance {
    pub id: String,
    pub config: ConnectorConfig,
    pub status: ConnectorStatus,
    pub last_health_check: Option<u64>,
    pub requests_count: u64,
    pub errors_count: u64,
    pub created_at: u64,
}

impl PartialEq for ConnectorInstance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Aggregate metrics across all connectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct ConnectorMetrics {
    pub total_connectors: usize,
    pub connected_count: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub auto_discovered: u64,
}


/// Webhook event received by the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub connector_id: String,
    pub event_type: String,
    pub payload: String,
    pub timestamp: u64,
}

/// Registry managing all connector instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorRegistry {
    pub connectors: HashMap<String, ConnectorInstance>,
    pub webhook_endpoint: Option<String>,
    pub webhook_events: Vec<WebhookEvent>,
    pub metrics: ConnectorMetrics,
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
            webhook_endpoint: None,
            webhook_events: Vec::new(),
            metrics: ConnectorMetrics::default(),
        }
    }

    pub fn with_webhook_endpoint(mut self, endpoint: &str) -> Self {
        self.webhook_endpoint = Some(endpoint.to_string());
        self
    }

    /// Register a new connector.
    pub fn register(&mut self, config: ConnectorConfig) -> Result<String, String> {
        let id = format!("conn-{}-{}", config.connector_type.name(), self.connectors.len());

        if self.connectors.values().any(|c| {
            c.config.connector_type == config.connector_type && c.config.base_url == config.base_url
        }) {
            return Err(format!("Connector {} already registered for {}", config.connector_type.name(), config.base_url));
        }

        let status = if config.api_key.is_some() || config.oauth_config.is_some() {
            ConnectorStatus::Connected
        } else {
            ConnectorStatus::AuthRequired
        };

        let instance = ConnectorInstance {
            id: id.clone(),
            config,
            status,
            last_health_check: None,
            requests_count: 0,
            errors_count: 0,
            created_at: 0,
        };
        self.connectors.insert(id.clone(), instance);
        self.metrics.total_connectors = self.connectors.len();
        self.metrics.connected_count = self.connectors.values()
            .filter(|c| c.status == ConnectorStatus::Connected).count();
        Ok(id)
    }

    /// Unregister a connector by ID.
    pub fn unregister(&mut self, id: &str) -> Result<ConnectorInstance, String> {
        let inst = self.connectors.remove(id)
            .ok_or_else(|| format!("Connector {} not found", id))?;
        self.metrics.total_connectors = self.connectors.len();
        self.metrics.connected_count = self.connectors.values()
            .filter(|c| c.status == ConnectorStatus::Connected).count();
        Ok(inst)
    }

    /// Run a health check on a connector (simulated).
    pub fn health_check(&mut self, id: &str) -> Result<ConnectorStatus, String> {
        let conn = self.connectors.get_mut(id)
            .ok_or_else(|| format!("Connector {} not found", id))?;

        conn.last_health_check = Some(conn.requests_count + 1);
        conn.requests_count += 1;

        let new_status = if conn.config.api_key.is_some() || conn.config.oauth_config.is_some() {
            ConnectorStatus::Connected
        } else {
            ConnectorStatus::AuthRequired
        };
        conn.status = new_status.clone();
        self.metrics.total_requests += 1;
        self.metrics.connected_count = self.connectors.values()
            .filter(|c| c.status == ConnectorStatus::Connected).count();
        Ok(new_status)
    }

    /// Auto-discover connectors from file contents (simulated).
    pub fn auto_discover(&mut self, files: &HashMap<String, String>) -> Vec<ConnectorType> {
        let mut discovered = Vec::new();

        for (filename, content) in files {
            let lower_name = filename.to_lowercase();
            let lower_content = content.to_lowercase();

            if (lower_name.contains("package.json") || lower_content.contains("stripe"))
                && !self.has_type(&ConnectorType::Stripe) {
                    discovered.push(ConnectorType::Stripe);
                }
            if lower_content.contains("sentry")
                && !self.has_type(&ConnectorType::Sentry) {
                    discovered.push(ConnectorType::Sentry);
                }
            if lower_content.contains("supabase")
                && !self.has_type(&ConnectorType::Supabase) {
                    discovered.push(ConnectorType::Supabase);
                }
            if lower_content.contains("firebase")
                && !self.has_type(&ConnectorType::Firebase) {
                    discovered.push(ConnectorType::Firebase);
                }
            if lower_name.contains(".env") {
                if lower_content.contains("datadog") && !self.has_type(&ConnectorType::Datadog) {
                    discovered.push(ConnectorType::Datadog);
                }
                if lower_content.contains("slack") && !self.has_type(&ConnectorType::Slack) {
                    discovered.push(ConnectorType::Slack);
                }
                if lower_content.contains("github") && !self.has_type(&ConnectorType::GitHub) {
                    discovered.push(ConnectorType::GitHub);
                }
                if lower_content.contains("linear") && !self.has_type(&ConnectorType::Linear) {
                    discovered.push(ConnectorType::Linear);
                }
                if lower_content.contains("vercel") && !self.has_type(&ConnectorType::Vercel) {
                    discovered.push(ConnectorType::Vercel);
                }
                if lower_content.contains("notion") && !self.has_type(&ConnectorType::Notion) {
                    discovered.push(ConnectorType::Notion);
                }
            }
            if lower_name.contains("vercel.json") && !self.has_type(&ConnectorType::Vercel) {
                discovered.push(ConnectorType::Vercel);
            }
            if lower_name.contains("netlify.toml") && !self.has_type(&ConnectorType::Netlify) {
                discovered.push(ConnectorType::Netlify);
            }
            if (lower_name.contains("aws") || lower_content.contains("aws_access_key"))
                && !self.has_type(&ConnectorType::Aws) {
                discovered.push(ConnectorType::Aws);
            }
        }

        self.metrics.auto_discovered += discovered.len() as u64;
        discovered
    }

    fn has_type(&self, ct: &ConnectorType) -> bool {
        self.connectors.values().any(|c| c.config.connector_type == *ct)
    }

    /// List connectors filtered by status.
    pub fn list_by_status(&self, status: &ConnectorStatus) -> Vec<&ConnectorInstance> {
        self.connectors.values()
            .filter(|c| &c.status == status)
            .collect()
    }

    /// Receive a webhook event.
    pub fn webhook_receive(&mut self, event: WebhookEvent) -> Result<(), String> {
        if !self.connectors.contains_key(&event.connector_id) {
            return Err(format!("Unknown connector {}", event.connector_id));
        }
        if let Some(conn) = self.connectors.get_mut(&event.connector_id) {
            conn.requests_count += 1;
        }
        self.webhook_events.push(event);
        Ok(())
    }

    /// Get a connector by ID.
    pub fn get(&self, id: &str) -> Option<&ConnectorInstance> {
        self.connectors.get(id)
    }

    /// List all connector IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> ConnectorRegistry {
        ConnectorRegistry::new()
    }

    fn stripe_config() -> ConnectorConfig {
        ConnectorConfig::new(ConnectorType::Stripe).with_api_key("sk_test_123")
    }

    #[test]
    fn test_connector_type_all() {
        assert_eq!(ConnectorType::all().len(), 20);
    }

    #[test]
    fn test_connector_type_names() {
        assert_eq!(ConnectorType::Stripe.name(), "stripe");
        assert_eq!(ConnectorType::GitHub.name(), "github");
        assert_eq!(ConnectorType::Confluence.name(), "confluence");
    }

    #[test]
    fn test_connector_type_default_urls() {
        assert!(ConnectorType::Stripe.default_base_url().contains("stripe"));
        assert!(ConnectorType::GitHub.default_base_url().contains("github"));
    }

    #[test]
    fn test_connector_config_new() {
        let cfg = ConnectorConfig::new(ConnectorType::Slack);
        assert_eq!(cfg.connector_type, ConnectorType::Slack);
        assert!(cfg.enabled);
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn test_connector_config_with_api_key() {
        let cfg = ConnectorConfig::new(ConnectorType::Stripe).with_api_key("key123");
        assert_eq!(cfg.api_key, Some("key123".to_string()));
    }

    #[test]
    fn test_registry_new() {
        let r = make_registry();
        assert!(r.connectors.is_empty());
        assert!(r.webhook_endpoint.is_none());
    }

    #[test]
    fn test_registry_with_webhook() {
        let r = ConnectorRegistry::new().with_webhook_endpoint("https://hooks.example.com");
        assert_eq!(r.webhook_endpoint, Some("https://hooks.example.com".to_string()));
    }

    #[test]
    fn test_register_connector() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        assert!(id.contains("stripe"));
        assert_eq!(r.connectors.len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let mut r = make_registry();
        r.register(stripe_config()).unwrap();
        assert!(r.register(stripe_config()).is_err());
    }

    #[test]
    fn test_register_connected_with_key() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        assert_eq!(r.connectors[&id].status, ConnectorStatus::Connected);
    }

    #[test]
    fn test_register_auth_required_no_key() {
        let mut r = make_registry();
        let id = r.register(ConnectorConfig::new(ConnectorType::Figma)).unwrap();
        assert_eq!(r.connectors[&id].status, ConnectorStatus::AuthRequired);
    }

    #[test]
    fn test_unregister_connector() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        let inst = r.unregister(&id).unwrap();
        assert_eq!(inst.config.connector_type, ConnectorType::Stripe);
        assert!(r.connectors.is_empty());
    }

    #[test]
    fn test_unregister_not_found() {
        let mut r = make_registry();
        assert!(r.unregister("nonexistent").is_err());
    }

    #[test]
    fn test_health_check() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        let status = r.health_check(&id).unwrap();
        assert_eq!(status, ConnectorStatus::Connected);
        assert_eq!(r.connectors[&id].requests_count, 1);
    }

    #[test]
    fn test_health_check_not_found() {
        let mut r = make_registry();
        assert!(r.health_check("nope").is_err());
    }

    #[test]
    fn test_health_check_auth_required() {
        let mut r = make_registry();
        let id = r.register(ConnectorConfig::new(ConnectorType::Jira)).unwrap();
        let status = r.health_check(&id).unwrap();
        assert_eq!(status, ConnectorStatus::AuthRequired);
    }

    #[test]
    fn test_auto_discover_stripe_from_package_json() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("package.json".to_string(), r#"{"dependencies":{"stripe":"^12.0"}}"#.to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Stripe));
    }

    #[test]
    fn test_auto_discover_sentry() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("src/index.ts".to_string(), "import * as Sentry from '@sentry/node'".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Sentry));
    }

    #[test]
    fn test_auto_discover_env_slack() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "SLACK_TOKEN=xoxb-123".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Slack));
    }

    #[test]
    fn test_auto_discover_env_datadog() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "DATADOG_API_KEY=abc".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Datadog));
    }

    #[test]
    fn test_auto_discover_vercel_json() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("vercel.json".to_string(), "{}".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Vercel));
    }

    #[test]
    fn test_auto_discover_netlify_toml() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("netlify.toml".to_string(), "[build]".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Netlify));
    }

    #[test]
    fn test_auto_discover_supabase() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("lib/db.ts".to_string(), "import { createClient } from '@supabase/supabase-js'".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Supabase));
    }

    #[test]
    fn test_auto_discover_firebase() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("firebase.json".to_string(), "firebase config".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Firebase));
    }

    #[test]
    fn test_auto_discover_no_duplicates() {
        let mut r = make_registry();
        r.register(ConnectorConfig::new(ConnectorType::Sentry).with_api_key("k")).unwrap();
        let mut files = HashMap::new();
        files.insert("app.js".to_string(), "sentry".to_string());
        let found = r.auto_discover(&files);
        assert!(!found.contains(&ConnectorType::Sentry));
    }

    #[test]
    fn test_auto_discover_updates_metrics() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert("netlify.toml".to_string(), "x".to_string());
        r.auto_discover(&files);
        assert!(r.metrics.auto_discovered > 0);
    }

    #[test]
    fn test_auto_discover_aws() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "AWS_ACCESS_KEY_ID=AKIA...".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Aws));
    }

    #[test]
    fn test_auto_discover_github_env() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "GITHUB_TOKEN=ghp_abc".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::GitHub));
    }

    #[test]
    fn test_auto_discover_linear_env() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "LINEAR_API_KEY=lin_123".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Linear));
    }

    #[test]
    fn test_auto_discover_notion_env() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "NOTION_API_KEY=secret_abc".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Notion));
    }

    #[test]
    fn test_list_by_status_connected() {
        let mut r = make_registry();
        r.register(stripe_config()).unwrap();
        r.register(ConnectorConfig::new(ConnectorType::Figma)).unwrap();
        let connected = r.list_by_status(&ConnectorStatus::Connected);
        assert_eq!(connected.len(), 1);
    }

    #[test]
    fn test_list_by_status_auth_required() {
        let mut r = make_registry();
        r.register(ConnectorConfig::new(ConnectorType::Figma)).unwrap();
        r.register(ConnectorConfig::new(ConnectorType::Notion)).unwrap();
        let auth = r.list_by_status(&ConnectorStatus::AuthRequired);
        assert_eq!(auth.len(), 2);
    }

    #[test]
    fn test_webhook_receive() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        let event = WebhookEvent {
            connector_id: id.clone(),
            event_type: "payment.succeeded".to_string(),
            payload: "{}".to_string(),
            timestamp: 100,
        };
        assert!(r.webhook_receive(event).is_ok());
        assert_eq!(r.webhook_events.len(), 1);
    }

    #[test]
    fn test_webhook_receive_unknown_connector() {
        let mut r = make_registry();
        let event = WebhookEvent {
            connector_id: "unknown".to_string(),
            event_type: "test".to_string(),
            payload: "{}".to_string(),
            timestamp: 0,
        };
        assert!(r.webhook_receive(event).is_err());
    }

    #[test]
    fn test_webhook_increments_requests() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        let event = WebhookEvent {
            connector_id: id.clone(),
            event_type: "evt".to_string(),
            payload: "{}".to_string(),
            timestamp: 0,
        };
        r.webhook_receive(event).unwrap();
        assert_eq!(r.connectors[&id].requests_count, 1);
    }

    #[test]
    fn test_get_connector() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        assert!(r.get(&id).is_some());
        assert!(r.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_ids() {
        let mut r = make_registry();
        r.register(stripe_config()).unwrap();
        r.register(ConnectorConfig::new(ConnectorType::Slack).with_api_key("k")).unwrap();
        assert_eq!(r.list_ids().len(), 2);
    }

    #[test]
    fn test_metrics_after_register_unregister() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        assert_eq!(r.metrics.total_connectors, 1);
        assert_eq!(r.metrics.connected_count, 1);
        r.unregister(&id).unwrap();
        assert_eq!(r.metrics.total_connectors, 0);
        assert_eq!(r.metrics.connected_count, 0);
    }

    #[test]
    fn test_metrics_default() {
        let m = ConnectorMetrics::default();
        assert_eq!(m.total_connectors, 0);
        assert_eq!(m.auto_discovered, 0);
    }

    #[test]
    fn test_connector_config_serde() {
        let cfg = stripe_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let de: ConnectorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, de);
    }

    #[test]
    fn test_connector_status_serde() {
        let s = ConnectorStatus::Error("timeout".to_string());
        let json = serde_json::to_string(&s).unwrap();
        let de: ConnectorStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, de);
    }

    #[test]
    fn test_multiple_connectors_same_type_diff_url() {
        let mut r = make_registry();
        r.register(stripe_config()).unwrap();
        let mut cfg2 = stripe_config();
        cfg2.base_url = "https://custom-stripe.example.com".to_string();
        assert!(r.register(cfg2).is_ok());
    }

    #[test]
    fn test_register_with_oauth() {
        let mut r = make_registry();
        let mut cfg = ConnectorConfig::new(ConnectorType::Figma);
        cfg.oauth_config = Some(OAuthConfig {
            client_id: "cid".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "http://localhost:3000".to_string(),
            scopes: vec!["read".to_string()],
            token: Some("tok".to_string()),
            refresh_token: None,
        });
        let id = r.register(cfg).unwrap();
        assert_eq!(r.connectors[&id].status, ConnectorStatus::Connected);
    }

    #[test]
    fn test_custom_headers() {
        let mut cfg = ConnectorConfig::new(ConnectorType::Jira);
        cfg.custom_headers.insert("X-Custom".to_string(), "value".to_string());
        assert_eq!(cfg.custom_headers.len(), 1);
    }

    #[test]
    fn test_health_check_updates_timestamp() {
        let mut r = make_registry();
        let id = r.register(stripe_config()).unwrap();
        assert!(r.connectors[&id].last_health_check.is_none());
        r.health_check(&id).unwrap();
        assert!(r.connectors[&id].last_health_check.is_some());
    }

    #[test]
    fn test_auto_discover_empty_files() {
        let mut r = make_registry();
        let files = HashMap::new();
        let found = r.auto_discover(&files);
        assert!(found.is_empty());
    }

    #[test]
    fn test_auto_discover_vercel_in_env() {
        let mut r = make_registry();
        let mut files = HashMap::new();
        files.insert(".env".to_string(), "VERCEL_TOKEN=abc".to_string());
        let found = r.auto_discover(&files);
        assert!(found.contains(&ConnectorType::Vercel));
    }
}
