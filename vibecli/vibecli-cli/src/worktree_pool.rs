#![allow(dead_code)]
//! Parallel agent execution via git worktrees.
//!
//! Enables spawning multiple agents in isolated git worktrees for parallel
//! task execution. Includes pool management, task splitting, branch naming,
//! merge orchestration, and metrics tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Status of a worktree agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeStatus {
    Idle,
    Running,
    Completed,
    Failed(String),
    Merging,
    Conflicted,
}

/// Strategy for merging worktree branches back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    Sequential,
    Rebase,
    OctopusMerge,
    CherryPick,
}

/// Priority level for a task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Type of agent to spawn in a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    VibeCody,
    ClaudeCode,
    GeminiCLI,
    Aider,
    Custom(String),
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Configuration for the worktree pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub max_worktrees: u32,
    pub base_dir: String,
    pub auto_cleanup: bool,
    pub auto_pr: bool,
    pub merge_strategy: MergeStrategy,
    pub resource_limit_mb: u64,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            max_worktrees: 4,
            base_dir: String::from("/tmp/vibecody-worktrees"),
            auto_cleanup: true,
            auto_pr: false,
            merge_strategy: MergeStrategy::Sequential,
            resource_limit_mb: 2048,
        }
    }
}

/// An agent running in an isolated worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeAgent {
    pub id: String,
    pub worktree_path: String,
    pub branch_name: String,
    pub status: WorktreeStatus,
    pub agent_type: AgentType,
    pub task_description: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub progress_pct: u8,
    pub output_log: Vec<String>,
}

/// A task that can be assigned to a worktree agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeTask {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub assigned_agent: Option<String>,
    pub subtasks: Vec<String>,
    pub created_at: u64,
}

/// Result of merging a worktree branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub conflicts: Vec<String>,
    pub merged_files: Vec<String>,
    pub branch_name: String,
}

/// Aggregate metrics for the worktree pool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeMetrics {
    pub total_spawned: u64,
    pub completed: u64,
    pub failed: u64,
    pub active: u64,
    pub avg_duration_secs: f64,
    pub merge_conflicts: u64,
}

// ---------------------------------------------------------------------------
// WorktreePool
// ---------------------------------------------------------------------------

