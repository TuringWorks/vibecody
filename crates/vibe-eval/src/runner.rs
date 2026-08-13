//! Orchestration: load suites, expand them across surfaces, run, grade.
//!
//! The runner's job is to make sure every reason a task did not pass is
//! recorded as the reason it actually was. There are five distinct ways a task
//! can fail to produce a pass — no harness for its surface, the surface is
//! down, a required tool is missing, the workspace could not be built, or the
//! agent genuinely got it wrong — and only the last one is a capability
//! result. Collapsing them is how an eval harness produces confident nonsense.

use futures::stream::StreamExt;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::grade::{GradeContext, GradeResult, JudgeModel, Verdict};
use crate::harness::{read_daemon_token, Harness, HarnessError, Preflight};
use crate::report::{EvalReport, RunConfigSummary, TaskResult};
use crate::suite::{self, Suite};
use crate::task::{Capability, Difficulty, Surface, TaskRef, WorkspaceMode};

/// Which tasks to run. Every field is "no constraint" when empty.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub suites: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub surfaces: Vec<Surface>,
    pub difficulties: Vec<Difficulty>,
    pub tags: Vec<String>,
    /// Substring match against `<suite>/<task-id>`.
    pub task_contains: Option<String>,
    /// Cap on how many tasks run, applied after every other filter. Recorded
    /// in the report when it actually truncated something, because a silently
    /// sampled run reads exactly like a complete one.
    pub limit: Option<usize>,
}

impl Filter {
    fn accepts(&self, task_ref: &TaskRef) -> bool {
        let t = &task_ref.task;
        (self.suites.is_empty() || self.suites.contains(&task_ref.suite))
            && (self.capabilities.is_empty() || self.capabilities.contains(&t.capability))
            && (self.difficulties.is_empty() || self.difficulties.contains(&t.difficulty))
            && (self.tags.is_empty() || t.tags.iter().any(|tag| self.tags.contains(tag)))
            && self
                .task_contains
                .as_ref()
                .is_none_or(|needle| task_ref.key().contains(needle.as_str()))
    }
}

/// One unit of work: a task, on a surface, for one repetition.
///
/// A named struct rather than a tuple because the fourth element is a sample
/// index, and `(TaskRef, Surface, PathBuf, usize)` gives a reader no way to
/// know that.
#[derive(Debug, Clone)]
pub struct PlannedRun {
    pub task: TaskRef,
    pub surface: Surface,
    /// Directory the task's suite was loaded from; fixture paths resolve
    /// against it.
    pub suite_dir: PathBuf,
    /// Zero-based repetition index.
    pub sample: usize,
}

pub struct RunConfig {
    pub suites_dir: PathBuf,
    /// Repository root, needed by `repo_root` conformance tasks.
    pub repo_root: Option<PathBuf>,
    pub filter: Filter,
    pub concurrency: usize,
    /// Budget for a task that declares none of its own.
    pub default_timeout: Duration,
    /// Hard ceiling the operator asked for, overriding any longer per-task or
    /// per-suite budget.
    ///
    /// Separate from `default_timeout` because the two are different requests.
    /// A default is a fallback and loses to a suite that sets its own; a cap is
    /// an instruction and must win. Conflating them made `--timeout 600` inert
    /// against every shipped suite — they all set `defaults.timeout_secs`, so
    /// the fallback was never consulted and a task ran for its full 900s while
    /// the operator was told it had 600. `None` means no ceiling.
    pub timeout_cap: Option<Duration>,
    /// How many times to run each task/surface pair.
    ///
    /// Agent runs are not deterministic, and the spread is not small: the
    /// greenfield build scored a complete 8/8 in 65s on one attempt and
    /// produced no file at all in 900s on the next three. A single run is
    /// therefore weak evidence, and a harness that only ever takes one has no
    /// way to say so. `1` keeps the cheap default; higher values let the
    /// report separate "this fails" from "this fails four times in five".
    pub samples: usize,
    /// Base URL `{daemon}` expands to in HTTP probes.
    pub daemon_base_url: Option<String>,
    pub judge: Option<Arc<dyn JudgeModel>>,
    /// Recorded in the report so a score is never quoted without its setup.
    pub provider: String,
    pub model: String,
    /// A stable identifier for this run.
    pub run_id: String,
}

