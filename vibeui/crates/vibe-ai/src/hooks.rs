//! Hooks system for intercepting and reacting to agent events.
//!
//! Hooks run in response to events like `PreToolUse`, `PostToolUse`, `Stop`,
//! and `TaskCompleted`. Each hook is either a shell command or an LLM eval.
//!
//! Shell command hooks receive a JSON payload on stdin and respond on stdout:
//! - `exit 0` → allow
//! - `exit 2` → block (stderr = reason)
//! - `stdout {"allow": false, "reason": "..."}` → block
//! - `stdout {"context": "..."}` → inject text into next model turn

use crate::provider::{AIProvider, Message, MessageRole};
use crate::tools::{ToolCall, ToolResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

// ── HookEvent ─────────────────────────────────────────────────────────────────

/// Events that hooks can respond to.
#[derive(Debug, Clone, Serialize)]
pub enum HookEvent {
    SessionStart { session_id: String },
    /// Fires before the agent processes the user's prompt. Blocking cancels the task.
    UserPromptSubmit { prompt: String, session_id: String },
    PreToolUse { call: ToolCall, session_id: String },
    PostToolUse { call: ToolCall, result: ToolResult, session_id: String },
    Stop { reason: String, session_id: String },
    TaskCompleted { summary: String, session_id: String },
    SubagentStart { name: String },
}

impl HookEvent {
    /// Canonical event type name used for matching hook configs.
    pub fn type_name(&self) -> &'static str {
        match self {
            HookEvent::SessionStart { .. } => "SessionStart",
            HookEvent::UserPromptSubmit { .. } => "UserPromptSubmit",
            HookEvent::PreToolUse { .. } => "PreToolUse",
            HookEvent::PostToolUse { .. } => "PostToolUse",
            HookEvent::Stop { .. } => "Stop",
            HookEvent::TaskCompleted { .. } => "TaskCompleted",
            HookEvent::SubagentStart { .. } => "SubagentStart",
        }
    }

    /// Tool name for events that carry a tool call (used for tool-name filtering).
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            HookEvent::PreToolUse { call, .. } => Some(call.name()),
            HookEvent::PostToolUse { call, .. } => Some(call.name()),
            _ => None,
        }
    }

    fn session_id(&self) -> &str {
        match self {
            HookEvent::SessionStart { session_id } => session_id,
            HookEvent::UserPromptSubmit { session_id, .. } => session_id,
            HookEvent::PreToolUse { session_id, .. } => session_id,
            HookEvent::PostToolUse { session_id, .. } => session_id,
            HookEvent::Stop { session_id, .. } => session_id,
            HookEvent::TaskCompleted { session_id, .. } => session_id,
            HookEvent::SubagentStart { .. } => "unknown",
        }
    }
}

// ── HookDecision ──────────────────────────────────────────────────────────────

/// What the hook tells the agent to do next.
#[derive(Debug, Clone)]
pub enum HookDecision {
    /// Proceed normally.
    Allow,
    /// Block the tool call with a reason message.
    Block { reason: String },
    /// Inject additional context text back into the model's next turn.
    InjectContext { text: String },
}

// ── HookHandler ───────────────────────────────────────────────────────────────

/// The execution strategy for a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookHandler {
    /// Shell command. Event JSON is piped to stdin; see exit-code/stdout protocol above.
    Command { shell: String },
    /// Single-turn LLM evaluation. The prompt template receives the event JSON.
    /// Expected response: `{"ok": true}` or `{"ok": false, "reason": "..."}`.
    Llm { prompt: String },
}

// ── HookConfig ────────────────────────────────────────────────────────────────

/// A single hook definition. Multiple hooks can be defined in `[[hooks]]` tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Event type that triggers this hook: "PreToolUse", "PostToolUse", "Stop", etc.
    pub event: String,
    /// Optional list of tool names (substring match) to filter.
    /// If absent, the hook fires for all tools on this event.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    /// How to handle the event.
    pub handler: HookHandler,
    /// If true, runs in the background and never blocks the agent.
    #[serde(default, rename = "async")]
    pub async_exec: bool,
}

impl HookConfig {
    /// Returns true if this config applies to the given event.
    pub fn matches(&self, event: &HookEvent) -> bool {
        if self.event != event.type_name() {
            return false;
        }
        if let Some(filter) = &self.tools {
            match event.tool_name() {
                Some(name) => filter.iter().any(|t| name.contains(t.as_str())),
                None => false,
            }
        } else {
            true
        }
    }
}

// ── HookRunner ────────────────────────────────────────────────────────────────