/// Manages a pool of worktree-based agent workers.
pub struct WorktreePool {
    config: WorktreeConfig,
    agents: HashMap<String, WorktreeAgent>,
    tasks: Vec<WorktreeTask>,
    metrics: WorktreeMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl WorktreePool {
    /// Create a new pool with the given configuration.
    pub fn new(config: WorktreeConfig) -> Self {
        Self {
            config,
            agents: HashMap::new(),
            tasks: Vec::new(),
            metrics: WorktreeMetrics::default(),
            next_id: 1,
            timestamp_counter: 1000,
        }
    }

    /// Get the next timestamp (monotonically increasing counter for tests).
    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    /// Spawn a new agent in an isolated worktree. Returns the agent ID.
    pub fn spawn_agent(
        &mut self,
        task_desc: &str,
        agent_type: AgentType,
    ) -> Result<String, String> {
        if self.active_count() as u32 >= self.config.max_worktrees {
            return Err(format!(
                "Max worktree limit reached ({})",
                self.config.max_worktrees
            ));
        }
        if task_desc.trim().is_empty() {
            return Err("Task description cannot be empty".to_string());
        }

        let id = format!("wt-{}", self.next_id);
        self.next_id += 1;

        let branch_name = BranchNamer::generate("wt", task_desc);
        let worktree_path = format!("{}/{}", self.config.base_dir, branch_name);
        let ts = self.next_timestamp();

        let agent = WorktreeAgent {
            id: id.clone(),
            worktree_path,
            branch_name,
            status: WorktreeStatus::Running,
            agent_type,
            task_description: task_desc.to_string(),
            created_at: ts,
            updated_at: ts,
            progress_pct: 0,
            output_log: Vec::new(),
        };

        self.agents.insert(id.clone(), agent);
        self.metrics.total_spawned += 1;
        self.metrics.active += 1;

        Ok(id)
    }

    /// List all agents in the pool.
    pub fn list_agents(&self) -> Vec<&WorktreeAgent> {
        let mut agents: Vec<&WorktreeAgent> = self.agents.values().collect();
        agents.sort_by_key(|a| &a.id);
        agents
    }

    /// Get a specific agent by ID.
    pub fn get_agent(&self, id: &str) -> Option<&WorktreeAgent> {
        self.agents.get(id)
    }

    /// Update the progress of an agent.
    pub fn update_progress(&mut self, id: &str, pct: u8, log_line: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;
        match &agent.status {
            WorktreeStatus::Running => {}
            other => return Err(format!("Cannot update progress: agent is {:?}", other)),
        }
        if pct > 100 {
            return Err("Progress cannot exceed 100%".to_string());
        }
        agent.progress_pct = pct;
        if !log_line.is_empty() {
            agent.output_log.push(log_line.to_string());
        }
        agent.updated_at = self.timestamp_counter + 1;
        self.timestamp_counter += 1;
        Ok(())
    }

    /// Mark an agent as completed.
    pub fn complete_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;
        match &agent.status {
            WorktreeStatus::Running => {}
            other => return Err(format!("Cannot complete: agent is {:?}", other)),
        }
        agent.status = WorktreeStatus::Completed;
        agent.progress_pct = 100;
        agent.updated_at = self.timestamp_counter + 1;
        self.timestamp_counter += 1;
        self.metrics.completed += 1;
        self.metrics.active = self.metrics.active.saturating_sub(1);
        // Update average duration
        let duration = agent.updated_at.saturating_sub(agent.created_at) as f64;
        let total_completed = self.metrics.completed as f64;
        self.metrics.avg_duration_secs =
            (self.metrics.avg_duration_secs * (total_completed - 1.0) + duration)
                / total_completed;
        Ok(())
    }

    /// Mark an agent as failed.
    pub fn fail_agent(&mut self, id: &str, reason: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;
        match &agent.status {
            WorktreeStatus::Running => {}
            other => return Err(format!("Cannot fail: agent is {:?}", other)),
        }
        agent.status = WorktreeStatus::Failed(reason.to_string());
        agent.output_log.push(format!("FAILED: {}", reason));
        agent.updated_at = self.timestamp_counter + 1;
        self.timestamp_counter += 1;
        self.metrics.failed += 1;
        self.metrics.active = self.metrics.active.saturating_sub(1);
        Ok(())
    }

