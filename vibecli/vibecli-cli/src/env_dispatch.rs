#![allow(dead_code)]
//! Cross-environment parallel agent dispatch — heterogeneous environment abstraction.
//!
//! Provides a `DispatchRouter` that manages a pool of heterogeneous execution
//! environments (local, git worktree, remote SSH, cloud VMs) and routes agent
//! tasks to the best available environment. Environments are tracked with health
//! status and resource utilisation. A `ProgressAggregator` collects structured
//! event logs keyed by task ID.
//!
//! # Architecture
//!
//! ```text
//! DispatchRouter
//!   ├─ Vec<EnvironmentStatus>   — registered execution environments
//!   └─ Vec<DispatchedTask>      — lifecycle-tracked tasks
//!
//! DispatchedTask (Pending → Running → Completed | Failed | Cancelled)
//!
//! ProgressAggregator
//!   └─ HashMap<task_id, Vec<String>>   — event log per task
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Cloud infrastructure provider for `CloudVM` environments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    Aws,
    Gce,
    Azure,
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "AWS"),
            Self::Gce => write!(f, "GCE"),
            Self::Azure => write!(f, "Azure"),
        }
    }
}

/// The kind of execution environment that a task can be dispatched to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionEnvironment {
    /// The local machine running this process.
    Local,
    /// An isolated git worktree on the local filesystem.
    GitWorktree {
        branch: String,
        path: String,
    },
    /// A remote host reachable via SSH.
    RemoteSSH {
        host: String,
        user: String,
        key_path: String,
        port: u16,
    },
    /// A cloud virtual machine (AWS / GCE / Azure).
    CloudVM {
        provider: CloudProvider,
        instance_id: String,
        region: String,
    },
}

impl std::fmt::Display for ExecutionEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "Local"),
            Self::GitWorktree { branch, .. } => write!(f, "GitWorktree({})", branch),
            Self::RemoteSSH { host, port, .. } => write!(f, "SSH({}:{})", host, port),
            Self::CloudVM { provider, instance_id, .. } => {
                write!(f, "CloudVM({}/{})", provider, instance_id)
            }
        }
    }
}

/// Liveness / readiness of an execution environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvironmentHealth {
    Healthy,
    /// The environment is partially available; includes a human-readable reason.
    Degraded(String),
    Unreachable,
}

impl std::fmt::Display for EnvironmentHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "Healthy"),
            Self::Degraded(r) => write!(f, "Degraded({})", r),
            Self::Unreachable => write!(f, "Unreachable"),
        }
    }
}

/// Lifecycle state of a dispatched task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// Tuning parameters for the `DispatchRouter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchConfig {
    /// Maximum number of tasks allowed to run concurrently.
    pub max_parallel: usize,
    /// Cumulative spend ceiling in US cents; routing stops when exceeded.
    pub cost_budget_cents: u64,
    /// When `true`, prefer the `Local` environment over remote ones.
    pub prefer_local: bool,
    /// Per-task deadline in seconds before it is considered failed.
    pub timeout_secs: u64,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            cost_budget_cents: 10_000,
            prefer_local: true,
            timeout_secs: 300,
        }
    }
}

/// Runtime snapshot of a single registered environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    /// Stable human-readable identifier for this environment slot.
    pub env_id: String,
    pub environment: ExecutionEnvironment,
    pub health: EnvironmentHealth,
    /// Task description currently executing in this environment, if any.
    pub current_task: Option<String>,
    /// CPU/memory utilisation percentage (0–100).
    pub resource_usage_pct: f32,
}

/// A task that has been handed off to the dispatch router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchedTask {
    pub task_id: String,
    pub description: String,
    pub environment: String,
    pub status: TaskStatus,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

// ─── DispatchRouter ──────────────────────────────────────────────────────────

/// Manages a heterogeneous pool of execution environments and routes tasks to
/// the best available slot respecting health, load, and configuration policy.
pub struct DispatchRouter {
    config: DispatchConfig,
    environments: Vec<EnvironmentStatus>,
    tasks: Vec<DispatchedTask>,
    next_task_id: u64,
    /// Monotonic fake clock for deterministic tests.
    clock_ms: u64,
}

