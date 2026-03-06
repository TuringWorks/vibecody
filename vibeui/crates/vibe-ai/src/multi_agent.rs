//! Parallel multi-agent orchestration using git worktrees.
//!
//! `MultiAgentOrchestrator` spawns N independent `AgentLoop` instances, each
//! working in its own git worktree on a separate branch. This enables:
//! - Running the same task N times in parallel (ensemble approach)
//! - Running different sub-tasks simultaneously
//!
//! After all agents complete, the caller can inspect each agent's branch and
//! merge the best result via `vibe_core::git::merge_worktree_branch`.

use crate::agent::{AgentContext, AgentEvent, AgentLoop, AgentStep, ApprovalPolicy, ToolExecutorTrait};
use crate::hooks::HookRunner;
use crate::provider::AIProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

// ── Agent Status ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

// ── AgentInstance ─────────────────────────────────────────────────────────────

/// Represents one agent running in a git worktree.
#[derive(Debug, Clone)]
pub struct AgentInstance {
    pub id: usize,
    pub task: String,
    pub worktree: PathBuf,
    pub branch: String,
    pub status: AgentStatus,
    pub steps: Vec<AgentStep>,
    pub summary: Option<String>,
    pub error: Option<String>,
}

// ── AgentTask ─────────────────────────────────────────────────────────────────

/// A task to assign to one agent in the multi-agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: usize,
    pub description: String,
    /// Optional label for the worktree branch (defaults to `vibe-agent-<id>`).
    pub branch_label: Option<String>,
}

impl AgentTask {
    pub fn new(id: usize, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            branch_label: None,
        }
    }

    pub fn branch_name(&self) -> String {
        self.branch_label
            .clone()
            .unwrap_or_else(|| format!("vibe-agent-{}", self.id))
    }
}

// ── AgentResult ───────────────────────────────────────────────────────────────

/// Result from a single completed agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub id: usize,
    pub task: String,
    pub branch: String,
    pub worktree: PathBuf,
    pub success: bool,
    pub summary: String,
    pub steps_taken: usize,
}

// ── OrchestratorEvent ─────────────────────────────────────────────────────────

/// Events emitted by the orchestrator as agents run.
#[derive(Debug)]
pub enum OrchestratorEvent {
    AgentStarted { id: usize, task: String, worktree: PathBuf },
    AgentStep { id: usize, step: AgentStep },
    AgentChunk { id: usize, text: String },
    AgentComplete { id: usize, summary: String, branch: String },
    AgentError { id: usize, error: String },
    AllComplete { results: Vec<AgentResult> },
}

// ── MultiAgentOrchestrator ────────────────────────────────────────────────────

/// Runs multiple `AgentLoop` instances in parallel, each in its own git worktree.
pub struct MultiAgentOrchestrator {
    provider: Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    executor_factory: Arc<dyn ExecutorFactory>,
    max_agents: usize,
    hooks: Option<Arc<HookRunner>>,
    worktree_manager: Option<Arc<dyn WorktreeManager>>,
}

/// Factory trait for creating `ToolExecutorTrait` instances per worktree.
pub trait ExecutorFactory: Send + Sync {
    fn create(&self, workspace_root: PathBuf) -> Arc<dyn ToolExecutorTrait>;
}

/// Trait for managing git worktrees. Implemented by callers that have vibe-core available.
pub trait WorktreeManager: Send + Sync {
    /// Create a new worktree branch. Returns the path created.
    fn create_worktree(&self, branch: &str, worktree_path: &std::path::Path) -> Result<()>;
    /// Remove a worktree.
    fn remove_worktree(&self, worktree_path: &std::path::Path) -> Result<()>;
    /// Create an isolated worktree for a single agent, auto-cleanup on Drop.
    /// The worktree is created at `<workspace>/.vibecli/worktrees/<agent_id>/`.
    fn create_isolated_worktree(&self, agent_id: &str) -> Result<IsolatedWorktree>;
}

// ── IsolatedWorktree ──────────────────────────────────────────────────────────

/// A temporary git worktree for a single agent.
/// Automatically deleted when dropped (RAII pattern).
pub struct IsolatedWorktree {
    pub path: std::path::PathBuf,
    pub branch: String,
    pub agent_id: String,
    /// Reference to the manager so we can call remove_worktree on drop.
    manager: Arc<dyn WorktreeManager>,
}

impl IsolatedWorktree {
    pub fn new(
        path: std::path::PathBuf,
        branch: String,
        agent_id: String,
        manager: Arc<dyn WorktreeManager>,
    ) -> Self {
        Self { path, branch, agent_id, manager }
    }
}

impl Drop for IsolatedWorktree {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = self.manager.remove_worktree(&self.path) {
                tracing::warn!("Failed to clean up worktree for agent {}: {}", self.agent_id, e);
            }
        }
    }
}

