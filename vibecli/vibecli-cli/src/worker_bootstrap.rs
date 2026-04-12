//! Worker agent bootstrap and capability negotiation.
//!
//! Claw-code parity Wave 2: initialises sub-agent workers with environment
//! context, capability declarations, and task-specific configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Capability ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    ReadFiles, WriteFiles, RunBash, BrowseWeb,
    CallTools, SpawnAgents, AccessDatabase, ReadGit,
    Custom(String),
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFiles => write!(f, "read_files"),
            Self::WriteFiles => write!(f, "write_files"),
            Self::RunBash => write!(f, "run_bash"),
            Self::BrowseWeb => write!(f, "browse_web"),
            Self::CallTools => write!(f, "call_tools"),
            Self::SpawnAgents => write!(f, "spawn_agents"),
            Self::AccessDatabase => write!(f, "access_database"),
            Self::ReadGit => write!(f, "read_git"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

// ─── Bootstrap Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub worker_id: String,
    pub parent_session_id: String,
    pub capabilities: Vec<Capability>,
    pub env: HashMap<String, String>,
    pub working_directory: String,
    pub model_id: String,
    pub token_budget: u64,
    pub timeout_ms: u64,
    pub tags: Vec<String>,
}

impl BootstrapConfig {
    pub fn new(worker_id: impl Into<String>, parent: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(), parent_session_id: parent.into(),
            capabilities: Vec::new(), env: HashMap::new(),
            working_directory: ".".into(), model_id: "claude-sonnet-4-6".into(),
            token_budget: 50_000, timeout_ms: 300_000, tags: Vec::new(),
        }
    }

    pub fn with_capability(mut self, c: Capability) -> Self { self.capabilities.push(c); self }
    pub fn with_env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self { self.env.insert(k.into(), v.into()); self }
    pub fn with_model(mut self, m: impl Into<String>) -> Self { self.model_id = m.into(); self }
    pub fn with_budget(mut self, b: u64) -> Self { self.token_budget = b; self }
    pub fn with_timeout(mut self, ms: u64) -> Self { self.timeout_ms = ms; self }
    pub fn with_tag(mut self, t: impl Into<String>) -> Self { self.tags.push(t.into()); self }
    pub fn in_dir(mut self, d: impl Into<String>) -> Self { self.working_directory = d.into(); self }

    pub fn has_capability(&self, c: &Capability) -> bool { self.capabilities.contains(c) }
}

// ─── Bootstrap Result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BootstrapStatus { Ready, CapabilityDenied { missing: Vec<Capability> }, ConfigError { reason: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    pub worker_id: String,
    pub status: BootstrapStatus,
    pub effective_capabilities: Vec<Capability>,
    pub effective_token_budget: u64,
}

// ─── Worker Registry ─────────────────────────────────────────────────────────

pub struct WorkerBootstrap {
    /// Allowed capabilities globally (parent may further restrict).
    pub allowed_capabilities: Vec<Capability>,
    /// Max token budget a worker may receive.
    pub max_token_budget: u64,
    workers: HashMap<String, BootstrapResult>,
}

impl WorkerBootstrap {
    pub fn new(allowed: Vec<Capability>, max_budget: u64) -> Self {
        Self { allowed_capabilities: allowed, max_token_budget: max_budget, workers: HashMap::new() }
    }

    pub fn bootstrap(&mut self, config: BootstrapConfig) -> BootstrapResult {
        // Validate capabilities
        let missing: Vec<Capability> = config.capabilities.iter()
            .filter(|c| !self.allowed_capabilities.contains(c))
            .cloned().collect();

        let budget = config.token_budget.min(self.max_token_budget);

        let status = if !missing.is_empty() {
            BootstrapStatus::CapabilityDenied { missing: missing.clone() }
        } else {
            BootstrapStatus::Ready
        };

        let effective_capabilities: Vec<Capability> = if missing.is_empty() {
            config.capabilities.clone()
        } else {
            config.capabilities.into_iter().filter(|c| self.allowed_capabilities.contains(c)).collect()
        };

        let result = BootstrapResult {
            worker_id: config.worker_id.clone(),
            status, effective_capabilities, effective_token_budget: budget,
        };
        self.workers.insert(config.worker_id, result.clone());
        result
    }

