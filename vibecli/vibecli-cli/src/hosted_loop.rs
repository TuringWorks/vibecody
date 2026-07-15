//! C1 — machine-off hosted loop execution (daemon-resident scheduler).
//!
//! The interactive `/loop` command (shipped earlier) runs a loop's body in the
//! REPL process — it dies when the client disconnects. "Machine-off hosted
//! execution" (à la Claude Code Routines / Managed Agents) moves recurring loops
//! into the long-lived **daemon**, so a client can enqueue a loop, close the lid,
//! and the loop keeps running server-side.
//!
//! This module is that scheduler:
//!
//! * [`due_hosted_jobs`] — the pure "which loops should run now" decision.
//! * [`scheduler_tick`] — one tick: run each due job's next iteration through a
//!   [`LoopExecutor`], fold the outcome back through the existing
//!   [`LoopJob::decide_next`] caps, and persist. `now`/`last_runs` are injected so
//!   it is unit-testable without a clock or a live agent.
//! * [`ProviderLoopExecutor`] — the daemon adapter that runs an iteration against
//!   any [`AIProvider`] (provider-agnostic; never hard-codes a provider).
//!
//! Secrets: unlike the REPL path (which sets process-global env for its own
//! single-user session), hosted execution resolves a job's WorkspaceStore
//! secrets and passes them **scoped** to the executor — the long-lived daemon
//! never mutates its own global environment, so one loop's secrets can't leak
//! into concurrent work.

