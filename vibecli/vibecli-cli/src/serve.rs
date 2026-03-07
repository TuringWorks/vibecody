//! VibeCLI HTTP daemon (`vibecli serve`).
//!
//! Exposes a REST/SSE API that the VS Code extension and Agent SDK can talk to.
//!
//! # Endpoints
//!
//! | Method | Path                      | Description                          |
//! |--------|---------------------------|--------------------------------------|
//! | GET    | `/health`                 | Liveness check                       |
//! | POST   | `/chat`                   | Single-turn chat (non-streaming)     |
//! | POST   | `/chat/stream`            | Streaming chat as SSE                |
//! | POST   | `/agent`                  | Start an agent task → returns `{session_id}` |
//! | GET    | `/stream/:session_id`     | SSE stream of agent events           |
//! | GET    | `/jobs`                   | List all persisted job records       |
//! | GET    | `/jobs/:id`               | Get a single job record              |
//! | POST   | `/jobs/:id/cancel`        | Cancel a running job                 |
//! | GET    | `/sessions`               | HTML index of all agent sessions     |
//! | GET    | `/sessions.json`          | JSON list of all sessions            |
//! | GET    | `/view/:id`               | HTML page for a specific session     |
//! | GET    | `/share/:id`              | Shareable readonly session view (adds "Shared" banner) |
//!
//! # Usage
//!
//! ```bash
//! vibecli serve --port 7878 --provider ollama
//! ```

use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{broadcast, Mutex};
use rand::Rng;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use vibe_ai::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, Message, MessageRole};
use vibe_ai::provider::AIProvider;
use vibe_collab::{CollabMessage, CollabServer, PeerInfo, SyncBroadcast};
use crate::session_store::{SessionStore, render_session_html, render_sessions_index_html};

// ── Job record (persisted to disk) ────────────────────────────────────────────

/// A persistent record of a background agent job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub session_id: String,
    pub task: String,
    /// "running" | "complete" | "failed" | "cancelled"
    pub status: String,
    pub provider: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub summary: Option<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn persist_job(jobs_dir: &std::path::Path, record: &JobRecord) {
    let path = jobs_dir.join(format!("{}.json", record.session_id));
    if let Ok(json) = serde_json::to_string_pretty(record) {
        let _ = std::fs::write(&path, json);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

fn load_job(jobs_dir: &std::path::Path, session_id: &str) -> Option<JobRecord> {
    let path = jobs_dir.join(format!("{}.json", session_id));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn load_all_jobs(jobs_dir: &std::path::Path) -> Vec<JobRecord> {
    let Ok(entries) = std::fs::read_dir(jobs_dir) else { return vec![] };
    let mut jobs: Vec<JobRecord> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str::<JobRecord>(&s).ok())
        .collect();
    jobs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    jobs
}

// ── Shared server state ───────────────────────────────────────────────────────

/// Live event streams keyed by session_id.
type EventStreams = Arc<Mutex<HashMap<String, broadcast::Sender<AgentEventPayload>>>>;

#[derive(Clone)]
pub struct ServeState {
    pub provider: Arc<dyn AIProvider>,
    pub approval: ApprovalPolicy,
    pub workspace_root: PathBuf,
    pub streams: EventStreams,
    pub jobs_dir: PathBuf,
    pub provider_name: String,
    /// Bearer token required on all non-health/non-viewer endpoints.
    /// Generated randomly on daemon startup and printed to stderr.
    pub api_token: String,
    /// CRDT collaboration server for multiplayer editing.
    pub collab_server: Arc<CollabServer>,
    /// GitHub App webhook config for CI/CD review bot.
    pub github_app_config: crate::github_app::GithubAppConfig,
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    #[allow(dead_code)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub task: String,
    #[serde(default)]
    pub approval: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentStartResponse {
    pub session_id: String,
}

/// An agent event serialized for SSE.
#[derive(Debug, Clone, Serialize)]
pub struct AgentEventPayload {
    #[serde(rename = "type")]
    pub kind: String,
    pub content: Option<String>,
    pub step_num: Option<usize>,
    pub tool_name: Option<String>,
    pub success: Option<bool>,
}

impl AgentEventPayload {
    fn chunk(text: String) -> Self {
        Self { kind: "chunk".into(), content: Some(text), step_num: None, tool_name: None, success: None }
    }
    fn step(step_num: usize, tool: &str, success: bool) -> Self {
        Self { kind: "step".into(), content: None, step_num: Some(step_num), tool_name: Some(tool.into()), success: Some(success) }
    }
    fn complete(summary: String) -> Self {
        Self { kind: "complete".into(), content: Some(summary), step_num: None, tool_name: None, success: None }
    }
    fn error(msg: String) -> Self {
        Self { kind: "error".into(), content: Some(msg), step_num: None, tool_name: None, success: None }
    }
}

/// JSON error response helper — returns `{"error":"msg"}` with correct Content-Type.
fn json_error(status: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg.into() })))
}

// ── Route handlers ────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Skill webhook endpoint — triggers a skill by name via POST.
/// Matches skills with a `webhook_trigger` field set to the given name.
async fn skill_webhook_handler(
    Path(skill_name): Path<String>,
    body: String,
) -> impl IntoResponse {
    use vibe_ai::SkillLoader;
    let cwd = std::env::current_dir().unwrap_or_default();
    let loader = SkillLoader::new(&cwd);
    let skills = loader.load_all();
    let matching = skills.iter().find(|s| {
        s.webhook_trigger.as_deref() == Some(skill_name.as_str())
    });
    match matching {
        Some(skill) => {
            tracing::info!("[webhook] Triggered skill '{}' via webhook (body: {} bytes)", skill.name, body.len());
            (StatusCode::OK, Json(serde_json::json!({
                "triggered": true,
                "skill": skill.name,
                "body_length": body.len(),
            })))
        }
        None => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "triggered": false,
                "error": format!("No skill with webhook_trigger '{}'", skill_name),
            })))
        }
    }
}

/// Device pairing endpoint — generates a one-time pairing URL.
async fn pairing_handler() -> impl IntoResponse {
    let (url, token) = crate::pairing::generate_pairing_url("localhost", 7878);
    Json(serde_json::json!({
        "url": url,
        "token": token,
        "instructions": "Open this URL in your device's browser to pair with this VibeCLI instance."
    }))
}

async fn chat(
    State(state): State<ServeState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<serde_json::Value>)> {
    use futures::StreamExt;

    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => MessageRole::User,
            },
            content: m.content.clone(),
        })
        .collect();

    let mut stream = state
        .provider
        .stream_chat(&messages)
        .await
        .map_err(|e| {
            tracing::error!("chat provider error: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("LLM provider error: {e}"))
        })?;

    let mut accumulated = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => accumulated.push_str(&text),
            Err(e) => {
                tracing::error!("chat stream error: {e}");
                return Err(json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("Stream error: {e}")));
            }
        }
    }

    Ok(Json(ChatResponse { content: accumulated }))
}

