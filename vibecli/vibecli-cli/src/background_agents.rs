//! Ambient / background agent definitions.
//!
//! Background agents are defined as TOML files in `.vibecli/agents/`:
//!
//! ```toml
//! # .vibecli/agents/test-runner.toml
//! name = "test-runner"
//! background = true
//! trigger = "on_demand"     # "on_demand" | "file_saved" | "scheduled"
//! trigger_paths = ["**/*.rs"]
//! task = "Run cargo test and report failures"
//! approval_policy = "full-auto"
//! max_steps = 10
//! ```
//!
//! REPL commands: `/agents list|start|status|stop`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// ── AgentDef ──────────────────────────────────────────────────────────────────

/// Definition of a background agent (loaded from .vibecli/agents/<name>.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    /// Whether to run in the background (non-blocking).
    #[serde(default = "default_true")]
    pub background: bool,
    /// Trigger type: "on_demand" | "file_saved" | "scheduled"
    #[serde(default = "default_on_demand")]
    pub trigger: String,
    /// Glob patterns for "file_saved" trigger.
    #[serde(default)]
    pub trigger_paths: Vec<String>,
    /// The task description passed to the agent.
    pub task: String,
    /// Approval policy: "suggest" | "auto-edit" | "full-auto"
    #[serde(default = "default_full_auto")]
    pub approval_policy: String,
    /// Maximum agent steps before stopping.
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
    /// Provider to use (defaults to current REPL provider).
    #[serde(default)]
    pub provider: Option<String>,
    /// Model to use (defaults to current REPL model).
    #[serde(default)]
    pub model: Option<String>,
}

fn default_true() -> bool { true }
fn default_on_demand() -> String { "on_demand".to_string() }
fn default_full_auto() -> String { "full-auto".to_string() }
fn default_max_steps() -> u32 { 20 }

// ── AgentRunStatus ────────────────────────────────────────────────────────────

/// Runtime status of a background agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentRunStatus {
    Running,
    Complete,
    Failed,
    Cancelled,
}

impl std::fmt::Display for AgentRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Runtime state for a launched background agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: String,
    pub name: String,
    pub task: String,
    pub status: AgentRunStatus,
    pub started_at: u64,    // unix millis
    pub finished_at: Option<u64>,
    pub summary: Option<String>,
}

impl AgentRun {
    pub fn new(id: impl Into<String>, name: impl Into<String>, task: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            id: id.into(),
            name: name.into(),
            task: task.into(),
            status: AgentRunStatus::Running,
            started_at: now,
            finished_at: None,
            summary: None,
        }
    }

    pub fn finish(&mut self, status: AgentRunStatus, summary: Option<String>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.status = status;
        self.finished_at = Some(now);
        self.summary = summary;
    }
}

// ── BackgroundAgentManager ────────────────────────────────────────────────────

/// Manages background agent definitions and running sessions.
pub struct BackgroundAgentManager {
    agents_dir: PathBuf,
    runs: Arc<Mutex<HashMap<String, AgentRun>>>,
}

impl BackgroundAgentManager {
    /// Create a manager rooted at `workspace_root/.vibecli/agents/`.
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            agents_dir: workspace_root.join(".vibecli").join("agents"),
            runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn new(agents_dir: PathBuf) -> Self {
        Self { agents_dir, runs: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Ensure the agents directory exists.
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.agents_dir)?;
        Ok(())
    }

