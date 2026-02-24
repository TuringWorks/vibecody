//! OpenTelemetry span attribute constants and recording helpers.
//!
//! This module provides the named attribute keys used when emitting tracing spans
//! for agent operations. The actual OTLP export pipeline is configured in
//! `vibecli-cli/src/otel_init.rs` — this layer only produces `tracing` spans,
//! which are zero-cost when no subscriber is registered.
//!
//! # Span hierarchy
//!
//! ```text
//! agent.session (root)
//! └── agent.llm_call    (one per LLM request)
//! └── agent.step        (one per tool execution)
//!     └── agent.hook    (pre/post tool hooks)
//! ```

// ── Attribute keys (OTel semantic conventions + custom) ───────────────────────

/// Unique session identifier for the agent run.
pub const ATTR_SESSION_ID: &str = "agent.session_id";

/// The task description (truncated to 200 chars).
pub const ATTR_TASK: &str = "agent.task";

/// Step number within the agent loop.
pub const ATTR_STEP_NUM: &str = "agent.step_num";

/// Tool name for a tool call span.
pub const ATTR_TOOL_NAME: &str = "tool.name";

/// Whether the tool call succeeded.
pub const ATTR_TOOL_SUCCESS: &str = "tool.success";

/// Whether the tool call was approved by the user.
pub const ATTR_TOOL_APPROVED: &str = "tool.approved";

/// LLM model identifier.
pub const ATTR_LLM_MODEL: &str = "llm.model";

/// Number of messages in the conversation at LLM call time.
pub const ATTR_LLM_MESSAGE_COUNT: &str = "llm.message_count";

/// Hook event name (e.g. "PreToolUse", "PostToolUse", "SessionStart").
pub const ATTR_HOOK_EVENT: &str = "hook.event";

/// Hook decision: "allow", "block", or "inject_context".
pub const ATTR_HOOK_DECISION: &str = "hook.decision";

/// Reason provided with a "block" hook decision.
pub const ATTR_HOOK_BLOCK_REASON: &str = "hook.block_reason";

// ── Span names ─────────────────────────────────────────────────────────────────

pub const SPAN_SESSION: &str = "agent.session";
pub const SPAN_LLM_CALL: &str = "agent.llm_call";
pub const SPAN_STEP: &str = "agent.step";
pub const SPAN_HOOK: &str = "agent.hook";

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate a task description to at most `max_len` chars for use as a span attribute.
pub fn truncate_task(task: &str, max_len: usize) -> &str {
    let end = task
        .char_indices()
        .nth(max_len)
        .map(|(i, _)| i)
        .unwrap_or(task.len());
    &task[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_task_short() {
        assert_eq!(truncate_task("hello", 200), "hello");
    }

    #[test]
    fn truncate_task_long() {
        let s = "x".repeat(300);
        let t = truncate_task(&s, 200);
        assert_eq!(t.len(), 200);
    }

    #[test]
    fn truncate_task_unicode() {
        // "日" is 3 bytes in UTF-8; 5 chars = 15 bytes
        let s = "日本語テスト".repeat(50);
        let t = truncate_task(&s, 10);
        // Should not panic and should be exactly 10 chars
        assert_eq!(t.chars().count(), 10);
    }
}
