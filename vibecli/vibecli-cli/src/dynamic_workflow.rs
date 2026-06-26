//! Dynamic large-scale workflow primitive (gap C2).
//!
//! Claude Code's "Dynamic Workflows" and Devin's Spaces plan a large task, fan
//! out parallel sub-agents, verify each output, and report back — tuned for
//! 100k+ line migrations. VibeCody already ships the pieces — [`crate::planner`]
//! (decompose), [`crate::worktree_pool`] (parallel isolation),
//! [`crate::nested_agents`] (sub-agent spawning), and the A8 verify-repair loop —
//! but nothing fused them into one self-scaling primitive. This is that fusion.
//!
//! This module owns the **control logic**: decomposition into sub-tasks, batched
//! fan-out bounded by a parallelism cap, a **verify gate** (a sub-task isn't
//! "done" until its output passes verification), bounded **retries** (essential
//! at migration scale, where transient failures are common), and report
//! aggregation. The actual sub-agent run + worktree spawn + test execution is the
//! integration seam the daemon/CLI supplies; keeping that out makes the phase
//! state machine fully unit-testable without live agents.

use serde::{Deserialize, Serialize};

/// Default cap on concurrently-running sub-tasks (matches the worktree-pool
/// default fan-out; large migrations stay bounded so the host isn't swamped).
pub const DEFAULT_MAX_PARALLELISM: usize = 8;

/// Default per-sub-task retry budget (transient failures are common at scale).
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Lifecycle of a single decomposed unit of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskStatus {
    /// Not yet dispatched.
    Pending,
    /// Dispatched to a sub-agent, awaiting result.
    Running,
    /// Output produced and passed verification — done.
    Verified,
    /// Exhausted its retry budget without passing verification.
    Failed,
}

impl SubTaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, SubTaskStatus::Verified | SubTaskStatus::Failed)
    }
}

/// One decomposed unit of a large task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub status: SubTaskStatus,
    /// How many times this sub-task has been dispatched.
    pub attempts: u32,
}

impl SubTask {
    fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            status: SubTaskStatus::Pending,
            attempts: 0,
        }
    }
}

/// Tuning for a dynamic workflow run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Max sub-tasks dispatched concurrently per batch.
    pub max_parallelism: usize,
    /// Per-sub-task retry budget before it is marked `Failed`.
    pub max_retries: u32,
    /// Whether a sub-task requires a passing verification to count as done.
    /// `false` treats any produced output as success (no verify gate).
    pub verify_required: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_parallelism: DEFAULT_MAX_PARALLELISM,
            max_retries: DEFAULT_MAX_RETRIES,
            verify_required: true,
        }
    }
}

/// Summary of a workflow's terminal (or in-progress) state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowReport {
    pub total: usize,
    pub verified: usize,
    pub failed: usize,
    pub pending: usize,
    pub running: usize,
    /// True when every sub-task verified (no failures).
    pub success: bool,
}

/// The dynamic workflow controller — decompose → fan-out → verify → report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicWorkflow {
    pub config: WorkflowConfig,
    pub task: String,
    pub subtasks: Vec<SubTask>,
}

impl DynamicWorkflow {
    pub fn new(task: impl Into<String>, config: WorkflowConfig) -> Self {
        Self {
            config,
            task: task.into(),
            subtasks: Vec::new(),
        }
    }

    /// Ingest pre-decomposed units (e.g. from [`crate::planner`]'s plan steps),
    /// assigning stable ids. Replaces any existing sub-tasks.
    pub fn decompose<I, S>(&mut self, units: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.subtasks = units
            .into_iter()
            .enumerate()
            .map(|(i, u)| SubTask::new(format!("st-{:04}", i + 1), u.into()))
            .collect();
    }

    /// Heuristic decomposition for a migration: one sub-task per target file.
    /// The verb (e.g. "Migrate", "Refactor") plus the file path becomes the
    /// sub-task description. Convenience over `decompose` for the common case.
    pub fn decompose_by_files(&mut self, verb: &str, files: &[String]) {
        let units = files
            .iter()
            .map(|f| format!("{verb} {f}"))
            .collect::<Vec<_>>();
        self.decompose(units);
    }

    /// The next batch to dispatch: up to `max_parallelism` `Pending` sub-tasks.
    /// Returns ids so the caller can fan them out over the worktree pool.
    pub fn next_batch(&self) -> Vec<String> {
        self.subtasks
            .iter()
            .filter(|t| t.status == SubTaskStatus::Pending)
            .take(self.config.max_parallelism)
            .map(|t| t.id.clone())
            .collect()
    }

    /// Mark a sub-task dispatched (increments its attempt counter).
    pub fn mark_running(&mut self, id: &str) -> Result<(), String> {
        let t = self.find_mut(id)?;
        if t.status.is_terminal() {
            return Err(format!("sub-task {id} is already {:?}", t.status));
        }
        t.status = SubTaskStatus::Running;
        t.attempts += 1;
        Ok(())
    }