impl DispatchRouter {
    pub fn new(config: DispatchConfig) -> Self {
        Self {
            config,
            environments: Vec::new(),
            tasks: Vec::new(),
            next_task_id: 1,
            clock_ms: 1_000,
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn tick(&mut self) -> u64 {
        self.clock_ms += 1;
        self.clock_ms
    }

    fn gen_task_id(&mut self) -> String {
        let id = format!("task-{}", self.next_task_id);
        self.next_task_id += 1;
        id
    }

    /// Returns `true` if the environment is healthy enough to accept work.
    fn is_available(status: &EnvironmentStatus) -> bool {
        matches!(status.health, EnvironmentHealth::Healthy | EnvironmentHealth::Degraded(_))
            && status.current_task.is_none()
    }

    // ── Public API ──────────────────────────────────────────────────────

    /// Register a new environment slot with the given identifier.
    ///
    /// Returns `Err` if an environment with the same `env_id` already exists.
    pub fn register_environment(
        &mut self,
        env_id: &str,
        env: ExecutionEnvironment,
    ) -> Result<(), String> {
        if self.environments.iter().any(|e| e.env_id == env_id) {
            return Err(format!("Environment '{}' already registered", env_id));
        }
        self.environments.push(EnvironmentStatus {
            env_id: env_id.to_string(),
            environment: env,
            health: EnvironmentHealth::Healthy,
            current_task: None,
            resource_usage_pct: 0.0,
        });
        Ok(())
    }

    /// Dispatch a task, optionally targeting a specific environment by ID.
    ///
    /// If `preferred_env` is given but unavailable, falls back to the first
    /// available environment in registration order.  Returns the generated
    /// `task_id` on success.
    pub fn dispatch_task(
        &mut self,
        description: &str,
        preferred_env: Option<&str>,
    ) -> Result<String, String> {
        // Determine the number of currently running tasks.
        let running_count = self.tasks.iter().filter(|t| t.status == TaskStatus::Running).count();
        if running_count >= self.config.max_parallel {
            return Err(format!(
                "max_parallel ({}) reached; cannot dispatch more tasks",
                self.config.max_parallel
            ));
        }

        // Try to honour the preferred env first.
        let chosen_id: String = if let Some(pref) = preferred_env {
            let pref_avail = self
                .environments
                .iter()
                .find(|e| e.env_id == pref)
                .map(|e| Self::is_available(e))
                .unwrap_or(false);

            if pref_avail {
                pref.to_string()
            } else {
                // Fall back: prefer Local if config says so, otherwise first available.
                self.pick_best_env()?
            }
        } else {
            self.pick_best_env()?
        };

        let ts = self.tick();
        let task_id = self.gen_task_id();

        // Mark environment as occupied.
        if let Some(env) = self.environments.iter_mut().find(|e| e.env_id == chosen_id) {
            env.current_task = Some(description.to_string());
        }

        self.tasks.push(DispatchedTask {
            task_id: task_id.clone(),
            description: description.to_string(),
            environment: chosen_id,
            status: TaskStatus::Running,
            started_at_ms: ts,
            completed_at_ms: None,
            result: None,
            error: None,
        });

        Ok(task_id)
    }

    /// Choose the best available environment according to config policy.
    fn pick_best_env(&self) -> Result<String, String> {
        // If prefer_local, try Local first.
        if self.config.prefer_local {
            if let Some(local_env) = self
                .environments
                .iter()
                .find(|e| e.environment == ExecutionEnvironment::Local && Self::is_available(e))
            {
                return Ok(local_env.env_id.clone());
            }
        }
        // First available in registration order.
        self.environments
            .iter()
            .find(|e| Self::is_available(e))
            .map(|e| e.env_id.clone())
            .ok_or_else(|| "No available environment".to_string())
    }

    /// Mark a running task as completed with its output.
    pub fn complete_task(&mut self, task_id: &str, result: String) -> Result<(), String> {
        let ts = self.tick();
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task '{}' not found", task_id))?;
        if task.status != TaskStatus::Running {
            return Err(format!("Task '{}' is not running (status: {})", task_id, task.status));
        }
        let env_id = task.environment.clone();
        task.status = TaskStatus::Completed;
        task.completed_at_ms = Some(ts);
        task.result = Some(result);
        self.free_environment(&env_id);
        Ok(())
    }

    /// Mark a running task as failed with an error description.
    pub fn fail_task(&mut self, task_id: &str, error: String) -> Result<(), String> {
        let ts = self.tick();
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task '{}' not found", task_id))?;
        if task.status != TaskStatus::Running {
            return Err(format!("Task '{}' is not running (status: {})", task_id, task.status));
        }
        let env_id = task.environment.clone();
        task.status = TaskStatus::Failed;
        task.completed_at_ms = Some(ts);
        task.error = Some(error);
        self.free_environment(&env_id);
        Ok(())
    }

    /// Cancel a task that is either pending or running.
    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), String> {
        let ts = self.tick();
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task '{}' not found", task_id))?;
        if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
            return Err(format!("Task '{}' already finished; cannot cancel", task_id));
        }
        let env_id = task.environment.clone();
        task.status = TaskStatus::Cancelled;
        task.completed_at_ms = Some(ts);
        self.free_environment(&env_id);
        Ok(())
    }

