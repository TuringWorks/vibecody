#![allow(dead_code)]
//! Agent-OS registry — discovery and capability advertisement for the agent pool.
//! Matches Devin 2.0's agent registry system.
//!
//! Agents self-register with their capabilities; the registry brokers capability
//! queries and maintains health state. Integrates with `agent_recruiter` for
//! dynamic pool management.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Unique agent identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn generate() -> Self {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros()).unwrap_or(0);
        Self(format!("agent-{:x}", ts))
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

/// Named capability an agent can advertise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability(pub String);

impl Capability {
    pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
    // Common standard capabilities
    pub const CODE_EDIT: &'static str = "code_edit";
    pub const CODE_REVIEW: &'static str = "code_review";
    pub const FILE_READ: &'static str = "file_read";
    pub const SHELL_EXEC: &'static str = "shell_exec";
    pub const WEB_SEARCH: &'static str = "web_search";
    pub const DATABASE: &'static str = "database";
    pub const DEPLOY: &'static str = "deploy";
    pub const TEST_RUN: &'static str = "test_run";
    pub const GIT_OPS: &'static str = "git_ops";
}

/// Current health state of a registered agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

impl std::fmt::Display for AgentHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentHealth::Healthy => write!(f, "healthy"),
            AgentHealth::Degraded { reason } => write!(f, "degraded: {}", reason),
            AgentHealth::Unhealthy { reason } => write!(f, "unhealthy: {}", reason),
            AgentHealth::Unknown => write!(f, "unknown"),
        }
    }
}

/// An agent's registration entry in the registry.
#[derive(Debug, Clone)]
pub struct AgentRegistration {
    pub id: AgentId,
    pub name: String,
    pub version: String,
    pub capabilities: HashSet<Capability>,
    pub metadata: HashMap<String, String>,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
    pub health: AgentHealth,
    pub load: f32, // 0.0 = idle, 1.0 = fully loaded
    pub max_concurrent_tasks: usize,
    pub current_task_count: usize,
}

impl AgentRegistration {
    pub fn is_available(&self) -> bool {
        matches!(self.health, AgentHealth::Healthy)
            && self.current_task_count < self.max_concurrent_tasks
    }

    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.contains(&Capability::new(cap))
    }

    pub fn capacity_ratio(&self) -> f32 {
        if self.max_concurrent_tasks == 0 { return 1.0; }
        self.current_task_count as f32 / self.max_concurrent_tasks as f32
    }

    pub fn is_stale(&self, timeout_ms: u64) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        now.saturating_sub(self.last_heartbeat_ms) > timeout_ms
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Central agent registry: register, query, heartbeat, deregister.
pub struct AgentRegistry {
    agents: HashMap<AgentId, AgentRegistration>,
    /// Milliseconds before a missing heartbeat marks agent as stale.
    pub heartbeat_timeout_ms: u64,
}