    /// List all defined agent names.
    pub fn list_defs(&self) -> Vec<String> {
        if !self.agents_dir.is_dir() { return vec![]; }
        let mut names: Vec<String> = std::fs::read_dir(&self.agents_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("toml"))
            .filter_map(|e| {
                e.path().file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        names.sort();
        names
    }

    /// Load an agent definition by name.
    pub fn load_def(&self, name: &str) -> Result<AgentDef> {
        let path = self.agents_dir.join(format!("{}.toml", name));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read agent '{}': {}", name, e))?;
        let def: AgentDef = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("Invalid agent '{}': {}", name, e))?;
        Ok(def)
    }

    /// Save an agent definition.
    pub fn save_def(&self, def: &AgentDef) -> Result<()> {
        std::fs::create_dir_all(&self.agents_dir)?;
        let path = self.agents_dir.join(format!("{}.toml", def.name));
        let content = toml::to_string_pretty(def)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Generate a short hex ID based on current time.
    fn short_id() -> String {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{:x}", millis & 0xFFFFFF)
    }

    /// Register a new agent run (returns a run ID).
    pub fn start_run(&self, def: &AgentDef) -> AgentRun {
        let id = format!("{}-{}", def.name, Self::short_id());
        let run = AgentRun::new(&id, &def.name, &def.task);
        self.runs.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), run.clone());
        run
    }

    /// Update the status of a run.
    pub fn finish_run(&self, id: &str, status: AgentRunStatus, summary: Option<String>) {
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(run) = runs.get_mut(id) {
            run.finish(status, summary);
        }
    }

    /// Cancel a running agent.
    pub fn cancel_run(&self, id: &str) {
        self.finish_run(id, AgentRunStatus::Cancelled, None);
    }

    /// List all runs (sorted newest first).
    pub fn list_runs(&self) -> Vec<AgentRun> {
        let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let mut list: Vec<AgentRun> = runs.values().cloned().collect();
        list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        list
    }

    /// Get a specific run by ID.
    pub fn get_run(&self, id: &str) -> Option<AgentRun> {
        self.runs.lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned()
    }

    /// Create a starter template in the agents directory.
    pub fn create_template(&self, name: &str, task: &str) -> Result<AgentDef> {
        let def = AgentDef {
            name: name.to_string(),
            background: true,
            trigger: "on_demand".to_string(),
            trigger_paths: vec![],
            task: task.to_string(),
            approval_policy: "full-auto".to_string(),
            max_steps: 20,
            provider: None,
            model: None,
        };
        self.save_def(&def)?;
        Ok(def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_and_load_agent_def() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());

        let def = mgr.create_template("test-runner", "Run tests").unwrap();
        assert_eq!(def.name, "test-runner");

        let loaded = mgr.load_def("test-runner").unwrap();
        assert_eq!(loaded.task, "Run tests");
        assert_eq!(loaded.approval_policy, "full-auto");
    }

    #[test]
    fn list_defs() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        mgr.create_template("alpha", "A").unwrap();
        mgr.create_template("beta", "B").unwrap();
        let names = mgr.list_defs();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn run_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        let def = mgr.create_template("ci", "Build").unwrap();

        let run = mgr.start_run(&def);
        assert_eq!(run.status, AgentRunStatus::Running);

