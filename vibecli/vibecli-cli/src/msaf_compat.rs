#![allow(dead_code)]
//! Microsoft Agent Framework 1.0 compatibility layer.
//!
//! Provides manifest generation, Azure AD token validation, task lifecycle
//! management, and an agent catalog with heartbeat-based liveness tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Capabilities that can be declared in an MSAF agent manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentCapabilityMsaf {
    CodeGeneration,
    CodeReview,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    Deployment,
    DataAnalysis,
}

/// Lifecycle state of an MSAF task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MsafTaskState {
    Queued,
    Running,
    Completed(String),
    Failed(String),
}

// ─── MsafManifest ────────────────────────────────────────────────────────────

/// Declarative description of an MSAF-compatible agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsafManifest {
    pub agent_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<AgentCapabilityMsaf>,
    /// JSON Schema stub for input validation.
    pub input_schema: String,
    /// JSON Schema stub for output validation.
    pub output_schema: String,
    pub auth_required: bool,
    pub health_endpoint: String,
}

// ─── MsafTask ────────────────────────────────────────────────────────────────

/// A single unit of work submitted to an MSAF agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsafTask {
    pub task_id: String,
    pub task_type: String,
    pub payload: String,
    pub state: MsafTaskState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

// ─── AzureAdClaims / TokenValidator ──────────────────────────────────────────

/// Decoded Azure AD JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureAdClaims {
    /// Subject (user or app OID string).
    pub sub: String,
    /// Object ID.
    pub oid: String,
    /// Group memberships.
    pub groups: Vec<String>,
    /// Expiry in milliseconds since Unix epoch.
    pub exp_ms: u64,
}

/// Validates test tokens of the form `"valid:<sub>:<oid>:<groups>:<exp_ms>"`.
pub struct TokenValidator;

impl TokenValidator {
    pub fn new() -> Self {
        Self
    }

    /// Parse and validate a token.
    ///
    /// Valid tokens follow the scheme `"valid:<sub>:<oid>:<g1,g2,...>:<exp_ms>"`.
    /// All other tokens return `Err`.
    pub fn validate_token(&self, token: &str, now_ms: u64) -> Result<AzureAdClaims, String> {
        let parts: Vec<&str> = token.splitn(5, ':').collect();
        if parts.len() != 5 || parts[0] != "valid" {
            return Err(format!("invalid token format: {}", token));
        }
        let sub = parts[1].to_string();
        let oid = parts[2].to_string();
        let groups: Vec<String> = if parts[3].is_empty() {
            vec![]
        } else {
            parts[3].split(',').map(|g| g.to_string()).collect()
        };
        let exp_ms: u64 = parts[4]
            .parse()
            .map_err(|_| "invalid exp_ms".to_string())?;

        let claims = AzureAdClaims { sub, oid, groups, exp_ms };
        if self.is_expired(&claims, now_ms) {
            return Err("token expired".to_string());
        }
        Ok(claims)
    }

    pub fn is_expired(&self, claims: &AzureAdClaims, now_ms: u64) -> bool {
        claims.exp_ms <= now_ms
    }
}

impl Default for TokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── MsafAgent ───────────────────────────────────────────────────────────────

/// An agent instance that manages its own task queue.
pub struct MsafAgent {
    manifest: MsafManifest,
    tasks: HashMap<String, MsafTask>,
    next_task_id: u64,
}

impl MsafAgent {
    pub fn new(manifest: MsafManifest) -> Self {
        Self {
            manifest,
            tasks: HashMap::new(),
            next_task_id: 1,
        }
    }

    pub fn manifest(&self) -> &MsafManifest {
        &self.manifest
    }

    /// Submit a new task. Returns the generated task ID.
    pub fn submit_task(&mut self, task_type: &str, payload: &str) -> String {
        let task_id = format!("task-{}", self.next_task_id);
        self.next_task_id += 1;
        let task = MsafTask {
            task_id: task_id.clone(),
            task_type: task_type.to_string(),
            payload: payload.to_string(),
            state: MsafTaskState::Queued,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        self.tasks.insert(task_id.clone(), task);
        task_id
    }

    /// Update the state of an existing task.
    pub fn update_task_state(
        &mut self,
        task_id: &str,
        state: MsafTaskState,
    ) -> Result<(), String> {
        self.tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("task not found: {}", task_id))
            .map(|t| t.state = state)
    }