impl Default for AgentRegistry {
    fn default() -> Self { Self::new() }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            heartbeat_timeout_ms: 30_000,
        }
    }

    /// Register a new agent. Returns Err if ID already registered.
    pub fn register(&mut self, reg: AgentRegistration) -> Result<(), String> {
        if self.agents.contains_key(&reg.id) {
            return Err(format!("Agent {} already registered", reg.id));
        }
        self.agents.insert(reg.id.clone(), reg);
        Ok(())
    }

    /// Re-register (upsert) — updates an existing registration.
    pub fn upsert(&mut self, reg: AgentRegistration) {
        self.agents.insert(reg.id.clone(), reg);
    }

    /// Deregister an agent.
    pub fn deregister(&mut self, id: &AgentId) -> Option<AgentRegistration> {
        self.agents.remove(id)
    }

    /// Record a heartbeat for an agent.
    pub fn heartbeat(&mut self, id: &AgentId, health: AgentHealth, load: f32, task_count: usize) -> Result<(), String> {
        let agent = self.agents.get_mut(id)
            .ok_or_else(|| format!("Agent {} not found", id))?;
        agent.last_heartbeat_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        agent.health = health;
        agent.load = load.clamp(0.0, 1.0);
        agent.current_task_count = task_count;
        Ok(())
    }

    pub fn get(&self, id: &AgentId) -> Option<&AgentRegistration> {
        self.agents.get(id)
    }

    /// All registered agents.
    pub fn all(&self) -> Vec<&AgentRegistration> {
        self.agents.values().collect()
    }

    /// Agents that have a specific capability and are available.
    pub fn find_capable(&self, capability: &str) -> Vec<&AgentRegistration> {
        self.agents.values()
            .filter(|a| a.has_capability(capability) && a.is_available())
            .collect()
    }

    /// Agents matching ALL of the required capabilities.
    pub fn find_all_capable(&self, capabilities: &[&str]) -> Vec<&AgentRegistration> {
        self.agents.values()
            .filter(|a| {
                a.is_available() && capabilities.iter().all(|c| a.has_capability(c))
            })
            .collect()
    }

    /// Least-loaded available agent with the required capability.
    pub fn least_loaded(&self, capability: &str) -> Option<&AgentRegistration> {
        self.find_capable(capability)
            .into_iter()
            .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Mark stale agents as unhealthy.
    pub fn reap_stale(&mut self) {
        let timeout = self.heartbeat_timeout_ms;
        for agent in self.agents.values_mut() {
            if agent.is_stale(timeout) {
                agent.health = AgentHealth::Unhealthy { reason: "heartbeat timeout".into() };
            }
        }
    }

    /// Remove all unhealthy agents.
    pub fn prune_unhealthy(&mut self) -> Vec<AgentId> {
        let to_remove: Vec<AgentId> = self.agents.values()
            .filter(|a| matches!(a.health, AgentHealth::Unhealthy { .. }))
            .map(|a| a.id.clone())
            .collect();
        for id in &to_remove {
            self.agents.remove(id);
        }
        to_remove
    }

    pub fn total_count(&self) -> usize { self.agents.len() }
    pub fn healthy_count(&self) -> usize { self.agents.values().filter(|a| a.health == AgentHealth::Healthy).count() }
    pub fn available_count(&self) -> usize { self.agents.values().filter(|a| a.is_available()).count() }

    /// Registry status summary.
    pub fn status_summary(&self) -> RegistryStatus {
        let total = self.total_count();
        let healthy = self.healthy_count();
        let available = self.available_count();
        let total_capacity: usize = self.agents.values().map(|a| a.max_concurrent_tasks).sum();
        let used_capacity: usize = self.agents.values().map(|a| a.current_task_count).sum();
        RegistryStatus { total, healthy, available, total_capacity, used_capacity }
    }
}

#[derive(Debug)]
pub struct RegistryStatus {
    pub total: usize,
    pub healthy: usize,
    pub available: usize,
    pub total_capacity: usize,
    pub used_capacity: usize,
}

impl RegistryStatus {
    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 { return 0.0; }
        self.used_capacity as f64 / self.total_capacity as f64
    }
}

// ---------------------------------------------------------------------------
// Builder helper
// ---------------------------------------------------------------------------

pub struct AgentRegistrationBuilder {
    id: AgentId,
    name: String,
    version: String,
    capabilities: HashSet<Capability>,
    max_concurrent_tasks: usize,
    metadata: HashMap<String, String>,
}

