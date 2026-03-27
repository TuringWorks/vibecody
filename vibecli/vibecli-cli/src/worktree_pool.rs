//! Parallel git worktree agent execution pool for VibeCody.
//!
//! Enables multiple AI agents to work simultaneously on different tasks,
//! each in its own git worktree. The pool manages lifecycle, progress tracking,
//! merge orchestration, PR generation, and cleanup of worktree-based agents.
//!
//! REPL commands: `/worktree-pool spawn|list|status|complete|fail|cancel|cleanup|merge|pr|metrics`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// === Configuration ===

/// Configuration for the worktree pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Maximum number of concurrent worktrees.
    pub max_worktrees: usize,
    /// Base branch to create worktrees from.
    pub base_branch: String,
    /// Prefix for worktree branch names.
    pub worktree_prefix: String,
    /// Whether to auto-cleanup completed worktrees.
    pub auto_cleanup: bool,
    /// Optional resource limits per agent.
    pub resource_limits: Option<ResourceLimits>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            max_worktrees: 4,
            base_branch: "main".to_string(),
            worktree_prefix: "vibe-wt-".to_string(),
            auto_cleanup: false,
            resource_limits: None,
        }
    }
}

/// Resource limits applied per worktree agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in megabytes.
    pub max_memory_mb: Option<u64>,
    /// Maximum CPU usage percentage (0-100).
    pub max_cpu_percent: Option<u32>,
    /// Maximum execution time in seconds.
    pub max_time_secs: Option<u64>,
}

// === Worktree Agent State ===

/// State of a worktree agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorktreeState {
    /// Worktree is being created.
    Creating,
    /// Worktree is ready for work.
    Ready,
    /// Agent is running in the worktree.
    Running,
    /// Agent is merging results back.
    Merging,
    /// Agent completed successfully.
    Completed,
    /// Agent failed with the given reason.
    Failed(String),
    /// Worktree is being cleaned up.
    Cleaning,
}

impl std::fmt::Display for WorktreeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::Merging => write!(f, "merging"),
            Self::Completed => write!(f, "completed"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
            Self::Cleaning => write!(f, "cleaning"),
        }
    }
}

// === Worktree Agent ===

/// A single agent working in its own git worktree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeAgent {
    /// Unique identifier for the agent.
    pub id: String,
    /// Filesystem path of the worktree.
    pub worktree_path: String,
    /// Git branch name for this worktree.
    pub branch_name: String,
    /// Current state of the agent.
    pub state: WorktreeState,
    /// Description of the task being performed.
    pub task_description: String,
    /// Unix timestamp of creation.
    pub created_at: u64,
    /// Unix timestamp of last update.
    pub updated_at: u64,
    /// Progress percentage (0-100).
    pub progress_percent: u8,
    /// Files changed by this agent.
    pub files_changed: Vec<String>,
    /// Commits made by this agent.
    pub commits: Vec<String>,
}

impl WorktreeAgent {
    /// Returns true if the agent is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            WorktreeState::Completed | WorktreeState::Failed(_) | WorktreeState::Cleaning
        )
    }

    /// Returns true if the agent is actively running.
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            WorktreeState::Creating
                | WorktreeState::Ready
                | WorktreeState::Running
                | WorktreeState::Merging
        )
    }
}

// === Worktree Pool ===

/// Pool managing multiple worktree agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreePool {
    /// Pool configuration.
    pub config: WorktreeConfig,
    /// Active and historical agents.
    pub agents: HashMap<String, WorktreeAgent>,
    /// Root path of the git repository.
    pub repo_root: String,
    /// Queue of agent IDs awaiting merge.
    pub merge_queue: Vec<String>,
    /// Counter for generating unique IDs.
    next_id: u64,
}

impl WorktreePool {
    /// Create a new worktree pool.
    pub fn new(repo_root: &str, config: WorktreeConfig) -> Self {
        Self {
            config,
            agents: HashMap::new(),
            repo_root: repo_root.to_string(),
            merge_queue: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawn a new worktree agent for the given task. Returns the agent ID.
    pub fn spawn_agent(&mut self, task: &str) -> Result<String, String> {
        if task.trim().is_empty() {
            return Err("Task description cannot be empty".to_string());
        }

        let active = self.active_count();
        if active >= self.config.max_worktrees {
            return Err(format!(
                "Pool at capacity: {}/{} worktrees active",
                active, self.config.max_worktrees
            ));
        }

        let id = format!("wt-{:04}", self.next_id);
        self.next_id += 1;

        let branch_name = format!(
            "{}{}",
            self.config.worktree_prefix,
            id.replace('-', "_")
        );
        let worktree_path = format!("{}/.worktrees/{}", self.repo_root, id);

        let now = current_timestamp();

        let agent = WorktreeAgent {
            id: id.clone(),
            worktree_path,
            branch_name,
            state: WorktreeState::Creating,
            task_description: task.to_string(),
            created_at: now,
            updated_at: now,
            progress_percent: 0,
            files_changed: Vec::new(),
            commits: Vec::new(),
        };

        self.agents.insert(id.clone(), agent);
        Ok(id)
    }

    /// Get a reference to an agent by ID.
    pub fn get_agent(&self, id: &str) -> Option<&WorktreeAgent> {
        self.agents.get(id)
    }

    /// Get a mutable reference to an agent by ID.
    pub fn get_agent_mut(&mut self, id: &str) -> Option<&mut WorktreeAgent> {
        self.agents.get_mut(id)
    }

    /// List all agents in the pool.
    pub fn list_agents(&self) -> Vec<&WorktreeAgent> {
        let mut agents: Vec<&WorktreeAgent> = self.agents.values().collect();
        agents.sort_by_key(|a| a.created_at);
        agents
    }

    /// Count active (non-terminal) agents.
    pub fn active_count(&self) -> usize {
        self.agents.values().filter(|a| a.is_active()).count()
    }

    /// Update progress of an agent.
    pub fn update_progress(&mut self, id: &str, percent: u8, files: Vec<String>) {
        if let Some(agent) = self.agents.get_mut(id) {
            agent.progress_percent = percent.min(100);
            agent.files_changed = files;
            agent.updated_at = current_timestamp();
            if agent.state == WorktreeState::Creating || agent.state == WorktreeState::Ready {
                agent.state = WorktreeState::Running;
            }
        }
    }

    /// Mark an agent as completed with the given commits.
    pub fn complete_agent(&mut self, id: &str, commits: Vec<String>) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.is_terminal() {
            return Err(format!("Agent '{}' is already in terminal state: {}", id, agent.state));
        }

        agent.state = WorktreeState::Completed;
        agent.progress_percent = 100;
        agent.commits = commits;
        agent.updated_at = current_timestamp();

        self.merge_queue.push(id.to_string());
        Ok(())
    }

    /// Mark an agent as failed.
    pub fn fail_agent(&mut self, id: &str, reason: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.is_terminal() {
            return Err(format!("Agent '{}' is already in terminal state: {}", id, agent.state));
        }

        agent.state = WorktreeState::Failed(reason.to_string());
        agent.updated_at = current_timestamp();
        Ok(())
    }

    /// Cancel an active agent.
    pub fn cancel_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.is_terminal() {
            return Err(format!("Agent '{}' is already in terminal state: {}", id, agent.state));
        }

        agent.state = WorktreeState::Failed("canceled by user".to_string());
        agent.updated_at = current_timestamp();
        Ok(())
    }

