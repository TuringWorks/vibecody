#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! Slice G part 1 — confirmation prompters for the [`Tainted`] gate.
//!
//! DREAD #1 design doc §8.2 (CLI prompt) and the broader §8 scoping.
//! Slices A–F lined up the type, the propagation, the per-sink gates,
//! the boundary helpers, and the log-redaction formatters. This module
//! ships the **CLI confirmation flow** so the gate can actually mint
//! a real [`Confirmation`] when the user approves an action — replacing
//! the `RejectionReason::InteractiveStub` placeholder for terminal /
//! daemon-REPL contexts.
//!
//! ## Why a trait, not a free function
//!
//! Three constraints:
//!
//!   1. The gate runs inside `vibecli-cli` business logic; the
//!      prompter needs to be **injectable for tests** (we cannot
//!      block on real stdin in `cargo test`).
//!   2. There will be **multiple prompter implementations** as Slice G
//!      expands: the CLI stdin/stderr one shipped here, the VibeUI
//!      WebView modal one, the mobile/watch push-confirm one. All
//!      three need to satisfy the same contract.
//!   3. The gate function signatures stay shape-compatible with
//!      slices A–C — callers that pass `ConfirmMode::Headless` or
//!      `ConfirmMode::Interactive` continue to get the existing
//!      `Err(RejectionReason::*)` behaviour. The new entry-point is
//!      [`confirm_with_prompter`], which takes a `&mut dyn Prompter`.
//!
//! ## Threat-model invariants the prompt MUST satisfy
//!
//!   - The **payload bytes never appear on stderr.** The prompt
//!     surfaces the value's [`audit_summary`] (kind, provenance
//!     fields, 16-hex audit_id) but not the wrapped bytes. This is
//!     the same invariant the log-redaction slice F enforces — the
//!     prompt surface is a log surface from the threat-model point
//!     of view.
//!   - **`y` is the only approval input.** Any other input — `n`,
//!     empty line, EOF, blank, words — is treated as **deny**. Design
//!     §10 q1: errors-on-side-of-deny. A regression that accepted
//!     "yes" or "1" would have to deliberately re-tune the matcher.
//!   - On approval, [`Prompter::prompt`] returns `true` and the
//!     gate mints a fresh [`Confirmation`] with a random correlation
//!     id (the audit-log primary key for this consent).
//!
//! ## Wiring
//!
//! `tool_executor::ToolExecutor` calls [`confirm_with_prompter`]
//! from its `dispatch_bash_tool_call` / `dispatch_fetch_url_tool_call`
//! flows when the user is in an interactive terminal context.
//! Headless flag (already shipped under slice B) overrides — headless
//! never invokes the prompter and always rejects.

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::Mutex;

use crate::tainted::{Confirmation, Provenance, Reason, RejectionReason, Tainted};

// ── Prompter trait ─────────────────────────────────────────────────

/// Single-method trait the gate calls to obtain user consent.
///
/// Implementations are **not** required to be `Send + Sync` — the
/// trait object is held briefly inside a single async task and never
/// stored across `.await` boundaries. (The CLI implementation uses
/// `std::io::Stdin`, which is `!Sync` on some platforms.)
pub trait Prompter {
    /// Prompt the user about a pending sink invocation. Returns
    /// `true` to approve, `false` to deny.
    ///
    /// Implementations:
    ///   - MUST surface `tainted.audit_summary()` to the user, not
    ///     the wrapped payload bytes.
    ///   - MUST default to deny on any input shape other than the
    ///     documented "approve" gesture (`y` on the CLI, accept-tap
    ///     on mobile, click of the green button in the WebView).
    ///   - MAY block on I/O. The gate is invoked from an async
    ///     context but spawn-blocking inside the prompter is fine.
    fn prompt(&mut self, tainted: &Tainted<String>, sink: Reason) -> bool;
}

// ── CliPrompter ─────────────────────────────────────────────────────

