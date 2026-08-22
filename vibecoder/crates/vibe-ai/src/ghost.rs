//! Explicit-trigger inline completion ("ghost text").
//!
//! # Why this module is not the surface that was removed
//!
//! Commit `5a7eef7c` deleted the previous ghost-text path (`SupercompleteEngine`
//! + `request_inline_completion` + `predict_next_edit`) after an internal patent
//! audit flagged it HIGH. That surface was **keystroke-driven**: an edit-history
//! ring buffer fed a debounced FIM request on every pause in typing, and the
//! model's output was inserted inline on a single keypress.
//!
//! This module deliberately keeps the useful half and drops the flagged half:
//!
//!   - **Trigger**: explicit chord only. The hosts gate on the editor's own
//!     "explicit" trigger kind (`InlineCompletionTriggerKind::Explicit` in
//!     Monaco, `::Invoke` in VS Code) and return nothing for the automatic kind.
//!     There is no debounce timer and no on-type path to remove, because none
//!     is ever installed.
//!   - **Hidden state**: none. The request carries the prefix/suffix window
//!     around the cursor and nothing else — no edit-event history, no
//!     accepted/rejected telemetry, no automatic embedding retrieval. What the
//!     model sees is what is on screen.
//!   - **Output**: a plain continuation for the current cursor position, capped
//!     to a bounded number of lines.
//!
//! The one thing shared with the removed surface is inline presentation and
//! Tab-to-accept, which the host provides natively.
//!
//! [`crate::diffcomplete`] remains the multi-line, review-before-apply surface;
//! this one is for the short continuation where opening a modal is too heavy.

use crate::provider::{AIProvider, Message, MessageRole};
use crate::tools::strip_code_fence;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// How many lines of continuation we ask for, and enforce on the way out.
///
/// A cap is part of the design, not a performance tweak: an unbounded
/// continuation is a code generator, and this surface is deliberately a
/// completion of the line(s) under the cursor.
pub const MAX_COMPLETION_LINES: usize = 12;

/// A single explicit-trigger completion request.
///
/// `prefix` is the text before the cursor and `suffix` the text after it, each
/// already windowed by the host. Splitting at the cursor rather than sending
/// the whole file is what lets the model complete *at* a point instead of
/// rewriting a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostRequest {
    pub file_path: String,
    pub language: String,
    /// Text before the cursor (windowed by the host).
    pub prefix: String,
    /// Text after the cursor (windowed by the host).
    pub suffix: String,
    /// Author-authored project memory, same audit-restricted source as
    /// diffcomplete's. **MUST NOT** carry auto-extracted state.
    #[serde(default)]
    pub project_memory: Option<String>,
}

/// A completion, ready for the host to render as ghost text at the cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostResponse {
    /// Text to insert verbatim at the cursor. Never empty on `Ok`.
    pub completion: String,
    pub model_name: String,
    /// True when [`MAX_COMPLETION_LINES`] clipped the model's output. The host
    /// surfaces this rather than pretending the suggestion was complete.
    pub truncated: bool,
}

const SYSTEM_PROMPT: &str = "You are a code completion engine. The user gives \
you a file split at the cursor into a PREFIX and a SUFFIX. Output the text that \
belongs at the cursor, so that PREFIX + your output + SUFFIX is valid code.\n\
\n\
Rules:\n\
- Output ONLY the insertion text. No prose, no explanation, no commentary.\n\
- Do NOT repeat any part of the PREFIX or the SUFFIX.\n\
- Do NOT wrap the output in a markdown code fence.\n\
- Continue the prefix exactly where it stops, mid-token if that is where the \
cursor is.\n\
- Match the surrounding indentation, naming style, and language idiom.\n\
- Keep it short: complete the current statement, expression, or block. Stop \
when a reasonable suggestion ends.\n\
- If nothing sensible belongs at the cursor, output nothing at all.";