    pub fn worker_result(&self, worker_id: &str) -> Option<&BootstrapResult> { self.workers.get(worker_id) }
    pub fn active_workers(&self) -> usize { self.workers.len() }
    pub fn deregister(&mut self, worker_id: &str) -> bool { self.workers.remove(worker_id).is_some() }
    pub fn workers_by_tag<'a>(&'a self, _tag: &str, configs: &'a [BootstrapConfig]) -> Vec<&'a str> {
        configs.iter().filter(|c| self.workers.contains_key(&c.worker_id)).map(|c| c.worker_id.as_str()).collect()
    }
}

// ── WorkerState ───────────────────────────────────────────────────────────────

/// Six-state lifecycle for spawned subprocess agent workers.
///
/// # State machine
/// ```text
/// Spawning → TrustRequired → ReadyForPrompt → Running → Finished
///                                                     ↘ Failed
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerState {
    Spawning,
    TrustRequired,
    ReadyForPrompt,
    Running,
    Finished,
    Failed,
}

impl std::fmt::Display for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawning       => write!(f, "spawning"),
            Self::TrustRequired  => write!(f, "trust_required"),
            Self::ReadyForPrompt => write!(f, "ready_for_prompt"),
            Self::Running        => write!(f, "running"),
            Self::Finished       => write!(f, "finished"),
            Self::Failed         => write!(f, "failed"),
        }
    }
}

impl WorkerState {
    /// Returns the valid successor states for this state.
    pub fn valid_transitions(&self) -> &'static [WorkerState] {
        match self {
            Self::Spawning       => &[WorkerState::TrustRequired, WorkerState::ReadyForPrompt, WorkerState::Failed],
            Self::TrustRequired  => &[WorkerState::ReadyForPrompt, WorkerState::Failed],
            Self::ReadyForPrompt => &[WorkerState::Running, WorkerState::Failed],
            Self::Running        => &[WorkerState::Finished, WorkerState::Failed],
            Self::Finished | Self::Failed => &[],
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Finished | Self::Failed)
    }
}

// ── Prompt detection constants ────────────────────────────────────────────────

/// Valid agent readiness prompts (>, ›, ❯).
pub const VALID_PROMPTS: &[char] = &['>', '\u{203a}', '\u{276f}'];
/// Shell-only prompts that are false positives ($, %, #).
pub const SHELL_ONLY_PROMPTS: &[char] = &['$', '%', '#'];

// ── WorkerLifecycle ───────────────────────────────────────────────────────────

/// Manages the six-state readiness handshake for a single spawned agent worker.
///
/// [`WorkerBootstrap`] (above) handles the capability-negotiation registry;
/// `WorkerLifecycle` manages the per-worker state machine, prompt detection,
/// delivery verification, and auto-recovery.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerLifecycle {
    pub id: String,
    pub state: WorkerState,
    pub task: String,
    pub cwd: String,
    pub retries: u32,
    pub max_retries: u32,
}

impl WorkerLifecycle {
    pub fn new(id: &str, task: &str, cwd: &str) -> Self {
        Self {
            id: id.to_string(),
            state: WorkerState::Spawning,
            task: task.to_string(),
            cwd: cwd.to_string(),
            retries: 0,
            max_retries: 3,
        }
    }

    /// Transition to a new state. Returns `Err` if the transition is invalid.
    pub fn transition(&mut self, new_state: WorkerState) -> Result<(), String> {
        if self.state.is_terminal() {
            return Err(format!("Cannot transition from terminal state {}", self.state));
        }
        if !self.state.valid_transitions().contains(&new_state) {
            return Err(format!("Invalid transition: {} → {}", self.state, new_state));
        }
        self.state = new_state;
        Ok(())
    }

    /// Is `line` a valid agent readiness prompt? (>, ›, ❯) — NOT a bare shell prompt.
    pub fn is_valid_prompt(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() { return false; }
        let last = trimmed.chars().last().unwrap_or(' ');
        if !VALID_PROMPTS.contains(&last) { return false; }
        // Reject bare single shell-only prompt chars
        if trimmed.len() == 1 && SHELL_ONLY_PROMPTS.contains(&trimmed.chars().next().unwrap()) {
            return false;
        }
        true
    }