        mgr.finish_run(&run.id, AgentRunStatus::Complete, Some("Build passed".to_string()));
        let updated = mgr.get_run(&run.id).unwrap();
        assert_eq!(updated.status, AgentRunStatus::Complete);
        assert_eq!(updated.summary.as_deref(), Some("Build passed"));
    }

    // ── cancel_run ─────────────────────────────────────────────────────────

    #[test]
    fn cancel_run_sets_cancelled() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        let def = mgr.create_template("runner", "Run").unwrap();
        let run = mgr.start_run(&def);

        mgr.cancel_run(&run.id);
        let updated = mgr.get_run(&run.id).unwrap();
        assert_eq!(updated.status, AgentRunStatus::Cancelled);
        assert!(updated.finished_at.is_some());
    }

    // ── AgentRunStatus Display ─────────────────────────────────────────────

    #[test]
    fn agent_run_status_display() {
        assert_eq!(format!("{}", AgentRunStatus::Running), "running");
        assert_eq!(format!("{}", AgentRunStatus::Complete), "complete");
        assert_eq!(format!("{}", AgentRunStatus::Failed), "failed");
        assert_eq!(format!("{}", AgentRunStatus::Cancelled), "cancelled");
    }

    // ── AgentRunStatus serde ───────────────────────────────────────────────

    #[test]
    fn agent_run_status_serde_roundtrip() {
        for status in [AgentRunStatus::Running, AgentRunStatus::Complete, AgentRunStatus::Failed, AgentRunStatus::Cancelled] {
            let json = serde_json::to_string(&status).unwrap();
            let back: AgentRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    // ── AgentDef serde ─────────────────────────────────────────────────────

    #[test]
    fn agent_def_serde_roundtrip() {
        let def = AgentDef {
            name: "test".to_string(),
            background: true,
            trigger: "on_demand".to_string(),
            trigger_paths: vec!["**/*.rs".to_string()],
            task: "Run tests".to_string(),
            approval_policy: "full-auto".to_string(),
            max_steps: 20,
            provider: Some("ollama".to_string()),
            model: None,
        };
        let toml_str = toml::to_string(&def).unwrap();
        let back: AgentDef = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.trigger_paths, vec!["**/*.rs"]);
        assert_eq!(back.provider, Some("ollama".to_string()));
    }

    // ── AgentRun ───────────────────────────────────────────────────────────

    #[test]
    fn agent_run_new_sets_running() {
        let run = AgentRun::new("id-1", "runner", "do stuff");
        assert_eq!(run.id, "id-1");
        assert_eq!(run.name, "runner");
        assert_eq!(run.status, AgentRunStatus::Running);
        assert!(run.finished_at.is_none());
        assert!(run.summary.is_none());
    }

    #[test]
    fn agent_run_finish_sets_fields() {
        let mut run = AgentRun::new("id-2", "worker", "task");
        run.finish(AgentRunStatus::Failed, Some("error msg".to_string()));
        assert_eq!(run.status, AgentRunStatus::Failed);
        assert!(run.finished_at.is_some());
        assert_eq!(run.summary.as_deref(), Some("error msg"));
    }

    // ── BackgroundAgentManager::init ───────────────────────────────────────

    #[test]
    fn init_creates_agents_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("subdir").join("agents");
        let mgr = BackgroundAgentManager::new(dir.clone());
        mgr.init().unwrap();
        assert!(dir.is_dir());
    }

    // ── list_runs / get_run ────────────────────────────────────────────────

    #[test]
    fn list_runs_sorted_newest_first() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        let d1 = mgr.create_template("a", "A").unwrap();
        let d2 = mgr.create_template("b", "B").unwrap();
        let _r1 = mgr.start_run(&d1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let _r2 = mgr.start_run(&d2);

        let runs = mgr.list_runs();
        assert_eq!(runs.len(), 2);
        assert!(runs[0].started_at >= runs[1].started_at);
    }

    #[test]
    fn get_run_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        assert!(mgr.get_run("nonexistent-id").is_none());
    }

    // ── load_def nonexistent ───────────────────────────────────────────────

    #[test]
    fn load_def_nonexistent_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        let result = mgr.load_def("nonexistent");
        assert!(result.is_err());
    }

    // ── list_defs on empty dir ─────────────────────────────────────────────

    #[test]
    fn list_defs_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        // Point to a dir that doesn't exist yet
        let mgr = BackgroundAgentManager::new(tmp.path().join("no_such_dir"));
        assert!(mgr.list_defs().is_empty());
    }

    // ── for_workspace ──────────────────────────────────────────────────────

    #[test]
    fn for_workspace_sets_correct_path() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::for_workspace(tmp.path());
        // Init should create the .vibecli/agents dir
        mgr.init().unwrap();
        assert!(tmp.path().join(".vibecli").join("agents").is_dir());
    }

    // ── AgentDef TOML defaults ──────────────────────────────────────────────

    #[test]
    fn agent_def_toml_defaults_applied() {
        let toml_str = r#"
            name = "minimal"
            task = "Do something"
        "#;
        let def: AgentDef = toml::from_str(toml_str).unwrap();
        assert_eq!(def.name, "minimal");
        assert!(def.background); // default_true
        assert_eq!(def.trigger, "on_demand"); // default_on_demand
        assert!(def.trigger_paths.is_empty()); // serde default
        assert_eq!(def.approval_policy, "full-auto"); // default_full_auto
        assert_eq!(def.max_steps, 20); // default_max_steps
        assert!(def.provider.is_none());
        assert!(def.model.is_none());
    }

    #[test]
    fn agent_def_toml_overrides_defaults() {
        let toml_str = r#"
            name = "watcher"
            background = false
            trigger = "file_saved"
            trigger_paths = ["**/*.py", "**/*.rs"]
            task = "Lint on save"
            approval_policy = "suggest"
            max_steps = 5
            provider = "openai"
            model = "gpt-4"
        "#;
        let def: AgentDef = toml::from_str(toml_str).unwrap();
        assert!(!def.background);
        assert_eq!(def.trigger, "file_saved");
        assert_eq!(def.trigger_paths, vec!["**/*.py", "**/*.rs"]);
        assert_eq!(def.approval_policy, "suggest");
        assert_eq!(def.max_steps, 5);
        assert_eq!(def.provider.as_deref(), Some("openai"));
        assert_eq!(def.model.as_deref(), Some("gpt-4"));
    }

    // ── AgentRun JSON serialization ─────────────────────────────────────────

    #[test]
    fn agent_run_json_roundtrip() {
        let mut run = AgentRun::new("run-1", "agent-a", "do work");
        run.finish(AgentRunStatus::Complete, Some("done".to_string()));
        let json = serde_json::to_string(&run).unwrap();
        let back: AgentRun = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "run-1");
        assert_eq!(back.name, "agent-a");
        assert_eq!(back.status, AgentRunStatus::Complete);
        assert!(back.finished_at.is_some());
        assert_eq!(back.summary.as_deref(), Some("done"));
    }

    // ── AgentRunStatus serde rename_all lowercase ───────────────────────────

    #[test]
    fn agent_run_status_json_lowercase_names() {
        assert_eq!(serde_json::to_string(&AgentRunStatus::Running).unwrap(), r#""running""#);
        assert_eq!(serde_json::to_string(&AgentRunStatus::Complete).unwrap(), r#""complete""#);
        assert_eq!(serde_json::to_string(&AgentRunStatus::Failed).unwrap(), r#""failed""#);
        assert_eq!(serde_json::to_string(&AgentRunStatus::Cancelled).unwrap(), r#""cancelled""#);
    }

    // ── Multiple runs tracked independently ─────────────────────────────────

    #[test]
    fn multiple_runs_tracked_independently() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        let def_a = mgr.create_template("a", "task a").unwrap();
        let def_b = mgr.create_template("b", "task b").unwrap();

        let run_a = mgr.start_run(&def_a);
        let run_b = mgr.start_run(&def_b);

        mgr.finish_run(&run_a.id, AgentRunStatus::Complete, Some("ok".into()));
        mgr.finish_run(&run_b.id, AgentRunStatus::Failed, Some("err".into()));

        let a = mgr.get_run(&run_a.id).unwrap();
        let b = mgr.get_run(&run_b.id).unwrap();
        assert_eq!(a.status, AgentRunStatus::Complete);
        assert_eq!(b.status, AgentRunStatus::Failed);
    }

    // ── finish_run on nonexistent ID is a no-op ─────────────────────────────

    #[test]
    fn finish_run_nonexistent_is_noop() {
        let tmp = TempDir::new().unwrap();
        let mgr = BackgroundAgentManager::new(tmp.path().to_path_buf());
        // Should not panic
        mgr.finish_run("does-not-exist", AgentRunStatus::Failed, None);
        assert!(mgr.get_run("does-not-exist").is_none());
    }

    // ── AgentRun finished_at is after started_at ────────────────────────────

    #[test]
    fn agent_run_finished_at_after_started_at() {
        let mut run = AgentRun::new("r1", "test", "task");
        let started = run.started_at;
        run.finish(AgentRunStatus::Complete, None);
        assert!(run.finished_at.unwrap() >= started);
    }
}
