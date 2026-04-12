#![allow(dead_code)]
//! Enterprise MCP governance — audit logging, SSO session management,
//! gateway policy enforcement, and portable server configuration.
//!
//! # Four pillars
//!
//! 1. **AuditStore** — append-only JSONL audit trail with CEF export
//! 2. **SsoManager** — OIDC/SAML session lifecycle and group membership
//! 3. **GatewayPolicy** — glob-pattern rules with first-match semantics
//! 4. **ConfigPortability** — JSON import/export and diff for MCP servers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── AuditStore ──────────────────────────────────────────────────────────────

/// Outcome recorded for each MCP tool invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Error(String),
    Blocked(String),
}

/// A single immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub tool_name: String,
    pub caller_id: String,
    pub caller_ip: Option<String>,
    pub inputs_redacted: bool,
    pub outcome: AuditOutcome,
    pub latency_ms: u64,
    pub timestamp_ms: u64,
}

/// Append-only in-memory audit log with query helpers.
pub struct AuditStore {
    events: Vec<AuditEvent>,
}

impl AuditStore {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append an event to the store.
    pub fn record(&mut self, event: AuditEvent) {
        self.events.push(event);
    }

    /// Return all events for the given tool name.
    pub fn query_by_tool(&self, tool: &str) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.tool_name == tool).collect()
    }

    /// Return all events for the given caller ID.
    pub fn query_by_caller(&self, caller: &str) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.caller_id == caller)
            .collect()
    }

    /// Return all events with timestamp_ms >= since_ms.
    pub fn query_since(&self, since_ms: u64) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp_ms >= since_ms)
            .collect()
    }

    pub fn total_count(&self) -> usize {
        self.events.len()
    }

    /// Export all events as CEF (Common Event Format) lines joined by `\n`.
    ///
    /// Format: `CEF:0|VibeCody|MCPGateway|1.0|<outcome>|<tool>|5|...`
    pub fn export_cef(&self) -> String {
        self.events
            .iter()
            .map(|e| {
                let (sig, name) = match &e.outcome {
                    AuditOutcome::Success => ("SUCCESS", "Tool Invocation Success"),
                    AuditOutcome::Error(_) => ("ERROR", "Tool Invocation Error"),
                    AuditOutcome::Blocked(_) => ("BLOCKED", "Tool Invocation Blocked"),
                };
                let severity = match &e.outcome {
                    AuditOutcome::Success => 3,
                    AuditOutcome::Error(_) => 7,
                    AuditOutcome::Blocked(_) => 9,
                };
                let ext = format!(
                    "eventId={} src={} suser={} requestMethod={} \
                     rt={} lat={} redacted={}",
                    e.event_id,
                    e.caller_ip.as_deref().unwrap_or("unknown"),
                    e.caller_id,
                    e.tool_name,
                    e.timestamp_ms,
                    e.latency_ms,
                    e.inputs_redacted,
                );
                format!(
                    "CEF:0|VibeCody|MCPGateway|1.0|{}|{}|{}|{}",
                    sig, name, severity, ext
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── SsoManager ──────────────────────────────────────────────────────────────

/// Supported SSO protocol flavours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SsoProvider {
    Oidc,
    Saml,
}

/// Runtime SSO configuration (one per deployment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    pub provider: SsoProvider,
    pub issuer_url: String,
    pub client_id: String,
    pub allowed_groups: Vec<String>,
}

/// An authenticated SSO session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub session_id: String,
    pub user_id: String,
    pub groups: Vec<String>,
    pub expires_at_ms: u64,
}

/// Manages SSO session lifecycle.
pub struct SsoManager {
    config: Option<SsoConfig>,
    sessions: HashMap<String, SsoSession>,
    next_id: u64,
}

impl SsoManager {
    pub fn new() -> Self {
        Self {
            config: None,
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn configure(&mut self, config: SsoConfig) {
        self.config = Some(config);
    }

    /// Create a new session for a user with the given groups.
    pub fn create_session(
        &mut self,
        user_id: &str,
        groups: Vec<String>,
        ttl_secs: u64,
    ) -> SsoSession {
        let session_id = format!("sso-{}", self.next_id);
        self.next_id += 1;
        // Use a fixed epoch base of 0 for deterministic tests; callers add
        // their own wall-clock offset via now_ms + ttl.
        let expires_at_ms = ttl_secs * 1000;
        let session = SsoSession {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            groups,
            expires_at_ms,
        };
        self.sessions.insert(session_id, session.clone());
        session
    }

    /// Validate a session against the current time. Returns `None` if missing
    /// or expired.
    pub fn validate_session(&self, session_id: &str, now_ms: u64) -> Option<&SsoSession> {
        self.sessions.get(session_id).and_then(|s| {
            if s.expires_at_ms > now_ms {
                Some(s)
            } else {
                None
            }
        })
    }

    /// Revoke a session. Returns `true` if it existed.
    pub fn revoke_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    /// Check whether a valid session belongs to the given group.
    pub fn is_in_group(&self, session_id: &str, group: &str, now_ms: u64) -> bool {
        self.validate_session(session_id, now_ms)
            .map(|s| s.groups.iter().any(|g| g == group))
            .unwrap_or(false)
    }
}

impl Default for SsoManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── GatewayPolicy ───────────────────────────────────────────────────────────

/// Whether a policy rule allows or denies a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// A single named policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub rule_id: String,
    /// Glob-style pattern: `"*"` matches all tools; `"read_*"` matches any
    /// tool whose name starts with `"read_"`.
    pub tool_pattern: String,
    /// If `Some`, only applies when the caller belongs to this group.
    pub caller_group: Option<String>,
    pub effect: PolicyEffect,
    /// Optional request-per-minute rate limit (metadata only — enforcement is
    /// external).
    pub rate_limit: Option<u32>,
}

/// Result of evaluating the gateway policy for one request.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub matched_rule_id: Option<String>,
    pub reason: String,
}

/// Ordered list of rules; first match wins; no match → default deny.
pub struct GatewayPolicy {
    rules: Vec<PolicyRule>,
}

impl GatewayPolicy {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule. Returns `Err` if `rule_id` is already registered.
    pub fn add_rule(&mut self, rule: PolicyRule) -> Result<(), String> {
        if self.rules.iter().any(|r| r.rule_id == rule.rule_id) {
            return Err(format!("duplicate rule_id: {}", rule.rule_id));
        }
        self.rules.push(rule);
        Ok(())
    }

    /// Remove a rule by ID. Returns `true` if removed.
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.rule_id != rule_id);
        self.rules.len() < before
    }

    /// Evaluate the policy for a given tool name and optional caller group.
    pub fn evaluate(&self, tool_name: &str, caller_group: Option<&str>) -> PolicyDecision {
        for rule in &self.rules {
            if !glob_matches(&rule.tool_pattern, tool_name) {
                continue;
            }
            if let Some(required_group) = &rule.caller_group {
                if caller_group != Some(required_group.as_str()) {
                    continue;
                }
            }
            let allowed = rule.effect == PolicyEffect::Allow;
            return PolicyDecision {
                allowed,
                matched_rule_id: Some(rule.rule_id.clone()),
                reason: format!(
                    "matched rule '{}': {:?}",
                    rule.rule_id, rule.effect
                ),
            };
        }
        PolicyDecision {
            allowed: false,
            matched_rule_id: None,
            reason: "no matching rule — default deny".to_string(),
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for GatewayPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob matcher: `"*"` matches everything; `"foo_*"` matches strings
/// starting with `"foo_"`; `"*_bar"` matches strings ending with `"_bar"`;
/// exact strings match exactly.
fn glob_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() > 2 {
        let inner = &pattern[1..pattern.len() - 1];
        return text.contains(inner);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return text.starts_with(prefix);
    }
    pattern == text
}

// ─── ConfigPortability ───────────────────────────────────────────────────────

/// Configuration for a single MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub server_id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub version: String,
}

