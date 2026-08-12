//! Graders: how a run is judged.
//!
//! Everything here is built around one rule — **a grader may never report a
//! pass it did not observe.** The four verdicts are distinct and none of them
//! collapses into another: a missing toolchain is [`Verdict::Skipped`], a
//! crashed grader is [`Verdict::Error`], and only an assertion that actually
//! ran and held is [`Verdict::Pass`]. A harness that cannot tell the
//! difference reports a number that looks like capability and is really
//! infrastructure.
//!
//! The default grader is execution-based: run the project's own tests against
//! the workspace the agent left behind. Rubric judging exists ([`Grader::Judge`])
//! but is opt-in, separately reported, and skipped rather than guessed at when
//! no judge model is wired.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::harness::RunOutcome;

/// The result of a single graded assertion or of a whole task.
///
/// `Fail` means the agent was measured and came up short. `Error` means *we*
/// came up short — the grader could not reach a judgement. `Skipped` means the
/// task never applied here. Reports keep all three apart because averaging
/// them together is how a harness quietly turns a broken machine into a
/// capability regression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail { reason: String },
    Error { reason: String },
    Skipped { reason: String },
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }

    /// Whether this verdict counts toward a pass rate at all. Skipped and
    /// errored tasks are excluded from the denominator and reported on their
    /// own line; folding them in would let a broken environment masquerade as
    /// a score.
    pub fn is_scored(&self) -> bool {
        matches!(self, Verdict::Pass | Verdict::Fail { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail { .. } => "fail",
            Verdict::Error { .. } => "error",
            Verdict::Skipped { .. } => "skipped",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Verdict::Pass => None,
            Verdict::Fail { reason } | Verdict::Error { reason } | Verdict::Skipped { reason } => {
                Some(reason.as_str())
            }
        }
    }
}

/// A graded node — one assertion, or a composite with children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradeResult {
    /// Human-readable name of what was checked.
    pub label: String,
    pub verdict: Verdict,
    /// Partial credit in `0.0..=1.0`, when the grader can express it.
    ///
    /// `None` is not zero. It means the grader reached no numeric judgement —
    /// an errored or skipped node — and consumers must propagate the absence
    /// rather than substitute a plausible default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<GradeResult>,
    /// Captured evidence (command output, diff excerpt) for the failure report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

impl GradeResult {
    pub fn pass(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            verdict: Verdict::Pass,
            score: Some(1.0),
            children: Vec::new(),
            evidence: None,
        }
    }

    pub fn fail(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            verdict: Verdict::Fail {
                reason: reason.into(),
            },
            score: Some(0.0),
            children: Vec::new(),
            evidence: None,
        }
    }

    pub fn error(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            verdict: Verdict::Error {
                reason: reason.into(),
            },
            score: None,
            children: Vec::new(),
            evidence: None,
        }
    }

    pub fn skipped(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            verdict: Verdict::Skipped {
                reason: reason.into(),
            },
            score: None,
            children: Vec::new(),
            evidence: None,
        }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        let text = evidence.into();
        if !text.trim().is_empty() {
            self.evidence = Some(truncate_evidence(&text));
        }
        self
    }

    /// Reduce children into this node under conjunction semantics.
    ///
    /// Precedence is Error > Fail > Skipped > Pass, and it is deliberately not
    /// "worst score wins". An errored child means the composite's truth is
    /// unknown, which outranks a child we know failed; a skipped child means
    /// we never checked something the task said mattered, so the composite
    /// cannot claim a pass either.
    fn reduce_all(label: impl Into<String>, children: Vec<GradeResult>) -> Self {
        if children.is_empty() {
            return GradeResult::error(label, "grader has no assertions to check");
        }
        let verdict = children
            .iter()
            .map(|c| &c.verdict)
            .fold(Verdict::Pass, |acc, v| match (&acc, v) {
                (Verdict::Error { .. }, _) => acc,
                (_, Verdict::Error { reason }) => Verdict::Error {
                    reason: reason.clone(),
                },
                (Verdict::Fail { .. }, _) => acc,
                (_, Verdict::Fail { reason }) => Verdict::Fail {
                    reason: reason.clone(),
                },
                (Verdict::Skipped { .. }, _) => acc,
                (_, Verdict::Skipped { reason }) => Verdict::Skipped {
                    reason: reason.clone(),
                },
                (Verdict::Pass, Verdict::Pass) => Verdict::Pass,
            });

        // Partial credit is the mean over children that produced a number.
        // Children with no score are left out of both sides of the fraction
        // rather than counted as zero.
        let scored: Vec<f64> = children.iter().filter_map(|c| c.score).collect();
        let score = if scored.is_empty() {
            None
        } else {
            Some(scored.iter().sum::<f64>() / scored.len() as f64)
        };

        Self {
            label: label.into(),
            verdict,
            score,
            children,
            evidence: None,
        }
    }

    /// Disjunction: one passing child is enough.
    fn reduce_any(label: impl Into<String>, children: Vec<GradeResult>) -> Self {
        if children.is_empty() {
            return GradeResult::error(label, "grader has no alternatives to check");
        }
        let label = label.into();
        if children.iter().any(|c| c.verdict.is_pass()) {
            let score = Some(1.0);
            return Self {
                label,
                verdict: Verdict::Pass,
                score,
                children,
                evidence: None,
            };
        }
        // Nothing passed. If every alternative was inconclusive we do not know
        // the answer; if at least one genuinely ran and failed, this is a fail.
        let any_conclusive = children
            .iter()
            .any(|c| matches!(c.verdict, Verdict::Fail { .. }));
        let verdict = if any_conclusive {
            Verdict::Fail {
                reason: "no alternative passed".to_string(),
            }
        } else {
            Verdict::Error {
                reason: "no alternative could be evaluated".to_string(),
            }
        };
        let score = if any_conclusive { Some(0.0) } else { None };
        Self {
            label,
            verdict,
            score,
            children,
            evidence: None,
        }
    }
}