/// Stdin/stderr prompter for the `vibecli` REPL and any other
/// terminal context where the user is present.
///
/// Reads one line from stdin and writes the consent banner to stderr
/// (so it doesn't interleave with subprocess stdout that the agent
/// loop may also be writing).
pub struct CliPrompter {
    stdin: BufReader<Box<dyn Read + Send>>,
    stderr: Box<dyn Write + Send>,
}

impl CliPrompter {
    /// Wire the prompter to the real `std::io::stdin()` and
    /// `std::io::stderr()`. This is the production constructor.
    pub fn new_real() -> Self {
        Self::new_with(Box::new(std::io::stdin()), Box::new(std::io::stderr()))
    }

    /// Wire the prompter to caller-supplied reader and writer. Used
    /// by tests to capture the prompt banner and inject the
    /// approve / deny input.
    pub fn new_with(stdin: Box<dyn Read + Send>, stderr: Box<dyn Write + Send>) -> Self {
        Self {
            stdin: BufReader::new(stdin),
            stderr,
        }
    }

    /// Render the consent banner to stderr. Format pinned to a stable
    /// shape so an external observability tool can parse it; the
    /// prompt itself is for a human, but the parts are predictable.
    fn render_banner(&mut self, tainted: &Tainted<String>, sink: Reason) -> std::io::Result<()> {
        let sink_name = match sink {
            Reason::ToolCallArgument => "tool-call argument",
            Reason::McpArgument => "MCP tool argument",
            Reason::LlmRequestBody => "LLM request body",
            Reason::ChatDisplay => "chat display",
            Reason::LogLine => "log line",
            Reason::Comparison => "comparison",
            Reason::StorageWrite => "storage write",
        };
        let origin_hint = match tainted.origin() {
            Provenance::File { path, byte_range } => match byte_range {
                Some(r) => format!("file {} (bytes {}-{})", path.display(), r.start, r.end),
                None => format!("file {}", path.display()),
            },
            Provenance::WebFetch { url, .. } => format!("web fetch from {url}"),
            Provenance::Mcp { server, tool, .. } => format!("MCP server '{server}' tool '{tool}'"),
            Provenance::Rag {
                index,
                doc_id,
                score,
            } => {
                format!("RAG index '{index}' doc {doc_id} (score {score:.3})")
            }
            Provenance::LlmResponse {
                provider, model, ..
            } => {
                format!("LLM provider '{provider}' model '{model}'")
            }
            Provenance::External { reason } => format!("external source ({reason})"),
        };
        writeln!(self.stderr)?;
        writeln!(
            self.stderr,
            "┌─ Confirmation required ─────────────────────────────────────────────"
        )?;
        writeln!(self.stderr, "│ Sink:    {sink_name}")?;
        writeln!(self.stderr, "│ Origin:  {origin_hint}")?;
        writeln!(self.stderr, "│ Audit:   {}", tainted.audit_summary())?;
        writeln!(self.stderr, "│")?;
        writeln!(
            self.stderr,
            "│ The above operation includes data from a T5 (attacker-controlled)"
        )?;
        writeln!(
            self.stderr,
            "│ source. The payload itself is not shown — the audit_id correlates"
        )?;
        writeln!(
            self.stderr,
            "│ to log entries if you need to inspect later. Type 'y' to approve,"
        )?;
        writeln!(self.stderr, "│ anything else to deny.")?;
        writeln!(
            self.stderr,
            "└─────────────────────────────────────────────────────────────────────"
        )?;
        write!(self.stderr, "Approve? [y/N]: ")?;
        self.stderr.flush()?;
        Ok(())
    }

    /// Read one line from stdin and decide. Anything other than
    /// case-insensitive exact `y` is a deny. EOF / read error → deny.
    fn read_decision(&mut self) -> bool {
        let mut line = String::new();
        match self.stdin.read_line(&mut line) {
            Ok(0) | Err(_) => false, // EOF / I/O error → deny
            Ok(_) => {
                let trimmed = line.trim();
                trimmed.eq_ignore_ascii_case("y")
            }
        }
    }
}

