#![allow(dead_code)]
//! Parallel tool scheduler — dependency-tracked concurrent tool execution.
//!
//! Matches Claude Code 1.x behaviour: up to N tools run concurrently when their
//! declared dependencies have all completed. Tools that share a write-target are
//! automatically sequenced to prevent races.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Unique identifier for a scheduled tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolJobId(pub String);

impl ToolJobId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ToolJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution state of a single tool job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Waiting for dependencies to complete.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Failed — dependents will be skipped.
    Failed(String),
    /// Skipped because an upstream dependency failed.
    Skipped,
    /// Cancelled by caller.
    Cancelled,
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Pending => write!(f, "pending"),
            JobState::Running => write!(f, "running"),
            JobState::Completed => write!(f, "completed"),
            JobState::Failed(e) => write!(f, "failed({e})"),
            JobState::Skipped => write!(f, "skipped"),
            JobState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A tool job to be scheduled.
#[derive(Debug, Clone)]
pub struct ToolJob {
    pub id: ToolJobId,
    /// Human-readable tool name (e.g. "Read", "Edit", "Bash").
    pub tool_name: String,
    /// IDs of jobs that must complete before this one can start.
    pub depends_on: Vec<ToolJobId>,
    /// Write targets (file paths, resource URIs) — jobs sharing a write target
    /// are automatically sequenced.
    pub write_targets: Vec<String>,
    /// Read targets — used for dependency inference (write→read ordering).
    pub read_targets: Vec<String>,
    /// Opaque payload (serialised tool call arguments).
    pub payload: String,
    /// Current execution state.
    pub state: JobState,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
}

impl ToolJob {
    pub fn new(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id: ToolJobId::new(id),
            tool_name: tool_name.into(),
            depends_on: vec![],
            write_targets: vec![],
            read_targets: vec![],
            payload: payload.into(),
            state: JobState::Pending,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn with_depends_on(mut self, deps: Vec<ToolJobId>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn with_write_targets(mut self, targets: Vec<String>) -> Self {
        self.write_targets = targets;
        self
    }

    pub fn with_read_targets(mut self, targets: Vec<String>) -> Self {
        self.read_targets = targets;
        self
    }

    pub fn elapsed(&self) -> Option<Duration> {
        match (self.started_at, self.finished_at) {
            (Some(s), Some(f)) => Some(f.duration_since(s)),
            (Some(s), None) => Some(s.elapsed()),
            _ => None,
        }
    }
}

/// Configuration for the scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of jobs running concurrently.
    pub max_concurrency: usize,
    /// Whether to automatically infer write→read ordering.
    pub auto_sequence_writes: bool,
    /// Whether a failed job should propagate failure to dependents.
    pub fail_fast: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            auto_sequence_writes: true,
            fail_fast: true,
        }
    }
}

/// Outcome of a scheduling tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickResult {
    /// One or more jobs were promoted to Running.
    Dispatched(Vec<ToolJobId>),
    /// Nothing to dispatch — all remaining jobs are blocked.
    Blocked,
    /// All jobs are terminal (Completed / Failed / Skipped / Cancelled).
    Done,
}

/// Snapshot of scheduler state for display.
#[derive(Debug, Clone)]
pub struct SchedulerStatus {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub cancelled: usize,
    pub max_concurrency: usize,
}

// ---------------------------------------------------------------------------
// Parallel tool scheduler
// ---------------------------------------------------------------------------

/// Dependency-tracked parallel tool scheduler.
///
/// Call `add_job()` to register jobs, then drive execution with `tick()`.
/// The caller is responsible for actually executing jobs returned by `tick()`
/// and calling `mark_completed()` / `mark_failed()` when they finish.
pub struct ParallelToolScheduler {
    jobs: HashMap<ToolJobId, ToolJob>,
    order: Vec<ToolJobId>,
    config: SchedulerConfig,
    /// write_target → last job that wrote it (for auto-sequencing)
    write_lock: HashMap<String, ToolJobId>,
}

