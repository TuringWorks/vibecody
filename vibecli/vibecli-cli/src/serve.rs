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
    http::{header, HeaderValue, Method, StatusCode},
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
    match serde_json::to_string_pretty(record) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("[serve] failed to persist job {}: {e}", record.session_id);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Err(e) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)) {
                    eprintln!("[serve] failed to set permissions on {}: {e}", path.display());
                }
            }
        }
        Err(e) => eprintln!("[serve] failed to serialize job {}: {e}", record.session_id),
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

/// Sanitize user-supplied input before echoing it in error messages.
/// Strips HTML/control characters and truncates to 200 chars.
fn sanitize_user_input(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(200)
        .collect()
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
                "error": format!("No skill with webhook_trigger '{}'", sanitize_user_input(&skill_name)),
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
            project_summary: None,
            task_context_files: vec![],
            memory_context: None,
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
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{}' not found", sanitize_user_input(&id))))
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
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{}' not found", sanitize_user_input(&id))))?;
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
            .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Session '{}' not found", sanitize_user_input(&session_id))))?;
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

// ── Agent-as-a-Service API (v1) ──────────────────────────────────────────────
//
// Full task lifecycle API with API key auth, webhook callbacks, browse tasks,
// and priority queuing. This is VibeCody's public agent framework API.

/// Request to create a new agent task via the v1 API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct V1TaskCreate {
    task: String,
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    mode: Option<String>, // "smart" | "rush" | "deep"
    #[serde(default)]
    approval: Option<String>, // "suggest" | "auto-edit" | "full-auto"
    #[serde(default)]
    webhook_url: Option<String>,
    #[serde(default)]
    priority: Option<u8>, // 0 (lowest) - 9 (highest), default 5
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

/// Full task status response.
#[derive(Debug, Serialize)]
struct V1TaskStatus {
    id: String,
    status: String,   // "queued" | "running" | "completed" | "failed" | "cancelled"
    task: String,
    mode: String,
    priority: u8,
    tags: Vec<String>,
    created_at: u64,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    summary: Option<String>,
    steps_completed: usize,
    webhook_url: Option<String>,
}

/// Request to create a browse (browser automation) task.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct V1BrowseCreate {
    url: String,
    task: String,
    #[serde(default)]
    headless: Option<bool>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    webhook_url: Option<String>,
}

/// Browse task status with screenshot history.
#[derive(Debug, Serialize)]
struct V1BrowseStatus {
    id: String,
    status: String,
    url: String,
    task: String,
    current_page: Option<String>,
    screenshots: Vec<V1Screenshot>,
    actions_taken: usize,
    created_at: u64,
    finished_at: Option<u64>,
    summary: Option<String>,
}

#[derive(Debug, Serialize)]
struct V1Screenshot {
    timestamp_ms: u64,
    action_before: String,
    page_url: String,
}

/// API key metadata for management endpoints.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
struct ApiKeyRecord {
    key_prefix: String,   // first 8 chars for display
    label: String,
    created_at: u64,
    permissions: Vec<String>, // "tasks", "browse", "chat", "admin"
    rate_limit: Option<usize>,
    #[serde(default)]
    active: bool,
}

// ── v1 API handlers ─────────────────────────────────────────────────────────

