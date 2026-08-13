//! The CLI surface: `vibecli --exec`, run as a subprocess in the task workspace.
//!
//! This is the reference harness. It is the only surface that owns the agent
//! loop *and* a filesystem workspace in one process, so it is where a
//! coding-capability number is cheapest and most trustworthy to obtain.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{Harness, HarnessError, Preflight, RunOutcome, StepRecord};
use crate::task::{EvalTask, Surface};

/// How to invoke the CLI under test.
#[derive(Debug, Clone)]
pub struct CliConfig {
    /// Path to the binary, or a bare name to resolve on `PATH`.
    pub binary: PathBuf,
    pub provider: String,
    pub model: Option<String>,
    /// Extra flags appended verbatim — an escape hatch for evaluating a
    /// configuration the harness does not model (e.g. `--plan`).
    pub extra_args: Vec<String>,
    /// Environment overrides applied to the child.
    pub env: BTreeMap<String, String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("vibecli"),
            // Zero-config: ollama needs no API key, so a fresh checkout can run
            // the suites without the operator configuring anything first.
            provider: "ollama".to_string(),
            model: None,
            extra_args: Vec::new(),
            env: BTreeMap::new(),
        }
    }
}

pub struct CliHarness {
    config: CliConfig,
}

impl CliHarness {
    pub fn new(config: CliConfig) -> Self {
        Self { config }
    }

    /// Resolve the binary to an **absolute** path.
    ///
    /// Absolute matters twice over. It makes the "not found" message name a
    /// path rather than repeat the bare word the operator typed — and, more
    /// importantly, the child runs with `current_dir` set to the task
    /// workspace, so a relative path like `target/debug/vibecli` would be
    /// resolved against the throwaway fixture directory and fail to spawn.
    /// That surfaced as every task erroring with a bare "No such file or
    /// directory".
    fn resolve_binary(&self) -> Option<PathBuf> {
        let bin = &self.config.binary;
        if bin.components().count() > 1 || bin.is_absolute() {
            if !bin.exists() {
                return None;
            }
            // Canonicalise against the *current* directory, before the child
            // gets its own.
            return std::fs::canonicalize(bin)
                .ok()
                .or_else(|| Some(bin.clone()));
        }
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|dir| dir.join(bin))
            .find(|candidate| candidate.is_file())
            .and_then(|found| std::fs::canonicalize(&found).ok().or(Some(found)))
    }
}

#[async_trait::async_trait]
impl Harness for CliHarness {
    fn surface(&self) -> Surface {
        Surface::Cli
    }

    fn describe(&self) -> String {
        let model = self.config.model.as_deref().unwrap_or("(provider default)");
        format!(
            "cli {} · provider={} model={}",
            self.config.binary.display(),
            self.config.provider,
            model
        )
    }

    async fn preflight(&self) -> Preflight {
        match self.resolve_binary() {
            Some(_) => Preflight::Ready,
            None => Preflight::unavailable(format!(
                "`{}` is not on PATH — build it with `cargo build --release -p vibecli` \
                 or pass --binary <path>",
                self.config.binary.display()
            )),
        }
    }

