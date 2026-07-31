//! `/loop` engine — recurring + self-paced "loop-until-done" execution (gap C1).
//!
//! Three forms, parity with Claude Code `/loop` and Codex `/goal` ergonomics:
//!
//! * **Recurring** — `/loop <interval> <prompt>` re-runs a prompt on a fixed
//!   cadence (e.g. `5m`, `30s`, `1h`) until stopped, expired, or the iteration
//!   cap is hit.
//! * **Self-paced** — `/loop auto <prompt>` (alias `--until-done`) re-runs the
//!   prompt until a validator says the task is provably done, bounded by a
//!   `MAX_ITER` guard (default 20) so it can never spin forever.
//! * **Goal-driven** — `/loop goal <id-prefix>` is a self-paced loop whose body
//!   and stop condition both come from a durable [`crate::exec_goal::Goal`]: the
//!   goal's statement drives each iteration and its `success_criteria` are what
//!   the validator checks. Confirmed completion marks the goal `done`.
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
    /// Names of `WorkspaceStore` secrets to inject into the loop body's
    /// environment each iteration (gap C1). Only the *names* are persisted here;
    /// the values live in the encrypted `WorkspaceStore` and are resolved at run
    /// time via [`resolve_loop_secrets`] — never written to `loops.json` or any
    /// plaintext file. `#[serde(default)]` keeps older persisted jobs loadable.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// `/loop goal <id>` binding — the durable [`crate::exec_goal::Goal`] this
    /// loop is driving toward, if any.
    ///
    /// Two-phase by design, because resolution needs the store and this module
    /// stays pure: [`parse_loop_args`] leaves the **user-typed id prefix** here
    /// and stashes any extra guidance in `prompt`; [`hydrate_goal_loop`]
    /// replaces both with the resolved full id and the rendered goal brief.
    /// Only hydrated specs are ever persisted. `#[serde(default)]` keeps
    /// pre-goal jobs in `loops.json` loadable.
    #[serde(default)]
    pub goal_id: Option<String>,
}

impl LoopSpec {
    /// Whether this loop is driving a `/goal` rather than a bare prompt.
    pub fn is_goal_loop(&self) -> bool {
        self.goal_id.is_some()
    }
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
    /// C1 — machine-off hosted execution. When true this loop is run by the
    /// daemon's resident scheduler ([`crate::hosted_loop`]) rather than an
    /// interactive REPL, so it survives client disconnect. Default false keeps
    /// REPL-created loops (and older persisted jobs) interactive-only.
    #[serde(default)]
    pub hosted: bool,
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
            hosted: false,
        }
    }

    /// Mark this job for daemon-resident hosted execution (C1). Chainable.
    pub fn hosted(mut self) -> Self {
        self.hosted = true;
        self
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
/// * `goal <id-prefix> [guidance]` → self-paced, bound to a durable goal;
///   returns an **unhydrated** spec (see [`LoopSpec::goal_id`]).
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
        let (prompt, secrets) = extract_secret_flags(rest);
        if prompt.is_empty() {
            return Err("A self-paced loop needs a prompt: /loop auto <prompt>".to_string());
        }
        return Ok(LoopSpec {
            mode: LoopMode::SelfPaced,
            prompt,
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: DEFAULT_MAX_DURATION.as_secs(),
            secrets,
            goal_id: None,
        });
    }

    if head.eq_ignore_ascii_case("goal") {
        return parse_goal_loop_args(rest);
    }

    if let Some(interval) = parse_interval(head) {
        let (prompt, secrets) = extract_secret_flags(rest);
        if prompt.is_empty() {
            return Err("A recurring loop needs a prompt: /loop <interval> <prompt>".to_string());
        }
        return Ok(LoopSpec {
            mode: LoopMode::Recurring {
                interval_secs: interval.as_secs(),
            },
            prompt,
            // Recurring loops also honour an iteration cap so an unattended REPL
            // can't run unbounded; the wall-clock budget is the primary expiry.
            max_iter: DEFAULT_MAX_ITER,
            max_duration_secs: DEFAULT_MAX_DURATION.as_secs(),
            secrets,
            goal_id: None,
        });
    }

    Err(usage())
}