impl AgentRegistrationBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(id),
            name: name.into(),
            version: "1.0.0".into(),
            capabilities: HashSet::new(),
            max_concurrent_tasks: 4,
            metadata: HashMap::new(),
        }
    }

    pub fn version(mut self, v: impl Into<String>) -> Self { self.version = v.into(); self }
    pub fn capability(mut self, cap: impl Into<String>) -> Self { self.capabilities.insert(Capability::new(cap)); self }
    pub fn max_tasks(mut self, n: usize) -> Self { self.max_concurrent_tasks = n; self }
    pub fn meta(mut self, k: impl Into<String>, v: impl Into<String>) -> Self { self.metadata.insert(k.into(), v.into()); self }

    pub fn build(self) -> AgentRegistration {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        AgentRegistration {
            id: self.id,
            name: self.name,
            version: self.version,
            capabilities: self.capabilities,
            metadata: self.metadata,
            registered_at_ms: now,
            last_heartbeat_ms: now,
            health: AgentHealth::Healthy,
            load: 0.0,
            max_concurrent_tasks: self.max_concurrent_tasks,
            current_task_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(id: &str, caps: &[&str]) -> AgentRegistration {
        let mut b = AgentRegistrationBuilder::new(id, id);
        for cap in caps { b = b.capability(*cap); }
        b.max_tasks(4).build()
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[Capability::CODE_EDIT])).unwrap();
        assert!(reg.get(&AgentId::new("a1")).is_some());
    }

    #[test]
    fn test_register_duplicate_fails() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[])).unwrap();
        assert!(reg.register(make_agent("a1", &[])).is_err());
    }

    #[test]
    fn test_deregister() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[])).unwrap();
        reg.deregister(&AgentId::new("a1"));
        assert!(reg.get(&AgentId::new("a1")).is_none());
    }

    #[test]
    fn test_find_capable() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[Capability::CODE_EDIT])).unwrap();
        reg.register(make_agent("a2", &[Capability::WEB_SEARCH])).unwrap();
        let found = reg.find_capable(Capability::CODE_EDIT);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id.0, "a1");
    }

    #[test]
    fn test_find_all_capable() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[Capability::CODE_EDIT, Capability::GIT_OPS])).unwrap();
        reg.register(make_agent("a2", &[Capability::CODE_EDIT])).unwrap();
        let found = reg.find_all_capable(&[Capability::CODE_EDIT, Capability::GIT_OPS]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id.0, "a1");
    }

    #[test]
    fn test_least_loaded() {
        let mut reg = AgentRegistry::new();
        let mut a1 = make_agent("a1", &[Capability::CODE_EDIT]);
        a1.load = 0.8;
        let mut a2 = make_agent("a2", &[Capability::CODE_EDIT]);
        a2.load = 0.2;
        reg.upsert(a1);
        reg.upsert(a2);
        let least = reg.least_loaded(Capability::CODE_EDIT).unwrap();
        assert_eq!(least.id.0, "a2");
    }

    #[test]
    fn test_heartbeat_updates_health() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[])).unwrap();
        reg.heartbeat(&AgentId::new("a1"), AgentHealth::Degraded { reason: "high load".into() }, 0.9, 3).unwrap();
        let a = reg.get(&AgentId::new("a1")).unwrap();
        assert!(matches!(a.health, AgentHealth::Degraded { .. }));
        assert_eq!(a.load, 0.9);
    }

    #[test]
    fn test_prune_unhealthy() {
        let mut reg = AgentRegistry::new();
        let mut a1 = make_agent("a1", &[]);
        a1.health = AgentHealth::Unhealthy { reason: "crash".into() };
        reg.upsert(a1);
        reg.register(make_agent("a2", &[])).unwrap();
        let pruned = reg.prune_unhealthy();
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].0, "a1");
        assert_eq!(reg.total_count(), 1);
    }

    #[test]
    fn test_unavailable_when_at_capacity() {
        let mut a = make_agent("a1", &[Capability::CODE_EDIT]);
        a.max_concurrent_tasks = 2;
        a.current_task_count = 2;
        assert!(!a.is_available());
    }

    #[test]
    fn test_registry_status() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[])).unwrap();
        reg.register(make_agent("a2", &[])).unwrap();
        let status = reg.status_summary();
        assert_eq!(status.total, 2);
        assert_eq!(status.healthy, 2);
    }

    #[test]
    fn test_utilization() {
        let mut status = RegistryStatus { total: 1, healthy: 1, available: 1, total_capacity: 4, used_capacity: 2 };
        assert!((status.utilization() - 0.5).abs() < 1e-9);
        status.total_capacity = 0;
        assert_eq!(status.utilization(), 0.0);
    }

    #[test]
    fn test_capacity_ratio() {
        let mut a = make_agent("a", &[]);
        a.max_concurrent_tasks = 4;
        a.current_task_count = 1;
        assert!((a.capacity_ratio() - 0.25).abs() < 1e-5);
    }

    #[test]
    fn test_upsert_overwrites() {
        let mut reg = AgentRegistry::new();
        reg.register(make_agent("a1", &[])).unwrap();
        let mut updated = make_agent("a1", &[Capability::DEPLOY]);
        updated.load = 0.5;
        reg.upsert(updated);
        assert_eq!(reg.total_count(), 1);
        assert!(reg.get(&AgentId::new("a1")).unwrap().has_capability(Capability::DEPLOY));
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new("my-agent");
        assert_eq!(id.to_string(), "my-agent");
    }

    #[test]
    fn test_builder_sets_fields() {
        let reg = AgentRegistrationBuilder::new("x", "X Agent")
            .version("2.0.0")
            .capability(Capability::TEST_RUN)
            .max_tasks(8)
            .meta("region", "us-west")
            .build();
        assert_eq!(reg.version, "2.0.0");
        assert!(reg.has_capability(Capability::TEST_RUN));
        assert_eq!(reg.max_concurrent_tasks, 8);
        assert_eq!(reg.metadata.get("region").unwrap(), "us-west");
    }
}
