//! Non-interactive CI/exec mode.
//!
//! `vibecli exec "task" --full-auto` runs an agent session without any user
//! prompts and writes a structured JSON or Markdown report.
//!
//! Exit codes:
//! * `0` — task completed successfully
//! * `1` — task completed with some step failures (partial)
//! * `2` — task failed entirely
//! * `3` — `suggest` approval policy was given (CI requires `auto-edit` or `full-auto`)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, ToolExecutorTrait};
use vibe_ai::provider::AIProvider;
use vibe_ai::trace::TraceWriter;

// ── Report types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CiReport {
    pub task: String,
    pub outcome: CiOutcome,
    pub steps: Vec<CiStep>,
    pub summary: String,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiOutcome {
    Success,
    Partial,
    Failed,
    ApprovalRequired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CiStep {
    pub step: usize,
    pub tool: String,
    pub input_summary: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
    pub approved_by: String,
}

impl CiReport {
    /// Suggested process exit code.
    pub fn exit_code(&self) -> i32 {
        match self.outcome {
            CiOutcome::Success => 0,
            CiOutcome::Partial => 1,
            CiOutcome::Failed => 2,
            CiOutcome::ApprovalRequired => 3,
        }
    }

    /// Render as a Markdown document.
    pub fn to_markdown(&self) -> String {
        let outcome_str = match self.outcome {
            CiOutcome::Success => "✅ Success",
            CiOutcome::Partial => "⚠️ Partial",
            CiOutcome::Failed => "❌ Failed",
            CiOutcome::ApprovalRequired => "🔒 Approval Required",
        };
        let mut md = format!(
            "# VibeCLI Agent Report\n\n**Task:** {}\n**Outcome:** {}\n**Duration:** {}ms\n\n",
            self.task, outcome_str, self.duration_ms
        );
        md.push_str("## Steps\n\n");
        for step in &self.steps {
            let icon = if step.success { "✅" } else { "❌" };
            md.push_str(&format!(
                "### {} Step {}: `{}`\n",
                icon,
                step.step + 1,
                step.tool
            ));
            md.push_str(&format!("**Input:** {}\n\n", step.input_summary));
            md.push_str(&format!("**Output:**\n```\n{}\n```\n\n", step.output));
        }
        md.push_str(&format!("## Summary\n\n{}\n", self.summary));
        md
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(outcome: CiOutcome, steps_ok: usize, steps_fail: usize) -> CiReport {
        let mut steps = Vec::new();
        for i in 0..steps_ok {
            steps.push(CiStep {
                step: i,
                tool: "read_file".to_string(),
                input_summary: format!("read_file(src/{}.rs)", i),
                output: "ok".to_string(),
                success: true,
                duration_ms: 10,
                approved_by: "auto".to_string(),
            });
        }
        for i in steps_ok..steps_ok + steps_fail {
            steps.push(CiStep {
                step: i,
                tool: "bash".to_string(),
                input_summary: "bash(cargo test)".to_string(),
                output: "error".to_string(),
                success: false,
                duration_ms: 500,
                approved_by: "ci-auto".to_string(),
            });
        }
        CiReport {
            task: "test task".to_string(),
            outcome,
            steps,
            summary: "done".to_string(),
            duration_ms: 1000,
        }
    }

    #[test]
    fn exit_codes() {
        assert_eq!(make_report(CiOutcome::Success, 1, 0).exit_code(), 0);
        assert_eq!(make_report(CiOutcome::Partial, 1, 1).exit_code(), 1);
        assert_eq!(make_report(CiOutcome::Failed, 0, 1).exit_code(), 2);
        assert_eq!(make_report(CiOutcome::ApprovalRequired, 0, 0).exit_code(), 3);
    }

    #[test]
    fn markdown_contains_task_and_outcome() {
        let report = make_report(CiOutcome::Success, 2, 0);
        let md = report.to_markdown();
        assert!(md.contains("test task"));
        assert!(md.contains("Success"));
        assert!(md.contains("read_file"));
    }

    #[test]
    fn markdown_partial_shows_failure_icon() {
        let report = make_report(CiOutcome::Partial, 1, 1);
        let md = report.to_markdown();
        assert!(md.contains("❌"));
        assert!(md.contains("✅"));
    }

    #[test]
    fn json_round_trip() {
        let report = make_report(CiOutcome::Partial, 1, 1);
        let json = serde_json::to_string(&report).unwrap();
        let back: CiReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.task, report.task);
        assert_eq!(back.steps.len(), report.steps.len());
    }

    #[test]
    fn output_format_from_str() {
        assert_eq!(CiOutputFormat::from_str("json"), CiOutputFormat::Json);
        assert_eq!(CiOutputFormat::from_str("markdown"), CiOutputFormat::Markdown);
        assert_eq!(CiOutputFormat::from_str("md"), CiOutputFormat::Markdown);
        assert_eq!(CiOutputFormat::from_str("verbose"), CiOutputFormat::Verbose);
        assert_eq!(CiOutputFormat::from_str("unknown"), CiOutputFormat::Json);
    }
}

// ── Output format ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CiOutputFormat {
    Json,
    Markdown,
    Verbose, // streaming progress to stderr + final JSON to stdout
}