    /// Remove a completed or failed agent from the pool.
    pub fn cleanup_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;
        match &agent.status {
            WorktreeStatus::Completed | WorktreeStatus::Failed(_) => {}
            other => {
                return Err(format!(
                    "Cannot cleanup: agent is {:?} (must be Completed or Failed)",
                    other
                ))
            }
        }
        self.agents.remove(id);
        Ok(())
    }

    /// Remove all completed or failed agents. Returns count removed.
    pub fn cleanup_all_completed(&mut self) -> usize {
        let to_remove: Vec<String> = self
            .agents
            .iter()
            .filter(|(_, a)| {
                matches!(
                    a.status,
                    WorktreeStatus::Completed | WorktreeStatus::Failed(_)
                )
            })
            .map(|(id, _)| id.clone())
            .collect();
        let count = to_remove.len();
        for id in to_remove {
            self.agents.remove(&id);
        }
        count
    }

    /// Simulate merging an agent's branch into a target branch.
    pub fn merge_agent(
        &mut self,
        id: &str,
        target_branch: &str,
    ) -> Result<MergeResult, String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;
        match &agent.status {
            WorktreeStatus::Completed => {}
            other => {
                return Err(format!(
                    "Cannot merge: agent is {:?} (must be Completed)",
                    other
                ))
            }
        }
        agent.status = WorktreeStatus::Merging;
        agent.updated_at = self.timestamp_counter + 1;
        self.timestamp_counter += 1;

        // Simulate: branches with "conflict" in the task description produce conflicts
        let has_conflict = agent.task_description.to_lowercase().contains("conflict");
        let branch = agent.branch_name.clone();

        if has_conflict {
            agent.status = WorktreeStatus::Conflicted;
            self.metrics.merge_conflicts += 1;
            Ok(MergeResult {
                success: false,
                conflicts: vec![format!("CONFLICT in {}", branch)],
                merged_files: Vec::new(),
                branch_name: branch,
            })
        } else {
            agent.status = WorktreeStatus::Completed;
            Ok(MergeResult {
                success: true,
                conflicts: Vec::new(),
                merged_files: vec![
                    format!("src/{}.rs", target_branch),
                    format!("src/{}_impl.rs", target_branch),
                ],
                branch_name: branch,
            })
        }
    }

    /// Merge all completed agents sequentially into target branch.
    pub fn merge_all(&mut self, target_branch: &str) -> Vec<MergeResult> {
        let completed_ids: Vec<String> = self
            .agents
            .iter()
            .filter(|(_, a)| matches!(a.status, WorktreeStatus::Completed))
            .map(|(id, _)| id.clone())
            .collect();

        let mut results = Vec::new();
        for id in completed_ids {
            match self.merge_agent(&id, target_branch) {
                Ok(result) => results.push(result),
                Err(e) => results.push(MergeResult {
                    success: false,
                    conflicts: vec![e],
                    merged_files: Vec::new(),
                    branch_name: format!("unknown-{}", id),
                }),
            }
        }
        results
    }

    /// Count of currently active (Running/Merging/Idle) agents.
    pub fn active_count(&self) -> usize {
        self.agents
            .values()
            .filter(|a| {
                matches!(
                    a.status,
                    WorktreeStatus::Running | WorktreeStatus::Merging | WorktreeStatus::Idle
                )
            })
            .count()
    }

    /// Get the current metrics snapshot.
    pub fn get_metrics(&self) -> &WorktreeMetrics {
        &self.metrics
    }
}

// ---------------------------------------------------------------------------
// TaskSplitter
// ---------------------------------------------------------------------------

/// Splits a high-level task into subtasks for parallel execution.
pub struct TaskSplitter;

impl TaskSplitter {
    /// Split a description into `num_parts` numbered subtasks.
    pub fn split_task(description: &str, num_parts: usize) -> Vec<WorktreeTask> {
        if num_parts == 0 || description.trim().is_empty() {
            return Vec::new();
        }
        let mut tasks = Vec::new();
        let ts = 1000u64;
        for i in 1..=num_parts {
            tasks.push(WorktreeTask {
                id: format!("task-{}", i),
                description: format!("Part {}/{}: {}", i, num_parts, description),
                priority: if i == 1 {
                    TaskPriority::High
                } else {
                    TaskPriority::Medium
                },
                assigned_agent: None,
                subtasks: Vec::new(),
                created_at: ts + i as u64,
            });
        }
        tasks
    }

    /// Estimate the number of parallel workers to use.
    pub fn estimate_parallelism(task_count: usize, max_workers: u32) -> u32 {
        std::cmp::min(task_count as u32, max_workers)
    }
}

// ---------------------------------------------------------------------------
// BranchNamer
// ---------------------------------------------------------------------------

/// Generates and validates git branch names.
pub struct BranchNamer;

impl BranchNamer {
    /// Generate a sanitized branch name from a prefix and task description.
    pub fn generate(prefix: &str, task_desc: &str) -> String {
        let sanitized: String = task_desc
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();

        // Collapse consecutive hyphens
        let mut collapsed = String::new();
        let mut last_was_hyphen = false;
        for c in sanitized.chars() {
            if c == '-' {
                if !last_was_hyphen {
                    collapsed.push(c);
                }
                last_was_hyphen = true;
            } else {
                collapsed.push(c);
                last_was_hyphen = false;
            }
        }

        // Trim leading/trailing hyphens
        let trimmed = collapsed.trim_matches('-');

        let name = if prefix.is_empty() {
            trimmed.to_string()
        } else {
            format!("{}/{}", prefix, trimmed)
        };

        // Truncate to 50 chars
        if name.len() > 50 {
            let truncated = &name[..50];
            truncated.trim_end_matches('-').to_string()
        } else {
            name
        }
    }

