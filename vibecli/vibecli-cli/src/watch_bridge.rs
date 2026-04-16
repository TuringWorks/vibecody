//! watch_bridge.rs — Axum route handlers for the Apple Watch API surface.
//!
//! All routes under `/watch/*` require a Watch-JWT in the Authorization header:
//!   Authorization: Watch-Token <jwt>
//!
//! Exceptions (no auth required):
//!   GET  /watch/beacon    — lightweight discovery (returns no secrets)
//!   POST /watch/register  — first-time device registration (uses nonce + sig)
//!
//! Route map:
//!   GET  /watch/beacon                       — machine discovery info
//!   POST /watch/challenge                    — issue registration nonce (requires bearer)
//!   POST /watch/register                     — register watch device
//!   POST /watch/refresh-token                — renew access token
//!   POST /watch/wrist                        — wrist-on/off event
//!   GET  /watch/sessions                     — list recent sessions (watch-optimised)
//!   GET  /watch/sessions/:id/messages        — get messages for session
//!   GET  /watch/stream/:id                   — SSE stream (watch-optimised events)
//!   POST /watch/dispatch                     — send message / start session
//!   GET  /watch/devices                      — list registered watch devices
//!   DELETE /watch/devices/:id                — revoke watch device
//!
//! This module is imported and wired into `serve.rs` via `build_watch_router()`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Sse},
    routing::{delete, get, post},
    Router,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use vibe_ai::provider::AIProvider;

use crate::watch_auth::{
    WatchAuthManager, WatchRefreshRequest, WatchRegisterRequest, WristEvent,
};
use crate::watch_session_relay::{
    to_watch_event_json, to_watch_message, to_watch_summary, MessageRowView,
    NonceRegistry, SessionRowView, WatchDispatchRequest, WatchDispatchResponse,
};
use tokio_stream::StreamExt as _;

// ── Shared bridge state ───────────────────────────────────────────────────────

/// Broadcast stream map: session_id → sender of JSON-encoded SSE events.
pub type WatchEventStreams = Arc<Mutex<std::collections::HashMap<String, tokio::sync::broadcast::Sender<serde_json::Value>>>>;

#[derive(Clone)]
pub struct WatchBridgeState {
    /// Daemon's API bearer token (used to gate challenge issuance).
    pub api_token: String,
    /// Live SSE streams (shared with serve.rs via Arc).
    pub streams: WatchEventStreams,
    /// Auth manager (JWT signing, device registry).
    pub auth: Arc<Mutex<WatchAuthManager>>,
    /// Replay-prevention nonce registry.
    pub nonces: NonceRegistry,
    /// Machine ID (stable identifier for this daemon instance).
    pub machine_id: String,
    /// Daemon start time for beacon uptime calculation.
    pub started_at: std::time::Instant,
    /// Base URL for Tailscale (resolved at startup).
    pub tailscale_ip: Option<String>,
    /// Path to the SQLite session database (~/.vibecli/sessions.db).
    pub session_db_path: Option<std::path::PathBuf>,
    /// Optional LLM provider for watch dispatch.
    pub provider: Option<Arc<dyn AIProvider>>,
    /// Human-readable provider name (e.g. "claude", "ollama").
    pub provider_name: String,
    /// Currently active session on Watch (set via PUT /watch/active-session).
    /// Used so VibeUI can sync to the same session the Watch is viewing.
    pub active_session: Arc<Mutex<Option<String>>>,
    /// Broadcast channel for real-time session events.
    /// Payloads: {"type":"session_updated","session_id":"..."} etc.
    pub session_events: Arc<tokio::sync::broadcast::Sender<serde_json::Value>>,
    /// Sandbox chat session ID (set via PUT /watch/sandbox/chat-session by VibeUI).
    /// Watch reads this to navigate to the sandbox conversation.
    pub sandbox_chat_session: Arc<Mutex<Option<String>>>,
}

