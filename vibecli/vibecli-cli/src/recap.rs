//! Recap & Resume — Phase F1.1 foundation.
//!
//! Per `docs/design/recap-resume/README.md`, a recap is a structured,
//! on-demand summary of a unit of work. This module holds the
//! cross-cutting `Recap` shape (Session / Job / DiffChain) plus the
//! Session-only **heuristic generator** — the deterministic, offline
//! algorithm that produces a usable recap without an LLM.
//!
//! F1.1 scope:
//! - Types: `Recap`, `RecapKind`, `RecapGenerator`, `RecapArtifact`,
//!   `ArtifactKind`, `ResumeHint`, `ResumeTarget`, `RecapTokenUsage`.
//! - Heuristic generator over `SessionDetail` (already in
//!   `session_store.rs`).
//!
//! F1.2 (separate slice) wires the `recaps` table CRUD and HTTP routes.
//! F1.3 wires `/v1/resume`. This module deliberately has no
//! database / HTTP code so it stays pure and trivially testable.
//!
//! See also: `01-session.md` for triggers, slicing, failure modes.
//!
//! ## Patent-distance note
//!
//! Recap is itself a context-trimming primitive — it shrinks context.
//! The Session-kind generator does not predict future code, does not
//! render inline, has no accept/reject UI on code, and runs only on
//! explicit triggers. The five Path-D claim elements remain at
//! distance. Diffcomplete-kind recaps (separate slice D1) require a
//! per-slice patent re-audit before they ship.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::session_store::{MessageRow, SessionDetail, StepRow};