    /// Check if a branch name is valid per git rules.
    pub fn is_valid(name: &str) -> bool {
        if name.is_empty() || name.len() > 255 {
            return false;
        }
        if name.starts_with('-') || name.starts_with('.') {
            return false;
        }
        if name.ends_with('/') || name.ends_with('.') || name.ends_with(".lock") {
            return false;
        }
        if name.contains("..") || name.contains("~") || name.contains("^") || name.contains(":") {
            return false;
        }
        if name.contains(' ') || name.contains('\\') || name.contains('\x7f') {
            return false;
        }
        if name.contains("@{") {
            return false;
        }
        // No ASCII control characters
        if name.chars().any(|c| (c as u32) < 32) {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pool() -> WorktreePool {
        WorktreePool::new(WorktreeConfig::default())
    }

    fn pool_with_max(max: u32) -> WorktreePool {
        WorktreePool::new(WorktreeConfig {
            max_worktrees: max,
            ..WorktreeConfig::default()
        })
    }

    // -- WorktreeConfig defaults --

    #[test]
    fn test_config_default_values() {
        let cfg = WorktreeConfig::default();
        assert_eq!(cfg.max_worktrees, 4);
        assert!(cfg.auto_cleanup);
        assert!(!cfg.auto_pr);
        assert_eq!(cfg.merge_strategy, MergeStrategy::Sequential);
        assert_eq!(cfg.resource_limit_mb, 2048);
    }

    #[test]
    fn test_config_custom_values() {
        let cfg = WorktreeConfig {
            max_worktrees: 8,
            base_dir: "/custom/dir".into(),
            auto_cleanup: false,
            auto_pr: true,
            merge_strategy: MergeStrategy::Rebase,
            resource_limit_mb: 4096,
        };
        assert_eq!(cfg.max_worktrees, 8);
        assert_eq!(cfg.base_dir, "/custom/dir");
        assert!(!cfg.auto_cleanup);
        assert!(cfg.auto_pr);
    }

    // -- Pool creation --

    #[test]
    fn test_pool_new_empty() {
        let pool = default_pool();
        assert_eq!(pool.agents.len(), 0);
        assert_eq!(pool.tasks.len(), 0);
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_new_with_custom_config() {
        let pool = pool_with_max(16);
        assert_eq!(pool.config.max_worktrees, 16);
    }

    // -- Spawn agents --

    #[test]
    fn test_spawn_agent_success() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Fix the login bug", AgentType::VibeCody)
            .unwrap();
        assert_eq!(id, "wt-1");
        assert_eq!(pool.active_count(), 1);
        assert_eq!(pool.metrics.total_spawned, 1);
    }

    #[test]
    fn test_spawn_multiple_agents() {
        let mut pool = default_pool();
        let id1 = pool.spawn_agent("Task A", AgentType::ClaudeCode).unwrap();
        let id2 = pool.spawn_agent("Task B", AgentType::GeminiCLI).unwrap();
        let id3 = pool.spawn_agent("Task C", AgentType::Aider).unwrap();
        assert_eq!(id1, "wt-1");
        assert_eq!(id2, "wt-2");
        assert_eq!(id3, "wt-3");
        assert_eq!(pool.active_count(), 3);
    }

    #[test]
    fn test_spawn_agent_max_limit() {
        let mut pool = pool_with_max(2);
        pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        let err = pool
            .spawn_agent("Task C", AgentType::VibeCody)
            .unwrap_err();
        assert!(err.contains("Max worktree limit reached"));
    }

    #[test]
    fn test_spawn_agent_empty_description() {
        let mut pool = default_pool();
        let err = pool.spawn_agent("", AgentType::VibeCody).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_spawn_agent_whitespace_description() {
        let mut pool = default_pool();
        let err = pool.spawn_agent("   ", AgentType::VibeCody).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_spawn_agent_custom_type() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Custom task", AgentType::Custom("MyAgent".into()))
            .unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.agent_type, AgentType::Custom("MyAgent".into()));
    }

    #[test]
    fn test_spawn_agent_sets_running_status() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Some task", AgentType::VibeCody)
            .unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.status, WorktreeStatus::Running);
        assert_eq!(agent.progress_pct, 0);
    }