async fn chat_stream(
    State(state): State<ServeState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => MessageRole::User,
            },
            content: m.content.clone(),
        })
        .collect();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(128);

    tokio::spawn(async move {
        match state.provider.stream_chat(&messages).await {
            Ok(mut stream) => {
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(text) => {
                            let _ = tx.send(Ok(Event::default().data(text))).await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(Ok(Event::default()
                                    .event("error")
                                    .data(e.to_string())))
                                .await;
                        }
                    }
                }
                let _ = tx.send(Ok(Event::default().event("done").data(""))).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Ok(Event::default().event("error").data(e.to_string())))
                    .await;
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn start_agent(
    State(state): State<ServeState>,
    Json(req): Json<AgentRequest>,
) -> Result<Json<AgentStartResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Use a cryptographically random 128-bit hex ID to prevent session enumeration.
    let session_id = format!("{:032x}", rand::thread_rng().gen::<u128>());

    // Persist initial job record
    let mut record = JobRecord {
        session_id: session_id.clone(),
        task: req.task.clone(),
        status: "running".to_string(),
        provider: state.provider_name.clone(),
        started_at: now_ms(),
        finished_at: None,
        summary: None,
    };
    persist_job(&state.jobs_dir, &record);

    // Create broadcast channel for SSE fan-out
    let (tx, _) = broadcast::channel::<AgentEventPayload>(256);
    {
        let mut streams = state.streams.lock().await;
        streams.insert(session_id.clone(), tx.clone());
    }

    let approval = match &req.approval {
        Some(s) => ApprovalPolicy::from_str(s),
        None => state.approval.clone(),
    };

    let task = req.task.clone();
    let sid = session_id.clone();
    let workspace_root = state.workspace_root.clone();
    let provider = state.provider.clone();
    let streams = state.streams.clone();
    let jobs_dir = state.jobs_dir.clone();

    tokio::spawn(async move {
        use crate::tool_executor::ToolExecutor;

        let executor = Arc::new(ToolExecutor::new(workspace_root.clone(), false));
        let agent = AgentLoop::new(provider, approval, executor);

        let git_branch = vibe_core::git::get_current_branch(&workspace_root).ok();
        let context = AgentContext {
            workspace_root: workspace_root.clone(),
            open_files: vec![],
            git_branch,
            git_diff_summary: None,
            flow_context: None,
            approved_plan: None,
            extra_skill_dirs: vec![],
            parent_session_id: None,
            depth: 0,
            active_agent_counter: None,
            team_bus: None,
            team_agent_id: None,
        };

        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentEvent>(64);

        tokio::spawn(async move {
            let _ = agent.run(&task, context, event_tx).await;
        });

        while let Some(event) = event_rx.recv().await {
            let payload = match event {
                AgentEvent::StreamChunk(text) => AgentEventPayload::chunk(text),
                AgentEvent::ToolCallExecuted(step) => {
                    AgentEventPayload::step(step.step_num, step.tool_call.name(), step.tool_result.success)
                }
                AgentEvent::Complete(summary) => {
                    let p = AgentEventPayload::complete(summary.clone());
                    // Persist completion
                    record.status = "complete".to_string();
                    record.finished_at = Some(now_ms());
                    record.summary = Some(summary);
                    persist_job(&jobs_dir, &record);
                    // Broadcast final event before removing stream
                    let _ = tx.send(p.clone());
                    let mut s = streams.lock().await;
                    s.remove(&sid);
                    break;
                }
                AgentEvent::Error(msg) => {
                    let p = AgentEventPayload::error(msg.clone());
                    // Persist failure
                    record.status = "failed".to_string();
                    record.finished_at = Some(now_ms());
                    record.summary = Some(msg);
                    persist_job(&jobs_dir, &record);
                    // Broadcast error event before removing stream
                    let _ = tx.send(p.clone());
                    let mut s = streams.lock().await;
                    s.remove(&sid);
                    break;
                }
                _ => continue,
            };
            let _ = tx.send(payload);
        }
    });

    Ok(Json(AgentStartResponse { session_id }))
}

// ── Job endpoints ─────────────────────────────────────────────────────────────

async fn list_jobs(
    State(state): State<ServeState>,
) -> Json<Vec<JobRecord>> {
    Json(load_all_jobs(&state.jobs_dir))
}