    async fn run(
        &self,
        task: &EvalTask,
        workspace: &Path,
        timeout: Duration,
    ) -> Result<RunOutcome, HarnessError> {
        let binary = self.resolve_binary().ok_or_else(|| {
            HarnessError::Spawn(format!("{} not found", self.config.binary.display()))
        })?;

        // The report goes to a file rather than stdout. `--exec` also writes
        // progress and provider notices to the same terminal, and scraping a
        // JSON document out of interleaved output is exactly the kind of
        // parsing that works until a provider adds one warning line.
        let report_path = workspace.join(".vibe-eval-report.json");
        let _ = std::fs::remove_file(&report_path);

        let mut args: Vec<String> = vec![
            "--exec".to_string(),
            task.prompt.clone(),
            // Non-interactive: `suggest` (the default) exits 3 asking for a
            // human, which would score every task as a failure.
            "--full-auto".to_string(),
            // `verbose` rather than `json`: both write the same JSON document
            // to `--output`, but verbose also streams each tool call to stderr
            // as it happens. Without that a killed run has nothing to say for
            // itself — the first timed-out builds reported "0% completion" and
            // an empty tail, which is indistinguishable from a hung provider.
            // The capture buffers keep only the last 64KB, so a chatty run
            // cannot grow without bound.
            "--output-format".to_string(),
            "verbose".to_string(),
            "--output".to_string(),
            report_path.to_string_lossy().to_string(),
            "--provider".to_string(),
            self.config.provider.clone(),
        ];
        if let Some(model) = &self.config.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        if task.limits.no_network.unwrap_or(false) {
            args.push("--no-network".to_string());
        }
        args.extend(self.config.extra_args.iter().cloned());

        let mut command = tokio::process::Command::new(&binary);
        command
            .args(&args)
            .current_dir(workspace)
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .envs(&self.config.env);

        let started = std::time::Instant::now();
        // Streamed into buffers rather than collected with `output()`, because
        // `output()` is cancelled by the timeout and takes every byte with it.
        // On a long run that is the whole diagnosis: what the agent was doing
        // for the twenty minutes before it was killed.
        let mut child = command
            .spawn()
            .map_err(|e| HarnessError::Spawn(e.to_string()))?;
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let stdout_buf = Arc::new(Mutex::new(String::new()));
        let stderr_buf = Arc::new(Mutex::new(String::new()));
        let stdout_reader = spawn_capture(stdout_pipe, Arc::clone(&stdout_buf));
        let stderr_reader = spawn_capture(stderr_pipe, Arc::clone(&stderr_buf));

        let status = match tokio::time::timeout(timeout, child.wait()).await {
            Err(_) => {
                let _ = child.kill().await;
                // Give the readers a moment to drain what is already buffered.
                let _ = tokio::time::timeout(Duration::from_secs(5), stdout_reader).await;
                let _ = tokio::time::timeout(Duration::from_secs(5), stderr_reader).await;
                let captured = format!(
                    "{}\n{}",
                    stdout_buf.lock().map(|s| s.clone()).unwrap_or_default(),
                    stderr_buf.lock().map(|s| s.clone()).unwrap_or_default()
                );
                return Err(HarnessError::Timeout {
                    secs: timeout.as_secs(),
                    tail: tail(captured.trim(), 2000),
                });
            }
            Ok(Err(e)) => return Err(HarnessError::Spawn(e.to_string())),
            Ok(Ok(status)) => status,
        };
        let _ = stdout_reader.await;
        let _ = stderr_reader.await;

        let duration_ms = started.elapsed().as_millis() as u64;
        let exit_code = status.code();
        let stderr = stderr_buf.lock().map(|s| s.clone()).unwrap_or_default();
        let stdout = stdout_buf.lock().map(|s| s.clone()).unwrap_or_default();

        // A non-zero exit is normal here: `--exec` maps its own outcome onto
        // the exit code (1 = partial, 2 = failed). Only the absence of a
        // report is a harness-level problem.
        let report_text = std::fs::read_to_string(&report_path).ok().or_else(|| {
            // Fall back to stdout in case a future build stops honouring
            // `--output`; better to parse awkwardly than to lose the run.
            stdout
                .find('{')
                .map(|start| stdout[start..].to_string())
                .filter(|s| serde_json::from_str::<serde_json::Value>(s).is_ok())
        });
        let _ = std::fs::remove_file(&report_path);

        let Some(report_text) = report_text else {
            return Err(HarnessError::Protocol(format!(
                "`--exec` wrote no JSON report (exit {}). stderr: {}",
                exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string()),
                tail(&stderr, 800)
            )));
        };

        let report: CiReportShape = serde_json::from_str(&report_text)
            .map_err(|e| HarnessError::Protocol(format!("malformed --exec report: {}", e)))?;

