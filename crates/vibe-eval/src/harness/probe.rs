//! The probe harness: for tasks with no agent turn at all.
//!
//! Surface conformance is not a capability question. "Does the Wear OS client
//! send a bearer token" has no prompt and no model in it — the answer is in
//! the shipped source and in what the daemon does when you call it. Those
//! tasks still belong in the same suite system, because the whole point of the
//! matrix is to show capability and reachability side by side: an agent that
//! scores well through the CLI and is unreachable from the watch is not a
//! working product, and only a report that covers both says so.
//!
//! So this harness runs nothing and reports nothing. The task's grader —
//! `http` probes against a live daemon, or `command`/`files` assertions over
//! the repository — does all the work.

use std::path::Path;
use std::time::Duration;

use super::{Harness, HarnessError, Preflight, RunOutcome};
use crate::task::{EvalTask, Surface};

/// A harness that performs no agent invocation.
pub struct ProbeHarness {
    surface: Surface,
}

impl ProbeHarness {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }
}

#[async_trait::async_trait]
impl Harness for ProbeHarness {
    fn surface(&self) -> Surface {
        self.surface
    }

    fn describe(&self) -> String {
        format!(
            "probe ({} conformance — no agent turn)",
            self.surface.slug()
        )
    }

    async fn preflight(&self) -> Preflight {
        // Nothing to check: the grader owns every dependency this kind of task
        // has, and it reports its own unavailability with a specific reason.
        Preflight::Ready
    }

    async fn run(
        &self,
        _task: &EvalTask,
        _workspace: &Path,
        _timeout: Duration,
    ) -> Result<RunOutcome, HarnessError> {
        // Deliberately empty rather than fabricated. Every transcript grader
        // checks for an empty transcript and reports `error`, so a conformance
        // task that is mistakenly given a transcript assertion is caught
        // instead of vacuously passing.
        Ok(RunOutcome::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Capability, Difficulty, EvalTask, Fixture, Limits, TaskSource};

    fn task() -> EvalTask {
        EvalTask {
            id: "t".into(),
            title: "t".into(),
            capability: Capability::SurfaceConformance,
            difficulty: Difficulty::Easy,
            surfaces: vec![],
            prompt: String::new(),
            fixture: Fixture::default(),
            grader: crate::grade::Grader::AlwaysSkip {
                reason: "test".into(),
            },
            limits: Limits::default(),
            tags: vec![],
            source: TaskSource::Vendored,
            requires: vec![],
            workspace: crate::task::WorkspaceMode::Temp,
        }
    }

    #[tokio::test]
    async fn probe_run_reports_nothing_rather_than_a_plausible_default() {
        let h = ProbeHarness::new(Surface::Watch);
        let dir = tempfile::tempdir().expect("tempdir");
        let out = h
            .run(&task(), dir.path(), Duration::from_secs(1))
            .await
            .expect("probe run");
        // An invented `outcome: "success"` here would make every conformance
        // task with an `outcome_is` assertion pass without evidence.
        assert_eq!(out.outcome, None);
        assert!(out.steps.is_empty());
        assert!(out.final_text.is_empty());
    }

    #[tokio::test]
    async fn probe_preflight_is_always_ready() {
        assert_eq!(
            ProbeHarness::new(Surface::Mobile).preflight().await,
            Preflight::Ready
        );
    }
}