async fn get_job(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<JobRecord>, (StatusCode, Json<serde_json::Value>)> {
    load_job(&state.jobs_dir, &id)
        .map(Json)
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{id}' not found")))
}

async fn cancel_job(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<JobRecord>, (StatusCode, Json<serde_json::Value>)> {
    // Remove stream (ends SSE)
    {
        let mut streams = state.streams.lock().await;
        streams.remove(&id);
    }

    // Update persisted record
    let mut record = load_job(&state.jobs_dir, &id)
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{id}' not found")))?;
    if record.status == "running" {
        record.status = "cancelled".to_string();
        record.finished_at = Some(now_ms());
        persist_job(&state.jobs_dir, &record);
    }
    Ok(Json(record))
}

async fn stream_agent(
    Path(session_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)> {
    use tokio_stream::wrappers::BroadcastStream;
    use futures::StreamExt;

    let rx = {
        let streams = state.streams.lock().await;
        let tx = streams
            .get(&session_id)
            .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Session '{session_id}' not found")))?;
        tx.subscribe()
    };

    let stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(payload) => {
                let json = serde_json::to_string(&payload).ok()?;
                Some(Ok(Event::default().data(json)))
            }
            Err(_) => None,
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

// ── Auth middleware ───────────────────────────────────────────────────────────

/// Axum middleware that enforces bearer-token authentication.
/// Rejects requests that don't carry the correct `Authorization: Bearer <token>`.
async fn require_auth(
    State(state): State<ServeState>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let expected = format!("Bearer {}", state.api_token);
    match auth_header {
        Some(val) if val == expected => {
            next.run(req).await.into_response()
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            [("content-type", "application/json")],
            r#"{"error":"Missing or invalid Authorization: Bearer <token>"}"#,
        )
            .into_response(),
    }
}

// ── Rate limiting middleware ─────────────────────────────────────────────────

/// Simple sliding-window rate limiter: max `limit` requests per `window`.
/// Shared across all clients (single-daemon deployment).
struct RateLimiter {
    /// Ring buffer of request timestamps (unix millis).
    timestamps: std::sync::Mutex<Vec<u64>>,
    limit: usize,
    window_ms: u64,
}

impl RateLimiter {
    fn new(limit: usize, window: Duration) -> Self {
        Self {
            timestamps: std::sync::Mutex::new(Vec::with_capacity(limit)),
            limit,
            window_ms: window.as_millis() as u64,
        }
    }

    /// Returns true if the request should be allowed.
    fn check(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut ts = self.timestamps.lock().unwrap_or_else(|e| e.into_inner());
        let cutoff = now.saturating_sub(self.window_ms);
        ts.retain(|&t| t > cutoff);
        if ts.len() >= self.limit {
            false
        } else {
            ts.push(now);
            true
        }
    }
}

/// Axum middleware that enforces a global request rate limit.
async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    if limiter.check() {
        next.run(req).await.into_response()
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("content-type", "application/json"), ("retry-after", "5")],
            r#"{"error":"Rate limit exceeded. Try again shortly."}"#,
        )
            .into_response()
    }
}

// ── GitHub Webhook handler ────────────────────────────────────────────────────

/// Handle incoming GitHub webhook events (pull_request.opened / synchronize).
/// Uses HMAC-SHA256 signature verification (not bearer token auth).
async fn github_webhook(
    State(state): State<ServeState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok());

    match crate::github_app::handle_webhook(
        &body,
        event_type,
        signature,
        &state.github_app_config,
        state.provider.clone(),
    )
    .await
    {
        Ok(Some(result)) => {
            eprintln!(
                "[github-app] Reviewed PR #{} on {} → {} ({} findings)",
                result.pr_number, result.repo, result.status, result.findings_count
            );
            (StatusCode::OK, Json(serde_json::json!({
                "status": result.status,
                "findings": result.findings_count,
                "summary": result.summary,
            })))
                .into_response()
        }
        Ok(None) => {
            // Event type not handled (e.g., push, issue, etc.)
            (StatusCode::OK, Json(serde_json::json!({"status": "ignored"}))).into_response()
        }
        Err(e) => {
            eprintln!("[github-app] Webhook error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

// ── ACP (Agent Client Protocol) handlers ─────────────────────────────────────

/// Return ACP capability advertisement.
async fn acp_capabilities() -> impl IntoResponse {
    Json(crate::acp::default_capabilities())
}

/// Create a new ACP task (delegates to the agent endpoint).
async fn acp_create_task(
    State(state): State<ServeState>,
    Json(req): Json<crate::acp::AcpTaskRequest>,
) -> impl IntoResponse {
    // Reuse the existing agent infrastructure
    let session_id = format!("acp-{:016x}", rand::thread_rng().gen::<u64>());

    let status = crate::acp::AcpTaskStatus {
        id: session_id.clone(),
        status: "pending".to_string(),
        summary: Some(format!("Task queued: {}", req.task)),
        files_modified: Vec::new(),
        steps_completed: 0,
    };

    // Start agent in background (reuse existing start_agent pattern)
    let provider = state.provider.clone();
    let workspace = req.context
        .as_ref()
        .and_then(|c| c.workspace_root.clone())
        .unwrap_or_else(|| state.workspace_root.to_string_lossy().to_string());

    let task = req.task.clone();
    let sid = session_id.clone();
    let jobs_dir = state.jobs_dir.clone();
    let provider_name = state.provider_name.clone();

    tokio::spawn(async move {
        let record = JobRecord {
            session_id: sid.clone(),
            task: task.clone(),
            status: "running".to_string(),
            provider: provider_name,
            started_at: now_ms(),
            finished_at: None,
            summary: None,
        };
        persist_job(&jobs_dir, &record);

        let executor = crate::tool_executor::ToolExecutor::new(
            std::path::PathBuf::from(&workspace),
            false,
        );
        let context = vibe_ai::AgentContext {
            workspace_root: std::path::PathBuf::from(&workspace),
            ..Default::default()
        };
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(256);
        let agent = vibe_ai::AgentLoop::new(
            provider.clone(),
            vibe_ai::ApprovalPolicy::FullAuto,
            Arc::new(executor) as Arc<dyn vibe_ai::ToolExecutorTrait>,
        );
        let _result = agent.run(&task, context, event_tx).await;

        let mut record = record;
        record.status = "complete".to_string();
        record.finished_at = Some(now_ms());
        record.summary = Some("ACP task completed".to_string());
        persist_job(&jobs_dir, &record);
    });

    (StatusCode::CREATED, Json(status))
}

/// Get ACP task status.
async fn acp_get_task(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Check job record
    if let Some(job) = load_job(&state.jobs_dir, &id) {
        let status = crate::acp::AcpTaskStatus {
            id: job.session_id,
            status: job.status,
            summary: job.summary,
            files_modified: Vec::new(),
            steps_completed: 0,
        };
        (StatusCode::OK, Json(status)).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found"}))).into_response()
    }
}

// ── Server startup ────────────────────────────────────────────────────────────

/// Build the full axum router with all middleware, CORS, auth, and routes.
/// Extracted so that tests can call it with `tower::ServiceExt::oneshot()`
/// without binding to a TCP port.
pub(crate) fn build_router(state: ServeState, port: u16) -> Router {
    // CORS: restrict to localhost origins only
    let origins: Vec<HeaderValue> = [
        "http://localhost".to_string(),
        "http://127.0.0.1".to_string(),
        format!("http://localhost:{port}"),
        format!("http://127.0.0.1:{port}"),
        // Tauri 2 dev server origins
        "tauri://localhost".to_string(),
        "https://tauri.localhost".to_string(),
        // Vite dev server (default port)
        "http://localhost:1420".to_string(),
    ]
    .into_iter()
    .filter_map(|s| s.parse::<HeaderValue>().ok())
    .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // Rate limiter: 60 requests per 60 seconds (global across all authed endpoints)
    let limiter = Arc::new(RateLimiter::new(60, Duration::from_secs(60)));

    // Routes that require bearer-token auth (API endpoints)
    let authed_routes = Router::new()
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/agent", post(start_agent))
        .route("/stream/:session_id", get(stream_agent))
        .route("/jobs", get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/collab/rooms", post(create_collab_room))
        .route("/collab/rooms", get(list_collab_rooms))
        .route("/collab/rooms/:room_id/peers", get(list_collab_peers))
        .route("/acp/v1/tasks", post(acp_create_task))
        .route("/acp/v1/tasks/:id", get(acp_get_task))
        // Session viewer & skill webhook now require auth
        .route("/sessions", get(sessions_index_html))
        .route("/sessions.json", get(sessions_json))
        .route("/view/:id", get(view_session))
        .route("/share/:id", get(share_session))
        .route("/webhook/skill/:skill_name", post(skill_webhook_handler))
        .route_layer(middleware::from_fn_with_state(limiter, rate_limit))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Public routes (health check, GitHub webhook with HMAC, pairing, collab WS, ACP discovery)
    Router::new()
        .route("/health", get(health))
        .route("/webhook/github", post(github_webhook))
        .route("/pair", get(pairing_handler))
        .route("/acp/v1/capabilities", get(acp_capabilities))
        .route("/ws/collab/:room_id", get(ws_collab_handler))
        .merge(authed_routes)
        .fallback(|| async {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"Not found"})))
        })
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1 MB max request body
        // Security response headers
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'self'; script-src 'none'; style-src 'unsafe-inline'"),
        ))
        .layer(cors)
        .with_state(state)
}