    pub fn get_task(&self, task_id: &str) -> Option<&MsafTask> {
        self.tasks.get(task_id)
    }

    pub fn running_tasks(&self) -> Vec<&MsafTask> {
        self.tasks
            .values()
            .filter(|t| t.state == MsafTaskState::Running)
            .collect()
    }

    pub fn completed_tasks(&self) -> Vec<&MsafTask> {
        self.tasks
            .values()
            .filter(|t| matches!(t.state, MsafTaskState::Completed(_)))
            .collect()
    }

    /// Returns `true` when the manifest has a non-empty agent_id and name.
    pub fn health_check(&self) -> bool {
        !self.manifest.agent_id.is_empty() && !self.manifest.name.is_empty()
    }
}

// ─── AgentCatalog ────────────────────────────────────────────────────────────

/// A registration entry in the agent catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub agent_id: String,
    pub endpoint_url: String,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
}

/// Directory of registered MSAF agents with heartbeat-based liveness.
pub struct AgentCatalog {
    entries: HashMap<String, CatalogEntry>,
}

impl AgentCatalog {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a new agent. Returns `Err` if the agent_id already exists.
    pub fn register(&mut self, entry: CatalogEntry) -> Result<(), String> {
        if self.entries.contains_key(&entry.agent_id) {
            return Err(format!("agent already registered: {}", entry.agent_id));
        }
        self.entries.insert(entry.agent_id.clone(), entry);
        Ok(())
    }

    /// Update the last-heartbeat timestamp for an existing agent.
    pub fn heartbeat(&mut self, agent_id: &str, now_ms: u64) -> Result<(), String> {
        self.entries
            .get_mut(agent_id)
            .ok_or_else(|| format!("agent not found: {}", agent_id))
            .map(|e| e.last_heartbeat_ms = now_ms)
    }

    /// Remove an agent from the catalog. Returns `true` if it existed.
    pub fn deregister(&mut self, agent_id: &str) -> bool {
        self.entries.remove(agent_id).is_some()
    }

    /// Return agents whose last heartbeat is within `ttl_secs` seconds of
    /// `now_ms`.
    pub fn active_agents(&self, now_ms: u64, ttl_secs: u64) -> Vec<&CatalogEntry> {
        let threshold_ms = ttl_secs * 1000;
        self.entries
            .values()
            .filter(|e| {
                now_ms.saturating_sub(e.last_heartbeat_ms) <= threshold_ms
            })
            .collect()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for AgentCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_manifest(id: &str) -> MsafManifest {
        MsafManifest {
            agent_id: id.to_string(),
            name: format!("Agent-{}", id),
            version: "1.0.0".to_string(),
            description: "Test agent".to_string(),
            capabilities: vec![AgentCapabilityMsaf::CodeGeneration],
            input_schema: r#"{"type":"object"}"#.to_string(),
            output_schema: r#"{"type":"object"}"#.to_string(),
            auth_required: true,
            health_endpoint: "/health".to_string(),
        }
    }

    fn make_entry(id: &str, hb: u64) -> CatalogEntry {
        CatalogEntry {
            agent_id: id.to_string(),
            endpoint_url: format!("https://agents.example/{}", id),
            registered_at_ms: 0,
            last_heartbeat_ms: hb,
        }
    }

    // ── MsafManifest ─────────────────────────────────────────────────────────

    #[test]
    fn manifest_fields_stored() {
        let m = make_manifest("agent-1");
        assert_eq!(m.agent_id, "agent-1");
        assert_eq!(m.version, "1.0.0");
        assert!(m.auth_required);
    }

    #[test]
    fn manifest_capabilities_list() {
        let mut m = make_manifest("a1");
        m.capabilities = vec![
            AgentCapabilityMsaf::CodeGeneration,
            AgentCapabilityMsaf::Testing,
        ];
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn manifest_serializes_to_json() {
        let m = make_manifest("a1");
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("a1"));
        assert!(json.contains("Agent-a1"));
    }

    // ── TokenValidator ────────────────────────────────────────────────────────

    #[test]
    fn token_valid_parses_correctly() {
        let tv = TokenValidator::new();
        let token = "valid:alice:oid-123:admin,devs:999999999999";
        let claims = tv.validate_token(token, 0).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.oid, "oid-123");
        assert!(claims.groups.contains(&"admin".to_string()));
        assert!(claims.groups.contains(&"devs".to_string()));
    }

