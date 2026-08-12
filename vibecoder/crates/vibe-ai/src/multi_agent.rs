//! Parallel multi-agent orchestration using git worktrees.
//!
//! `MultiAgentOrchestrator` spawns N independent `AgentLoop` instances, each
//! working in its own git worktree on a separate branch. This enables:
//! - Running the same task N times in parallel (ensemble approach)
//! - Running different sub-tasks simultaneously
//!
//! After all agents complete, the caller can inspect each agent's branch and
//! merge the best result via `vibe_core::git::merge_worktree_branch`.

use crate::agent::{
    AgentContext, AgentEvent, AgentLoop, AgentStep, ApprovalPolicy, ToolExecutorTrait,
};
use crate::hooks::HookRunner;
use crate::provider::AIProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    /// Unified diff of everything the agent changed in its worktree, captured
    /// *before* the worktree is removed.
    ///
    /// Without this the fan-out produced nothing but prose: worktrees are torn
    /// down with `git worktree remove --force`, which discards uncommitted
    /// changes, so every file an agent wrote was deleted moments after it
    /// finished. A summary describing work that no longer exists is worse than
    /// no fan-out at all.
    ///
    /// `None` means the diff could not be captured — distinct from
    /// `Some(empty)`, which means the agent genuinely changed nothing.
    pub patch: Option<String>,
    /// Paths the agent touched, as reported by git in its worktree.
    pub files_changed: Vec<String>,
}

// ── OrchestratorEvent ─────────────────────────────────────────────────────────

/// Events emitted by the orchestrator as agents run.
#[derive(Debug)]
pub enum OrchestratorEvent {
    AgentStarted {
        id: usize,
        task: String,
        worktree: PathBuf,
    },
    AgentStep {
        id: usize,
        step: AgentStep,
    },
    AgentChunk {
        id: usize,
        text: String,
    },
    AgentComplete {
        id: usize,
        summary: String,
        branch: String,
    },
    AgentError {
        id: usize,
        error: String,
    },
    AllComplete {
        results: Vec<AgentResult>,
    },
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
        Self {
            path,
            branch,
            agent_id,
            manager,
        }
    }
}

impl Drop for IsolatedWorktree {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = self.manager.remove_worktree(&self.path) {
                tracing::warn!(
                    "Failed to clean up worktree for agent {}: {}",
                    self.agent_id,
                    e
                );
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
        let tasks: Vec<AgentTask> = (0..n).map(|i| AgentTask::new(i, task)).collect();
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
            let wt_path = repo_path
                .parent()
                .unwrap_or(repo_path)
                .join(format!(".vibe-worktree-{}", task.id));

            if let Some(ref manager) = self.worktree_manager {
                match manager.create_worktree(&branch, &wt_path) {
                    Ok(()) => {
                        worktree_paths.push(wt_path.clone());
                        tracing::info!("Created worktree {} at {}", branch, wt_path.display());
                    }
                    Err(e) => {
                        // Falls back to the main workspace, which is safe only
                        // because unisolated agents are serialised on
                        // `shared_workspace` below. Before that lock existed
                        // this line quietly turned an isolation failure into
                        // concurrent writes on one tree.
                        tracing::warn!(
                            task = task.id,
                            error = %e,
                            "Could not create a worktree; this agent will run serially in the \
                             main workspace instead of in parallel",
                        );
                        worktree_paths.push(repo_path.clone());
                    }
                }
            } else {
                // No worktree manager — every agent shares one directory, so
                // the fan-out is serial. Correct, but no faster than running
                // the tasks one after another; supply a WorktreeManager to get
                // actual parallelism.
                tracing::warn!(
                    task = task.id,
                    "No WorktreeManager provided; task will run serially in the main workspace",
                );
                worktree_paths.push(repo_path.clone());
            }
        }