/// POST /v1/tasks — Create a new agent task.
async fn v1_create_task(
    State(state): State<ServeState>,
    Json(req): Json<V1TaskCreate>,
) -> impl IntoResponse {
    let session_id = format!("v1-{:016x}", now_ms());
    let mode = req.mode.as_deref().unwrap_or("smart");
    let priority = req.priority.unwrap_or(5);

    let record = JobRecord {
        session_id: session_id.clone(),
        task: req.task.clone(),
        status: "queued".to_string(),
        provider: state.provider_name.clone(),
        started_at: now_ms(),
        finished_at: None,
        summary: None,
    };
    persist_job(&state.jobs_dir, &record);

    // Spawn background agent
    let provider = state.provider.clone();
    let workspace = req.workspace.unwrap_or_else(|| state.workspace_root.to_string_lossy().to_string());
    let task = req.task.clone();
    let sid = session_id.clone();
    let jobs_dir = state.jobs_dir.clone();
    let provider_name = state.provider_name.clone();
    let webhook_url = req.webhook_url.clone();
    let timeout = req.timeout_secs.unwrap_or(300);

    tokio::spawn(async move {
        let mut record = JobRecord {
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

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            agent.run(&task, context, event_tx),
        ).await;

        match result {
            Ok(Ok(())) => {
                record.status = "completed".to_string();
                record.summary = Some("Task completed successfully".to_string());
            }
            Ok(Err(e)) => {
                record.status = "failed".to_string();
                record.summary = Some(format!("Task failed: {e}"));
            }
            Err(_) => {
                record.status = "failed".to_string();
                record.summary = Some(format!("Task timed out after {timeout}s"));
            }
        }
        record.finished_at = Some(now_ms());
        persist_job(&jobs_dir, &record);

        // Fire webhook callback if configured
        if let Some(url) = &webhook_url {
            let payload = serde_json::json!({
                "event": format!("task.{}", record.status),
                "task_id": sid,
                "status": record.status,
                "summary": record.summary,
                "finished_at": record.finished_at,
            });
            let client = reqwest::Client::new();
            let _ = client.post(url)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await;
        }
    });

    let status = V1TaskStatus {
        id: session_id,
        status: "queued".to_string(),
        task: req.task,
        mode: mode.to_string(),
        priority,
        tags: req.tags,
        created_at: now_ms(),
        started_at: None,
        finished_at: None,
        summary: None,
        steps_completed: 0,
        webhook_url: req.webhook_url,
    };

    (StatusCode::CREATED, Json(status))
}

/// GET /v1/tasks — List all tasks.
async fn v1_list_tasks(
    State(state): State<ServeState>,
) -> impl IntoResponse {
    let tasks = load_all_jobs(&state.jobs_dir);
    let statuses: Vec<serde_json::Value> = tasks.iter().map(|j| {
        serde_json::json!({
            "id": j.session_id,
            "status": j.status,
            "task": j.task,
            "provider": j.provider,
            "started_at": j.started_at,
            "finished_at": j.finished_at,
            "summary": j.summary,
        })
    }).collect();
    Json(serde_json::json!({ "tasks": statuses, "total": statuses.len() }))
}

/// GET /v1/tasks/:id — Get task status.
async fn v1_get_task(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(job) => {
            let status = serde_json::json!({
                "id": job.session_id,
                "status": job.status,
                "task": job.task,
                "provider": job.provider,
                "started_at": job.started_at,
                "finished_at": job.finished_at,
                "summary": job.summary,
            });
            (StatusCode::OK, Json(status)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        ).into_response(),
    }
}

