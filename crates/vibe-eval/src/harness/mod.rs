//! Harnesses: the things under test.
//!
//! A harness is "how do I hand this prompt to VibeCody through *this* surface
//! and get back what it did". The runner and the graders are written against
//! the trait, so adding a surface never touches either — which is the point,
//! since VibeCody is one daemon behind fourteen clients and the interesting
//! failures live in the transports rather than in the agent loop.
//!
//! Two rules keep the numbers honest:
//!
//! 1. [`Harness::preflight`] runs before any task, and an unavailable surface
//!    produces *skipped* tasks with a stated reason. A stopped daemon must
//!    never look like a capability regression.
//! 2. [`RunOutcome`] models what a surface actually reported. Fields a surface
//!    does not provide stay `None`/empty rather than being filled with a
//!    plausible default, and graders that need them say so.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::task::{EvalTask, Surface};

pub mod cli;
pub mod daemon;
pub mod probe;

pub use cli::{CliConfig, CliHarness};
pub use daemon::{DaemonConfig, DaemonHarness};
pub use probe::ProbeHarness;

/// One tool call the agent made.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    pub tool: String,
    #[serde(default)]
    pub input_summary: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub duration_ms: u64,
}

/// What a surface reported about a run.
///
/// Every optional field means "this surface did not tell us". Graders treat
/// that as [`crate::grade::Verdict::Error`] rather than inventing a value —
/// see the transcript assertions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOutcome {
    /// The agent's final message / summary.
    pub final_text: String,
    /// Surface-reported outcome (`success`, `partial`, `failed`, …), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    /// Tool calls, in order. Empty means "not reported", which is why
    /// transcript graders check emptiness before asserting.
    #[serde(default)]
    pub steps: Vec<StepRecord>,
    pub duration_ms: u64,
    /// Process exit code, for surfaces that are processes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Raw surface response, kept for debugging a confusing result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// Whether a surface can be exercised right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preflight {
    Ready,
    /// The surface is not usable, and here is the specific reason — "daemon
    /// not running on :7878", not "unavailable". Four different causes with
    /// one message is how an operator ends up debugging the wrong thing.
    Unavailable {
        reason: String,
    },
}

impl Preflight {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Preflight::Unavailable {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("could not start the surface: {0}")]
    Spawn(String),
    #[error("surface returned no usable response: {0}")]
    Protocol(String),
    /// The run blew its budget. `tail` carries the last of whatever the
    /// surface had printed before it was killed.
    ///
    /// Without it a long timeout is a black box: three consecutive
    /// hour-long greenfield runs produced no file and no explanation, and
    /// "the agent is bad" was indistinguishable from "the provider stopped
    /// answering forty minutes in". The tail is the only evidence that
    /// survives a killed process.
    #[error("run exceeded its {secs}s budget")]
    Timeout { secs: u64, tail: String },
    #[error("transport error: {0}")]
    Transport(String),
}

/// How the runner drives one surface.
#[async_trait::async_trait]
pub trait Harness: Send + Sync {
    fn surface(&self) -> Surface;

    /// Human-readable identity of what is being tested — binary path, daemon
    /// URL, provider and model. Recorded in the report so a number is always
    /// attributable to a configuration.
    fn describe(&self) -> String;

    /// Checked once before the run. Cheap, and never mutates anything.
    async fn preflight(&self) -> Preflight;

    /// Run one task in `workspace` and report what happened.
    async fn run(
        &self,
        task: &EvalTask,
        workspace: &Path,
        timeout: Duration,
    ) -> Result<RunOutcome, HarnessError>;
}

/// Read the daemon bearer token.
///
/// The token rotates on every daemon start and VibeCody restarts the daemon
/// itself, so it is read fresh at call time rather than cached — a token
/// captured at construction is stale by the time a long suite reaches its
/// second surface.
pub(crate) fn read_daemon_token() -> Option<String> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
    std::fs::read_to_string(home.join(".vibecli").join("daemon.token"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Default daemon port, honouring the documented environment overrides.
pub(crate) fn daemon_port() -> u16 {
    std::env::var("VIBECLI_DAEMON_PORT")
        .or_else(|_| std::env::var("VIBEDESK_DAEMON_PORT"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7878)
}

/// Identity check against `GET /health`.
///
/// Liveness is not identity: any process holding the port answers a TCP
/// connect, and treating that as "the daemon is up" makes every downstream
/// panel report a daemon fault when the real problem is a port conflict. The
/// current daemon reports `service: "vibecli"`; daemons predating that field
/// are accepted via their legacy shape (`status: "ok"` plus a `version`),
/// because VibeCody reuses an already-running daemon and refusing the older
/// shape tells upgrading users their own daemon is "another program".
pub(crate) fn health_is_vibecli(body: &serde_json::Value) -> bool {
    let service_matches = body
        .get("service")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("vibecli"));
    let legacy_shape = body
        .get("status")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("ok"))
        && body.get("version").is_some();
    service_matches || legacy_shape
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn health_accepts_the_current_service_field() {
        assert!(health_is_vibecli(
            &json!({"service": "vibecli", "status": "ok"})
        ));
    }

    #[test]
    fn health_accepts_a_pre_service_daemon_by_its_legacy_shape() {
        // Upgrading users run a daemon that predates `service`. Rejecting it
        // would report their own daemon as a foreign process on the port.
        assert!(health_is_vibecli(
            &json!({"status": "ok", "version": "0.5.1"})
        ));
    }

    #[test]
    fn health_rejects_an_unrelated_service_on_the_port() {
        assert!(!health_is_vibecli(&json!({"service": "grafana"})));
        assert!(!health_is_vibecli(&json!({"status": "ok"})));
        assert!(!health_is_vibecli(&json!({})));
    }

    #[test]
    fn daemon_port_default_is_7878() {
        // Only asserted when the env is not overridden, so this stays true on
        // a developer machine that exports the variable.
        if std::env::var_os("VIBECLI_DAEMON_PORT").is_none()
            && std::env::var_os("VIBEDESK_DAEMON_PORT").is_none()
        {
            assert_eq!(daemon_port(), 7878);
        }
    }
}