/// Parse the tail of `/loop goal <id-prefix> [guidance] [flags]`.
///
/// Flags (order-independent, may appear anywhere in the tail):
/// * `--max-iter N` — override the iteration cap.
/// * `--max-duration <interval>` — override the wall-clock budget (`45m`, `2h`).
/// * `--secret NAME` — repeatable, same semantics as the other loop forms.
///
/// The first non-flag token is the goal id prefix; anything after it is extra
/// guidance, parked in `prompt` until [`hydrate_goal_loop`] folds it into the
/// rendered brief.
pub fn parse_goal_loop_args(rest: &str) -> Result<LoopSpec, String> {
    let (residual, secrets) = extract_secret_flags(rest);
    let (residual, max_iter, max_duration_secs) = extract_goal_bounds(&residual)?;
    let (prefix, guidance) = match residual.trim().split_once(char::is_whitespace) {
        Some((p, g)) => (p, g.trim().to_string()),
        None => (residual.trim(), String::new()),
    };
    if prefix.is_empty() {
        return Err(
            "A goal loop needs a goal id: /loop goal <id-prefix> [extra guidance]".to_string(),
        );
    }
    Ok(LoopSpec {
        mode: LoopMode::SelfPaced,
        prompt: guidance,
        max_iter,
        max_duration_secs,
        secrets,
        goal_id: Some(prefix.to_string()),
    })
}

/// Pull `--max-iter N` / `--max-duration <interval>` out of a goal-loop tail,
/// returning the cleaned residual plus the effective caps (defaults when the
/// flags are absent). A flag with a missing or unparseable value is an error
/// rather than a silent fallback — a mistyped bound on an autonomous loop is
/// exactly the thing that should not be guessed at.
fn extract_goal_bounds(rest: &str) -> Result<(String, u32, u64), String> {
    let mut words: Vec<&str> = Vec::new();
    let mut max_iter = DEFAULT_MAX_ITER;
    let mut max_duration_secs = DEFAULT_MAX_DURATION.as_secs();
    let mut tokens = rest.split_whitespace();
    while let Some(tok) = tokens.next() {
        match tok {
            "--max-iter" => {
                let raw = tokens
                    .next()
                    .ok_or_else(|| "--max-iter needs a number, e.g. --max-iter 8".to_string())?;
                max_iter =
                    raw.parse::<u32>().ok().filter(|n| *n > 0).ok_or_else(|| {
                        format!("--max-iter expects a positive integer, got {raw:?}")
                    })?;
            }
            "--max-duration" => {
                let raw = tokens.next().ok_or_else(|| {
                    "--max-duration needs an interval, e.g. --max-duration 45m".to_string()
                })?;
                max_duration_secs = parse_interval(raw)
                    .ok_or_else(|| {
                        format!("--max-duration expects an interval like 45m or 2h, got {raw:?}")
                    })?
                    .as_secs();
            }
            other => words.push(other),
        }
    }
    Ok((words.join(" "), max_iter, max_duration_secs))
}

// ── Goal-bound loops ─────────────────────────────────────────────────────────

/// Minimal projection of a [`crate::exec_goal::Goal`] needed to drive a loop.
///
/// Keeping this separate from `Goal` lets the prompt/validator builders stay
/// pure and unit-testable without a `SessionStore` — the store-backed
/// resolution happens once, at the REPL/daemon edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalBrief {
    pub id: String,
    pub title: String,
    pub statement: String,
    pub success_criteria: Vec<String>,
    /// Rendered `ExecutionPlan`, when the goal has one.
    pub plan: Option<String>,
}

impl GoalBrief {
    pub fn from_goal(goal: &crate::exec_goal::Goal) -> Self {
        Self {
            id: goal.id.clone(),
            title: goal.title.clone(),
            statement: goal.statement.clone(),
            success_criteria: goal.success_criteria.clone(),
            plan: goal.current_plan.as_ref().map(|p| p.display()),
        }
    }

    /// A goal with no recorded criteria can still drive a loop — the validator
    /// falls back to the statement — but the caller should say so out loud,
    /// because "done" then rests on a much softer judgement.
    pub fn has_criteria(&self) -> bool {
        !self.success_criteria.is_empty()
    }
}

/// Fold a resolved goal into an unhydrated `/loop goal` spec: the full goal id
/// replaces the typed prefix, and the rendered brief (plus any extra guidance
/// that was in `prompt`) becomes the loop body.
pub fn hydrate_goal_loop(spec: LoopSpec, brief: &GoalBrief) -> LoopSpec {
    let guidance = spec.prompt.trim().to_string();
    LoopSpec {
        prompt: goal_loop_prompt(brief, &guidance),
        goal_id: Some(brief.id.clone()),
        ..spec
    }
}