/// Build the message list for a request.
///
/// Mirrors [`crate::diffcomplete::build_messages`]: canonical system prompt
/// first, then project memory as its own system message when present, then the
/// user message. Memory is *context*, never folded into the instruction.
pub fn build_messages(request: &GhostRequest) -> Vec<Message> {
    let memory = request
        .project_memory
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut messages = Vec::with_capacity(if memory.is_some() { 3 } else { 2 });
    messages.push(Message {
        role: MessageRole::System,
        content: SYSTEM_PROMPT.to_string(),
    });
    if let Some(mem) = memory {
        messages.push(Message {
            role: MessageRole::System,
            content: format!(
                "Project memory (author-authored, from VIBECLI.md / AGENTS.md / CLAUDE.md):\n\n{mem}"
            ),
        });
    }
    messages.push(Message {
        role: MessageRole::User,
        content: build_user_prompt(request),
    });
    messages
}

/// Build the user message.
pub fn build_user_prompt(req: &GhostRequest) -> String {
    let mut out = String::with_capacity(req.prefix.len() + req.suffix.len() + 256);
    out.push_str("File: ");
    out.push_str(&req.file_path);
    out.push_str("\nLanguage: ");
    out.push_str(&req.language);
    out.push_str("\n\n=== PREFIX (text before the cursor) ===\n");
    out.push_str(&req.prefix);
    out.push_str("\n=== CURSOR ===\n=== SUFFIX (text after the cursor) ===\n");
    out.push_str(&req.suffix);
    out.push_str("\n\nOutput the insertion text for the cursor position:");
    out
}

/// Strip the wrappers models add despite being told not to, and enforce the
/// line cap.
///
/// Returns `(completion, truncated)`. An empty completion means the model
/// declined — that is a valid answer here ("nothing belongs at the cursor"),
/// and the caller reports it as such rather than as an error.
///
/// Trailing whitespace is trimmed but **leading whitespace is preserved**: at a
/// cursor sitting at column 0 of an indented block, the indentation *is* the
/// first thing that belongs at the cursor.
pub fn sanitize_completion(raw: &str) -> (String, bool) {
    let unfenced = strip_code_fence(raw);
    let mut lines: Vec<&str> = unfenced.lines().collect();

    // Drop trailing blank lines before counting, so a model that pads its
    // answer doesn't burn the line budget or trip the truncation flag.
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    let truncated = lines.len() > MAX_COMPLETION_LINES;
    if truncated {
        lines.truncate(MAX_COMPLETION_LINES);
    }

    let joined = lines.join("\n");
    // `trim_end` only — see the doc comment on leading whitespace.
    (joined.trim_end().to_string(), truncated)
}