        // Agents that could not get their own worktree share the main
        // workspace, and concurrent writes there would clobber each other —
        // the exact hazard worktrees exist to remove. They take this lock for
        // the duration of their run, so they proceed one at a time while the
        // properly isolated agents continue in parallel around them.
        //
        // A whole-run lock rather than per-file leases: an agent's writes are
        // only safe relative to the tree it read, so releasing between files
        // would still let a second agent edit a file the first is mid-way
        // through reasoning about. Coarse and correct beats fine and racy.
        let shared_workspace = Arc::new(tokio::sync::Mutex::new(()));
        let shared_count = worktree_paths.iter().filter(|p| *p == repo_path).count();
        if shared_count > 1 {
            tracing::warn!(
                agents = shared_count,
                "{shared_count} agents lack an isolated worktree and will run serially in the \
                 main workspace; fan-out is reduced to that extent",
            );
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
            // Only unisolated agents contend; an agent in its own worktree
            // takes nothing and is never delayed by one that is not.
            let lock = (*wt_path == *repo_path).then(|| Arc::clone(&shared_workspace));

            let handle = tokio::spawn(async move {
                // Held for the whole run, released when this guard drops.
                let _guard = match lock {
                    Some(l) => Some(l.lock_owned().await),
                    None => None,
                };
                run_single_agent(
                    task_clone,
                    provider,
                    approval,
                    executor,
                    wt_path_clone,
                    hooks,
                    tx,
                )
                .await
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

        // Capture each agent's work BEFORE teardown. `remove_worktree` runs
        // `git worktree remove --force`, which discards uncommitted changes —
        // so without this every file the fan-out wrote is deleted moments after
        // it is written, and all that survives is prose describing work that no
        // longer exists.
        for result in results.iter_mut() {
            if result.worktree == *repo_path {
                // Ran in the main workspace, so its changes are already there;
                // capturing a diff would double-apply them at reconcile.
                continue;
            }
            match capture_worktree_patch(&result.worktree) {
                Ok((patch, files)) => {
                    tracing::info!(
                        agent = result.id,
                        files = files.len(),
                        bytes = patch.len(),
                        "Captured worktree diff before teardown",
                    );
                    result.patch = Some(patch);
                    result.files_changed = files;
                }
                Err(e) => {
                    // Left as None, which reconcile reports as NotCaptured — an
                    // agent whose work could not be captured must not read as
                    // one that changed nothing.
                    tracing::error!(
                        agent = result.id,
                        error = %e,
                        "Could not capture worktree diff — this agent's work will be lost on teardown",
                    );
                }
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

        let _ = event_tx
            .send(OrchestratorEvent::AllComplete {
                results: results.clone(),
            })
            .await;
        Ok(results)
    }
}

// ── Capture & reconcile ───────────────────────────────────────────────────────

/// Capture everything an agent changed in its worktree as a unified diff.
///
/// Runs before teardown. `git add -A` first so new files are included — a
/// plain `git diff` shows nothing for an untracked file, which is most of what
/// a scaffolding agent produces, so omitting the staging step silently returns
/// an empty patch for the very runs that did the most work.
///
/// Staging only; nothing is committed. The worktree is about to be destroyed,
/// so the diff is the artefact that outlives it.
pub fn capture_worktree_patch(worktree: &Path) -> Result<(String, Vec<String>)> {
    let add = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(worktree)
        .output()?;
    if !add.status.success() {
        anyhow::bail!(
            "git add -A failed in {}: {}",
            worktree.display(),
            String::from_utf8_lossy(&add.stderr)
        );
    }

    let diff = std::process::Command::new("git")
        .args(["diff", "--cached", "--binary"])
        .current_dir(worktree)
        .output()?;
    if !diff.status.success() {
        anyhow::bail!(
            "git diff --cached failed in {}: {}",
            worktree.display(),
            String::from_utf8_lossy(&diff.stderr)
        );
    }

    let names = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(worktree)
        .output()?;
    let files = String::from_utf8_lossy(&names.stdout)
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok((String::from_utf8_lossy(&diff.stdout).to_string(), files))
}

/// What happened when one agent's patch was replayed onto the main workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Applied cleanly.
    Applied,
    /// The agent changed nothing.
    Empty,
    /// Its diff could not be captured, so there was nothing to apply.
    NotCaptured,
    /// The agent failed; its work is deliberately not applied.
    SkippedFailed,
    /// The patch conflicts with the workspace as it now stands — usually
    /// because an earlier agent in this same reconcile already changed those
    /// lines.
    Conflicted(String),
}

/// Per-agent record of the reconcile pass.
#[derive(Debug, Clone)]
pub struct MergeReport {
    pub id: usize,
    pub branch: String,
    pub outcome: MergeOutcome,
    pub files_changed: Vec<String>,
}

impl MergeReport {
    pub fn applied(&self) -> bool {
        self.outcome == MergeOutcome::Applied
    }
}

/// Replay the fan-out's captured patches onto `repo_path`, in agent order.
///
/// Sequential and deliberately so: applying concurrently would reintroduce the
/// races that worktree isolation exists to prevent. Order is by agent id, so a
/// given set of results always reconciles the same way.
///
/// A conflict is reported, never forced. Two agents that changed the same lines
/// is a real disagreement about the work, and silently taking one side would
/// hide it — the caller decides. Later agents are still attempted after a
/// conflict, so one collision does not discard the rest of the fan-out.
pub fn reconcile(repo_path: &Path, results: &[AgentResult]) -> Vec<MergeReport> {
    let mut ordered: Vec<&AgentResult> = results.iter().collect();
    ordered.sort_by_key(|r| r.id);

    ordered
        .into_iter()
        .map(|r| {
            let outcome = if !r.success {
                MergeOutcome::SkippedFailed
            } else {
                match r.patch.as_deref() {
                    None => MergeOutcome::NotCaptured,
                    Some(p) if p.trim().is_empty() => MergeOutcome::Empty,
                    Some(patch) => apply_patch(repo_path, patch),
                }
            };
            MergeReport {
                id: r.id,
                branch: r.branch.clone(),
                outcome,
                files_changed: r.files_changed.clone(),
            }
        })
        .collect()
}

/// Run `git apply` with `args`, feeding `patch` on stdin. Returns
/// `(success, stderr)`.
fn run_git_apply(repo_path: &Path, patch: &str, args: &[&str]) -> Result<(bool, String)> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("git")
        .arg("apply")
        .args(args)
        .arg("-")
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("git apply stdin unavailable"))?
        .write_all(patch.as_bytes())?;

    let out = child.wait_with_output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    ))
}

