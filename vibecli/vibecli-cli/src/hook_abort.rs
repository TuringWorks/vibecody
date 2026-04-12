//! Hook abort logic: exit-code 2 = block, exit-code 0 = allow, custom messages.
//!
//! Claw-code parity Wave 4: implements the hook protocol for PreToolUse /
//! PostToolUse / Notification hooks with structured decision parsing.

use serde::{Deserialize, Serialize};

// ─── Hook Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    Notification,
    PreCompact,
    Stop,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreToolUse   => write!(f, "PreToolUse"),
            Self::PostToolUse  => write!(f, "PostToolUse"),
            Self::Notification => write!(f, "Notification"),
            Self::PreCompact   => write!(f, "PreCompact"),
            Self::Stop         => write!(f, "Stop"),
        }
    }
}

// ─── Hook Exit Codes ─────────────────────────────────────────────────────────

/// The hook protocol exit codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookExitCode {
    /// Allow and continue.
    Allow = 0,
    /// Generic error (non-blocking in most contexts).
    GenericError = 1,
    /// Block: halt the tool call and surface the reason to the agent.
    Block = 2,
}

impl HookExitCode {
    pub fn from_i32(code: i32) -> Self {
        match code { 0 => Self::Allow, 2 => Self::Block, _ => Self::GenericError }
    }

    pub fn is_blocking(&self) -> bool { matches!(self, Self::Block) }
}

// ─── Hook Output ─────────────────────────────────────────────────────────────

/// Parsed output of a hook invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutput {
    pub exit_code: HookExitCode,
    /// Message surfaced to the user / agent (from stderr or JSON `message` field).
    pub message: Option<String>,
    /// Structured JSON decision, if the hook emitted valid JSON.
    pub decision: Option<HookDecision>,
}

/// Structured JSON that a hook can emit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookDecision {
    /// "allow", "block", or "modify".
    pub action: String,
    pub reason: Option<String>,
    /// Agent-visible message.
    pub message: Option<String>,
    /// Whether the agent should retry after the block is explained.
    pub suggest_retry: Option<bool>,
}

impl HookOutput {
    pub fn allow() -> Self {
        Self { exit_code: HookExitCode::Allow, message: None, decision: None }
    }

    pub fn block(message: impl Into<String>) -> Self {
        Self {
            exit_code: HookExitCode::Block,
            message: Some(message.into()),
            decision: Some(HookDecision {
                action: "block".into(), reason: None, message: None, suggest_retry: None,
            }),
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.exit_code.is_blocking()
        || self.decision.as_ref().map(|d| d.action == "block").unwrap_or(false)
    }

    /// Readable reason for the hook's decision.
    pub fn reason(&self) -> Option<&str> {
        self.decision.as_ref()
            .and_then(|d| d.reason.as_deref().or(d.message.as_deref()))
            .or(self.message.as_deref())
    }
}

// ─── Hook Parser ─────────────────────────────────────────────────────────────

pub struct HookParser;

impl HookParser {
    /// Parse hook output from raw exit code + stdout string.
    pub fn parse(exit_code: i32, stdout: &str) -> HookOutput {
        let code = HookExitCode::from_i32(exit_code);
        // Try to parse JSON decision from stdout
        let decision: Option<HookDecision> = serde_json::from_str(stdout.trim()).ok();
        // Determine blocking: exit 2, OR JSON action=block
        let is_json_block = decision.as_ref().map(|d| d.action == "block").unwrap_or(false);
        let effective_code = if is_json_block { HookExitCode::Block } else { code };
        let message = if stdout.trim().starts_with('{') {
            decision.as_ref().and_then(|d| d.message.clone())
        } else if !stdout.trim().is_empty() {
            Some(stdout.trim().to_string())
        } else {
            None
        };
        HookOutput { exit_code: effective_code, message, decision }
    }

    /// Summarise all hook outputs and return the aggregate decision.
    pub fn aggregate(outputs: &[HookOutput]) -> HookOutput {
        if let Some(blocking) = outputs.iter().find(|o| o.is_blocking()) {
            blocking.clone()
        } else {
            HookOutput::allow()
        }
    }
}

// ─── Hook Context ─────────────────────────────────────────────────────────────

/// Context passed to a hook as JSON stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: HookEvent,
    pub session_id: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_output: Option<String>,
}