/// Numbered success-criteria block, or an explicit "none recorded" marker so
/// neither the worker nor the validator has to infer the absence.
fn criteria_block(brief: &GoalBrief) -> String {
    if !brief.has_criteria() {
        return "  (none recorded — judge against the statement and title above)".to_string();
    }
    brief
        .success_criteria
        .iter()
        .enumerate()
        .map(|(i, c)| format!("  {}. {c}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The per-iteration worker prompt for a goal-bound loop.
///
/// Deliberately does *not* ask the worker whether it is finished — completion
/// is decided by [`goal_validator_prompt`] in a separate turn, so a model that
/// declares victory mid-edit can't end the loop.
pub fn goal_loop_prompt(brief: &GoalBrief, guidance: &str) -> String {
    let statement = (!brief.statement.trim().is_empty())
        .then(|| format!("\n## Statement\n{}\n", brief.statement.trim()))
        .unwrap_or_default();
    let plan = brief
        .plan
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| format!("\n## Current plan\n{p}\n"))
        .unwrap_or_default();
    let extra = (!guidance.trim().is_empty())
        .then(|| {
            format!(
                "\n## Additional guidance for this run\n{}\n",
                guidance.trim()
            )
        })
        .unwrap_or_default();
    format!(
        "You are advancing a single durable goal. Make concrete progress in this \
         iteration; the loop will re-run you until every success criterion is met.\n\n\
         # Goal\n{title}\n{statement}\n## Success criteria (all must hold for the goal to be done)\n\
         {criteria}\n{plan}{extra}\n\
         Pick the highest-leverage criterion that is not yet met and drive it to \
         completion. Verify with builds/tests rather than assuming. Do not declare \
         the goal finished yourself — a separate validator checks the criteria after \
         each iteration.",
        title = brief.title,
        statement = statement,
        criteria = criteria_block(brief),
        plan = plan,
        extra = extra,
    )
}

/// System prompt for the loop's completion validator. Shared by the REPL and
/// the daemon's hosted scheduler so both judge "done" by the same standard.
pub const VALIDATOR_SYSTEM_PROMPT: &str =
    "You are a strict completion validator for an autonomous coding loop. \
     Given the user's goal and the current repository state, decide whether \
     the goal is fully and verifiably achieved. Reply with exactly one word: \
     DONE if it is complete, or CONTINUE if any work remains.";

/// Read a validator reply as a done-signal. Anything that isn't an explicit
/// `DONE` — including a hedge or an error message — means keep going.
pub fn validator_says_done(reply: &str) -> bool {
    reply.trim().to_uppercase().starts_with("DONE")
}

/// The validator prompt for a goal-bound loop: criterion-by-criterion, and
/// `DONE` only when every one of them holds.
pub fn goal_validator_prompt(brief: &GoalBrief) -> String {
    let statement = (!brief.statement.trim().is_empty())
        .then(|| format!("\nStatement:\n{}\n", brief.statement.trim()))
        .unwrap_or_default();
    format!(
        "Goal: {title}\n{statement}\nSuccess criteria:\n{criteria}\n\n\
         Check each criterion against the current repository state. Answer DONE only \
         if EVERY criterion is verifiably met right now; if even one is unmet, \
         partially done, or unverifiable, answer CONTINUE.",
        title = brief.title,
        statement = statement,
        criteria = criteria_block(brief),
    )
}

/// Pull `--secret NAME` (repeatable) flags out of a loop's argument tail,
/// returning the cleaned prompt (flags removed) and the collected secret names.
/// A trailing `--secret` with no name is ignored. Names are the keys looked up
/// in the `WorkspaceStore` at run time by [`resolve_loop_secrets`].
pub fn extract_secret_flags(rest: &str) -> (String, Vec<String>) {
    let mut prompt_words: Vec<&str> = Vec::new();
    let mut secrets: Vec<String> = Vec::new();
    let mut tokens = rest.split_whitespace();
    while let Some(tok) = tokens.next() {
        if tok == "--secret" {
            if let Some(name) = tokens.next() {
                if !secrets.iter().any(|s| s == name) {
                    secrets.push(name.to_string());
                }
            }
        } else {
            prompt_words.push(tok);
        }
    }
    (prompt_words.join(" "), secrets)
}

/// Resolve a loop's declared secret names into `(NAME, VALUE)` environment pairs
/// using `lookup` — at the live call site a closure over
/// `WorkspaceStore::secret_get`, so values are read from the encrypted store and
/// never touch `loops.json` or the environment until injected into the child
/// process. Returns the resolved pairs plus the names that were missing (so the
/// caller can warn rather than silently run without a required secret).
pub fn resolve_loop_secrets<F>(names: &[String], lookup: F) -> (Vec<(String, String)>, Vec<String>)
where
    F: Fn(&str) -> Option<String>,
{
    let mut resolved = Vec::new();
    let mut missing = Vec::new();
    for name in names {
        match lookup(name) {
            Some(value) => resolved.push((name.clone(), value)),
            None => missing.push(name.clone()),
        }
    }
    (resolved, missing)
}

fn usage() -> String {
    "Usage: /loop <interval> <prompt>   (e.g. /loop 5m run the tests)\n       \
     /loop auto <prompt>          (self-paced, runs until done; MAX_ITER guard)\n       \
     /loop goal <id> [guidance]   (drive a /goal until its success criteria are met)\n       \
     goal loops accept --max-iter N and --max-duration 45m\n       \
     add --secret NAME to inject a WorkspaceStore secret into the loop's env\n       \
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
        assert_eq!(spec.mode, LoopMode::Recurring { interval_secs: 300 });
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

    // -- C1: WorkspaceStore secret injection --------------------------------

    #[test]
    fn parse_extracts_secret_flags_from_prompt() {
        let spec = parse_loop_args("5m --secret OPENAI_KEY deploy the service --secret DB_URL")
            .unwrap();
        assert_eq!(spec.prompt, "deploy the service");
        assert_eq!(spec.secrets, vec!["OPENAI_KEY", "DB_URL"]);

        // Self-paced too, with a duplicate name deduped.
        let s2 = parse_loop_args("auto --secret TOK --secret TOK run it").unwrap();
        assert_eq!(s2.prompt, "run it");
        assert_eq!(s2.secrets, vec!["TOK"]);
    }

    #[test]
    fn parse_without_secrets_defaults_empty() {
        let spec = parse_loop_args("5m just run").unwrap();
        assert!(spec.secrets.is_empty());
    }

    #[test]
    fn secret_flag_without_name_is_ignored_and_still_needs_prompt() {
        // A dangling `--secret` consumes no name and leaves an empty prompt → err.
        assert!(parse_loop_args("auto --secret").is_err());
    }

    #[test]
    fn resolve_secrets_splits_present_and_missing() {
        let names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let store: std::collections::HashMap<&str, &str> =
            [("A", "1"), ("C", "3")].into_iter().collect();
        let (resolved, missing) =
            resolve_loop_secrets(&names, |k| store.get(k).map(|v| v.to_string()));
        assert_eq!(
            resolved,
            vec![("A".to_string(), "1".to_string()), ("C".to_string(), "3".to_string())]
        );
        assert_eq!(missing, vec!["B".to_string()]);
    }

    // -- Goal-bound loops ---------------------------------------------------

    fn brief() -> GoalBrief {
        GoalBrief {
            id: "abc123def".into(),
            title: "Ship the loop↔goal wiring".into(),
            statement: "Wire /loop to run a goal until its criteria are met.".into(),
            success_criteria: vec![
                "cargo test --workspace passes".into(),
                "docs/vibecli.md documents the command".into(),
            ],
            plan: None,
        }
    }

    #[test]
    fn parse_goal_form_yields_unhydrated_self_paced_spec() {
        let spec = parse_loop_args("goal abc123 focus on the parser first").unwrap();
        assert_eq!(spec.mode, LoopMode::SelfPaced);
        assert!(spec.is_goal_loop());
        // Pre-hydration: prefix in goal_id, extra guidance in prompt.
        assert_eq!(spec.goal_id.as_deref(), Some("abc123"));
        assert_eq!(spec.prompt, "focus on the parser first");
        assert_eq!(spec.max_iter, DEFAULT_MAX_ITER);

        // Bare form — no guidance.
        let bare = parse_loop_args("goal abc123").unwrap();
        assert_eq!(bare.goal_id.as_deref(), Some("abc123"));
        assert!(bare.prompt.is_empty());
    }

    #[test]
    fn goal_form_honours_bound_and_secret_flags() {
        let spec =
            parse_loop_args("goal abc --max-iter 3 --secret TOK --max-duration 45m be careful")
                .unwrap();
        assert_eq!(spec.goal_id.as_deref(), Some("abc"));
        assert_eq!(spec.max_iter, 3);
        assert_eq!(spec.max_duration_secs, 45 * 60);
        assert_eq!(spec.secrets, vec!["TOK"]);
        assert_eq!(spec.prompt, "be careful");
    }

    #[test]
    fn goal_form_rejects_missing_id_and_bad_bounds() {
        assert!(parse_loop_args("goal").is_err());
        assert!(parse_loop_args("goal --max-iter 5").is_err()); // flags only, no id
        assert!(parse_loop_args("goal abc --max-iter 0").is_err());
        assert!(parse_loop_args("goal abc --max-iter lots").is_err());
        assert!(parse_loop_args("goal abc --max-iter").is_err());
        assert!(parse_loop_args("goal abc --max-duration never").is_err());
    }

    #[test]
    fn hydration_swaps_prefix_for_id_and_renders_the_brief() {
        let spec = parse_loop_args("goal abc keep the diff small").unwrap();
        let hydrated = hydrate_goal_loop(spec, &brief());
        // Full id replaces the typed prefix.
        assert_eq!(hydrated.goal_id.as_deref(), Some("abc123def"));
        // The brief carries title, statement, every criterion, and the guidance.
        assert!(hydrated.prompt.contains("Ship the loop↔goal wiring"));
        assert!(hydrated.prompt.contains("cargo test --workspace passes"));
        assert!(hydrated
            .prompt
            .contains("docs/vibecli.md documents the command"));
        assert!(hydrated.prompt.contains("keep the diff small"));
        // Caps and mode survive hydration untouched.
        assert_eq!(hydrated.mode, LoopMode::SelfPaced);
        assert_eq!(hydrated.max_iter, DEFAULT_MAX_ITER);
    }

    #[test]
    fn worker_prompt_defers_completion_to_the_validator() {
        let p = goal_loop_prompt(&brief(), "");
        assert!(p.contains("Do not declare"));
        // No guidance section when none was given.
        assert!(!p.contains("Additional guidance"));
        // A goal with a plan surfaces it.
        let with_plan = GoalBrief {
            plan: Some("1. do the thing".into()),
            ..brief()
        };
        assert!(goal_loop_prompt(&with_plan, "").contains("1. do the thing"));
    }

    #[test]
    fn validator_prompt_enumerates_every_criterion() {
        let v = goal_validator_prompt(&brief());
        assert!(v.contains("1. cargo test --workspace passes"));
        assert!(v.contains("2. docs/vibecli.md documents the command"));
        assert!(v.contains("EVERY criterion"));
    }

    #[test]
    fn criteria_free_goal_falls_back_to_the_statement() {
        let bare = GoalBrief {
            success_criteria: Vec::new(),
            ..brief()
        };
        assert!(!bare.has_criteria());
        // Both prompts say so explicitly rather than showing an empty list.
        assert!(goal_loop_prompt(&bare, "").contains("none recorded"));
        assert!(goal_validator_prompt(&bare).contains("none recorded"));
    }

    #[test]
    fn goal_binding_survives_persistence_and_older_jobs_load() {
        let spec = hydrate_goal_loop(parse_loop_args("goal abc").unwrap(), &brief());
        let job = LoopJob::new("loop-g".into(), spec, 7);
        let back: LoopJob = serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert_eq!(back.spec.goal_id.as_deref(), Some("abc123def"));

        // A job persisted before goal loops existed still loads (serde default).
        let legacy = r#"{"id":"x","spec":{"mode":"SelfPaced","prompt":"p","max_iter":5,"max_duration_secs":60,"secrets":[]},"iterations_done":0,"status":"running","created_at_secs":1}"#;
        let loaded: LoopJob = serde_json::from_str(legacy).unwrap();
        assert!(!loaded.spec.is_goal_loop());
    }

    #[test]
    fn secrets_survive_persistence_roundtrip() {
        let spec = parse_loop_args("10s --secret KEY tick").unwrap();
        let job = LoopJob::new("loop-abc".into(), spec, 100);
        let json = serde_json::to_string(&job).unwrap();
        let back: LoopJob = serde_json::from_str(&json).unwrap();
        assert_eq!(back.spec.secrets, vec!["KEY"]);
        // And an old job JSON with no `secrets` field still loads (serde default).
        let legacy = r#"{"id":"x","spec":{"mode":{"SelfPaced":null},"prompt":"p","max_iter":5,"max_duration_secs":60},"iterations_done":0,"status":"running","created_at_secs":1}"#;
        let loaded: LoopJob = serde_json::from_str(legacy).unwrap();
        assert!(loaded.spec.secrets.is_empty());
    }
}