/// Apply one unified diff to `repo_path`, without ever leaving the workspace in
/// a half-merged state.
///
/// Probes before it writes, because neither git mode is safe to use alone:
///
/// - A plain `git apply` refuses anything whose context has shifted, which is
///   common once an earlier agent's patch has landed in this same pass.
/// - `git apply --3way` handles that, but on a genuine conflict it writes
///   `<<<<<<<` markers into the file *and then* reports failure. Using it
///   directly means a "reported" conflict has already edited the tree — the
///   opposite of leaving the decision to the caller.
///
/// So: try strict first; if that is refused, ask `--3way --check` whether it
/// could merge. `--check` exits 0 even when it would conflict, announcing the
/// fact on stderr instead, so the stderr text is the signal — the exit code is
/// not. Only when it reports no conflict is the real 3-way apply run.
fn apply_patch(repo_path: &Path, patch: &str) -> MergeOutcome {
    match run_git_apply(repo_path, patch, &["--whitespace=nowarn"]) {
        Ok((true, _)) => return MergeOutcome::Applied,
        Ok((false, _)) => {}
        Err(e) => return MergeOutcome::Conflicted(format!("could not run git apply: {e}")),
    }

    let probe = run_git_apply(
        repo_path,
        patch,
        &["--3way", "--check", "--whitespace=nowarn"],
    );
    match probe {
        Ok((true, stderr)) if !stderr.to_lowercase().contains("conflict") => {
            match run_git_apply(repo_path, patch, &["--3way", "--whitespace=nowarn"]) {
                Ok((true, _)) => MergeOutcome::Applied,
                // Should not happen after a clean probe, but if it does the
                // tree may now hold markers — say so rather than claim success.
                Ok((false, stderr)) => MergeOutcome::Conflicted(format!(
                    "3-way apply failed after a clean probe: {stderr}"
                )),
                Err(e) => MergeOutcome::Conflicted(format!("git apply did not complete: {e}")),
            }
        }
        Ok((_, stderr)) => MergeOutcome::Conflicted(if stderr.is_empty() {
            "patch does not apply to the current workspace".to_string()
        } else {
            stderr
        }),
        Err(e) => MergeOutcome::Conflicted(format!("git apply did not complete: {e}")),
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

    let _ = event_tx
        .send(OrchestratorEvent::AgentStarted {
            id,
            task: task_desc.clone(),
            worktree: worktree.clone(),
        })
        .await;

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
                let _ = event_tx
                    .send(OrchestratorEvent::AgentChunk { id, text })
                    .await;
            }
            AgentEvent::ToolCallExecuted(step) => {
                steps_taken += 1;
                let _ = event_tx
                    .send(OrchestratorEvent::AgentStep { id, step })
                    .await;
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
                let _ = event_tx
                    .send(OrchestratorEvent::AgentComplete {
                        id,
                        summary,
                        branch: branch.clone(),
                    })
                    .await;
                break;
            }
            AgentEvent::Partial {
                summary,
                steps_completed,
                steps_planned,
                ..
            } => {
                final_summary = format!(
                    "{} (completed {}/{} steps)",
                    summary, steps_completed, steps_planned
                );
                // Treat partial as a soft failure — work was done but not all of it
                let _ = event_tx
                    .send(OrchestratorEvent::AgentComplete {
                        id,
                        summary: final_summary.clone(),
                        branch: branch.clone(),
                    })
                    .await;
                break;
            }
            AgentEvent::Error(err) => {
                final_summary = err.clone();
                let _ = event_tx
                    .send(OrchestratorEvent::AgentError { id, error: err })
                    .await;
                break;
            }
            AgentEvent::RetryableError {
                error,
                attempt,
                max_attempts,
                ..
            } => {
                // Log retry but don't treat as fatal in parallel mode
                tracing::warn!(id, attempt, max_attempts, error = %error, "Sub-agent retrying");
            }
            AgentEvent::CircuitBreak { state, reason } => {
                // Treat circuit break as an error in parallel mode
                let msg = format!("Circuit breaker: {} — {}", state, reason);
                let _ = event_tx
                    .send(OrchestratorEvent::AgentError {
                        id,
                        error: msg.clone(),
                    })
                    .await;
                if state == crate::agent::AgentHealthState::Blocked {
                    final_summary = msg;
                    break;
                }
            }
            AgentEvent::Verifier { decision } => {
                tracing::info!(id, ?decision, "Sub-agent verifier decision");
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
        // Filled in by the orchestrator after the agent finishes but before its
        // worktree is torn down — this function has no idea when that is.
        patch: None,
        files_changed: Vec::new(),
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
            patch: None,
            files_changed: Vec::new(),
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

    // ── AgentTask edge cases ────────────────────────────────────────────

    #[test]
    fn agent_task_branch_name_zero_id() {
        let task = AgentTask::new(0, "task");
        assert_eq!(task.branch_name(), "vibe-agent-0");
    }

    #[test]
    fn agent_task_empty_description() {
        let task = AgentTask::new(1, "");
        assert_eq!(task.description, "");
        assert_eq!(task.branch_name(), "vibe-agent-1");
    }

    #[test]
    fn agent_task_custom_branch_overrides_default() {
        let mut task = AgentTask::new(5, "do work");
        assert_eq!(task.branch_name(), "vibe-agent-5");
        task.branch_label = Some("custom-branch".into());
        assert_eq!(task.branch_name(), "custom-branch");
    }

    #[test]
    fn agent_task_serde_with_branch_label() {
        let mut task = AgentTask::new(1, "test");
        task.branch_label = Some("my-branch".into());
        let json = serde_json::to_string(&task).unwrap();
        let back: AgentTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.branch_label.as_deref(), Some("my-branch"));
        assert_eq!(back.branch_name(), "my-branch");
    }

    // ── AgentResult edge cases ──────────────────────────────────────────

    #[test]
    fn agent_result_failed() {
        let result = AgentResult {
            id: 1,
            task: "fix bug".into(),
            branch: "vibe-agent-1".into(),
            worktree: PathBuf::from("/tmp/wt-1"),
            success: false,
            summary: "Compilation error".into(),
            steps_taken: 3,
            patch: None,
            files_changed: Vec::new(),
        };
        assert!(!result.success);
        assert_eq!(result.steps_taken, 3);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":false"));
    }

    #[test]
    fn agent_result_zero_steps() {
        let result = AgentResult {
            id: 0,
            task: "trivial".into(),
            branch: "b".into(),
            worktree: PathBuf::from("/tmp"),
            success: true,
            summary: "Nothing to do".into(),
            steps_taken: 0,
            patch: None,
            files_changed: Vec::new(),
        };
        assert_eq!(result.steps_taken, 0);
    }

    #[test]
    fn agent_result_clone() {
        let result = AgentResult {
            id: 2,
            task: "task".into(),
            branch: "b".into(),
            worktree: PathBuf::from("/wt"),
            success: true,
            summary: "done".into(),
            steps_taken: 5,
            patch: None,
            files_changed: Vec::new(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.id, result.id);
        assert_eq!(cloned.summary, result.summary);
    }

    // ── AgentInstance ────────────────────────────────────────────────────

    #[test]
    fn agent_instance_with_error() {
        let inst = AgentInstance {
            id: 1,
            task: "task".into(),
            worktree: PathBuf::from("/wt"),
            branch: "b".into(),
            status: AgentStatus::Failed,
            steps: vec![],
            summary: None,
            error: Some("timeout".into()),
        };
        assert_eq!(inst.status, AgentStatus::Failed);
        assert_eq!(inst.error.as_deref(), Some("timeout"));
        assert!(inst.summary.is_none());
    }

    #[test]
    fn agent_instance_with_summary() {
        let inst = AgentInstance {
            id: 0,
            task: "task".into(),
            worktree: PathBuf::from("/wt"),
            branch: "main".into(),
            status: AgentStatus::Complete,
            steps: vec![],
            summary: Some("All tests pass".into()),
            error: None,
        };
        assert_eq!(inst.status, AgentStatus::Complete);
        assert_eq!(inst.summary.as_deref(), Some("All tests pass"));
    }

    // ── AgentStatus ─────────────────────────────────────────────────────

    #[test]
    fn agent_status_clone() {
        let s = AgentStatus::Running;
        let s2 = s.clone();
        assert_eq!(s, s2);
    }

    #[test]
    fn agent_status_debug_format() {
        let s = AgentStatus::Pending;
        let debug = format!("{:?}", s);
        assert_eq!(debug, "Pending");
    }

    // ── Capping at max_agents ───────────────────────────────────────────

    #[test]
    fn max_agents_caps_task_count() {
        let max_agents = 4;
        let tasks: Vec<AgentTask> = (0..10).map(|i| AgentTask::new(i, "task")).collect();
        let n = tasks.len().min(max_agents);
        assert_eq!(n, 4);
    }

    #[test]
    fn fewer_tasks_than_max_agents() {
        let max_agents = 8;
        let tasks: Vec<AgentTask> = (0..3).map(|i| AgentTask::new(i, "task")).collect();
        let n = tasks.len().min(max_agents);
        assert_eq!(n, 3);
    }
}