impl ParallelToolScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            jobs: HashMap::new(),
            order: vec![],
            config,
            write_lock: HashMap::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SchedulerConfig::default())
    }

    /// Register a job with the scheduler. Automatically infers write→read
    /// ordering when `auto_sequence_writes` is enabled.
    pub fn add_job(&mut self, mut job: ToolJob) {
        if self.config.auto_sequence_writes {
            // If another job already holds a write lock on this job's read
            // targets, add that job as a dependency.
            for rt in &job.read_targets.clone() {
                if let Some(writer_id) = self.write_lock.get(rt) {
                    if !job.depends_on.contains(writer_id) {
                        job.depends_on.push(writer_id.clone());
                    }
                }
            }
            // Register this job's write targets.
            for wt in &job.write_targets.clone() {
                // If another job is already writing this target, depend on it.
                if let Some(prev_writer) = self.write_lock.get(wt) {
                    if !job.depends_on.contains(prev_writer) {
                        job.depends_on.push(prev_writer.clone());
                    }
                }
                self.write_lock.insert(wt.clone(), job.id.clone());
            }
        }
        self.order.push(job.id.clone());
        self.jobs.insert(job.id.clone(), job);
    }

    /// Promote eligible pending jobs to Running and return their IDs.
    ///
    /// A job is eligible when:
    /// 1. It is `Pending`.
    /// 2. All its `depends_on` jobs are `Completed`.
    /// 3. The running count is below `max_concurrency`.
    pub fn tick(&mut self) -> TickResult {
        let running_count = self
            .jobs
            .values()
            .filter(|j| j.state == JobState::Running)
            .count();

        // Propagate failure/skips first.
        if self.config.fail_fast {
            self.propagate_failures();
        }

        // Check if all jobs are terminal.
        let all_terminal = self.jobs.values().all(|j| {
            matches!(
                j.state,
                JobState::Completed | JobState::Failed(_) | JobState::Skipped | JobState::Cancelled
            )
        });
        if all_terminal {
            return TickResult::Done;
        }

        let mut dispatched = vec![];
        let mut slots = self.config.max_concurrency.saturating_sub(running_count);

        for id in &self.order.clone() {
            if slots == 0 {
                break;
            }
            let job = match self.jobs.get(id) {
                Some(j) if j.state == JobState::Pending => j,
                _ => continue,
            };

            // Check all dependencies are completed.
            let deps_ok = job.depends_on.iter().all(|dep_id| {
                self.jobs
                    .get(dep_id)
                    .map(|d| d.state == JobState::Completed)
                    .unwrap_or(false)
            });

            if deps_ok {
                dispatched.push(id.clone());
                slots -= 1;
            }
        }

        if dispatched.is_empty() {
            return TickResult::Blocked;
        }

        for id in &dispatched {
            if let Some(job) = self.jobs.get_mut(id) {
                job.state = JobState::Running;
                job.started_at = Some(Instant::now());
            }
        }

        TickResult::Dispatched(dispatched)
    }

    /// Mark a running job as successfully completed.
    pub fn mark_completed(&mut self, id: &ToolJobId) -> bool {
        if let Some(job) = self.jobs.get_mut(id) {
            if job.state == JobState::Running {
                job.state = JobState::Completed;
                job.finished_at = Some(Instant::now());
                return true;
            }
        }
        false
    }

    /// Mark a running job as failed with an error message.
    pub fn mark_failed(&mut self, id: &ToolJobId, error: impl Into<String>) -> bool {
        if let Some(job) = self.jobs.get_mut(id) {
            if job.state == JobState::Running {
                job.state = JobState::Failed(error.into());
                job.finished_at = Some(Instant::now());
                return true;
            }
        }
        false
    }

    /// Cancel a pending or running job.
    pub fn cancel(&mut self, id: &ToolJobId) -> bool {
        if let Some(job) = self.jobs.get_mut(id) {
            if matches!(job.state, JobState::Pending | JobState::Running) {
                job.state = JobState::Cancelled;
                job.finished_at = Some(Instant::now());
                return true;
            }
        }
        false
    }

    /// Cancel all jobs that are still pending or running.
    pub fn cancel_all(&mut self) {
        let now = Instant::now();
        for job in self.jobs.values_mut() {
            if matches!(job.state, JobState::Pending | JobState::Running) {
                job.state = JobState::Cancelled;
                job.finished_at = Some(now);
            }
        }
    }

    /// Get current state of a job.
    pub fn job_state(&self, id: &ToolJobId) -> Option<&JobState> {
        self.jobs.get(id).map(|j| &j.state)
    }

    /// Get all jobs in insertion order.
    pub fn jobs(&self) -> Vec<&ToolJob> {
        self.order
            .iter()
            .filter_map(|id| self.jobs.get(id))
            .collect()
    }

    /// Snapshot current scheduler statistics.
    pub fn status(&self) -> SchedulerStatus {
        let mut s = SchedulerStatus {
            pending: 0,
            running: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            cancelled: 0,
            max_concurrency: self.config.max_concurrency,
        };
        for job in self.jobs.values() {
            match &job.state {
                JobState::Pending => s.pending += 1,
                JobState::Running => s.running += 1,
                JobState::Completed => s.completed += 1,
                JobState::Failed(_) => s.failed += 1,
                JobState::Skipped => s.skipped += 1,
                JobState::Cancelled => s.cancelled += 1,
            }
        }
        s
    }

    /// Topologically sort jobs respecting explicit `depends_on` edges.
    /// Returns `Err` if a cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<ToolJobId>, String> {
        let mut in_degree: HashMap<&ToolJobId, usize> = HashMap::new();
        let mut adj: HashMap<&ToolJobId, Vec<&ToolJobId>> = HashMap::new();

        for id in &self.order {
            in_degree.entry(id).or_insert(0);
            adj.entry(id).or_default();
        }
        for job in self.jobs.values() {
            for dep in &job.depends_on {
                adj.entry(dep).or_default().push(&job.id);
                *in_degree.entry(&job.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&ToolJobId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = vec![];
        while let Some(id) = queue.pop_front() {
            result.push(id.clone());
            if let Some(neighbors) = adj.get(id) {
                for next in neighbors {
                    let deg = in_degree.get_mut(next).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }

        if result.len() != self.order.len() {
            Err("Cycle detected in tool dependency graph".to_string())
        } else {
            Ok(result)
        }
    }

    // Internal: mark dependents of failed/skipped jobs as Skipped (transitive).
    fn propagate_failures(&mut self) {
        loop {
            // Re-compute terminal-failure set each iteration to catch newly-skipped jobs.
            let failed_ids: HashSet<ToolJobId> = self
                .jobs
                .values()
                .filter(|j| {
                    matches!(j.state, JobState::Failed(_)) || j.state == JobState::Skipped
                })
                .map(|j| j.id.clone())
                .collect();

            if failed_ids.is_empty() {
                return;
            }

            let to_skip: Vec<ToolJobId> = self
                .jobs
                .values()
                .filter(|j| j.state == JobState::Pending)
                .filter(|j| j.depends_on.iter().any(|d| failed_ids.contains(d)))
                .map(|j| j.id.clone())
                .collect();

            if to_skip.is_empty() {
                break;
            }
            for id in to_skip {
                if let Some(job) = self.jobs.get_mut(&id) {
                    job.state = JobState::Skipped;
                }
            }
        }
    }
}

/// Thread-safe wrapper around `ParallelToolScheduler`.
pub struct SharedScheduler(Arc<Mutex<ParallelToolScheduler>>);

impl SharedScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self(Arc::new(Mutex::new(ParallelToolScheduler::new(config))))
    }

    pub fn clone_handle(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ParallelToolScheduler) -> R,
    {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str) -> ToolJob {
        ToolJob::new(id, "Read", r#"{"path":"file.rs"}"#)
    }

    fn job_with_deps(id: &str, deps: &[&str]) -> ToolJob {
        ToolJob::new(id, "Edit", r#"{"path":"file.rs"}"#).with_depends_on(
            deps.iter().map(|d| ToolJobId::new(*d)).collect(),
        )
    }

    #[test]
    fn test_single_job_dispatched() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("j1"));
        let r = sched.tick();
        assert_eq!(r, TickResult::Dispatched(vec![ToolJobId::new("j1")]));
    }

    #[test]
    fn test_independent_jobs_run_concurrently() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("a"));
        sched.add_job(job("b"));
        sched.add_job(job("c"));
        match sched.tick() {
            TickResult::Dispatched(ids) => assert_eq!(ids.len(), 3),
            other => panic!("expected Dispatched, got {:?}", other),
        }
    }

    #[test]
    fn test_dependent_job_blocked_until_dep_completes() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("a"));
        sched.add_job(job_with_deps("b", &["a"]));

        let tick1 = sched.tick();
        assert_eq!(tick1, TickResult::Dispatched(vec![ToolJobId::new("a")]));

        let tick2 = sched.tick();
        assert_eq!(tick2, TickResult::Blocked);

        sched.mark_completed(&ToolJobId::new("a"));
        let tick3 = sched.tick();
        assert_eq!(tick3, TickResult::Dispatched(vec![ToolJobId::new("b")]));
    }

    #[test]
    fn test_done_when_all_complete() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("x"));
        sched.tick();
        sched.mark_completed(&ToolJobId::new("x"));
        assert_eq!(sched.tick(), TickResult::Done);
    }

    #[test]
    fn test_fail_fast_skips_dependents() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("root"));
        sched.add_job(job_with_deps("child", &["root"]));
        sched.add_job(job_with_deps("grandchild", &["child"]));

        sched.tick(); // dispatches root
        sched.mark_failed(&ToolJobId::new("root"), "timeout");

        sched.tick(); // propagates skips
        assert_eq!(
            sched.job_state(&ToolJobId::new("child")),
            Some(&JobState::Skipped)
        );
        assert_eq!(
            sched.job_state(&ToolJobId::new("grandchild")),
            Some(&JobState::Skipped)
        );
        assert_eq!(sched.tick(), TickResult::Done);
    }

    #[test]
    fn test_max_concurrency_respected() {
        let cfg = SchedulerConfig {
            max_concurrency: 2,
            ..Default::default()
        };
        let mut sched = ParallelToolScheduler::new(cfg);
        for i in 0..5 {
            sched.add_job(job(&format!("j{i}")));
        }
        match sched.tick() {
            TickResult::Dispatched(ids) => assert_eq!(ids.len(), 2),
            other => panic!("expected 2 dispatched, got {:?}", other),
        }
    }

    #[test]
    fn test_cancel_all() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("a"));
        sched.add_job(job("b"));
        sched.cancel_all();
        let s = sched.status();
        assert_eq!(s.pending + s.running, 0);
        assert_eq!(s.cancelled, 2);
    }

    #[test]
    fn test_auto_sequence_write_targets() {
        let mut sched = ParallelToolScheduler::with_default_config();
        let writer = ToolJob::new("writer", "Edit", "{}")
            .with_write_targets(vec!["src/main.rs".into()]);
        let reader = ToolJob::new("reader", "Read", "{}")
            .with_read_targets(vec!["src/main.rs".into()]);

        sched.add_job(writer);
        sched.add_job(reader);

        // reader should have writer as a dep
        let reader_job = sched.jobs.get(&ToolJobId::new("reader")).unwrap();
        assert!(reader_job.depends_on.contains(&ToolJobId::new("writer")));
    }

    #[test]
    fn test_topological_sort_simple_chain() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("a"));
        sched.add_job(job_with_deps("b", &["a"]));
        sched.add_job(job_with_deps("c", &["b"]));

        let sorted = sched.topological_sort().unwrap();
        let a = sorted.iter().position(|x| x.0 == "a").unwrap();
        let b = sorted.iter().position(|x| x.0 == "b").unwrap();
        let c = sorted.iter().position(|x| x.0 == "c").unwrap();
        assert!(a < b && b < c);
    }

    #[test]
    fn test_status_counts() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("a"));
        sched.add_job(job("b"));
        sched.tick();
        sched.mark_completed(&ToolJobId::new("a"));

        let s = sched.status();
        assert_eq!(s.running, 1);
        assert_eq!(s.completed, 1);
    }

    #[test]
    fn test_shared_scheduler_thread_safe() {
        let shared = SharedScheduler::new(SchedulerConfig::default());
        shared.with(|s| s.add_job(job("t1")));
        let result = shared.with(|s| s.tick());
        assert!(matches!(result, TickResult::Dispatched(_)));
    }

    #[test]
    fn test_job_elapsed_time() {
        let mut sched = ParallelToolScheduler::with_default_config();
        sched.add_job(job("t"));
        sched.tick();
        sched.mark_completed(&ToolJobId::new("t"));
        let elapsed = sched.jobs.get(&ToolJobId::new("t")).unwrap().elapsed();
        assert!(elapsed.is_some());
    }
}