impl Prompter for CliPrompter {
    fn prompt(&mut self, tainted: &Tainted<String>, sink: Reason) -> bool {
        // Banner-render errors are *not* approvals. If stderr is
        // closed (broken pipe, etc.), default to deny.
        if self.render_banner(tainted, sink).is_err() {
            return false;
        }
        self.read_decision()
    }
}

// ── Test prompters ─────────────────────────────────────────────────

/// Test prompter that always approves. Used in `cfg(test)` to drive
/// the dispatcher's approve-path tests without involving stdin.
pub struct ApprovePrompter;

impl Prompter for ApprovePrompter {
    fn prompt(&mut self, _tainted: &Tainted<String>, _sink: Reason) -> bool {
        true
    }
}

/// Test prompter that always denies. Documents the deny-path more
/// explicitly than a `match` arm.
pub struct DenyPrompter;

impl Prompter for DenyPrompter {
    fn prompt(&mut self, _tainted: &Tainted<String>, _sink: Reason) -> bool {
        false
    }
}

/// Test prompter that records the prompt arguments for assertion
/// without making a decision until told. Used to verify that the
/// gate hands the prompter the right `Tainted<String>` and `Reason`.
pub struct RecordingPrompter {
    pub decisions: Mutex<Vec<(String, Reason)>>,
    pub approve: bool,
}

impl RecordingPrompter {
    pub fn new(approve: bool) -> Self {
        Self {
            decisions: Mutex::new(Vec::new()),
            approve,
        }
    }
}

impl Prompter for RecordingPrompter {
    fn prompt(&mut self, tainted: &Tainted<String>, sink: Reason) -> bool {
        let mut d = self.decisions.lock().unwrap_or_else(|e| e.into_inner());
        d.push((tainted.audit_summary(), sink));
        self.approve
    }
}

// ── Public entry point: gate with prompter ─────────────────────────

/// Slice-G entry point parallel to [`crate::tainted::confirm_shell_command`].
///
/// When the gate decides to ask the user, it calls
/// `prompter.prompt(tainted, sink)`. Approval mints a fresh
/// [`Confirmation`] with a random id. Denial returns
/// [`RejectionReason::PolicyDenied`] with `"user denied"` so the
/// agent loop can surface the rejection as a `tool_result` and the
/// model can adapt (design §10 q2).
pub fn confirm_with_prompter(
    tainted: &Tainted<String>,
    sink: Reason,
    prompter: &mut dyn Prompter,
) -> Result<Confirmation, RejectionReason> {
    tracing::debug!(
        target: "vibecody::tainted::prompter",
        origin = %tainted.origin().kind(),
        fingerprint = %tainted.log_fingerprint(),
        sink = ?sink,
        "prompter consent requested",
    );
    if prompter.prompt(tainted, sink) {
        let id = mint_confirmation_id();
        tracing::info!(
            target: "vibecody::tainted::prompter",
            decision = "approve",
            confirmation_id = %id,
            audit_id = %tainted.audit_id(),
            sink = ?sink,
            "user approved tainted-argument sink invocation",
        );
        Ok(Confirmation {
            id,
            sink,
            at: std::time::SystemTime::now(),
        })
    } else {
        tracing::info!(
            target: "vibecody::tainted::prompter",
            decision = "deny",
            audit_id = %tainted.audit_id(),
            sink = ?sink,
            "user denied tainted-argument sink invocation",
        );
        Err(RejectionReason::PolicyDenied("user denied".into()))
    }
}