fn truncate_evidence(text: &str) -> String {
    const MAX: usize = 4000;
    if text.len() <= MAX {
        return text.to_string();
    }
    // Keep the tail: compiler and test-runner failures put the useful part at
    // the end, and a head-truncated log usually shows only the banner.
    let start = text.len().saturating_sub(MAX);
    let boundary = text
        .char_indices()
        .map(|(i, _)| i)
        .find(|i| *i >= start)
        .unwrap_or(text.len());
    format!("…(truncated)\n{}", &text[boundary..])
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// One command to run, plus what counts as success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandStep {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory relative to the workspace root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Exit code that counts as success. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_exit: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_not_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

impl CommandStep {
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.cmd.clone()
        } else {
            format!("{} {}", self.cmd, self.args.join(" "))
        }
    }
}

/// What actually happened when a command ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRun {
    /// `None` when the process was killed by a signal or timed out — there is
    /// no exit code in that case, and inventing 0 or 1 would be a lie about
    /// what the process did.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_ms: u64,
}

impl CommandRun {
    pub fn combined_output(&self) -> String {
        match (self.stdout.trim().is_empty(), self.stderr.trim().is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.stdout.clone(),
            (true, false) => self.stderr.clone(),
            (false, false) => format!("{}\n{}", self.stdout, self.stderr),
        }
    }
}

/// Run a command inside `workspace`, bounded by its timeout.
pub async fn run_command(
    step: &CommandStep,
    workspace: &Path,
    default_timeout: Duration,
) -> Result<CommandRun, String> {
    let dir = match &step.cwd {
        Some(rel) => workspace.join(rel),
        None => workspace.to_path_buf(),
    };
    let mut command = tokio::process::Command::new(&step.cmd);
    command
        .args(&step.args)
        .current_dir(&dir)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .envs(&step.env);

    let started = std::time::Instant::now();
    let timeout = step
        .timeout_secs
        .map(Duration::from_secs)
        .unwrap_or(default_timeout);

    match tokio::time::timeout(timeout, command.output()).await {
        Err(_elapsed) => Ok(CommandRun {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("timed out after {}s", timeout.as_secs()),
            timed_out: true,
            duration_ms: started.elapsed().as_millis() as u64,
        }),
        // A command that cannot even be spawned is an environment fault, not
        // a failed assertion — the caller turns this into `Error`, not `Fail`.
        Ok(Err(e)) => Err(format!("could not spawn `{}`: {}", step.display(), e)),
        Ok(Ok(out)) => Ok(CommandRun {
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            timed_out: false,
            duration_ms: started.elapsed().as_millis() as u64,
        }),
    }
}

fn judge_command_run(step: &CommandStep, run: &CommandRun) -> GradeResult {
    let label = format!("$ {}", step.display());
    if run.timed_out {
        return GradeResult::fail(label, format!("command timed out: {}", step.display()))
            .with_evidence(run.combined_output());
    }
    let expected = step.expect_exit.unwrap_or(0);
    match run.exit_code {
        None => {
            return GradeResult::error(
                label,
                "process terminated by a signal with no exit code".to_string(),
            )
            .with_evidence(run.combined_output())
        }
        Some(code) if code != expected => {
            return GradeResult::fail(label, format!("exit code {} (expected {})", code, expected))
                .with_evidence(run.combined_output())
        }
        Some(_) => {}
    }
    let output = run.combined_output();
    if let Some(needle) = &step.stdout_contains {
        if !output.contains(needle.as_str()) {
            return GradeResult::fail(label, format!("output does not contain {:?}", needle))
                .with_evidence(output);
        }
    }
    if let Some(needle) = &step.stdout_not_contains {
        if output.contains(needle.as_str()) {
            return GradeResult::fail(label, format!("output contains forbidden {:?}", needle))
                .with_evidence(output);
        }
    }
    GradeResult::pass(label)
}

// ── File assertions ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "assert", rename_all = "snake_case")]
pub enum FileAssertion {
    Exists {
        path: String,
    },
    NotExists {
        path: String,
    },
    Contains {
        path: String,
        text: String,
    },
    NotContains {
        path: String,
        text: String,
    },
    Matches {
        path: String,
        regex: String,
    },
    /// The file does not match. A file that does not exist passes: "the old
    /// definition is no longer in utils.py" is satisfied by deleting utils.py,
    /// and treating the read failure as an error would fail a legitimate
    /// refactor.
    NotMatches {
        path: String,
        regex: String,
    },
    /// The file is byte-identical to how the fixture left it.
    Unchanged {
        path: String,
    },
    /// The file parses as JSON and the value at `pointer` (RFC 6901) equals
    /// `value`.
    JsonEquals {
        path: String,
        pointer: String,
        value: serde_json::Value,
    },
}

