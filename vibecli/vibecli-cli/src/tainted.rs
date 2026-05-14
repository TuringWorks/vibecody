//! Provenance-tracking newtype for T5-derived data — the building block
//! for prompt-injection containment per DREAD #1 and the design in
//! [`docs/security/tainted-data-flow.md`](../../docs/security/tainted-data-flow.md).
//!
//! **Slice A foundation.** This module ships the type, the [`Provenance`]
//! enum, the exit-method API (`expose_for` / `into_inner_after_confirmation`
//! / `sanitize_to`), and the propagation helpers (concat, slice, parse,
//! hash, length). It does **not** yet gate any tool-call sink — that's
//! the next slice. Existing code can start adopting the type without
//! waiting for the sink gating to land.
//!
//! Design choices mirror [`crate::redact::Redact`]:
//!
//! * **No `Deref`.** Forces explicit `.expose_for(Reason)` at every
//!   sink. Auto-deref would silently feed tainted strings into
//!   `format!` / `tool_call.args[…]` and defeat the discipline.
//! * **`Debug`/`Display` are redacted.** Show the provenance, not the
//!   bytes. Audit logs grep the provenance, not the payload.
//! * **Serde-transparent for `serde::Serialize`-only flows.** Tainted
//!   values often need to round-trip through SessionStore (which
//!   persists the taint metadata separately — see §6.4 of the design
//!   doc). For now, the serde impl is symmetric with the inner type
//!   and the provenance is stored alongside, not embedded.
//! * **No blanket `PartialEq`.** Comparing two tainted values is rare
//!   and should be explicit (`a.expose_for(Reason::Comparison) ==
//!   b.expose_for(Reason::Comparison)`) so the audit trail records
//!   the comparison.
//!
//! Example (slice-A usage, no sink gating yet):
//!
//! ```ignore
//! let file_text: Tainted<String> = Tainted::from_file(&path, contents);
//! // 100 lines of business logic...
//! let echo: String = file_text.expose_for(Reason::ChatDisplay).to_string();
//! tracing::info!(target: "vibecody::chat", "displayed {} bytes to user", echo.len());
//! ```
//!
//! When slice B+ lands, `fs.write` and `shell.exec` will accept
//! `TaintedOrTrusted<PathBuf>` / `TaintedOrTrusted<String>` and the
//! caller will be forced to choose between `.expose_for(...)` (which
//! routes through the confirmation flow when appropriate) and
//! `.sanitize_to::<WorkspacePath>()` (which composes with the existing
//! `safe_resolve_path` helper).

use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::path::PathBuf;
use std::time::SystemTime;

/// Where a tainted value came from. Recorded so the confirmation modal
/// and the audit log can show the user *why* a value is tainted.
///
/// New variants land here as new T5 sources are integrated. The 5
/// shapes below cover the 8 entry points named in §5 of the design
/// doc (e.g. `repo.diff` reuses [`Provenance::File`] with the diff
/// file's path; clipboard paste reuses [`Provenance::External`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Provenance {
    /// Read from a workspace file. Byte range is optional — when set,
    /// the audit log can show line numbers.
    File {
        path: PathBuf,
        byte_range: Option<Range<usize>>,
    },
    /// Fetched from the web (browser, `web.fetch`, MCP server that
    /// scrapes the open web).
    WebFetch {
        url: String,
        fetched_at: SystemTime,
    },
    /// Returned by an MCP tool invocation.
    Mcp {
        server: String,
        tool: String,
        call_id: String,
    },
    /// Returned by a RAG / semantic-index query against the workspace.
    Rag {
        index: String,
        doc_id: String,
        score: f32,
    },
    /// Returned by an LLM completion (the model is a T5 actor — its
    /// output may echo earlier T5 inputs that subverted its
    /// instructions).
    LlmResponse {
        provider: String,
        model: String,
        request_id: String,
    },
    /// Manual taint — for code paths that consume an external feed
    /// without going through one of the standard sources above
    /// (clipboard paste, pasted-into-prompt drag-drop, OAuth body).
    External { reason: String },
}

impl Provenance {
    /// Short stable identifier for log/UI use — surfaces as the
    /// `kind=…` field in audit-log lines without bringing the full
    /// payload along.
    pub fn kind(&self) -> &'static str {
        match self {
            Provenance::File { .. } => "file",
            Provenance::WebFetch { .. } => "web",
            Provenance::Mcp { .. } => "mcp",
            Provenance::Rag { .. } => "rag",
            Provenance::LlmResponse { .. } => "llm",
            Provenance::External { .. } => "ext",
        }
    }
}

