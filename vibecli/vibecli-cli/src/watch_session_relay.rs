//! watch_session_relay.rs — Watch-optimised session and sandbox views.
//!
//! The Watch has limited screen space and bandwidth.  This module transforms
//! the full session model into compact representations:
//!
//!  • `WatchSessionSummary`: one-liner per session (id, status, last message preview)
//!  • `WatchMessage`: role + capped content (512 chars) + timestamp
//!  • `WatchSandboxStatus`: container state + CPU/RAM bars + last 5 output lines
//!  • `WatchAgentEvent`: minimal SSE payload (type + delta text) for streaming
//!
//! These compact payloads keep Watch battery and data usage low and are
//! suitable for rendering in a 184×224pt OLED display.

use serde::{Deserialize, Serialize};

// ── Watch-side session summary ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSessionSummary {
    pub session_id: String,
    pub task_preview: String,   // first 80 chars of task
    pub status: String,         // "running" | "complete" | "failed"
    pub provider: String,
    pub model: String,
    pub message_count: u32,
    pub step_count: u32,
    pub started_at: u64,        // Unix secs
    pub last_activity: u64,     // Unix secs
    pub last_message_preview: String, // first 120 chars of last assistant message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchMessage {
    pub id: i64,
    pub role: String,           // "user" | "assistant" | "system" | "tool"
    pub content: String,        // capped at 512 chars; "…" suffix if truncated
    pub created_at: u64,
}

// ── Sandbox status ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSandboxStatus {
    pub container_id: String,
    pub session_id: Option<String>,
    pub state: String,          // "running" | "paused" | "stopped" | "error"
    pub uptime_secs: u64,
    pub cpu_pct: f32,           // 0.0–100.0
    pub mem_mb: u64,
    pub mem_limit_mb: u64,
    pub last_output_lines: Vec<String>, // last 5 lines of stdout/stderr
    pub exit_code: Option<i32>,
}

// ── Streaming event (Watch-optimised SSE payload) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchAgentEvent {
    /// Event kind: "delta" | "tool_start" | "tool_end" | "done" | "error"
    pub kind: String,
    /// Text delta for streaming assistant messages (empty for other kinds)
    pub delta: Option<String>,
    /// Tool name for tool_start/tool_end events
    pub tool: Option<String>,
    /// Final status for "done" events
    pub status: Option<String>,
    /// Error message for "error" events
    pub error: Option<String>,
    /// Step number
    pub step: Option<u32>,
}

// ── Dispatch request from Watch ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchDispatchRequest {
    /// Existing session_id to continue, or None to start a new session
    pub session_id: Option<String>,
    /// Message content (text from voice transcription or keyboard)
    pub content: String,
    /// Optional: preferred provider override
    pub provider: Option<String>,
    /// Nonce to prevent replay (128-bit hex)
    pub nonce: String,
    /// Unix timestamp of the request
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchDispatchResponse {
    pub session_id: String,
    pub message_id: i64,
    pub streaming_url: String, // relative: /watch/stream/{session_id}
}

// ── Control commands for sandbox sessions ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSandboxControlRequest {
    pub action: String, // "pause" | "resume" | "stop" | "restart"
    pub nonce: String,
    pub timestamp: u64,
}

// ── Conversion helpers ────────────────────────────────────────────────────────

/// Truncate string to `max` chars and append "…" if truncated.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max - 1).collect();
        t.push('…');
        t
    }
}

// ── Generic session row (mirrors session_store::SessionRow) ──────────────────

/// Minimal session row accepted by conversion helpers.
/// Matches the fields used by `watch_bridge.rs` to avoid a hard dependency on
/// the binary-only `session_store` module.
pub struct SessionRowView<'a> {
    pub id: &'a str,
    pub task: &'a str,
    pub status: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub step_count: usize,
    pub started_at: u64,
}

pub struct MessageRowView<'a> {
    pub id: i64,
    pub role: &'a str,
    pub content: &'a str,
    pub created_at: u64,
}

/// Convert session + messages into a `WatchSessionSummary`.
pub fn to_watch_summary(session: &SessionRowView<'_>, messages: &[MessageRowView<'_>]) -> WatchSessionSummary {
    let last_message_preview = messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| truncate(m.content, 120))
        .unwrap_or_default();

    WatchSessionSummary {
        session_id: session.id.to_string(),
        task_preview: truncate(session.task, 80),
        status: session.status.to_string(),
        provider: session.provider.to_string(),
        model: session.model.to_string(),
        message_count: messages.len() as u32,
        step_count: session.step_count as u32,
        started_at: session.started_at,
        last_activity: messages
            .last()
            .map(|m| m.created_at)
            .unwrap_or(session.started_at),
        last_message_preview,
    }
}

/// Convert a message row view into a `WatchMessage`.
pub fn to_watch_message(row: &MessageRowView<'_>) -> WatchMessage {
    WatchMessage {
        id: row.id,
        role: row.role.to_string(),
        content: truncate(row.content, 512),
        created_at: row.created_at,
    }
}