    /// Cleanup a single agent's worktree.
    pub fn cleanup_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.is_active() && !matches!(agent.state, WorktreeState::Creating) {
            // Allow cleanup of Creating state in case setup failed
            if matches!(agent.state, WorktreeState::Running | WorktreeState::Merging) {
                return Err(format!(
                    "Cannot cleanup active agent '{}' in state: {}",
                    id, agent.state
                ));
            }
        }

        agent.state = WorktreeState::Cleaning;
        agent.updated_at = current_timestamp();

        // Remove from merge queue if present
        self.merge_queue.retain(|qid| qid != id);
        Ok(())
    }

    /// Cleanup all completed and failed agents. Returns count of agents cleaned.
    pub fn cleanup_completed(&mut self) -> usize {
        let ids_to_clean: Vec<String> = self
            .agents
            .iter()
            .filter(|(_, a)| matches!(a.state, WorktreeState::Completed | WorktreeState::Failed(_)))
            .map(|(id, _)| id.clone())
            .collect();

        let count = ids_to_clean.len();
        for id in &ids_to_clean {
            if let Some(agent) = self.agents.get_mut(id) {
                agent.state = WorktreeState::Cleaning;
                agent.updated_at = current_timestamp();
            }
            self.merge_queue.retain(|qid| qid != id);
        }
        count
    }

    /// Compute pool-level metrics.
    pub fn metrics(&self) -> WorktreePoolMetrics {
        let mut total_spawned = 0u64;
        let mut total_completed = 0u64;
        let mut total_failed = 0u64;
        let mut total_canceled = 0u64;
        let mut total_files_changed = 0usize;
        let mut total_commits = 0usize;
        let mut completion_times: Vec<u64> = Vec::new();

        for agent in self.agents.values() {
            total_spawned += 1;
            total_files_changed += agent.files_changed.len();
            total_commits += agent.commits.len();

            match &agent.state {
                WorktreeState::Completed | WorktreeState::Cleaning => {
                    if agent.commits.is_empty()
                        && matches!(agent.state, WorktreeState::Cleaning)
                        && agent.progress_percent < 100
                    {
                        // Was likely failed/canceled then cleaned
                        if agent
                            .task_description
                            .contains("canceled")
                        {
                            total_canceled += 1;
                        } else {
                            total_failed += 1;
                        }
                    } else {
                        total_completed += 1;
                        if agent.updated_at >= agent.created_at {
                            completion_times.push(agent.updated_at - agent.created_at);
                        }
                    }
                }
                WorktreeState::Failed(reason) => {
                    if reason == "canceled by user" {
                        total_canceled += 1;
                    } else {
                        total_failed += 1;
                    }
                }
                _ => {}
            }
        }

        let avg_completion_secs = if completion_times.is_empty() {
            0.0
        } else {
            completion_times.iter().sum::<u64>() as f64 / completion_times.len() as f64
        };

        WorktreePoolMetrics {
            total_spawned,
            total_completed,
            total_failed,
            total_canceled,
            avg_completion_secs,
            total_files_changed,
            total_commits,
        }
    }
}

// === Merge Orchestrator ===

/// Strategy for merging completed worktree agents back to the base branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Merge agents one at a time in order.
    Sequential,
    /// Merge all in parallel, then resolve conflicts.
    ParallelThenResolve,
    /// Create a single combined PR from all agents.
    CombinedPR,
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequential => write!(f, "sequential"),
            Self::ParallelThenResolve => write!(f, "parallel-then-resolve"),
            Self::CombinedPR => write!(f, "combined-pr"),
        }
    }
}

/// Type of file conflict between two agents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// Both agents modified the same file.
    BothModified,
    /// One agent deleted a file the other modified.
    DeletedVsModified,
    /// Both agents added the same file.
    AddedInBoth,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BothModified => write!(f, "both-modified"),
            Self::DeletedVsModified => write!(f, "deleted-vs-modified"),
            Self::AddedInBoth => write!(f, "added-in-both"),
        }
    }
}

/// A detected file conflict between two worktree agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileConflict {
    /// Path of the conflicting file.
    pub file_path: String,
    /// Type of conflict.
    pub conflict_type: ConflictType,
}

/// Orchestrates merging of completed worktree agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeOrchestrator {
    /// Selected merge strategy.
    pub strategy: MergeStrategy,
}

impl MergeOrchestrator {
    /// Create a new merge orchestrator with the given strategy.
    pub fn new(strategy: MergeStrategy) -> Self {
        Self { strategy }
    }

    /// Plan merge order based on completion time (earliest first).
    pub fn plan_merge_order(agents: &[&WorktreeAgent]) -> Vec<String> {
        let mut sorted: Vec<&&WorktreeAgent> = agents
            .iter()
            .filter(|a| matches!(a.state, WorktreeState::Completed))
            .collect();
        sorted.sort_by_key(|a| a.updated_at);
        sorted.iter().map(|a| a.id.clone()).collect()
    }

    /// Detect file conflicts between two agents.
    pub fn detect_conflicts(
        agent_a: &WorktreeAgent,
        agent_b: &WorktreeAgent,
    ) -> Vec<FileConflict> {
        let mut conflicts = Vec::new();
        let files_a: std::collections::HashSet<&String> =
            agent_a.files_changed.iter().collect();
        let files_b: std::collections::HashSet<&String> =
            agent_b.files_changed.iter().collect();

        for file in files_a.intersection(&files_b) {
            conflicts.push(FileConflict {
                file_path: (*file).clone(),
                conflict_type: ConflictType::BothModified,
            });
        }

        conflicts.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        conflicts
    }
}

// === PR Generator ===

/// Mode for generating pull requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrMode {
    /// One PR per worktree agent.
    PerWorktree,
    /// A single combined PR for all agents.
    Combined,
}

