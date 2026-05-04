//! Diff-mode code completion.
//!
//! Design intent: a patent-distant alternative to keystroke-driven ghost-text
//! inline completion. The surface differs on every claim element:
//!
//!   - **Trigger**: explicit chord (⌘.) in the host — never keystroke-driven.
//!   - **Hidden state**: only the user's explicit selection (or the whole file
//!     if nothing is selected) is forwarded, plus a bounded context window.
//!   - **Output**: a **unified diff**, not a code-suggestion-for-inline-insertion.
//!   - **Presentation**: a modal review UI on the host side — not inline
//!     ghost text or a drop-down.
//!   - **Accept**: multi-step review → (optional edit) → apply. No
//!     single-keystroke insertion path.
//!
//! Phase 1: no retrieval augmentation. Prefix/suffix + instruction only.

use crate::provider::{AIProvider, Message, MessageRole};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A single diff-completion request.
///
/// `selection_text` is the user's explicitly-selected text. If `None`, the
/// before/after context already contains everything the model needs.
///
/// `additional_files` is a user-selected list of related files to include as
/// extra context. This is human-in-the-loop retrieval — files are added by
/// the user via an explicit picker, never by automatic embedding search. That
/// distinction keeps Phase 2 patent-distant from keystroke-driven RAG paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffCompleteRequest {
    pub file_path: String,
    pub language: String,
    pub selection_text: Option<String>,
    pub selection_start_line: Option<u32>,
    pub selection_end_line: Option<u32>,
    pub before_context: String,
    pub after_context: String,
    pub instruction: String,
    #[serde(default)]
    pub additional_files: Vec<AdditionalFile>,
    /// The unified diff returned by the previous generate() call in this
    /// review session, if the user is iterating. Rendered into the prompt as
    /// a "Previous attempt" block so the model can refine rather than restart.
    /// User-supplied chain — never auto-collected from edit history.
    #[serde(default)]
    pub previous_diff: Option<String>,
    /// The user's natural-language refinement instruction for the previous
    /// diff (e.g. "tighten the error path"). Layered on top of `instruction`,
    /// not a replacement — keeping the chain visible is part of the explicit-
    /// user-direction posture.
    #[serde(default)]
    pub refinement: Option<String>,
    /// Author-authored project memory (VIBECLI.md / AGENTS.md / CLAUDE.md
    /// hierarchy + `~/.vibecli/memory.md` scratch). Phase 7 quick-win
    /// (slice 2026-04-26) — see `notes/PATENT_AUDIT_INLINE.md` "Slice
    /// audit — 2026-04-26". Carried as a separate optional field rather
    /// than spliced into `instruction` so it's emitted as a distinct
    /// system message and can be audited / disabled at the wire layer.
    /// **MUST NOT** carry auto-extracted state (OpenMemory, scratchpad,
    /// orchestration lessons) — only files the user explicitly authored.
    #[serde(default)]
    pub project_memory: Option<String>,
}

/// A single related file the user explicitly added as context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalFile {
    pub path: String,
    pub content: String,
}

/// The model's response, parsed into a unified-diff body and optional prose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffCompleteResponse {
    pub unified_diff: String,
    pub explanation: Option<String>,
    pub model_name: String,
}

const SYSTEM_PROMPT: &str = "You are an expert code editor. The user has \
selected a region of a file and given you an instruction. Respond with a \
unified diff (git-diff format) that accomplishes the instruction. \
\n\n\
Output format — exactly one fenced diff block, nothing else:\n\
```diff\n\
--- a/<path>\n\
+++ b/<path>\n\
@@ -<start>,<len> +<start>,<len> @@\n\
 context line\n\
-removed line\n\
+added line\n\
 context line\n\
```\n\
\n\
Rules:\n\
- Emit exactly one diff block, fenced with ```diff.\n\
- Use the file path given in the request for both a/ and b/ sides.\n\
- Include at least one line of context above and below every hunk.\n\
- Do not output any prose outside the diff block.";