    #[test]
    fn test_spawn_agent_branch_name_generated() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Add user authentication", AgentType::VibeCody)
            .unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert!(agent.branch_name.starts_with("wt/"));
        assert!(agent.branch_name.contains("add-user-authentication"));
    }

    // -- List / Get agents --

    #[test]
    fn test_list_agents_empty() {
        let pool = default_pool();
        assert!(pool.list_agents().is_empty());
    }

    #[test]
    fn test_list_agents_sorted() {
        let mut pool = default_pool();
        pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        let agents = pool.list_agents();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].id, "wt-1");
        assert_eq!(agents[1].id, "wt-2");
    }

    #[test]
    fn test_get_agent_not_found() {
        let pool = default_pool();
        assert!(pool.get_agent("wt-999").is_none());
    }

    // -- Update progress --

    #[test]
    fn test_update_progress_success() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.update_progress(&id, 50, "Halfway done").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.progress_pct, 50);
        assert_eq!(agent.output_log.len(), 1);
        assert_eq!(agent.output_log[0], "Halfway done");
    }

    #[test]
    fn test_update_progress_multiple_times() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.update_progress(&id, 25, "Step 1").unwrap();
        pool.update_progress(&id, 75, "Step 2").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.progress_pct, 75);
        assert_eq!(agent.output_log.len(), 2);
    }

    #[test]
    fn test_update_progress_over_100() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        let err = pool.update_progress(&id, 101, "Overflow").unwrap_err();
        assert!(err.contains("exceed 100"));
    }

    #[test]
    fn test_update_progress_not_found() {
        let mut pool = default_pool();
        let err = pool.update_progress("wt-999", 10, "").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_update_progress_completed_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        let err = pool.update_progress(&id, 50, "Late update").unwrap_err();
        assert!(err.contains("Cannot update progress"));
    }

    #[test]
    fn test_update_progress_empty_log_line() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.update_progress(&id, 10, "").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.output_log.len(), 0);
    }

    // -- Complete agent --

    #[test]
    fn test_complete_agent_success() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.status, WorktreeStatus::Completed);
        assert_eq!(agent.progress_pct, 100);
    }

    #[test]
    fn test_complete_agent_updates_metrics() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        assert_eq!(pool.metrics.completed, 1);
        assert_eq!(pool.metrics.active, 0);
    }

    #[test]
    fn test_complete_agent_not_found() {
        let mut pool = default_pool();
        let err = pool.complete_agent("wt-999").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_complete_agent_already_completed() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        let err = pool.complete_agent(&id).unwrap_err();
        assert!(err.contains("Cannot complete"));
    }

    // -- Fail agent --

    #[test]
    fn test_fail_agent_success() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.fail_agent(&id, "OOM killed").unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.status, WorktreeStatus::Failed("OOM killed".into()));
        assert!(agent.output_log.last().unwrap().contains("FAILED"));
    }

    #[test]
    fn test_fail_agent_updates_metrics() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.fail_agent(&id, "error").unwrap();
        assert_eq!(pool.metrics.failed, 1);
        assert_eq!(pool.metrics.active, 0);
    }

    #[test]
    fn test_fail_agent_not_found() {
        let mut pool = default_pool();
        let err = pool.fail_agent("wt-999", "reason").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_fail_already_failed_agent() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.fail_agent(&id, "first").unwrap();
        let err = pool.fail_agent(&id, "second").unwrap_err();
        assert!(err.contains("Cannot fail"));
    }

    // -- Cleanup --

    #[test]
    fn test_cleanup_agent_completed() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        pool.cleanup_agent(&id).unwrap();
        assert!(pool.get_agent(&id).is_none());
    }

    #[test]
    fn test_cleanup_agent_failed() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.fail_agent(&id, "err").unwrap();
        pool.cleanup_agent(&id).unwrap();
        assert!(pool.get_agent(&id).is_none());
    }

    #[test]
    fn test_cleanup_agent_running_fails() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        let err = pool.cleanup_agent(&id).unwrap_err();
        assert!(err.contains("Cannot cleanup"));
    }

    #[test]
    fn test_cleanup_all_completed() {
        let mut pool = default_pool();
        let id1 = pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        let id2 = pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        pool.spawn_agent("Task C", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id1).unwrap();
        pool.fail_agent(&id2, "err").unwrap();
        let removed = pool.cleanup_all_completed();
        assert_eq!(removed, 2);
        assert_eq!(pool.agents.len(), 1);
    }

    #[test]
    fn test_cleanup_all_none_completed() {
        let mut pool = default_pool();
        pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        let removed = pool.cleanup_all_completed();
        assert_eq!(removed, 0);
    }

    // -- Merge --

    #[test]
    fn test_merge_agent_success() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Add feature X", AgentType::VibeCody)
            .unwrap();
        pool.complete_agent(&id).unwrap();
        let result = pool.merge_agent(&id, "main").unwrap();
        assert!(result.success);
        assert!(result.conflicts.is_empty());
        assert!(!result.merged_files.is_empty());
    }

    #[test]
    fn test_merge_agent_with_conflict() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Fix conflict in parser", AgentType::VibeCody)
            .unwrap();
        pool.complete_agent(&id).unwrap();
        let result = pool.merge_agent(&id, "main").unwrap();
        assert!(!result.success);
        assert!(!result.conflicts.is_empty());
        assert_eq!(pool.metrics.merge_conflicts, 1);
    }

    #[test]
    fn test_merge_agent_not_completed() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        let err = pool.merge_agent(&id, "main").unwrap_err();
        assert!(err.contains("Cannot merge"));
    }

    #[test]
    fn test_merge_agent_not_found() {
        let mut pool = default_pool();
        let err = pool.merge_agent("wt-999", "main").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_merge_all_completed() {
        let mut pool = default_pool();
        let id1 = pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        let id2 = pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        pool.spawn_agent("Task C still running", AgentType::VibeCody)
            .unwrap();
        pool.complete_agent(&id1).unwrap();
        pool.complete_agent(&id2).unwrap();
        let results = pool.merge_all("main");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_merge_all_none_completed() {
        let mut pool = default_pool();
        pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        let results = pool.merge_all("main");
        assert!(results.is_empty());
    }

    // -- Active count --

    #[test]
    fn test_active_count_tracks_correctly() {
        let mut pool = default_pool();
        assert_eq!(pool.active_count(), 0);
        let id1 = pool.spawn_agent("A", AgentType::VibeCody).unwrap();
        assert_eq!(pool.active_count(), 1);
        pool.spawn_agent("B", AgentType::VibeCody).unwrap();
        assert_eq!(pool.active_count(), 2);
        pool.complete_agent(&id1).unwrap();
        assert_eq!(pool.active_count(), 1);
    }

    // -- Metrics --

    #[test]
    fn test_metrics_initial() {
        let pool = default_pool();
        let m = pool.get_metrics();
        assert_eq!(m.total_spawned, 0);
        assert_eq!(m.completed, 0);
        assert_eq!(m.failed, 0);
        assert_eq!(m.active, 0);
    }

    #[test]
    fn test_metrics_after_lifecycle() {
        let mut pool = default_pool();
        let id1 = pool.spawn_agent("A", AgentType::VibeCody).unwrap();
        let id2 = pool.spawn_agent("B", AgentType::VibeCody).unwrap();
        let id3 = pool.spawn_agent("C", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id1).unwrap();
        pool.complete_agent(&id2).unwrap();
        pool.fail_agent(&id3, "err").unwrap();
        let m = pool.get_metrics();
        assert_eq!(m.total_spawned, 3);
        assert_eq!(m.completed, 2);
        assert_eq!(m.failed, 1);
        assert_eq!(m.active, 0);
    }

    #[test]
    fn test_metrics_avg_duration() {
        let mut pool = default_pool();
        let id = pool.spawn_agent("Task", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        // Duration is timestamp difference, should be positive
        assert!(pool.get_metrics().avg_duration_secs > 0.0);
    }

    // -- Agent lifecycle: spawn -> progress -> complete -> cleanup --

    #[test]
    fn test_full_lifecycle() {
        let mut pool = default_pool();
        let id = pool
            .spawn_agent("Implement feature Y", AgentType::ClaudeCode)
            .unwrap();

        // Progress updates
        pool.update_progress(&id, 25, "Starting analysis").unwrap();
        pool.update_progress(&id, 50, "Writing code").unwrap();
        pool.update_progress(&id, 75, "Running tests").unwrap();

        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.output_log.len(), 3);

        // Complete
        pool.complete_agent(&id).unwrap();
        let agent = pool.get_agent(&id).unwrap();
        assert_eq!(agent.status, WorktreeStatus::Completed);
        assert_eq!(agent.progress_pct, 100);

        // Merge
        let result = pool.merge_agent(&id, "main").unwrap();
        assert!(result.success);

        // Cleanup
        pool.cleanup_agent(&id).unwrap();
        assert!(pool.get_agent(&id).is_none());
    }

    // -- TaskSplitter --

    #[test]
    fn test_split_task_basic() {
        let tasks = TaskSplitter::split_task("Build the API", 3);
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Part 1/3"));
        assert!(tasks[2].description.contains("Part 3/3"));
    }

    #[test]
    fn test_split_task_first_is_high_priority() {
        let tasks = TaskSplitter::split_task("Some task", 2);
        assert_eq!(tasks[0].priority, TaskPriority::High);
        assert_eq!(tasks[1].priority, TaskPriority::Medium);
    }

    #[test]
    fn test_split_task_zero_parts() {
        let tasks = TaskSplitter::split_task("Task", 0);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_split_task_empty_description() {
        let tasks = TaskSplitter::split_task("", 3);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_split_task_single_part() {
        let tasks = TaskSplitter::split_task("Single task", 1);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].description.contains("Part 1/1"));
    }

    #[test]
    fn test_split_task_ids_are_sequential() {
        let tasks = TaskSplitter::split_task("X", 5);
        for (i, t) in tasks.iter().enumerate() {
            assert_eq!(t.id, format!("task-{}", i + 1));
        }
    }

    #[test]
    fn test_estimate_parallelism_within_limit() {
        assert_eq!(TaskSplitter::estimate_parallelism(3, 8), 3);
    }

    #[test]
    fn test_estimate_parallelism_at_limit() {
        assert_eq!(TaskSplitter::estimate_parallelism(8, 8), 8);
    }

    #[test]
    fn test_estimate_parallelism_exceeds_limit() {
        assert_eq!(TaskSplitter::estimate_parallelism(20, 4), 4);
    }

    // -- BranchNamer --

    #[test]
    fn test_branch_generate_basic() {
        let name = BranchNamer::generate("wt", "Fix login bug");
        assert_eq!(name, "wt/fix-login-bug");
    }

    #[test]
    fn test_branch_generate_special_chars() {
        let name = BranchNamer::generate("wt", "Add user auth (v2.0)!");
        assert!(name.starts_with("wt/"));
        assert!(!name.contains(' '));
        assert!(!name.contains('('));
    }

    #[test]
    fn test_branch_generate_truncation() {
        let long_desc = "a".repeat(100);
        let name = BranchNamer::generate("wt", &long_desc);
        assert!(name.len() <= 50);
    }

    #[test]
    fn test_branch_generate_empty_prefix() {
        let name = BranchNamer::generate("", "My task");
        assert_eq!(name, "my-task");
    }

    #[test]
    fn test_branch_generate_collapses_hyphens() {
        let name = BranchNamer::generate("wt", "fix - - - bug");
        assert!(!name.contains("--"));
    }

    #[test]
    fn test_branch_is_valid_simple() {
        assert!(BranchNamer::is_valid("feature/add-login"));
    }

    #[test]
    fn test_branch_is_valid_empty() {
        assert!(!BranchNamer::is_valid(""));
    }

    #[test]
    fn test_branch_is_valid_dot_start() {
        assert!(!BranchNamer::is_valid(".hidden"));
    }

    #[test]
    fn test_branch_is_valid_double_dot() {
        assert!(!BranchNamer::is_valid("a..b"));
    }

    #[test]
    fn test_branch_is_valid_tilde() {
        assert!(!BranchNamer::is_valid("feat~1"));
    }

    #[test]
    fn test_branch_is_valid_space() {
        assert!(!BranchNamer::is_valid("my branch"));
    }

    #[test]
    fn test_branch_is_valid_ends_with_lock() {
        assert!(!BranchNamer::is_valid("ref.lock"));
    }

    #[test]
    fn test_branch_is_valid_ends_with_slash() {
        assert!(!BranchNamer::is_valid("feature/"));
    }

    #[test]
    fn test_branch_is_valid_at_brace() {
        assert!(!BranchNamer::is_valid("branch@{0}"));
    }

    // -- Serialization --

    #[test]
    fn test_config_serialization() {
        let cfg = WorktreeConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: WorktreeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_worktrees, cfg.max_worktrees);
    }

    #[test]
    fn test_agent_serialization() {
        let agent = WorktreeAgent {
            id: "wt-1".into(),
            worktree_path: "/tmp/wt".into(),
            branch_name: "wt/test".into(),
            status: WorktreeStatus::Running,
            agent_type: AgentType::VibeCody,
            task_description: "Test".into(),
            created_at: 1000,
            updated_at: 1000,
            progress_pct: 0,
            output_log: Vec::new(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        let parsed: WorktreeAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "wt-1");
    }

    #[test]
    fn test_metrics_serialization() {
        let m = WorktreeMetrics::default();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: WorktreeMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_spawned, 0);
    }

    // -- Edge cases --

    #[test]
    fn test_spawn_after_cleanup_frees_slot() {
        let mut pool = pool_with_max(1);
        let id = pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        pool.cleanup_agent(&id).unwrap();
        // Should be able to spawn again since active count is 0
        let id2 = pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        assert_eq!(id2, "wt-2");
    }

    #[test]
    fn test_complete_frees_slot_for_spawn() {
        let mut pool = pool_with_max(1);
        let id = pool.spawn_agent("Task A", AgentType::VibeCody).unwrap();
        pool.complete_agent(&id).unwrap();
        // Completed agents are not active, so we can spawn
        let id2 = pool.spawn_agent("Task B", AgentType::VibeCody).unwrap();
        assert_eq!(id2, "wt-2");
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Medium);
        assert!(TaskPriority::Medium < TaskPriority::Low);
    }

    #[test]
    fn test_worktree_status_variants() {
        let statuses = vec![
            WorktreeStatus::Idle,
            WorktreeStatus::Running,
            WorktreeStatus::Completed,
            WorktreeStatus::Failed("err".into()),
            WorktreeStatus::Merging,
            WorktreeStatus::Conflicted,
        ];
        assert_eq!(statuses.len(), 6);
    }

    #[test]
    fn test_merge_strategy_variants() {
        let strategies = vec![
            MergeStrategy::Sequential,
            MergeStrategy::Rebase,
            MergeStrategy::OctopusMerge,
            MergeStrategy::CherryPick,
        ];
        assert_eq!(strategies.len(), 4);
    }
}