/// POST /v1/tasks/:id/cancel — Cancel a running task.
async fn v1_cancel_task(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(mut job) => {
            if job.status == "running" || job.status == "queued" {
                job.status = "cancelled".to_string();
                job.finished_at = Some(now_ms());
                job.summary = Some("Cancelled by API request".to_string());
                persist_job(&state.jobs_dir, &job);
                (StatusCode::OK, Json(serde_json::json!({"id": id, "status": "cancelled"}))).into_response()
            } else {
                (StatusCode::CONFLICT, Json(serde_json::json!({"error": format!("Task is already {}", job.status)}))).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found"}))).into_response(),
    }
}

/// POST /v1/tasks/:id/feedback — Submit human feedback on a task.
async fn v1_task_feedback(
    State(state): State<ServeState>,
    Path(id): Path<String>,
    Json(feedback): Json<serde_json::Value>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(_) => {
            // Persist feedback alongside the job
            let feedback_path = state.jobs_dir.join(format!("{id}.feedback.json"));
            let _ = std::fs::write(&feedback_path, serde_json::to_string_pretty(&feedback).unwrap_or_default());
            (StatusCode::OK, Json(serde_json::json!({"status": "feedback_recorded"}))).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found"}))).into_response(),
    }
}

/// POST /v1/browse — Create a browser automation task.
async fn v1_create_browse(
    State(state): State<ServeState>,
    Json(req): Json<V1BrowseCreate>,
) -> impl IntoResponse {
    let session_id = format!("browse-{:016x}", now_ms());

    let status = V1BrowseStatus {
        id: session_id.clone(),
        status: "queued".to_string(),
        url: req.url.clone(),
        task: req.task.clone(),
        current_page: Some(req.url.clone()),
        screenshots: Vec::new(),
        actions_taken: 0,
        created_at: now_ms(),
        finished_at: None,
        summary: None,
    };

    // Persist as a job record
    let record = JobRecord {
        session_id: session_id.clone(),
        task: format!("[browse] {} — {}", req.url, req.task),
        status: "queued".to_string(),
        provider: state.provider_name.clone(),
        started_at: now_ms(),
        finished_at: None,
        summary: None,
    };
    persist_job(&state.jobs_dir, &record);

    (StatusCode::CREATED, Json(status))
}

/// GET /v1/browse/:id — Get browse task status.
async fn v1_get_browse(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(job) => {
            let status = V1BrowseStatus {
                id: job.session_id,
                status: job.status,
                url: "".to_string(),
                task: job.task,
                current_page: None,
                screenshots: Vec::new(),
                actions_taken: 0,
                created_at: job.started_at,
                finished_at: job.finished_at,
                summary: job.summary,
            };
            (StatusCode::OK, Json(status)).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Browse task not found"}))).into_response(),
    }
}

/// GET /v1/browse/:id/screenshots — Get screenshot history.
async fn v1_browse_screenshots(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(_) => {
            // Screenshots stored per-session in ~/.vibecli/recordings/{id}/
            let screenshots_dir = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vibecli")
                .join("recordings")
                .join(&id);
            let screenshots: Vec<V1Screenshot> = if screenshots_dir.exists() {
                std::fs::read_dir(&screenshots_dir)
                    .map(|entries| {
                        entries.filter_map(|e| {
                            let e = e.ok()?;
                            let name = e.file_name().to_string_lossy().to_string();
                            if name.ends_with(".png") {
                                Some(V1Screenshot {
                                    timestamp_ms: now_ms(),
                                    action_before: name.clone(),
                                    page_url: "".to_string(),
                                })
                            } else {
                                None
                            }
                        }).collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            (StatusCode::OK, Json(serde_json::json!({"id": id, "screenshots": screenshots}))).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Browse task not found"}))).into_response(),
    }
}

/// POST /v1/browse/:id/intervene — Human takeover of a browse session.
async fn v1_browse_intervene(
    State(state): State<ServeState>,
    Path(id): Path<String>,
    Json(action): Json<serde_json::Value>,
) -> impl IntoResponse {
    match load_job(&state.jobs_dir, &id) {
        Some(_) => {
            // Store intervention action
            let intervene_path = state.jobs_dir.join(format!("{id}.intervene.json"));
            let _ = std::fs::write(&intervene_path, serde_json::to_string_pretty(&action).unwrap_or_default());
            (StatusCode::OK, Json(serde_json::json!({"status": "intervention_recorded", "id": id}))).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Browse task not found"}))).into_response(),
    }
}

/// GET /v1/capabilities — Advertise agent framework capabilities.
async fn v1_capabilities() -> impl IntoResponse {
    Json(serde_json::json!({
        "framework": "VibeCody Agent Framework",
        "version": env!("CARGO_PKG_VERSION"),
        "capabilities": {
            "tasks": {
                "description": "General-purpose autonomous code agent tasks",
                "modes": ["smart", "rush", "deep"],
                "max_concurrent": 10,
                "timeout_max_secs": 3600,
            },
            "browse": {
                "description": "Browser automation via CDP (Chrome DevTools Protocol)",
                "headless": true,
                "actions": ["navigate", "click", "type", "scroll", "screenshot", "extract", "evaluate_js"],
            },
            "observe_act": {
                "description": "Continuous visual grounding loop (screenshot → LLM → action → verify)",
                "vision_providers": ["claude", "openai", "gemini"],
                "max_steps": 50,
            },
            "desktop": {
                "description": "Desktop GUI automation (mouse, keyboard, windows)",
                "platforms": ["macos", "linux", "windows"],
            },
            "chat": {
                "description": "Single-turn and streaming chat",
                "providers": 18,
            },
            "tools": {
                "description": "11 built-in tools + MCP extensibility",
                "builtin": ["ReadFile", "WriteFile", "ApplyPatch", "Bash", "SearchFiles", "ListDirectory", "WebSearch", "FetchUrl", "TaskComplete", "SpawnAgent", "Think"],
                "mcp": true,
            },
            "memory": {
                "description": "OpenMemory cognitive engine + Infinite Context",
                "sectors": ["episodic", "semantic", "procedural", "emotional", "reflective"],
            },
            "multi_agent": {
                "description": "Parallel agent teams with isolated worktrees",
                "max_depth": 5,
                "roles": ["Lead", "Teammate", "Reviewer", "Specialist"],
            },
            "webhooks": {
                "events": ["task.queued", "task.running", "task.completed", "task.failed", "task.cancelled"],
            },
            "gateway": {
                "platforms": 18,
                "protocols": ["REST", "SSE", "WebSocket", "Telegram", "Discord", "Slack", "Matrix", "IRC", "Teams"],
            },
        },
    }))
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
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT]);

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
        // OpenMemory — cognitive memory engine REST API
        .route("/memory/add", post(memory_add))
        .route("/memory/query", post(memory_query))
        .route("/memory/list", get(memory_list))
        .route("/memory/stats", get(memory_stats))
        .route("/memory/fact", post(memory_add_fact))
        .route("/memory/facts", get(memory_facts))
        .route("/memory/decay", post(memory_decay))
        .route("/memory/consolidate", post(memory_consolidate))
        .route("/memory/export", get(memory_export))
        // Vulnerability Scanner — industry-grade SCA + SAST
        .route("/vulnscan/scan", post(vulnscan_scan))
        .route("/vulnscan/file", post(vulnscan_file))
        .route("/vulnscan/summary", get(vulnscan_summary))
        // Agent-as-a-Service v1 API
        .route("/v1/tasks", post(v1_create_task))
        .route("/v1/tasks", get(v1_list_tasks))
        .route("/v1/tasks/:id", get(v1_get_task))
        .route("/v1/tasks/:id/cancel", post(v1_cancel_task))
        .route("/v1/tasks/:id/feedback", post(v1_task_feedback))
        .route("/v1/browse", post(v1_create_browse))
        .route("/v1/browse/:id", get(v1_get_browse))
        .route("/v1/browse/:id/screenshots", get(v1_browse_screenshots))
        .route("/v1/browse/:id/intervene", post(v1_browse_intervene))
        .route_layer(middleware::from_fn_with_state(limiter, rate_limit))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // More restrictive rate limiter for public routes: 10 requests per 60 seconds
    let public_limiter = Arc::new(RateLimiter::new(10, Duration::from_secs(60)));

    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/webhook/github", post(github_webhook))
        .route("/pair", get(pairing_handler))
        .route("/acp/v1/capabilities", get(acp_capabilities))
        .route("/v1/capabilities", get(v1_capabilities))
        .route("/ws/collab/:room_id", get(ws_collab_handler))
        .route_layer(middleware::from_fn_with_state(public_limiter, rate_limit));

    // Public routes (health check, GitHub webhook with HMAC, pairing, collab WS, ACP discovery)
    public_routes
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
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
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

    // Background: periodic OpenMemory maintenance (decay + consolidation every 24h)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
        interval.tick().await; // Skip first immediate tick
        loop {
            interval.tick().await;
            let mem_dir = openmemory_dir();
            if let Ok(mut store) = crate::open_memory::OpenMemoryStore::load(&mem_dir, "default") {
                let decayed = store.run_decay();
                let consolidated = store.consolidate();
                let _ = store.save();
                if decayed > 0 || !consolidated.is_empty() {
                    eprintln!(
                        "[openmemory] Daily maintenance: {} decayed, {} consolidated",
                        decayed, consolidated.len()
                    );
                }
            }
        }
    });

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[vibecli serve] Listening on http://{addr}");
    // Write the full token to a file (mode 0600) instead of logging it
    let token_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("daemon.token");
    std::fs::write(&token_path, &api_token).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o600));
    }
    let masked = format!("{}...{}", &api_token[..4], &api_token[api_token.len()-4..]);
    eprintln!("[vibecli serve] API token: {masked} (full token in ~/.vibecli/daemon.token)");
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
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Room '{}' not found", sanitize_user_input(&room_id))))?;
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