/// Why a tainted value is being exposed. Lands in the audit log so an
/// incident reviewer can ask "what sink consumed this T5 bytes?".
///
/// `Reason` is *additive* — when in doubt, add a new variant rather
/// than overload an existing one. The cost is small (one match arm in
/// the audit-log formatter) and the alternative is forensic ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Displaying to the user in the chat UI / CLI (already runs
    /// through DOMPurify for the WebView path — see DREAD #10).
    ChatDisplay,
    /// Logging at INFO / WARN / ERROR level. Slice F will redact
    /// origin/byte-range to a 16-char hash before this exposure
    /// surfaces in the daemon log file.
    LogLine,
    /// Comparing for equality or hashing. The compare/hash output is
    /// not itself tainted (see propagation rule #4 / #5).
    Comparison,
    /// Building the JSON body of an outbound LLM-provider HTTPS call.
    /// Expected; the model needs to *receive* T5 context.
    LlmRequestBody,
    /// Building an MCP tool-call argument.
    McpArgument,
    /// Argument to a privileged tool sink (fs.write, shell.exec,
    /// git.commit, http.request). Slice B+ will require this exposure
    /// to be paired with a [`Confirmation`] token.
    ToolCallArgument,
    /// Persisting to SessionStore / WorkspaceStore / recap row. Taint
    /// is preserved on read-back via the store's typed accessor.
    StorageWrite,
}

/// Proof that the user (or a configured headless-mode trust marker)
/// authorized a specific tool-call exposure. Issued by the confirmation
/// modal (desktop / CLI / mobile) and consumed exactly once at the
/// sink. Slice G builds the modal; this struct is the contract.
///
/// The fields are intentionally minimal — they identify the
/// confirmation, not the bytes. The audit log correlates this token's
/// `id` to the bytes that were exposed.
#[derive(Debug, Clone)]
pub struct Confirmation {
    /// Stable correlation id — also lands in the audit log entry.
    pub id: String,
    /// Which sink the user approved.
    pub sink: Reason,
    /// When the user clicked / typed `y`.
    pub at: SystemTime,
}

/// A value whose origin is outside T1 (the user's keyboard / local
/// WebView). See module docs.
#[derive(Clone)]
pub struct Tainted<T> {
    value: T,
    origin: Provenance,
}

impl<T> Tainted<T> {
    /// Wrap a value with explicit provenance. Use this at the source
    /// — the function that *reads* the file, *fetches* the URL,
    /// *receives* the MCP response.
    #[inline]
    pub fn new(value: T, origin: Provenance) -> Self {
        Self { value, origin }
    }

    /// Borrow the inner value with an explicit [`Reason`]. This is
    /// the slice-A exit — it does not yet enforce that a
    /// [`Confirmation`] is present for tool-call sinks. Slice B will
    /// split this into `expose_for(Reason)` (allowed sinks) and
    /// `expose_for_with_confirmation(Reason, &Confirmation)`
    /// (privileged sinks). Existing call sites that adopt the type
    /// today will compile-error at slice B's release and be guided
    /// to the right exit by the type.
    #[inline]
    pub fn expose_for(&self, _reason: Reason) -> &T {
        // Audit-log emission lands in slice F. The plumbing is
        // deliberately a no-op today so callers can adopt the type
        // before the log surface is finalized.
        &self.value
    }

    /// Consume the wrapper after the user explicitly confirmed via a
    /// [`Confirmation`] token. The token is consumed by value — it
    /// can't be reused — and the audit log records the exposure.
    /// Slice G mints these tokens at the modal.
    pub fn into_inner_after_confirmation(self, confirmation: Confirmation) -> T {
        // Slice F will emit the audit-log line here. The
        // confirmation's `sink` is matched against the sink the
        // caller is using — slice B+ will refine this into a typed
        // contract.
        let _ = confirmation;
        self.value
    }

    /// Provenance of the wrapped value. Cheap (`&Provenance`) — the
    /// confirmation modal uses this to render the "this command
    /// includes text from …" cue described in design doc §8.1.
    #[inline]
    pub fn origin(&self) -> &Provenance {
        &self.origin
    }

    /// Map the inner value, preserving the same provenance. The
    /// closure must not introduce a new T5 source — if you're mixing
    /// in tainted data from elsewhere, use [`Tainted::concat`]
    /// instead so the merged provenance is captured.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Tainted<U> {
        Tainted {
            value: f(self.value),
            origin: self.origin,
        }
    }
}