fn check_file_assertion(a: &FileAssertion, ws: &Path, baseline: Option<&Path>) -> GradeResult {
    let read = |rel: &str| -> Result<String, String> {
        std::fs::read_to_string(ws.join(rel)).map_err(|e| format!("cannot read {}: {}", rel, e))
    };
    match a {
        FileAssertion::Exists { path } => {
            let label = format!("exists: {}", path);
            if ws.join(path).exists() {
                GradeResult::pass(label)
            } else {
                GradeResult::fail(label, format!("{} was not created", path))
            }
        }
        FileAssertion::NotExists { path } => {
            let label = format!("absent: {}", path);
            if ws.join(path).exists() {
                GradeResult::fail(label, format!("{} should not exist", path))
            } else {
                GradeResult::pass(label)
            }
        }
        FileAssertion::Contains { path, text } => {
            let label = format!("contains: {}", path);
            match read(path) {
                Err(e) => GradeResult::fail(label, e),
                Ok(body) if body.contains(text.as_str()) => GradeResult::pass(label),
                Ok(_) => GradeResult::fail(label, format!("{} does not contain {:?}", path, text)),
            }
        }
        FileAssertion::NotContains { path, text } => {
            let label = format!("omits: {}", path);
            match read(path) {
                // A file that does not exist cannot contain the forbidden
                // text. That is a genuine pass, not a read error.
                Err(_) if !ws.join(path).exists() => GradeResult::pass(label),
                Err(e) => GradeResult::error(label, e),
                Ok(body) if body.contains(text.as_str()) => {
                    GradeResult::fail(label, format!("{} still contains {:?}", path, text))
                }
                Ok(_) => GradeResult::pass(label),
            }
        }
        FileAssertion::Matches { path, regex } => {
            let label = format!("matches: {}", path);
            let re = match regex::Regex::new(regex) {
                Ok(r) => r,
                // A malformed regex is our bug, not the agent's.
                Err(e) => {
                    return GradeResult::error(label, format!("bad regex {:?}: {}", regex, e))
                }
            };
            match read(path) {
                Err(e) => GradeResult::fail(label, e),
                Ok(body) if re.is_match(&body) => GradeResult::pass(label),
                Ok(_) => GradeResult::fail(label, format!("{} does not match /{}/", path, regex)),
            }
        }
        FileAssertion::NotMatches { path, regex } => {
            let label = format!("does not match: {}", path);
            let re = match regex::Regex::new(regex) {
                Ok(r) => r,
                Err(e) => {
                    return GradeResult::error(label, format!("bad regex {:?}: {}", regex, e))
                }
            };
            match read(path) {
                Err(_) if !ws.join(path).exists() => GradeResult::pass(label),
                Err(e) => GradeResult::error(label, e),
                Ok(body) if re.is_match(&body) => {
                    GradeResult::fail(label, format!("{} still matches /{}/", path, regex))
                }
                Ok(_) => GradeResult::pass(label),
            }
        }
        FileAssertion::Unchanged { path } => {
            let label = format!("unchanged: {}", path);
            let Some(base) = baseline else {
                return GradeResult::error(
                    label,
                    "no fixture baseline was captured, so `unchanged` cannot be decided",
                );
            };
            match (
                std::fs::read(base.join(path)).ok(),
                std::fs::read(ws.join(path)).ok(),
            ) {
                (Some(a), Some(b)) if a == b => GradeResult::pass(label),
                (Some(_), Some(_)) => GradeResult::fail(label, format!("{} was modified", path)),
                (Some(_), None) => GradeResult::fail(label, format!("{} was deleted", path)),
                (None, _) => {
                    GradeResult::error(label, format!("{} is not in the fixture baseline", path))
                }
            }
        }
        FileAssertion::JsonEquals {
            path,
            pointer,
            value,
        } => {
            let label = format!("json {}{}", path, pointer);
            match read(path) {
                Err(e) => GradeResult::fail(label, e),
                Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
                    Err(e) => {
                        GradeResult::fail(label, format!("{} is not valid JSON: {}", path, e))
                    }
                    Ok(json) => match json.pointer(pointer) {
                        None => GradeResult::fail(label, format!("no value at {}", pointer)),
                        Some(found) if found == value => GradeResult::pass(label),
                        Some(found) => {
                            GradeResult::fail(label, format!("expected {}, found {}", value, found))
                        }
                    },
                },
            }
        }
    }
}

// ── Transcript assertions ────────────────────────────────────────────────────

/// Assertions about *how* the agent worked, not just what it left behind.
///
/// Process metrics are what separate "solved it" from "solved it the way a
/// competent engineer would". An agent that edits a file it was told not to
/// touch, or that never runs the test suite before declaring victory, is worth
/// distinguishing from one that does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "assert", rename_all = "snake_case")]
pub enum TranscriptAssertion {
    /// The agent reported this outcome (`success`, `failure`, …).
    OutcomeIs {
        outcome: String,
    },
    UsedTool {
        tool: String,
    },
    DidNotUseTool {
        tool: String,
    },
    /// The agent finished within `n` steps.
    MaxSteps {
        n: usize,
    },
    /// No tool call reported an error.
    NoToolErrors,
    FinalContains {
        text: String,
    },
    FinalMatches {
        regex: String,
    },
    FinalOmits {
        text: String,
    },
}