// ── OpenMemory REST API handlers ─────────────────────────────────────────────

fn openmemory_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vibecli")
        .join("openmemory")
}

fn load_memory_store() -> crate::open_memory::OpenMemoryStore {
    crate::open_memory::OpenMemoryStore::load(openmemory_dir(), "default")
        .unwrap_or_else(|_| crate::open_memory::OpenMemoryStore::new(openmemory_dir(), "default"))
}

#[derive(Deserialize)]
struct MemoryAddRequest {
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct MemoryQueryRequest {
    text: String,
    #[serde(default = "default_limit")]
    limit: usize,
    sector: Option<String>,
}

fn default_limit() -> usize { 10 }

#[derive(Deserialize)]
struct MemoryAddFactRequest {
    subject: String,
    predicate: String,
    object: String,
}

async fn memory_add(
    _state: State<ServeState>,
    Json(req): Json<MemoryAddRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    let id = store.add_with_tags(req.content, req.tags, std::collections::HashMap::new());
    let sector = store.get(&id).map(|m| m.sector.to_string()).unwrap_or_default();
    match store.save() {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id, "sector": sector }))),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("save failed: {e}")),
    }
}

async fn memory_query(
    _state: State<ServeState>,
    Json(req): Json<MemoryQueryRequest>,
) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let sector_filter = req.sector.as_deref().and_then(|s| s.parse().ok());
    let results = store.query_with_filters(&req.text, req.limit, sector_filter, None);

    let items: Vec<serde_json::Value> = results.iter().map(|r| {
        serde_json::json!({
            "id": r.memory.id,
            "content": r.memory.content,
            "sector": r.memory.sector.to_string(),
            "score": r.score,
            "similarity": r.similarity,
            "salience": r.effective_salience,
            "recency": r.recency_score,
            "tags": r.memory.tags,
            "pinned": r.memory.pinned,
        })
    }).collect();

    Json(serde_json::json!({ "results": items, "count": items.len() }))
}