impl<T> Tainted<T>
where
    T: AsRef<[u8]>,
{
    /// SHA-256 of the wrapped bytes, untainted. Rule #4: hashing
    /// strips the carrier capacity of the value, so the result is
    /// safe to feed into the audit-log correlation column or a cache
    /// key without dragging taint along.
    pub fn hash_sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.value.as_ref());
        h.into()
    }

    /// Byte length of the wrapped value, untainted. Rule #5: a
    /// length is not a useful prompt-injection carrier.
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.value.as_ref().len()
    }
}

impl Tainted<String> {
    /// Tainted file-read constructor — the most common entry point.
    pub fn from_file(path: impl Into<PathBuf>, value: String) -> Self {
        Self::new(
            value,
            Provenance::File {
                path: path.into(),
                byte_range: None,
            },
        )
    }

    /// Tainted web-fetch constructor.
    pub fn from_web(url: impl Into<String>, value: String) -> Self {
        Self::new(
            value,
            Provenance::WebFetch {
                url: url.into(),
                fetched_at: SystemTime::now(),
            },
        )
    }

    /// Tainted LLM-completion constructor — for the load-bearing
    /// rule that model output is itself T5 (see design doc §5 #8).
    pub fn from_llm_response(
        provider: impl Into<String>,
        model: impl Into<String>,
        request_id: impl Into<String>,
        value: String,
    ) -> Self {
        Self::new(
            value,
            Provenance::LlmResponse {
                provider: provider.into(),
                model: model.into(),
                request_id: request_id.into(),
            },
        )
    }

    /// Tainted MCP-tool-return constructor.
    pub fn from_mcp(
        server: impl Into<String>,
        tool: impl Into<String>,
        call_id: impl Into<String>,
        value: String,
    ) -> Self {
        Self::new(
            value,
            Provenance::Mcp {
                server: server.into(),
                tool: tool.into(),
                call_id: call_id.into(),
            },
        )
    }

    /// Concatenate two tainted strings. Propagation rule #1: the
    /// result is tainted with **the first operand's provenance**
    /// (the design doc leaves multi-origin provenance as a future
    /// extension — slice A picks the conservative single-origin form
    /// so the type stays a thin wrapper).
    ///
    /// If you need multi-origin attribution, use
    /// [`Tainted::map`] with a closure that knows the full set of
    /// sources, or wait for slice E's `Provenance::Merged` variant.
    pub fn concat(self, other: Tainted<String>) -> Tainted<String> {
        let merged = self.value + other.value.as_str();
        Tainted {
            value: merged,
            origin: self.origin,
        }
    }

    /// Slice that preserves taint. Returns `None` if the range falls
    /// outside the value, matching the standard `String::get`
    /// contract.
    pub fn slice(&self, range: Range<usize>) -> Option<Tainted<String>> {
        self.value.get(range.clone()).map(|s| Tainted {
            value: s.to_string(),
            origin: match &self.origin {
                Provenance::File {
                    path,
                    byte_range: _,
                } => Provenance::File {
                    path: path.clone(),
                    byte_range: Some(range),
                },
                other => other.clone(),
            },
        })
    }
}

impl<T> std::fmt::Debug for Tainted<T> {
    /// Surfaces the provenance kind, not the value. Mirrors
    /// [`crate::redact::Redact`] but keeps the `kind` visible so
    /// debugging tools can tell `[tainted/file]` apart from
    /// `[tainted/web]` without dumping content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[tainted/{}]", self.origin.kind())
    }
}

impl<T> std::fmt::Display for Tainted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[tainted/{}]", self.origin.kind())
    }
}

// ── Slice A: shell.exec gate ──────────────────────────────────────────
//
// Slice A's promise in design §9 is "core type + first sink". The type
// ships above; the gate function below is the contract for the first
// sink. Wiring it into `tool_executor::run_bash` is a follow-up in this
// same slice — keeping the gate function separate lets `run_bash` adopt
// the contract incrementally without a sweeping signature change.