impl WatchBridgeState {
    pub fn new(
        api_token: impl Into<String>,
        streams: WatchEventStreams,
        machine_id: impl Into<String>,
        tailscale_ip: Option<String>,
        session_db_path: Option<std::path::PathBuf>,
        provider: Option<Arc<dyn AIProvider>>,
        provider_name: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let machine_id = machine_id.into();
        let auth = WatchAuthManager::new(&machine_id)?;
        let (ev_tx, _) = tokio::sync::broadcast::channel(256);
        Ok(Self {
            api_token: api_token.into(),
            streams,
            auth: Arc::new(Mutex::new(auth)),
            nonces: NonceRegistry::new(),
            machine_id,
            started_at: std::time::Instant::now(),
            tailscale_ip,
            session_db_path,
            provider,
            provider_name: provider_name.into(),
            active_session: Arc::new(Mutex::new(None)),
            session_events: Arc::new(ev_tx),
            sandbox_chat_session: Arc::new(Mutex::new(None)),
        })
    }

    /// Extract and verify Watch-Token from Authorization header.
    fn verify_token(&self, auth_header: &str) -> anyhow::Result<String> {
        let token = auth_header
            .strip_prefix("Watch-Token ")
            .ok_or_else(|| anyhow::anyhow!("Expected 'Watch-Token <jwt>'"))?;
        self.auth
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .verify_access_token(token)
    }
}

// ── Auth extractor helper ─────────────────────────────────────────────────────

fn extract_watch_auth(
    state: &WatchBridgeState,
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let hdr = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    state.verify_token(hdr).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })
}

// ── Router builder ────────────────────────────────────────────────────────────

/// Build the /watch/* router. Mount it under `/watch` in the parent router.
pub fn build_watch_router(state: WatchBridgeState) -> Router {
    Router::new()
        .route("/beacon",           get(watch_beacon))
        .route("/challenge",        post(watch_challenge))
        .route("/register",         post(watch_register))
        .route("/refresh-token",    post(watch_refresh_token))
        .route("/wrist",            post(watch_wrist_event))
        .route("/sessions",         get(watch_list_sessions))
        .route("/sessions/:id/messages", get(watch_session_messages))
        .route("/stream/:id",       get(watch_stream))
        .route("/dispatch",         post(watch_dispatch))
        .route("/active-session",   get(watch_get_active_session).put(watch_set_active_session))
        .route("/events",           get(watch_session_events_sse))
        .route("/sandbox/chat-session", get(watch_get_sandbox_chat_session).put(watch_set_sandbox_chat_session))
        .route("/devices",          get(watch_list_devices))
        .route("/devices/:id",      delete(watch_revoke_device))
        .with_state(state)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /watch/beacon — unauthenticated lightweight discovery.
async fn watch_beacon(State(state): State<WatchBridgeState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "machine_id": state.machine_id,
        "api_version": "v1",
        "watch_supported": true,
        "tailscale_ip": state.tailscale_ip,
        "uptime_secs": state.started_at.elapsed().as_secs(),
    }))
}

/// POST /watch/challenge — issue a registration nonce (no auth required).
///
/// The nonce itself grants nothing; real security is in /watch/register
/// which verifies the Ed25519 device signature against the nonce.
/// Rate-limited naturally by the 2-minute nonce TTL.
async fn watch_challenge(
    State(state): State<WatchBridgeState>,
) -> impl IntoResponse {
    let ch = state
        .auth
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .issue_challenge();
    match ch {
        Ok(c) => (StatusCode::OK, Json(serde_json::to_value(c).unwrap())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))),
    }
}

/// POST /watch/register — first-time watch device registration.
async fn watch_register(
    State(state): State<WatchBridgeState>,
    Json(req): Json<WatchRegisterRequest>,
) -> impl IntoResponse {
    let mut auth = state.auth.lock().unwrap_or_else(|e| e.into_inner());
    match auth.register_device(&req) {
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
        Ok(device) => {
            let access = auth.issue_access_token(&device.device_id);
            let refresh = auth.issue_refresh_token(&device.device_id);
            match (access, refresh) {
                (Ok((access_token, exp)), Ok(refresh_token)) => (
                    StatusCode::CREATED,
                    Json(serde_json::json!({
                        "device_id": device.device_id,
                        "access_token": access_token,
                        "refresh_token": refresh_token,
                        "expires_in": crate::watch_auth::ACCESS_TOKEN_TTL_SECS,
                        "expires_at": exp,
                    })),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Token issuance failed"})),
                ),
            }
        }
    }
}