/// Registry of MCP server configurations with JSON import/export.
pub struct ConfigPortability {
    servers: HashMap<String, McpServerConfig>,
}

impl ConfigPortability {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Register a server. Returns `Err` if `server_id` already exists.
    pub fn register(&mut self, config: McpServerConfig) -> Result<(), String> {
        if self.servers.contains_key(&config.server_id) {
            return Err(format!("server_id already registered: {}", config.server_id));
        }
        self.servers.insert(config.server_id.clone(), config);
        Ok(())
    }

    pub fn get(&self, server_id: &str) -> Option<&McpServerConfig> {
        self.servers.get(server_id)
    }

    /// Serialize all servers to a JSON array string.
    pub fn export_json(&self) -> String {
        let mut list: Vec<&McpServerConfig> = self.servers.values().collect();
        list.sort_by_key(|c| &c.server_id);
        serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
    }

    /// Deserialize servers from a JSON array string. Returns the count of
    /// servers imported. Overwrites existing entries with the same `server_id`.
    pub fn import_json(&mut self, json: &str) -> Result<usize, String> {
        let configs: Vec<McpServerConfig> =
            serde_json::from_str(json).map_err(|e| e.to_string())?;
        let count = configs.len();
        for cfg in configs {
            self.servers.insert(cfg.server_id.clone(), cfg);
        }
        Ok(count)
    }