/// Stable wire shape for a recap, mirrored across SQL row + REST JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recap {
    pub id: String,
    pub kind: RecapKind,
    pub subject_id: String,
    /// The last message ID included when the recap was generated. Acts
    /// as the idempotency key together with `subject_id`: regenerating
    /// a recap with the same `(subject_id, last_message_id)` returns
    /// the existing row unchanged.
    #[serde(default)]
    pub last_message_id: Option<i64>,
    pub workspace: Option<PathBuf>,
    pub generated_at: DateTime<Utc>,
    pub generator: RecapGenerator,
    /// ≤ 80 chars, single line, plain prose. Trailing punctuation
    /// stripped so UIs can append their own.
    pub headline: String,
    /// 3–7 short bullets describing what happened. Each ≤ 120 chars.
    pub bullets: Vec<String>,
    /// 0–3 imperative-form follow-ups; populates `seed_instruction` on
    /// `ResumeHint` if non-empty.
    pub next_actions: Vec<String>,
    pub artifacts: Vec<RecapArtifact>,
    pub resume_hint: Option<ResumeHint>,
    pub token_usage: Option<RecapTokenUsage>,
    /// Schema version for forward-compat. Start at 1; bump on
    /// breaking field changes, not additive ones.
    pub schema_version: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecapKind {
    Session,
    Job,
    DiffChain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecapGenerator {
    /// Deterministic, offline, < 50ms on 200-message session. Default.
    Heuristic,
    /// Routed to the user's currently selected provider — never a
    /// silent fan-out (see `01-session.md` "LLM recap prompt").
    Llm { provider: String, model: String },
    /// User edited an LLM/heuristic recap via `/recap --edit`.
    UserEdited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapArtifact {
    pub kind: ArtifactKind,
    pub label: String,
    pub locator: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    File,
    Diff,
    Job,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResumeHint {
    pub target: ResumeTarget,
    /// Resume cursor for sessions; `None` = end of transcript.
    pub from_message: Option<i64>,
    pub from_step: Option<u32>,
    pub from_diff_index: Option<u32>,
    pub seed_instruction: Option<String>,
    /// When `true`, resume forks a new `session_id`. Default `true`
    /// when `from_message` is set (mid-conversation), `false` when
    /// resuming from the tail.
    #[serde(default)]
    pub branch_on_resume: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResumeTarget {
    Session { id: String },
    Job { id: String },
    DiffChain { id: String },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapTokenUsage {
    pub input: u32,
    pub output: u32,
}

// ── Heuristic generator ──────────────────────────────────────────────────────

/// Generate a Session recap from a `SessionDetail` using only
/// deterministic rules. No LLM, no network, no I/O. Designed to run
/// in well under 50ms on a 200-message session — the contract pinned
/// by `heuristic_recap_runs_in_well_under_one_second_on_large_session`.
///
/// Algorithm (per `01-session.md` "Heuristic recap algorithm"):
/// 1. Headline: first prose user message (skip leading `/command`s),
///    trimmed to ≤ 80 chars, trailing punctuation stripped.
/// 2. Bullets: one per distinct tool name with count, plus a
///    "Stopped: …" bullet when the session ended in failure.
/// 3. next_actions: imperative-form sentences from the last 3
///    assistant messages, capped at 3.
/// 4. artifacts: unique file paths inferred from step input/output
///    summaries.
/// 5. resume_hint: `from_message = last_message_id`,
///    `seed_instruction = next_actions[0]` if present;
///    `branch_on_resume = false` (resume from tail).
pub fn heuristic_recap(detail: &SessionDetail) -> Recap {
    let headline = derive_headline(&detail.messages);
    let bullets = derive_bullets(&detail.steps, &detail.session.status);
    let next_actions = derive_next_actions(&detail.messages);
    let artifacts = derive_artifacts(&detail.steps);

    let last_message_id = detail.messages.last().map(|m| m.id);
    let seed_instruction = next_actions.first().cloned();

    let resume_hint = Some(ResumeHint {
        target: ResumeTarget::Session {
            id: detail.session.id.clone(),
        },
        from_message: last_message_id,
        from_step: None,
        from_diff_index: None,
        seed_instruction,
        // Tail resume: continue the same session_id, no fork.
        branch_on_resume: false,
    });

    Recap {
        id: new_recap_id(),
        kind: RecapKind::Session,
        subject_id: detail.session.id.clone(),
        last_message_id,
        workspace: detail
            .session
            .project_path
            .as_ref()
            .map(PathBuf::from),
        generated_at: Utc::now(),
        generator: RecapGenerator::Heuristic,
        headline,
        bullets,
        next_actions,
        artifacts,
        resume_hint,
        token_usage: None,
        schema_version: 1,
    }
}

/// Generate a fresh recap ID. UUIDv4 hex (32 chars, no dashes) — not
/// a true ULID, but the sort order doesn't matter at the ID level
/// because the SQL layer indexes `generated_at` for newest-first
/// pagination. Switching to ULID/UUIDv7 later is a non-breaking
/// change (still a TEXT PRIMARY KEY).
fn new_recap_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn derive_headline(messages: &[MessageRow]) -> String {
    // Skip system messages and `/command` user messages — those don't
    // describe what the user wanted from the conversation.
    let first_prose = messages.iter().find(|m| {
        m.role == "user" && !is_slash_command(&m.content)
    });
    let raw = match first_prose {
        Some(m) => m.content.trim(),
        None => return "(empty session)".to_string(),
    };

    // Take the first non-empty line so a multi-line message doesn't
    // produce a multi-line "single line" headline.
    let first_line = raw.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let trimmed = first_line.trim();

    // Trim to ≤ 80 chars at a UTF-8 char boundary. Split-on-char so
    // we don't truncate mid-codepoint.
    let truncated: String = trimmed.chars().take(80).collect();
    let cleaned = truncated.trim_end_matches(|c: char| {
        c == '.' || c == '!' || c == '?' || c == ';' || c == ',' || c == ':'
    });
    if cleaned.is_empty() {
        "(empty session)".to_string()
    } else {
        cleaned.to_string()
    }
}

fn is_slash_command(content: &str) -> bool {
    content.trim_start().starts_with('/')
}

fn derive_bullets(steps: &[StepRow], status: &str) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();
    for s in steps {
        *counts.entry(s.tool_name.clone()).or_insert(0) += 1;
    }

    let mut bullets: Vec<String> = counts
        .into_iter()
        .map(|(tool, count)| {
            if count > 1 {
                format!("Ran `{tool}` ({count}×)")
            } else {
                format!("Ran `{tool}`")
            }
        })
        .collect();

    // Failure tail: surface what stopped the agent. Tests pin that the
    // bullet starts with "Stopped:" so UIs can render it differently.
    if status == "failed" {
        let last_failed_step = steps.iter().rev().find(|s| !s.success);
        let reason = last_failed_step
            .map(|s| {
                let snippet = s.output.lines().next().unwrap_or("").trim();
                if snippet.is_empty() {
                    format!("`{}` failed", s.tool_name)
                } else {
                    format!("`{}` — {}", s.tool_name, truncate_chars(snippet, 100))
                }
            })
            .unwrap_or_else(|| "agent ended in failure".to_string());
        bullets.push(format!("Stopped: {reason}"));
    }

    // Cap at 7 to honor the README's "3–7 bullets" contract; if we
    // have fewer than 3, that's fine — the contract says "3–7 short
    // bullets, what happened" but the heuristic doesn't fabricate.
    if bullets.len() > 7 {
        bullets.truncate(7);
    }
    bullets
}

fn derive_next_actions(messages: &[MessageRow]) -> Vec<String> {
    // Look at the last 3 assistant messages for imperative-form
    // sentences. The cap-at-3 contract is enforced after collection.
    let assistants: Vec<&MessageRow> = messages
        .iter()
        .rev()
        .filter(|m| m.role == "assistant")
        .take(3)
        .collect();

    let mut actions: Vec<String> = Vec::new();
    for m in assistants.iter().rev() {
        for sentence in split_sentences(&m.content) {
            if let Some(action) = parse_imperative(&sentence) {
                if !actions.iter().any(|a| a.eq_ignore_ascii_case(&action)) {
                    actions.push(action);
                    if actions.len() == 3 {
                        return actions;
                    }
                }
            }
        }
    }
    actions
}

/// Split text into sentence-ish units. Naive but deterministic — we
/// split on `.`, `!`, `?` followed by whitespace, plus newlines.
/// Doesn't handle abbreviations (Mr. Mrs. etc.) but the heuristic
/// recap tolerates noise; the LLM path does the real prose work.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut prev_was_terminator = false;
    for c in text.chars() {
        if c == '\n' {
            if !current.trim().is_empty() {
                out.push(current.trim().to_string());
            }
            current.clear();
            prev_was_terminator = false;
            continue;
        }
        current.push(c);
        if c == '.' || c == '!' || c == '?' {
            prev_was_terminator = true;
        } else if prev_was_terminator && c.is_whitespace() {
            if !current.trim().is_empty() {
                out.push(current.trim().to_string());
            }
            current.clear();
            prev_was_terminator = false;
        } else if !c.is_whitespace() {
            prev_was_terminator = false;
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// Pull an imperative-form action out of a sentence if one is
/// recognizable. Returns the action stem (without leading
/// "Next, " / "TODO: " / "Should also ") so UI can present it
/// uniformly. Returns `None` for non-imperative sentences.
fn parse_imperative(sentence: &str) -> Option<String> {
    let s = sentence.trim();
    if s.is_empty() {
        return None;
    }

    // Bullet-list-style markers from chat output.
    let lower = s.to_lowercase();
    let prefixes: &[&str] = &[
        "next,",
        "next:",
        "todo:",
        "todo -",
        "to do:",
        "should also",
        "should",
        "we should",
        "you should",
        "let's",
        "let's also",
    ];
    for p in prefixes {
        if let Some(rest) = lower.strip_prefix(p) {
            // Map back to the original case at the same byte offset.
            let cut = s.len() - rest.len();
            let raw = s[cut..].trim_start_matches(|c: char| {
                c == ' ' || c == ',' || c == ':' || c == '-'
            });
            let trimmed = raw.trim_end_matches(|c: char| {
                c == '.' || c == '!' || c == '?' || c == ';' || c == ','
            });
            if !trimmed.is_empty() {
                return Some(capitalize_first(trimmed));
            }
        }
    }

    None
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

fn derive_artifacts(steps: &[StepRow]) -> Vec<RecapArtifact> {
    // Collect file-shaped paths from `input_summary`. Steps that look
    // like file ops contribute one artifact each; we dedupe on locator.
    let mut seen: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for s in steps {
        for path in extract_file_paths(&s.input_summary) {
            if seen.insert(path.clone()) {
                let label = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path)
                    .to_string();
                out.push(RecapArtifact {
                    kind: ArtifactKind::File,
                    label,
                    locator: path,
                });
            }
        }
    }
    out
}

/// Extract path-like tokens from a free-form input summary. Conservative:
/// we only accept tokens that contain at least one `/` or `\` and a
/// `.` (extension), and that are bounded by whitespace or quotes.
/// Handles quoted paths with spaces (e.g. `"src/foo bar.rs"`).
fn extract_file_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    for c in text.chars() {
        match (in_quote, c) {
            (None, '"') | (None, '\'') => in_quote = Some(c),
            (Some(q), c) if c == q => {
                if looks_like_path(&current) {
                    out.push(current.clone());
                }
                current.clear();
                in_quote = None;
            }
            (Some(_), c) => current.push(c),
            (None, c) if c.is_whitespace() || c == ',' || c == ')' => {
                if looks_like_path(&current) {
                    out.push(current.clone());
                }
                current.clear();
            }
            (None, c) => current.push(c),
        }
    }
    if looks_like_path(&current) {
        out.push(current);
    }
    out
}

fn looks_like_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let has_sep = s.contains('/') || s.contains('\\');
    let has_dot = s.contains('.');
    has_sep && has_dot && !s.starts_with("http")
}

// ── F1.4: REPL surface helpers ──────────────────────────────────────────────

/// Render a recap for stdout — headline first, then bullets, then a
/// "Next:" block if `next_actions` is non-empty. Pure, deterministic;
/// the REPL command and any future end-of-agent auto-print site share
/// this so the user sees the same shape everywhere.
///
/// No ANSI / colors here — that's the REPL frontend's responsibility,
/// and many wrappers (TUI, log capture) handle bare ASCII better.
pub fn format_recap(recap: &Recap) -> String {
    let mut out = String::new();
    out.push_str(&format!("Recap: {}\n", recap.headline));
    if !recap.bullets.is_empty() {
        out.push('\n');
        for b in &recap.bullets {
            out.push_str(&format!("  • {b}\n"));
        }
    }
    if !recap.next_actions.is_empty() {
        out.push_str("\nNext:\n");
        for a in &recap.next_actions {
            out.push_str(&format!("  → {a}\n"));
        }
    }
    if !recap.artifacts.is_empty() {
        out.push_str("\nFiles:\n");
        for a in &recap.artifacts {
            out.push_str(&format!("  • {} ({})\n", a.label, a.locator));
        }
    }
    let gen_label = match &recap.generator {
        RecapGenerator::Heuristic => "heuristic".to_string(),
        RecapGenerator::Llm { provider, model } => format!("llm: {provider}/{model}"),
        RecapGenerator::UserEdited => "user-edited".to_string(),
    };
    out.push_str(&format!(
        "\n[generator: {gen_label}, id: {}]\n",
        &recap.id[..recap.id.len().min(8)]
    ));
    out
}

/// Load the most-recent stored recap for `subject_id`, or generate a
/// fresh heuristic one and persist it via the F1.1 idempotency rule.
/// The caller gets back the recap they should display — never `None`
/// so REPL handlers don't have to branch.
///
/// Returns `Ok(None)` when the subject session itself is missing from
/// the store; the REPL handler maps that to a "session not found"
/// stderr line.
pub fn load_or_generate_session_recap(
    store: &crate::session_store::SessionStore,
    subject_id: &str,
) -> anyhow::Result<Option<Recap>> {
    // Fast path: most recent stored recap wins. F1.1's
    // list_recaps_for_subject sorts newest-first; limit=1 gives us the
    // freshest row in one query.
    if let Some(top) = store
        .list_recaps_for_subject(subject_id, 1)?
        .into_iter()
        .next()
    {
        // If the session has new messages since this recap was made
        // (last_message_id has advanced), regenerate so the REPL shows
        // up-to-date content. F1.1's idempotency rule keeps the
        // pre-existing row when nothing changed; otherwise the new
        // (subject, last_msg) pair gets a fresh row with a new id.
        let detail = match store.get_session_detail(subject_id)? {
            Some(d) => d,
            None => return Ok(None),
        };
        let current_last_msg = detail.messages.last().map(|m| m.id);
        if top.last_message_id == current_last_msg {
            return Ok(Some(top));
        }
        let fresh = heuristic_recap(&detail);
        return Ok(Some(store.insert_recap(&fresh)?));
    }

    // No prior recap — generate, store, return.
    let detail = match store.get_session_detail(subject_id)? {
        Some(d) => d,
        None => return Ok(None),
    };
    let recap = heuristic_recap(&detail);
    Ok(Some(store.insert_recap(&recap)?))
}

// ── J1.2: Job-kind heuristic generator ──────────────────────────────────────
//
// Operates on `JobRecord` + replayed `(seq, AgentEventPayload)` events. The
// shared `Recap` shape carries the result so the recaps table (J1.1), HTTP
// surface (J1.3+), and UI consumers can reuse the same wire format used for
// session recaps.
//
// Cursor convention: `last_message_id` carries the last event seq for jobs
// (the storage layer in `JobsDb` reads it as `last_event_seq`). This mirrors
// the session pattern where the same field carries the last message id.

/// Generate a Job recap from the durable record + the full event replay.
/// Pure, deterministic, no I/O. The caller is responsible for fetching
/// the events (e.g. via `JobsDb::list_events_since(sid, 0)`).
///
/// Algorithm (mirrors the session heuristic, adapted for job events):
/// 1. Headline: first non-empty line of `job.task`, ≤ 80 chars,
///    trailing punctuation stripped.
/// 2. Bullets: count of `step` events grouped by tool_name (sorted), plus
///    a `Stopped: …` bullet when the job ended in failure or
///    cancellation.
/// 3. next_actions: imperative sentences pulled from the `complete`
///    event's content (or `job.summary`), capped at 3.
/// 4. artifacts: file-shaped paths inferred from step tool names and
///    chunk events (light heuristic — the LLM path produces real
///    artifact summaries).
/// 5. resume_hint: `target=Job{id}`, `from_message=None`,
///    `seed_instruction = next_actions[0]` if any,
///    `branch_on_resume = false`.
pub fn heuristic_job_recap(
    job: &crate::job_manager::JobRecord,
    events: &[(u64, crate::job_manager::AgentEventPayload)],
) -> Recap {
    let headline = derive_job_headline(&job.task);
    let bullets = derive_job_bullets(events, &job.status, job.cancellation_reason.as_deref());
    let next_actions = derive_job_next_actions(events, job.summary.as_deref());
    let artifacts = derive_job_artifacts(events);

    let last_event_seq: Option<i64> = events.last().map(|(s, _)| *s as i64);
    let seed_instruction = next_actions.first().cloned();

    let resume_hint = Some(ResumeHint {
        target: ResumeTarget::Job {
            id: job.session_id.clone(),
        },
        from_message: None,
        from_step: None,
        from_diff_index: None,
        seed_instruction,
        branch_on_resume: false,
    });

    Recap {
        id: new_recap_id(),
        kind: RecapKind::Job,
        subject_id: job.session_id.clone(),
        last_message_id: last_event_seq,
        workspace: None,
        generated_at: Utc::now(),
        generator: RecapGenerator::Heuristic,
        headline,
        bullets,
        next_actions,
        artifacts,
        resume_hint,
        token_usage: None,
        schema_version: 1,
    }
}

fn derive_job_headline(task: &str) -> String {
    let first_line = task.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let trimmed = first_line.trim();
    let truncated: String = trimmed.chars().take(80).collect();
    let cleaned = truncated.trim_end_matches(|c: char| {
        c == '.' || c == '!' || c == '?' || c == ';' || c == ',' || c == ':'
    });
    if cleaned.is_empty() {
        "(empty job)".to_string()
    } else {
        cleaned.to_string()
    }
}

fn derive_job_bullets(
    events: &[(u64, crate::job_manager::AgentEventPayload)],
    status: &str,
    cancellation_reason: Option<&str>,
) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();
    for (_, ev) in events {
        if ev.kind == "step" {
            if let Some(tool) = &ev.tool_name {
                *counts.entry(tool.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut bullets: Vec<String> = counts
        .into_iter()
        .map(|(tool, count)| {
            if count > 1 {
                format!("Ran `{tool}` ({count}×)")
            } else {
                format!("Ran `{tool}`")
            }
        })
        .collect();

    match status {
        "failed" => {
            // Surface the last error event content if any, else the
            // last failed step's tool name.
            let last_error = events
                .iter()
                .rev()
                .find(|(_, e)| e.kind == "error")
                .and_then(|(_, e)| e.content.clone());
            let last_failed_step = events
                .iter()
                .rev()
                .find(|(_, e)| e.kind == "step" && e.success == Some(false))
                .and_then(|(_, e)| e.tool_name.clone());
            let reason = match (last_error, last_failed_step) {
                (Some(msg), _) => truncate_chars(msg.lines().next().unwrap_or(&msg).trim(), 100),
                (None, Some(tool)) => format!("`{tool}` failed"),
                (None, None) => "agent ended in failure".to_string(),
            };
            bullets.push(format!("Stopped: {reason}"));
        }
        "cancelled" => {
            let reason = cancellation_reason
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| truncate_chars(s, 100))
                .unwrap_or_else(|| "user cancelled".to_string());
            bullets.push(format!("Stopped: {reason}"));
        }
        _ => {}
    }

    if bullets.len() > 7 {
        bullets.truncate(7);
    }
    bullets
}

fn derive_job_next_actions(
    events: &[(u64, crate::job_manager::AgentEventPayload)],
    summary_fallback: Option<&str>,
) -> Vec<String> {
    // Prefer the final `complete` event's content (carries the
    // agent's summary). Fall back to `job.summary` if no complete
    // event was emitted (e.g. failed before completion).
    let source: Option<String> = events
        .iter()
        .rev()
        .find(|(_, e)| e.kind == "complete")
        .and_then(|(_, e)| e.content.clone())
        .or_else(|| summary_fallback.map(|s| s.to_string()));

    let text = match source {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut actions: Vec<String> = Vec::new();
    for sentence in split_sentences(&text) {
        if let Some(action) = parse_imperative(&sentence) {
            if !actions.iter().any(|a| a.eq_ignore_ascii_case(&action)) {
                actions.push(action);
                if actions.len() == 3 {
                    return actions;
                }
            }
        }
    }
    actions
}

fn derive_job_artifacts(
    events: &[(u64, crate::job_manager::AgentEventPayload)],
) -> Vec<RecapArtifact> {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (_, e) in events {
        if let Some(content) = &e.content {
            for path in extract_file_paths(content) {
                if seen.insert(path.clone()) {
                    let label = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                        .to_string();
                    out.push(RecapArtifact {
                        kind: ArtifactKind::File,
                        label,
                        locator: path,
                    });
                }
            }
        }
    }
    out
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_store::SessionRow;

    fn fixture_session(id: &str, status: &str) -> SessionRow {
        SessionRow {
            id: id.to_string(),
            task: "test".to_string(),
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            started_at: 0,
            finished_at: None,
            status: status.to_string(),
            summary: None,
            step_count: 0,
            parent_session_id: None,
            depth: 0,
            project_path: None,
        }
    }

    fn msg(id: i64, role: &str, content: &str) -> MessageRow {
        MessageRow {
            id,
            session_id: "S".to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: 0,
        }
    }

    fn step(num: i64, tool: &str, input: &str, output: &str, ok: bool) -> StepRow {
        StepRow {
            id: num,
            session_id: "S".to_string(),
            step_num: num,
            tool_name: tool.to_string(),
            input_summary: input.to_string(),
            output: output.to_string(),
            success: ok,
            created_at: 0,
        }
    }

    fn detail(
        session_id: &str,
        status: &str,
        messages: Vec<MessageRow>,
        steps: Vec<StepRow>,
    ) -> SessionDetail {
        SessionDetail {
            session: fixture_session(session_id, status),
            messages,
            steps,
        }
    }

    // ── Headline ─────────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_extracts_headline_from_first_user_message() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "system", "you are a helpful assistant"),
                msg(2, "user", "Refactor the auth middleware"),
                msg(3, "assistant", "Sure, I'll start by reading auth.rs"),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert_eq!(r.headline, "Refactor the auth middleware");
    }

    #[test]
    fn heuristic_recap_truncates_long_headline_to_80_chars() {
        // 90-char message should truncate at 80 chars (no trailing
        // punctuation in this fixture so the cap is the binding rule).
        let long_msg = "a".repeat(90);
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", &long_msg)],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert_eq!(
            r.headline.chars().count(),
            80,
            "headline must be exactly 80 chars after truncation"
        );
    }

    #[test]
    fn heuristic_recap_skips_slash_commands_for_headline() {
        // `/command` style messages aren't prose — they shouldn't
        // become the headline. The next prose user message wins.
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "/init"),
                msg(2, "user", "Implement the resume flow"),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert_eq!(r.headline, "Implement the resume flow");
    }

    #[test]
    fn heuristic_recap_strips_trailing_punctuation_from_headline() {
        // UIs append their own ellipsis / dot / etc. — we hand them
        // a clean stem so they don't end up doubled.
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "Fix the auth bug.")],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert_eq!(r.headline, "Fix the auth bug");
    }

    #[test]
    fn heuristic_recap_handles_empty_session_gracefully() {
        let d = detail("S", "complete", vec![], vec![]);
        let r = heuristic_recap(&d);
        assert_eq!(r.headline, "(empty session)");
        assert!(r.bullets.is_empty());
        assert!(r.next_actions.is_empty());
        assert!(r.artifacts.is_empty());
    }

    #[test]
    fn heuristic_recap_takes_first_line_for_multiline_headline() {
        // A multiline first user message must produce a single-line
        // headline. The contract is "single line"; subsequent lines
        // belong in the conversation, not the recap header.
        let d = detail(
            "S",
            "complete",
            vec![msg(
                1,
                "user",
                "Refactor the auth middleware\nspecifically validate_jwt",
            )],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert_eq!(r.headline, "Refactor the auth middleware");
    }

    // ── Bullets ──────────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_groups_tool_calls_by_name_with_count() {
        // Multiple invocations of the same tool collapse to a single
        // bullet with the count. Single invocations omit the count
        // suffix entirely so the bullet reads naturally.
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "test")],
            vec![
                step(1, "bash", "cargo test", "ok", true),
                step(2, "bash", "cargo test", "ok", true),
                step(3, "bash", "cargo test", "ok", true),
                step(4, "read_file", "src/lib.rs", "...", true),
            ],
        );
        let r = heuristic_recap(&d);
        assert!(
            r.bullets.iter().any(|b| b == "Ran `bash` (3×)"),
            "expected bash with count; got: {:?}",
            r.bullets
        );
        assert!(
            r.bullets.iter().any(|b| b == "Ran `read_file`"),
            "expected single read_file without count; got: {:?}",
            r.bullets
        );
    }

    #[test]
    fn heuristic_recap_includes_failure_bullet_when_session_failed() {
        // The audit pins that a failed session produces a "Stopped:"
        // bullet so UIs can render it visually distinct from
        // success-path tool calls.
        let d = detail(
            "S",
            "failed",
            vec![msg(1, "user", "test")],
            vec![
                step(1, "bash", "cargo build", "ok", true),
                step(2, "bash", "cargo test", "test failed: 3 errors", false),
            ],
        );
        let r = heuristic_recap(&d);
        assert!(
            r.bullets.iter().any(|b| b.starts_with("Stopped: ")),
            "expected Stopped: bullet on failed session; got: {:?}",
            r.bullets
        );
    }

    #[test]
    fn heuristic_recap_no_failure_bullet_for_successful_session() {
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "test")],
            vec![step(1, "bash", "cargo test", "ok", true)],
        );
        let r = heuristic_recap(&d);
        assert!(
            !r.bullets.iter().any(|b| b.starts_with("Stopped: ")),
            "no Stopped: bullet on successful session; got: {:?}",
            r.bullets
        );
    }

    // ── next_actions ─────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_extracts_next_actions_from_imperatives() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "test"),
                msg(
                    2,
                    "assistant",
                    "Done with auth.rs. Next, wire refresh-token rotation. TODO: open a PR.",
                ),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        // Implementation capitalizes the first letter for nicer UI
        // rendering; assertions are case-insensitive to pin the
        // *content* contract without locking in capitalization.
        assert!(
            r.next_actions
                .iter()
                .any(|a| a.to_lowercase().contains("wire refresh-token rotation")),
            "expected 'wire refresh-token rotation' in next_actions; got: {:?}",
            r.next_actions
        );
        assert!(
            r.next_actions.iter().any(|a| a.to_lowercase().contains("open a pr")),
            "expected 'open a PR' in next_actions; got: {:?}",
            r.next_actions
        );
    }

    #[test]
    fn heuristic_recap_caps_next_actions_at_three() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "test"),
                msg(
                    2,
                    "assistant",
                    "Next, A. Next, B. Next, C. Next, D. Next, E.",
                ),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        assert!(
            r.next_actions.len() <= 3,
            "next_actions must be capped at 3; got: {:?}",
            r.next_actions
        );
    }

    #[test]
    fn heuristic_recap_dedupes_repeated_next_actions() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "test"),
                msg(
                    2,
                    "assistant",
                    "Next, write tests. TODO: write tests. Should also write tests.",
                ),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        let count = r
            .next_actions
            .iter()
            .filter(|a| a.eq_ignore_ascii_case("Write tests"))
            .count();
        assert!(count <= 1, "duplicates not deduped; got: {:?}", r.next_actions);
    }

    // ── artifacts ────────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_collects_unique_file_artifacts_from_steps() {
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "test")],
            vec![
                step(1, "read_file", "src/auth.rs", "...", true),
                // Same path again — dedupe.
                step(2, "write_file", "src/auth.rs", "...", true),
                step(3, "read_file", "src/lib.rs", "...", true),
                // Non-path tokens — must NOT become artifacts.
                step(4, "bash", "cargo test", "ok", true),
            ],
        );
        let r = heuristic_recap(&d);
        let locators: Vec<&str> = r.artifacts.iter().map(|a| a.locator.as_str()).collect();
        assert!(
            locators.contains(&"src/auth.rs"),
            "expected src/auth.rs in artifacts; got: {locators:?}"
        );
        assert!(
            locators.contains(&"src/lib.rs"),
            "expected src/lib.rs in artifacts; got: {locators:?}"
        );
        assert_eq!(
            locators.iter().filter(|p| **p == "src/auth.rs").count(),
            1,
            "duplicate paths must be deduped; got: {locators:?}"
        );
    }

    #[test]
    fn heuristic_recap_artifact_label_is_basename() {
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "test")],
            vec![step(1, "read_file", "vibecli/src/recap.rs", "...", true)],
        );
        let r = heuristic_recap(&d);
        assert_eq!(r.artifacts.len(), 1);
        assert_eq!(r.artifacts[0].label, "recap.rs");
        assert_eq!(r.artifacts[0].locator, "vibecli/src/recap.rs");
    }

    #[test]
    fn heuristic_recap_extract_paths_skips_urls() {
        // URLs contain `/` and `.` but are not file artifacts. The
        // extractor must reject them so a recap of a session that
        // mentioned a URL doesn't fabricate file paths.
        let paths = extract_file_paths("see https://example.com/foo.html for details");
        assert!(
            paths.is_empty(),
            "extract_file_paths must skip URLs; got: {paths:?}"
        );
    }

    // ── resume_hint ──────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_resume_hint_points_at_last_message_id() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "test"),
                msg(2, "assistant", "ack"),
                msg(42, "user", "follow up"),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        let hint = r.resume_hint.expect("resume_hint must be populated");
        assert_eq!(hint.from_message, Some(42));
        assert!(matches!(hint.target, ResumeTarget::Session { ref id } if id == "S"));
    }

    #[test]
    fn heuristic_recap_resume_hint_seeds_with_first_next_action() {
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "test"),
                msg(2, "assistant", "Next, wire refresh tokens."),
            ],
            vec![],
        );
        let r = heuristic_recap(&d);
        let hint = r.resume_hint.expect("resume_hint must be populated");
        assert!(
            hint.seed_instruction
                .as_deref()
                .map(|s| s.to_lowercase().contains("wire refresh tokens"))
                .unwrap_or(false),
            "seed_instruction should mirror first next_action; got: {:?}",
            hint.seed_instruction
        );
    }

    #[test]
    fn heuristic_recap_resume_hint_branch_false_for_tail_resume() {
        let d = detail(
            "S",
            "complete",
            vec![msg(1, "user", "test"), msg(2, "assistant", "ack")],
            vec![],
        );
        let r = heuristic_recap(&d);
        let hint = r.resume_hint.expect("resume_hint must be populated");
        assert!(
            !hint.branch_on_resume,
            "tail-resume must not branch; got branch_on_resume = true"
        );
    }

    // ── shape ────────────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_marks_generator_as_heuristic() {
        let d = detail("S", "complete", vec![msg(1, "user", "test")], vec![]);
        let r = heuristic_recap(&d);
        assert!(
            matches!(r.generator, RecapGenerator::Heuristic),
            "heuristic path must mark generator = Heuristic; got: {:?}",
            r.generator
        );
    }

    #[test]
    fn heuristic_recap_starts_at_schema_version_one() {
        let d = detail("S", "complete", vec![msg(1, "user", "test")], vec![]);
        let r = heuristic_recap(&d);
        assert_eq!(r.schema_version, 1);
    }

    #[test]
    fn heuristic_recap_kind_is_session() {
        let d = detail("S", "complete", vec![msg(1, "user", "test")], vec![]);
        let r = heuristic_recap(&d);
        assert!(matches!(r.kind, RecapKind::Session));
    }

    // ── perf ─────────────────────────────────────────────────────────────

    #[test]
    fn heuristic_recap_runs_in_well_under_one_second_on_large_session() {
        // The design pins "<50ms on 200-message session" but unit
        // tests run on shared CI hardware so we use a generous 1s
        // ceiling here. Local dev typically lands under 5ms.
        let mut messages = vec![msg(1, "user", "Implement feature X")];
        for i in 2..=200 {
            messages.push(msg(
                i,
                if i % 2 == 0 { "assistant" } else { "user" },
                "lorem ipsum dolor sit amet, consectetur adipiscing elit",
            ));
        }
        let mut steps = Vec::new();
        for i in 1..=200 {
            steps.push(step(
                i,
                if i % 3 == 0 { "bash" } else { "read_file" },
                "src/lib.rs",
                "ok",
                true,
            ));
        }
        let d = detail("S", "complete", messages, steps);
        let start = std::time::Instant::now();
        let _ = heuristic_recap(&d);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 1000,
            "heuristic took {elapsed:?} on 200-message session; expected <1s"
        );
    }

    // ── serde round-trip ─────────────────────────────────────────────────

    #[test]
    fn recap_round_trips_through_json() {
        // Wire shape must survive serde so HTTP routes (F1.2) and
        // JSONL telemetry (future) can rely on a stable round-trip.
        let d = detail(
            "S",
            "complete",
            vec![
                msg(1, "user", "Refactor auth"),
                msg(2, "assistant", "Next, write tests."),
            ],
            vec![step(1, "read_file", "src/auth.rs", "...", true)],
        );
        let r = heuristic_recap(&d);
        let json = serde_json::to_string(&r).expect("serialize");
        let back: Recap = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn recap_kind_serializes_as_snake_case() {
        // Wire format pinning: `RecapKind::DiffChain` must serialize
        // as `"diff_chain"`, not `"DiffChain"`. Mobile/watch clients
        // depend on lowercase tags.
        let s = serde_json::to_string(&RecapKind::DiffChain).unwrap();
        assert_eq!(s, "\"diff_chain\"");
    }

    // ── F1.4: format_recap + load_or_generate ───────────────────────────

    fn fixture_recap_for_render() -> Recap {
        Recap {
            id: "abcd1234deadbeefdeadbeefdeadbeef".to_string(),
            kind: RecapKind::Session,
            subject_id: "S".to_string(),
            last_message_id: Some(7),
            workspace: None,
            generated_at: chrono::Utc::now(),
            generator: RecapGenerator::Heuristic,
            headline: "Refactor the auth middleware".to_string(),
            bullets: vec![
                "Pulled validate_jwt into guards/jwt.rs".to_string(),
                "Updated 3 call sites; tests green".to_string(),
            ],
            next_actions: vec!["Wire refresh-token rotation".to_string()],
            artifacts: vec![RecapArtifact {
                kind: ArtifactKind::File,
                label: "auth.rs".to_string(),
                locator: "src/auth.rs".to_string(),
            }],
            resume_hint: None,
            token_usage: None,
            schema_version: 1,
        }
    }

    #[test]
    fn format_recap_starts_with_headline_label() {
        // Pin: stdout output begins with `Recap: <headline>` so users
        // can scan terminal output for the keyword. The REPL command
        // and any future auto-print site share this prefix.
        let r = fixture_recap_for_render();
        let out = format_recap(&r);
        assert!(
            out.starts_with("Recap: Refactor the auth middleware\n"),
            "format_recap must lead with `Recap: <headline>`; got: {out:?}"
        );
    }

    #[test]
    fn format_recap_renders_bullets_with_dot_marker() {
        let r = fixture_recap_for_render();
        let out = format_recap(&r);
        assert!(
            out.contains("  • Pulled validate_jwt into guards/jwt.rs"),
            "expected bullet marker `  • <bullet>`; got: {out}"
        );
        assert!(
            out.contains("  • Updated 3 call sites; tests green"),
            "second bullet missing; got: {out}"
        );
    }

    #[test]
    fn format_recap_renders_next_actions_with_arrow_marker() {
        let r = fixture_recap_for_render();
        let out = format_recap(&r);
        assert!(
            out.contains("Next:") && out.contains("  → Wire refresh-token rotation"),
            "expected `Next:` block + `  → <action>`; got: {out}"
        );
    }

    #[test]
    fn format_recap_renders_artifacts_with_label_and_locator() {
        let r = fixture_recap_for_render();
        let out = format_recap(&r);
        assert!(
            out.contains("Files:") && out.contains("  • auth.rs (src/auth.rs)"),
            "expected `Files:` block with `<label> (<locator>)`; got: {out}"
        );
    }

    #[test]
    fn format_recap_omits_empty_optional_blocks() {
        // A recap with no bullets / next_actions / artifacts should
        // render headline + generator footer only — no stray empty
        // section labels.
        let mut r = fixture_recap_for_render();
        r.bullets.clear();
        r.next_actions.clear();
        r.artifacts.clear();
        let out = format_recap(&r);
        assert!(out.starts_with("Recap: "));
        assert!(!out.contains("Next:"), "Next: must hide when empty");
        assert!(!out.contains("Files:"), "Files: must hide when empty");
        // Bullets render under no header — so an empty bullets block
        // means the only `•` markers should be from artifacts (also
        // empty), so no `•` should appear at all.
        assert!(!out.contains("  •"), "no bullet markers when empty: {out}");
    }

    #[test]
    fn format_recap_includes_truncated_id_in_footer() {
        // Pin: footer shows the first 8 chars of the id so users can
        // copy-paste it into `/recap <prefix>` without typing 32 chars.
        let r = fixture_recap_for_render();
        let out = format_recap(&r);
        assert!(
            out.contains("[generator: heuristic, id: abcd1234]"),
            "footer must show generator + 8-char id prefix; got: {out}"
        );
    }

    #[test]
    fn format_recap_distinguishes_user_edited_generator() {
        // UI cue for human-edited recaps so they don't look
        // machine-generated. Pin the wire label.
        let mut r = fixture_recap_for_render();
        r.generator = RecapGenerator::UserEdited;
        let out = format_recap(&r);
        assert!(
            out.contains("[generator: user-edited"),
            "user-edited footer label drift; got: {out}"
        );
    }

    #[test]
    fn format_recap_distinguishes_llm_generator_with_provider_and_model() {
        let mut r = fixture_recap_for_render();
        r.generator = RecapGenerator::Llm {
            provider: "anthropic".to_string(),
            model: "claude-opus-4-7".to_string(),
        };
        let out = format_recap(&r);
        assert!(
            out.contains("[generator: llm: anthropic/claude-opus-4-7"),
            "llm footer must include provider/model; got: {out}"
        );
    }

    // ── load_or_generate_session_recap ──────────────────────────────────

    fn load_fixture() -> (
        crate::session_store::SessionStore,
        String,
        tempfile::TempDir,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::session_store::SessionStore::open(
            dir.path().join("sessions.db"),
        )
        .unwrap();
        let sid = "F14-load-test".to_string();
        store
            .insert_session_with_parent(
                &sid,
                "Refactor the auth middleware",
                "mock",
                "test-model",
                None,
                0,
            )
            .unwrap();
        store
            .insert_message(&sid, "user", "Refactor the auth middleware")
            .unwrap();
        (store, sid, dir)
    }

    #[test]
    fn load_or_generate_returns_none_for_missing_session() {
        let (store, _sid, _dir) = load_fixture();
        let out =
            load_or_generate_session_recap(&store, "ghost-session").unwrap();
        assert!(
            out.is_none(),
            "missing session must yield None so REPL can 404 cleanly"
        );
    }

    #[test]
    fn load_or_generate_creates_recap_on_first_call() {
        // First call has no prior recap → generates + stores. The
        // store must end up with exactly one row for this subject.
        let (store, sid, _dir) = load_fixture();
        let r = load_or_generate_session_recap(&store, &sid)
            .unwrap()
            .expect("session exists, recap must be Some");
        assert_eq!(r.subject_id, sid);
        let list = store.list_recaps_for_subject(&sid, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, r.id);
    }

    #[test]
    fn load_or_generate_reuses_recap_when_session_unchanged() {
        // Second call without new messages must return the same id —
        // the F1.1 idempotency rule plus our list-newest-first lookup
        // means we never generate a duplicate row.
        let (store, sid, _dir) = load_fixture();
        let r1 = load_or_generate_session_recap(&store, &sid).unwrap().unwrap();
        let r2 = load_or_generate_session_recap(&store, &sid).unwrap().unwrap();
        assert_eq!(
            r1.id, r2.id,
            "no new messages → must reuse the prior recap, not duplicate"
        );
    }

    #[test]
    fn load_or_generate_regenerates_when_new_messages_exist() {
        // Add a message after the first recap → load_or_generate
        // must produce a fresh row keyed off the new last_message_id.
        // This is the "REPL shows up-to-date content" contract: a
        // user who runs `/recap` after sending more chat must see the
        // new state, not a stale snapshot.
        let (store, sid, _dir) = load_fixture();
        let r1 = load_or_generate_session_recap(&store, &sid).unwrap().unwrap();
        store.insert_message(&sid, "assistant", "ack").unwrap();
        let r2 = load_or_generate_session_recap(&store, &sid).unwrap().unwrap();
        assert_ne!(
            r1.id, r2.id,
            "new last_message_id must yield a new recap row"
        );
        assert_ne!(r1.last_message_id, r2.last_message_id);
    }

    // ── J1.2: Job-kind heuristic generator ─────────────────────────────

    use crate::job_manager::{AgentEventPayload, JobRecord};

    fn fixture_job(sid: &str, task: &str, status: &str) -> JobRecord {
        JobRecord {
            session_id: sid.to_string(),
            task: task.to_string(),
            status: status.to_string(),
            provider: "mock".to_string(),
            started_at: 0,
            finished_at: Some(1),
            summary: None,
            priority: 5,
            queued_at: 0,
            webhook_url: None,
            tags: vec![],
            cancellation_reason: None,
            steps_completed: 0,
            tokens_used: 0,
            cost_cents: 0,
        }
    }

    #[test]
    fn job_recap_extracts_headline_from_task() {
        let job = fixture_job("j1", "Refactor the broker SSRF guard.", "complete");
        let r = heuristic_job_recap(&job, &[]);
        assert_eq!(r.headline, "Refactor the broker SSRF guard");
        assert_eq!(r.kind, RecapKind::Job);
        assert_eq!(r.subject_id, "j1");
    }

    #[test]
    fn job_recap_truncates_long_task_to_80_chars() {
        let task = "a".repeat(120);
        let job = fixture_job("j2", &task, "complete");
        let r = heuristic_job_recap(&job, &[]);
        assert_eq!(r.headline.chars().count(), 80);
    }

    #[test]
    fn job_recap_takes_first_line_for_multiline_task() {
        let job = fixture_job("j3", "First line\nSecond line\nThird", "complete");
        let r = heuristic_job_recap(&job, &[]);
        assert_eq!(r.headline, "First line");
    }

    #[test]
    fn job_recap_handles_empty_task_gracefully() {
        let job = fixture_job("j4", "   \n  ", "complete");
        let r = heuristic_job_recap(&job, &[]);
        assert_eq!(r.headline, "(empty job)");
    }

    #[test]
    fn job_recap_groups_step_events_by_tool_with_count() {
        let job = fixture_job("j5", "Do work", "complete");
        let events = vec![
            (1u64, AgentEventPayload::step(1, "shell", true)),
            (2, AgentEventPayload::step(2, "shell", true)),
            (3, AgentEventPayload::step(3, "edit", true)),
            (4, AgentEventPayload::chunk("noise".into())), // ignored — not a step
        ];
        let r = heuristic_job_recap(&job, &events);
        // Sorted alphabetically (BTreeMap), so `edit` before `shell`.
        assert_eq!(r.bullets.len(), 2);
        assert_eq!(r.bullets[0], "Ran `edit`");
        assert_eq!(r.bullets[1], "Ran `shell` (2×)");
    }

    #[test]
    fn job_recap_includes_stopped_bullet_for_failure() {
        let job = fixture_job("j6", "Build", "failed");
        let events = vec![
            (1u64, AgentEventPayload::step(1, "shell", false)),
            (2, AgentEventPayload::error("compile error: missing semicolon".into())),
        ];
        let r = heuristic_job_recap(&job, &events);
        let stopped = r.bullets.iter().find(|b| b.starts_with("Stopped:"));
        let stopped = stopped.expect("failed job must produce a Stopped bullet");
        assert!(stopped.contains("compile error"), "got {stopped}");
    }

    #[test]
    fn job_recap_uses_cancellation_reason_when_cancelled() {
        let mut job = fixture_job("j7", "Long-running query", "cancelled");
        job.cancellation_reason = Some("user pressed Esc".to_string());
        let r = heuristic_job_recap(&job, &[]);
        let stopped = r
            .bullets
            .iter()
            .find(|b| b.starts_with("Stopped:"))
            .expect("cancelled job must produce a Stopped bullet");
        assert!(stopped.contains("user pressed Esc"));
    }

    #[test]
    fn job_recap_no_stopped_bullet_for_complete() {
        let job = fixture_job("j8", "OK", "complete");
        let events = vec![(1u64, AgentEventPayload::step(1, "shell", true))];
        let r = heuristic_job_recap(&job, &events);
        assert!(
            r.bullets.iter().all(|b| !b.starts_with("Stopped:")),
            "complete job must not have a Stopped bullet: {:?}",
            r.bullets
        );
    }

    #[test]
    fn job_recap_resume_hint_targets_job() {
        let job = fixture_job("j9", "Task", "complete");
        let r = heuristic_job_recap(&job, &[]);
        let hint = r.resume_hint.expect("must populate resume_hint");
        match hint.target {
            ResumeTarget::Job { id } => assert_eq!(id, "j9"),
            other => panic!("expected ResumeTarget::Job, got {other:?}"),
        }
        assert!(!hint.branch_on_resume);
        assert_eq!(hint.from_message, None);
    }

    #[test]
    fn job_recap_last_event_seq_tracks_final_event() {
        let job = fixture_job("j10", "Task", "complete");
        let events = vec![
            (5u64, AgentEventPayload::step(1, "shell", true)),
            (8, AgentEventPayload::complete("done".into())),
        ];
        let r = heuristic_job_recap(&job, &events);
        assert_eq!(r.last_message_id, Some(8));
    }

    #[test]
    fn job_recap_pulls_next_actions_from_complete_event() {
        let job = fixture_job("j11", "Task", "complete");
        let events = vec![
            (1u64, AgentEventPayload::step(1, "shell", true)),
            (
                2,
                AgentEventPayload::complete(
                    "Done. Next, run the test suite. Also update the changelog.".into(),
                ),
            ),
        ];
        let r = heuristic_job_recap(&job, &events);
        assert!(
            !r.next_actions.is_empty(),
            "expected at least one imperative action"
        );
    }

    #[test]
    fn job_recap_falls_back_to_summary_when_no_complete_event() {
        let mut job = fixture_job("j12", "Task", "failed");
        // Use a phrase the imperative parser recognises (matches "Next,").
        job.summary = Some("Next, run cargo test to verify the fix.".to_string());
        let r = heuristic_job_recap(&job, &[]);
        assert!(!r.next_actions.is_empty(), "summary should seed actions");
    }
}