        // A run that never made a tool call and failed with a provider error
        // did not exercise the agent at all. `--exec` reports it as
        // `outcome: "failed"` with the provider's message in the summary,
        // which is structurally identical to "the agent tried and got it
        // wrong" — and scoring it as such is precisely how an out-of-memory
        // model or an expired key turns into a fabricated capability
        // regression. Reported as a harness error instead, so it stays out of
        // the pass-rate denominator.
        if let Some(reason) = provider_failure(&report) {
            return Err(HarnessError::Transport(reason));
        }

        Ok(RunOutcome {
            final_text: report.summary.clone(),
            outcome: Some(report.outcome.clone()),
            steps: report
                .steps
                .into_iter()
                .map(|s| StepRecord {
                    tool: s.tool,
                    input_summary: s.input_summary,
                    output: s.output,
                    success: s.success,
                    duration_ms: s.duration_ms,
                })
                .collect(),
            // The CLI's own duration excludes process startup; the wall clock
            // is what an operator waits, so that is what gets reported.
            duration_ms,
            exit_code,
            raw: serde_json::from_str(&report_text).ok(),
        })
    }
}

/// Mirror of `vibecli`'s `CiReport`, kept structurally lenient on purpose.
///
/// This crate must not depend on the `vibecli` binary crate, so the contract
/// is restated rather than imported. Everything but `outcome` and `summary`
/// carries a default so a field added on the vibecli side does not break the
/// harness — and `tests/exec_contract.rs` pins the shape against the real
/// serializer so drift is caught rather than silently tolerated.
#[derive(Debug, serde::Deserialize)]
struct CiReportShape {
    #[serde(default)]
    #[allow(dead_code)]
    task: String,
    outcome: String,
    #[serde(default)]
    steps: Vec<CiStepShape>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    #[allow(dead_code)]
    duration_ms: u64,
}

#[derive(Debug, serde::Deserialize)]
struct CiStepShape {
    #[serde(default)]
    tool: String,
    #[serde(default)]
    input_summary: String,
    #[serde(default)]
    output: String,
    #[serde(default)]
    success: bool,
    #[serde(default)]
    duration_ms: u64,
}

/// Whether a failed run failed for infrastructure reasons rather than agent
/// ones, returning the reason if so.
///
/// Two conditions must both hold, and the conjunction is what makes this safe:
///
/// 1. **Zero tool calls.** The agent never acted. Whatever went wrong happened
///    before it could demonstrate anything.
/// 2. **The summary names a transport-level failure.** Pattern matching on
///    provider messages is inherently approximate, so it is used only to
///    *upgrade* a fail into an error, never the reverse.
///
/// Getting this wrong in the conservative direction costs one measurement.
/// Getting it wrong the other way invents a capability regression out of an
/// unset API key — which is the mistake that makes a whole report untrustworthy.
fn provider_failure(report: &CiReportShape) -> Option<String> {
    if !report.outcome.eq_ignore_ascii_case("failed") || !report.steps.is_empty() {
        return None;
    }
    let summary = report.summary.to_lowercase();
    const SIGNATURES: &[&str] = &[
        "api key",
        "unauthorized",
        "401",
        "403",
        "429",
        "500 internal server error",
        "502",
        "503",
        "rate limit",
        "quota",
        "insufficient",
        "connection refused",
        "connection reset",
        "dns",
        "timed out",
        "timeout",
        "no such model",
        "model not found",
        "requires", // "model requires 21.1 GiB but only 17.3 GiB are available"
        "out of memory",
        "provider",
        "failed to connect",
    ];
    SIGNATURES
        .iter()
        .find(|sig| summary.contains(*sig))
        .map(|_| {
            format!(
                "the agent made no tool call and the run failed at the provider: {}",
                report.summary.trim()
            )
        })
}

/// Drain a child pipe into `buf` as it arrives, capped so a chatty run cannot
/// exhaust memory. Returns a handle the caller awaits or abandons.
fn spawn_capture<R>(pipe: Option<R>, buf: Arc<Mutex<String>>) -> tokio::task::JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let Some(mut pipe) = pipe else { return };
        let mut chunk = vec![0u8; 8192];
        loop {
            match pipe.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if let Ok(mut guard) = buf.lock() {
                        guard.push_str(&String::from_utf8_lossy(&chunk[..n]));
                        // Keep only the tail: the useful part of a long run is
                        // what it was doing when it stopped.
                        const CAP: usize = 64 * 1024;
                        if guard.len() > CAP {
                            let cut = guard.len() - CAP;
                            let boundary = guard
                                .char_indices()
                                .map(|(i, _)| i)
                                .find(|i| *i >= cut)
                                .unwrap_or(guard.len());
                            let kept = guard[boundary..].to_string();
                            *guard = kept;
                        }
                    }
                }
            }
        }
    })
}