    #[test]
    fn token_malformed_returns_err() {
        let tv = TokenValidator::new();
        assert!(tv.validate_token("garbage", 0).is_err());
    }

    #[test]
    fn token_wrong_prefix_returns_err() {
        let tv = TokenValidator::new();
        assert!(tv.validate_token("invalid:sub:oid:g:9999", 0).is_err());
    }

    #[test]
    fn token_expired_returns_err() {
        let tv = TokenValidator::new();
        // exp_ms = 1000, now_ms = 2000 → expired
        let token = "valid:bob:oid-bob:devs:1000";
        assert!(tv.validate_token(token, 2000).is_err());
    }

    #[test]
    fn token_not_yet_expired() {
        let tv = TokenValidator::new();
        let token = "valid:carol:oid-carol::99999999999";
        let claims = tv.validate_token(token, 0).unwrap();
        assert_eq!(claims.sub, "carol");
        assert!(claims.groups.is_empty());
    }

    #[test]
    fn token_is_expired_helper() {
        let tv = TokenValidator::new();
        let claims = AzureAdClaims {
            sub: "u".to_string(),
            oid: "o".to_string(),
            groups: vec![],
            exp_ms: 5000,
        };
        assert!(tv.is_expired(&claims, 5001));
        assert!(!tv.is_expired(&claims, 4999));
    }

    #[test]
    fn token_exact_expiry_is_expired() {
        let tv = TokenValidator::new();
        let claims = AzureAdClaims {
            sub: "u".to_string(),
            oid: "o".to_string(),
            groups: vec![],
            exp_ms: 1000,
        };
        assert!(tv.is_expired(&claims, 1000));
    }

    #[test]
    fn token_invalid_exp_ms_format() {
        let tv = TokenValidator::new();
        assert!(tv.validate_token("valid:sub:oid:g:notanumber", 0).is_err());
    }

    // ── MsafAgent task lifecycle ──────────────────────────────────────────────