fn mint_confirmation_id() -> String {
    use rand::Rng;
    // 96 bits of correlation entropy — collision chance is astronomical
    // and the format matches the existing `serve.rs` correlation-id
    // shape (lowercase hex). Two u64 draws + truncate to keep the
    // dependency surface identical to other id mints in the crate.
    let hi = rand::rng().random::<u64>();
    let lo = rand::rng().random::<u32>();
    format!("conf-{hi:016x}{lo:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CliPrompter unit tests ────────────────────────────────────

    #[test]
    fn cli_prompter_approves_on_lowercase_y() {
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"y\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "rm -rf /tmp".into());
        assert!(p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_approves_on_uppercase_y() {
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"Y\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "ls".into());
        assert!(p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_denies_on_explicit_n() {
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"n\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "rm".into());
        assert!(!p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_denies_on_empty_line() {
        // Just-hit-enter must NOT approve. Same as a banking prompt.
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "x".into());
        assert!(!p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_denies_on_word_yes() {
        // `yes` is NOT `y`. Spell-it-out approval is intentionally
        // denied so the matcher stays tight against fat-fingering.
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"yes\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "x".into());
        assert!(!p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_denies_on_eof() {
        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(Vec::<u8>::new()));
        let stderr: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", "x".into());
        assert!(!p.prompt(&t, Reason::ToolCallArgument));
    }

    #[test]
    fn cli_prompter_banner_does_not_contain_payload() {
        // The whole point — the banner shown to the user must surface
        // the audit metadata, not the bytes.
        let payload = "secret-payload-that-must-never-print";
        let captured: Vec<u8> = Vec::new();
        let stderr_buf = std::sync::Arc::new(Mutex::new(captured));

        struct SharedWriter(std::sync::Arc<Mutex<Vec<u8>>>);
        impl Write for SharedWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let stdin: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"n\n".to_vec()));
        let stderr: Box<dyn Write + Send> = Box::new(SharedWriter(stderr_buf.clone()));
        let mut p = CliPrompter::new_with(stdin, stderr);
        let t = Tainted::from_file("/repo/x", payload.into());
        let _ = p.prompt(&t, Reason::ToolCallArgument);

        let written = stderr_buf.lock().unwrap().clone();
        let s = String::from_utf8(written).expect("ASCII banner");
        assert!(!s.contains(payload), "banner leaked payload: {s}",);
        // Sanity: the banner *does* contain the kind + audit_id.
        assert!(s.contains("kind=file"));
        assert!(s.contains("audit_id="));
    }

    // ── confirm_with_prompter integration tests ────────────────────

    #[test]
    fn confirm_with_prompter_returns_confirmation_on_approve() {
        let mut prompter = ApprovePrompter;
        let t = Tainted::from_file("/repo/x", "x".into());
        let c = confirm_with_prompter(&t, Reason::ToolCallArgument, &mut prompter)
            .expect("approve must mint a Confirmation");
        assert!(c.id.starts_with("conf-"));
        assert_eq!(c.sink, Reason::ToolCallArgument);
        // The id has 24 hex chars after `conf-` (12 bytes × 2).
        assert_eq!(c.id.len(), "conf-".len() + 24);
    }

    #[test]
    fn confirm_with_prompter_returns_policy_denied_on_deny() {
        let mut prompter = DenyPrompter;
        let t = Tainted::from_file("/repo/x", "x".into());
        let err = confirm_with_prompter(&t, Reason::ToolCallArgument, &mut prompter).unwrap_err();
        match err {
            RejectionReason::PolicyDenied(msg) => assert!(msg.contains("user denied")),
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
    }

    #[test]
    fn confirm_with_prompter_forwards_tainted_and_sink_to_prompter() {
        let mut prompter = RecordingPrompter::new(true);
        let t = Tainted::from_mcp("fs-server", "read", "call-7", "x".into());
        let _ = confirm_with_prompter(&t, Reason::McpArgument, &mut prompter);
        let decisions = prompter.decisions.lock().unwrap();
        assert_eq!(decisions.len(), 1);
        let (summary, sink) = &decisions[0];
        assert!(summary.contains("kind=mcp"));
        assert!(summary.contains("server=fs-server"));
        assert_eq!(*sink, Reason::McpArgument);
    }

    #[test]
    fn mint_confirmation_id_is_unique_across_calls() {
        // Sanity: two consecutive mints don't collide (random 96-bit
        // tag — collision is astronomical, but pin the basic shape).
        let a = mint_confirmation_id();
        let b = mint_confirmation_id();
        assert_ne!(a, b);
        assert!(a.starts_with("conf-"));
        assert!(b.starts_with("conf-"));
    }
}