    /// Releases the `current_task` lock on the named environment.
    fn free_environment(&mut self, env_id: &str) {
        if let Some(env) = self.environments.iter_mut().find(|e| e.env_id == env_id) {
            env.current_task = None;
        }
    }

    /// Returns references to all tasks currently in `Running` state.
    pub fn running_tasks(&self) -> Vec<&DispatchedTask> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .collect()
    }

    /// Returns environments that are healthy/degraded and have no current task.
    pub fn available_environments(&self) -> Vec<&EnvironmentStatus> {
        self.environments.iter().filter(|e| Self::is_available(e)).collect()
    }

    /// Update the health status of a named environment.
    pub fn update_health(
        &mut self,
        env_id: &str,
        health: EnvironmentHealth,
    ) -> Result<(), String> {
        self.environments
            .iter_mut()
            .find(|e| e.env_id == env_id)
            .ok_or_else(|| format!("Environment '{}' not found", env_id))
            .map(|e| e.health = health)
    }

    /// Total number of registered environments.
    pub fn environment_count(&self) -> usize {
        self.environments.len()
    }

    /// Fraction of environments that are currently executing a task (0.0–1.0).
    pub fn utilization_pct(&self) -> f32 {
        let total = self.environments.len();
        if total == 0 {
            return 0.0;
        }
        let busy = self.environments.iter().filter(|e| e.current_task.is_some()).count();
        busy as f32 / total as f32
    }

    /// Look up a task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&DispatchedTask> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    /// Look up an environment status by ID.
    pub fn get_environment(&self, env_id: &str) -> Option<&EnvironmentStatus> {
        self.environments.iter().find(|e| e.env_id == env_id)
    }

    /// Returns all tasks (any status).
    pub fn all_tasks(&self) -> &[DispatchedTask] {
        &self.tasks
    }
}

// ─── ProgressAggregator ──────────────────────────────────────────────────────

/// Collects free-form event messages keyed by task ID.
///
/// Consumers append messages as tasks progress; the aggregator provides ordered
/// retrieval per task or across all tasks.
pub struct ProgressAggregator {
    /// task_id → ordered list of event messages
    events: HashMap<String, Vec<String>>,
    /// Insertion-order record of (task_id, message) for `all_events`.
    ordered: Vec<(String, String)>,
}

