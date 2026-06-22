//! `/loop` engine — recurring + self-paced "loop-until-done" execution (gap C1).
//!
//! Two modes, parity with Claude Code `/loop` and Codex `/goal` ergonomics:
//!
//! * **Recurring** — `/loop <interval> <prompt>` re-runs a prompt on a fixed
//!   cadence (e.g. `5m`, `30s`, `1h`) until stopped, expired, or the iteration
//!   cap is hit.
//! * **Self-paced** — `/loop auto <prompt>` (alias `--until-done`) re-runs the
//!   prompt until a validator says the task is provably done, bounded by a
//!   `MAX_ITER` guard (default 20) so it can never spin forever.
//!
//! Every job gets a short ID, an auto-expiry (wall-clock + iteration caps), and
//! is persisted to `~/.vibecli/loops.json` so `/loop list` / `/loop stop` work
//! across the REPL session. The trigger *plumbing* (Cron/FileWatch/Webhook)
//! already lives in [`crate::automations`]; this module adds the loop-controller
//! ergonomic and the self-paced loop-until-done decision logic that was missing.
//!
//! This module is intentionally pure/decidable: it owns parsing, the
//! continue/stop decision, and persistence. The REPL layer supplies the actual
//! per-iteration agent run and the LLM "done?" validator, so the control logic
//! is unit-testable without a provider.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Default guard on self-paced loops — never run the body more than this many
/// times, regardless of the validator (Ralph-loop / loop-engineering bound).
pub const DEFAULT_MAX_ITER: u32 = 20;

/// Default wall-clock expiry for a loop job (30 minutes) when none is given.
pub const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(30 * 60);

/// How a loop paces itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopMode {
    /// Re-run on a fixed cadence.
    Recurring { interval_secs: u64 },
    /// Re-run until a validator reports the task done.
    SelfPaced,
}

/// A parsed `/loop` invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopSpec {
    pub mode: LoopMode,
    pub prompt: String,
    pub max_iter: u32,
    pub max_duration_secs: u64,
}

/// Lifecycle status of a loop job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopStatus {
    Running,
    Done,
    Stopped,
    Expired,
    MaxIter,
    Failed,
}

impl LoopStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, LoopStatus::Running)
    }
}

/// A persisted loop job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopJob {
    pub id: String,
    pub spec: LoopSpec,
    pub iterations_done: u32,
    pub status: LoopStatus,
    /// Unix-epoch seconds when the job was created.
    pub created_at_secs: u64,
}

/// The controller's decision for the next tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopDecision {
    /// Run the body again.
    Continue,
    /// Validator reported the task complete (self-paced only).
    StopDone,
    /// `max_iter` reached.
    StopMaxIter,
    /// Wall-clock budget exhausted.
    StopExpired,
}

impl LoopJob {
    pub fn new(id: String, spec: LoopSpec, created_at_secs: u64) -> Self {
        Self {
            id,
            spec,
            iterations_done: 0,
            status: LoopStatus::Running,
            created_at_secs,
        }
    }

    /// Decide whether to run the body again.
    ///
    /// * `done_signal` — for self-paced loops, whether the validator says the
    ///   task is complete (ignored for recurring loops, which run until the
    ///   caller stops them or a bound is hit).
    /// * `elapsed_secs` — wall-clock seconds since the job started.
    ///
    /// Caps are checked first so a self-paced loop with a satisfied validator on
    /// the very last allowed iteration still reports `StopDone` (success wins
    /// over `MaxIter`) — but an exhausted budget always halts.
    pub fn decide_next(&self, done_signal: bool, elapsed_secs: u64) -> LoopDecision {
        if elapsed_secs >= self.spec.max_duration_secs {
            return LoopDecision::StopExpired;
        }
        if matches!(self.spec.mode, LoopMode::SelfPaced) && done_signal {
            return LoopDecision::StopDone;
        }
        if self.iterations_done >= self.spec.max_iter {
            return LoopDecision::StopMaxIter;
        }
        LoopDecision::Continue
    }

    /// Map a non-`Continue` decision onto a terminal status.
    pub fn status_for(decision: LoopDecision) -> LoopStatus {
        match decision {
            LoopDecision::StopDone => LoopStatus::Done,
            LoopDecision::StopMaxIter => LoopStatus::MaxIter,
            LoopDecision::StopExpired => LoopStatus::Expired,
            LoopDecision::Continue => LoopStatus::Running,
        }
    }
}

/// Parse a duration token like `30s`, `5m`, `1h`, or a bare integer (seconds).
/// Returns `None` for unparseable / zero / negative input.
pub fn parse_interval(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num_part, mult) = if let Some(n) = s.strip_suffix('s') {
        (n, 1u64)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60)
    } else if let Some(n) = s.strip_suffix('h') {
        (n, 3600)
    } else {
        (s, 1)
    };
    let n: u64 = num_part.trim().parse().ok()?;
    if n == 0 {
        return None;
    }
    Some(Duration::from_secs(n * mult))
}