async fn memory_list(_state: State<ServeState>) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let mems = store.list_memories(0, 100);
    let items: Vec<serde_json::Value> = mems.iter().map(|m| {
        serde_json::json!({
            "id": m.id,
            "content": m.content,
            "sector": m.sector.to_string(),
            "salience": m.effective_salience(),
            "tags": m.tags,
            "pinned": m.pinned,
            "created_at": m.created_at,
        })
    }).collect();
    Json(serde_json::json!({ "memories": items, "total": store.total_memories() }))
}

async fn memory_stats(_state: State<ServeState>) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let stats = store.sector_stats();
    let sectors: Vec<serde_json::Value> = stats.iter().map(|s| {
        serde_json::json!({
            "sector": s.sector.to_string(),
            "count": s.count,
            "avg_salience": s.avg_salience,
            "avg_age_days": s.avg_age_days,
            "pinned_count": s.pinned_count,
        })
    }).collect();
    Json(serde_json::json!({
        "total_memories": store.total_memories(),
        "total_waypoints": store.total_waypoints(),
        "total_facts": store.total_facts(),
        "sectors": sectors,
    }))
}

async fn memory_add_fact(
    _state: State<ServeState>,
    Json(req): Json<MemoryAddFactRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    let id = store.add_fact(req.subject, req.predicate, req.object);
    match store.save() {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("save failed: {e}")),
    }
}