/// How a confirmation request is fulfilled in the current process.
/// Mirrors design §10 question 4 (headless behaviour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmMode {
    /// CI / scripted / non-interactive process. Default = reject every
    /// tainted-argument tool call (`RejectionReason::Headless`).
    Headless,
    /// Interactive process with a real user. Slice A doesn't wire the
    /// modal UI yet — rejects with `RejectionReason::InteractiveStub` so
    /// any caller that depends on the gate fails loud rather than
    /// silently auto-approving during the rollout window. Slice G
    /// replaces this branch with the actual modal flow.
    Interactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    /// Headless mode never approves tainted-argument tool calls.
    Headless,
    /// Interactive UI is not yet wired. Callers see this until Slice G
    /// ships the modal.
    InteractiveStub,
    /// Workspace- or daemon-scoped admin policy forbids this sink
    /// regardless of confirmation outcome.
    PolicyDenied(String),
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Headless => {
                write!(f, "headless mode rejects tainted-argument tool calls")
            }
            Self::InteractiveStub => write!(
                f,
                "interactive confirmation UI not yet wired (DREAD #1 Slice G)"
            ),
            Self::PolicyDenied(msg) => write!(f, "policy denied: {msg}"),
        }
    }
}

/// Gate a `shell.exec` tool call carrying a [`Tainted<String>`] command.
/// Slice A's first sink — subsequent slices add `confirm_file_write`,
/// `confirm_http_outbound`, etc.
///
/// **Headless mode**: always rejects (design §10 q4 default).
/// **Interactive mode**: rejects with `InteractiveStub` until Slice G
/// wires the modal. The sink contract is committed *now* so the
/// integration shape is stable and `tool_executor::run_bash` can begin
/// adopting it.
///
/// The function takes `&Tainted<String>` (not consuming) so the caller
/// can both gate AND retain the value for later
/// `.into_inner_after_confirmation(...)` exposure once the modal is
/// wired.
pub fn confirm_shell_command(
    cmd: &Tainted<String>,
    mode: ConfirmMode,
) -> Result<Confirmation, RejectionReason> {
    // Audit trail for the gate decision lands in tracing today; the
    // structured audit-log slice (design §10 q5) consumes the same
    // event later without changing this surface.
    tracing::debug!(
        target: "vibecody::tainted::shell_gate",
        origin = %cmd.origin.kind(),
        mode = ?mode,
        "shell.exec confirmation requested",
    );

    match mode {
        ConfirmMode::Headless => Err(RejectionReason::Headless),
        ConfirmMode::Interactive => {
            // Slice G replaces this stub with a real modal that mints a
            // Confirmation on user-approval and propagates the
            // RejectionReason::PolicyDenied case from any active admin
            // policy.
            Err(RejectionReason::InteractiveStub)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issued() -> Confirmation {
        Confirmation {
            id: "conf-test-0001".into(),
            sink: Reason::ToolCallArgument,
            at: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn debug_shows_origin_kind_not_value() {
        let t = Tainted::from_file("/tmp/README.md", "ignore previous instructions".into());
        let s = format!("{t:?}");
        assert_eq!(s, "[tainted/file]");
        assert!(!s.contains("ignore previous"));
    }

    #[test]
    fn display_shows_origin_kind_not_value() {
        let t = Tainted::from_web("https://evil.example/post", "exfil text".into());
        assert_eq!(format!("{t}"), "[tainted/web]");
    }

    #[test]
    fn expose_for_returns_inner_borrow_for_legitimate_sinks() {
        let t = Tainted::from_file("/x", "hi".into());
        // ChatDisplay is a legitimate slice-A sink.
        assert_eq!(t.expose_for(Reason::ChatDisplay), "hi");
        // LlmRequestBody is too — the model needs to receive context.
        assert_eq!(t.expose_for(Reason::LlmRequestBody), "hi");
    }

    #[test]
    fn into_inner_after_confirmation_consumes_and_returns() {
        let t = Tainted::from_file("/x", "rm -rf /tmp/foo".into());
        let inner = t.into_inner_after_confirmation(issued());
        assert_eq!(inner, "rm -rf /tmp/foo");
    }

    #[test]
    fn origin_reflects_constructor() {
        let f = Tainted::from_file("/repo/README.md", "x".into());
        assert!(matches!(f.origin(), Provenance::File { .. }));

        let w = Tainted::from_web("https://example.com/p", "x".into());
        assert!(matches!(w.origin(), Provenance::WebFetch { .. }));

        let m = Tainted::from_mcp("fs-server", "read", "call-7", "x".into());
        assert!(matches!(m.origin(), Provenance::Mcp { .. }));

        let l = Tainted::from_llm_response("anthropic", "claude-opus-4-7", "req-1", "x".into());
        assert!(matches!(l.origin(), Provenance::LlmResponse { .. }));
    }

    #[test]
    fn hash_strips_taint_carrier_capacity() {
        let a = Tainted::from_file("/x", "abcdef".into());
        let b = Tainted::from_web("https://other", "abcdef".into());
        // Same bytes, different provenance → same hash. The hash is
        // a fixed-size summary that can't carry a prompt-injection
        // payload, so it's safe to surface in logs.
        assert_eq!(a.hash_sha256(), b.hash_sha256());
        assert_eq!(a.hash_sha256().len(), 32);
    }

    #[test]
    fn byte_len_is_untainted() {
        let t = Tainted::from_file("/x", "12345".into());
        assert_eq!(t.byte_len(), 5);
    }

    #[test]
    fn concat_is_contagious_and_preserves_first_origin() {
        let a = Tainted::from_file("/repo/README.md", "hello ".into());
        let b = Tainted::from_web("https://x", "world".into());
        let c = a.concat(b);
        assert_eq!(c.expose_for(Reason::ChatDisplay), "hello world");
        // The first operand's provenance wins — design doc §7 rule #1
        // with the conservative single-origin Provenance shape.
        assert!(matches!(c.origin(), Provenance::File { .. }));
    }

    #[test]
    fn slice_preserves_taint_and_narrows_byte_range_for_file_origin() {
        let t = Tainted::from_file("/repo/x.md", "0123456789".into());
        let sub = t.slice(2..5).expect("in range");
        assert_eq!(sub.expose_for(Reason::ChatDisplay), "234");
        match sub.origin() {
            Provenance::File { byte_range, .. } => {
                assert_eq!(*byte_range, Some(2..5));
            }
            other => panic!("expected File origin, got {other:?}"),
        }
    }

    #[test]
    fn slice_returns_none_for_out_of_range() {
        let t = Tainted::from_file("/x", "hi".into());
        assert!(t.slice(10..20).is_none());
    }

    #[test]
    fn map_preserves_origin() {
        let t = Tainted::from_file("/x", "  hello  ".to_string());
        let trimmed = t.map(|s| s.trim().to_string());
        assert_eq!(trimmed.expose_for(Reason::ChatDisplay), "hello");
        assert!(matches!(trimmed.origin(), Provenance::File { .. }));
    }

    #[test]
    fn provenance_kind_is_stable_for_log_grep() {
        assert_eq!(
            Provenance::File {
                path: "/x".into(),
                byte_range: None
            }
            .kind(),
            "file"
        );
        assert_eq!(
            Provenance::WebFetch {
                url: "u".into(),
                fetched_at: SystemTime::UNIX_EPOCH
            }
            .kind(),
            "web"
        );
        assert_eq!(
            Provenance::Mcp {
                server: "s".into(),
                tool: "t".into(),
                call_id: "c".into()
            }
            .kind(),
            "mcp"
        );
        assert_eq!(
            Provenance::LlmResponse {
                provider: "p".into(),
                model: "m".into(),
                request_id: "r".into()
            }
            .kind(),
            "llm"
        );
        assert_eq!(
            Provenance::Rag {
                index: "i".into(),
                doc_id: "d".into(),
                score: 0.9
            }
            .kind(),
            "rag"
        );
        assert_eq!(
            Provenance::External {
                reason: "clipboard".into()
            }
            .kind(),
            "ext"
        );
    }

    // ── Slice A: shell.exec gate ──────────────────────────────────────

    #[test]
    fn confirm_shell_command_headless_always_rejects() {
        let cmd = Tainted::from_file("/repo/README.md", "rm -rf /tmp/foo".into());
        let res = confirm_shell_command(&cmd, ConfirmMode::Headless);
        assert!(matches!(res, Err(RejectionReason::Headless)));
    }

    #[test]
    fn confirm_shell_command_interactive_stubs_until_slice_g() {
        let cmd = Tainted::from_file("/repo/README.md", "ls".into());
        let res = confirm_shell_command(&cmd, ConfirmMode::Interactive);
        // Slice G replaces this with a real modal — fail-closed is the
        // safer default during the rollout.
        assert!(matches!(res, Err(RejectionReason::InteractiveStub)));
    }

    #[test]
    fn rejection_reason_display_is_actionable() {
        assert!(format!("{}", RejectionReason::Headless).contains("headless"));
        assert!(format!("{}", RejectionReason::InteractiveStub).contains("Slice G"));
        assert!(format!("{}", RejectionReason::PolicyDenied("no shell".into()))
            .contains("no shell"));
    }

    #[test]
    fn external_provenance_carries_reason() {
        let t = Tainted::new(
            "pasted".to_string(),
            Provenance::External {
                reason: "clipboard paste from VibeMobile".into(),
            },
        );
        match t.origin() {
            Provenance::External { reason } => {
                assert!(reason.contains("clipboard"));
            }
            other => panic!("expected External, got {other:?}"),
        }
    }
}