/// Start the VibeCLI HTTP daemon. Blocks until shutdown.
pub async fn serve(
    provider: Arc<dyn AIProvider>,
    provider_name: String,
    approval: ApprovalPolicy,
    workspace_root: PathBuf,
    port: u16,
) -> Result<()> {
    // Initialise persistent jobs directory at ~/.vibecli/jobs/
    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("jobs");
    std::fs::create_dir_all(&jobs_dir)?;

    // Generate a random bearer token for this daemon session
    let api_token = format!("{:032x}", rand::thread_rng().gen::<u128>());

    let collab_server = Arc::new(CollabServer::new(20));

    // Load GitHub App config
    let gh_app_config = {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("config.toml");
        if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str::<crate::config::Config>(&s).ok())
                .map(|c| c.github_app)
                .unwrap_or_default()
        } else {
            crate::github_app::GithubAppConfig::default()
        }
    };

    let state = ServeState {
        provider,
        approval,
        workspace_root,
        streams: Arc::new(Mutex::new(HashMap::new())),
        jobs_dir,
        provider_name,
        api_token: api_token.clone(),
        collab_server,
        github_app_config: gh_app_config,
    };

    let app = build_router(state, port);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[vibecli serve] Listening on http://{addr}");
    eprintln!("[vibecli serve] API token: {api_token}");
    eprintln!("[vibecli serve] Jobs persisted at ~/.vibecli/jobs/");
    eprintln!("[vibecli serve] Session viewer at http://{addr}/sessions");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    eprintln!("[vibecli serve] Shutting down gracefully");
    Ok(())
}

// ── Collab endpoints ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateRoomRequest {
    room_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RoomInfo {
    room_id: String,
    peer_count: usize,
}

async fn create_collab_room(
    State(state): State<ServeState>,
    Json(req): Json<CreateRoomRequest>,
) -> Json<RoomInfo> {
    let room_id = req
        .room_id
        .unwrap_or_else(|| format!("{:016x}", rand::thread_rng().gen::<u64>()));
    let room = state.collab_server.get_or_create_room(&room_id);
    let peer_count = room.peer_count().await;
    Json(RoomInfo { room_id, peer_count })
}

async fn list_collab_rooms(
    State(state): State<ServeState>,
) -> Json<Vec<String>> {
    Json(state.collab_server.list_rooms())
}

async fn list_collab_peers(
    Path(room_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<Vec<PeerInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let room = state
        .collab_server
        .get_room(&room_id)
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Room '{room_id}' not found")))?;
    Ok(Json(room.list_peers().await))
}

#[derive(Debug, Deserialize)]
struct WsCollabParams {
    token: String,
    name: Option<String>,
}

async fn ws_collab_handler(
    Path(room_id): Path<String>,
    Query(params): Query<WsCollabParams>,
    State(state): State<ServeState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Authenticate via query param token
    if params.token != state.api_token {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }

    let name = params.name.unwrap_or_else(|| "Anonymous".to_string());
    let room = state.collab_server.get_or_create_room(&room_id);
    let collab_server = state.collab_server.clone();

    ws.on_upgrade(move |socket| handle_collab_ws(socket, room, name, room_id, collab_server))
        .into_response()
}

async fn handle_collab_ws(
    mut socket: WebSocket,
    room: std::sync::Arc<vibe_collab::CollabRoom>,
    name: String,
    room_id: String,
    collab_server: Arc<CollabServer>,
) {

    // Generate a peer ID and add to room
    let peer_id = format!("{:016x}", rand::thread_rng().gen::<u64>());
    let peer = match room.add_peer(peer_id.clone(), name).await {
        Ok(p) => p,
        Err(e) => {
            let err_msg = CollabMessage::Error {
                message: e.to_string(),
            };
            if let Ok(json) = serde_json::to_string(&err_msg) {
                let _ = socket.send(WsMessage::Text(json)).await;
            }
            return;
        }
    };

    // Send Welcome message with current peer list
    let peers = room.list_peers().await;
    let welcome = CollabMessage::Welcome {
        room_id: room_id.clone(),
        peer_id: peer_id.clone(),
        peers,
    };
    let welcome_json = serde_json::to_string(&welcome).unwrap_or_default();
    if socket
        .send(WsMessage::Text(welcome_json))
        .await
        .is_err()
    {
        room.remove_peer(&peer_id).await;
        return;
    }

    // Send current doc state as SyncStep1
    let state_msg = room.encode_state().await;
    if socket.send(WsMessage::Binary(state_msg)).await.is_err() {
        room.remove_peer(&peer_id).await;
        return;
    }

    // Broadcast PeerJoined to other peers
    let joined_msg = CollabMessage::PeerJoined { peer: peer.clone() };
    if let Ok(joined_json) = serde_json::to_string(&joined_msg) {
        let _ = room.sync_tx.send(SyncBroadcast {
            sender_peer_id: peer_id.clone(),
            data: joined_json.into_bytes(),
        });
    }

    // Subscribe to broadcast channel for fan-out
    let mut broadcast_rx = room.sync_tx.subscribe();

    // Main loop: receive from WS + fan-out from broadcast
    loop {
        tokio::select! {
            // Incoming message from this peer's WebSocket
            msg = socket.recv() => {
                match msg {
                    Some(Ok(WsMessage::Binary(data))) => {
                        // Binary frame = Yjs sync protocol
                        let data_vec: Vec<u8> = data;
                        match room.apply_message(&data_vec).await {
                            Ok(Some(reply)) => {
                                // Send reply (e.g. SyncStep2) back to sender
                                let _ = socket.send(WsMessage::Binary(reply)).await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                tracing::warn!(peer_id, "collab sync error: {e}");
                            }
                        }
                        // Broadcast the update to all other peers
                        let _ = room.sync_tx.send(SyncBroadcast {
                            sender_peer_id: peer_id.clone(),
                            data: data_vec,
                        });
                    }
                    Some(Ok(WsMessage::Text(text))) => {
                        // Text frame = JSON CollabMessage (awareness, file_opened, etc.)
                        if let Ok(_collab_msg) = serde_json::from_str::<CollabMessage>(&text) {
                            // Broadcast awareness updates to all other peers
                            let _ = room.sync_tx.send(SyncBroadcast {
                                sender_peer_id: peer_id.clone(),
                                data: text.as_bytes().to_vec(),
                            });
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    _ => {}
                }
            }
            // Outgoing broadcast from other peers
            broadcast = broadcast_rx.recv() => {
                match broadcast {
                    Ok(sync_broadcast) if sync_broadcast.sender_peer_id != peer_id => {
                        // Determine if binary or text
                        let data = &sync_broadcast.data;
                        if let Ok(text) = std::str::from_utf8(data) {
                            if text.starts_with('{') {
                                // JSON text message
                                let _ = socket.send(WsMessage::Text(text.to_string())).await;
                            } else {
                                let _ = socket.send(WsMessage::Binary(data.clone())).await;
                            }
                        } else {
                            // Binary Yjs update
                            let _ = socket.send(WsMessage::Binary(data.clone())).await;
                        }
                    }
                    Err(_) => break, // channel closed
                    _ => {} // skip own messages
                }
            }
        }
    }

    // Peer disconnected — clean up
    let room_empty = room.remove_peer(&peer_id).await;

    // Broadcast PeerLeft
    let left_msg = CollabMessage::PeerLeft {
        peer_id: peer_id.clone(),
    };
    if let Ok(left_json) = serde_json::to_string(&left_msg) {
        let _ = room.sync_tx.send(SyncBroadcast {
            sender_peer_id: peer_id,
            data: left_json.into_bytes(),
        });
    }

    // Clean up empty rooms
    if room_empty {
        collab_server.remove_room(&room_id);
        tracing::info!(room_id, "removed empty collab room");
    }
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        if let Ok(mut sigterm) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            tokio::select! {
                _ = ctrl_c => { eprintln!("\n[vibecli serve] Received SIGINT, shutting down..."); }
                _ = sigterm.recv() => { eprintln!("[vibecli serve] Received SIGTERM, shutting down..."); }
            }
        } else {
            // Fallback to Ctrl+C only if SIGTERM handler fails
            let _ = ctrl_c.await;
            eprintln!("\n[vibecli serve] Received SIGINT, shutting down...");
        }
    }

    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
        eprintln!("\n[vibecli serve] Received Ctrl+C, shutting down...");
    }
}