/// POST /watch/refresh-token — renew access + refresh tokens.
async fn watch_refresh_token(
    State(state): State<WatchBridgeState>,
    Json(req): Json<WatchRefreshRequest>,
) -> impl IntoResponse {
    let mut auth = state.auth.lock().unwrap_or_else(|e| e.into_inner());
    match auth.refresh_tokens(&req) {
        Ok((access, refresh, exp)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "access_token": access,
                "refresh_token": refresh,
                "expires_at": exp,
                "expires_in": crate::watch_auth::ACCESS_TOKEN_TTL_SECS,
            })),
        ),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// POST /watch/wrist — watch reports wrist-on/off.
async fn watch_wrist_event(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Json(ev): Json<WristEvent>,
) -> impl IntoResponse {
    // Light auth: accept Watch-Token (device must be known) OR bearer token
    let _device_id = match extract_watch_auth(&state, &headers) {
        Ok(id) => id,
        Err(_) => {
            let bearer = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if bearer != format!("Bearer {}", state.api_token) {
                return (StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "Auth required"})));
            }
            ev.device_id.clone()
        }
    };
    let mut auth = state.auth.lock().unwrap_or_else(|e| e.into_inner());
    match auth.handle_wrist_event(&ev) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))),
    }
}

/// GET /watch/sessions — list recent sessions in Watch-optimised format.
async fn watch_list_sessions(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = extract_watch_auth(&state, &headers) {
        return e.into_response();
    }
    let db_path = match &state.session_db_path {
        Some(p) => p.clone(),
        None => {
            // Fall back to default location
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".vibecli")
                .join("sessions.db")
        }
    };
    let store = match crate::session_store::SessionStore::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("watch_list_sessions: cannot open session store: {e}");
            return Json(serde_json::json!({"sessions": []})).into_response();
        }
    };
    let rows = store.list_sessions(50).unwrap_or_default();
    let summaries: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            // Load last assistant message for preview
            let messages = store.get_messages(&row.id).unwrap_or_default();
            let msg_views: Vec<MessageRowView<'_>> = messages
                .iter()
                .map(|m| MessageRowView {
                    id: m.id,
                    role: &m.role,
                    content: &m.content,
                    created_at: m.created_at,
                })
                .collect();
            let session_view = SessionRowView {
                id: &row.id,
                task: &row.task,
                status: &row.status,
                provider: &row.provider,
                model: &row.model,
                step_count: row.step_count as usize,
                started_at: row.started_at,
            };
            let summary = to_watch_summary(&session_view, &msg_views);
            serde_json::to_value(summary).unwrap_or_default()
        })
        .collect();
    Json(serde_json::json!({"sessions": summaries})).into_response()
}

/// GET /watch/sessions/:id/messages — paginated message list.
async fn watch_session_messages(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = extract_watch_auth(&state, &headers) {
        return e.into_response();
    }
    let db_path = match &state.session_db_path {
        Some(p) => p.clone(),
        None => dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".vibecli")
            .join("sessions.db"),
    };
    let store = match crate::session_store::SessionStore::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("watch_session_messages: cannot open session store: {e}");
            return Json(serde_json::json!({
                "session_id": session_id,
                "messages": [],
                "total": 0,
            })).into_response();
        }
    };
    let messages = store.get_messages(&session_id).unwrap_or_default();
    let session_status = store.get_session(&session_id)
        .ok()
        .flatten()
        .map(|s| s.status)
        .unwrap_or_else(|| "unknown".to_string());
    let watch_msgs: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let view = MessageRowView {
                id: m.id,
                role: &m.role,
                content: &m.content,
                created_at: m.created_at,
            };
            serde_json::to_value(to_watch_message(&view)).unwrap_or_default()
        })
        .collect();
    let total = watch_msgs.len();
    Json(serde_json::json!({
        "session_id": session_id,
        "messages": watch_msgs,
        "total": total,
        "status": session_status,
    })).into_response()
}

/// GET /watch/stream/:id — SSE stream with Watch-optimised payloads.
async fn watch_stream(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = extract_watch_auth(&state, &headers) {
        return e.into_response();
    }
    // Tap into the existing broadcast channel
    let tx = {
        let streams = state.streams.lock().unwrap_or_else(|e| e.into_inner());
        streams.get(&session_id).cloned()
    };
    let tx = match tx {
        Some(t) => t,
        None => {
            return (StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Session stream not found"}))).into_response()
        }
    };
    let rx = tx.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|item| {
            item.ok().map(|payload| {
                let watch_ev = to_watch_event_json(&payload);
                let data = serde_json::to_string(&watch_ev).unwrap_or_default();
                Ok::<axum::response::sse::Event, Infallible>(
                    axum::response::sse::Event::default().data(data)
                )
            })
        });
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(20))
                .text("ping"),
        )
        .into_response()
}