/// Generates pull requests from worktree agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrGenerator {
    /// PR generation mode.
    pub mode: PrMode,
}

impl PrGenerator {
    /// Create a new PR generator.
    pub fn new(mode: PrMode) -> Self {
        Self { mode }
    }

    /// Generate a PR title for a single agent.
    pub fn generate_pr_title(agent: &WorktreeAgent) -> String {
        let desc = &agent.task_description;
        let truncated = if desc.len() > 60 {
            format!("{}...", &desc[..57])
        } else {
            desc.clone()
        };
        format!("[{}] {}", agent.id, truncated)
    }

    /// Generate a PR body from one or more agents.
    pub fn generate_pr_body(agents: &[&WorktreeAgent]) -> String {
        let mut body = String::new();
        body.push_str("## Worktree Agent Summary\n\n");

        if agents.len() == 1 {
            let agent = agents[0];
            body.push_str(&format!("**Task:** {}\n\n", agent.task_description));
            body.push_str(&format!("**Agent:** `{}`\n", agent.id));
            body.push_str(&format!("**Branch:** `{}`\n", agent.branch_name));
            body.push_str(&format!(
                "**Files changed:** {}\n",
                agent.files_changed.len()
            ));
            body.push_str(&format!("**Commits:** {}\n", agent.commits.len()));
            if !agent.files_changed.is_empty() {
                body.push_str("\n### Changed files\n");
                for f in &agent.files_changed {
                    body.push_str(&format!("- `{}`\n", f));
                }
            }
        } else {
            body.push_str(&format!(
                "Combined PR from **{}** worktree agents.\n\n",
                agents.len()
            ));
            body.push_str("| Agent | Task | Files | Commits |\n");
            body.push_str("|-------|------|-------|---------|\n");
            for agent in agents {
                let task_short = if agent.task_description.len() > 40 {
                    format!("{}...", &agent.task_description[..37])
                } else {
                    agent.task_description.clone()
                };
                body.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    agent.id,
                    task_short,
                    agent.files_changed.len(),
                    agent.commits.len()
                ));
            }

            let total_files: usize = agents.iter().map(|a| a.files_changed.len()).sum();
            let total_commits: usize = agents.iter().map(|a| a.commits.len()).sum();
            body.push_str(&format!(
                "\n**Total:** {} files changed, {} commits\n",
                total_files, total_commits
            ));
        }

        body.push_str("\n---\n_Generated by VibeCody Worktree Pool_\n");
        body
    }
}

// === Metrics ===

/// Pool-level metrics for worktree agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreePoolMetrics {
    /// Total agents ever spawned.
    pub total_spawned: u64,
    /// Total agents that completed successfully.
    pub total_completed: u64,
    /// Total agents that failed.
    pub total_failed: u64,
    /// Total agents that were canceled.
    pub total_canceled: u64,
    /// Average completion time in seconds.
    pub avg_completion_secs: f64,
    /// Total files changed across all agents.
    pub total_files_changed: usize,
    /// Total commits across all agents.
    pub total_commits: usize,
}

// === Task Splitter ===

/// Estimated complexity of a sub-task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// A sub-task generated by splitting a larger task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier for the sub-task.
    pub id: String,
    /// Description of the sub-task.
    pub description: String,
    /// Estimated complexity.
    pub estimated_complexity: TaskComplexity,
    /// IDs of sub-tasks this depends on.
    pub dependencies: Vec<String>,
}

/// Splits a task into sub-tasks for parallel execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSplitter;

impl TaskSplitter {
    /// Split a task description into sub-tasks.
    ///
    /// Uses sentence boundaries and keyword heuristics to decompose the task.
    /// Returns exactly `num_agents` sub-tasks when possible.
    pub fn split_task(task: &str, num_agents: usize) -> Vec<SubTask> {
        if num_agents == 0 {
            return Vec::new();
        }

        // Split on sentence boundaries, newlines, and semicolons
        let separators = ['.', ';', '\n'];
        let mut parts: Vec<String> = Vec::new();
        let mut current = String::new();

        for ch in task.chars() {
            if separators.contains(&ch) {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
                current.clear();
            } else {
                current.push(ch);
            }
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }

        // If we got fewer parts than agents, duplicate or keep what we have
        if parts.is_empty() {
            parts.push(task.trim().to_string());
        }

        // Distribute parts across num_agents sub-tasks
        let mut subtasks: Vec<SubTask> = Vec::new();

        if parts.len() <= num_agents {
            // One sub-task per part, fill remaining with subdivisions of largest
            for (i, part) in parts.iter().enumerate() {
                let complexity = estimate_complexity(part);
                subtasks.push(SubTask {
                    id: format!("sub-{}", i + 1),
                    description: part.clone(),
                    estimated_complexity: complexity,
                    dependencies: Vec::new(),
                });
            }
            // Fill remaining with generic sub-tasks
            let mut idx = parts.len();
            while subtasks.len() < num_agents {
                idx += 1;
                subtasks.push(SubTask {
                    id: format!("sub-{}", idx),
                    description: format!("Additional work for: {}", task.trim()),
                    estimated_complexity: TaskComplexity::Low,
                    dependencies: vec![format!("sub-{}", idx - 1)],
                });
            }
        } else {
            // More parts than agents; merge parts into num_agents buckets
            let bucket_size = parts.len().div_ceil(num_agents);
            for (i, chunk) in parts.chunks(bucket_size).enumerate() {
                let desc = chunk.join(". ");
                let complexity = estimate_complexity(&desc);
                subtasks.push(SubTask {
                    id: format!("sub-{}", i + 1),
                    description: desc,
                    estimated_complexity: complexity,
                    dependencies: Vec::new(),
                });
            }
        }

        subtasks.truncate(num_agents);
        subtasks
    }
}

/// Heuristic complexity estimation based on keywords.
fn estimate_complexity(desc: &str) -> TaskComplexity {
    let lower = desc.to_lowercase();
    let high_keywords = [
        "refactor",
        "redesign",
        "migrate",
        "rewrite",
        "architecture",
        "security",
        "performance",
        "distributed",
        "concurrent",
    ];
    let medium_keywords = [
        "add",
        "implement",
        "create",
        "build",
        "integrate",
        "test",
        "update",
        "modify",
    ];

    if high_keywords.iter().any(|k| lower.contains(k)) {
        TaskComplexity::High
    } else if medium_keywords.iter().any(|k| lower.contains(k)) {
        TaskComplexity::Medium
    } else {
        TaskComplexity::Low
    }
}

// === Progress Aggregator ===