/// Runs all matching hooks for a given event and returns the aggregate decision.
pub struct HookRunner {
    configs: Vec<HookConfig>,
    /// Optional LLM provider for `HookHandler::Llm` hooks.
    llm_provider: Option<Arc<dyn AIProvider>>,
}

impl HookRunner {
    pub fn new(configs: Vec<HookConfig>) -> Self {
        Self { configs, llm_provider: None }
    }

    pub fn empty() -> Self {
        Self { configs: vec![], llm_provider: None }
    }

    /// Attach an LLM provider so `handler = { llm = "..." }` hooks can evaluate.
    pub fn with_llm_provider(mut self, provider: Arc<dyn AIProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    /// Run all matching hooks. Returns the first blocking/injecting decision;
    /// returns `Allow` if all hooks pass.
    pub async fn run(&self, event: &HookEvent) -> HookDecision {
        for config in &self.configs {
            if !config.matches(event) {
                continue;
            }

            if config.async_exec {
                let cfg = config.clone();
                let evt = event.clone();
                let llm = self.llm_provider.clone();
                tokio::spawn(async move {
                    if let Err(e) = exec_handler(&cfg, &evt, llm).await {
                        tracing::warn!("Async hook error: {}", e);
                    }
                });
                continue;
            }

            match exec_handler(config, event, self.llm_provider.clone()).await {
                Ok(HookDecision::Allow) => {} // keep going
                Ok(decision) => return decision,
                Err(e) => {
                    tracing::warn!("Hook '{}' failed: {} — failing open", config.event, e);
                }
            }
        }
        HookDecision::Allow
    }
}

// ── Internal ──────────────────────────────────────────────────────────────────

async fn exec_handler(
    config: &HookConfig,
    event: &HookEvent,
    llm: Option<Arc<dyn AIProvider>>,
) -> Result<HookDecision> {
    match &config.handler {
        HookHandler::Command { shell } => exec_shell_hook(shell, event).await,
        HookHandler::Llm { prompt } => exec_llm_hook(prompt, event, llm).await,
    }
}

async fn exec_llm_hook(
    prompt: &str,
    event: &HookEvent,
    llm: Option<Arc<dyn AIProvider>>,
) -> Result<HookDecision> {
    let provider = match llm {
        Some(p) => p,
        None => {
            tracing::debug!("LLM hook for '{}' skipped — no provider set", event.type_name());
            return Ok(HookDecision::Allow);
        }
    };

    let payload = build_payload(event);
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    let full_prompt = format!("{}\n\nEvent JSON:\n{}", prompt, payload_str);

    let messages = vec![
        Message { role: MessageRole::User, content: full_prompt },
    ];

    match provider.chat(&messages, None).await {
        Ok(response) => {
            #[derive(Deserialize)]
            struct LlmHookResponse { ok: bool, reason: Option<String> }
            // Try to find JSON in the response (model may wrap it in text)
            let json_str = response
                .lines()
                .find(|l| l.trim_start().starts_with('{'))
                .unwrap_or(&response);
            if let Ok(resp) = serde_json::from_str::<LlmHookResponse>(json_str) {
                if !resp.ok {
                    return Ok(HookDecision::Block {
                        reason: resp.reason.unwrap_or_else(|| "LLM hook blocked".to_string()),
                    });
                }
            }
            Ok(HookDecision::Allow)
        }
        Err(e) => {
            tracing::warn!("LLM hook call failed: {} — failing open", e);
            Ok(HookDecision::Allow)
        }
    }
}

async fn exec_shell_hook(shell: &str, event: &HookEvent) -> Result<HookDecision> {
    let payload = build_payload(event);
    let payload_json = serde_json::to_string(&payload)?;

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(shell)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let _ = stdin.write_all(payload_json.as_bytes()).await;
        // stdin dropped here → EOF to child
    }

    let output = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output()).await??;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    // exit 2 = block
    if output.status.code() == Some(2) {
        let reason = if !stderr.is_empty() {
            stderr
        } else {
            stdout.clone()
        };
        return Ok(HookDecision::Block {
            reason: if reason.is_empty() { "Blocked by hook".to_string() } else { reason },
        });
    }

    // Try to parse stdout JSON
    if !stdout.is_empty() {
        if let Ok(resp) = serde_json::from_str::<HookResponse>(&stdout) {
            if resp.allow == Some(false) {
                return Ok(HookDecision::Block {
                    reason: resp.reason.unwrap_or_else(|| "Blocked by hook".to_string()),
                });
            }
            if let Some(ctx) = resp.context {
                return Ok(HookDecision::InjectContext { text: ctx });
            }
        }
    }

    Ok(HookDecision::Allow)
}