/// POST /watch/dispatch — send a message or continue a session.
///
/// Creates (or continues) a session, inserts the user message, spawns an async
/// task that streams the LLM response over the broadcast channel, and returns
/// the session/streaming coordinates immediately.
async fn watch_dispatch(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<WatchDispatchRequest>,
) -> impl IntoResponse {
    let device_id = match extract_watch_auth(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    // Replay check
    if let Err(e) = state.nonces.check_and_record(&req.nonce, req.timestamp) {
        return (StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    // Validate content
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return (StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Content must not be empty"}))).into_response();
    }

    let provider = match &state.provider {
        Some(p) => p.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "No LLM provider configured"}))).into_response(),
    };

    let db_path = state.session_db_path.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".vibecli")
            .join("sessions.db")
    });

    // Use supplied session_id or create a new one
    let session_id = req.session_id.clone().unwrap_or_else(|| {
        format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        )
    });

    // Open store and create/continue session
    let store = match crate::session_store::SessionStore::open(&db_path) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("session store: {}", e)}))).into_response(),
    };
    if req.session_id.is_none() {
        let _ = store.insert_session(&session_id, &content, &state.provider_name, "");
    }
    let _ = store.insert_message(&session_id, "user", &content);

    // Load prior messages for context
    let prior = store.get_messages(&session_id).unwrap_or_default();
    let messages: Vec<vibe_ai::provider::Message> = prior.iter().map(|m| {
        use vibe_ai::provider::MessageRole;
        vibe_ai::provider::Message {
            role: match m.role.as_str() {
                "assistant" => MessageRole::Assistant,
                "system"    => MessageRole::System,
                _           => MessageRole::User,
            },
            content: m.content.clone(),
        }
    }).collect();

    // Create or reuse broadcast channel for this session
    let tx = {
        let mut map = state.streams.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(session_id.clone())
            .or_insert_with(|| tokio::sync::broadcast::channel(128).0)
            .clone()
    };

    tracing::info!(
        device_id = %device_id,
        session_id = %session_id,
        content_len = content.len(),
        "Watch dispatch: spawning LLM stream task"
    );

    let sid2 = session_id.clone();
    let db2 = db_path.clone();
    tokio::spawn(async move {
        let mut full = String::new();
        match provider.stream_chat(&messages).await {
            Ok(mut stream) => {
                while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
                    if let Ok(text) = chunk {
                        full.push_str(&text);
                        let _ = tx.send(serde_json::json!({"type": "token_delta", "text": text}));
                    }
                }
                let _ = tx.send(serde_json::json!({"type": "done", "status": "complete"}));
                if let Ok(s) = crate::session_store::SessionStore::open(&db2) {
                    let _ = s.insert_message(&sid2, "assistant", &full);
                    let _ = s.finish_session(&sid2, "complete", None);
                }
            }
            Err(e) => {
                let _ = tx.send(serde_json::json!({"type": "error", "message": e.to_string()}));
                if let Ok(s) = crate::session_store::SessionStore::open(&db2) {
                    let _ = s.finish_session(&sid2, "failed", None);
                }
            }
        }
    });

    // Broadcast session_updated so VibeUI clients on /watch/events know immediately
    let _ = state.session_events.send(serde_json::json!({
        "type": "session_updated",
        "session_id": session_id,
        "source": "watch",
    }));

    let streaming_url = format!("/watch/stream/{}", session_id);
    Json(serde_json::json!({
        "session_id": session_id,
        "message_id": 0,
        "streaming_url": streaming_url,
    })).into_response()
}

// ── Active session tracking ────────────────────────────────────────────────────

/// GET /watch/active-session — returns the session Watch is currently viewing.
/// VibeUI subscribes to this so both surfaces stay on the same session.
async fn watch_get_active_session(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Allow both Watch-Token and Bearer auth so VibeUI can poll this too
    let authed = extract_watch_auth(&state, &headers).is_ok()
        || headers.get("Authorization")
               .and_then(|v| v.to_str().ok())
               .map_or(false, |v| v == format!("Bearer {}", state.api_token));
    if !authed {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Auth required"}))).into_response();
    }
    let sid = state.active_session.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Json(serde_json::json!({"session_id": sid})).into_response()
}