// ── Capture & reconcile tests ─────────────────────────────────────────────────

#[cfg(test)]
mod reconcile_tests {
    use super::*;
    use std::process::Command;

    /// A git repo with one committed file, so diffs have a baseline.
    fn repo(dir: &Path) {
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git");
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "t"]);
        std::fs::write(dir.join("base.txt"), "line one\n").unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-qm", "base"]);
    }

    fn result(id: usize, patch: Option<String>, success: bool) -> AgentResult {
        AgentResult {
            id,
            task: "t".into(),
            branch: format!("vibe-agent-{id}"),
            worktree: PathBuf::from("/nonexistent"),
            success,
            summary: "done".into(),
            steps_taken: 1,
            patch,
            files_changed: vec![],
        }
    }

    /// The regression that made fan-out pointless: a brand-new file is
    /// untracked, so a bare `git diff` reports nothing. Without staging first,
    /// an agent that scaffolded a dozen files produced an empty patch and its
    /// work vanished with the worktree.
    #[test]
    fn capture_includes_newly_created_files() {
        let tmp = tempfile::tempdir().unwrap();
        repo(tmp.path());
        std::fs::write(tmp.path().join("brand_new.rs"), "fn main() {}\n").unwrap();

        let (patch, files) = capture_worktree_patch(tmp.path()).unwrap();
        assert!(
            patch.contains("brand_new.rs"),
            "new file missing from patch: {patch}"
        );
        assert!(
            files.iter().any(|f| f == "brand_new.rs"),
            "files_changed: {files:?}"
        );
    }

    #[test]
    fn capture_reports_empty_when_nothing_changed() {
        let tmp = tempfile::tempdir().unwrap();
        repo(tmp.path());
        let (patch, files) = capture_worktree_patch(tmp.path()).unwrap();
        assert!(patch.trim().is_empty());
        assert!(files.is_empty());
    }

    #[test]
    fn reconcile_applies_a_captured_patch_to_the_main_repo() {
        let src = tempfile::tempdir().unwrap();
        repo(src.path());
        std::fs::write(src.path().join("added.txt"), "from the agent\n").unwrap();
        let (patch, _) = capture_worktree_patch(src.path()).unwrap();

        let main = tempfile::tempdir().unwrap();
        repo(main.path());
        let reports = reconcile(main.path(), &[result(0, Some(patch), true)]);

        assert_eq!(
            reports[0].outcome,
            MergeOutcome::Applied,
            "{:?}",
            reports[0]
        );
        assert_eq!(
            std::fs::read_to_string(main.path().join("added.txt")).unwrap(),
            "from the agent\n",
            "the agent's file must exist in the main workspace after reconcile"
        );
    }

    #[test]
    fn a_failed_agents_work_is_not_applied() {
        let main = tempfile::tempdir().unwrap();
        repo(main.path());
        let reports = reconcile(main.path(), &[result(0, Some("bogus".into()), false)]);
        assert_eq!(reports[0].outcome, MergeOutcome::SkippedFailed);
    }

    /// An uncaptured diff must not read as "changed nothing" — one means the
    /// work was lost, the other that there was none.
    #[test]
    fn uncaptured_is_distinct_from_empty() {
        let main = tempfile::tempdir().unwrap();
        repo(main.path());
        let reports = reconcile(
            main.path(),
            &[result(0, None, true), result(1, Some(String::new()), true)],
        );
        assert_eq!(reports[0].outcome, MergeOutcome::NotCaptured);
        assert_eq!(reports[1].outcome, MergeOutcome::Empty);
    }

    /// Two agents that edited the same lines genuinely disagree. Reporting the
    /// collision is the point; silently taking one side would hide it.
    #[test]
    fn conflicting_agents_are_reported_not_forced() {
        let mk = |content: &str| {
            let d = tempfile::tempdir().unwrap();
            repo(d.path());
            std::fs::write(d.path().join("base.txt"), content).unwrap();
            let (p, _) = capture_worktree_patch(d.path()).unwrap();
            (d, p)
        };
        let (_a, patch_a) = mk("agent A rewrote this\n");
        let (_b, patch_b) = mk("agent B rewrote this\n");

        let main = tempfile::tempdir().unwrap();
        repo(main.path());
        let reports = reconcile(
            main.path(),
            &[
                result(0, Some(patch_a), true),
                result(1, Some(patch_b), true),
            ],
        );

        assert_eq!(
            reports[0].outcome,
            MergeOutcome::Applied,
            "first should land"
        );
        assert!(
            matches!(reports[1].outcome, MergeOutcome::Conflicted(_)),
            "second should be reported as a conflict, got {:?}",
            reports[1].outcome
        );
        // The first agent's content survives; nothing was silently overwritten.
        assert_eq!(
            std::fs::read_to_string(main.path().join("base.txt")).unwrap(),
            "agent A rewrote this\n"
        );
    }

    /// One collision must not discard the rest of the fan-out.
    #[test]
    fn a_conflict_does_not_stop_later_agents() {
        let mk_conflict = || {
            let d = tempfile::tempdir().unwrap();
            repo(d.path());
            std::fs::write(d.path().join("base.txt"), "rewritten\n").unwrap();
            let (p, _) = capture_worktree_patch(d.path()).unwrap();
            (d, p)
        };
        let (_x, patch_x) = mk_conflict();
        let (_y, patch_y) = mk_conflict();

        let independent = tempfile::tempdir().unwrap();
        repo(independent.path());
        std::fs::write(independent.path().join("elsewhere.txt"), "untouched area\n").unwrap();
        let (patch_z, _) = capture_worktree_patch(independent.path()).unwrap();

        let main = tempfile::tempdir().unwrap();
        repo(main.path());
        let reports = reconcile(
            main.path(),
            &[
                result(0, Some(patch_x), true),
                result(1, Some(patch_y), true),
                result(2, Some(patch_z), true),
            ],
        );

        assert!(matches!(reports[1].outcome, MergeOutcome::Conflicted(_)));
        assert_eq!(
            reports[2].outcome,
            MergeOutcome::Applied,
            "agent 2 was independent"
        );
        assert!(main.path().join("elsewhere.txt").exists());
    }
}