/// Parse the argument string of a `/loop` command into a [`LoopSpec`].
///
/// Accepted forms:
/// * `auto <prompt>` / `--until-done <prompt>` → self-paced (MAX_ITER guard).
/// * `<interval> <prompt>` (e.g. `5m run the tests`) → recurring.
///
/// Returns `Err` with a usage hint when the form is unrecognised or the prompt
/// is empty.
pub fn parse_loop_args(args: &str) -> Result<LoopSpec, String> {
    let args = args.trim();
    let (head, rest) = match args.split_once(char::is_whitespace) {
        Some((h, r)) => (h, r.trim()),
        None => (args, ""),
    };
    if head.is_empty() {
        return Err(usage());
    }

    if head.eq_ignore_ascii_case("auto") || head == "--until-done" {
        if rest.is_empty() {
            return Err("A self-paced loop needs a prompt: /loop auto <prompt>".to_string());
        }
        return Ok(LoopSpec {
            mode: LoopMode::SelfPaced,
            prompt: rest.to_string(),
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: DEFAULT_MAX_DURATION.as_secs(),
        });
    }

    if let Some(interval) = parse_interval(head) {
        if rest.is_empty() {
            return Err("A recurring loop needs a prompt: /loop <interval> <prompt>".to_string());
        }
        return Ok(LoopSpec {
            mode: LoopMode::Recurring {
                interval_secs: interval.as_secs(),
            },
            prompt: rest.to_string(),
            // Recurring loops also honour an iteration cap so an unattended REPL
            // can't run unbounded; the wall-clock budget is the primary expiry.
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: DEFAULT_MAX_DURATION.as_secs(),
        });
    }

    Err(usage())
}

fn usage() -> String {
    "Usage: /loop <interval> <prompt>   (e.g. /loop 5m run the tests)\n       \
     /loop auto <prompt>          (self-paced, runs until done; MAX_ITER guard)\n       \
     /loop list | /loop stop <id> | /loop status <id>"
        .to_string()
}

/// Generate a short, sortable job id from a unix timestamp (seconds) + a salt.
/// Caller passes the timestamp so this stays deterministic/testable.
pub fn job_id(now_secs: u64, salt: u32) -> String {
    format!("loop-{:x}{:02x}", now_secs, salt & 0xff)
}

/// Path to the persisted loop-jobs file (`~/.vibecli/loops.json`).
pub fn loops_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("loops.json")
}

/// Load persisted loop jobs (empty on missing/corrupt file — never errors).
pub fn load_jobs(path: &PathBuf) -> Vec<LoopJob> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist loop jobs, creating the parent dir if needed.
pub fn save_jobs(path: &PathBuf, jobs: &[LoopJob]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(jobs).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_units() {
        assert_eq!(parse_interval("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_interval("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_interval("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_interval("45"), Some(Duration::from_secs(45)));
        assert_eq!(parse_interval("0s"), None);
        assert_eq!(parse_interval("abc"), None);
        assert_eq!(parse_interval(""), None);
    }

    #[test]
    fn parse_self_paced() {
        let spec = parse_loop_args("auto fix all failing tests").unwrap();
        assert_eq!(spec.mode, LoopMode::SelfPaced);
        assert_eq!(spec.prompt, "fix all failing tests");
        assert_eq!(spec.max_iter, DEFAULT_MAX_ITER);

        let spec2 = parse_loop_args("--until-done ship the feature").unwrap();
        assert_eq!(spec2.mode, LoopMode::SelfPaced);
        assert_eq!(spec2.prompt, "ship the feature");
    }

    #[test]
    fn parse_recurring() {
        let spec = parse_loop_args("5m run the tests").unwrap();
        assert_eq!(
            spec.mode,
            LoopMode::Recurring { interval_secs: 300 }
        );
        assert_eq!(spec.prompt, "run the tests");
    }

    #[test]
    fn parse_errors() {
        assert!(parse_loop_args("").is_err());
        assert!(parse_loop_args("auto").is_err()); // no prompt
        assert!(parse_loop_args("5m").is_err()); // no prompt
        assert!(parse_loop_args("notaninterval and a prompt").is_err());
    }

    #[test]
    fn self_paced_stops_when_done() {
        let spec = parse_loop_args("auto do the thing").unwrap();
        let job = LoopJob::new("loop-1".into(), spec, 0);
        // Validator says done on the first check, well within budget.
        assert_eq!(job.decide_next(true, 1), LoopDecision::StopDone);
        // Not done yet → keep going.
        assert_eq!(job.decide_next(false, 1), LoopDecision::Continue);
    }

    #[test]
    fn max_iter_guard_halts_runaway() {
        let mut spec = parse_loop_args("auto never satisfiable").unwrap();
        spec.max_iter = 3;
        let mut job = LoopJob::new("loop-2".into(), spec, 0);
        job.iterations_done = 3;
        // Validator never satisfied, but the cap is reached.
        assert_eq!(job.decide_next(false, 1), LoopDecision::StopMaxIter);
    }

    #[test]
    fn wall_clock_expiry_wins() {
        let mut spec = parse_loop_args("auto long task").unwrap();
        spec.max_duration_secs = 10;
        let job = LoopJob::new("loop-3".into(), spec, 0);
        // Budget exhausted halts even if the validator would say "done".
        assert_eq!(job.decide_next(true, 10), LoopDecision::StopExpired);
        assert_eq!(job.decide_next(false, 999), LoopDecision::StopExpired);
    }

    #[test]
    fn status_mapping() {
        assert_eq!(
            LoopJob::status_for(LoopDecision::StopDone),
            LoopStatus::Done
        );
        assert_eq!(
            LoopJob::status_for(LoopDecision::StopMaxIter),
            LoopStatus::MaxIter
        );
        assert!(LoopStatus::Done.is_terminal());
        assert!(!LoopStatus::Running.is_terminal());
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vibecli-loop-test-{}", job_id(42, 7)));
        let path = dir.join("loops.json");
        let spec = parse_loop_args("auto persist me").unwrap();
        let jobs = vec![LoopJob::new("loop-x".into(), spec, 123)];
        save_jobs(&path, &jobs).unwrap();
        let loaded = load_jobs(&path);
        assert_eq!(loaded, jobs);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