impl RunConfig {
    /// The budget a task actually gets: its own if it declares one, otherwise
    /// the run default — then clamped by an explicit `--timeout` ceiling.
    pub fn effective_timeout(&self, task_secs: Option<u64>) -> Duration {
        let requested = task_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);
        match self.timeout_cap {
            Some(cap) => requested.min(cap),
            None => requested,
        }
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            suites_dir: PathBuf::from("evals/suites"),
            repo_root: None,
            filter: Filter::default(),
            // Deliberately low. Eval tasks spawn compilers and test runners;
            // oversubscribing turns a capability measurement into a
            // measurement of the machine's load average, and timeouts start
            // firing on tasks that would have passed.
            concurrency: 4,
            default_timeout: Duration::from_secs(600),
            timeout_cap: None,
            samples: 1,
            daemon_base_url: None,
            judge: None,
            provider: String::new(),
            model: String::new(),
            run_id: String::new(),
        }
    }
}

pub struct Runner {
    harnesses: HashMap<Surface, Arc<dyn Harness>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            harnesses: HashMap::new(),
        }
    }

    pub fn with_harness(mut self, harness: Arc<dyn Harness>) -> Self {
        self.harnesses.insert(harness.surface(), harness);
        self
    }

    pub fn surfaces(&self) -> Vec<Surface> {
        let mut s: Vec<Surface> = self.harnesses.keys().copied().collect();
        s.sort();
        s
    }

    /// Load, filter, and expand suites into the concrete work list.
    ///
    /// Split out from [`Runner::run`] so `eval list` shows exactly what a
    /// `run` with the same filter would execute — a preview that disagrees
    /// with the run is worse than none.
    pub fn plan(&self, config: &RunConfig) -> (Vec<PlannedRun>, Vec<String>) {
        let (suites, load_errors) = suite::load_dir(&config.suites_dir);
        let load_errors: Vec<String> = load_errors.iter().map(ToString::to_string).collect();

        let base_dirs: BTreeMap<String, PathBuf> = suites
            .iter()
            .map(|s| (s.id.clone(), s.base_dir.clone()))
            .collect();

        let mut work: Vec<PlannedRun> = suites
            .iter()
            .flat_map(Suite::task_refs)
            .filter(|t| config.filter.accepts(t))
            .flat_map(|task_ref| {
                let base = base_dirs
                    .get(&task_ref.suite)
                    .cloned()
                    .unwrap_or_else(|| config.suites_dir.clone());
                // A task with no surfaces would silently vanish; give it the
                // CLI, the one surface that always owns an agent loop.
                let surfaces = if task_ref.task.surfaces.is_empty() {
                    vec![Surface::Cli]
                } else {
                    task_ref.task.surfaces.clone()
                };
                surfaces
                    .into_iter()
                    .filter(|s| {
                        config.filter.surfaces.is_empty() || config.filter.surfaces.contains(s)
                    })
                    .flat_map(|s| {
                        let task_ref = task_ref.clone();
                        let base = base.clone();
                        (0..config.samples.max(1))
                            .map(move |n| PlannedRun {
                                task: task_ref.clone(),
                                surface: s,
                                suite_dir: base.clone(),
                                sample: n,
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        work.sort_by_key(|p| (p.task.key(), p.surface, p.sample));
        if let Some(limit) = config.filter.limit {
            work.truncate(limit);
        }
        (work, load_errors)
    }

    pub async fn run(&self, config: &RunConfig) -> EvalReport {
        let started_at_unix = unix_now();
        let (work, load_errors) = self.plan(config);

        // Preflight each surface once. Doing it per task would multiply a
        // stopped daemon into hundreds of identical probes, and — worse —
        // would let a surface that dies mid-run produce a mix of capability
        // failures and availability skips for the same cause.
        let mut preflights: HashMap<Surface, Preflight> = HashMap::new();
        let needed: Vec<Surface> = {
            let mut s: Vec<Surface> = work.iter().map(|p| p.surface).collect();
            s.sort();
            s.dedup();
            s
        };
        for surface in needed {
            let state = match self.harnesses.get(&surface) {
                Some(h) => h.preflight().await,
                None => Preflight::unavailable(format!(
                    "no harness registered for the {} surface in this run",
                    surface.slug()
                )),
            };
            preflights.insert(surface, state);
        }

        // Read once per run rather than per probe: the token is stable for a
        // daemon's lifetime, and re-reading it hundreds of times would hide a
        // mid-run rotation behind inconsistent results instead of a clean
        // wave of 401s.
        let daemon_token = read_daemon_token();

        let results: Vec<TaskResult> = futures::stream::iter(work.into_iter().map(|planned| {
            let preflight = preflights.get(&planned.surface).cloned();
            let harness = self.harnesses.get(&planned.surface).cloned();
            let token = daemon_token.clone();
            async move {
                self.run_one(config, planned, preflight, harness, token)
                    .await
            }
        }))
        .buffer_unordered(config.concurrency.max(1))
        .collect()
        .await;

        let mut results = results;
        results.sort_by(|a, b| {
            (a.key.as_str(), a.surface, a.sample).cmp(&(b.key.as_str(), b.surface, b.sample))
        });

        EvalReport {
            run_id: if config.run_id.is_empty() {
                format!("run-{}", started_at_unix)
            } else {
                config.run_id.clone()
            },
            started_at_unix,
            finished_at_unix: unix_now(),
            config: RunConfigSummary {
                provider: config.provider.clone(),
                model: config.model.clone(),
                surfaces: self.surfaces(),
                suites: {
                    let mut s: Vec<String> = results.iter().map(|r| r.suite.clone()).collect();
                    s.sort();
                    s.dedup();
                    s
                },
                concurrency: config.concurrency,
                judge: config.judge.as_ref().map(|j| j.describe()),
                harnesses: {
                    let mut h: Vec<String> =
                        self.harnesses.values().map(|x| x.describe()).collect();
                    h.sort();
                    h
                },
            },
            results,
            load_errors,
        }
    }

    async fn run_one(
        &self,
        config: &RunConfig,
        planned: PlannedRun,
        preflight: Option<Preflight>,
        harness: Option<Arc<dyn Harness>>,
        daemon_token: Option<String>,
    ) -> TaskResult {
        let PlannedRun {
            task: task_ref,
            surface,
            suite_dir: base_dir,
            sample,
        } = planned;
        let started = std::time::Instant::now();
        let task = &task_ref.task;
        let describe = harness
            .as_ref()
            .map(|h| h.describe())
            .unwrap_or_else(|| format!("(no harness for {})", surface.slug()));

        let finish =
            |verdict: Verdict, grade: Option<GradeResult>, score: Option<f64>| TaskResult {
                key: task_ref.key(),
                suite: task_ref.suite.clone(),
                task_id: task.id.clone(),
                title: task.title.clone(),
                capability: task.capability,
                difficulty: task.difficulty,
                surface,
                verdict,
                score,
                grade,
                duration_ms: started.elapsed().as_millis() as u64,
                harness: describe.clone(),
                source: task.source.clone(),
                sample,
            };

        // 1. Is there a harness at all?
        let Some(harness) = harness else {
            return finish(
                Verdict::Skipped {
                    reason: format!("no harness registered for the {} surface", surface.slug()),
                },
                None,
                None,
            );
        };

        // 2. Is the surface usable? Its own words, not a generic message.
        if let Some(Preflight::Unavailable { reason }) = preflight {
            return finish(Verdict::Skipped { reason }, None, None);
        }

        // 3. Are the tools the task needs actually present?
        if let Some(missing) = task.requires.iter().find(|tool| !on_path(tool)) {
            return finish(
                Verdict::Skipped {
                    reason: format!(
                        "`{}` is not on PATH — this task needs it to be gradeable",
                        missing
                    ),
                },
                None,
                None,
            );
        }

        // 4. Can the workspace be built? A fixture that will not materialise
        //    is our bug, so it errors rather than failing the agent.
        let prepared =
            match crate::workspace::prepare(task, &base_dir, config.repo_root.as_deref()).await {
                Ok(ws) => ws,
                Err(e) => {
                    return finish(
                        Verdict::Error {
                            reason: format!("workspace preparation failed: {}", e),
                        },
                        None,
                        None,
                    )
                }
            };

        // 5. Run the agent — unless this task has no prompt, in which case it
        //    is a conformance check and the grader does everything.
        let timeout = config.effective_timeout(task.limits.timeout_secs);

        // A timeout does not discard the work. The agent was cut off, but
        // whatever it built is on disk, and grading it is the difference
        // between "ran out of time" and "ran out of time having got 5 of 8
        // milestones working". Returning `Some(0.0)` here instead — as this
        // did — asserts a completion depth of zero that nobody measured, on
        // exactly the long-horizon tasks the ladder exists to score.
        let mut timed_out_after: Option<(u64, String)> = None;

        let run_outcome =
            if task.prompt.trim().is_empty() || task.workspace == WorkspaceMode::RepoRoot {
                crate::harness::RunOutcome::default()
            } else {
                match harness.run(task, &prepared.root, timeout).await {
                    Ok(outcome) => outcome,
                    Err(HarnessError::Timeout { secs, tail }) => {
                        timed_out_after = Some((secs, tail));
                        // No transcript survives a killed run, so the outcome
                        // stays empty rather than being invented; transcript
                        // assertions will report `error` for this task, which
                        // is the truth — we never saw what it did.
                        crate::harness::RunOutcome::default()
                    }
                    Err(e) => {
                        // Spawn / transport / protocol problems are ours.
                        return finish(
                            Verdict::Error {
                                reason: format!("{} surface: {}", surface.slug(), e),
                            },
                            None,
                            None,
                        );
                    }
                }
            };

        // 6. Grade.
        let ctx = GradeContext {
            workspace: &prepared.root,
            baseline: prepared.baseline.clone(),
            run: &run_outcome,
            prompt: &task.prompt,
            judge: config.judge.as_deref(),
            default_timeout: timeout,
            daemon_base_url: config.daemon_base_url.clone(),
            daemon_token,
        };
        let grade = task.grader.grade(&ctx).await;
        let score = grade.score;

        let verdict = match timed_out_after {
            None => grade.verdict.clone(),
            // The budget was the operator's instruction, so blowing it stays a
            // failure even when the work on disk happens to be complete —
            // reporting a pass would hide that the run never terminated. The
            // measured depth rides along in `score` and in the grade tree, so
            // the report can say how far it got without calling it a win.
            Some((secs, tail)) => {
                let depth = match score {
                    Some(d) => format!(
                        "the work it left behind grades at {:.0}% completion",
                        d * 100.0
                    ),
                    None => "its workspace could not be graded".to_string(),
                };
                // The tail is what distinguishes "the agent achieved nothing"
                // from "the provider stopped answering half an hour in", and
                // those call for opposite responses.
                let evidence = if tail.trim().is_empty() {
                    String::new()
                } else {
                    format!("; last output before the kill: {}", tail.trim())
                };
                Verdict::Fail {
                    reason: format!("agent exceeded its {}s budget; {}{}", secs, depth, evidence),
                }
            }
        };
        finish(verdict, Some(grade), score)
    }
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether `tool` is an executable on `PATH`.
fn on_path(tool: &str) -> bool {
    if tool.contains('/') {
        return Path::new(tool).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(tool);
                candidate.is_file()
                    || candidate.with_extension("exe").is_file()
                    || candidate.with_extension("cmd").is_file()
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{ProbeHarness, RunOutcome};
    use crate::task::EvalTask;

    const SUITE: &str = r#"
id: demo
title: Demo
default_surfaces: [cli]
tasks:
  - id: writes-file
    title: Writes a file
    capability: code_generation
    difficulty: easy
    tags: [quick]
    prompt: create out.txt
    grader:
      type: files
      assertions:
        - assert: exists
          path: out.txt
  - id: needs-missing-tool
    title: Needs a tool nobody has
    capability: code_repair
    difficulty: hard
    prompt: do something
    requires: [definitely-not-a-real-binary-xyzzy]
    grader:
      type: files
      assertions:
        - assert: exists
          path: whatever
"#;

    fn suites_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("demo.yaml"), SUITE).expect("write");
        dir
    }

    /// A harness that pretends to be the CLI and writes whatever the test says.
    struct FakeCli {
        creates: Option<String>,
    }

    #[async_trait::async_trait]
    impl Harness for FakeCli {
        fn surface(&self) -> Surface {
            Surface::Cli
        }
        fn describe(&self) -> String {
            "fake cli".to_string()
        }
        async fn preflight(&self) -> Preflight {
            Preflight::Ready
        }
        async fn run(
            &self,
            _task: &EvalTask,
            workspace: &Path,
            _timeout: Duration,
        ) -> Result<RunOutcome, HarnessError> {
            if let Some(name) = &self.creates {
                std::fs::write(workspace.join(name), "done")
                    .map_err(|e| HarnessError::Spawn(e.to_string()))?;
            }
            Ok(RunOutcome {
                final_text: "did it".to_string(),
                outcome: Some("success".to_string()),
                ..RunOutcome::default()
            })
        }
    }

    fn config(dir: &Path) -> RunConfig {
        RunConfig {
            suites_dir: dir.to_path_buf(),
            concurrency: 2,
            default_timeout: Duration::from_secs(30),
            ..RunConfig::default()
        }
    }

    #[tokio::test]
    async fn a_passing_task_passes_and_a_missing_tool_skips() {
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(FakeCli {
            creates: Some("out.txt".to_string()),
        }));
        let report = runner.run(&config(dir.path())).await;

        let overall = report.overall();
        assert_eq!(overall.passed, 1, "{:#?}", report.results);
        assert_eq!(overall.skipped, 1);
        // The skipped task must not drag the rate down.
        assert_eq!(overall.pass_rate(), Some(1.0));

        let skipped = report
            .results
            .iter()
            .find(|r| r.task_id == "needs-missing-tool")
            .expect("task present");
        assert!(
            skipped
                .verdict
                .reason()
                .unwrap_or_default()
                .contains("not on PATH"),
            "reason should name the cause: {:?}",
            skipped.verdict
        );
    }

    #[tokio::test]
    async fn a_task_the_agent_does_not_solve_fails() {
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(FakeCli { creates: None }));
        let report = runner.run(&config(dir.path())).await;
        assert_eq!(report.overall().failed, 1);
        assert_eq!(report.overall().pass_rate(), Some(0.0));
    }

    #[tokio::test]
    async fn an_unregistered_surface_skips_with_a_specific_reason() {
        let dir = suites_dir();
        // No harness at all.
        let report = Runner::new().run(&config(dir.path())).await;
        assert_eq!(report.overall().passed, 0);
        assert_eq!(report.overall().pass_rate(), None, "nothing was measured");
        let reason = report.results[0]
            .verdict
            .reason()
            .unwrap_or_default()
            .to_string();
        assert!(reason.contains("no harness registered"), "{}", reason);
    }

    #[tokio::test]
    async fn an_unavailable_surface_skips_rather_than_fails() {
        struct Down;
        #[async_trait::async_trait]
        impl Harness for Down {
            fn surface(&self) -> Surface {
                Surface::Cli
            }
            fn describe(&self) -> String {
                "down".into()
            }
            async fn preflight(&self) -> Preflight {
                Preflight::unavailable("daemon not running on :7878")
            }
            async fn run(
                &self,
                _t: &EvalTask,
                _w: &Path,
                _d: Duration,
            ) -> Result<RunOutcome, HarnessError> {
                panic!("must not run when preflight failed")
            }
        }
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(Down));
        let report = runner.run(&config(dir.path())).await;
        assert_eq!(report.overall().failed, 0);
        assert_eq!(report.overall().skipped, 2);
        assert!(report.results[0]
            .verdict
            .reason()
            .unwrap_or_default()
            .contains(":7878"));
    }

    #[tokio::test]
    async fn a_harness_timeout_is_a_scored_failure_not_an_error() {
        struct Slow;
        #[async_trait::async_trait]
        impl Harness for Slow {
            fn surface(&self) -> Surface {
                Surface::Cli
            }
            fn describe(&self) -> String {
                "slow".into()
            }
            async fn preflight(&self) -> Preflight {
                Preflight::Ready
            }
            async fn run(
                &self,
                _t: &EvalTask,
                _w: &Path,
                _d: Duration,
            ) -> Result<RunOutcome, HarnessError> {
                Err(HarnessError::Timeout {
                    secs: 600,
                    tail: String::new(),
                })
            }
        }
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(Slow));
        let report = runner.run(&config(dir.path())).await;
        // The agent had a budget and blew it: that is a real result.
        assert_eq!(report.overall().failed, 1);
        assert_eq!(report.overall().errored, 0);
    }

    #[tokio::test]
    async fn a_timed_out_run_is_still_graded_for_partial_progress() {
        // Observed on the greenfield build: the agent ran the full hour, was
        // cut off, and the run recorded `score: 0.0` with no grade tree — an
        // assertion of zero completion that nothing had measured, on the one
        // suite whose whole point is measuring how far it got.
        struct SlowButProductive;
        #[async_trait::async_trait]
        impl Harness for SlowButProductive {
            fn surface(&self) -> Surface {
                Surface::Cli
            }
            fn describe(&self) -> String {
                "slow".into()
            }
            async fn preflight(&self) -> Preflight {
                Preflight::Ready
            }
            async fn run(
                &self,
                _t: &EvalTask,
                workspace: &Path,
                _d: Duration,
            ) -> Result<RunOutcome, HarnessError> {
                // Real work landed on disk before the clock ran out.
                std::fs::write(workspace.join("out.txt"), "done")
                    .map_err(|e| HarnessError::Spawn(e.to_string()))?;
                Err(HarnessError::Timeout {
                    secs: 3600,
                    tail: "last output before the kill".into(),
                })
            }
        }

        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(SlowButProductive));
        let mut cfg = config(dir.path());
        cfg.filter.task_contains = Some("writes-file".to_string());
        let report = runner.run(&cfg).await;

        let r = &report.results[0];
        // Still a failure: the budget was an instruction, and a run that never
        // terminated must not be reported as a win.
        assert!(matches!(r.verdict, Verdict::Fail { .. }), "{:?}", r.verdict);
        // …but the work it left behind was measured rather than assumed.
        assert_eq!(r.score, Some(1.0), "partial progress must be graded");
        assert!(r.grade.is_some(), "the grade tree must survive a timeout");
        let reason = r.verdict.reason().unwrap_or_default();
        assert!(reason.contains("3600s budget"), "{}", reason);
        assert!(reason.contains("100% completion"), "{}", reason);
    }

    #[tokio::test]
    async fn a_transport_error_is_an_error_not_a_failure() {
        struct Broken;
        #[async_trait::async_trait]
        impl Harness for Broken {
            fn surface(&self) -> Surface {
                Surface::Cli
            }
            fn describe(&self) -> String {
                "broken".into()
            }
            async fn preflight(&self) -> Preflight {
                Preflight::Ready
            }
            async fn run(
                &self,
                _t: &EvalTask,
                _w: &Path,
                _d: Duration,
            ) -> Result<RunOutcome, HarnessError> {
                Err(HarnessError::Transport("401".into()))
            }
        }
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(Broken));
        let report = runner.run(&config(dir.path())).await;
        assert_eq!(report.overall().errored, 1);
        assert_eq!(report.overall().failed, 0);
        assert_eq!(report.overall().pass_rate(), None);
    }

    #[test]
    fn plan_respects_filters_and_matches_what_run_would_do() {
        let dir = suites_dir();
        let runner = Runner::new().with_harness(Arc::new(FakeCli { creates: None }));
        let mut cfg = config(dir.path());
        cfg.filter.tags = vec!["quick".to_string()];
        let (work, errors) = runner.plan(&cfg);
        assert!(errors.is_empty());
        assert_eq!(work.len(), 1);
        assert_eq!(work[0].task.task.id, "writes-file");
    }

    #[test]
    fn plan_applies_the_limit_last() {
        let dir = suites_dir();
        let runner = Runner::new();
        let mut cfg = config(dir.path());
        cfg.filter.limit = Some(1);
        let (work, _) = runner.plan(&cfg);
        assert_eq!(work.len(), 1);
    }

    #[tokio::test]
    async fn a_conformance_task_runs_without_an_agent_turn() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("conf.yaml"),
            r#"