    /// Record a sub-task's outcome after its sub-agent ran and (optionally)
    /// verification ran.
    ///
    /// * `produced_output` — the sub-agent returned something.
    /// * `verified` — verification passed (ignored when `verify_required` is
    ///   false). A sub-task is `Verified` only when output was produced AND
    ///   (verification passed OR the gate is off). Otherwise it retries until
    ///   `max_retries` is exhausted, then `Failed`.
    pub fn record_result(
        &mut self,
        id: &str,
        produced_output: bool,
        verified: bool,
    ) -> Result<SubTaskStatus, String> {
        let max_retries = self.config.max_retries;
        let verify_required = self.config.verify_required;
        let t = self.find_mut(id)?;
        let ok = produced_output && (verified || !verify_required);
        if ok {
            t.status = SubTaskStatus::Verified;
        } else if t.attempts > max_retries {
            // attempts is 1-based (incremented at dispatch); allow max_retries
            // re-dispatches after the first attempt.
            t.status = SubTaskStatus::Failed;
        } else {
            t.status = SubTaskStatus::Pending; // eligible for re-dispatch
        }
        Ok(t.status)
    }

    /// True when every sub-task has reached a terminal state.
    pub fn is_complete(&self) -> bool {
        !self.subtasks.is_empty() && self.subtasks.iter().all(|t| t.status.is_terminal())
    }

    /// Aggregate the current state into a report.
    pub fn report(&self) -> WorkflowReport {
        let mut verified = 0;
        let mut failed = 0;
        let mut pending = 0;
        let mut running = 0;
        for t in &self.subtasks {
            match t.status {
                SubTaskStatus::Verified => verified += 1,
                SubTaskStatus::Failed => failed += 1,
                SubTaskStatus::Pending => pending += 1,
                SubTaskStatus::Running => running += 1,
            }
        }
        WorkflowReport {
            total: self.subtasks.len(),
            verified,
            failed,
            pending,
            running,
            success: failed == 0 && pending == 0 && running == 0 && !self.subtasks.is_empty(),
        }
    }

    fn find_mut(&mut self, id: &str) -> Result<&mut SubTask, String> {
        self.subtasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("no sub-task {id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wf() -> DynamicWorkflow {
        let mut w = DynamicWorkflow::new("migrate to async", WorkflowConfig::default());
        w.decompose_by_files(
            "Migrate",
            &["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()],
        );
        w
    }

    #[test]
    fn decompose_assigns_ids_and_descriptions() {
        let w = wf();
        assert_eq!(w.subtasks.len(), 3);
        assert_eq!(w.subtasks[0].id, "st-0001");
        assert_eq!(w.subtasks[0].description, "Migrate src/a.rs");
        assert!(w
            .subtasks
            .iter()
            .all(|t| t.status == SubTaskStatus::Pending));
    }

    #[test]
    fn next_batch_respects_parallelism_cap() {
        let mut w = wf();
        w.config.max_parallelism = 2;
        assert_eq!(w.next_batch(), vec!["st-0001", "st-0002"]);
    }

    #[test]
    fn happy_path_verifies_and_completes() {
        let mut w = wf();
        for id in ["st-0001", "st-0002", "st-0003"] {
            w.mark_running(id).unwrap();
            assert_eq!(
                w.record_result(id, true, true).unwrap(),
                SubTaskStatus::Verified
            );
        }
        assert!(w.is_complete());
        let r = w.report();
        assert_eq!(r.verified, 3);
        assert!(r.success);
        assert!(w.next_batch().is_empty());
    }

    #[test]
    fn unverified_output_retries_then_fails() {
        let mut w = wf();
        w.config.max_retries = 2; // 1 initial attempt + 2 retries = 3 attempts
        let id = "st-0001";
        // Attempts 1 and 2: output produced but verification fails → re-queued.
        for _ in 0..2 {
            w.mark_running(id).unwrap();
            assert_eq!(
                w.record_result(id, true, false).unwrap(),
                SubTaskStatus::Pending
            );
        }
        // Attempt 3 exhausts the budget (attempts 3 > max_retries 2) → Failed.
        w.mark_running(id).unwrap();
        assert_eq!(
            w.record_result(id, true, false).unwrap(),
            SubTaskStatus::Failed
        );
    }

    #[test]
    fn verify_gate_off_accepts_any_output() {
        let mut w = wf();
        w.config.verify_required = false;
        w.mark_running("st-0001").unwrap();
        assert_eq!(
            w.record_result("st-0001", true, false).unwrap(),
            SubTaskStatus::Verified
        );
    }

    #[test]
    fn report_counts_mixed_states_and_no_output_retries() {
        let mut w = wf();
        w.mark_running("st-0001").unwrap();
        w.record_result("st-0001", true, true).unwrap(); // verified
        w.mark_running("st-0002").unwrap();
        w.record_result("st-0002", false, false).unwrap(); // no output → pending (retry)
        let r = w.report();
        assert_eq!(r.total, 3);
        assert_eq!(r.verified, 1);
        assert_eq!(r.pending, 2); // st-0002 back to pending + st-0003 never started
        assert!(!r.success);
        assert!(!w.is_complete());
    }

    #[test]
    fn cannot_run_terminal_subtask() {
        let mut w = wf();
        w.mark_running("st-0001").unwrap();
        w.record_result("st-0001", true, true).unwrap();
        assert!(w.mark_running("st-0001").is_err());
        assert!(w.mark_running("nope").is_err());
    }
}