impl CiOutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Self::Markdown,
            "verbose" | "v" => Self::Verbose,
            _ => Self::Json,
        }
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Run an agent task non-interactively and return a [`CiReport`].
pub async fn run_ci(
    task: &str,
    approval_policy: ApprovalPolicy,
    provider: Arc<dyn AIProvider>,
    executor: Arc<dyn ToolExecutorTrait>,
    trace_writer: Option<TraceWriter>,
    verbose: bool,
) -> Result<CiReport> {
    // Suggest mode would require stdin — reject immediately.
    if approval_policy == ApprovalPolicy::Suggest {
        return Ok(CiReport {
            task: task.to_string(),
            outcome: CiOutcome::ApprovalRequired,
            steps: vec![],
            summary: "CI mode requires --auto-edit or --full-auto approval policy.".to_string(),
            duration_ms: 0,
        });
    }

    let workspace = std::env::current_dir()?;
    let agent = AgentLoop::new(provider, approval_policy, executor.clone());
    let context = AgentContext {
        workspace_root: workspace,
        ..Default::default()
    };

    let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(64);
    let task_str = task.to_string();
    tokio::spawn(async move {
        let _ = agent.run(&task_str, context, event_tx).await;
    });

    let run_start = Instant::now();
    let mut steps: Vec<CiStep> = Vec::new();
    let mut outcome = CiOutcome::Failed;
    let mut summary = String::new();
    let mut step_start = Instant::now();

    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::StreamChunk(text) => {
                if verbose {
                    eprint!("{}", text);
                }
            }
            AgentEvent::ToolCallExecuted(step) => {
                let dur = step_start.elapsed().as_millis() as u64;
                step_start = Instant::now();
                let input_summary = step.tool_call.summary();
                let approved_by = "auto".to_string();

                if let Some(ref tw) = trace_writer {
                    tw.record(
                        step.step_num,
                        step.tool_call.name(),
                        &input_summary,
                        &step.tool_result.output,
                        step.tool_result.success,
                        dur,
                        &approved_by,
                    );
                }

                if verbose {
                    let icon = if step.tool_result.success { "✅" } else { "❌" };
                    eprintln!("\n{} {}", icon, input_summary);
                }

                steps.push(CiStep {
                    step: step.step_num,
                    tool: step.tool_call.name().to_string(),
                    input_summary,
                    output: step.tool_result.output,
                    success: step.tool_result.success,
                    duration_ms: dur,
                    approved_by,
                });
            }
            AgentEvent::ToolCallPending { call, result_tx } => {
                // In CI mode with AutoEdit, bash commands still need approval.
                // We auto-approve them (CI user has explicitly opted in).
                let dur = step_start.elapsed().as_millis() as u64;
                step_start = Instant::now();
                let input_summary = call.summary();
                let result = executor.execute(&call).await;
                let approved_by = "ci-auto".to_string();

                if let Some(ref tw) = trace_writer {
                    tw.record(
                        steps.len(),
                        call.name(),
                        &input_summary,
                        &result.output,
                        result.success,
                        dur,
                        &approved_by,
                    );
                }

                if verbose {
                    let icon = if result.success { "✅" } else { "❌" };
                    eprintln!("\n{} {} [ci-auto]", icon, input_summary);
                }

                steps.push(CiStep {
                    step: steps.len(),
                    tool: call.name().to_string(),
                    input_summary,
                    output: result.output.clone(),
                    success: result.success,
                    duration_ms: dur,
                    approved_by,
                });
                let _ = result_tx.send(Some(result));
            }
            AgentEvent::Complete(s) => {
                summary = s;
                outcome = if steps.is_empty() || steps.iter().all(|s| s.success) {
                    CiOutcome::Success
                } else {
                    CiOutcome::Partial
                };
                break;
            }
            AgentEvent::Error(e) => {
                summary = e;
                outcome = CiOutcome::Failed;
                break;
            }
        }
    }

    if verbose {
        eprintln!();
    }

    Ok(CiReport {
        task: task.to_string(),
        outcome,
        steps,
        summary,
        duration_ms: run_start.elapsed().as_millis() as u64,
    })
}