fn tail(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let start = text.len().saturating_sub(max);
    let boundary = text
        .char_indices()
        .map(|(i, _)| i)
        .find(|i| *i >= start)
        .unwrap_or(text.len());
    text[boundary..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_exec_report_contract() {
        // This is the exact serialization `vibecli::ci::CiReport` produces:
        // `CiOutcome` is `rename_all = "lowercase"`, so the outcome is a bare
        // lowercase string, not a tagged object.
        let json = r#"{
            "task": "fix the bug",
            "outcome": "success",
            "steps": [
                {"step":0,"tool":"read_file","input_summary":"read_file(a.rs)",
                 "output":"ok","success":true,"duration_ms":12,"approved_by":"auto"}
            ],
            "summary": "Fixed it.",
            "duration_ms": 4321
        }"#;
        let report: CiReportShape = serde_json::from_str(json).expect("parse");
        assert_eq!(report.outcome, "success");
        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].tool, "read_file");
        assert!(report.steps[0].success);
        assert_eq!(report.summary, "Fixed it.");
    }

    #[test]
    fn tolerates_unknown_fields_added_by_newer_vibecli_builds() {
        let json = r#"{"outcome":"partial","summary":"s","brand_new_field":42}"#;
        let report: CiReportShape = serde_json::from_str(json).expect("parse");
        assert_eq!(report.outcome, "partial");
        assert!(report.steps.is_empty());
    }

    fn report_of(outcome: &str, summary: &str, steps: usize) -> CiReportShape {
        CiReportShape {
            task: "t".into(),
            outcome: outcome.into(),
            steps: (0..steps)
                .map(|_| CiStepShape {
                    tool: "bash".into(),
                    input_summary: String::new(),
                    output: String::new(),
                    success: true,
                    duration_ms: 1,
                })
                .collect(),
            summary: summary.into(),
            duration_ms: 1,
        }
    }

    #[test]
    fn a_provider_outage_is_not_scored_as_a_capability_failure() {
        // Observed for real: the local model could not be loaded, `--exec`
        // reported `failed` with the provider's message, and the task was
        // scored as though the agent had tried and got it wrong.
        let report = report_of(
            "failed",
            "Ollama streaming chat failed (500 Internal Server Error): model \
             requires 21.1 GiB but only 17.3 GiB are available",
            0,
        );
        let reason = provider_failure(&report).expect("should be recognised as infrastructure");
        assert!(reason.contains("no tool call"), "{}", reason);
    }

    #[test]
    fn common_provider_outages_are_all_recognised() {
        for summary in [
            "401 Unauthorized: invalid API key",
            "429 rate limit exceeded",
            "connection refused",
            "request timed out after 60s",
            "no such model: gpt-9",
            "insufficient quota",
        ] {
            assert!(
                provider_failure(&report_of("failed", summary, 0)).is_some(),
                "not recognised: {}",
                summary
            );
        }
    }

    #[test]
    fn a_genuine_agent_failure_stays_a_failure() {
        // The agent worked and came up short. That is a real result and must
        // stay in the denominator.
        assert!(provider_failure(&report_of(
            "failed",
            "Could not make the tests pass after 12 steps.",
            12
        ))
        .is_none());
        // Even with no steps, a failure that does not look like transport is
        // left alone — the heuristic only ever upgrades fail to error.
        assert!(provider_failure(&report_of(
            "failed",
            "The task description was ambiguous, so I stopped.",
            0
        ))
        .is_none());
    }

    #[test]
    fn a_successful_run_is_never_reclassified() {
        // A provider hiccup that the agent recovered from must not erase an
        // otherwise valid success.
        assert!(
            provider_failure(&report_of("success", "429 rate limit hit once, retried", 0))
                .is_none()
        );
        assert!(provider_failure(&report_of("partial", "timeout", 0)).is_none());
    }

    #[tokio::test]
    async fn a_timed_out_run_still_reports_what_the_child_printed() {
        // Three consecutive 900s greenfield timeouts produced no file and no
        // explanation, because `output()` is cancelled by the timeout and
        // takes the child's entire output with it. "The agent achieved
        // nothing" and "the provider stopped answering" then look identical,
        // and they call for opposite responses.
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("chatty");
        std::fs::write(
            &script,
            "#!/bin/sh\necho 'PROVIDER ERROR: 429 rate limit' >&2\nsleep 30\n",
        )
        .expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }

        let h = CliHarness::new(CliConfig {
            binary: script,
            ..CliConfig::default()
        });
        let task = EvalTask {
            id: "t".into(),
            title: "t".into(),
            capability: crate::task::Capability::CodeGeneration,
            difficulty: crate::task::Difficulty::Easy,
            surfaces: vec![],
            prompt: "build something".into(),
            fixture: Default::default(),
            grader: crate::grade::Grader::AlwaysSkip {
                reason: "n/a".into(),
            },
            limits: Default::default(),
            tags: vec![],
            source: Default::default(),
            requires: vec![],
            workspace: crate::task::WorkspaceMode::Temp,
        };

        let err = h
            .run(&task, dir.path(), Duration::from_secs(2))
            .await
            .expect_err("should time out");
        match err {
            HarnessError::Timeout { secs, tail } => {
                assert_eq!(secs, 2);
                assert!(
                    tail.contains("429 rate limit"),
                    "the child's output must survive the kill, got: {:?}",
                    tail
                );
            }
            other => panic!("expected a timeout, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn preflight_names_the_missing_binary() {
        let h = CliHarness::new(CliConfig {
            binary: PathBuf::from("/nonexistent/vibecli-xyzzy"),
            ..CliConfig::default()
        });
        match h.preflight().await {
            Preflight::Unavailable { reason } => {
                assert!(reason.contains("vibecli-xyzzy"), "{}", reason)
            }
            Preflight::Ready => panic!("should not be ready"),
        }
    }

    #[test]
    fn tail_keeps_the_end_of_long_output() {
        let long = format!("{}IMPORTANT", "x".repeat(2000));
        assert!(tail(&long, 100).contains("IMPORTANT"));
        assert!(tail(&long, 100).len() <= 110);
    }

    #[test]
    fn a_relative_binary_path_resolves_to_an_absolute_one() {
        // The child runs with `current_dir` set to the task workspace, so a
        // relative binary would be looked up inside the fixture directory and
        // every task would error with "No such file or directory".
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("target/debug");
        std::fs::create_dir_all(&nested).expect("mkdir");
        let bin = nested.join("vibecli");
        std::fs::write(&bin, "#!/bin/sh\n").expect("write");

        let previous = std::env::current_dir().ok();
        // Scoped: the relative form only means anything relative to a cwd.
        std::env::set_current_dir(dir.path()).expect("chdir");
        let h = CliHarness::new(CliConfig {
            binary: PathBuf::from("target/debug/vibecli"),
            ..CliConfig::default()
        });
        let resolved = h.resolve_binary();
        if let Some(p) = previous {
            let _ = std::env::set_current_dir(p);
        }

        let resolved = resolved.expect("should resolve");
        assert!(resolved.is_absolute(), "got {}", resolved.display());
        assert!(
            resolved.ends_with("target/debug/vibecli"),
            "got {}",
            resolved.display()
        );
    }

    #[test]
    fn describe_records_the_configuration_under_test() {
        let h = CliHarness::new(CliConfig {
            provider: "claude".into(),
            model: Some("claude-opus-5".into()),
            ..CliConfig::default()
        });
        let d = h.describe();
        assert!(d.contains("claude"));
        assert!(d.contains("claude-opus-5"));
    }
}