/// JSON payload sent to shell hooks via stdin.
#[derive(Debug, Serialize)]
struct HookPayload {
    event: &'static str,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

fn build_payload(event: &HookEvent) -> HookPayload {
    match event {
        HookEvent::UserPromptSubmit { prompt, session_id } => HookPayload {
            event: "UserPromptSubmit",
            session_id: session_id.clone(),
            tool: None,
            input: Some(serde_json::json!({ "prompt": prompt })),
            output: None,
        },
        HookEvent::PreToolUse { call, session_id } => HookPayload {
            event: "PreToolUse",
            session_id: session_id.clone(),
            tool: Some(call.name().to_string()),
            input: serde_json::to_value(call).ok(),
            output: None,
        },
        HookEvent::PostToolUse { call, result, session_id } => HookPayload {
            event: "PostToolUse",
            session_id: session_id.clone(),
            tool: Some(call.name().to_string()),
            input: serde_json::to_value(call).ok(),
            output: Some(result.output.clone()),
        },
        HookEvent::Stop { reason, session_id } => HookPayload {
            event: "Stop",
            session_id: session_id.clone(),
            tool: None,
            input: None,
            output: Some(reason.clone()),
        },
        HookEvent::TaskCompleted { summary, session_id } => HookPayload {
            event: "TaskCompleted",
            session_id: session_id.clone(),
            tool: None,
            input: None,
            output: Some(summary.clone()),
        },
        HookEvent::SessionStart { session_id } => HookPayload {
            event: "SessionStart",
            session_id: session_id.clone(),
            tool: None,
            input: None,
            output: None,
        },
        HookEvent::SubagentStart { name } => HookPayload {
            event: "SubagentStart",
            session_id: "unknown".to_string(),
            tool: None,
            input: Some(serde_json::json!({ "name": name })),
            output: None,
        },
    }
}

/// JSON response read from a shell hook's stdout.
#[derive(Debug, Deserialize)]
struct HookResponse {
    allow: Option<bool>,
    reason: Option<String>,
    context: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolCall;

    fn pre_tool_call() -> HookEvent {
        HookEvent::PreToolUse {
            call: ToolCall::Bash { command: "cargo build".to_string() },
            session_id: "test-session".to_string(),
        }
    }

    #[test]
    fn hook_matches_event_type() {
        let cfg = HookConfig {
            event: "PreToolUse".to_string(),
            tools: None,
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        assert!(cfg.matches(&pre_tool_call()));
    }

    #[test]
    fn hook_matches_tool_filter() {
        let cfg = HookConfig {
            event: "PreToolUse".to_string(),
            tools: Some(vec!["bash".to_string()]),
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        assert!(cfg.matches(&pre_tool_call()));
    }

    #[test]
    fn hook_rejects_wrong_tool_filter() {
        let cfg = HookConfig {
            event: "PreToolUse".to_string(),
            tools: Some(vec!["write_file".to_string()]),
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        assert!(!cfg.matches(&pre_tool_call()));
    }

    #[test]
    fn hook_rejects_wrong_event() {
        let cfg = HookConfig {
            event: "PostToolUse".to_string(),
            tools: None,
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        assert!(!cfg.matches(&pre_tool_call()));
    }

    #[tokio::test]
    async fn empty_runner_returns_allow() {
        let runner = HookRunner::empty();
        let decision = runner.run(&pre_tool_call()).await;
        assert!(matches!(decision, HookDecision::Allow));
    }

    #[tokio::test]
    async fn shell_hook_allow_on_exit_zero() {
        let runner = HookRunner::new(vec![HookConfig {
            event: "PreToolUse".to_string(),
            tools: None,
            handler: HookHandler::Command { shell: "exit 0".to_string() },
            async_exec: false,
        }]);
        let decision = runner.run(&pre_tool_call()).await;
        assert!(matches!(decision, HookDecision::Allow));
    }

    #[tokio::test]
    async fn shell_hook_block_on_exit_two() {
        let runner = HookRunner::new(vec![HookConfig {
            event: "PreToolUse".to_string(),
            tools: None,
            handler: HookHandler::Command {
                shell: "echo 'blocked' >&2; exit 2".to_string(),
            },
            async_exec: false,
        }]);
        let decision = runner.run(&pre_tool_call()).await;
        assert!(matches!(decision, HookDecision::Block { .. }));
    }

    #[tokio::test]
    async fn shell_hook_inject_context() {
        let runner = HookRunner::new(vec![HookConfig {
            event: "PreToolUse".to_string(),
            tools: None,
            handler: HookHandler::Command {
                shell: r#"echo '{"context":"lint passed"}'"#.to_string(),
            },
            async_exec: false,
        }]);
        let decision = runner.run(&pre_tool_call()).await;
        assert!(matches!(decision, HookDecision::InjectContext { .. }));
    }
}