    /// Is `line` a shell-only false-positive prompt? ($, %, #)
    pub fn is_shell_only_prompt(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.len() <= 2 && trimmed.chars().any(|c| SHELL_ONLY_PROMPTS.contains(&c))
    }

    /// Scan output for agent readiness signals; reject shell-only false positives.
    pub fn detect_readiness(output: &str) -> bool {
        for line in output.lines() {
            let trimmed = line.trim();
            if Self::is_shell_only_prompt(trimmed) { continue; }
            if Self::is_valid_prompt(trimmed) { return true; }
        }
        false
    }

    /// Validate that the worker's actual CWD matches the expected CWD.
    pub fn verify_delivery(actual_cwd: &str, expected_cwd: &str) -> bool {
        actual_cwd.trim_end_matches('/') == expected_cwd.trim_end_matches('/')
    }

    /// Increment retry count and return the new state (Failed if at max retries).
    pub fn attempt_recovery(&mut self) -> WorkerState {
        self.retries += 1;
        if self.retries >= self.max_retries {
            self.state = WorkerState::Failed;
            WorkerState::Failed
        } else {
            self.state = WorkerState::Spawning;
            WorkerState::Spawning
        }
    }

    /// Persist lifecycle state to `<state_dir>/worker-state.json`.
    pub fn persist_state(&self, state_dir: &std::path::Path) -> Result<(), String> {
        let path = state_dir.join("worker-state.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize error: {e}"))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("write error: {e}"))
    }

    /// Load lifecycle state from a JSON file.
    pub fn load_state(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("parse error: {e}"))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn initial_state_is_spawning() {
        let w = WorkerLifecycle::new("w1", "fix tests", "/project");
        assert_eq!(w.state, WorkerState::Spawning);
    }

    #[test]
    fn valid_transition_spawning_to_ready_for_prompt() {
        let mut w = WorkerLifecycle::new("w1", "task", "/");
        assert!(w.transition(WorkerState::ReadyForPrompt).is_ok());
        assert_eq!(w.state, WorkerState::ReadyForPrompt);
    }

    #[test]
    fn valid_transition_ready_to_running() {
        let mut w = WorkerLifecycle::new("w1", "task", "/");
        w.transition(WorkerState::ReadyForPrompt).unwrap();
        assert!(w.transition(WorkerState::Running).is_ok());
    }

    #[test]
    fn valid_transition_running_to_finished() {
        let mut w = WorkerLifecycle::new("w1", "task", "/");
        w.transition(WorkerState::ReadyForPrompt).unwrap();
        w.transition(WorkerState::Running).unwrap();
        assert!(w.transition(WorkerState::Finished).is_ok());
    }

    #[test]
    fn invalid_transition_from_terminal_state_fails() {
        let mut w = WorkerLifecycle::new("w1", "task", "/");
        w.transition(WorkerState::ReadyForPrompt).unwrap();
        w.transition(WorkerState::Running).unwrap();
        w.transition(WorkerState::Finished).unwrap();
        assert!(w.transition(WorkerState::Running).is_err());
    }

    #[test]
    fn is_valid_prompt_detects_arrow() {
        assert!(WorkerLifecycle::is_valid_prompt(">"));
        assert!(WorkerLifecycle::is_valid_prompt("vibecli >"));
    }

    #[test]
    fn is_valid_prompt_detects_unicode_arrow() {
        assert!(WorkerLifecycle::is_valid_prompt("❯"));
        assert!(WorkerLifecycle::is_valid_prompt("agent ❯"));
    }

    #[test]
    fn shell_prompt_rejected_as_false_positive() {
        assert!(!WorkerLifecycle::detect_readiness("$ "));
        assert!(!WorkerLifecycle::detect_readiness("% "));
        assert!(!WorkerLifecycle::detect_readiness("# "));
    }

    #[test]
    fn verify_delivery_with_matching_cwd() {
        assert!(WorkerLifecycle::verify_delivery("/project", "/project"));
        assert!(WorkerLifecycle::verify_delivery("/project/", "/project"));
    }

    #[test]
    fn attempt_recovery_increments_retries() {
        let mut w = WorkerLifecycle::new("w1", "task", "/");
        assert_eq!(w.retries, 0);
        let new_state = w.attempt_recovery();
        assert_eq!(w.retries, 1);
        assert_eq!(new_state, WorkerState::Spawning);
    }
}