use crate::loop_engine::{
    load_jobs, resolve_loop_secrets, save_jobs, LoopDecision, LoopJob, LoopMode, LoopStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use vibe_ai::provider::{AIProvider, Message, MessageRole};

/// Executor seam: run one iteration of a hosted loop body. `secrets` are the
/// resolved `(NAME, VALUE)` pairs from the WorkspaceStore, passed scoped so a
/// tool-running iteration can use them without the daemon touching global env.
/// Returns the self-paced "done" signal (ignored for recurring loops).
#[async_trait::async_trait]
pub trait LoopExecutor {
    async fn run_iteration(
        &self,
        prompt: &str,
        secrets: &[(String, String)],
    ) -> Result<bool, String>;
}

/// Which hosted, non-terminal jobs are due to run at `now_secs`.
///
/// A **recurring** job is due when `now - last_run >= interval` (a job that has
/// never run has `last_run = 0` → due immediately). A **self-paced** job is due
/// whenever it isn't terminal — it paces itself via the done-signal and its
/// `max_iter` / `max_duration` caps. Non-hosted and terminal jobs are excluded.
pub fn due_hosted_jobs(
    jobs: &[LoopJob],
    now_secs: u64,
    last_runs: &HashMap<String, u64>,
) -> Vec<String> {
    jobs.iter()
        .filter(|j| j.hosted && !j.status.is_terminal())
        .filter(|j| match j.spec.mode {
            LoopMode::Recurring { interval_secs } => {
                let last = last_runs.get(&j.id).copied().unwrap_or(0);
                now_secs.saturating_sub(last) >= interval_secs
            }
            LoopMode::SelfPaced => true,
        })
        .map(|j| j.id.clone())
        .collect()
}

/// One scheduler tick over the persisted loop-jobs file: run every due hosted
/// job's next iteration, fold the outcome through the loop's caps
/// ([`LoopJob::decide_next`]), stamp `last_runs`, and persist. Returns the ids
/// that ran this tick.
///
/// `now_secs` + `last_runs` are injected for testability; `secret_lookup`
/// resolves a job's declared secret names (a closure over the `WorkspaceStore` at
/// the live call site). A job whose iteration errors is marked `Failed` rather
/// than aborting the tick.
pub async fn scheduler_tick<E, F>(
    jobs_path: &Path,
    now_secs: u64,
    last_runs: &mut HashMap<String, u64>,
    executor: &E,
    secret_lookup: F,
) -> Vec<String>
where
    E: LoopExecutor + Sync,
    F: Fn(&str) -> Option<String>,
{
    let path = jobs_path.to_path_buf();
    let mut jobs = load_jobs(&path);
    let due = due_hosted_jobs(&jobs, now_secs, last_runs);

    for id in &due {
        let idx = match jobs.iter().position(|j| &j.id == id) {
            Some(i) => i,
            None => continue,
        };
        let prompt = jobs[idx].spec.prompt.clone();
        let (secrets, _missing) = resolve_loop_secrets(&jobs[idx].spec.secrets, &secret_lookup);

        match executor.run_iteration(&prompt, &secrets).await {
            Ok(done) => {
                last_runs.insert(id.clone(), now_secs);
                jobs[idx].iterations_done += 1;
                let elapsed = now_secs.saturating_sub(jobs[idx].created_at_secs);
                let decision = jobs[idx].decide_next(done, elapsed);
                if decision != LoopDecision::Continue {
                    jobs[idx].status = LoopJob::status_for(decision);
                }
            }
            Err(_) => {
                last_runs.insert(id.clone(), now_secs);
                jobs[idx].status = LoopStatus::Failed;
            }
        }
    }

    let _ = save_jobs(&path, &jobs);
    due
}

/// Daemon adapter: run a hosted iteration as one chat turn against any provider.
///
/// A plain chat turn doesn't consume `secrets`; they're threaded through for the
/// day a hosted iteration runs an agent with shell tools. Returns `done = false`
/// — recurring loops terminate by interval + `max_iter` / `max_duration`; a
/// self-paced hosted loop relies on those caps (a hosted done-validator is a
/// later refinement).
pub struct ProviderLoopExecutor {
    provider: Arc<dyn AIProvider>,
}

impl ProviderLoopExecutor {
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait::async_trait]
impl LoopExecutor for ProviderLoopExecutor {
    async fn run_iteration(
        &self,
        prompt: &str,
        _secrets: &[(String, String)],
    ) -> Result<bool, String> {
        let messages = vec![Message {
            role: MessageRole::User,
            content: prompt.to_string(),
        }];
        self.provider
            .chat(&messages, None)
            .await
            .map_err(|e| e.to_string())?;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loop_engine::{LoopSpec, DEFAULT_MAX_ITER};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn recurring_job(id: &str, interval: u64, hosted: bool) -> LoopJob {
        let spec = LoopSpec {
            mode: LoopMode::Recurring {
                interval_secs: interval,
            },
            prompt: format!("work {id}"),
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: 3600,
            secrets: Vec::new(),
        };
        let mut j = LoopJob::new(id.to_string(), spec, 0);
        j.hosted = hosted;
        j
    }

    #[test]
    fn due_excludes_non_hosted_terminal_and_not_yet_due() {
        let mut jobs = vec![
            recurring_job("a", 60, true),  // hosted, never ran → due
            recurring_job("b", 60, false), // not hosted → excluded
            recurring_job("c", 60, true),  // hosted but ran recently → not due
        ];
        jobs[2].hosted = true;
        let mut last = HashMap::new();
        last.insert("c".to_string(), 100); // c ran at t=100
        let due = due_hosted_jobs(&jobs, 120, &last); // t=120, c only 20s later
        assert_eq!(due, vec!["a"]);

        // Terminal jobs are never due.
        jobs[0].status = LoopStatus::Stopped;
        assert!(due_hosted_jobs(&jobs, 120, &last).is_empty() || !due_hosted_jobs(&jobs, 120, &last).contains(&"a".to_string()));
    }

    #[test]
    fn recurring_becomes_due_after_interval_elapses() {
        let jobs = vec![recurring_job("a", 60, true)];
        let mut last = HashMap::new();
        last.insert("a".to_string(), 100);
        assert!(due_hosted_jobs(&jobs, 150, &last).is_empty()); // 50s < 60s
        assert_eq!(due_hosted_jobs(&jobs, 160, &last), vec!["a"]); // 60s ≥ 60s
    }

    /// Mock executor counting invocations; returns a configurable done-signal.
    struct CountingExec {
        calls: AtomicU32,
        done: bool,
    }

    #[async_trait::async_trait]
    impl LoopExecutor for CountingExec {
        async fn run_iteration(&self, _p: &str, _s: &[(String, String)]) -> Result<bool, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.done)
        }
    }

    #[tokio::test]
    async fn tick_runs_due_job_updates_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loops.json");
        let jobs = vec![recurring_job("a", 60, true)];
        save_jobs(&path, &jobs).unwrap();

        let exec = CountingExec {
            calls: AtomicU32::new(0),
            done: false,
        };
        let mut last = HashMap::new();
        let ran = scheduler_tick(&path, 1000, &mut last, &exec, |_| None).await;
        assert_eq!(ran, vec!["a"]);
        assert_eq!(exec.calls.load(Ordering::SeqCst), 1);
        assert_eq!(last.get("a"), Some(&1000));

        // Persisted state advanced: one iteration done, still running.
        let reloaded = load_jobs(&path);
        assert_eq!(reloaded[0].iterations_done, 1);
        assert_eq!(reloaded[0].status, LoopStatus::Running);
    }

    #[tokio::test]
    async fn tick_marks_failed_when_executor_errors() {
        struct FailExec;
        #[async_trait::async_trait]
        impl LoopExecutor for FailExec {
            async fn run_iteration(&self, _p: &str, _s: &[(String, String)]) -> Result<bool, String> {
                Err("boom".into())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loops.json");
        save_jobs(&path, &vec![recurring_job("a", 60, true)]).unwrap();
        let mut last = HashMap::new();
        // now=100 ≥ interval 60 (last_run defaults to 0) so the job is due.
        scheduler_tick(&path, 100, &mut last, &FailExec, |_| None).await;
        assert_eq!(load_jobs(&path)[0].status, LoopStatus::Failed);
    }

    #[tokio::test]
    async fn tick_resolves_secrets_and_passes_them_scoped() {
        // A job declaring a secret; the lookup provides it; the executor asserts
        // it arrived (proving scoped injection without touching global env).
        struct SecretAssertExec;
        #[async_trait::async_trait]
        impl LoopExecutor for SecretAssertExec {
            async fn run_iteration(&self, _p: &str, s: &[(String, String)]) -> Result<bool, String> {
                assert_eq!(s, &[("TOK".to_string(), "abc".to_string())]);
                Ok(false)
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loops.json");
        let spec = LoopSpec {
            mode: LoopMode::Recurring { interval_secs: 1 },
            prompt: "p".into(),
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: 3600,
            secrets: vec!["TOK".into()],
        };
        let mut job = LoopJob::new("a".into(), spec, 0);
        job.hosted = true;
        save_jobs(&path, &vec![job]).unwrap();
        let mut last = HashMap::new();
        scheduler_tick(&path, 5, &mut last, &SecretAssertExec, |n| {
            (n == "TOK").then(|| "abc".to_string())
        })
        .await;
    }

    #[tokio::test]
    async fn self_paced_hosted_stops_on_done_signal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loops.json");
        let spec = LoopSpec {
            mode: LoopMode::SelfPaced,
            prompt: "p".into(),
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: 3600,
            secrets: Vec::new(),
        };
        let mut job = LoopJob::new("a".into(), spec, 0);
        job.hosted = true;
        save_jobs(&path, &vec![job]).unwrap();

        let exec = CountingExec {
            calls: AtomicU32::new(0),
            done: true, // validator says "done"
        };
        let mut last = HashMap::new();
        scheduler_tick(&path, 10, &mut last, &exec, |_| None).await;
        assert_eq!(load_jobs(&path)[0].status, LoopStatus::Done);
    }
}
