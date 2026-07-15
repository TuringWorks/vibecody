#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! ACP (Agent Client Protocol) v0.11+ server — JSON-RPC 2.0 over stdio.
//!
//! Zed and JetBrains co-developed ACP as the LSP-equivalent for AI coding
//! agents: any IDE that speaks ACP can drive any agent that speaks ACP,
//! decoupling editors from agents. Q1 2026 saw Zed + JetBrains ship an
//! ACP Registry listing Claude Code, Codex CLI, GitHub Copilot CLI,
//! OpenCode, Gemini CLI as available agents.
//!
//! This module is the dispatcher half of "VibeCLI as an ACP server" —
//! pure functions that turn a JSON-RPC 2.0 envelope (`{"jsonrpc":"2.0",
//! "id":…, "method":…, "params":…}`) into a response payload. The stdin /
//! stdout plumbing (read line-delimited JSON from stdin, write line-
//! delimited responses to stdout) is a thin wrapper that can live in the
//! CLI subcommand or a separate binary.
//!
//! Methods implemented in this slice (A4 of the v13 fitgap):
//!   - `initialize`   — handshake, returns supported protocol version +
//!                      agent capabilities + serverInfo.
//!   - `authenticate` — optional, advertises no auth required.
//!   - `newSession`   — start a new session, return session id.
//!   - `loadSession`  — resume an existing session by id (404 if
//!                      unknown — full resume semantics arrive in a
//!                      follow-up).
//!   - `cancel`       — cancel an in-flight prompt for a session.
//!
//! Out of scope for this slice (intentional, sized for one PR):
//!   - `prompt` and `sessionUpdate` notifications — full agent loop
//!     integration with `provider`, `agent_runtime`, etc. Tracked
//!     separately; the dispatcher knows the methods and rejects them
//!     with `MethodNotImplemented` (-32601 with a clear message) so
//!     hosts can surface the partial-implementation state honestly.
//!
//! State is held in `Arc<Mutex<…>>` so handlers can be cloned without
//! lifetime gymnastics. Sessions are in-memory for this slice; persistence
//! ties into [`crate::session_store`] in a follow-up.

use crate::sync_ext::LockRecover;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Protocol version advertised by the server. Mirrors Zed's ACP v0.11.0
/// header. Bumped lockstep with the `agent-client-protocol` spec; hosts
/// negotiate during `initialize`.
pub const ACP_PROTOCOL_VERSION: &str = "0.11.0";

/// Standard JSON-RPC 2.0 error codes, plus ACP-specific extensions.
pub mod errors {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    /// ACP extension — method known but not yet implemented in this build.
    pub const METHOD_NOT_IMPLEMENTED: i32 = -32001;
    /// ACP extension — session id is well-formed but unknown.
    pub const SESSION_NOT_FOUND: i32 = -32002;
}