async fn memory_facts(_state: State<ServeState>) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let facts: Vec<serde_json::Value> = store.query_current_facts().iter().map(|f| {
        serde_json::json!({
            "id": f.id,
            "subject": f.subject,
            "predicate": f.predicate,
            "object": f.object,
            "valid_from": f.valid_from,
            "valid_to": f.valid_to,
            "confidence": f.confidence,
        })
    }).collect();
    Json(serde_json::json!({ "facts": facts, "count": facts.len() }))
}

async fn memory_decay(_state: State<ServeState>) -> Json<serde_json::Value> {
    let mut store = load_memory_store();
    let purged = store.run_decay();
    let _ = store.save();
    Json(serde_json::json!({ "purged": purged, "remaining": store.total_memories() }))
}

async fn memory_consolidate(_state: State<ServeState>) -> Json<serde_json::Value> {
    let mut store = load_memory_store();
    let results = store.consolidate();
    let _ = store.save();
    Json(serde_json::json!({ "consolidated": results.len(), "remaining": store.total_memories() }))
}

async fn memory_export(_state: State<ServeState>) -> (StatusCode, String) {
    let store = load_memory_store();
    (StatusCode::OK, store.export_markdown())
}

// ── Vulnerability Scanner REST API ────────────────────────────────────────────

#[derive(Deserialize)]
struct VulnscanScanRequest {
    /// Lockfile content.
    content: String,
    /// Lockfile filename (for format detection).
    filename: String,
}

#[derive(Deserialize)]
struct VulnscanFileRequest {
    /// Source code content.
    content: String,
    /// File path (for language detection).
    file_path: String,
}

async fn vulnscan_scan(
    _state: State<ServeState>,
    Json(req): Json<VulnscanScanRequest>,
) -> Json<serde_json::Value> {
    let deps = crate::vulnerability_db::parse_lockfile(&req.filename, &req.content);
    let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
    scanner.scan_dependencies(&deps);
    let summary = scanner.summary();
    let findings: Vec<serde_json::Value> = scanner.active_findings().iter().map(|f| {
        serde_json::json!({
            "id": f.id, "severity": format!("{}", f.severity), "cvss": f.cvss_score,
            "title": f.title, "cve": f.cve_id, "cwe": f.cwe_id,
            "package": f.package, "version": f.installed_version, "fix": f.fixed_version,
            "epss": f.epss_score, "exploit": f.exploit_available, "remediation": f.remediation,
        })
    }).collect();
    Json(serde_json::json!({
        "summary": summary,
        "findings": findings,
        "packages_scanned": deps.len(),
    }))
}

async fn vulnscan_file(
    _state: State<ServeState>,
    Json(req): Json<VulnscanFileRequest>,
) -> Json<serde_json::Value> {
    let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
    scanner.scan_file(&req.file_path, &req.content);
    let findings: Vec<serde_json::Value> = scanner.active_findings().iter().map(|f| {
        serde_json::json!({
            "id": f.id, "severity": format!("{}", f.severity),
            "title": f.title, "cwe": f.cwe_id,
            "file": f.file_path, "line": f.line,
            "remediation": f.remediation, "owasp": f.owasp,
        })
    }).collect();
    Json(serde_json::json!({ "findings": findings, "count": findings.len() }))
}