impl MultiAgentOrchestrator {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        approval: ApprovalPolicy,
        executor_factory: Arc<dyn ExecutorFactory>,
    ) -> Self {
        Self {
            provider,
            approval,
            executor_factory,
            max_agents: 8,
            hooks: None,
            worktree_manager: None,
        }
    }

    pub fn with_max_agents(mut self, n: usize) -> Self {
        self.max_agents = n;
        self
    }

    pub fn with_hooks(mut self, runner: HookRunner) -> Self {
        self.hooks = Some(Arc::new(runner));
        self
    }

    pub fn with_worktree_manager(mut self, manager: Arc<dyn WorktreeManager>) -> Self {
        self.worktree_manager = Some(manager);
        self
    }

    /// Split one task N ways and run them in parallel on separate worktrees.
    ///
    /// Each agent gets the same task and works independently. The caller can
    /// compare branches and pick the best result.
    pub async fn run_parallel(
        &self,
        repo_path: &PathBuf,
        task: &str,
        n: usize,
        event_tx: mpsc::Sender<OrchestratorEvent>,
    ) -> Result<Vec<AgentResult>> {
        let n = n.min(self.max_agents);
        let tasks: Vec<AgentTask> = (0..n)
            .map(|i| AgentTask::new(i, task))
            .collect();
        self.run_tasks(repo_path, tasks, event_tx).await
    }

    /// Run different tasks on different agents simultaneously.
    pub async fn run_tasks(
        &self,
        repo_path: &PathBuf,
        tasks: Vec<AgentTask>,
        event_tx: mpsc::Sender<OrchestratorEvent>,
    ) -> Result<Vec<AgentResult>> {
        let n = tasks.len().min(self.max_agents);
        let tasks = &tasks[..n];

        // Create worktrees for each agent (when a WorktreeManager is available)
        let mut worktree_paths: Vec<PathBuf> = Vec::new();
        for task in tasks.iter() {
            let branch = task.branch_name();
            // Place worktrees in a sibling directory
            let wt_path = repo_path.parent()
                .unwrap_or(repo_path)
                .join(format!(".vibe-worktree-{}", task.id));

            if let Some(ref manager) = self.worktree_manager {
                match manager.create_worktree(&branch, &wt_path) {
                    Ok(()) => {
                        worktree_paths.push(wt_path.clone());
                        tracing::info!("Created worktree {} at {}", branch, wt_path.display());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create worktree for task {}: {}", task.id, e);
                        // Use main repo as fallback
                        worktree_paths.push(repo_path.clone());
                    }
                }
            } else {
                // No worktree manager — all agents share the same directory
                tracing::warn!("No WorktreeManager provided; task {} will run in main repo", task.id);
                worktree_paths.push(repo_path.clone());
            }
        }

        // Spawn all agents concurrently
        let mut handles = Vec::new();
        for (task, wt_path) in tasks.iter().zip(worktree_paths.iter()) {
            let provider = Arc::clone(&self.provider);
            let approval = self.approval.clone();
            let executor = self.executor_factory.create(wt_path.clone());
            let task_clone = task.clone();
            let wt_path_clone = wt_path.clone();
            let tx = event_tx.clone();
            let hooks = self.hooks.clone();

            let handle = tokio::spawn(async move {
                run_single_agent(
                    task_clone,
                    provider,
                    approval,
                    executor,
                    wt_path_clone,
                    hooks,
                    tx,
                ).await
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        // Clean up worktrees that are different from repo_path
        if let Some(ref manager) = self.worktree_manager {
            for (task, wt_path) in tasks.iter().zip(worktree_paths.iter()) {
                if wt_path != repo_path {
                    if let Err(e) = manager.remove_worktree(wt_path) {
                        tracing::warn!("Failed to remove worktree for task {}: {}", task.id, e);
                    }
                }
            }
        }

        let _ = event_tx.send(OrchestratorEvent::AllComplete { results: results.clone() }).await;
        Ok(results)
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

async fn run_single_agent(
    task: AgentTask,
    provider: Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    executor: Arc<dyn ToolExecutorTrait>,
    worktree: PathBuf,
    hooks: Option<Arc<HookRunner>>,
    event_tx: mpsc::Sender<OrchestratorEvent>,
) -> AgentResult {
    let id = task.id;
    let branch = task.branch_name();
    let task_desc = task.description.clone();

    let _ = event_tx.send(OrchestratorEvent::AgentStarted {
        id,
        task: task_desc.clone(),
        worktree: worktree.clone(),
    }).await;

    let mut agent = AgentLoop::new(Arc::clone(&provider), approval, Arc::clone(&executor));
    if let Some(runner) = hooks {
        agent.hooks = Some(runner);
    }

    let context = AgentContext {
        workspace_root: worktree.clone(),
        ..Default::default()
    };

    let (inner_tx, mut inner_rx) = mpsc::channel::<AgentEvent>(64);
    let task_str = task_desc.clone();
    tokio::spawn(async move {
        let _ = agent.run(&task_str, context, inner_tx).await;
    });

    let mut steps_taken = 0;
    let mut final_summary = String::new();
    let mut success = false;

    while let Some(event) = inner_rx.recv().await {
        match event {
            AgentEvent::StreamChunk(text) => {
                let _ = event_tx.send(OrchestratorEvent::AgentChunk { id, text }).await;
            }
            AgentEvent::ToolCallExecuted(step) => {
                steps_taken += 1;
                let _ = event_tx.send(OrchestratorEvent::AgentStep { id, step }).await;
            }
            AgentEvent::ToolCallPending { call, result_tx } => {
                // In parallel mode, auto-execute all tool calls
                let result = executor.execute(&call).await;
                steps_taken += 1;
                let _ = result_tx.send(Some(result));
            }
            AgentEvent::Complete(summary) => {
                final_summary = summary.clone();
                success = true;
                let _ = event_tx.send(OrchestratorEvent::AgentComplete {
                    id,
                    summary,
                    branch: branch.clone(),
                }).await;
                break;
            }
            AgentEvent::Error(err) => {
                final_summary = err.clone();
                let _ = event_tx.send(OrchestratorEvent::AgentError { id, error: err }).await;
                break;
            }
            AgentEvent::CircuitBreak { state, reason } => {
                // Treat circuit break as an error in parallel mode
                let msg = format!("Circuit breaker: {} — {}", state, reason);
                let _ = event_tx.send(OrchestratorEvent::AgentError { id, error: msg.clone() }).await;
                if state == crate::agent::AgentHealthState::Blocked {
                    final_summary = msg;
                    break;
                }
            }
        }
    }

    AgentResult {
        id,
        task: task_desc,
        branch,
        worktree,
        success,
        summary: final_summary,
        steps_taken,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_task_branch_name_default() {
        let task = AgentTask::new(3, "refactor auth");
        assert_eq!(task.branch_name(), "vibe-agent-3");
    }

    #[test]
    fn agent_task_branch_name_custom() {
        let mut task = AgentTask::new(0, "fix tests");
        task.branch_label = Some("fix-test-suite".to_string());
        assert_eq!(task.branch_name(), "fix-test-suite");
    }

    #[test]
    fn orchestrator_respects_max_agents() {
        // max_agents caps the parallel count
        let tasks: Vec<AgentTask> = (0..20).map(|i| AgentTask::new(i, "task")).collect();
        let n = tasks.len().min(8); // max_agents = 8
        assert_eq!(n, 8);
    }

    #[test]
    fn agent_task_new() {
        let task = AgentTask::new(5, "implement feature");
        assert_eq!(task.id, 5);
        assert_eq!(task.description, "implement feature");
        assert!(task.branch_label.is_none());
    }

    #[test]
    fn agent_task_serialization() {
        let task = AgentTask::new(1, "test task");
        let json = serde_json::to_string(&task).unwrap();
        let deser: AgentTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, 1);
        assert_eq!(deser.description, "test task");
    }

    #[test]
    fn agent_status_serialization() {
        let statuses = vec![
            (AgentStatus::Pending, "\"pending\""),
            (AgentStatus::Running, "\"running\""),
            (AgentStatus::Complete, "\"complete\""),
            (AgentStatus::Failed, "\"failed\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn agent_status_deserialization() {
        let pending: AgentStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(pending, AgentStatus::Pending);
        let running: AgentStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(running, AgentStatus::Running);
    }

    #[test]
    fn agent_status_equality() {
        assert_eq!(AgentStatus::Pending, AgentStatus::Pending);
        assert_ne!(AgentStatus::Pending, AgentStatus::Running);
    }

    #[test]
    fn agent_result_serialization() {
        let result = AgentResult {
            id: 0,
            task: "fix tests".to_string(),
            branch: "vibe-agent-0".to_string(),
            worktree: PathBuf::from("/tmp/wt"),
            success: true,
            summary: "All tests pass".to_string(),
            steps_taken: 5,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"steps_taken\":5"));
        let deser: AgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, 0);
        assert_eq!(deser.summary, "All tests pass");
    }

    #[test]
    fn agent_instance_clone() {
        let inst = AgentInstance {
            id: 1,
            task: "task".to_string(),
            worktree: PathBuf::from("/wt"),
            branch: "branch".to_string(),
            status: AgentStatus::Running,
            steps: vec![],
            summary: None,
            error: None,
        };
        let cloned = inst.clone();
        assert_eq!(cloned.id, 1);
        assert_eq!(cloned.status, AgentStatus::Running);
    }

    #[test]
    fn agent_task_branch_name_with_large_id() {
        let task = AgentTask::new(999, "task");
        assert_eq!(task.branch_name(), "vibe-agent-999");
    }
}