/// Convert a raw SSE JSON event payload into a `WatchAgentEvent`.
/// Accepts the JSON as emitted by the existing AgentEventPayload SSE stream.
pub fn to_watch_event_json(payload: &serde_json::Value) -> WatchAgentEvent {
    let kind = payload.get("type")
        .or_else(|| payload.get("kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    match kind {
        "token_delta" | "delta" => WatchAgentEvent {
            kind: "delta".into(),
            delta: payload.get("text").and_then(|v| v.as_str()).map(String::from),
            tool: None, status: None, error: None, step: None,
        },
        "tool_start" => WatchAgentEvent {
            kind: "tool_start".into(),
            delta: None,
            tool: payload.get("name").and_then(|v| v.as_str()).map(String::from),
            status: None, error: None,
            step: payload.get("step").and_then(|v| v.as_u64()).map(|s| s as u32),
        },
        "tool_end" => WatchAgentEvent {
            kind: "tool_end".into(),
            delta: None,
            tool: payload.get("name").and_then(|v| v.as_str()).map(String::from),
            status: Some(
                if payload.get("success").and_then(|v| v.as_bool()).unwrap_or(true) {
                    "ok"
                } else {
                    "err"
                }.into()
            ),
            error: None,
            step: payload.get("step").and_then(|v| v.as_u64()).map(|s| s as u32),
        },
        "done" => WatchAgentEvent {
            kind: "done".into(),
            delta: None, tool: None,
            status: payload.get("status").and_then(|v| v.as_str()).map(String::from),
            error: None, step: None,
        },
        "error" => WatchAgentEvent {
            kind: "error".into(),
            delta: None, tool: None, status: None,
            error: payload.get("message").and_then(|v| v.as_str())
                .map(|m| truncate(m, 200)),
            step: None,
        },
        _ => WatchAgentEvent {
            kind: "info".into(),
            delta: None, tool: None, status: None, error: None, step: None,
        },
    }
}

// ── Nonce registry (replay prevention for dispatch) ──────────────────────────

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
pub struct NonceRegistry(Arc<Mutex<HashMap<String, u64>>>);

impl NonceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns Ok(()) if nonce is fresh and unseen; Err if replay detected.
    pub fn check_and_record(&self, nonce: &str, timestamp: u64) -> anyhow::Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now.saturating_sub(timestamp) > 30 || timestamp > now + 5 {
            anyhow::bail!("Request timestamp out of acceptable window (±30s)");
        }
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // Prune entries older than 60s
        map.retain(|_, ts| now - ts < 60);
        if map.contains_key(nonce) {
            anyhow::bail!("Nonce already used (replay detected)");
        }
        map.insert(nonce.to_string(), timestamp);
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_gets_ellipsis() {
        let s = "a".repeat(100);
        let t = truncate(&s, 10);
        assert_eq!(t.chars().count(), 10);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn nonce_registry_accepts_fresh_nonce() {
        let reg = NonceRegistry::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(reg.check_and_record("abc123", now).is_ok());
    }

    #[test]
    fn nonce_registry_rejects_replay() {
        let reg = NonceRegistry::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        reg.check_and_record("abc123", now).unwrap();
        let err = reg.check_and_record("abc123", now).unwrap_err();
        assert!(err.to_string().contains("Nonce already used"));
    }

    #[test]
    fn nonce_registry_rejects_stale_timestamp() {
        let reg = NonceRegistry::new();
        let stale = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 60;
        let err = reg.check_and_record("fresh_nonce", stale).unwrap_err();
        assert!(err.to_string().contains("timestamp"));
    }

    #[test]
    fn watch_session_summary_serde_roundtrip() {
        let s = WatchSessionSummary {
            session_id: "sess-1".into(),
            task_preview: "Build a CLI".into(),
            status: "running".into(),
            provider: "claude".into(),
            model: "claude-opus-4-6".into(),
            message_count: 5,
            step_count: 3,
            started_at: 1_700_000_000,
            last_activity: 1_700_001_000,
            last_message_preview: "I have completed the task.".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: WatchSessionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "sess-1");
        assert_eq!(back.message_count, 5);
    }

    #[test]
    fn watch_message_caps_at_512_chars() {
        let long = "x".repeat(600);
        let row = MessageRowView {
            id: 1,
            role: "assistant",
            content: &long,
            created_at: 0,
        };
        let wm = to_watch_message(&row);
        assert!(wm.content.chars().count() <= 512);
        assert!(wm.content.ends_with('…'));
    }

    #[test]
    fn watch_agent_event_delta_kind() {
        let payload = serde_json::json!({ "type": "token_delta", "text": "Hello" });
        let ev = to_watch_event_json(&payload);
        assert_eq!(ev.kind, "delta");
        assert_eq!(ev.delta.as_deref(), Some("Hello"));
    }

    #[test]
    fn watch_dispatch_request_serde() {
        let req = WatchDispatchRequest {
            session_id: Some("s1".into()),
            content: "What's the weather?".into(),
            provider: None,
            nonce: "abc".into(),
            timestamp: 1_700_000_000,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: WatchDispatchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.content, "What's the weather?");
    }
}