fn check_transcript_assertion(a: &TranscriptAssertion, run: &RunOutcome) -> GradeResult {
    match a {
        TranscriptAssertion::OutcomeIs { outcome } => {
            let label = format!("outcome == {}", outcome);
            match &run.outcome {
                None => GradeResult::error(
                    label,
                    "the surface reported no outcome field, so it cannot be compared",
                ),
                Some(actual) if actual.eq_ignore_ascii_case(outcome) => GradeResult::pass(label),
                Some(actual) => GradeResult::fail(label, format!("outcome was {:?}", actual)),
            }
        }
        TranscriptAssertion::UsedTool { tool } => {
            let label = format!("used tool: {}", tool);
            if run.steps.is_empty() {
                return GradeResult::error(
                    label,
                    "no step transcript was captured for this surface",
                );
            }
            if run.steps.iter().any(|s| s.tool.eq_ignore_ascii_case(tool)) {
                GradeResult::pass(label)
            } else {
                let used: Vec<&str> = run.steps.iter().map(|s| s.tool.as_str()).collect();
                GradeResult::fail(label, format!("tools used: {}", used.join(", ")))
            }
        }
        TranscriptAssertion::DidNotUseTool { tool } => {
            let label = format!("avoided tool: {}", tool);
            if run.steps.is_empty() {
                return GradeResult::error(
                    label,
                    "no step transcript was captured for this surface",
                );
            }
            if run.steps.iter().any(|s| s.tool.eq_ignore_ascii_case(tool)) {
                GradeResult::fail(label, format!("{} was used", tool))
            } else {
                GradeResult::pass(label)
            }
        }
        TranscriptAssertion::MaxSteps { n } => {
            let label = format!("steps <= {}", n);
            if run.steps.is_empty() && run.outcome.is_none() {
                return GradeResult::error(label, "no step transcript was captured");
            }
            if run.steps.len() <= *n {
                GradeResult::pass(label)
            } else {
                GradeResult::fail(label, format!("took {} steps", run.steps.len()))
            }
        }
        TranscriptAssertion::NoToolErrors => {
            let label = "no tool errors".to_string();
            if run.steps.is_empty() {
                return GradeResult::error(label, "no step transcript was captured");
            }
            let failed: Vec<&str> = run
                .steps
                .iter()
                .filter(|s| !s.success)
                .map(|s| s.tool.as_str())
                .collect();
            if failed.is_empty() {
                GradeResult::pass(label)
            } else {
                GradeResult::fail(label, format!("failing tools: {}", failed.join(", ")))
            }
        }
        TranscriptAssertion::FinalContains { text } => {
            let label = format!("final answer contains {:?}", text);
            if run.final_text.to_lowercase().contains(&text.to_lowercase()) {
                GradeResult::pass(label)
            } else {
                GradeResult::fail(label, "not present in the final answer")
                    .with_evidence(run.final_text.clone())
            }
        }
        TranscriptAssertion::FinalOmits { text } => {
            let label = format!("final answer omits {:?}", text);
            if run.final_text.to_lowercase().contains(&text.to_lowercase()) {
                GradeResult::fail(label, "present in the final answer")
                    .with_evidence(run.final_text.clone())
            } else {
                GradeResult::pass(label)
            }
        }
        TranscriptAssertion::FinalMatches { regex } => {
            let label = format!("final answer matches /{}/", regex);
            match regex::Regex::new(regex) {
                Err(e) => GradeResult::error(label, format!("bad regex: {}", e)),
                Ok(re) if re.is_match(&run.final_text) => GradeResult::pass(label),
                Ok(_) => GradeResult::fail(label, "no match").with_evidence(run.final_text.clone()),
            }
        }
    }
}

// ── HTTP probes ──────────────────────────────────────────────────────────────

/// One request against a live surface, plus what the response must look like.
///
/// The two invariants worth most here are complementary: a route that should
/// be protected and answers without a token is a hole, and a route that should
/// be public and demands one is unreachable from the clients that cannot send
/// one. Both are transport bugs no capability score would ever reveal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpProbe {
    /// `GET` when omitted.
    #[serde(default)]
    pub method: Option<String>,
    /// `{daemon}` expands to the daemon base URL the run was configured with.
    pub url: String,
    /// Send the daemon bearer token. When false the request is deliberately
    /// anonymous — that is how "this route is public" is actually tested.
    #[serde(default)]
    pub authenticated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Any one of these status codes counts as a pass.
    #[serde(default)]
    pub expect_status: Vec<u16>,
    /// A status that must *not* come back. Used for "with a token this must
    /// not be a 401", which is a weaker and more durable claim than pinning
    /// the exact success code of a route whose body may vary.
    #[serde(default)]
    pub reject_status: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_contains: Option<String>,
    /// RFC 6901 pointer into a JSON response body that must equal `value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_pointer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

async fn run_http_probe(
    probe: &HttpProbe,
    daemon_base: Option<&str>,
    token: Option<&str>,
) -> GradeResult {
    let url = match daemon_base {
        Some(base) => probe.url.replace("{daemon}", base.trim_end_matches('/')),
        None if probe.url.contains("{daemon}") => {
            return GradeResult::skipped(
                format!("{} {}", probe.method.as_deref().unwrap_or("GET"), probe.url),
                "no daemon base URL configured for this run",
            )
        }
        None => probe.url.clone(),
    };
    let method_name = probe.method.as_deref().unwrap_or("GET").to_uppercase();
    let label = format!("{} {}", method_name, url);

    if probe.authenticated && token.is_none() {
        // Without a token this probe would measure the anonymous path while
        // claiming to measure the authenticated one, and a 401 would be
        // reported as a conformance failure of the route.
        return GradeResult::skipped(
            label,
            "probe needs the daemon bearer token but ~/.vibecli/daemon.token is missing",
        );
    }

    let method = match reqwest::Method::from_bytes(method_name.as_bytes()) {
        Ok(m) => m,
        Err(e) => return GradeResult::error(label, format!("bad HTTP method: {}", e)),
    };
    let client = reqwest::Client::new();
    let mut req = client
        .request(method, &url)
        .timeout(Duration::from_secs(probe.timeout_secs.unwrap_or(20)));
    if probe.authenticated {
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
    }
    if let Some(body) = &probe.body {
        req = req.json(body);
    }

    let resp = match req.send().await {
        Ok(r) => r,
        // A refused connection is an unavailable surface, not a failed
        // contract: nothing was tested.
        Err(e) => return GradeResult::skipped(label, format!("surface not reachable: {}", e)),
    };
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();

    if !probe.expect_status.is_empty() && !probe.expect_status.contains(&status) {
        return GradeResult::fail(
            label,
            format!(
                "status {} (expected one of {:?})",
                status, probe.expect_status
            ),
        )
        .with_evidence(text);
    }
    if probe.reject_status.contains(&status) {
        return GradeResult::fail(label, format!("status {} is forbidden here", status))
            .with_evidence(text);
    }
    if let Some(needle) = &probe.body_contains {
        if !text.contains(needle.as_str()) {
            return GradeResult::fail(label, format!("body does not contain {:?}", needle))
                .with_evidence(text);
        }
    }
    if let Some(pointer) = &probe.json_pointer {
        let Some(expected) = &probe.json_value else {
            return GradeResult::error(label, "json_pointer given without json_value");
        };
        match serde_json::from_str::<serde_json::Value>(&text) {
            Err(e) => {
                return GradeResult::fail(label, format!("response is not JSON: {}", e))
                    .with_evidence(text)
            }
            Ok(json) => match json.pointer(pointer) {
                None => {
                    return GradeResult::fail(label, format!("no value at {}", pointer))
                        .with_evidence(text)
                }
                Some(found) if found != expected => {
                    return GradeResult::fail(
                        label,
                        format!("{} is {}, expected {}", pointer, found, expected),
                    )
                    .with_evidence(text)
                }
                Some(_) => {}
            },
        }
    }
    GradeResult::pass(label)
}