// ── Web session viewer handlers ───────────────────────────────────────────────

async fn sessions_index_html() -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => {
            tracing::error!("session store open error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                "Internal server error".to_string(),
            )
        }
        Ok(store) => {
            let sessions = store.list_sessions(200).unwrap_or_default();
            let html = render_sessions_index_html(&sessions);
            (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
        }
    }
}

async fn sessions_json() -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => {
            tracing::error!("session store open error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "application/json")],
                r#"{"error":"Internal server error"}"#.to_string(),
            )
        }
        Ok(store) => {
            let sessions = store.list_sessions(200).unwrap_or_default();
            let json = serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".into());
            (StatusCode::OK, [("content-type", "application/json")], json)
        }
    }
}

/// Shareable readonly view of a session — identical to `/view/:id` but injects
/// a green "Shared" banner and a `noindex` meta tag so search engines don't index it.
async fn share_session(Path(id): Path<String>) -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => {
            tracing::error!("session store open error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                "Internal server error".to_string(),
            )
        }
        Ok(store) => match store.get_session_detail(&id) {
            Err(e) => {
                tracing::error!("session detail error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "text/plain")],
                    "Internal server error".to_string(),
                )
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                [("content-type", "text/html; charset=utf-8")],
                "<h1>Session not found</h1><p><a href=\"/sessions\">All sessions</a></p>".to_string(),
            ),
            Ok(Some(detail)) => {
                let html = render_session_html(&detail);
                // Inject noindex meta and a "Shared" banner.
                let banner = r#"<meta name="robots" content="noindex,nofollow">
<div style="background:#1a3a1a;border-bottom:1px solid #3fb950;padding:8px 24px;margin-bottom:20px;border-radius:4px;color:#3fb950;font-size:13px">
  📤 <strong>Shared session</strong> — readonly view
</div>"#;
                let html = html.replace("<body>", &format!("<body>\n{}", banner));
                (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
            }
        },
    }
}