/// Generate a completion for the cursor position using the supplied provider.
///
/// Callers must have established that the user explicitly asked for this — the
/// hosts do that by gating on the editor's explicit trigger kind. Nothing in
/// this function can tell an explicit request from an automatic one, so the
/// gate belongs at the edge and must not be relaxed there.
pub async fn generate(
    provider: Arc<dyn AIProvider>,
    request: GhostRequest,
) -> Result<GhostResponse> {
    let provider_name = provider.name().to_string();

    tracing::debug!(
        target: "vibecody::ghost",
        provider = %provider_name,
        language = %request.language,
        file_path = %request.file_path,
        prefix_len = request.prefix.len(),
        suffix_len = request.suffix.len(),
        "ghost completion requested"
    );

    if !provider.is_available().await {
        tracing::warn!(
            target: "vibecody::ghost",
            provider = %provider_name,
            "ghost provider unavailable"
        );
        anyhow::bail!("Provider {} is not available", provider_name);
    }

    let messages = build_messages(&request);

    let raw = provider.chat(&messages, None).await.map_err(|e| {
        tracing::warn!(
            target: "vibecody::ghost",
            provider = %provider_name,
            error = %e,
            "ghost provider chat call failed"
        );
        e
    })?;

    let (completion, truncated) = sanitize_completion(&raw);

    tracing::info!(
        target: "vibecody::ghost",
        provider = %provider_name,
        completion_len = completion.len(),
        truncated,
        "ghost completion generated"
    );

    Ok(GhostResponse {
        completion,
        model_name: provider_name,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_stub() -> GhostRequest {
        GhostRequest {
            file_path: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            prefix: "fn add(a: i32, b: i32) -> i32 {\n    ".to_string(),
            suffix: "\n}\n".to_string(),
            project_memory: None,
        }
    }

    // ── Prompt construction ──────────────────────────────────────────────

    #[test]
    fn build_messages_emits_only_system_and_user_when_memory_absent() {
        let msgs = build_messages(&request_stub());
        assert_eq!(msgs.len(), 2, "no memory → exactly 2 messages");
        assert_eq!(msgs[0].role, MessageRole::System);
        assert_eq!(msgs[1].role, MessageRole::User);
    }

    #[test]
    fn build_messages_inserts_memory_as_second_system_message() {
        let req = GhostRequest {
            project_memory: Some("Always use anyhow::Result.".to_string()),
            ..request_stub()
        };
        let msgs = build_messages(&req);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1].role, MessageRole::System);
        assert!(msgs[1].content.contains("Always use anyhow::Result."));
        assert_eq!(msgs[2].role, MessageRole::User);
        assert!(
            !msgs[2].content.contains("Always use anyhow::Result."),
            "memory is context, not instruction — it must not leak into the user message"
        );
    }

    #[test]
    fn whitespace_only_memory_is_treated_as_absent() {
        let req = GhostRequest {
            project_memory: Some("   \n  ".to_string()),
            ..request_stub()
        };
        assert_eq!(build_messages(&req).len(), 2);
    }

    #[test]
    fn user_prompt_splits_at_the_cursor() {
        let prompt = build_user_prompt(&request_stub());
        let cursor = prompt
            .find("=== CURSOR ===")
            .expect("cursor marker present");
        let prefix_at = prompt.find("fn add(a: i32").expect("prefix present");
        let suffix_at = prompt.rfind("\n}\n").expect("suffix present");
        assert!(prefix_at < cursor, "prefix must precede the cursor marker");
        assert!(suffix_at > cursor, "suffix must follow the cursor marker");
    }

    // ── Sanitising ───────────────────────────────────────────────────────

    #[test]
    fn plain_completion_passes_through() {
        let (out, truncated) = sanitize_completion("a + b");
        assert_eq!(out, "a + b");
        assert!(!truncated);
    }

    #[test]
    fn leading_indentation_is_preserved() {
        // The cursor sits at column 0 of an indented block; the indentation is
        // part of what belongs at the cursor. Trimming it would left-align the
        // suggestion against its neighbours.
        let (out, _) = sanitize_completion("    let x = 1;\n    x + 1");
        assert_eq!(out, "    let x = 1;\n    x + 1");
    }

    #[test]
    fn fenced_response_is_unwrapped() {
        let (out, _) = sanitize_completion("```rust\na + b\n```");
        assert_eq!(out, "a + b");
    }

    #[test]
    fn fenced_response_without_info_string_is_unwrapped() {
        let (out, _) = sanitize_completion("```\na + b\n```");
        assert_eq!(out, "a + b");
    }

    #[test]
    fn interior_fence_is_left_alone() {
        // A completion that writes a doc comment containing a fence must not
        // be mangled — only a fence wrapping the *whole* response is stripped.
        let raw = "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}";
        let (out, _) = sanitize_completion(raw);
        assert_eq!(out, raw);
    }

    #[test]
    fn declining_to_complete_yields_empty_not_error() {
        let (out, truncated) = sanitize_completion("   \n\n  ");
        assert!(
            out.is_empty(),
            "an empty answer is a valid 'nothing fits here'"
        );
        assert!(!truncated);
    }

    #[test]
    fn output_is_capped_and_flagged() {
        let long = (0..MAX_COMPLETION_LINES + 5)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (out, truncated) = sanitize_completion(&long);
        assert!(truncated, "over-long output must report truncation");
        assert_eq!(out.lines().count(), MAX_COMPLETION_LINES);
    }

    #[test]
    fn trailing_blank_lines_do_not_trip_truncation() {
        // Exactly at the cap plus padding: the padding is dropped before
        // counting, so this is not a truncation.
        let padded = format!(
            "{}\n\n\n",
            (0..MAX_COMPLETION_LINES)
                .map(|i| format!("line {i}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let (out, truncated) = sanitize_completion(&padded);
        assert!(!truncated);
        assert_eq!(out.lines().count(), MAX_COMPLETION_LINES);
    }

    #[test]
    fn unterminated_fence_still_unwraps() {
        // Streaming responses get cut off mid-fence; the body is still usable.
        let (out, _) = sanitize_completion("```rust\na + b");
        assert_eq!(out, "a + b");
    }

    #[test]
    fn bare_fence_yields_empty() {
        let (out, _) = sanitize_completion("```");
        assert!(out.is_empty());
    }
}