/// JSON-RPC 2.0 envelope as it arrives over stdin.
#[derive(Debug, Clone, Deserialize)]
pub struct AcpRequest {
    pub jsonrpc: String,
    /// Notifications omit `id`. Requests carry one.
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 success or error envelope. Use [`AcpResponse::ok`] /
/// [`AcpResponse::err`] rather than constructing directly.
#[derive(Debug, Clone, Serialize)]
pub struct AcpResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AcpResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }
    pub fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(AcpError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Server-side state. Sessions are kept in-memory for this slice; full
/// persistence ties into `session_store.rs` and arrives with the
/// `prompt` slice.
#[derive(Debug, Clone, Default)]
pub struct AcpSession {
    pub id: String,
    pub mode: String,
}

/// In-memory ACP server. Cheap to clone (Arc<Mutex>) so handlers can
/// capture it without lifetime gymnastics.
#[derive(Debug, Clone, Default)]
pub struct AcpServer {
    state: Arc<Mutex<AcpServerState>>,
}

#[derive(Debug, Default)]
struct AcpServerState {
    /// session id → session
    sessions: BTreeMap<String, AcpSession>,
    /// monotonic counter feeding session-id generation
    next_session_seq: u64,
}

impl AcpServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience for tests: how many sessions are currently held.
    pub fn session_count(&self) -> usize {
        self.state.lock_recover().sessions.len()
    }

    /// Top-level dispatch — turn an `AcpRequest` into the response
    /// envelope. Notifications (no `id`) return `Ok(None)`; requests
    /// always return a response. Parse errors are the caller's
    /// responsibility (see [`parse_request`]).
    pub fn dispatch(&self, req: AcpRequest) -> Result<Option<AcpResponse>> {
        // Notifications carry no id and expect no reply (per JSON-RPC 2.0
        // §4.1). The handler still runs for side effects, but we drop
        // the response.
        let id = match req.id.clone() {
            Some(id) => id,
            None => {
                let _ = self.run_method(&req.method, req.params);
                return Ok(None);
            }
        };

        let result = match self.run_method(&req.method, req.params) {
            Ok(value) => AcpResponse::ok(id, value),
            Err(handler_err) => match handler_err {
                HandlerError::MethodNotFound => AcpResponse::err(
                    id,
                    errors::METHOD_NOT_FOUND,
                    format!("Method not found: {}", req.method),
                ),
                HandlerError::MethodNotImplemented(msg) => {
                    AcpResponse::err(id, errors::METHOD_NOT_IMPLEMENTED, msg)
                }
                HandlerError::SessionNotFound(sid) => AcpResponse::err(
                    id,
                    errors::SESSION_NOT_FOUND,
                    format!("Session not found: {sid}"),
                ),
                HandlerError::InvalidParams(msg) => {
                    AcpResponse::err(id, errors::INVALID_PARAMS, msg)
                }
                HandlerError::Internal(msg) => AcpResponse::err(id, errors::INTERNAL_ERROR, msg),
            },
        };
        Ok(Some(result))
    }

    fn run_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        match method {
            "initialize" => self.handle_initialize(params),
            "authenticate" => self.handle_authenticate(params),
            "newSession" => self.handle_new_session(params),
            "loadSession" => self.handle_load_session(params),
            "cancel" => self.handle_cancel(params),
            // Methods we know about but haven't shipped yet — surface
            // partial-implementation honestly so the host doesn't think
            // a network error happened.
            "prompt" | "setSessionMode" => Err(HandlerError::MethodNotImplemented(format!(
                "ACP method '{method}' is recognised but not yet implemented in this build"
            ))),
            _ => Err(HandlerError::MethodNotFound),
        }
    }

    /// `initialize` handshake — server advertises its capabilities.
    pub fn handle_initialize(
        &self,
        _params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        Ok(json!({
            "protocolVersion": ACP_PROTOCOL_VERSION,
            "agentCapabilities": {
                "loadSession": true,
                "promptCapabilities": {
                    "image": false,
                    "audio": false,
                    "embeddedContext": true,
                },
                "mcpCapabilities": {
                    "http": true,
                    "sse": true,
                },
            },
            "serverInfo": {
                "name": "vibecli",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }))
    }

    /// `authenticate` — VibeCLI requires no auth; advertise so hosts
    /// can skip the prompt.
    pub fn handle_authenticate(
        &self,
        _params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        Ok(json!({ "authenticated": true }))
    }

    /// `newSession` — create a new session and return its id.
    pub fn handle_new_session(
        &self,
        _params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| HandlerError::Internal(format!("session lock poisoned: {e}")))?;
        state.next_session_seq += 1;
        let id = format!("vibecli-acp-{:016x}", state.next_session_seq);
        state.sessions.insert(
            id.clone(),
            AcpSession {
                id: id.clone(),
                mode: "default".to_string(),
            },
        );
        Ok(json!({ "sessionId": id }))
    }

    /// `loadSession` — return existing session or SESSION_NOT_FOUND.
    pub fn handle_load_session(
        &self,
        params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        let sid = params
            .as_ref()
            .and_then(|p| p.get("sessionId"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| HandlerError::InvalidParams("loadSession requires sessionId".into()))?
            .to_string();
        let state = self
            .state
            .lock()
            .map_err(|e| HandlerError::Internal(format!("session lock poisoned: {e}")))?;
        if !state.sessions.contains_key(&sid) {
            return Err(HandlerError::SessionNotFound(sid));
        }
        Ok(json!({ "sessionId": sid }))
    }

    /// `cancel` — best-effort cancel for an in-flight prompt. Stub
    /// returns success; the prompt slice will hook this into the agent
    /// loop's cancellation token.
    pub fn handle_cancel(
        &self,
        _params: Option<Value>,
    ) -> std::result::Result<Value, HandlerError> {
        Ok(json!({ "cancelled": true }))
    }
}