// ── Judge model ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeScore {
    /// `0.0..=1.0`.
    pub score: f64,
    pub rationale: String,
}

/// An LLM used to score open-ended answers against a rubric.
///
/// Injected rather than constructed here: this crate must not decide which
/// provider judges, and a run with no judge configured must report
/// [`Verdict::Skipped`] rather than fall back to a default provider.
#[async_trait::async_trait]
pub trait JudgeModel: Send + Sync {
    async fn score(&self, rubric: &str, prompt: &str, answer: &str) -> Result<JudgeScore, String>;
    /// Identifier recorded in the report, so a rubric score is always
    /// attributable to the model that produced it.
    fn describe(&self) -> String;
}

// ── The grader tree ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Grader {
    /// Run commands in the workspace; every step must meet its expectation.
    /// This is the default and the only kind whose pass is unambiguous.
    Command { steps: Vec<CommandStep> },
    /// Assert over the files the agent left behind.
    Files { assertions: Vec<FileAssertion> },
    /// Assert over how the agent worked.
    Transcript {
        assertions: Vec<TranscriptAssertion>,
    },
    /// SWE-bench shape: apply the held-out test patch, then require the
    /// previously-failing tests to pass *and* the previously-passing ones to
    /// keep passing. The second half is what stops "delete the test" from
    /// scoring.
    PatchAndTest {
        /// Unified diff applied after the agent finishes, before testing.
        test_patch: String,
        #[serde(default)]
        fail_to_pass: Vec<String>,
        #[serde(default)]
        pass_to_pass: Vec<String>,
        /// How to run one test; `{test}` in an arg is replaced by its name.
        runner: CommandStep,
    },
    /// Probe a live surface over HTTP. Every probe must hold.
    Http { probes: Vec<HttpProbe> },
    /// Score an open-ended answer against a rubric using a judge model.
    Judge {
        rubric: String,
        /// Minimum score in `0.0..=1.0` to count as a pass.
        #[serde(default = "default_threshold")]
        threshold: f64,
    },
    /// Every child must pass.
    All { of: Vec<Grader> },
    /// At least one child must pass.
    Any { of: Vec<Grader> },
    /// Explicitly not gradeable yet. Reports as skipped — never as a pass.
    /// Used so a task can be authored and reviewed before its grader lands,
    /// without inflating the score in the meantime.
    AlwaysSkip { reason: String },
}

fn default_threshold() -> f64 {
    0.7
}

/// Everything a grader is allowed to look at.
pub struct GradeContext<'a> {
    pub workspace: &'a Path,
    /// Pristine copy of the fixture, for `unchanged` assertions.
    pub baseline: Option<PathBuf>,
    pub run: &'a RunOutcome,
    /// The task's prompt, so a judge can see what was asked.
    pub prompt: &'a str,
    pub judge: Option<&'a dyn JudgeModel>,
    pub default_timeout: Duration,
    /// Base URL that `{daemon}` expands to in HTTP probes.
    pub daemon_base_url: Option<String>,
    /// Daemon bearer token, read fresh by the runner for each run because the
    /// daemon rotates it on every start.
    pub daemon_token: Option<String>,
}

impl Grader {
    /// Whether grading this tree needs a judge model. The runner uses it to
    /// warn once up front rather than let every rubric task skip silently.
    pub fn needs_judge(&self) -> bool {
        match self {
            Grader::Judge { .. } => true,
            Grader::All { of } | Grader::Any { of } => of.iter().any(Grader::needs_judge),
            Grader::Command { .. }
            | Grader::Files { .. }
            | Grader::Transcript { .. }
            | Grader::PatchAndTest { .. }
            | Grader::Http { .. }
            | Grader::AlwaysSkip { .. } => false,
        }
    }

    pub async fn grade(&self, ctx: &GradeContext<'_>) -> GradeResult {
        // Recursion through an async fn needs an explicit box.
        Box::pin(self.grade_inner(ctx)).await
    }