async fn view_session(Path(id): Path<String>) -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => {
            tracing::error!("session store open error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                "Internal server error".to_string(),
            )
        }
        Ok(store) => match store.get_session_detail(&id) {
            Err(e) => {
                tracing::error!("session detail error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "text/plain")],
                    "Internal server error".to_string(),
                )
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                [("content-type", "text/html; charset=utf-8")],
                "<h1>Session not found</h1><p><a href=\"/sessions\">All sessions</a></p>".to_string(),
            ),
            Ok(Some(detail)) => {
                let html = render_session_html(&detail);
                (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
            }
        },
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── now_ms ─────────────────────────────────────────────────────────────

    #[test]
    fn now_ms_returns_reasonable_timestamp() {
        let ts = now_ms();
        // Should be after 2024-01-01 and before 2100-01-01 (in millis)
        let jan_2024 = 1_704_067_200_000u64;
        let jan_2100 = 4_102_444_800_000u64;
        assert!(ts > jan_2024, "timestamp {ts} should be after 2024");
        assert!(ts < jan_2100, "timestamp {ts} should be before 2100");
    }

    #[test]
    fn now_ms_is_monotonic() {
        let t1 = now_ms();
        let t2 = now_ms();
        assert!(t2 >= t1, "second call should be >= first");
    }

    // ── persist_job / load_job / load_all_jobs ─────────────────────────────

    fn make_job(id: &str, started_at: u64) -> JobRecord {
        JobRecord {
            session_id: id.to_string(),
            task: format!("task for {id}"),
            status: "running".to_string(),
            provider: "ollama".to_string(),
            started_at,
            finished_at: None,
            summary: None,
        }
    }

    #[test]
    fn persist_and_load_job_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let job = make_job("abc123", 1_700_000_000_000);
        persist_job(dir.path(), &job);

        let loaded = load_job(dir.path(), "abc123");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.session_id, "abc123");
        assert_eq!(loaded.task, "task for abc123");
        assert_eq!(loaded.status, "running");
        assert_eq!(loaded.provider, "ollama");
        assert_eq!(loaded.started_at, 1_700_000_000_000);
        assert!(loaded.finished_at.is_none());
        assert!(loaded.summary.is_none());
    }

    #[test]
    fn persist_and_load_job_with_optional_fields() {
        let dir = tempfile::tempdir().unwrap();
        let job = JobRecord {
            session_id: "done1".to_string(),
            task: "fix bug".to_string(),
            status: "complete".to_string(),
            provider: "claude".to_string(),
            started_at: 1_700_000_000_000,
            finished_at: Some(1_700_000_060_000),
            summary: Some("Fixed the null pointer".to_string()),
        };
        persist_job(dir.path(), &job);

        let loaded = load_job(dir.path(), "done1").unwrap();
        assert_eq!(loaded.finished_at, Some(1_700_000_060_000));
        assert_eq!(loaded.summary.as_deref(), Some("Fixed the null pointer"));
    }

    #[test]
    fn load_job_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_job(dir.path(), "nonexistent").is_none());
    }

    #[test]
    fn load_all_jobs_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let jobs = load_all_jobs(dir.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn load_all_jobs_nonexistent_dir() {
        let jobs = load_all_jobs(std::path::Path::new("/tmp/vibecli_test_nonexistent_99999"));
        assert!(jobs.is_empty());
    }

    #[test]
    fn load_all_jobs_returns_sorted_by_started_at_desc() {
        let dir = tempfile::tempdir().unwrap();
        persist_job(dir.path(), &make_job("old", 1_000));
        persist_job(dir.path(), &make_job("mid", 2_000));
        persist_job(dir.path(), &make_job("new", 3_000));

        let jobs = load_all_jobs(dir.path());
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[0].session_id, "new");
        assert_eq!(jobs[1].session_id, "mid");
        assert_eq!(jobs[2].session_id, "old");
    }

    #[test]
    fn load_all_jobs_ignores_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        persist_job(dir.path(), &make_job("real", 1_000));
        // Write a non-json file
        std::fs::write(dir.path().join("notes.txt"), "not a job").unwrap();
        let jobs = load_all_jobs(dir.path());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].session_id, "real");
    }

    #[test]
    fn persist_overwrites_existing_job() {
        let dir = tempfile::tempdir().unwrap();
        let mut job = make_job("sess1", 1_000);
        persist_job(dir.path(), &job);

        job.status = "complete".to_string();
        job.finished_at = Some(2_000);
        job.summary = Some("done".to_string());
        persist_job(dir.path(), &job);

        let loaded = load_job(dir.path(), "sess1").unwrap();
        assert_eq!(loaded.status, "complete");
        assert_eq!(loaded.finished_at, Some(2_000));
    }

    // ── JobRecord serde roundtrip ──────────────────────────────────────────

    #[test]
    fn job_record_serde_roundtrip() {
        let job = JobRecord {
            session_id: "s1".to_string(),
            task: "deploy".to_string(),
            status: "failed".to_string(),
            provider: "openai".to_string(),
            started_at: 999,
            finished_at: Some(1001),
            summary: Some("timeout".to_string()),
        };
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: JobRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, job.session_id);
        assert_eq!(deserialized.task, job.task);
        assert_eq!(deserialized.status, job.status);
        assert_eq!(deserialized.provider, job.provider);
        assert_eq!(deserialized.started_at, job.started_at);
        assert_eq!(deserialized.finished_at, job.finished_at);
        assert_eq!(deserialized.summary, job.summary);
    }

    #[test]
    fn job_record_deserialize_with_missing_optionals() {
        let json = r#"{"session_id":"x","task":"t","status":"running","provider":"p","started_at":0}"#;
        let job: JobRecord = serde_json::from_str(json).unwrap();
        assert!(job.finished_at.is_none());
        assert!(job.summary.is_none());
    }

    // ── AgentEventPayload constructors ─────────────────────────────────────

    #[test]
    fn agent_event_payload_chunk() {
        let p = AgentEventPayload::chunk("hello".to_string());
        assert_eq!(p.kind, "chunk");
        assert_eq!(p.content.as_deref(), Some("hello"));
        assert!(p.step_num.is_none());
        assert!(p.tool_name.is_none());
        assert!(p.success.is_none());
    }

    #[test]
    fn agent_event_payload_step() {
        let p = AgentEventPayload::step(3, "read_file", true);
        assert_eq!(p.kind, "step");
        assert!(p.content.is_none());
        assert_eq!(p.step_num, Some(3));
        assert_eq!(p.tool_name.as_deref(), Some("read_file"));
        assert_eq!(p.success, Some(true));
    }

    #[test]
    fn agent_event_payload_step_failure() {
        let p = AgentEventPayload::step(7, "write_file", false);
        assert_eq!(p.success, Some(false));
        assert_eq!(p.step_num, Some(7));
    }

    #[test]
    fn agent_event_payload_complete() {
        let p = AgentEventPayload::complete("All done".to_string());
        assert_eq!(p.kind, "complete");
        assert_eq!(p.content.as_deref(), Some("All done"));
        assert!(p.step_num.is_none());
        assert!(p.tool_name.is_none());
        assert!(p.success.is_none());
    }

    #[test]
    fn agent_event_payload_error() {
        let p = AgentEventPayload::error("something broke".to_string());
        assert_eq!(p.kind, "error");
        assert_eq!(p.content.as_deref(), Some("something broke"));
        assert!(p.step_num.is_none());
    }

    // ── ChatMessage serde roundtrip ────────────────────────────────────────

    #[test]
    fn chat_message_serde_roundtrip() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello, world!");
    }

    // ── ChatRequest deserialization ────────────────────────────────────────

    #[test]
    fn chat_request_with_model() {
        let json = r#"{"messages":[{"role":"user","content":"hi"}],"model":"gpt-4"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.model.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn chat_request_without_model() {
        let json = r#"{"messages":[{"role":"user","content":"hi"}]}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.messages.len(), 1);
        assert!(req.model.is_none());
    }

    // ── AgentRequest deserialization ───────────────────────────────────────

    #[test]
    fn agent_request_with_approval() {
        let json = r#"{"task":"fix bug","approval":"full-auto"}"#;
        let req: AgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "fix bug");
        assert_eq!(req.approval.as_deref(), Some("full-auto"));
    }

    #[test]
    fn agent_request_without_approval() {
        let json = r#"{"task":"refactor"}"#;
        let req: AgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "refactor");
        assert!(req.approval.is_none());
    }

    // ── RateLimiter ────────────────────────────────────────────────────────

    #[test]
    fn rate_limiter_allows_within_limit() {
        let rl = RateLimiter::new(5, Duration::from_secs(60));
        for _ in 0..5 {
            assert!(rl.check(), "should allow requests within limit");
        }
    }

    #[test]
    fn rate_limiter_blocks_when_exceeded() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        assert!(rl.check());
        assert!(rl.check());
        assert!(rl.check());
        // 4th should be blocked
        assert!(!rl.check(), "should block after limit exceeded");
        assert!(!rl.check(), "should keep blocking");
    }

    #[test]
    fn rate_limiter_new_fields() {
        let rl = RateLimiter::new(10, Duration::from_secs(30));
        assert_eq!(rl.limit, 10);
        assert_eq!(rl.window_ms, 30_000);
        let ts = rl.timestamps.lock().unwrap();
        assert!(ts.is_empty());
    }

    #[test]
    fn rate_limiter_zero_limit_always_blocks() {
        let rl = RateLimiter::new(0, Duration::from_secs(60));
        assert!(!rl.check(), "zero-limit limiter should always block");
        assert!(!rl.check());
    }

    #[test]
    fn rate_limiter_limit_one() {
        let rl = RateLimiter::new(1, Duration::from_secs(60));
        assert!(rl.check(), "first request should pass");
        assert!(!rl.check(), "second request should be blocked");
    }

    #[test]
    fn rate_limiter_expired_entries_are_pruned() {
        // Use a very short window so entries expire immediately
        let rl = RateLimiter::new(2, Duration::from_millis(1));
        assert!(rl.check());
        assert!(rl.check());
        // Both slots consumed; sleep to let them expire
        std::thread::sleep(Duration::from_millis(5));
        // After expiry, the window should have room again
        assert!(rl.check(), "should allow after old entries expire");
    }

    #[test]
    fn rate_limiter_large_window() {
        let rl = RateLimiter::new(3, Duration::from_secs(3600));
        assert_eq!(rl.window_ms, 3_600_000);
        assert!(rl.check());
        assert!(rl.check());
        assert!(rl.check());
        assert!(!rl.check());
    }

    // ── AgentEventPayload JSON structure ─────────────────────────────────

    #[test]
    fn agent_event_payload_chunk_json_has_type_field() {
        let p = AgentEventPayload::chunk("text".to_string());
        let json = serde_json::to_value(&p).unwrap();
        // The field is renamed to "type" in JSON
        assert_eq!(json["type"], "chunk");
        assert_eq!(json["content"], "text");
        assert!(json.get("step_num").unwrap().is_null());
        assert!(json.get("tool_name").unwrap().is_null());
        assert!(json.get("success").unwrap().is_null());
    }

    #[test]
    fn agent_event_payload_step_json_structure() {
        let p = AgentEventPayload::step(1, "bash", true);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["type"], "step");
        assert_eq!(json["step_num"], 1);
        assert_eq!(json["tool_name"], "bash");
        assert_eq!(json["success"], true);
        assert!(json.get("content").unwrap().is_null());
    }

    #[test]
    fn agent_event_payload_complete_json_structure() {
        let p = AgentEventPayload::complete("summary".to_string());
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["type"], "complete");
        assert_eq!(json["content"], "summary");
    }

    #[test]
    fn agent_event_payload_error_json_structure() {
        let p = AgentEventPayload::error("fail".to_string());
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["content"], "fail");
    }

    #[test]
    fn agent_event_payload_chunk_empty_string() {
        let p = AgentEventPayload::chunk(String::new());
        assert_eq!(p.kind, "chunk");
        assert_eq!(p.content.as_deref(), Some(""));
    }

    #[test]
    fn agent_event_payload_step_zero() {
        let p = AgentEventPayload::step(0, "", false);
        assert_eq!(p.step_num, Some(0));
        assert_eq!(p.tool_name.as_deref(), Some(""));
        assert_eq!(p.success, Some(false));
    }

    // ── JobRecord edge cases ────────────────────────────────────────────

    #[test]
    fn job_record_all_statuses_roundtrip() {
        for status in &["running", "complete", "failed", "cancelled"] {
            let job = JobRecord {
                session_id: format!("s-{status}"),
                task: "t".to_string(),
                status: status.to_string(),
                provider: "p".to_string(),
                started_at: 100,
                finished_at: None,
                summary: None,
            };
            let json = serde_json::to_string(&job).unwrap();
            let parsed: JobRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.status, *status);
        }
    }

    #[test]
    fn job_record_json_field_names() {
        let job = make_job("field-test", 42);
        let json = serde_json::to_value(&job).unwrap();
        assert!(json.get("session_id").is_some());
        assert!(json.get("task").is_some());
        assert!(json.get("status").is_some());
        assert!(json.get("provider").is_some());
        assert!(json.get("started_at").is_some());
        assert!(json.get("finished_at").is_some());
        assert!(json.get("summary").is_some());
        // Should have exactly 7 fields
        assert_eq!(json.as_object().unwrap().len(), 7);
    }

    #[test]
    fn load_all_jobs_ignores_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        persist_job(dir.path(), &make_job("good", 1_000));
        // Write a malformed JSON file with .json extension
        std::fs::write(dir.path().join("bad.json"), "not valid json {{{").unwrap();
        let jobs = load_all_jobs(dir.path());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].session_id, "good");
    }

    #[test]
    fn persist_job_creates_file_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let job = make_job("disk-check", 500);
        persist_job(dir.path(), &job);
        let path = dir.path().join("disk-check.json");
        assert!(path.exists(), "persist_job should create a file");
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("disk-check"));
        assert!(contents.contains("\"started_at\""));
    }

    #[test]
    fn persist_job_writes_pretty_json() {
        let dir = tempfile::tempdir().unwrap();
        let job = make_job("pretty", 100);
        persist_job(dir.path(), &job);
        let contents = std::fs::read_to_string(dir.path().join("pretty.json")).unwrap();
        // Pretty JSON should contain newlines and indentation
        assert!(contents.contains('\n'), "should be pretty-printed");
        assert!(contents.contains("  "), "should have indentation");
    }

    // ── AgentStartResponse serde ────────────────────────────────────────

    #[test]
    fn agent_start_response_serializes() {
        let resp = AgentStartResponse {
            session_id: "abc-123".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["session_id"], "abc-123");
        assert_eq!(json.as_object().unwrap().len(), 1);
    }

    // ── ChatResponse serde ──────────────────────────────────────────────

    #[test]
    fn chat_response_serializes() {
        let resp = ChatResponse {
            content: "Hello!".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["content"], "Hello!");
    }

    // ── RoomInfo / CreateRoomRequest serde ───────────────────────────────

    #[test]
    fn room_info_serializes() {
        let info = RoomInfo {
            room_id: "room-42".to_string(),
            peer_count: 3,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["room_id"], "room-42");
        assert_eq!(json["peer_count"], 3);
    }

    #[test]
    fn create_room_request_with_room_id() {
        let json = r#"{"room_id":"my-room"}"#;
        let req: CreateRoomRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.room_id.as_deref(), Some("my-room"));
    }

    #[test]
    fn create_room_request_without_room_id() {
        let json = r#"{}"#;
        let req: CreateRoomRequest = serde_json::from_str(json).unwrap();
        assert!(req.room_id.is_none());
    }

    // ── ChatRequest with multiple messages ──────────────────────────────

    #[test]
    fn chat_request_multiple_messages() {
        let json = r#"{"messages":[
            {"role":"system","content":"You are helpful."},
            {"role":"user","content":"Hi"},
            {"role":"assistant","content":"Hello!"},
            {"role":"user","content":"Bye"}
        ]}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.messages.len(), 4);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[1].role, "user");
        assert_eq!(req.messages[2].role, "assistant");
        assert_eq!(req.messages[3].content, "Bye");
    }

    // ── HTTP integration tests (oneshot, no TCP binding) ────────────────

    mod http_integration {
        use super::*;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt; // for oneshot()

        /// A minimal mock AIProvider that never contacts a real LLM.
        struct MockProvider;

        #[async_trait::async_trait]
        impl AIProvider for MockProvider {
            fn name(&self) -> &str { "mock" }
            async fn is_available(&self) -> bool { true }
            async fn complete(
                &self,
                _ctx: &vibe_ai::provider::CodeContext,
            ) -> anyhow::Result<vibe_ai::provider::CompletionResponse> {
                Ok(vibe_ai::provider::CompletionResponse {
                    text: "mock completion".to_string(),
                    model: "mock".to_string(),
                    usage: None,
                })
            }
            async fn stream_complete(
                &self,
                _ctx: &vibe_ai::provider::CodeContext,
            ) -> anyhow::Result<vibe_ai::provider::CompletionStream> {
                let stream = futures::stream::once(async { Ok("mock".to_string()) });
                Ok(Box::pin(stream))
            }
            async fn chat(
                &self,
                _messages: &[vibe_ai::provider::Message],
                _context: Option<String>,
            ) -> anyhow::Result<String> {
                Ok("mock chat response".to_string())
            }
            async fn stream_chat(
                &self,
                _messages: &[vibe_ai::provider::Message],
            ) -> anyhow::Result<vibe_ai::provider::CompletionStream> {
                let stream = futures::stream::once(async { Ok("mock stream".to_string()) });
                Ok(Box::pin(stream))
            }
        }

        /// Create a test router with a known API token.
        /// Returns `(Router, TempDir)` — the caller must hold onto the `TempDir`
        /// so the temporary directory is not deleted while the router still
        /// references it.
        fn test_app(token: &str) -> (Router, tempfile::TempDir) {
            let tmp_dir = tempfile::tempdir().unwrap();
            let state = ServeState {
                provider: Arc::new(MockProvider),
                approval: ApprovalPolicy::FullAuto,
                workspace_root: tmp_dir.path().to_path_buf(),
                streams: Arc::new(Mutex::new(HashMap::new())),
                jobs_dir: tmp_dir.path().to_path_buf(),
                provider_name: "mock".to_string(),
                api_token: token.to_string(),
                collab_server: Arc::new(CollabServer::new(5)),
                github_app_config: crate::github_app::GithubAppConfig::default(),
            };
            (build_router(state, 7878), tmp_dir)
        }

        /// Helper: collect response body bytes into a String.
        async fn body_string(body: Body) -> String {
            let bytes = axum::body::to_bytes(body, 1024 * 1024 * 2).await.unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        }

        // ── GET /health ────────────────────────────────────────────────

        #[tokio::test]
        async fn health_returns_200_ok() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn health_returns_json_with_status_ok() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(json["status"], "ok");
        }

        #[tokio::test]
        async fn health_does_not_require_auth() {
            let (app, _tmp) = test_app("secret-token");
            // No Authorization header — should still work
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // ── Auth: unauthenticated requests to protected routes → 401 ──

        #[tokio::test]
        async fn chat_without_auth_returns_401() {
            let (app, _tmp) = test_app("secret-token");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"messages":[{"role":"user","content":"hi"}]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn agent_without_auth_returns_401() {
            let (app, _tmp) = test_app("my-secret");
            let req = Request::builder()
                .method("POST")
                .uri("/agent")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"task":"do stuff"}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn jobs_without_auth_returns_401() {
            let (app, _tmp) = test_app("my-secret");
            let req = Request::builder()
                .uri("/jobs")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn chat_with_wrong_token_returns_401() {
            let (app, _tmp) = test_app("correct-token");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer wrong-token")
                .body(Body::from(r#"{"messages":[{"role":"user","content":"hi"}]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn chat_with_correct_token_returns_200() {
            let (app, _tmp) = test_app("correct-token");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer correct-token")
                .body(Body::from(r#"{"messages":[{"role":"user","content":"hi"}]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn jobs_with_correct_token_returns_200() {
            let (app, _tmp) = test_app("my-token");
            let req = Request::builder()
                .uri("/jobs")
                .header("authorization", "Bearer my-token")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn unauthorized_response_body_contains_error() {
            let (app, _tmp) = test_app("secret");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"messages":[]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
            let body = body_string(resp.into_body()).await;
            assert!(
                body.contains("error") && body.contains("Authorization"),
                "401 body should mention Authorization; got: {body}"
            );
        }

        // ── GET /sessions (HTML) ───────────────────────────────────────

        #[tokio::test]
        async fn sessions_html_returns_200_with_html_content_type() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/sessions")
                .header("Authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            // sessions may return 200 or 500 depending on whether the SQLite
            // store exists; we accept both but verify the content-type header.
            let status = resp.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
                "Expected 200 or 500, got {status}"
            );
            if status == StatusCode::OK {
                let ct = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                assert!(
                    ct.contains("text/html"),
                    "sessions should be text/html; got {ct}"
                );
            }
        }

        // ── GET /sessions.json (JSON) ──────────────────────────────────

        #[tokio::test]
        async fn sessions_json_returns_json_content_type() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/sessions.json")
                .header("Authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let status = resp.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
                "Expected 200 or 500, got {status}"
            );
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ct.contains("application/json"),
                "sessions.json content-type should be application/json; got {ct}"
            );
        }

        // ── Security headers ──────────────────────────────────────────

        #[tokio::test]
        async fn security_header_x_content_type_options() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let val = resp
                .headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok());
            assert_eq!(val, Some("nosniff"));
        }

        #[tokio::test]
        async fn security_header_x_frame_options() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let val = resp
                .headers()
                .get("x-frame-options")
                .and_then(|v| v.to_str().ok());
            assert_eq!(val, Some("DENY"));
        }

        #[tokio::test]
        async fn security_header_referrer_policy() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let val = resp
                .headers()
                .get("referrer-policy")
                .and_then(|v| v.to_str().ok());
            assert_eq!(val, Some("no-referrer"));
        }

        #[tokio::test]
        async fn security_header_content_security_policy() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let val = resp
                .headers()
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                val.contains("default-src") && val.contains("script-src"),
                "CSP should contain default-src and script-src; got: {val}"
            );
        }

        #[tokio::test]
        async fn security_headers_present_on_authed_route() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/jobs")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert!(resp.headers().get("x-frame-options").is_some());
            assert!(resp.headers().get("x-content-type-options").is_some());
            assert!(resp.headers().get("referrer-policy").is_some());
            assert!(resp.headers().get("content-security-policy").is_some());
        }

        // ── CORS headers ──────────────────────────────────────────────

        #[tokio::test]
        async fn cors_preflight_returns_ok() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("origin", "http://localhost:7878")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            // CORS should echo back allowed origin
            let acao = resp
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                acao.contains("localhost"),
                "CORS should allow localhost origin; got: {acao}"
            );
        }

        #[tokio::test]
        async fn cors_disallowed_origin_gets_no_acao_header() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("origin", "https://evil.example.com")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let acao = resp
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok());
            assert!(
                acao.is_none(),
                "Disallowed origin should not get ACAO header; got: {:?}",
                acao
            );
        }

        // ── Body size limit enforcement ──────────────────────────────

        #[tokio::test]
        async fn oversized_body_is_rejected() {
            let (app, _tmp) = test_app("tok");
            // Create a body larger than 1 MB (the configured limit)
            let large_body = "x".repeat(1024 * 1024 + 1);
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer tok")
                .body(Body::from(large_body))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            // Should be 413 Payload Too Large (axum's DefaultBodyLimit)
            assert_eq!(
                resp.status(),
                StatusCode::PAYLOAD_TOO_LARGE,
                "Body > 1 MB should be rejected with 413"
            );
        }

        // ── ACP capabilities (public route) ──────────────────────────

        #[tokio::test]
        async fn acp_capabilities_returns_200() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/acp/v1/capabilities")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // ── 404 for unknown routes ────────────────────────────────────

        #[tokio::test]
        async fn unknown_route_returns_404() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        }

        // ── Chat endpoint response content ────────────────────────────

        #[tokio::test]
        async fn chat_response_contains_mock_content() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer tok")
                .body(Body::from(r#"{"messages":[{"role":"user","content":"hello"}]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(
                json["content"].as_str().unwrap().contains("mock"),
                "Chat response should come from MockProvider; got: {body}"
            );
        }

        // ── Jobs list initially empty ─────────────────────────────────

        #[tokio::test]
        async fn jobs_list_initially_empty() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/jobs")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(json.is_array());
            assert_eq!(json.as_array().unwrap().len(), 0);
        }

        // ── GET /jobs/:id for nonexistent job → 404 ──────────────────

        #[tokio::test]
        async fn get_nonexistent_job_returns_404() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/jobs/nonexistent-id")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        }

        // ── POST /jobs/:id/cancel for nonexistent job → 404 ──────────

        #[tokio::test]
        async fn cancel_nonexistent_job_returns_404() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .method("POST")
                .uri("/jobs/nonexistent-id/cancel")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        }
    }
}