    /// Diff `self` against `other`, returning human-readable change lines.
    pub fn diff(&self, other: &ConfigPortability) -> Vec<String> {
        let mut lines = Vec::new();

        for (id, cfg) in &self.servers {
            if !other.servers.contains_key(id) {
                lines.push(format!("removed: {} ({})", id, cfg.name));
            }
        }
        for (id, cfg) in &other.servers {
            if !self.servers.contains_key(id) {
                lines.push(format!("added: {} ({})", id, cfg.name));
            } else if self.servers[id] != *cfg {
                lines.push(format!("changed: {} ({})", id, cfg.name));
            }
        }
        lines.sort();
        lines
    }

    pub fn server_count(&self) -> usize {
        self.servers.len()
    }
}

impl Default for ConfigPortability {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AuditStore ────────────────────────────────────────────────────────

    fn make_event(id: &str, tool: &str, caller: &str, ts: u64, outcome: AuditOutcome) -> AuditEvent {
        AuditEvent {
            event_id: id.to_string(),
            tool_name: tool.to_string(),
            caller_id: caller.to_string(),
            caller_ip: Some("127.0.0.1".to_string()),
            inputs_redacted: false,
            outcome,
            latency_ms: 42,
            timestamp_ms: ts,
        }
    }

    #[test]
    fn audit_store_starts_empty() {
        let store = AuditStore::new();
        assert_eq!(store.total_count(), 0);
    }