    async fn grade_inner(&self, ctx: &GradeContext<'_>) -> GradeResult {
        match self {
            Grader::AlwaysSkip { reason } => GradeResult::skipped("ungraded", reason.clone()),

            Grader::Command { steps } => {
                let mut children = Vec::with_capacity(steps.len());
                for step in steps {
                    let result = match run_command(step, ctx.workspace, ctx.default_timeout).await {
                        Ok(run) => judge_command_run(step, &run),
                        // A command we could not even start is an environment
                        // problem: the agent is not responsible for it.
                        Err(e) => GradeResult::error(format!("$ {}", step.display()), e),
                    };
                    let stop = !result.verdict.is_pass();
                    children.push(result);
                    // Later steps usually assume earlier ones succeeded; running
                    // them anyway produces cascade noise in the failure report.
                    if stop {
                        break;
                    }
                }
                GradeResult::reduce_all("command", children)
            }

            Grader::Files { assertions } => {
                let children = assertions
                    .iter()
                    .map(|a| check_file_assertion(a, ctx.workspace, ctx.baseline.as_deref()))
                    .collect();
                GradeResult::reduce_all("files", children)
            }

            Grader::Transcript { assertions } => {
                let children = assertions
                    .iter()
                    .map(|a| check_transcript_assertion(a, ctx.run))
                    .collect();
                GradeResult::reduce_all("transcript", children)
            }

            Grader::PatchAndTest {
                test_patch,
                fail_to_pass,
                pass_to_pass,
                runner,
            } => {
                self.grade_patch_and_test(ctx, test_patch, fail_to_pass, pass_to_pass, runner)
                    .await
            }

            Grader::Http { probes } => {
                let mut children = Vec::with_capacity(probes.len());
                for probe in probes {
                    children.push(
                        run_http_probe(
                            probe,
                            ctx.daemon_base_url.as_deref(),
                            ctx.daemon_token.as_deref(),
                        )
                        .await,
                    );
                }
                GradeResult::reduce_all("http", children)
            }

            Grader::Judge { rubric, threshold } => {
                let Some(judge) = ctx.judge else {
                    return GradeResult::skipped(
                        "judge",
                        "no judge model configured — run with --judge-provider to score rubric tasks",
                    );
                };
                match judge.score(rubric, ctx.prompt, &ctx.run.final_text).await {
                    Err(e) => GradeResult::error("judge", format!("judge model failed: {}", e)),
                    Ok(JudgeScore { score, rationale }) => {
                        let label = format!("judge ({})", judge.describe());
                        let mut result = if score >= *threshold {
                            GradeResult::pass(label)
                        } else {
                            GradeResult::fail(
                                label,
                                format!("scored {:.2}, needed {:.2}", score, threshold),
                            )
                        };
                        // Keep the judge's own number rather than the 1.0/0.0
                        // the pass/fail constructors set: the rubric score is
                        // the more informative signal.
                        result.score = Some(score.clamp(0.0, 1.0));
                        result.with_evidence(rationale)
                    }
                }
            }

            Grader::All { of } => {
                let mut children = Vec::with_capacity(of.len());
                for g in of {
                    children.push(Box::pin(g.grade_inner(ctx)).await);
                }
                GradeResult::reduce_all("all", children)
            }

            Grader::Any { of } => {
                let mut children = Vec::with_capacity(of.len());
                for g in of {
                    let r = Box::pin(g.grade_inner(ctx)).await;
                    let done = r.verdict.is_pass();
                    children.push(r);
                    if done {
                        break;
                    }
                }
                GradeResult::reduce_any("any", children)
            }
        }
    }