/// Aggregated progress across all agents in the pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolProgress {
    /// Total number of agents.
    pub total_agents: usize,
    /// Number of completed agents.
    pub completed: usize,
    /// Number of failed agents.
    pub failed: usize,
    /// Number of currently running agents.
    pub running: usize,
    /// Overall progress percentage (0-100).
    pub overall_percent: u8,
    /// Estimated remaining time in seconds.
    pub estimated_remaining_secs: Option<u64>,
}

/// Aggregates progress from multiple agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressAggregator;

impl ProgressAggregator {
    /// Aggregate progress from a set of agents.
    pub fn aggregate(agents: &[&WorktreeAgent]) -> PoolProgress {
        if agents.is_empty() {
            return PoolProgress {
                total_agents: 0,
                completed: 0,
                failed: 0,
                running: 0,
                overall_percent: 0,
                estimated_remaining_secs: None,
            };
        }

        let total_agents = agents.len();
        let completed = agents
            .iter()
            .filter(|a| matches!(a.state, WorktreeState::Completed))
            .count();
        let failed = agents
            .iter()
            .filter(|a| matches!(a.state, WorktreeState::Failed(_)))
            .count();
        let running = agents
            .iter()
            .filter(|a| {
                matches!(
                    a.state,
                    WorktreeState::Creating | WorktreeState::Ready | WorktreeState::Running
                )
            })
            .count();

        let total_percent: u32 = agents.iter().map(|a| a.progress_percent as u32).sum();
        let overall_percent = (total_percent / total_agents as u32).min(100) as u8;

        // Estimate remaining time from completed agents
        let estimated_remaining_secs = if completed > 0 && running > 0 {
            let avg_time: f64 = agents
                .iter()
                .filter(|a| matches!(a.state, WorktreeState::Completed))
                .map(|a| (a.updated_at - a.created_at) as f64)
                .sum::<f64>()
                / completed as f64;

            let remaining_work: f64 = agents
                .iter()
                .filter(|a| !a.is_terminal())
                .map(|a| (100 - a.progress_percent) as f64 / 100.0)
                .sum();

            Some((avg_time * remaining_work) as u64)
        } else {
            None
        };

        PoolProgress {
            total_agents,
            completed,
            failed,
            running,
            overall_percent,
            estimated_remaining_secs,
        }
    }
}

// === Helpers ===