/// Build the full message list sent to the provider. Always emits the
/// canonical SYSTEM_PROMPT first; when `request.project_memory` is
/// `Some`, follows it with a second system message carrying the
/// author-authored project memory verbatim under a clearly-labeled
/// header. The user message comes last and never embeds the memory
/// (memory is *context*, not *instruction* — keeping the layers
/// distinct is part of the patent-distance posture, see
/// `notes/PATENT_AUDIT_INLINE.md`).
pub fn build_messages(request: &DiffCompleteRequest) -> Vec<Message> {
    let mut messages = Vec::with_capacity(3);
    messages.push(Message {
        role: MessageRole::System,
        content: SYSTEM_PROMPT.to_string(),
    });
    if let Some(mem) = request
        .project_memory
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
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

/// Build the user message for the provider.
pub fn build_user_prompt(req: &DiffCompleteRequest) -> String {
    let mut out = String::with_capacity(
        req.before_context.len()
            + req.after_context.len()
            + req.selection_text.as_deref().unwrap_or("").len()
            + req.instruction.len()
            + 256,
    );
    out.push_str("File: ");
    out.push_str(&req.file_path);
    out.push_str("\nLanguage: ");
    out.push_str(&req.language);
    out.push_str("\n\n");

    out.push_str("=== Before selection ===\n");
    out.push_str(&req.before_context);
    out.push_str("\n");

    if let Some(sel) = &req.selection_text {
        out.push_str("=== Selected (");
        match (req.selection_start_line, req.selection_end_line) {
            (Some(s), Some(e)) => {
                out.push_str(&format!("lines {s}-{e}"));
            }
            _ => out.push_str("lines ?"),
        }
        out.push_str(") ===\n");
        out.push_str(sel);
        out.push_str("\n");
    }

    out.push_str("=== After selection ===\n");
    out.push_str(&req.after_context);
    out.push_str("\n");

    if !req.additional_files.is_empty() {
        out.push_str("\n=== Additional files (user-supplied context) ===\n");
        for file in &req.additional_files {
            out.push_str("\n--- ");
            out.push_str(&file.path);
            out.push_str(" ---\n");
            out.push_str(&file.content);
            out.push_str("\n");
        }
    }

    if let Some(prev) = req.previous_diff.as_ref().filter(|s| !s.trim().is_empty()) {
        out.push_str("\n=== Previous attempt (your last unified diff) ===\n");
        out.push_str(prev);
        out.push_str("\n");
    }

    if let Some(refine) = req.refinement.as_ref().filter(|s| !s.trim().is_empty()) {
        out.push_str("\nRefinement: ");
        out.push_str(refine);
        out.push_str("\n");
    }

    out.push_str("\nInstruction: ");
    out.push_str(&req.instruction);
    out
}

/// Extract the fenced diff body from a model response.
///
/// Returns `(diff_body, prose_outside_diff_or_none)`. When the model follows
/// the prompt, `prose` is empty or only whitespace and we discard it.
pub fn extract_diff(response: &str) -> (String, Option<String>) {
    let mut lines = response.lines().peekable();
    let mut before: Vec<&str> = Vec::new();
    let mut after: Vec<&str> = Vec::new();
    let mut diff_lines: Vec<&str> = Vec::new();
    let mut state = ExtractState::Before;

    while let Some(line) = lines.next() {
        match state {
            ExtractState::Before => {
                let trimmed = line.trim_start();
                if trimmed.starts_with("```diff") || trimmed.starts_with("```patch") {
                    state = ExtractState::InDiff;
                } else if trimmed.starts_with("```") && looks_like_diff_header(lines.peek().copied()) {
                    state = ExtractState::InDiff;
                } else {
                    before.push(line);
                }
            }
            ExtractState::InDiff => {
                if line.trim_start().starts_with("```") {
                    state = ExtractState::After;
                } else {
                    diff_lines.push(line);
                }
            }
            ExtractState::After => {
                after.push(line);
            }
        }
    }

    // Fallback: no fenced block. If the response starts with `--- ` assume the
    // whole thing is a raw diff.
    if diff_lines.is_empty() && response.trim_start().starts_with("--- ") {
        return (response.trim_end().to_string(), None);
    }

    let diff = diff_lines.join("\n");
    let mut prose = before;
    prose.extend(after);
    let prose_joined = prose.join("\n").trim().to_string();
    let prose_opt = if prose_joined.is_empty() { None } else { Some(prose_joined) };
    (diff, prose_opt)
}

enum ExtractState { Before, InDiff, After }

fn looks_like_diff_header(next: Option<&str>) -> bool {
    matches!(next, Some(l) if l.starts_with("--- ") || l.starts_with("diff --git"))
}

/// Generate a diff for the given request using the supplied provider.
///
/// The provider is invoked through its `chat` surface with a dedicated
/// system prompt — diffcomplete deliberately does not use any FIM/inline-
/// completion path (those have all been removed from VibeCody) so this
/// surface cannot be accidentally routed into keystroke-driven ghost-text
/// flows.
pub async fn generate(
    provider: Arc<dyn AIProvider>,
    request: DiffCompleteRequest,
) -> Result<DiffCompleteResponse> {
    let provider_name = provider.name().to_string();
    let instruction_len = request.instruction.len();
    let context_lines = request.before_context.lines().count() + request.after_context.lines().count();
    let has_selection = request.selection_text.is_some();
    let is_refinement = request.previous_diff.is_some();
    let extra_files = request.additional_files.len();

    tracing::debug!(
        target: "vibecody::diffcomplete",
        provider = %provider_name,
        language = %request.language,
        file_path = %request.file_path,
        instruction_len,
        context_lines,
        has_selection,
        is_refinement,
        extra_files,
        "diffcomplete request received"
    );

    if !provider.is_available().await {
        tracing::warn!(
            target: "vibecody::diffcomplete",
            provider = %provider_name,
            "diffcomplete provider unavailable"
        );
        anyhow::bail!("Provider {} is not available", provider_name);
    }

    let messages = build_messages(&request);

    let raw = provider.chat(&messages, None).await.map_err(|e| {
        tracing::warn!(
            target: "vibecody::diffcomplete",
            provider = %provider_name,
            error = %e,
            "diffcomplete provider chat call failed"
        );
        e
    })?;

    let (diff, prose) = extract_diff(&raw);

    if diff.trim().is_empty() {
        tracing::warn!(
            target: "vibecody::diffcomplete",
            provider = %provider_name,
            response_len = raw.len(),
            "diffcomplete model returned no diff block"
        );
        anyhow::bail!("Model response did not contain a diff block");
    }

    tracing::info!(
        target: "vibecody::diffcomplete",
        provider = %provider_name,
        diff_len = diff.len(),
        had_explanation = prose.is_some(),
        "diffcomplete generation succeeded"
    );

    Ok(DiffCompleteResponse {
        unified_diff: diff,
        explanation: prose,
        model_name: provider_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_stub() -> DiffCompleteRequest {
        DiffCompleteRequest {
            file_path: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            selection_text: Some("let x = 1;\n".to_string()),
            selection_start_line: Some(10),
            selection_end_line: Some(10),
            before_context: "fn main() {".to_string(),
            after_context: "}".to_string(),
            instruction: "rename x to count".to_string(),
            additional_files: Vec::new(),
            previous_diff: None,
            refinement: None,
            project_memory: None,
        }
    }

    // ── Phase 7 quick-win: project_memory wiring ─────────────────────────

    #[test]
    fn build_messages_emits_only_system_and_user_when_memory_absent() {
        let msgs = build_messages(&request_stub());
        assert_eq!(msgs.len(), 2, "no memory → exactly 2 messages");
        assert_eq!(msgs[0].role, MessageRole::System);
        assert!(msgs[0].content.starts_with("You are an expert code editor"));
        assert_eq!(msgs[1].role, MessageRole::User);
    }

    #[test]
    fn build_messages_inserts_memory_as_second_system_message() {
        let req = DiffCompleteRequest {
            project_memory: Some(
                "## Project Instructions\n\nUse Rust edition 2021. Prefer `?` over `.unwrap()`.".to_string(),
            ),
            ..request_stub()
        };
        let msgs = build_messages(&req);
        assert_eq!(msgs.len(), 3, "memory present → 3 messages (sys + memory + user)");
        assert_eq!(msgs[0].role, MessageRole::System);
        assert_eq!(msgs[1].role, MessageRole::System);
        assert!(
            msgs[1].content.contains("Project memory (author-authored"),
            "memory message must declare its provenance; got: {}",
            msgs[1].content
        );
        assert!(
            msgs[1].content.contains("Rust edition 2021"),
            "memory content must be carried verbatim; got: {}",
            msgs[1].content
        );
        assert_eq!(msgs[2].role, MessageRole::User);
    }

    #[test]
    fn build_messages_drops_blank_memory() {
        // Whitespace-only memory must not produce a system message —
        // there's nothing to add and an empty header would just confuse
        // the model. This pins the same defensive trim/filter as the
        // refinement field uses.
        let req = DiffCompleteRequest {
            project_memory: Some("   \n\t\n   ".to_string()),
            ..request_stub()
        };
        let msgs = build_messages(&req);
        assert_eq!(
            msgs.len(),
            2,
            "blank memory should be ignored; got {} messages",
            msgs.len()
        );
    }

    #[test]
    fn build_messages_keeps_memory_out_of_user_prompt() {
        // Patent-distance pin (notes/PATENT_AUDIT_INLINE.md slice 2026-04-26):
        // memory is system-level *context*, never spliced into the user's
        // instruction. Verify the user message is identical with or
        // without project_memory present.
        let with_mem = DiffCompleteRequest {
            project_memory: Some("Some project rules".to_string()),
            ..request_stub()
        };
        let without_mem = request_stub();
        let user_with = build_messages(&with_mem)
            .into_iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.content)
            .expect("user msg with memory");
        let user_without = build_messages(&without_mem)
            .into_iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.content)
            .expect("user msg without memory");
        assert_eq!(
            user_with, user_without,
            "project_memory must NOT leak into the user prompt"
        );
    }

    #[test]
    fn user_prompt_contains_all_sections() {
        let p = build_user_prompt(&request_stub());
        assert!(p.contains("File: src/lib.rs"));
        assert!(p.contains("Language: rust"));
        assert!(p.contains("=== Before selection ==="));
        assert!(p.contains("=== Selected (lines 10-10) ==="));
        assert!(p.contains("=== After selection ==="));
        assert!(p.contains("Instruction: rename x to count"));
    }

    #[test]
    fn user_prompt_handles_no_selection() {
        let req = DiffCompleteRequest { selection_text: None, ..request_stub() };
        let p = build_user_prompt(&req);
        assert!(!p.contains("=== Selected"));
        assert!(p.contains("=== Before selection ==="));
    }

    #[test]
    fn user_prompt_omits_additional_files_section_when_empty() {
        let p = build_user_prompt(&request_stub());
        assert!(!p.contains("Additional files"));
        assert!(!p.contains("(user-supplied context)"));
    }

    #[test]
    fn user_prompt_renders_additional_files_when_populated() {
        let req = DiffCompleteRequest {
            additional_files: vec![
                AdditionalFile {
                    path: "src/helper.rs".to_string(),
                    content: "pub fn helper() {}\n".to_string(),
                },
                AdditionalFile {
                    path: "src/types.rs".to_string(),
                    content: "pub struct Foo;\n".to_string(),
                },
            ],
            ..request_stub()
        };
        let p = build_user_prompt(&req);
        assert!(p.contains("=== Additional files (user-supplied context) ==="));
        assert!(p.contains("--- src/helper.rs ---"));
        assert!(p.contains("pub fn helper() {}"));
        assert!(p.contains("--- src/types.rs ---"));
        assert!(p.contains("pub struct Foo;"));
        // Section appears before the trailing instruction.
        let af_idx = p.find("Additional files").unwrap();
        let instr_idx = p.find("Instruction:").unwrap();
        assert!(af_idx < instr_idx);
    }

    #[test]
    fn user_prompt_omits_previous_attempt_when_absent() {
        let p = build_user_prompt(&request_stub());
        assert!(!p.contains("Previous attempt"));
        assert!(!p.contains("Refinement:"));
    }

    #[test]
    fn user_prompt_renders_previous_attempt_and_refinement() {
        let req = DiffCompleteRequest {
            previous_diff: Some(
                "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -10 +10 @@\n-let x = 1;\n+let count = 1;\n".to_string(),
            ),
            refinement: Some("also add a doc comment".to_string()),
            ..request_stub()
        };
        let p = build_user_prompt(&req);
        assert!(p.contains("=== Previous attempt (your last unified diff) ==="));
        assert!(p.contains("+let count = 1;"));
        assert!(p.contains("Refinement: also add a doc comment"));
        // Order: previous attempt → refinement → instruction.
        let prev_idx = p.find("Previous attempt").unwrap();
        let refine_idx = p.find("Refinement:").unwrap();
        let instr_idx = p.find("Instruction:").unwrap();
        assert!(prev_idx < refine_idx);
        assert!(refine_idx < instr_idx);
    }

    #[test]
    fn user_prompt_treats_blank_refinement_as_absent() {
        let req = DiffCompleteRequest {
            previous_diff: Some("--- a/x\n+++ b/x\n".to_string()),
            refinement: Some("   \n  ".to_string()),
            ..request_stub()
        };
        let p = build_user_prompt(&req);
        assert!(p.contains("Previous attempt"));
        assert!(!p.contains("Refinement:"));
    }

    #[test]
    fn extract_diff_fenced_diff_block() {
        let resp = "Some intro prose.\n```diff\n--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-old\n+new\n```\nTrailing note.";
        let (diff, prose) = extract_diff(resp);
        assert!(diff.contains("--- a/f.rs"));
        assert!(diff.contains("+new"));
        assert!(!diff.contains("```"));
        assert_eq!(prose.as_deref(), Some("Some intro prose.\nTrailing note."));
    }

    #[test]
    fn extract_diff_fenced_patch_block() {
        let resp = "```patch\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n```";
        let (diff, prose) = extract_diff(resp);
        assert!(diff.contains("+b"));
        assert!(prose.is_none());
    }

    #[test]
    fn extract_diff_raw_diff_fallback() {
        let resp = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
        let (diff, prose) = extract_diff(resp);
        assert!(diff.contains("--- a/x"));
        assert!(prose.is_none());
    }

    #[test]
    fn extract_diff_empty_on_no_fence_no_header() {
        let resp = "I cannot help with that.";
        let (diff, _) = extract_diff(resp);
        assert!(diff.is_empty());
    }

    #[test]
    fn extract_diff_plain_triple_backticks_with_diff_header() {
        let resp = "```\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n```";
        let (diff, _) = extract_diff(resp);
        assert!(diff.contains("--- a/x"));
        assert!(diff.contains("+new"));
    }

    #[test]
    fn system_prompt_mentions_diff_format() {
        assert!(SYSTEM_PROMPT.contains("unified diff"));
        assert!(SYSTEM_PROMPT.contains("```diff"));
    }

    // ── generate() — end-to-end through a mock provider ─────────────────────

    use crate::mock_provider::MockAIProvider;

    #[tokio::test]
    async fn generate_returns_parsed_diff() {
        let mock_response = "Here's the change:\n```diff\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -10,1 +10,1 @@\n-let x = 1;\n+let count = 1;\n```";
        let provider: Arc<dyn AIProvider> = Arc::new(
            MockAIProvider::with_responses("mock", vec![mock_response]),
        );

        let response = generate(provider, request_stub()).await.unwrap();

        assert!(response.unified_diff.contains("--- a/src/lib.rs"));
        assert!(response.unified_diff.contains("+let count = 1;"));
        assert!(!response.unified_diff.contains("```"));
        assert_eq!(response.model_name, "mock");
        assert_eq!(response.explanation.as_deref(), Some("Here's the change:"));
    }

    #[tokio::test]
    async fn generate_errors_when_no_diff_in_response() {
        let provider: Arc<dyn AIProvider> = Arc::new(
            MockAIProvider::with_responses("mock", vec!["Sorry, I cannot help."]),
        );
        let err = generate(provider, request_stub()).await.unwrap_err();
        assert!(err.to_string().contains("did not contain a diff"));
    }

    #[tokio::test]
    async fn generate_errors_when_provider_unavailable() {
        let mut mock = MockAIProvider::new("mock");
        mock.set_available(false);
        let provider: Arc<dyn AIProvider> = Arc::new(mock);
        let err = generate(provider, request_stub()).await.unwrap_err();
        assert!(err.to_string().contains("not available"));
    }
}