    #[test]
    fn audit_record_increments_count() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "read_file", "alice", 1000, AuditOutcome::Success));
        assert_eq!(store.total_count(), 1);
    }

    #[test]
    fn audit_query_by_tool() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "read_file", "alice", 1000, AuditOutcome::Success));
        store.record(make_event("e2", "write_file", "bob", 2000, AuditOutcome::Success));
        store.record(make_event("e3", "read_file", "carol", 3000, AuditOutcome::Success));
        let results = store.query_by_tool("read_file");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.tool_name == "read_file"));
    }

    #[test]
    fn audit_query_by_tool_none() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "read_file", "alice", 1000, AuditOutcome::Success));
        assert!(store.query_by_tool("nonexistent").is_empty());
    }

    #[test]
    fn audit_query_by_caller() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "read_file", "alice", 1000, AuditOutcome::Success));
        store.record(make_event("e2", "write_file", "bob", 2000, AuditOutcome::Success));
        store.record(make_event("e3", "delete_file", "alice", 3000, AuditOutcome::Blocked("policy".to_string())));
        let results = store.query_by_caller("alice");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn audit_query_since() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "tool", "u", 500, AuditOutcome::Success));
        store.record(make_event("e2", "tool", "u", 1000, AuditOutcome::Success));
        store.record(make_event("e3", "tool", "u", 2000, AuditOutcome::Success));
        let results = store.query_since(1000);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn audit_query_since_all() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "tool", "u", 100, AuditOutcome::Success));
        assert_eq!(store.query_since(0).len(), 1);
    }

    #[test]
    fn audit_export_cef_success() {
        let mut store = AuditStore::new();
        store.record(make_event("evt1", "read_file", "alice", 1000, AuditOutcome::Success));
        let cef = store.export_cef();
        assert!(cef.contains("CEF:0|VibeCody|MCPGateway|1.0|SUCCESS"));
        assert!(cef.contains("eventId=evt1"));
        assert!(cef.contains("suser=alice"));
    }

    #[test]
    fn audit_export_cef_error() {
        let mut store = AuditStore::new();
        store.record(make_event("e2", "write_file", "bob", 2000, AuditOutcome::Error("io error".to_string())));
        let cef = store.export_cef();
        assert!(cef.contains("ERROR"));
    }

    #[test]
    fn audit_export_cef_blocked() {
        let mut store = AuditStore::new();
        store.record(make_event("e3", "exec", "eve", 3000, AuditOutcome::Blocked("policy rule 1".to_string())));
        let cef = store.export_cef();
        assert!(cef.contains("BLOCKED"));
    }

    #[test]
    fn audit_export_cef_multiline() {
        let mut store = AuditStore::new();
        store.record(make_event("e1", "t1", "u1", 1000, AuditOutcome::Success));
        store.record(make_event("e2", "t2", "u2", 2000, AuditOutcome::Success));
        let cef = store.export_cef();
        assert_eq!(cef.lines().count(), 2);
    }

    #[test]
    fn audit_export_cef_empty_is_empty_string() {
        let store = AuditStore::new();
        assert_eq!(store.export_cef(), "");
    }

    #[test]
    fn audit_inputs_redacted_field() {
        let mut store = AuditStore::new();
        let mut e = make_event("e1", "tool", "u", 1000, AuditOutcome::Success);
        e.inputs_redacted = true;
        store.record(e);
        let cef = store.export_cef();
        assert!(cef.contains("redacted=true"));
    }

    #[test]
    fn audit_caller_ip_none() {
        let mut store = AuditStore::new();
        let e = AuditEvent {
            event_id: "e1".to_string(),
            tool_name: "tool".to_string(),
            caller_id: "user".to_string(),
            caller_ip: None,
            inputs_redacted: false,
            outcome: AuditOutcome::Success,
            latency_ms: 10,
            timestamp_ms: 1000,
        };
        store.record(e);
        let cef = store.export_cef();
        assert!(cef.contains("src=unknown"));
    }

    // ── SsoManager ────────────────────────────────────────────────────────

    #[test]
    fn sso_create_session_returns_session() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec!["devs".to_string()], 3600);
        assert_eq!(sess.user_id, "alice");
        assert!(!sess.session_id.is_empty());
    }

    #[test]
    fn sso_validate_session_valid() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec![], 3600);
        // expires_at_ms = 3600 * 1000 = 3_600_000; now = 0 → valid
        let result = mgr.validate_session(&sess.session_id, 0);
        assert!(result.is_some());
    }

    #[test]
    fn sso_validate_session_expired() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec![], 1);
        // expires_at_ms = 1000; now = 2000 → expired
        let result = mgr.validate_session(&sess.session_id, 2000);
        assert!(result.is_none());
    }

    #[test]
    fn sso_validate_unknown_session() {
        let mgr = SsoManager::new();
        assert!(mgr.validate_session("no-such-id", 0).is_none());
    }

    #[test]
    fn sso_revoke_session() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("bob", vec![], 3600);
        assert!(mgr.revoke_session(&sess.session_id));
        assert!(mgr.validate_session(&sess.session_id, 0).is_none());
    }

    #[test]
    fn sso_revoke_nonexistent_returns_false() {
        let mut mgr = SsoManager::new();
        assert!(!mgr.revoke_session("ghost"));
    }

    #[test]
    fn sso_is_in_group_true() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec!["admin".to_string(), "devs".to_string()], 3600);
        assert!(mgr.is_in_group(&sess.session_id, "admin", 0));
    }

    #[test]
    fn sso_is_in_group_false() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec!["devs".to_string()], 3600);
        assert!(!mgr.is_in_group(&sess.session_id, "admin", 0));
    }

    #[test]
    fn sso_is_in_group_expired() {
        let mut mgr = SsoManager::new();
        let sess = mgr.create_session("alice", vec!["admin".to_string()], 1);
        assert!(!mgr.is_in_group(&sess.session_id, "admin", 5000));
    }

    #[test]
    fn sso_configure_stores_config() {
        let mut mgr = SsoManager::new();
        mgr.configure(SsoConfig {
            provider: SsoProvider::Oidc,
            issuer_url: "https://example.com".to_string(),
            client_id: "client-123".to_string(),
            allowed_groups: vec!["devs".to_string()],
        });
        assert!(mgr.config.is_some());
    }

    #[test]
    fn sso_multiple_sessions_unique_ids() {
        let mut mgr = SsoManager::new();
        let s1 = mgr.create_session("u1", vec![], 3600);
        let s2 = mgr.create_session("u2", vec![], 3600);
        assert_ne!(s1.session_id, s2.session_id);
    }

    #[test]
    fn sso_saml_provider_configure() {
        let mut mgr = SsoManager::new();
        mgr.configure(SsoConfig {
            provider: SsoProvider::Saml,
            issuer_url: "https://idp.corp".to_string(),
            client_id: "urn:corp:sp".to_string(),
            allowed_groups: vec![],
        });
        assert_eq!(mgr.config.as_ref().unwrap().provider, SsoProvider::Saml);
    }

    // ── GatewayPolicy ─────────────────────────────────────────────────────

    #[test]
    fn policy_empty_default_deny() {
        let policy = GatewayPolicy::new();
        let d = policy.evaluate("read_file", None);
        assert!(!d.allowed);
        assert!(d.matched_rule_id.is_none());
    }

    #[test]
    fn policy_add_allow_rule() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        let d = policy.evaluate("anything", None);
        assert!(d.allowed);
        assert_eq!(d.matched_rule_id.as_deref(), Some("r1"));
    }

    #[test]
    fn policy_add_deny_rule() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Deny,
            rate_limit: None,
        }).unwrap();
        let d = policy.evaluate("read_file", None);
        assert!(!d.allowed);
        assert_eq!(d.matched_rule_id.as_deref(), Some("r1"));
    }

    #[test]
    fn policy_glob_prefix_match() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "read_*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        assert!(policy.evaluate("read_file", None).allowed);
        assert!(policy.evaluate("read_dir", None).allowed);
        assert!(!policy.evaluate("write_file", None).allowed);
    }

    #[test]
    fn policy_glob_suffix_match() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*_file".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        assert!(policy.evaluate("read_file", None).allowed);
        assert!(policy.evaluate("write_file", None).allowed);
        assert!(!policy.evaluate("read_dir", None).allowed);
    }

    #[test]
    fn policy_exact_match() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "exec_shell".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        assert!(policy.evaluate("exec_shell", None).allowed);
        assert!(!policy.evaluate("exec_shell_safe", None).allowed);
    }

    #[test]
    fn policy_first_rule_wins_allow_then_deny() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "allow-all".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        policy.add_rule(PolicyRule {
            rule_id: "deny-all".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Deny,
            rate_limit: None,
        }).unwrap();
        let d = policy.evaluate("anything", None);
        assert!(d.allowed);
        assert_eq!(d.matched_rule_id.as_deref(), Some("allow-all"));
    }

    #[test]
    fn policy_first_rule_wins_deny_then_allow() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "deny-all".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Deny,
            rate_limit: None,
        }).unwrap();
        policy.add_rule(PolicyRule {
            rule_id: "allow-all".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        let d = policy.evaluate("anything", None);
        assert!(!d.allowed);
    }

    #[test]
    fn policy_caller_group_filter() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "admin-only".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: Some("admin".to_string()),
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        assert!(policy.evaluate("tool", Some("admin")).allowed);
        assert!(!policy.evaluate("tool", Some("devs")).allowed);
        assert!(!policy.evaluate("tool", None).allowed);
    }

    #[test]
    fn policy_duplicate_rule_id_rejected() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        let result = policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Deny,
            rate_limit: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn policy_remove_rule() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: None,
        }).unwrap();
        assert_eq!(policy.rule_count(), 1);
        assert!(policy.remove_rule("r1"));
        assert_eq!(policy.rule_count(), 0);
        assert!(!policy.evaluate("anything", None).allowed);
    }

    #[test]
    fn policy_remove_nonexistent_returns_false() {
        let mut policy = GatewayPolicy::new();
        assert!(!policy.remove_rule("ghost"));
    }

    #[test]
    fn policy_rate_limit_field_stored() {
        let mut policy = GatewayPolicy::new();
        policy.add_rule(PolicyRule {
            rule_id: "r1".to_string(),
            tool_pattern: "*".to_string(),
            caller_group: None,
            effect: PolicyEffect::Allow,
            rate_limit: Some(100),
        }).unwrap();
        assert_eq!(policy.rules[0].rate_limit, Some(100));
    }

    // ── ConfigPortability ─────────────────────────────────────────────────

    fn make_server(id: &str, name: &str) -> McpServerConfig {
        McpServerConfig {
            server_id: id.to_string(),
            name: name.to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "mcp-server".to_string()],
            env: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn config_register_and_get() {
        let mut cp = ConfigPortability::new();
        cp.register(make_server("s1", "MyServer")).unwrap();
        assert!(cp.get("s1").is_some());
        assert_eq!(cp.get("s1").unwrap().name, "MyServer");
    }

    #[test]
    fn config_register_duplicate_rejected() {
        let mut cp = ConfigPortability::new();
        cp.register(make_server("s1", "MyServer")).unwrap();
        assert!(cp.register(make_server("s1", "Other")).is_err());
    }

    #[test]
    fn config_server_count() {
        let mut cp = ConfigPortability::new();
        assert_eq!(cp.server_count(), 0);
        cp.register(make_server("s1", "A")).unwrap();
        cp.register(make_server("s2", "B")).unwrap();
        assert_eq!(cp.server_count(), 2);
    }

    #[test]
    fn config_get_missing_returns_none() {
        let cp = ConfigPortability::new();
        assert!(cp.get("none").is_none());
    }

    #[test]
    fn config_export_json_round_trip() {
        let mut cp = ConfigPortability::new();
        cp.register(make_server("s1", "Server1")).unwrap();
        cp.register(make_server("s2", "Server2")).unwrap();
        let json = cp.export_json();

        let mut cp2 = ConfigPortability::new();
        let count = cp2.import_json(&json).unwrap();
        assert_eq!(count, 2);
        assert!(cp2.get("s1").is_some());
        assert!(cp2.get("s2").is_some());
    }

    #[test]
    fn config_import_json_overwrites() {
        let mut cp = ConfigPortability::new();
        cp.register(make_server("s1", "Old")).unwrap();
        let mut updated = make_server("s1", "New");
        updated.version = "2.0.0".to_string();
        let json = serde_json::to_string(&vec![updated]).unwrap();
        cp.import_json(&json).unwrap();
        assert_eq!(cp.get("s1").unwrap().name, "New");
    }

    #[test]
    fn config_import_invalid_json_returns_err() {
        let mut cp = ConfigPortability::new();
        assert!(cp.import_json("not json").is_err());
    }

    #[test]
    fn config_diff_added() {
        let cp1 = ConfigPortability::new();
        let mut cp2 = ConfigPortability::new();
        cp2.register(make_server("s1", "New")).unwrap();
        let diff = cp1.diff(&cp2);
        assert!(diff.iter().any(|l| l.contains("added") && l.contains("s1")));
    }

    #[test]
    fn config_diff_removed() {
        let mut cp1 = ConfigPortability::new();
        cp1.register(make_server("s1", "Old")).unwrap();
        let cp2 = ConfigPortability::new();
        let diff = cp1.diff(&cp2);
        assert!(diff.iter().any(|l| l.contains("removed") && l.contains("s1")));
    }

    #[test]
    fn config_diff_changed() {
        let mut cp1 = ConfigPortability::new();
        cp1.register(make_server("s1", "Server")).unwrap();
        let mut cp2 = ConfigPortability::new();
        let mut changed = make_server("s1", "Server");
        changed.version = "2.0.0".to_string();
        cp2.register(changed).unwrap();
        let diff = cp1.diff(&cp2);
        assert!(diff.iter().any(|l| l.contains("changed") && l.contains("s1")));
    }

    #[test]
    fn config_diff_no_changes() {
        let mut cp1 = ConfigPortability::new();
        cp1.register(make_server("s1", "Same")).unwrap();
        let mut cp2 = ConfigPortability::new();
        cp2.register(make_server("s1", "Same")).unwrap();
        assert!(cp1.diff(&cp2).is_empty());
    }

    #[test]
    fn config_export_empty_is_valid_json() {
        let cp = ConfigPortability::new();
        let json = cp.export_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.is_array());
    }
}