/// Returns a mock timestamp for deterministic behavior. In production,
/// this would use `std::time::SystemTime`.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pool() -> WorktreePool {
        WorktreePool::new("/tmp/test-repo", WorktreeConfig::default())
    }

    fn pool_with_capacity(cap: usize) -> WorktreePool {
        let mut config = WorktreeConfig::default();
        config.max_worktrees = cap;
        WorktreePool::new("/tmp/test-repo", config)
    }

    fn make_agent(id: &str, state: WorktreeState, files: Vec<&str>, commits: Vec<&str>) -> WorktreeAgent {
        let progress = if matches!(&state, WorktreeState::Completed) { 100 } else { 50 };
        WorktreeAgent {
            id: id.to_string(),
            worktree_path: format!("/tmp/.worktrees/{}", id),
            branch_name: format!("vibe-wt-{}", id),
            state,
            task_description: format!("Task for {}", id),
            created_at: 1000,
            updated_at: 1100,
            progress_percent: progress,
            files_changed: files.into_iter().map(String::from).collect(),
            commits: commits.into_iter().map(String::from).collect(),
        }
    }

    // --- Config Tests ---

    #[test]
    fn test_default_config() {
        let config = WorktreeConfig::default();
        assert_eq!(config.max_worktrees, 4);
        assert_eq!(config.base_branch, "main");
        assert_eq!(config.worktree_prefix, "vibe-wt-");
        assert!(!config.auto_cleanup);
        assert!(config.resource_limits.is_none());
    }

    #[test]
    fn test_config_with_resource_limits() {
        let config = WorktreeConfig {
            resource_limits: Some(ResourceLimits {
                max_memory_mb: Some(1024),
                max_cpu_percent: Some(80),
                max_time_secs: Some(3600),
            }),
            ..Default::default()
        };
        let limits = config.resource_limits.unwrap();
        assert_eq!(limits.max_memory_mb, Some(1024));
        assert_eq!(limits.max_cpu_percent, Some(80));
        assert_eq!(limits.max_time_secs, Some(3600));
    }

    #[test]
    fn test_config_serialization() {
        let config = WorktreeConfig::default();
        let json = serde_json::to_string(&config).expect("serialize config");
        let deserialized: WorktreeConfig = serde_json::from_str(&json).expect("deserialize config");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_resource_limits_partial() {
        let limits = ResourceLimits {
            max_memory_mb: Some(512),
            max_cpu_percent: None,
            max_time_secs: None,
        };
        assert_eq!(limits.max_memory_mb, Some(512));
        assert!(limits.max_cpu_percent.is_none());
    }

    // --- Pool Creation Tests ---

    #[test]
    fn test_pool_creation() {
        let pool = default_pool();
        assert_eq!(pool.repo_root, "/tmp/test-repo");
        assert!(pool.agents.is_empty());
        assert!(pool.merge_queue.is_empty());
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_with_custom_config() {
        let config = WorktreeConfig {
            max_worktrees: 8,
            base_branch: "develop".to_string(),
            worktree_prefix: "custom-".to_string(),
            auto_cleanup: true,
            resource_limits: None,
        };
        let pool = WorktreePool::new("/home/user/repo", config.clone());
        assert_eq!(pool.config.max_worktrees, 8);
        assert_eq!(pool.config.base_branch, "develop");
        assert_eq!(pool.config.worktree_prefix, "custom-");
        assert!(pool.config.auto_cleanup);
    }

    // --- Agent Spawning Tests ---

    #[test]
    fn test_spawn_agent_success() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Fix the login bug").unwrap();
        assert!(id.starts_with("wt-"));
        assert_eq!(pool.agents.len(), 1);
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.task_description, "Fix the login bug");
        assert_eq!(agent.state, WorktreeState::Creating);
        assert_eq!(agent.progress_percent, 0);
    }

    #[test]
    fn test_spawn_multiple_agents() {
        let mut pool = default_pool();
        let id1 = pool.spawn_agent("Task 1").unwrap();
        let id2 = pool.spawn_agent("Task 2").unwrap();
        let id3 = pool.spawn_agent("Task 3").unwrap();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(pool.agents.len(), 3);
        assert_eq!(pool.active_count(), 3);
    }

    #[test]
    fn test_spawn_at_capacity() {
        let mut pool = pool_with_capacity(2);
        pool.spawn_agent("Task 1").unwrap();
        pool.spawn_agent("Task 2").unwrap();
        let result = pool.spawn_agent("Task 3");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at capacity"));
    }

    #[test]
    fn test_spawn_empty_task() {
        let mut pool = default_pool();
        let result = pool.spawn_agent("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_spawn_whitespace_task() {
        let mut pool = default_pool();
        let result = pool.spawn_agent("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_generates_unique_ids() {
        let mut pool = pool_with_capacity(10);
        let mut ids = Vec::new();
        for i in 0..10 {
            ids.push(pool.spawn_agent(&format!("Task {}", i)).unwrap());
        }
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), 10);
    }

    #[test]
    fn test_spawn_worktree_path_format() {
        let mut pool = WorktreePool::new("/my/repo", WorktreeConfig::default());
        let id = pool.spawn_agent("Test task").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert!(agent.worktree_path.starts_with("/my/repo/.worktrees/"));
    }

    #[test]
    fn test_spawn_branch_name_format() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Test").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert!(agent.branch_name.starts_with("vibe-wt-"));
    }

    // --- Agent Lifecycle Tests ---

    #[test]
    fn test_lifecycle_spawn_to_complete() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Implement feature").unwrap();

        // Progress update transitions to Running
        pool.update_progress(&id, 50, vec!["src/lib.rs".to_string()]);
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Running);
        assert_eq!(pool.get_agent(&id).unwrap().progress_percent, 50);

        // Complete
        pool.complete_agent(&id, vec!["abc123".to_string()]).unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.state, WorktreeState::Completed);
        assert_eq!(agent.progress_percent, 100);
        assert_eq!(agent.commits, vec!["abc123"]);
        assert!(pool.merge_queue.contains(&id));
    }

    #[test]
    fn test_lifecycle_spawn_to_fail() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Risky task").unwrap();
        pool.update_progress(&id, 30, vec![]);
        pool.fail_agent(&id, "compilation error").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(
            agent.state,
            WorktreeState::Failed("compilation error".to_string())
        );
    }

    #[test]
    fn test_lifecycle_spawn_to_cancel() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Long task").unwrap();
        pool.update_progress(&id, 20, vec![]);
        pool.cancel_agent(&id).unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(
            agent.state,
            WorktreeState::Failed("canceled by user".to_string())
        );
    }

    #[test]
    fn test_cannot_complete_terminal_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.complete_agent(&id, vec![]).unwrap();
        let result = pool.complete_agent(&id, vec!["extra".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("terminal state"));
    }

    #[test]
    fn test_cannot_fail_terminal_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.fail_agent(&id, "oops").unwrap();
        let result = pool.fail_agent(&id, "again");
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_cancel_terminal_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.complete_agent(&id, vec![]).unwrap();
        let result = pool.cancel_agent(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_unknown_agent() {
        let mut pool = default_pool();
        let result = pool.complete_agent("nonexistent", vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fail_unknown_agent() {
        let mut pool = default_pool();
        let result = pool.fail_agent("nonexistent", "reason");
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_unknown_agent() {
        let mut pool = default_pool();
        let result = pool.cancel_agent("nonexistent");
        assert!(result.is_err());
    }

    // --- Progress Tests ---

    #[test]
    fn test_update_progress() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.update_progress(&id, 75, vec!["a.rs".to_string(), "b.rs".to_string()]);
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.progress_percent, 75);
        assert_eq!(agent.files_changed, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn test_update_progress_caps_at_100() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.update_progress(&id, 150, vec![]);
        assert_eq!(pool.get_agent(&id).unwrap().progress_percent, 100);
    }

    #[test]
    fn test_update_progress_transitions_to_running() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Creating);
        pool.update_progress(&id, 10, vec![]);
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Running);
    }

    #[test]
    fn test_update_progress_nonexistent_agent() {
        let mut pool = default_pool();
        // Should not panic
        pool.update_progress("nonexistent", 50, vec![]);
    }

    // --- Cleanup Tests ---

    #[test]
    fn test_cleanup_completed_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.complete_agent(&id, vec!["commit1".to_string()]).unwrap();
        pool.cleanup_agent(&id).unwrap();
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Cleaning);
    }

    #[test]
    fn test_cleanup_failed_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.fail_agent(&id, "error").unwrap();
        pool.cleanup_agent(&id).unwrap();
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Cleaning);
    }

    #[test]
    fn test_cleanup_running_agent_fails() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.update_progress(&id, 50, vec![]);
        let result = pool.cleanup_agent(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot cleanup active"));
    }

    #[test]
    fn test_cleanup_completed_batch() {
        let mut pool = pool_with_capacity(10);
        let id1 = pool.spawn_agent("Task 1").unwrap();
        let id2 = pool.spawn_agent("Task 2").unwrap();
        let id3 = pool.spawn_agent("Task 3").unwrap();

        pool.complete_agent(&id1, vec![]).unwrap();
        pool.fail_agent(&id2, "err").unwrap();
        // id3 remains active

        let cleaned = pool.cleanup_completed();
        assert_eq!(cleaned, 2);
        assert_eq!(pool.get_agent(&id1).unwrap().state, WorktreeState::Cleaning);
        assert_eq!(pool.get_agent(&id2).unwrap().state, WorktreeState::Cleaning);
        assert_eq!(pool.get_agent(&id3).unwrap().state, WorktreeState::Creating);
    }

    #[test]
    fn test_cleanup_empty_pool() {
        let mut pool = default_pool();
        assert_eq!(pool.cleanup_completed(), 0);
    }

    #[test]
    fn test_cleanup_removes_from_merge_queue() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        pool.complete_agent(&id, vec!["c1".to_string()]).unwrap();
        assert!(pool.merge_queue.contains(&id));
        pool.cleanup_agent(&id).unwrap();
        assert!(!pool.merge_queue.contains(&id));
    }

    // --- List and Query Tests ---

    #[test]
    fn test_list_agents_empty() {
        let pool = default_pool();
        assert!(pool.list_agents().is_empty());
    }

    #[test]
    fn test_list_agents_sorted_by_created_at() {
        let mut pool = pool_with_capacity(5);
        let _id1 = pool.spawn_agent("First").unwrap();
        let _id2 = pool.spawn_agent("Second").unwrap();
        let _id3 = pool.spawn_agent("Third").unwrap();

        let agents = pool.list_agents();
        assert_eq!(agents.len(), 3);
        // Verify sorted by created_at (ascending)
        assert!(agents[0].created_at <= agents[1].created_at);
        assert!(agents[1].created_at <= agents[2].created_at);
    }

    #[test]
    fn test_get_agent_exists() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        assert!(pool.get_agent(&id).is_some());
    }

    #[test]
    fn test_get_agent_not_exists() {
        let pool = default_pool();
        assert!(pool.get_agent("nonexistent").is_none());
    }

    #[test]
    fn test_active_count_excludes_terminal() {
        let mut pool = pool_with_capacity(10);
        pool.spawn_agent("Active 1").unwrap();
        pool.spawn_agent("Active 2").unwrap();
        let id3 = pool.spawn_agent("Will complete").unwrap();
        let id4 = pool.spawn_agent("Will fail").unwrap();

        pool.complete_agent(&id3, vec![]).unwrap();
        pool.fail_agent(&id4, "err").unwrap();

        assert_eq!(pool.active_count(), 2);
    }

    // --- Merge Orchestrator Tests ---

    #[test]
    fn test_plan_merge_order_by_completion_time() {
        let mut agent_a = make_agent("a", WorktreeState::Completed, vec![], vec!["c1"]);
        agent_a.updated_at = 2000;
        let mut agent_b = make_agent("b", WorktreeState::Completed, vec![], vec!["c2"]);
        agent_b.updated_at = 1500;

        let order = MergeOrchestrator::plan_merge_order(&[&agent_a, &agent_b]);
        assert_eq!(order, vec!["b", "a"]); // b completed earlier
    }

    #[test]
    fn test_plan_merge_order_excludes_non_completed() {
        let agent_a = make_agent("a", WorktreeState::Completed, vec![], vec![]);
        let agent_b = make_agent("b", WorktreeState::Running, vec![], vec![]);
        let agent_c = make_agent("c", WorktreeState::Failed("err".to_string()), vec![], vec![]);

        let order = MergeOrchestrator::plan_merge_order(&[&agent_a, &agent_b, &agent_c]);
        assert_eq!(order, vec!["a"]);
    }

    #[test]
    fn test_plan_merge_order_empty() {
        let order = MergeOrchestrator::plan_merge_order(&[]);
        assert!(order.is_empty());
    }

    #[test]
    fn test_detect_conflicts_both_modified() {
        let agent_a = make_agent("a", WorktreeState::Completed, vec!["src/lib.rs", "README.md"], vec![]);
        let agent_b = make_agent("b", WorktreeState::Completed, vec!["src/lib.rs", "Cargo.toml"], vec![]);

        let conflicts = MergeOrchestrator::detect_conflicts(&agent_a, &agent_b);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].file_path, "src/lib.rs");
        assert_eq!(conflicts[0].conflict_type, ConflictType::BothModified);
    }

    #[test]
    fn test_detect_conflicts_no_overlap() {
        let agent_a = make_agent("a", WorktreeState::Completed, vec!["a.rs"], vec![]);
        let agent_b = make_agent("b", WorktreeState::Completed, vec!["b.rs"], vec![]);

        let conflicts = MergeOrchestrator::detect_conflicts(&agent_a, &agent_b);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_multiple_overlaps() {
        let agent_a = make_agent(
            "a",
            WorktreeState::Completed,
            vec!["x.rs", "y.rs", "z.rs"],
            vec![],
        );
        let agent_b = make_agent(
            "b",
            WorktreeState::Completed,
            vec!["y.rs", "z.rs", "w.rs"],
            vec![],
        );

        let conflicts = MergeOrchestrator::detect_conflicts(&agent_a, &agent_b);
        assert_eq!(conflicts.len(), 2);
        // Sorted by file_path
        assert_eq!(conflicts[0].file_path, "y.rs");
        assert_eq!(conflicts[1].file_path, "z.rs");
    }

    #[test]
    fn test_detect_conflicts_empty_files() {
        let agent_a = make_agent("a", WorktreeState::Completed, vec![], vec![]);
        let agent_b = make_agent("b", WorktreeState::Completed, vec!["x.rs"], vec![]);
        assert!(MergeOrchestrator::detect_conflicts(&agent_a, &agent_b).is_empty());
    }

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(MergeStrategy::Sequential.to_string(), "sequential");
        assert_eq!(
            MergeStrategy::ParallelThenResolve.to_string(),
            "parallel-then-resolve"
        );
        assert_eq!(MergeStrategy::CombinedPR.to_string(), "combined-pr");
    }

    #[test]
    fn test_conflict_type_display() {
        assert_eq!(ConflictType::BothModified.to_string(), "both-modified");
        assert_eq!(
            ConflictType::DeletedVsModified.to_string(),
            "deleted-vs-modified"
        );
        assert_eq!(ConflictType::AddedInBoth.to_string(), "added-in-both");
    }

    // --- PR Generator Tests ---

    #[test]
    fn test_pr_title_single_agent() {
        let agent = make_agent("wt-0001", WorktreeState::Completed, vec![], vec![]);
        let title = PrGenerator::generate_pr_title(&agent);
        assert!(title.contains("wt-0001"));
        assert!(title.contains("Task for wt-0001"));
    }

    #[test]
    fn test_pr_title_truncation() {
        let mut agent = make_agent("wt-0001", WorktreeState::Completed, vec![], vec![]);
        agent.task_description = "A".repeat(100);
        let title = PrGenerator::generate_pr_title(&agent);
        assert!(title.len() < 80);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_pr_body_single_agent() {
        let agent = make_agent(
            "wt-0001",
            WorktreeState::Completed,
            vec!["src/main.rs", "src/lib.rs"],
            vec!["abc123", "def456"],
        );
        let body = PrGenerator::generate_pr_body(&[&agent]);
        assert!(body.contains("wt-0001"));
        assert!(body.contains("**Files changed:** 2"));
        assert!(body.contains("**Commits:** 2"));
        assert!(body.contains("`src/main.rs`"));
        assert!(body.contains("VibeCody Worktree Pool"));
    }

    #[test]
    fn test_pr_body_combined() {
        let agent_a = make_agent("a", WorktreeState::Completed, vec!["x.rs"], vec!["c1"]);
        let agent_b = make_agent("b", WorktreeState::Completed, vec!["y.rs", "z.rs"], vec!["c2", "c3"]);
        let body = PrGenerator::generate_pr_body(&[&agent_a, &agent_b]);
        assert!(body.contains("**2** worktree agents"));
        assert!(body.contains("3 files changed, 3 commits"));
        assert!(body.contains("| `a` |"));
        assert!(body.contains("| `b` |"));
    }

    #[test]
    fn test_pr_mode_equality() {
        assert_eq!(PrMode::PerWorktree, PrMode::PerWorktree);
        assert_ne!(PrMode::PerWorktree, PrMode::Combined);
    }

    // --- Task Splitter Tests ---

    #[test]
    fn test_split_single_task() {
        let subtasks = TaskSplitter::split_task("Fix the login bug", 1);
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].id, "sub-1");
        assert!(subtasks[0].description.contains("Fix the login bug"));
    }

    #[test]
    fn test_split_multiple_sentences() {
        let subtasks = TaskSplitter::split_task(
            "Add user authentication. Implement rate limiting. Create admin panel",
            3,
        );
        assert_eq!(subtasks.len(), 3);
        assert!(subtasks[0].description.contains("authentication"));
        assert!(subtasks[1].description.contains("rate limiting"));
        assert!(subtasks[2].description.contains("admin panel"));
    }

    #[test]
    fn test_split_fewer_parts_than_agents() {
        let subtasks = TaskSplitter::split_task("Fix the bug", 3);
        assert_eq!(subtasks.len(), 3);
        assert_eq!(subtasks[0].description, "Fix the bug");
        // Additional sub-tasks should be generated
        assert!(subtasks[1].description.contains("Additional work"));
    }

    #[test]
    fn test_split_zero_agents() {
        let subtasks = TaskSplitter::split_task("Some task", 0);
        assert!(subtasks.is_empty());
    }

    #[test]
    fn test_split_complexity_estimation() {
        let subtasks = TaskSplitter::split_task(
            "Refactor the database layer. Add a button. Fix typo",
            3,
        );
        assert_eq!(subtasks[0].estimated_complexity, TaskComplexity::High); // refactor
        assert_eq!(subtasks[1].estimated_complexity, TaskComplexity::Medium); // add
        assert_eq!(subtasks[2].estimated_complexity, TaskComplexity::Low); // typo
    }

    #[test]
    fn test_split_more_parts_than_agents() {
        let subtasks = TaskSplitter::split_task(
            "Fix bug A. Fix bug B. Fix bug C. Fix bug D. Fix bug E. Fix bug F",
            2,
        );
        assert_eq!(subtasks.len(), 2);
    }

    #[test]
    fn test_split_semicolons() {
        let subtasks = TaskSplitter::split_task("Task A; Task B; Task C", 3);
        assert_eq!(subtasks.len(), 3);
        assert!(subtasks[0].description.contains("Task A"));
    }

    #[test]
    fn test_split_dependencies() {
        let subtasks = TaskSplitter::split_task("Simple task", 3);
        // First sub-task has no deps, additional ones depend on previous
        assert!(subtasks[0].dependencies.is_empty());
        assert!(!subtasks[2].dependencies.is_empty());
    }

    #[test]
    fn test_task_complexity_display() {
        assert_eq!(TaskComplexity::Low.to_string(), "low");
        assert_eq!(TaskComplexity::Medium.to_string(), "medium");
        assert_eq!(TaskComplexity::High.to_string(), "high");
    }

    // --- Progress Aggregator Tests ---

    #[test]
    fn test_aggregate_empty() {
        let progress = ProgressAggregator::aggregate(&[]);
        assert_eq!(progress.total_agents, 0);
        assert_eq!(progress.overall_percent, 0);
        assert!(progress.estimated_remaining_secs.is_none());
    }

    #[test]
    fn test_aggregate_all_running() {
        let mut a = make_agent("a", WorktreeState::Running, vec![], vec![]);
        a.progress_percent = 50;
        let mut b = make_agent("b", WorktreeState::Running, vec![], vec![]);
        b.progress_percent = 70;

        let progress = ProgressAggregator::aggregate(&[&a, &b]);
        assert_eq!(progress.total_agents, 2);
        assert_eq!(progress.running, 2);
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.overall_percent, 60); // (50+70)/2
    }

    #[test]
    fn test_aggregate_mixed_states() {
        let a = make_agent("a", WorktreeState::Completed, vec![], vec![]);
        let b = make_agent("b", WorktreeState::Running, vec![], vec![]);
        let c = make_agent("c", WorktreeState::Failed("err".to_string()), vec![], vec![]);

        let progress = ProgressAggregator::aggregate(&[&a, &b, &c]);
        assert_eq!(progress.total_agents, 3);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.running, 1);
        assert_eq!(progress.failed, 1);
    }

    #[test]
    fn test_aggregate_all_completed() {
        let a = make_agent("a", WorktreeState::Completed, vec![], vec![]);
        let b = make_agent("b", WorktreeState::Completed, vec![], vec![]);

        let progress = ProgressAggregator::aggregate(&[&a, &b]);
        assert_eq!(progress.completed, 2);
        assert_eq!(progress.overall_percent, 100);
    }

    // --- Metrics Tests ---

    #[test]
    fn test_metrics_empty_pool() {
        let pool = default_pool();
        let m = pool.metrics();
        assert_eq!(m.total_spawned, 0);
        assert_eq!(m.total_completed, 0);
        assert_eq!(m.total_failed, 0);
        assert_eq!(m.total_canceled, 0);
        assert_eq!(m.avg_completion_secs, 0.0);
    }

    #[test]
    fn test_metrics_with_agents() {
        let mut pool = pool_with_capacity(10);
        let id1 = pool.spawn_agent("Task 1").unwrap();
        let id2 = pool.spawn_agent("Task 2").unwrap();
        let id3 = pool.spawn_agent("Task 3").unwrap();

        pool.update_progress(&id1, 100, vec!["a.rs".to_string()]);
        pool.complete_agent(&id1, vec!["c1".to_string(), "c2".to_string()])
            .unwrap();

        pool.update_progress(&id2, 50, vec!["b.rs".to_string(), "c.rs".to_string()]);
        pool.fail_agent(&id2, "build failed").unwrap();

        pool.cancel_agent(&id3).unwrap();

        let m = pool.metrics();
        assert_eq!(m.total_spawned, 3);
        assert_eq!(m.total_completed, 1);
        assert_eq!(m.total_failed, 1);
        assert_eq!(m.total_canceled, 1);
        assert_eq!(m.total_commits, 2);
        assert_eq!(m.total_files_changed, 3);
    }

    // --- Worktree State Tests ---

    #[test]
    fn test_worktree_state_display() {
        assert_eq!(WorktreeState::Creating.to_string(), "creating");
        assert_eq!(WorktreeState::Ready.to_string(), "ready");
        assert_eq!(WorktreeState::Running.to_string(), "running");
        assert_eq!(WorktreeState::Merging.to_string(), "merging");
        assert_eq!(WorktreeState::Completed.to_string(), "completed");
        assert_eq!(
            WorktreeState::Failed("err".to_string()).to_string(),
            "failed: err"
        );
        assert_eq!(WorktreeState::Cleaning.to_string(), "cleaning");
    }

    #[test]
    fn test_worktree_state_equality() {
        assert_eq!(WorktreeState::Creating, WorktreeState::Creating);
        assert_ne!(WorktreeState::Creating, WorktreeState::Ready);
        assert_eq!(
            WorktreeState::Failed("x".to_string()),
            WorktreeState::Failed("x".to_string())
        );
        assert_ne!(
            WorktreeState::Failed("x".to_string()),
            WorktreeState::Failed("y".to_string())
        );
    }

    #[test]
    fn test_agent_is_terminal() {
        let completed = make_agent("a", WorktreeState::Completed, vec![], vec![]);
        let failed = make_agent("b", WorktreeState::Failed("err".to_string()), vec![], vec![]);
        let cleaning = make_agent("c", WorktreeState::Cleaning, vec![], vec![]);
        let running = make_agent("d", WorktreeState::Running, vec![], vec![]);

        assert!(completed.is_terminal());
        assert!(failed.is_terminal());
        assert!(cleaning.is_terminal());
        assert!(!running.is_terminal());
    }

    #[test]
    fn test_agent_is_active() {
        assert!(make_agent("a", WorktreeState::Creating, vec![], vec![]).is_active());
        assert!(make_agent("b", WorktreeState::Ready, vec![], vec![]).is_active());
        assert!(make_agent("c", WorktreeState::Running, vec![], vec![]).is_active());
        assert!(make_agent("d", WorktreeState::Merging, vec![], vec![]).is_active());
        assert!(!make_agent("e", WorktreeState::Completed, vec![], vec![]).is_active());
        assert!(!make_agent("f", WorktreeState::Failed("x".to_string()), vec![], vec![]).is_active());
    }

    // --- Serialization Tests ---

    #[test]
    fn test_worktree_agent_serialization() {
        let agent = make_agent("wt-0001", WorktreeState::Running, vec!["a.rs"], vec!["c1"]);
        let json = serde_json::to_string(&agent).expect("serialize agent");
        let deserialized: WorktreeAgent = serde_json::from_str(&json).expect("deserialize agent");
        assert_eq!(agent, deserialized);
    }

    #[test]
    fn test_pool_serialization() {
        let mut pool = default_pool();
        pool.spawn_agent("Task 1").unwrap();
        let json = serde_json::to_string(&pool).expect("serialize pool");
        let deserialized: WorktreePool = serde_json::from_str(&json).expect("deserialize pool");
        assert_eq!(deserialized.agents.len(), 1);
        assert_eq!(deserialized.repo_root, "/tmp/test-repo");
    }

    #[test]
    fn test_file_conflict_serialization() {
        let conflict = FileConflict {
            file_path: "src/main.rs".to_string(),
            conflict_type: ConflictType::BothModified,
        };
        let json = serde_json::to_string(&conflict).expect("serialize");
        let deserialized: FileConflict = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(conflict, deserialized);
    }

    #[test]
    fn test_subtask_serialization() {
        let subtask = SubTask {
            id: "sub-1".to_string(),
            description: "Do the thing".to_string(),
            estimated_complexity: TaskComplexity::High,
            dependencies: vec!["sub-0".to_string()],
        };
        let json = serde_json::to_string(&subtask).expect("serialize");
        let deserialized: SubTask = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(subtask, deserialized);
    }

    #[test]
    fn test_pool_progress_serialization() {
        let progress = PoolProgress {
            total_agents: 5,
            completed: 2,
            failed: 1,
            running: 2,
            overall_percent: 65,
            estimated_remaining_secs: Some(120),
        };
        let json = serde_json::to_string(&progress).expect("serialize");
        let deserialized: PoolProgress = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(progress, deserialized);
    }

    // --- Edge Case Tests ---

    #[test]
    fn test_spawn_after_cleanup_frees_capacity() {
        let mut pool = pool_with_capacity(2);
        let id1 = pool.spawn_agent("Task 1").unwrap();
        let _id2 = pool.spawn_agent("Task 2").unwrap();

        // At capacity
        assert!(pool.spawn_agent("Task 3").is_err());

        // Complete and note: completed agents are terminal, so capacity is freed
        pool.complete_agent(&id1, vec![]).unwrap();
        assert_eq!(pool.active_count(), 1);

        // Now can spawn again
        let id3 = pool.spawn_agent("Task 3");
        assert!(id3.is_ok());
    }

    #[test]
    fn test_pool_capacity_one() {
        let mut pool = pool_with_capacity(1);
        let id = pool.spawn_agent("Solo task").unwrap();
        assert!(pool.spawn_agent("Second task").is_err());
        pool.complete_agent(&id, vec![]).unwrap();
        assert!(pool.spawn_agent("Now ok").is_ok());
    }

    #[test]
    fn test_merge_orchestrator_new() {
        let orch = MergeOrchestrator::new(MergeStrategy::Sequential);
        assert_eq!(orch.strategy, MergeStrategy::Sequential);

        let orch2 = MergeOrchestrator::new(MergeStrategy::CombinedPR);
        assert_eq!(orch2.strategy, MergeStrategy::CombinedPR);
    }

    #[test]
    fn test_pr_generator_new() {
        let gen = PrGenerator::new(PrMode::PerWorktree);
        assert_eq!(gen.mode, PrMode::PerWorktree);

        let gen2 = PrGenerator::new(PrMode::Combined);
        assert_eq!(gen2.mode, PrMode::Combined);
    }

    #[test]
    fn test_complexity_keywords() {
        assert_eq!(estimate_complexity("refactor the code"), TaskComplexity::High);
        assert_eq!(estimate_complexity("migrate database"), TaskComplexity::High);
        assert_eq!(estimate_complexity("add a button"), TaskComplexity::Medium);
        assert_eq!(estimate_complexity("implement feature"), TaskComplexity::Medium);
        assert_eq!(estimate_complexity("fix typo"), TaskComplexity::Low);
        assert_eq!(estimate_complexity("rename variable"), TaskComplexity::Low);
    }

    #[test]
    fn test_agent_get_mut() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task").unwrap();
        {
            let agent = pool.get_agent_mut(&id).unwrap();
            agent.state = WorktreeState::Ready;
        }
        assert_eq!(pool.get_agent(&id).unwrap().state, WorktreeState::Ready);
    }

    #[test]
    fn test_worktree_pool_metrics_serialization() {
        let m = WorktreePoolMetrics {
            total_spawned: 10,
            total_completed: 7,
            total_failed: 2,
            total_canceled: 1,
            avg_completion_secs: 45.5,
            total_files_changed: 100,
            total_commits: 30,
        };
        let json = serde_json::to_string(&m).expect("serialize");
        let deserialized: WorktreePoolMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, deserialized);
    }
}