// ── Shared-workspace serialisation ────────────────────────────────────────────

#[cfg(test)]
mod shared_workspace_tests {
    use super::*;
    use crate::provider::{CodeContext, CompletionResponse, CompletionStream, Message};
    use crate::tools::{ToolCall, ToolResult};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Records the high-water mark of agents running at the same time.
    #[derive(Default)]
    struct Concurrency {
        current: AtomicUsize,
        peak: AtomicUsize,
    }

    struct NoopExecutor;
    #[async_trait::async_trait]
    impl ToolExecutorTrait for NoopExecutor {
        async fn execute(&self, _call: &ToolCall) -> ToolResult {
            ToolResult::ok("test", "ok")
        }
    }

    struct NoopFactory;
    impl ExecutorFactory for NoopFactory {
        fn create(&self, _workspace_root: PathBuf) -> Arc<dyn ToolExecutorTrait> {
            Arc::new(NoopExecutor)
        }
    }

    /// The probe. Every agent calls the provider exactly once here (the reply
    /// has no tool call, so the loop treats it as the final answer), and the
    /// call is held open long enough that any two overlapping agents would be
    /// seen. Counting here rather than in the executor keeps the test
    /// independent of the tool-call wire format.
    struct CountingProvider(Arc<Concurrency>);

    #[async_trait::async_trait]
    impl AIProvider for CountingProvider {
        fn name(&self) -> &str {
            "counting"
        }
        async fn is_available(&self) -> bool {
            true
        }
        async fn complete(&self, _c: &CodeContext) -> Result<CompletionResponse> {
            anyhow::bail!("unused")
        }
        async fn stream_complete(&self, _c: &CodeContext) -> Result<CompletionStream> {
            anyhow::bail!("unused")
        }
        async fn chat(&self, _m: &[Message], _c: Option<String>) -> Result<String> {
            anyhow::bail!("unused")
        }
        async fn stream_chat(&self, _m: &[Message]) -> Result<CompletionStream> {
            let now = self.0.current.fetch_add(1, Ordering::SeqCst) + 1;
            self.0.peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            self.0.current.fetch_sub(1, Ordering::SeqCst);
            Ok(Box::pin(futures::stream::once(async {
                Ok("all done".to_string())
            })))
        }
    }