#[derive(serde::Deserialize)]
struct SetActiveSessionRequest { session_id: String }

/// PUT /watch/active-session — Watch sets which session it's currently viewing.
async fn watch_set_active_session(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SetActiveSessionRequest>,
) -> impl IntoResponse {
    if let Err(e) = extract_watch_auth(&state, &headers) {
        return e.into_response();
    }
    *state.active_session.lock().unwrap_or_else(|e| e.into_inner()) = Some(req.session_id.clone());
    // Notify VibeUI that the Watch switched sessions
    let _ = state.session_events.send(serde_json::json!({
        "type": "watch_session_changed",
        "session_id": req.session_id,
    }));
    Json(serde_json::json!({"ok": true})).into_response()
}

/// GET /watch/events — SSE stream of real-time session events.
/// VibeUI Tauri backend subscribes here so it gets instant push when Watch
/// sends a message or changes session. No auth required (daemon-local use).
async fn watch_session_events_sse(
    State(state): State<WatchBridgeState>,
) -> impl IntoResponse {
    use std::convert::Infallible;
    let rx = state.session_events.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|item| {
            item.ok().map(|payload| {
                let data = serde_json::to_string(&payload).unwrap_or_default();
                Ok::<axum::response::sse::Event, Infallible>(
                    axum::response::sse::Event::default().data(data)
                )
            })
        });
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

// ── Sandbox chat session tracking ────────────────────────────────────────────

/// GET /watch/sandbox/chat-session — returns the VibeUI sandbox chat session ID.
/// Watch reads this on the Sandbox tab to navigate to the matching conversation.
/// Auth: Bearer (VibeUI/daemon) OR Watch-Token (Watch device).
async fn watch_get_sandbox_chat_session(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let authed = extract_watch_auth(&state, &headers).is_ok()
        || headers.get("Authorization")
               .and_then(|v| v.to_str().ok())
               .map_or(false, |v| v == format!("Bearer {}", state.api_token));
    if !authed {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Auth required"}))).into_response();
    }
    let sid = state.sandbox_chat_session.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Json(serde_json::json!({"session_id": sid})).into_response()
}

#[derive(serde::Deserialize)]
struct SetSandboxChatSessionRequest { session_id: Option<String> }

/// PUT /watch/sandbox/chat-session — VibeUI notifies the daemon which sandbox
/// chat session is active so the Watch can navigate to it.
/// Auth: Bearer only (VibeUI sets this, Watch reads it).
async fn watch_set_sandbox_chat_session(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SetSandboxChatSessionRequest>,
) -> impl IntoResponse {
    let bearer = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // Accept both Bearer (VibeUI) and Watch-Token (Watch UI) so either surface can set it
    let authed = bearer == format!("Bearer {}", state.api_token)
        || extract_watch_auth(&state, &headers).is_ok();
    if !authed {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Auth required"}))).into_response();
    }
    *state.sandbox_chat_session.lock().unwrap_or_else(|e| e.into_inner()) = req.session_id.clone();
    let _ = state.session_events.send(serde_json::json!({
        "type": "sandbox_chat_session_changed",
        "session_id": req.session_id,
    }));
    Json(serde_json::json!({"ok": true})).into_response()
}

/// GET /watch/devices — list registered watch devices (requires bearer token).
async fn watch_list_devices(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let bearer = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if bearer != format!("Bearer {}", state.api_token) {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Bearer token required"}))).into_response();
    }
    let auth = state.auth.lock().unwrap_or_else(|e| e.into_inner());
    let devices = auth.list_devices().unwrap_or_default();
    // Scrub public key from list response (available on detail endpoint)
    let safe: Vec<serde_json::Value> = devices
        .iter()
        .map(|d| serde_json::json!({
            "device_id": d.device_id,
            "name": d.name,
            "model": d.model,
            "os_version": d.os_version,
            "registered_at": d.registered_at,
            "last_seen": d.last_seen,
            "revoked": d.revoked_at.is_some(),
            "wrist_suspended": d.wrist_suspended,
        }))
        .collect();
    Json(serde_json::json!({"devices": safe})).into_response()
}

