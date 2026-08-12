//! The CLI surface: `vibecli --exec`, run as a subprocess in the task workspace.
//!
//! This is the reference harness. It is the only surface that owns the agent
//! loop *and* a filesystem workspace in one process, so it is where a
//! coding-capability number is cheapest and most trustworthy to obtain.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

    /// Resolve the binary to an absolute path if possible, so the error for a
    /// missing binary names a path rather than repeating the bare word the
    /// operator already typed.
    fn resolve_binary(&self) -> Option<PathBuf> {
        let bin = &self.config.binary;
        if bin.components().count() > 1 || bin.is_absolute() {
            return bin.exists().then(|| bin.clone());
        }
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|dir| dir.join(bin))
            .find(|candidate| candidate.is_file())
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
            "--output-format".to_string(),
            "json".to_string(),
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
            .envs(&self.config.env);

        let started = std::time::Instant::now();
        let output = match tokio::time::timeout(timeout, command.output()).await {
            Err(_) => return Err(HarnessError::Timeout(timeout.as_secs())),
            Ok(Err(e)) => return Err(HarnessError::Spawn(e.to_string())),
            Ok(Ok(out)) => out,
        };
        let duration_ms = started.elapsed().as_millis() as u64;
        let exit_code = output.status.code();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

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