// ─── Original Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn wb() -> WorkerBootstrap {
        WorkerBootstrap::new(vec![Capability::ReadFiles, Capability::WriteFiles, Capability::RunBash], 100_000)
    }

    fn cfg(id: &str) -> BootstrapConfig {
        BootstrapConfig::new(id, "parent-1").with_capability(Capability::ReadFiles)
    }

    #[test]
    fn test_bootstrap_ready() {
        let mut wb = wb();
        let r = wb.bootstrap(cfg("w1"));
        assert_eq!(r.status, BootstrapStatus::Ready);
    }

    #[test]
    fn test_bootstrap_capability_denied() {
        let mut wb = wb();
        let config = BootstrapConfig::new("w1", "p").with_capability(Capability::SpawnAgents);
        let r = wb.bootstrap(config);
        assert!(matches!(r.status, BootstrapStatus::CapabilityDenied { .. }));
    }

    #[test]
    fn test_bootstrap_budget_capped() {
        let mut wb = wb();
        let config = BootstrapConfig::new("w1", "p").with_budget(999_999);
        let r = wb.bootstrap(config);
        assert_eq!(r.effective_token_budget, 100_000);
    }

    #[test]
    fn test_bootstrap_budget_within_limit() {
        let mut wb = wb();
        let config = BootstrapConfig::new("w1", "p").with_budget(50_000);
        let r = wb.bootstrap(config);
        assert_eq!(r.effective_token_budget, 50_000);
    }

    #[test]
    fn test_worker_registered_after_bootstrap() {
        let mut wb = wb();
        wb.bootstrap(cfg("w1"));
        assert!(wb.worker_result("w1").is_some());
    }

    #[test]
    fn test_deregister_worker() {
        let mut wb = wb();
        wb.bootstrap(cfg("w1"));
        assert!(wb.deregister("w1"));
        assert!(wb.worker_result("w1").is_none());
    }

    #[test]
    fn test_active_workers_count() {
        let mut wb = wb();
        wb.bootstrap(cfg("w1"));
        wb.bootstrap(cfg("w2"));
        assert_eq!(wb.active_workers(), 2);
    }

    #[test]
    fn test_effective_capabilities_subset_on_partial_deny() {
        let mut wb = WorkerBootstrap::new(vec![Capability::ReadFiles], 100_000);
        let config = BootstrapConfig::new("w1", "p")
            .with_capability(Capability::ReadFiles)
            .with_capability(Capability::RunBash); // not allowed
        let r = wb.bootstrap(config);
        assert!(r.effective_capabilities.contains(&Capability::ReadFiles));
        assert!(!r.effective_capabilities.contains(&Capability::RunBash));
    }

    #[test]
    fn test_capability_display() {
        assert_eq!(Capability::ReadFiles.to_string(), "read_files");
        assert_eq!(Capability::Custom("foo".into()).to_string(), "foo");
    }

    #[test]
    fn test_config_has_capability() {
        let c = cfg("w1");
        assert!(c.has_capability(&Capability::ReadFiles));
        assert!(!c.has_capability(&Capability::RunBash));
    }

    #[test]
    fn test_config_with_env() {
        let c = BootstrapConfig::new("w1", "p").with_env("MY_KEY", "value");
        assert_eq!(c.env.get("MY_KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_config_with_model() {
        let c = BootstrapConfig::new("w1", "p").with_model("claude-opus-4-6");
        assert_eq!(c.model_id, "claude-opus-4-6");
    }

    #[test]
    fn test_config_with_tags() {
        let c = BootstrapConfig::new("w1", "p").with_tag("review").with_tag("frontend");
        assert_eq!(c.tags, vec!["review", "frontend"]);
    }

    #[test]
    fn test_unknown_worker_result_none() {
        let wb = wb();
        assert!(wb.worker_result("unknown").is_none());
    }
}