/// Internal handler error — translated into a JSON-RPC error envelope
/// at the dispatch boundary.
#[derive(Debug)]
pub enum HandlerError {
    MethodNotFound,
    MethodNotImplemented(String),
    SessionNotFound(String),
    InvalidParams(String),
    Internal(String),
}

/// Parse a single JSON-RPC line from stdin. Returns the typed envelope
/// on success or a pre-formed parse-error response on failure (so the
/// caller can write it directly without further translation).
pub fn parse_request(line: &str) -> std::result::Result<AcpRequest, AcpResponse> {
    match serde_json::from_str::<AcpRequest>(line) {
        Ok(req) if req.jsonrpc == "2.0" => Ok(req),
        Ok(_) => Err(AcpResponse::err(
            json!(null),
            errors::INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        )),
        Err(e) => Err(AcpResponse::err(
            json!(null),
            errors::PARSE_ERROR,
            format!("parse: {e}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(id: i64, method: &str, params: Value) -> AcpRequest {
        AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: method.into(),
            params: Some(params),
        }
    }

    // ── Scenario 1: initialize handshake ─────────────────────────────────────

    #[test]
    fn initialize_returns_protocol_version_and_capabilities() {
        let s = AcpServer::new();
        let resp = s
            .dispatch(req(1, "initialize", json!({})))
            .unwrap()
            .expect("initialize must produce a response");
        assert_eq!(resp.id, json!(1));
        let result = resp.result.expect("initialize must succeed");
        assert_eq!(result["protocolVersion"], ACP_PROTOCOL_VERSION);
        assert!(
            result["agentCapabilities"].is_object(),
            "agentCapabilities required: {result}"
        );
        assert!(
            result["serverInfo"].is_object(),
            "serverInfo required: {result}"
        );
        assert_eq!(result["serverInfo"]["name"], "vibecli");
    }

    // ── Scenario 2: newSession returns a fresh id ────────────────────────────

    #[test]
    fn new_session_returns_unique_session_id() {
        let s = AcpServer::new();
        let r1 = s
            .dispatch(req(2, "newSession", json!({})))
            .unwrap()
            .expect("newSession must respond");
        let r2 = s
            .dispatch(req(3, "newSession", json!({})))
            .unwrap()
            .expect("newSession must respond");
        let id1 = r1.result.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        let id2 = r2.result.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2, "session ids must be unique");
        assert_eq!(s.session_count(), 2);
    }

    // ── Scenario 3: loadSession on unknown id returns SESSION_NOT_FOUND ─────

    #[test]
    fn load_session_with_unknown_id_returns_session_not_found() {
        let s = AcpServer::new();
        let resp = s
            .dispatch(req(
                4,
                "loadSession",
                json!({"sessionId": "does-not-exist"}),
            ))
            .unwrap()
            .expect("loadSession must respond");
        let err = resp.error.expect("must be an error");
        assert_eq!(err.code, errors::SESSION_NOT_FOUND);
        assert!(
            err.message.to_lowercase().contains("session"),
            "error message should mention session: {}",
            err.message
        );
    }

    // ── Scenario 4: unknown method returns METHOD_NOT_FOUND ──────────────────

    #[test]
    fn unknown_method_returns_method_not_found() {
        let s = AcpServer::new();
        let resp = s
            .dispatch(req(5, "totally-not-a-real-method", json!({})))
            .unwrap()
            .expect("unknown method must produce a response");
        let err = resp.error.expect("must be an error");
        assert_eq!(err.code, errors::METHOD_NOT_FOUND);
    }

    // ── Scenario 5: malformed JSON parses to a PARSE_ERROR response ─────────

    #[test]
    fn malformed_json_yields_parse_error_response() {
        let result = parse_request("{not valid json");
        let resp = result.expect_err("must error");
        let err = resp.error.expect("must be an error");
        assert_eq!(err.code, errors::PARSE_ERROR);
        assert_eq!(resp.id, json!(null), "parse-error response carries null id");
    }

    // ── Scenario 6: jsonrpc != \"2.0\" rejects with INVALID_REQUEST ─────────

    #[test]
    fn wrong_jsonrpc_version_rejected_with_invalid_request() {
        let result = parse_request(r#"{"jsonrpc":"1.0","id":1,"method":"initialize"}"#);
        let resp = result.expect_err("must error");
        let err = resp.error.expect("must be an error");
        assert_eq!(err.code, errors::INVALID_REQUEST);
    }
}
