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
//!
//! # Usage
//!
//! ```bash
//! vibecli serve --port 7878 --provider ollama
//! ```

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
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
use tower_http::cors::{Any, CorsLayer};

use vibe_ai::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, Message, MessageRole};
use vibe_ai::provider::AIProvider;
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
        let _ = std::fs::write(path, json);
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
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
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

// ── Route handlers ────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn chat(
    State(state): State<ServeState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut accumulated = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => accumulated.push_str(&text),
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
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
) -> Result<Json<AgentStartResponse>, (StatusCode, String)> {
    let session_id = format!("{:x}", now_ms());

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
                    // Remove the stream after completion
                    let mut s = streams.lock().await;
                    s.remove(&sid);
                    // Broadcast final event then break
                    let _ = tx.send(p.clone());
                    break;
                }
                AgentEvent::Error(msg) => {
                    let p = AgentEventPayload::error(msg.clone());
                    // Persist failure
                    record.status = "failed".to_string();
                    record.finished_at = Some(now_ms());
                    record.summary = Some(msg);
                    persist_job(&jobs_dir, &record);
                    let mut s = streams.lock().await;
                    s.remove(&sid);
                    let _ = tx.send(p.clone());
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
) -> Result<Json<JobRecord>, (StatusCode, String)> {
    load_job(&state.jobs_dir, &id)
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job '{}' not found", id)))
}

async fn cancel_job(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<JobRecord>, (StatusCode, String)> {
    // Remove stream (ends SSE)
    {
        let mut streams = state.streams.lock().await;
        streams.remove(&id);
    }

    // Update persisted record
    let mut record = load_job(&state.jobs_dir, &id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job '{}' not found", id)))?;
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
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    use tokio_stream::wrappers::BroadcastStream;
    use futures::StreamExt;

    let rx = {
        let streams = state.streams.lock().await;
        let tx = streams
            .get(&session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Session '{}' not found", session_id)))?;
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

// ── Server startup ────────────────────────────────────────────────────────────

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

    let state = ServeState {
        provider,
        approval,
        workspace_root,
        streams: Arc::new(Mutex::new(HashMap::new())),
        jobs_dir,
        provider_name,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/agent", post(start_agent))
        .route("/stream/:session_id", get(stream_agent))
        .route("/jobs", get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        // Web session viewer
        .route("/sessions", get(sessions_index_html))
        .route("/sessions.json", get(sessions_json))
        .route("/view/:id", get(view_session))
        .layer(cors)
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[vibecli serve] Listening on http://{addr}");
    eprintln!("[vibecli serve] Jobs persisted at ~/.vibecli/jobs/");
    eprintln!("[vibecli serve] Session viewer at http://{addr}/sessions");

    axum::serve(listener, app).await?;
    Ok(())
}

// ── Web session viewer handlers ───────────────────────────────────────────────

async fn sessions_index_html() -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            format!("Failed to open session store: {}", e),
        ),
        Ok(store) => {
            let sessions = store.list_sessions(200).unwrap_or_default();
            let html = render_sessions_index_html(&sessions);
            (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
        }
    }
}

async fn sessions_json() -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "application/json")],
            format!("{{\"error\":\"{}\"}}", e),
        ),
        Ok(store) => {
            let sessions = store.list_sessions(200).unwrap_or_default();
            let json = serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".into());
            (StatusCode::OK, [("content-type", "application/json")], json)
        }
    }
}

async fn view_session(Path(id): Path<String>) -> impl IntoResponse {
    match SessionStore::open_default() {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            format!("Failed to open session store: {}", e),
        ),
        Ok(store) => match store.get_session_detail(&id) {
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("DB error: {}", e),
            ),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                [("content-type", "text/html; charset=utf-8")],
                format!("<h1>Session not found</h1><p>No session with ID: <code>{}</code></p><p><a href=\"/sessions\">All sessions</a></p>", id),
            ),
            Ok(Some(detail)) => {
                let html = render_session_html(&detail);
                (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
            }
        },
    }
}