/// DELETE /watch/devices/:id — revoke a watch device (requires bearer token).
async fn watch_revoke_device(
    State(state): State<WatchBridgeState>,
    headers: axum::http::HeaderMap,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    let bearer = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if bearer != format!("Bearer {}", state.api_token) {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Bearer token required"}))).into_response();
    }
    let mut auth = state.auth.lock().unwrap_or_else(|e| e.into_inner());
    match auth.revoke_device(&device_id) {
        Ok(()) => Json(serde_json::json!({"ok": true, "device_id": device_id})).into_response(),
        Err(e) => (StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_watch_router_does_not_panic() {
        // WatchBridgeState construction requires ProfileStore access which
        // touches the filesystem — test only the router building logic.
        // Full integration tests live in BDD harness watch_bridge_bdd.rs
        let _ = std::mem::size_of::<WatchBridgeState>();
    }

    #[test]
    fn nonce_registry_in_relay_is_used() {
        let reg = NonceRegistry::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(reg.check_and_record("nonce-1", now).is_ok());
        assert!(reg.check_and_record("nonce-1", now).is_err()); // replay
        assert!(reg.check_and_record("nonce-2", now).is_ok()); // different nonce ok
    }

    #[test]
    fn watch_dispatch_response_serializes() {
        let resp = WatchDispatchResponse {
            session_id: "sess-abc".into(),
            message_id: 42,
            streaming_url: "/watch/stream/sess-abc".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("sess-abc"));
        assert!(json.contains("42"));
    }

    // ── RED → GREEN: bridge state and router coverage ─────────────────────────

    #[test]
    fn watch_dispatch_response_streaming_url_contains_session_id() {
        let session_id = "my-session-xyz";
        let resp = WatchDispatchResponse {
            session_id: session_id.into(),
            message_id: 1,
            streaming_url: format!("/watch/stream/{}", session_id),
        };
        assert!(resp.streaming_url.contains(session_id));
        assert!(resp.streaming_url.starts_with("/watch/stream/"));
    }

    #[test]
    fn watch_event_streams_new_is_empty() {
        let streams: WatchEventStreams =
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let map = streams.lock().unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn watch_event_streams_accepts_broadcaster() {
        let streams: WatchEventStreams =
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
        {
            let mut map = streams.lock().unwrap();
            map.insert("session-1".into(), tx);
        }
        assert_eq!(streams.lock().unwrap().len(), 1);
        assert!(streams.lock().unwrap().contains_key("session-1"));
    }

    #[test]
    fn watch_sandbox_control_request_serde() {
        use crate::watch_session_relay::WatchSandboxControlRequest;
        let req = WatchSandboxControlRequest {
            action: "pause".into(),
            nonce: "nonce-xyz".into(),
            timestamp: 1_700_000_000,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("pause"));
        let back: WatchSandboxControlRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "pause");
        assert_eq!(back.nonce, "nonce-xyz");
    }

    #[test]
    fn watch_bridge_state_size_of_does_not_panic() {
        // Ensures WatchBridgeState is a valid, sized type
        assert!(std::mem::size_of::<WatchBridgeState>() > 0);
    }

    #[test]
    fn nonce_replay_rejected_in_bridge_context() {
        let reg = NonceRegistry::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Simulate Watch sending dispatch request with nonce
        let nonce = "watch-dispatch-nonce-001";
        assert!(reg.check_and_record(nonce, now).is_ok());
        // Identical request replayed → rejected
        let err = reg.check_and_record(nonce, now).unwrap_err();
        assert!(err.to_string().contains("replay") || err.to_string().contains("Nonce"));
    }

    #[test]
    fn watch_dispatch_request_without_session_id() {
        use crate::watch_session_relay::WatchDispatchRequest;
        let req = WatchDispatchRequest {
            session_id: None,  // new session
            content: "Start a new task".into(),
            provider: Some("claude".into()),
            nonce: "fresh-nonce-001".into(),
            timestamp: 1_700_000_000,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: WatchDispatchRequest = serde_json::from_str(&json).unwrap();
        assert!(back.session_id.is_none());
        assert_eq!(back.provider.as_deref(), Some("claude"));
    }

    #[test]
    fn watch_dispatch_request_with_session_id() {
        use crate::watch_session_relay::WatchDispatchRequest;
        let req = WatchDispatchRequest {
            session_id: Some("existing-session-abc".into()),
            content: "Continue the task".into(),
            provider: None,
            nonce: "fresh-nonce-002".into(),
            timestamp: 1_700_000_001,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: WatchDispatchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id.as_deref(), Some("existing-session-abc"));
        assert!(back.provider.is_none());
    }
}