impl ProgressAggregator {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            ordered: Vec::new(),
        }
    }

    /// Append a progress event for the given `task_id`.
    pub fn record_event(&mut self, task_id: &str, message: &str) {
        self.events
            .entry(task_id.to_string())
            .or_default()
            .push(message.to_string());
        self.ordered.push((task_id.to_string(), message.to_string()));
    }

    /// Returns all event messages recorded for `task_id`, in insertion order.
    pub fn events_for(&self, task_id: &str) -> Vec<&str> {
        self.events
            .get(task_id)
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Returns every `(task_id, message)` pair across all tasks, in insertion order.
    pub fn all_events(&self) -> Vec<(&str, &str)> {
        self.ordered
            .iter()
            .map(|(tid, msg)| (tid.as_str(), msg.as_str()))
            .collect()
    }

    /// Number of unique task IDs that have at least one event.
    pub fn task_count(&self) -> usize {
        self.events.len()
    }

    /// Total number of events recorded across all tasks.
    pub fn total_event_count(&self) -> usize {
        self.ordered.len()
    }
}

impl Default for ProgressAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn default_router() -> DispatchRouter {
        DispatchRouter::new(DispatchConfig::default())
    }

    fn router_with_capacity(max_parallel: usize) -> DispatchRouter {
        DispatchRouter::new(DispatchConfig {
            max_parallel,
            ..DispatchConfig::default()
        })
    }

    fn register_local(r: &mut DispatchRouter, id: &str) {
        r.register_environment(id, ExecutionEnvironment::Local).unwrap();
    }

    fn register_worktree(r: &mut DispatchRouter, id: &str, branch: &str) {
        r.register_environment(
            id,
            ExecutionEnvironment::GitWorktree {
                branch: branch.to_string(),
                path: "/tmp/wt".to_string(),
            },
        )
        .unwrap();
    }

    fn register_ssh(r: &mut DispatchRouter, id: &str) {
        r.register_environment(
            id,
            ExecutionEnvironment::RemoteSSH {
                host: "build.example.com".to_string(),
                user: "ci".to_string(),
                key_path: "~/.ssh/id_rsa".to_string(),
                port: 22,
            },
        )
        .unwrap();
    }

    fn register_cloud(r: &mut DispatchRouter, id: &str, provider: CloudProvider) {
        r.register_environment(
            id,
            ExecutionEnvironment::CloudVM {
                provider,
                instance_id: "i-abc123".to_string(),
                region: "us-east-1".to_string(),
            },
        )
        .unwrap();
    }

    // ── 1: CloudProvider display ─────────────────────────────────────────

    #[test]
    fn test_cloud_provider_display_aws() {
        assert_eq!(CloudProvider::Aws.to_string(), "AWS");
    }

    // 2
    #[test]
    fn test_cloud_provider_display_gce() {
        assert_eq!(CloudProvider::Gce.to_string(), "GCE");
    }

    // 3
    #[test]
    fn test_cloud_provider_display_azure() {
        assert_eq!(CloudProvider::Azure.to_string(), "Azure");
    }

    // ── 4: ExecutionEnvironment display ─────────────────────────────────

    #[test]
    fn test_env_display_local() {
        assert_eq!(ExecutionEnvironment::Local.to_string(), "Local");
    }

    // 5
    #[test]
    fn test_env_display_git_worktree() {
        let env = ExecutionEnvironment::GitWorktree {
            branch: "feat/x".to_string(),
            path: "/tmp/x".to_string(),
        };
        assert_eq!(env.to_string(), "GitWorktree(feat/x)");
    }

    // 6
    #[test]
    fn test_env_display_ssh() {
        let env = ExecutionEnvironment::RemoteSSH {
            host: "h".to_string(),
            user: "u".to_string(),
            key_path: "k".to_string(),
            port: 2222,
        };
        assert_eq!(env.to_string(), "SSH(h:2222)");
    }

    // 7
    #[test]
    fn test_env_display_cloud_vm() {
        let env = ExecutionEnvironment::CloudVM {
            provider: CloudProvider::Aws,
            instance_id: "i-1".to_string(),
            region: "eu-west-1".to_string(),
        };
        assert_eq!(env.to_string(), "CloudVM(AWS/i-1)");
    }

    // ── 8: EnvironmentHealth display ────────────────────────────────────

    #[test]
    fn test_health_display_healthy() {
        assert_eq!(EnvironmentHealth::Healthy.to_string(), "Healthy");
    }

    // 9
    #[test]
    fn test_health_display_degraded() {
        let h = EnvironmentHealth::Degraded("high load".to_string());
        assert_eq!(h.to_string(), "Degraded(high load)");
    }

    // 10
    #[test]
    fn test_health_display_unreachable() {
        assert_eq!(EnvironmentHealth::Unreachable.to_string(), "Unreachable");
    }

    // ── 11: TaskStatus display ───────────────────────────────────────────

    #[test]
    fn test_task_status_display_variants() {
        assert_eq!(TaskStatus::Pending.to_string(), "Pending");
        assert_eq!(TaskStatus::Running.to_string(), "Running");
        assert_eq!(TaskStatus::Completed.to_string(), "Completed");
        assert_eq!(TaskStatus::Failed.to_string(), "Failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "Cancelled");
    }

    // ── 12: DispatchConfig default ───────────────────────────────────────

    #[test]
    fn test_dispatch_config_default() {
        let c = DispatchConfig::default();
        assert_eq!(c.max_parallel, 4);
        assert!(c.prefer_local);
        assert_eq!(c.timeout_secs, 300);
    }

    // ── 13: register_environment ─────────────────────────────────────────

    #[test]
    fn test_register_environment_success() {
        let mut r = default_router();
        assert!(r.register_environment("env-1", ExecutionEnvironment::Local).is_ok());
        assert_eq!(r.environment_count(), 1);
    }

    // 14
    #[test]
    fn test_register_environment_duplicate_errors() {
        let mut r = default_router();
        r.register_environment("env-1", ExecutionEnvironment::Local).unwrap();
        let res = r.register_environment("env-1", ExecutionEnvironment::Local);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("already registered"));
    }

    // 15
    #[test]
    fn test_register_multiple_environments() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "main");
        register_ssh(&mut r, "ssh-1");
        assert_eq!(r.environment_count(), 3);
    }

    // 16
    #[test]
    fn test_register_cloud_vm() {
        let mut r = default_router();
        register_cloud(&mut r, "cloud-1", CloudProvider::Gce);
        assert_eq!(r.environment_count(), 1);
        let env = r.get_environment("cloud-1").unwrap();
        assert!(matches!(env.environment, ExecutionEnvironment::CloudVM { provider: CloudProvider::Gce, .. }));
    }

    // ── 17: dispatch_task ────────────────────────────────────────────────

    #[test]
    fn test_dispatch_task_returns_task_id() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build project", None).unwrap();
        assert!(tid.starts_with("task-"));
    }

    // 18
    #[test]
    fn test_dispatched_task_is_running() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.status, TaskStatus::Running);
    }

    // 19
    #[test]
    fn test_dispatch_occupies_environment() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.dispatch_task("build", None).unwrap();
        let env = r.get_environment("local").unwrap();
        assert!(env.current_task.is_some());
    }

    // 20
    #[test]
    fn test_dispatch_no_env_errors() {
        let mut r = default_router();
        let res = r.dispatch_task("build", None);
        assert!(res.is_err());
    }

    // 21
    #[test]
    fn test_dispatch_prefers_local_when_config_set() {
        let mut r = default_router();
        register_ssh(&mut r, "ssh-1");
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.environment, "local");
    }

    // 22
    #[test]
    fn test_dispatch_preferred_env_honoured() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "feat");
        let tid = r.dispatch_task("test", Some("wt-1")).unwrap();
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.environment, "wt-1");
    }

    // 23
    #[test]
    fn test_dispatch_preferred_busy_falls_back() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "feat");
        // Occupy local (prefer_local is true, but local will be occupied).
        r.dispatch_task("first", Some("local")).unwrap();
        // Now dispatch preferring local again — should fall back to wt-1.
        let tid = r.dispatch_task("second", Some("local")).unwrap();
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.environment, "wt-1");
    }

    // 24
    #[test]
    fn test_dispatch_respects_max_parallel() {
        let mut r = router_with_capacity(2);
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        register_worktree(&mut r, "wt-2", "b");
        r.dispatch_task("t1", None).unwrap();
        r.dispatch_task("t2", None).unwrap();
        let res = r.dispatch_task("t3", None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("max_parallel"));
    }

    // 25
    #[test]
    fn test_dispatch_task_description_stored() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("run unit tests", None).unwrap();
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.description, "run unit tests");
    }

    // ── 26: complete_task ────────────────────────────────────────────────

    #[test]
    fn test_complete_task_success() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        assert!(r.complete_task(&tid, "ok".to_string()).is_ok());
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.result.as_deref(), Some("ok"));
        assert!(task.completed_at_ms.is_some());
    }

    // 27
    #[test]
    fn test_complete_task_frees_environment() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.complete_task(&tid, "done".to_string()).unwrap();
        let env = r.get_environment("local").unwrap();
        assert!(env.current_task.is_none());
    }

    // 28
    #[test]
    fn test_complete_task_missing_errors() {
        let mut r = default_router();
        assert!(r.complete_task("task-99", "x".to_string()).is_err());
    }

    // 29
    #[test]
    fn test_complete_already_completed_errors() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.complete_task(&tid, "done".to_string()).unwrap();
        assert!(r.complete_task(&tid, "again".to_string()).is_err());
    }

    // ── 30: fail_task ────────────────────────────────────────────────────

    #[test]
    fn test_fail_task_success() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        assert!(r.fail_task(&tid, "OOM".to_string()).is_ok());
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error.as_deref(), Some("OOM"));
        assert!(task.completed_at_ms.is_some());
    }

    // 31
    #[test]
    fn test_fail_task_frees_environment() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.fail_task(&tid, "crash".to_string()).unwrap();
        let env = r.get_environment("local").unwrap();
        assert!(env.current_task.is_none());
    }

    // 32
    #[test]
    fn test_fail_task_missing_errors() {
        let mut r = default_router();
        assert!(r.fail_task("task-99", "x".to_string()).is_err());
    }

    // 33
    #[test]
    fn test_fail_completed_task_errors() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.complete_task(&tid, "done".to_string()).unwrap();
        assert!(r.fail_task(&tid, "late error".to_string()).is_err());
    }

    // ── 34: cancel_task ──────────────────────────────────────────────────

    #[test]
    fn test_cancel_running_task() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        assert!(r.cancel_task(&tid).is_ok());
        let task = r.get_task(&tid).unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    // 35
    #[test]
    fn test_cancel_frees_environment() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.cancel_task(&tid).unwrap();
        let env = r.get_environment("local").unwrap();
        assert!(env.current_task.is_none());
    }

    // 36
    #[test]
    fn test_cancel_completed_task_errors() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.complete_task(&tid, "ok".to_string()).unwrap();
        assert!(r.cancel_task(&tid).is_err());
    }

    // 37
    #[test]
    fn test_cancel_failed_task_errors() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("build", None).unwrap();
        r.fail_task(&tid, "crash".to_string()).unwrap();
        assert!(r.cancel_task(&tid).is_err());
    }

    // 38
    #[test]
    fn test_cancel_missing_task_errors() {
        let mut r = default_router();
        assert!(r.cancel_task("task-99").is_err());
    }

    // ── 39: running_tasks ────────────────────────────────────────────────

    #[test]
    fn test_running_tasks_empty_initially() {
        let r = default_router();
        assert!(r.running_tasks().is_empty());
    }

    // 40
    #[test]
    fn test_running_tasks_count() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        r.dispatch_task("t1", None).unwrap();
        r.dispatch_task("t2", None).unwrap();
        assert_eq!(r.running_tasks().len(), 2);
    }

    // 41
    #[test]
    fn test_running_tasks_decrements_on_complete() {
        let mut r = default_router();
        register_local(&mut r, "local");
        let tid = r.dispatch_task("t1", None).unwrap();
        assert_eq!(r.running_tasks().len(), 1);
        r.complete_task(&tid, "done".to_string()).unwrap();
        assert_eq!(r.running_tasks().len(), 0);
    }

    // ── 42: available_environments ───────────────────────────────────────

    #[test]
    fn test_available_environments_all_free() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        assert_eq!(r.available_environments().len(), 2);
    }

    // 43
    #[test]
    fn test_available_environments_busy_excluded() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.dispatch_task("t", None).unwrap();
        assert_eq!(r.available_environments().len(), 0);
    }

    // 44
    #[test]
    fn test_available_environments_unreachable_excluded() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.update_health("local", EnvironmentHealth::Unreachable).unwrap();
        assert_eq!(r.available_environments().len(), 0);
    }

    // 45
    #[test]
    fn test_available_environments_degraded_included() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.update_health("local", EnvironmentHealth::Degraded("slow".to_string())).unwrap();
        assert_eq!(r.available_environments().len(), 1);
    }

    // ── 46: update_health ────────────────────────────────────────────────

    #[test]
    fn test_update_health_success() {
        let mut r = default_router();
        register_local(&mut r, "local");
        assert!(r.update_health("local", EnvironmentHealth::Degraded("high cpu".to_string())).is_ok());
        let env = r.get_environment("local").unwrap();
        assert!(matches!(env.health, EnvironmentHealth::Degraded(_)));
    }

    // 47
    #[test]
    fn test_update_health_missing_errors() {
        let mut r = default_router();
        assert!(r.update_health("nope", EnvironmentHealth::Healthy).is_err());
    }

    // 48
    #[test]
    fn test_update_health_to_unreachable_makes_unavailable() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.update_health("local", EnvironmentHealth::Unreachable).unwrap();
        assert!(r.available_environments().is_empty());
    }

    // 49
    #[test]
    fn test_update_health_recover_to_healthy() {
        let mut r = default_router();
        register_local(&mut r, "local");
        r.update_health("local", EnvironmentHealth::Unreachable).unwrap();
        r.update_health("local", EnvironmentHealth::Healthy).unwrap();
        assert_eq!(r.available_environments().len(), 1);
    }

    // ── 50: utilization_pct ──────────────────────────────────────────────

    #[test]
    fn test_utilization_zero_when_no_envs() {
        let r = default_router();
        assert_eq!(r.utilization_pct(), 0.0);
    }

    // 51
    #[test]
    fn test_utilization_zero_when_all_free() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        assert_eq!(r.utilization_pct(), 0.0);
    }

    // 52
    #[test]
    fn test_utilization_full_when_all_busy() {
        let mut r = router_with_capacity(2);
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        r.dispatch_task("t1", None).unwrap();
        r.dispatch_task("t2", None).unwrap();
        assert_eq!(r.utilization_pct(), 1.0);
    }

    // 53
    #[test]
    fn test_utilization_partial() {
        let mut r = default_router();
        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "a");
        r.dispatch_task("t1", Some("local")).unwrap();
        let util = r.utilization_pct();
        assert!((util - 0.5).abs() < 1e-5);
    }

    // ── 54: ProgressAggregator ───────────────────────────────────────────

    #[test]
    fn test_progress_aggregator_new() {
        let agg = ProgressAggregator::new();
        assert_eq!(agg.task_count(), 0);
        assert_eq!(agg.total_event_count(), 0);
    }

    // 55
    #[test]
    fn test_record_event_single() {
        let mut agg = ProgressAggregator::new();
        agg.record_event("task-1", "started");
        assert_eq!(agg.events_for("task-1"), vec!["started"]);
    }

    // 56
    #[test]
    fn test_record_event_multiple_for_same_task() {
        let mut agg = ProgressAggregator::new();
        agg.record_event("task-1", "started");
        agg.record_event("task-1", "50% done");
        agg.record_event("task-1", "complete");
        let evts = agg.events_for("task-1");
        assert_eq!(evts.len(), 3);
        assert_eq!(evts[1], "50% done");
    }

    // 57
    #[test]
    fn test_record_event_multiple_tasks() {
        let mut agg = ProgressAggregator::new();
        agg.record_event("task-1", "a");
        agg.record_event("task-2", "b");
        assert_eq!(agg.task_count(), 2);
    }

    // 58
    #[test]
    fn test_events_for_unknown_task_empty() {
        let agg = ProgressAggregator::new();
        assert!(agg.events_for("task-99").is_empty());
    }

    // 59
    #[test]
    fn test_all_events_insertion_order() {
        let mut agg = ProgressAggregator::new();
        agg.record_event("t1", "start");
        agg.record_event("t2", "init");
        agg.record_event("t1", "finish");
        let all = agg.all_events();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], ("t1", "start"));
        assert_eq!(all[1], ("t2", "init"));
        assert_eq!(all[2], ("t1", "finish"));
    }

    // 60
    #[test]
    fn test_total_event_count() {
        let mut agg = ProgressAggregator::new();
        agg.record_event("t1", "e1");
        agg.record_event("t1", "e2");
        agg.record_event("t2", "e3");
        assert_eq!(agg.total_event_count(), 3);
    }

    // ── 61: full lifecycle smoke test ────────────────────────────────────

    #[test]
    fn test_full_dispatch_lifecycle() {
        let mut r = default_router();
        let mut agg = ProgressAggregator::new();

        register_local(&mut r, "local");
        register_worktree(&mut r, "wt-1", "feat/bar");
        register_ssh(&mut r, "ssh-1");
        register_cloud(&mut r, "cloud-1", CloudProvider::Azure);

        assert_eq!(r.environment_count(), 4);
        assert_eq!(r.available_environments().len(), 4);
        assert_eq!(r.utilization_pct(), 0.0);

        let t1 = r.dispatch_task("compile", None).unwrap();
        agg.record_event(&t1, "compiling…");

        let t2 = r.dispatch_task("lint", Some("wt-1")).unwrap();
        agg.record_event(&t2, "linting…");

        assert_eq!(r.running_tasks().len(), 2);
        assert!((r.utilization_pct() - 0.5).abs() < 1e-5);

        r.complete_task(&t1, "binary built".to_string()).unwrap();
        agg.record_event(&t1, "done");

        r.fail_task(&t2, "lint error on line 42".to_string()).unwrap();
        agg.record_event(&t2, "failed");

        assert_eq!(r.running_tasks().len(), 0);
        assert_eq!(r.utilization_pct(), 0.0);

        assert_eq!(agg.events_for(&t1).len(), 2);
        assert_eq!(agg.events_for(&t2).len(), 2);
        assert_eq!(agg.total_event_count(), 4);
    }

    // 62: dispatch after complete re-uses the freed environment
    #[test]
    fn test_redispatch_after_complete() {
        let mut r = router_with_capacity(1);
        register_local(&mut r, "local");
        let t1 = r.dispatch_task("first", None).unwrap();
        r.complete_task(&t1, "ok".to_string()).unwrap();
        // Should succeed now that local is free again.
        let t2 = r.dispatch_task("second", None);
        assert!(t2.is_ok());
    }

    // 63: SSH environment registered correctly
    #[test]
    fn test_ssh_env_fields_stored() {
        let mut r = default_router();
        r.register_environment(
            "ssh-prod",
            ExecutionEnvironment::RemoteSSH {
                host: "prod.server.io".to_string(),
                user: "deploy".to_string(),
                key_path: "/etc/deploy.pem".to_string(),
                port: 2222,
            },
        )
        .unwrap();
        let env = r.get_environment("ssh-prod").unwrap();
        assert!(matches!(
            &env.environment,
            ExecutionEnvironment::RemoteSSH { port: 2222, .. }
        ));
    }
}