    async fn grade_patch_and_test(
        &self,
        ctx: &GradeContext<'_>,
        test_patch: &str,
        fail_to_pass: &[String],
        pass_to_pass: &[String],
        runner: &CommandStep,
    ) -> GradeResult {
        // The test patch is held out from the agent on purpose: it is the
        // specification it was not allowed to read. Applying it here is what
        // makes the score mean "the fix is correct" rather than "the agent
        // wrote something that satisfies its own tests".
        if !test_patch.trim().is_empty() {
            let patch_path = ctx.workspace.join(".vibe-eval-test.patch");
            if let Err(e) = std::fs::write(&patch_path, test_patch) {
                return GradeResult::error("patch", format!("cannot stage test patch: {}", e));
            }
            let apply = CommandStep {
                cmd: "git".to_string(),
                args: vec![
                    "apply".to_string(),
                    "--whitespace=nowarn".to_string(),
                    ".vibe-eval-test.patch".to_string(),
                ],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: Some(0),
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: Some(120),
            };
            match run_command(&apply, ctx.workspace, ctx.default_timeout).await {
                Err(e) => return GradeResult::error("patch", e),
                Ok(run) if run.exit_code != Some(0) => {
                    // The agent may have edited the very test file the patch
                    // touches. That is not a pass and not a crash — it is a
                    // fail with a specific, reportable cause.
                    return GradeResult::fail(
                        "patch",
                        "held-out test patch does not apply — the agent likely modified the test files",
                    )
                    .with_evidence(run.combined_output());
                }
                Ok(_) => {}
            }
        }

        if fail_to_pass.is_empty() && pass_to_pass.is_empty() {
            return GradeResult::error(
                "patch_and_test",
                "no fail_to_pass or pass_to_pass tests declared, so nothing would be verified",
            );
        }

        let run_one = |name: &str| {
            let step = CommandStep {
                cmd: runner.cmd.clone(),
                args: runner
                    .args
                    .iter()
                    .map(|a| a.replace("{test}", name))
                    .collect(),
                cwd: runner.cwd.clone(),
                env: runner.env.clone(),
                expect_exit: runner.expect_exit,
                stdout_contains: runner.stdout_contains.clone(),
                stdout_not_contains: runner.stdout_not_contains.clone(),
                timeout_secs: runner.timeout_secs,
            };
            step
        };

        let mut children = Vec::new();
        for name in fail_to_pass {
            let step = run_one(name);
            let r = match run_command(&step, ctx.workspace, ctx.default_timeout).await {
                Ok(run) => {
                    let base = judge_command_run(&step, &run);
                    GradeResult {
                        label: format!("FAIL_TO_PASS {}", name),
                        ..base
                    }
                }
                Err(e) => GradeResult::error(format!("FAIL_TO_PASS {}", name), e),
            };
            children.push(r);
        }
        for name in pass_to_pass {
            let step = run_one(name);
            let r = match run_command(&step, ctx.workspace, ctx.default_timeout).await {
                Ok(run) => {
                    let base = judge_command_run(&step, &run);
                    // A regression in a previously-passing test is worth
                    // naming differently from an unfixed bug: it means the
                    // agent broke something it was not asked to touch.
                    match base.verdict {
                        Verdict::Fail { .. } => GradeResult::fail(
                            format!("PASS_TO_PASS {}", name),
                            "regressed a test that passed before the change",
                        )
                        .with_evidence(base.evidence.unwrap_or_default()),
                        _ => GradeResult {
                            label: format!("PASS_TO_PASS {}", name),
                            ..base
                        },
                    }
                }
                Err(e) => GradeResult::error(format!("PASS_TO_PASS {}", name), e),
            };
            children.push(r);
        }
        GradeResult::reduce_all("patch_and_test", children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{RunOutcome, StepRecord};

    fn empty_run() -> RunOutcome {
        RunOutcome::default()
    }

    fn ctx<'a>(ws: &'a Path, run: &'a RunOutcome) -> GradeContext<'a> {
        GradeContext {
            workspace: ws,
            baseline: None,
            run,
            prompt: "prompt",
            judge: None,
            default_timeout: Duration::from_secs(30),
            daemon_base_url: None,
            daemon_token: None,
        }
    }

    #[test]
    fn skipped_and_errored_verdicts_are_not_scored() {
        assert!(Verdict::Pass.is_scored());
        assert!(Verdict::Fail { reason: "x".into() }.is_scored());
        assert!(!Verdict::Error { reason: "x".into() }.is_scored());
        assert!(!Verdict::Skipped { reason: "x".into() }.is_scored());
    }

    #[test]
    fn error_outranks_fail_which_outranks_skip_in_conjunction() {
        let mk = |v: Verdict| GradeResult {
            label: "c".into(),
            verdict: v,
            score: None,
            children: vec![],
            evidence: None,
        };
        let r = GradeResult::reduce_all(
            "all",
            vec![
                mk(Verdict::Pass),
                mk(Verdict::Fail { reason: "f".into() }),
                mk(Verdict::Error { reason: "e".into() }),
            ],
        );
        assert!(matches!(r.verdict, Verdict::Error { .. }));

        let r = GradeResult::reduce_all(
            "all",
            vec![mk(Verdict::Pass), mk(Verdict::Fail { reason: "f".into() })],
        );
        assert!(matches!(r.verdict, Verdict::Fail { .. }));

        // A skipped child means something the task asked for was never
        // checked, so the composite must not claim a pass.
        let r = GradeResult::reduce_all(
            "all",
            vec![
                mk(Verdict::Pass),
                mk(Verdict::Skipped { reason: "s".into() }),
            ],
        );
        assert!(matches!(r.verdict, Verdict::Skipped { .. }));
    }

    #[test]
    fn empty_grader_is_an_error_not_a_pass() {
        // A grader with no assertions vacuously "passes" under naive
        // all-of semantics. That is the single most dangerous bug a harness
        // can have, so it is pinned here.
        let r = GradeResult::reduce_all("all", vec![]);
        assert!(matches!(r.verdict, Verdict::Error { .. }));
        let r = GradeResult::reduce_any("any", vec![]);
        assert!(matches!(r.verdict, Verdict::Error { .. }));
    }

    #[test]
    fn unscored_children_do_not_count_as_zero() {
        let r = GradeResult::reduce_all(
            "all",
            vec![
                GradeResult::pass("a"),
                GradeResult::skipped("b", "no toolchain"),
            ],
        );
        // One pass, one unscored → mean over the scored child only.
        assert_eq!(r.score, Some(1.0));
    }

    #[test]
    fn any_of_all_inconclusive_is_error_not_fail() {
        let r = GradeResult::reduce_any(
            "any",
            vec![
                GradeResult::error("a", "boom"),
                GradeResult::skipped("b", "n/a"),
            ],
        );
        assert!(matches!(r.verdict, Verdict::Error { .. }));
        assert_eq!(r.score, None);
    }

    #[tokio::test]
    async fn command_grader_passes_on_zero_exit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Command {
            steps: vec![CommandStep {
                cmd: "sh".into(),
                args: vec!["-c".into(), "echo hello".into()],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: None,
                stdout_contains: Some("hello".into()),
                stdout_not_contains: None,
                timeout_secs: Some(30),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    }

    #[tokio::test]
    async fn command_grader_fails_on_nonzero_exit_and_keeps_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Command {
            steps: vec![CommandStep {
                cmd: "sh".into(),
                args: vec!["-c".into(), "echo boom >&2; exit 3".into()],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: None,
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: Some(30),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Fail { .. }));
        let evidence = r.children.first().and_then(|c| c.evidence.clone());
        assert!(evidence.unwrap_or_default().contains("boom"));
    }

    #[tokio::test]
    async fn missing_binary_is_an_error_not_a_failure() {
        // The agent cannot be blamed for a grader command that does not exist
        // on this machine.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Command {
            steps: vec![CommandStep {
                cmd: "definitely-not-a-real-binary-xyzzy".into(),
                args: vec![],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: None,
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: Some(10),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(
            matches!(r.verdict, Verdict::Error { .. }),
            "{:?}",
            r.verdict
        );
    }

    #[tokio::test]
    async fn command_timeout_has_no_exit_code_and_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Command {
            steps: vec![CommandStep {
                cmd: "sh".into(),
                args: vec!["-c".into(), "sleep 5".into()],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: None,
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: Some(1),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Fail { .. }));
    }

    #[tokio::test]
    async fn judge_without_a_model_skips_rather_than_guesses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Judge {
            rubric: "is it good".into(),
            threshold: 0.7,
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Skipped { .. }));
        assert_eq!(r.score, None);
    }

    #[tokio::test]
    async fn file_assertions_check_the_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("out.txt"), "hello world").expect("write");
        let run = empty_run();
        let g = Grader::Files {
            assertions: vec![
                FileAssertion::Exists {
                    path: "out.txt".into(),
                },
                FileAssertion::Contains {
                    path: "out.txt".into(),
                    text: "hello".into(),
                },
                FileAssertion::NotExists {
                    path: "nope.txt".into(),
                },
            ],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    }

    #[tokio::test]
    async fn unchanged_without_a_baseline_errors_rather_than_passing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "x").expect("write");
        let run = empty_run();
        let g = Grader::Files {
            assertions: vec![FileAssertion::Unchanged {
                path: "a.txt".into(),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Error { .. }));
    }

    #[tokio::test]
    async fn transcript_assertions_error_when_no_transcript_was_captured() {
        // A surface that cannot report its steps must not silently satisfy
        // "did not use tool X" — it never told us either way.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Transcript {
            assertions: vec![TranscriptAssertion::DidNotUseTool {
                tool: "bash".into(),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Error { .. }));
    }

    #[tokio::test]
    async fn transcript_tool_use_is_checked_against_recorded_steps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = RunOutcome {
            steps: vec![
                StepRecord {
                    tool: "read_file".into(),
                    success: true,
                    ..StepRecord::default()
                },
                StepRecord {
                    tool: "bash".into(),
                    success: true,
                    ..StepRecord::default()
                },
            ],
            ..RunOutcome::default()
        };
        let g = Grader::Transcript {
            assertions: vec![
                TranscriptAssertion::UsedTool {
                    tool: "bash".into(),
                },
                TranscriptAssertion::DidNotUseTool {
                    tool: "write_file".into(),
                },
                TranscriptAssertion::NoToolErrors,
                TranscriptAssertion::MaxSteps { n: 5 },
            ],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    }

    #[tokio::test]
    async fn patch_and_test_with_no_declared_tests_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::PatchAndTest {
            test_patch: String::new(),
            fail_to_pass: vec![],
            pass_to_pass: vec![],
            runner: CommandStep {
                cmd: "true".into(),
                args: vec![],
                cwd: None,
                env: BTreeMap::new(),
                expect_exit: None,
                stdout_contains: None,
                stdout_not_contains: None,
                timeout_secs: None,
            },
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Error { .. }));
    }