impl HookContext {
    pub fn pre_tool(session_id: impl Into<String>, tool_name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            event: HookEvent::PreToolUse, session_id: session_id.into(),
            tool_name: Some(tool_name.into()), tool_input: Some(input),
            tool_output: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_0_allows() {
        let out = HookParser::parse(0, "");
        assert!(!out.is_blocking());
        assert_eq!(out.exit_code, HookExitCode::Allow);
    }

    #[test]
    fn test_exit_2_blocks() {
        let out = HookParser::parse(2, "dangerous command");
        assert!(out.is_blocking());
        assert_eq!(out.exit_code, HookExitCode::Block);
    }

    #[test]
    fn test_exit_1_non_blocking() {
        let out = HookParser::parse(1, "warning");
        assert!(!out.is_blocking());
    }

    #[test]
    fn test_plain_text_message_captured() {
        let out = HookParser::parse(2, "This command is blocked for safety.");
        assert_eq!(out.message.as_deref(), Some("This command is blocked for safety."));
    }

    #[test]
    fn test_json_block_decision() {
        let json = r#"{"action":"block","reason":"rm -rf detected","message":"Blocked destructive command"}"#;
        let out = HookParser::parse(0, json); // exit 0 but JSON says block
        assert!(out.is_blocking());
        assert_eq!(out.decision.as_ref().unwrap().action, "block");
    }

    #[test]
    fn test_json_allow_decision() {
        let json = r#"{"action":"allow","reason":"safe read"}"#;
        let out = HookParser::parse(0, json);
        assert!(!out.is_blocking());
    }

    #[test]
    fn test_json_message_captured() {
        let json = r#"{"action":"block","message":"Not allowed"}"#;
        let out = HookParser::parse(0, json);
        assert_eq!(out.message.as_deref(), Some("Not allowed"));
    }

    #[test]
    fn test_aggregate_allows_when_all_allow() {
        let outputs = vec![HookOutput::allow(), HookOutput::allow()];
        let agg = HookParser::aggregate(&outputs);
        assert!(!agg.is_blocking());
    }

    #[test]
    fn test_aggregate_blocks_when_any_blocks() {
        let outputs = vec![HookOutput::allow(), HookOutput::block("denied")];
        let agg = HookParser::aggregate(&outputs);
        assert!(agg.is_blocking());
    }

    #[test]
    fn test_aggregate_returns_first_blocking_message() {
        let outputs = vec![HookOutput::block("reason A"), HookOutput::block("reason B")];
        let agg = HookParser::aggregate(&outputs);
        assert_eq!(agg.message.as_deref(), Some("reason A"));
    }

    #[test]
    fn test_hook_output_reason_from_decision() {
        let out = HookOutput {
            exit_code: HookExitCode::Block,
            message: None,
            decision: Some(HookDecision {
                action: "block".into(), reason: Some("safety".into()), message: None, suggest_retry: None,
            }),
        };
        assert_eq!(out.reason(), Some("safety"));
    }

    #[test]
    fn test_hook_output_reason_falls_back_to_message() {
        let out = HookOutput { exit_code: HookExitCode::Block, message: Some("fallback".into()), decision: None };
        assert_eq!(out.reason(), Some("fallback"));
    }

    #[test]
    fn test_hook_context_pre_tool_json() {
        let ctx = HookContext::pre_tool("s1", "Bash", serde_json::json!({"command":"ls"}));
        let json = ctx.to_json();
        assert!(json.contains("PreToolUse"));
        assert!(json.contains("Bash"));
    }

    #[test]
    fn test_hook_event_display() {
        assert_eq!(HookEvent::PreToolUse.to_string(), "PreToolUse");
        assert_eq!(HookEvent::Stop.to_string(), "Stop");
    }

    #[test]
    fn test_exit_code_from_i32() {
        assert_eq!(HookExitCode::from_i32(0), HookExitCode::Allow);
        assert_eq!(HookExitCode::from_i32(2), HookExitCode::Block);
        assert_eq!(HookExitCode::from_i32(99), HookExitCode::GenericError);
    }

    #[test]
    fn test_is_blocking_false_for_allow() {
        assert!(!HookExitCode::Allow.is_blocking());
    }

    #[test]
    fn test_empty_stdout_no_message() {
        let out = HookParser::parse(0, "   ");
        assert!(out.message.is_none());
    }
}