    #[test]
    fn agent_submit_task_returns_id() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id = agent.submit_task("code_gen", "{}");
        assert!(!id.is_empty());
    }

    #[test]
    fn agent_get_task_after_submit() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id = agent.submit_task("review", "payload");
        let task = agent.get_task(&id).unwrap();
        assert_eq!(task.task_type, "review");
        assert_eq!(task.state, MsafTaskState::Queued);
    }

    #[test]
    fn agent_update_task_state_to_running() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id = agent.submit_task("test", "{}");
        agent.update_task_state(&id, MsafTaskState::Running).unwrap();
        assert_eq!(agent.get_task(&id).unwrap().state, MsafTaskState::Running);
    }

    #[test]
    fn agent_update_task_state_to_completed() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id = agent.submit_task("deploy", "{}");
        agent.update_task_state(&id, MsafTaskState::Completed("ok".to_string())).unwrap();
        assert!(matches!(agent.get_task(&id).unwrap().state, MsafTaskState::Completed(_)));
    }

    #[test]
    fn agent_update_task_state_to_failed() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id = agent.submit_task("debug", "{}");
        agent.update_task_state(&id, MsafTaskState::Failed("timeout".to_string())).unwrap();
        assert!(matches!(agent.get_task(&id).unwrap().state, MsafTaskState::Failed(_)));
    }

    #[test]
    fn agent_update_missing_task_returns_err() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        assert!(agent.update_task_state("no-such-task", MsafTaskState::Running).is_err());
    }

    #[test]
    fn agent_running_tasks_filter() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id1 = agent.submit_task("t1", "{}");
        let id2 = agent.submit_task("t2", "{}");
        agent.update_task_state(&id1, MsafTaskState::Running).unwrap();
        let running = agent.running_tasks();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].task_id, id1);
        let _ = id2;
    }

    #[test]
    fn agent_completed_tasks_filter() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id1 = agent.submit_task("t1", "{}");
        let _id2 = agent.submit_task("t2", "{}");
        agent.update_task_state(&id1, MsafTaskState::Completed("done".to_string())).unwrap();
        let completed = agent.completed_tasks();
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn agent_multiple_tasks_unique_ids() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        let id1 = agent.submit_task("t1", "{}");
        let id2 = agent.submit_task("t2", "{}");
        assert_ne!(id1, id2);
    }

    #[test]
    fn agent_health_check_valid_manifest() {
        let agent = MsafAgent::new(make_manifest("a1"));
        assert!(agent.health_check());
    }

    #[test]
    fn agent_health_check_empty_id_fails() {
        let mut m = make_manifest("");
        m.agent_id = String::new();
        let agent = MsafAgent::new(m);
        assert!(!agent.health_check());
    }

    #[test]
    fn agent_manifest_accessor() {
        let agent = MsafAgent::new(make_manifest("a1"));
        assert_eq!(agent.manifest().agent_id, "a1");
    }

    #[test]
    fn agent_get_missing_task_returns_none() {
        let agent = MsafAgent::new(make_manifest("a1"));
        assert!(agent.get_task("ghost").is_none());
    }

    #[test]
    fn agent_no_running_when_all_queued() {
        let mut agent = MsafAgent::new(make_manifest("a1"));
        agent.submit_task("t1", "{}");
        assert!(agent.running_tasks().is_empty());
    }

    // ── AgentCatalog ─────────────────────────────────────────────────────────

    #[test]
    fn catalog_register_and_count() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 1000)).unwrap();
        assert_eq!(catalog.entry_count(), 1);
    }

    #[test]
    fn catalog_register_duplicate_rejected() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 1000)).unwrap();
        assert!(catalog.register(make_entry("a1", 2000)).is_err());
    }

    #[test]
    fn catalog_heartbeat_updates_timestamp() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 1000)).unwrap();
        catalog.heartbeat("a1", 5000).unwrap();
        let entry = catalog.entries.get("a1").unwrap();
        assert_eq!(entry.last_heartbeat_ms, 5000);
    }

    #[test]
    fn catalog_heartbeat_missing_agent_err() {
        let mut catalog = AgentCatalog::new();
        assert!(catalog.heartbeat("ghost", 1000).is_err());
    }

    #[test]
    fn catalog_deregister_existing() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 1000)).unwrap();
        assert!(catalog.deregister("a1"));
        assert_eq!(catalog.entry_count(), 0);
    }

    #[test]
    fn catalog_deregister_nonexistent_returns_false() {
        let mut catalog = AgentCatalog::new();
        assert!(!catalog.deregister("ghost"));
    }

    #[test]
    fn catalog_active_agents_within_ttl() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 9000)).unwrap();
        // now = 10000, ttl = 5s → threshold = 5000ms; 10000 - 9000 = 1000 ≤ 5000 → active
        let active = catalog.active_agents(10000, 5);
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn catalog_active_agents_expired() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 1000)).unwrap();
        // now = 10000, ttl = 2s → threshold = 2000; 10000 - 1000 = 9000 > 2000 → inactive
        let active = catalog.active_agents(10000, 2);
        assert!(active.is_empty());
    }

    #[test]
    fn catalog_active_agents_mixed() {
        let mut catalog = AgentCatalog::new();
        catalog.register(make_entry("a1", 9500)).unwrap();
        catalog.register(make_entry("a2", 1000)).unwrap();
        // now = 10000, ttl = 1s → threshold = 1000; a1: 500 ≤ 1000 active, a2: 9000 > 1000 inactive
        let active = catalog.active_agents(10000, 1);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "a1");
    }

    #[test]
    fn catalog_empty_active_agents() {
        let catalog = AgentCatalog::new();
        assert!(catalog.active_agents(9999, 60).is_empty());
    }

    #[test]
    fn catalog_entry_count_zero_initially() {
        let catalog = AgentCatalog::new();
        assert_eq!(catalog.entry_count(), 0);
    }
}