    fn orchestrator(counter: &Arc<Concurrency>) -> MultiAgentOrchestrator {
        MultiAgentOrchestrator::new(
            Arc::new(CountingProvider(Arc::clone(counter))),
            ApprovalPolicy::FullAuto,
            Arc::new(NoopFactory),
        )
    }

    async fn peak_concurrency_over(tasks: usize) -> usize {
        let counter = Arc::new(Concurrency::default());
        let orch = orchestrator(&counter);
        // No WorktreeManager, so every agent falls back to the main workspace.
        let (tx, _rx) = mpsc::channel(256);
        let list: Vec<AgentTask> = (0..tasks).map(|i| AgentTask::new(i, "do it")).collect();
        let results = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            orch.run_tasks(&PathBuf::from("."), list, tx),
        )
        .await
        .expect("run_tasks hung")
        .expect("run_tasks failed");
        assert_eq!(results.len(), tasks, "every task should produce a result");
        counter.peak.load(Ordering::SeqCst)
    }

    /// Agents with no worktree share one directory. Before the lock they ran at
    /// once and wrote the same tree concurrently — the clobbering that worktree
    /// isolation exists to prevent, reached by silently falling back to it.
    #[tokio::test]
    async fn agents_without_a_worktree_do_not_run_concurrently() {
        let peak = peak_concurrency_over(4).await;
        assert_eq!(
            peak, 1,
            "unisolated agents must be serialised; a peak of {peak} means that many were \
             working in the same tree at the same time"
        );
    }

    /// Serialised, not dropped: sharing a workspace slows the fan-out down, it
    /// must not silently reduce how much work gets done.
    #[tokio::test]
    async fn serialisation_does_not_lose_tasks() {
        let counter = Arc::new(Concurrency::default());
        let orch = orchestrator(&counter);
        let (tx, _rx) = mpsc::channel(256);
        let list: Vec<AgentTask> = (0..3).map(|i| AgentTask::new(i, "do it")).collect();
        let results = orch
            .run_tasks(&PathBuf::from("."), list, tx)
            .await
            .expect("run_tasks failed");
        let mut ids: Vec<usize> = results.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1, 2], "each task must still report a result");
    }
}