id: conf
title: Conformance
default_surfaces: [watch]
tasks:
  - id: repo-has-cargo-toml
    title: Repo has a Cargo.toml
    capability: surface_conformance
    difficulty: easy
    workspace: repo_root
    grader:
      type: files
      assertions:
        - assert: exists
          path: Cargo.toml
"#,
        )
        .expect("write");

        let repo = tempfile::tempdir().expect("repo");
        std::fs::write(repo.path().join("Cargo.toml"), "[package]").expect("write");

        let runner = Runner::new().with_harness(Arc::new(ProbeHarness::new(Surface::Watch)));
        let mut cfg = config(dir.path());
        cfg.repo_root = Some(repo.path().to_path_buf());
        let report = runner.run(&cfg).await;
        assert_eq!(report.overall().passed, 1, "{:#?}", report.results);
    }

    #[test]
    fn an_explicit_timeout_caps_a_longer_suite_budget() {
        // Observed for real: every shipped suite sets `defaults.timeout_secs:
        // 900`, so `--timeout 600` was never consulted and a task burned its
        // full 900s. A flag that silently does nothing is worse than no flag.
        let mut cfg = RunConfig {
            default_timeout: Duration::from_secs(600),
            ..RunConfig::default()
        };
        assert_eq!(
            cfg.effective_timeout(Some(900)),
            Duration::from_secs(900),
            "with no cap, the task's own budget stands"
        );

        cfg.timeout_cap = Some(Duration::from_secs(600));
        assert_eq!(
            cfg.effective_timeout(Some(900)),
            Duration::from_secs(600),
            "an explicit cap must beat a longer per-task budget"
        );
        // A cap is a ceiling, not an assignment: a task that asks for less
        // keeps its shorter budget.
        assert_eq!(cfg.effective_timeout(Some(30)), Duration::from_secs(30));
        // And a task declaring nothing falls back to the run default, still
        // clamped.
        assert_eq!(cfg.effective_timeout(None), Duration::from_secs(600));
    }

    #[test]
    fn on_path_finds_a_ubiquitous_binary_and_rejects_a_fake_one() {
        assert!(on_path("sh"));
        assert!(!on_path("definitely-not-a-real-binary-xyzzy"));
    }
}