async fn vulnscan_summary(_state: State<ServeState>) -> Json<serde_json::Value> {
    let scanner = crate::vulnerability_db::VulnerabilityScanner::new();
    let snapshot = crate::vulnerability_db::OsvSnapshotDb::new(
        crate::vulnerability_db::OsvSnapshotDb::default_path()
    );
    Json(serde_json::json!({
        "vuln_db_size": scanner.vuln_db_size(),
        "sast_rule_count": scanner.sast_rule_count(),
        "snapshot_exists": snapshot.exists(),
        "snapshot_advisory_count": snapshot.advisory_count(),
        "snapshot_age_hours": snapshot.age_hours(),
        "lockfile_formats": ["package-lock.json", "yarn.lock", "Cargo.lock", "requirements.txt", "poetry.lock", "go.sum", "Gemfile.lock"],
        "ecosystems": ["npm", "PyPI", "crates.io", "Go", "Maven", "RubyGems", "NuGet", "Packagist"],
    }))
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

        // ── json_error helper ────────────────────────────────────────

        #[tokio::test]
        async fn json_error_returns_correct_status_and_body() {
            let (status, Json(body)) = json_error(StatusCode::BAD_REQUEST, "bad input");
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["error"], "bad input");
            // Should have exactly one field
            assert_eq!(body.as_object().unwrap().len(), 1);
        }

        #[tokio::test]
        async fn json_error_internal_server_error() {
            let (status, Json(body)) = json_error(StatusCode::INTERNAL_SERVER_ERROR, "boom");
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["error"], "boom");
        }

        #[tokio::test]
        async fn json_error_not_found_with_dynamic_message() {
            let msg = format!("Job '{}' not found", "abc-123");
            let (status, Json(body)) = json_error(StatusCode::NOT_FOUND, msg);
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert!(body["error"].as_str().unwrap().contains("abc-123"));
        }

        // ── Health endpoint version field ────────────────────────────

        #[tokio::test]
        async fn health_response_includes_version() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(
                json.get("version").is_some(),
                "Health response should include a 'version' field; got: {body}"
            );
            let version = json["version"].as_str().unwrap();
            assert!(!version.is_empty(), "version should not be empty");
        }

        // ── 404 fallback returns JSON ────────────────────────────────

        #[tokio::test]
        async fn fallback_404_returns_json_error_body() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .uri("/does/not/exist")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(json["error"], "Not found");
        }

        // ── Auth edge cases ──────────────────────────────────────────

        #[tokio::test]
        async fn auth_with_no_bearer_prefix_returns_401() {
            let (app, _tmp) = test_app("my-token");
            let req = Request::builder()
                .uri("/jobs")
                .header("authorization", "my-token") // missing "Bearer " prefix
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn auth_with_empty_token_returns_401() {
            let (app, _tmp) = test_app("real-token");
            let req = Request::builder()
                .uri("/jobs")
                .header("authorization", "Bearer ")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        // ── CORS for Tauri origin ────────────────────────────────────

        #[tokio::test]
        async fn cors_allows_tauri_localhost_origin() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("origin", "https://tauri.localhost")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let acao = resp
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert_eq!(
                acao, "https://tauri.localhost",
                "CORS should allow tauri.localhost origin"
            );
        }

        #[tokio::test]
        async fn cors_allows_vite_dev_server_origin() {
            let (app, _tmp) = test_app("t");
            let req = Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("origin", "http://localhost:1420")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let acao = resp
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert_eq!(
                acao, "http://localhost:1420",
                "CORS should allow Vite dev server origin"
            );
        }

        // ── GET /jobs/:id 404 body contains error message ───────────

        #[tokio::test]
        async fn get_nonexistent_job_body_has_error_message() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/jobs/missing-id-xyz")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(
                json["error"].as_str().unwrap().contains("missing-id-xyz"),
                "404 body should mention the job ID; got: {body}"
            );
        }

        // ── Chat endpoint rejects malformed JSON ─────────────────────

        #[tokio::test]
        async fn chat_with_invalid_json_returns_4xx() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer tok")
                .body(Body::from("not valid json"))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let status = resp.status().as_u16();
            assert!(
                (400..500).contains(&status),
                "Malformed JSON should return 4xx; got {status}"
            );
        }

        // ── Chat with empty messages ─────────────────────────────────

        #[tokio::test]
        async fn chat_with_empty_messages_returns_200() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .header("authorization", "Bearer tok")
                .body(Body::from(r#"{"messages":[]}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            // The mock provider should still return a response
            assert_eq!(resp.status(), StatusCode::OK);
        }
    }

    // ── json_error unit tests (non-async) ────────────────────────────────

    #[test]
    fn json_error_empty_message() {
        let (status, Json(body)) = json_error(StatusCode::FORBIDDEN, "");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "");
    }

    #[test]
    fn json_error_accepts_string_type() {
        let msg = String::from("owned string error");
        let (status, Json(body)) = json_error(StatusCode::CONFLICT, msg);
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "owned string error");
    }

    // ── AgentEventPayload serde roundtrip ────────────────────────────────

    #[test]
    fn agent_event_payload_chunk_deserializes_back() {
        let p = AgentEventPayload::chunk("round trip".to_string());
        let json = serde_json::to_string(&p).unwrap();
        // Verify the JSON can be parsed as a generic Value and fields match
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["type"], "chunk");
        assert_eq!(val["content"], "round trip");
    }

    #[test]
    fn agent_event_payload_step_large_step_num() {
        let p = AgentEventPayload::step(usize::MAX, "tool", true);
        assert_eq!(p.step_num, Some(usize::MAX));
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["type"], "step");
    }

    // ── JobRecord with special characters ────────────────────────────────

    #[test]
    fn job_record_with_unicode_task() {
        let job = JobRecord {
            session_id: "unicode-test".to_string(),
            task: "Fix bug in \u{1F600} emoji handler \u{00E9}\u{00F1}".to_string(),
            status: "complete".to_string(),
            provider: "claude".to_string(),
            started_at: 100,
            finished_at: Some(200),
            summary: Some("Resolved \u{2714}".to_string()),
        };
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobRecord = serde_json::from_str(&json).unwrap();
        assert!(parsed.task.contains('\u{1F600}'));
        assert!(parsed.summary.as_deref().unwrap().contains('\u{2714}'));
    }

    #[test]
    fn persist_and_load_job_with_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let job = JobRecord {
            session_id: "uni".to_string(),
            task: "Handle caf\u{00E9} menu".to_string(),
            status: "running".to_string(),
            provider: "test".to_string(),
            started_at: 1,
            finished_at: None,
            summary: None,
        };
        persist_job(dir.path(), &job);
        let loaded = load_job(dir.path(), "uni").unwrap();
        assert_eq!(loaded.task, "Handle caf\u{00E9} menu");
    }

    // ── RateLimiter thread safety ────────────────────────────────────────

    #[test]
    fn rate_limiter_concurrent_access() {
        use std::sync::Arc;
        let rl = Arc::new(RateLimiter::new(100, Duration::from_secs(60)));
        let mut handles = vec![];
        for _ in 0..10 {
            let rl_clone = Arc::clone(&rl);
            handles.push(std::thread::spawn(move || {
                let mut count = 0u32;
                for _ in 0..20 {
                    if rl_clone.check() {
                        count += 1;
                    }
                }
                count
            }));
        }
        let total: u32 = handles.into_iter().map(|h| h.join().unwrap()).sum();
        // With limit 100, exactly 100 should succeed out of 200 attempts
        assert_eq!(total, 100, "Exactly 100 requests should pass; got {total}");
    }

    // ── ChatRequest deserialization edge cases ───────────────────────────

    #[test]
    fn chat_request_rejects_missing_messages_field() {
        let json = r#"{"model":"gpt-4"}"#;
        let result = serde_json::from_str::<ChatRequest>(json);
        assert!(result.is_err(), "ChatRequest without 'messages' should fail to parse");
    }

    #[test]
    fn agent_request_rejects_missing_task() {
        let json = r#"{"approval":"full-auto"}"#;
        let result = serde_json::from_str::<AgentRequest>(json);
        assert!(result.is_err(), "AgentRequest without 'task' should fail to parse");
    }
}