    #[test]
    fn needs_judge_walks_the_whole_tree() {
        let g = Grader::All {
            of: vec![
                Grader::Files { assertions: vec![] },
                Grader::Any {
                    of: vec![Grader::Judge {
                        rubric: "r".into(),
                        threshold: 0.5,
                    }],
                },
            ],
        };
        assert!(g.needs_judge());
        assert!(!Grader::Files { assertions: vec![] }.needs_judge());
    }

    #[tokio::test]
    async fn an_unreachable_http_surface_skips_rather_than_fails() {
        // Nothing was tested, so nothing may be reported as a broken contract.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Http {
            probes: vec![HttpProbe {
                method: None,
                url: "http://127.0.0.1:1/health".into(),
                authenticated: false,
                body: None,
                expect_status: vec![200],
                reject_status: vec![],
                body_contains: None,
                json_pointer: None,
                json_value: None,
                timeout_secs: Some(3),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(
            matches!(r.verdict, Verdict::Skipped { .. }),
            "{:?}",
            r.verdict
        );
    }

    #[tokio::test]
    async fn an_authenticated_probe_without_a_token_skips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Http {
            probes: vec![HttpProbe {
                method: None,
                url: "http://127.0.0.1:1/sessions".into(),
                authenticated: true,
                body: None,
                expect_status: vec![200],
                reject_status: vec![],
                body_contains: None,
                json_pointer: None,
                json_value: None,
                timeout_secs: Some(3),
            }],
        };
        // ctx() supplies no token, so this must not be scored as a 401.
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(
            matches!(r.verdict, Verdict::Skipped { .. }),
            "{:?}",
            r.verdict
        );
    }

    #[tokio::test]
    async fn a_daemon_placeholder_without_a_base_url_skips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = empty_run();
        let g = Grader::Http {
            probes: vec![HttpProbe {
                method: None,
                url: "{daemon}/health".into(),
                authenticated: false,
                body: None,
                expect_status: vec![200],
                reject_status: vec![],
                body_contains: None,
                json_pointer: None,
                json_value: None,
                timeout_secs: Some(3),
            }],
        };
        let r = g.grade(&ctx(dir.path(), &run)).await;
        assert!(matches!(r.verdict, Verdict::Skipped { .. }));
    }

    #[test]
    fn grader_yaml_round_trips() {
        let yaml = r#"
type: all
of:
  - type: command
    steps:
      - cmd: cargo
        args: [test]
  - type: transcript
    assertions:
      - assert: used_tool
        tool: bash
"#;
        let g: Grader = serde_yaml::from_str(yaml).expect("parse");
        match &g {
            Grader::All { of } => assert_eq!(of.len(), 2),
            other => panic!("wrong variant: {:?}", other),
        }
    }
}
