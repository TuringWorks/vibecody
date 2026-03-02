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
    /// Fires when a file is written (e.g. by the agent's WriteFile tool or by the editor save).
    FileSaved { path: String, content: String, language: String },
    /// Fires when a new file is created.
    FileCreated { path: String },
    /// Fires when a file is deleted.
    FileDeleted { path: String },
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
            HookEvent::FileSaved { .. } => "FileSaved",
            HookEvent::FileCreated { .. } => "FileCreated",
            HookEvent::FileDeleted { .. } => "FileDeleted",
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

    /// File path for file-event hooks (used for path-glob filtering).
    pub fn file_path(&self) -> Option<&str> {
        match self {
            HookEvent::FileSaved { path, .. } => Some(path.as_str()),
            HookEvent::FileCreated { path } => Some(path.as_str()),
            HookEvent::FileDeleted { path } => Some(path.as_str()),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn session_id(&self) -> &str {
        match self {
            HookEvent::SessionStart { session_id } => session_id,
            HookEvent::UserPromptSubmit { session_id, .. } => session_id,
            HookEvent::PreToolUse { session_id, .. } => session_id,
            HookEvent::PostToolUse { session_id, .. } => session_id,
            HookEvent::Stop { session_id, .. } => session_id,
            HookEvent::TaskCompleted { session_id, .. } => session_id,
            HookEvent::SubagentStart { .. } => "unknown",
            HookEvent::FileSaved { .. } => "file",
            HookEvent::FileCreated { .. } => "file",
            HookEvent::FileDeleted { .. } => "file",
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
    /// Event type that triggers this hook: "PreToolUse", "PostToolUse", "FileSaved", etc.
    pub event: String,
    /// Optional list of tool names (substring match) to filter PreToolUse/PostToolUse hooks.
    /// If absent, the hook fires for all tools on this event.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    /// Optional glob patterns for file paths (used by FileSaved/FileCreated/FileDeleted hooks).
    /// Example: `["**/*.rs", "src/**/*.ts"]`
    /// If absent, the hook fires for all paths.
    #[serde(default)]
    pub paths: Option<Vec<String>>,
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
        // Tool name filter (for PreToolUse / PostToolUse)
        if let Some(filter) = &self.tools {
            match event.tool_name() {
                Some(name) => {
                    if !filter.iter().any(|t| name.contains(t.as_str())) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        // Path glob filter (for FileSaved / FileCreated / FileDeleted)
        if let Some(path_filters) = &self.paths {
            match event.file_path() {
                Some(p) => {
                    if !path_filters.iter().any(|pat| glob_match_path(pat, p)) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
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
        HookEvent::FileSaved { path, content, language } => HookPayload {
            event: "FileSaved",
            session_id: "file".to_string(),
            tool: None,
            input: Some(serde_json::json!({
                "path": path,
                "language": language,
                "content_len": content.len()
            })),
            output: None,
        },
        HookEvent::FileCreated { path } => HookPayload {
            event: "FileCreated",
            session_id: "file".to_string(),
            tool: None,
            input: Some(serde_json::json!({ "path": path })),
            output: None,
        },
        HookEvent::FileDeleted { path } => HookPayload {
            event: "FileDeleted",
            session_id: "file".to_string(),
            tool: None,
            input: Some(serde_json::json!({ "path": path })),
            output: None,
        },
    }
}

// ── Path glob matching ─────────────────────────────────────────────────────────

/// Match a glob pattern against a file path.
/// Supports `**` (any path segments) and `*` (within a single segment).
fn glob_match_path(pattern: &str, path: &str) -> bool {
    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();
    glob_parts(&pat_parts, &path_parts)
}

fn glob_parts(pat: &[&str], path: &[&str]) -> bool {
    match (pat.first(), path.first()) {
        (None, None) => true,
        (None, _) => false,
        (Some(&"**"), _) => {
            // ** matches 0 or more path segments
            for i in 0..=path.len() {
                if glob_parts(&pat[1..], &path[i..]) {
                    return true;
                }
            }
            false
        }
        (_, None) => pat.iter().all(|p| *p == "**"),
        (Some(p), Some(s)) => {
            if segment_match(p, s) {
                glob_parts(&pat[1..], &path[1..])
            } else {
                false
            }
        }
    }
}

/// Match a single path segment against a pattern (supports `*` wildcard within segment).
fn segment_match(pattern: &str, s: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == s;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !s.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            return s[pos..].ends_with(part);
        } else if let Some(found) = s[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
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
            paths: None,
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
            paths: None,
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
            paths: None,
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
            paths: None,
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
            paths: None,
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
            paths: None,
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
            paths: None,
            handler: HookHandler::Command {
                shell: r#"echo '{"context":"lint passed"}'"#.to_string(),
            },
            async_exec: false,
        }]);
        let decision = runner.run(&pre_tool_call()).await;
        assert!(matches!(decision, HookDecision::InjectContext { .. }));
    }

    // ── type_name tests ───────────────────────────────────────────────────

    #[test]
    fn type_name_session_start() {
        let e = HookEvent::SessionStart { session_id: "s".into() };
        assert_eq!(e.type_name(), "SessionStart");
    }

    #[test]
    fn type_name_user_prompt_submit() {
        let e = HookEvent::UserPromptSubmit { prompt: "hi".into(), session_id: "s".into() };
        assert_eq!(e.type_name(), "UserPromptSubmit");
    }

    #[test]
    fn type_name_stop() {
        let e = HookEvent::Stop { reason: "done".into(), session_id: "s".into() };
        assert_eq!(e.type_name(), "Stop");
    }

    #[test]
    fn type_name_task_completed() {
        let e = HookEvent::TaskCompleted { summary: "ok".into(), session_id: "s".into() };
        assert_eq!(e.type_name(), "TaskCompleted");
    }

    #[test]
    fn type_name_subagent_start() {
        let e = HookEvent::SubagentStart { name: "worker".into() };
        assert_eq!(e.type_name(), "SubagentStart");
    }

    #[test]
    fn type_name_file_saved() {
        let e = HookEvent::FileSaved { path: "/a.rs".into(), content: "c".into(), language: "rust".into() };
        assert_eq!(e.type_name(), "FileSaved");
    }

    #[test]
    fn type_name_file_created() {
        let e = HookEvent::FileCreated { path: "/a.rs".into() };
        assert_eq!(e.type_name(), "FileCreated");
    }

    #[test]
    fn type_name_file_deleted() {
        let e = HookEvent::FileDeleted { path: "/a.rs".into() };
        assert_eq!(e.type_name(), "FileDeleted");
    }

    // ── tool_name tests ───────────────────────────────────────────────────

    #[test]
    fn tool_name_pre_tool_use() {
        let e = pre_tool_call();
        assert!(e.tool_name().is_some());
        assert_eq!(e.tool_name().unwrap(), "bash");
    }

    #[test]
    fn tool_name_none_for_non_tool_events() {
        let e = HookEvent::SessionStart { session_id: "s".into() };
        assert!(e.tool_name().is_none());
    }

    #[test]
    fn tool_name_post_tool_use() {
        let e = HookEvent::PostToolUse {
            call: ToolCall::Bash { command: "ls".to_string() },
            result: crate::tools::ToolResult { tool_name: "bash".into(), output: "ok".into(), success: true, truncated: false },
            session_id: "s".into(),
        };
        assert_eq!(e.tool_name(), Some("bash"));
    }

    // ── file_path tests ───────────────────────────────────────────────────

    #[test]
    fn file_path_file_saved() {
        let e = HookEvent::FileSaved { path: "/src/main.rs".into(), content: "".into(), language: "rust".into() };
        assert_eq!(e.file_path(), Some("/src/main.rs"));
    }

    #[test]
    fn file_path_file_created() {
        let e = HookEvent::FileCreated { path: "/new.ts".into() };
        assert_eq!(e.file_path(), Some("/new.ts"));
    }

    #[test]
    fn file_path_file_deleted() {
        let e = HookEvent::FileDeleted { path: "/old.py".into() };
        assert_eq!(e.file_path(), Some("/old.py"));
    }

    #[test]
    fn file_path_none_for_non_file_events() {
        let e = HookEvent::Stop { reason: "done".into(), session_id: "s".into() };
        assert!(e.file_path().is_none());
    }

    // ── HookRunner::is_empty ──────────────────────────────────────────────

    #[test]
    fn runner_is_empty() {
        assert!(HookRunner::empty().is_empty());
    }

    #[test]
    fn runner_not_empty() {
        let runner = HookRunner::new(vec![HookConfig {
            event: "PreToolUse".to_string(),
            tools: None,
            paths: None,
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        }]);
        assert!(!runner.is_empty());
    }

    // ── HookConfig::matches with path filter ──────────────────────────────

    #[test]
    fn hook_matches_path_filter() {
        let cfg = HookConfig {
            event: "FileSaved".to_string(),
            tools: None,
            paths: Some(vec!["**/*.rs".to_string()]),
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        let event = HookEvent::FileSaved {
            path: "src/main.rs".into(),
            content: "".into(),
            language: "rust".into(),
        };
        assert!(cfg.matches(&event));
    }

    #[test]
    fn hook_rejects_non_matching_path() {
        let cfg = HookConfig {
            event: "FileSaved".to_string(),
            tools: None,
            paths: Some(vec!["**/*.rs".to_string()]),
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        let event = HookEvent::FileSaved {
            path: "src/app.ts".into(),
            content: "".into(),
            language: "ts".into(),
        };
        assert!(!cfg.matches(&event));
    }

    #[test]
    fn hook_path_filter_on_non_file_event_returns_false() {
        let cfg = HookConfig {
            event: "SessionStart".to_string(),
            tools: None,
            paths: Some(vec!["**/*.rs".to_string()]),
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: false,
        };
        let event = HookEvent::SessionStart { session_id: "s".into() };
        assert!(!cfg.matches(&event));
    }

    // ── glob_match_path tests ──────────────────────────────────────────────

    #[test]
    fn glob_match_double_star() {
        assert!(glob_match_path("**/*.rs", "src/main.rs"));
        assert!(glob_match_path("**/*.rs", "a/b/c/lib.rs"));
        assert!(!glob_match_path("**/*.rs", "src/main.ts"));
    }

    #[test]
    fn glob_match_single_star() {
        assert!(glob_match_path("src/*", "src/main.rs"));
        assert!(!glob_match_path("src/*", "src/sub/main.rs"));
    }

    #[test]
    fn glob_match_exact() {
        assert!(glob_match_path("Makefile", "Makefile"));
        assert!(!glob_match_path("Makefile", "Rakefile"));
    }

    #[test]
    fn glob_match_no_pattern() {
        assert!(glob_match_path("**", "anything/at/all"));
    }

    // ── segment_match tests ────────────────────────────────────────────────

    #[test]
    fn segment_match_star_matches_any() {
        assert!(segment_match("*", "anything"));
        assert!(segment_match("*", ""));
    }

    #[test]
    fn segment_match_exact() {
        assert!(segment_match("foo", "foo"));
        assert!(!segment_match("foo", "bar"));
    }

    #[test]
    fn segment_match_prefix_star() {
        assert!(segment_match("*.rs", "main.rs"));
        assert!(!segment_match("*.rs", "main.ts"));
    }

    #[test]
    fn segment_match_suffix_star() {
        assert!(segment_match("test_*", "test_foo"));
        assert!(!segment_match("test_*", "prod_foo"));
    }

    // ── HookHandler serde ──────────────────────────────────────────────────

    #[test]
    fn hook_handler_command_serde() {
        let handler = HookHandler::Command { shell: "echo hi".to_string() };
        let json = serde_json::to_string(&handler).unwrap();
        let back: HookHandler = serde_json::from_str(&json).unwrap();
        if let HookHandler::Command { shell } = back {
            assert_eq!(shell, "echo hi");
        } else {
            panic!("expected Command");
        }
    }

    #[test]
    fn hook_handler_llm_serde() {
        let handler = HookHandler::Llm { prompt: "check safety".to_string() };
        let json = serde_json::to_string(&handler).unwrap();
        let back: HookHandler = serde_json::from_str(&json).unwrap();
        if let HookHandler::Llm { prompt } = back {
            assert_eq!(prompt, "check safety");
        } else {
            panic!("expected Llm");
        }
    }

    // ── HookConfig serde ───────────────────────────────────────────────────

    #[test]
    fn hook_config_serde_roundtrip() {
        let cfg = HookConfig {
            event: "PreToolUse".to_string(),
            tools: Some(vec!["bash".to_string()]),
            paths: None,
            handler: HookHandler::Command { shell: "true".to_string() },
            async_exec: true,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: HookConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event, "PreToolUse");
        assert!(back.async_exec);
        assert_eq!(back.tools.unwrap(), vec!["bash"]);
    }

    // ── build_payload tests ────────────────────────────────────────────────

    #[test]
    fn build_payload_pre_tool_use() {
        let event = pre_tool_call();
        let payload = build_payload(&event);
        assert_eq!(payload.event, "PreToolUse");
        assert_eq!(payload.tool, Some("bash".to_string()));
        assert_eq!(payload.session_id, "test-session");
    }

    #[test]
    fn build_payload_file_saved() {
        let event = HookEvent::FileSaved {
            path: "/a.rs".into(),
            content: "hello".into(),
            language: "rust".into(),
        };
        let payload = build_payload(&event);
        assert_eq!(payload.event, "FileSaved");
        assert_eq!(payload.session_id, "file");
        let input = payload.input.unwrap();
        assert_eq!(input["path"], "/a.rs");
        assert_eq!(input["content_len"], 5);
    }

    #[test]
    fn build_payload_subagent_start() {
        let event = HookEvent::SubagentStart { name: "worker".into() };
        let payload = build_payload(&event);
        assert_eq!(payload.event, "SubagentStart");
        assert_eq!(payload.session_id, "unknown");
    }
}
