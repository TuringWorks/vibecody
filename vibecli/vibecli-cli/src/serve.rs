//! VibeCLI HTTP daemon (`vibecli --serve`).
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
//! | POST   | `/memory/add`             | Add a cognitive memory               |
//! | POST   | `/memory/query`           | Semantic query with composite scoring |
//! | GET    | `/memory/list`            | List all memories                    |
//! | GET    | `/memory/stats`           | Sector counts + total_drawers + TurboQuant index info |
//! | POST   | `/memory/fact`            | Add a temporal fact                  |
//! | GET    | `/memory/facts`           | List active facts                    |
//! | POST   | `/memory/decay`           | Run salience decay                   |
//! | POST   | `/memory/consolidate`     | Sleep-cycle consolidation            |
//! | GET    | `/memory/export`          | Export memories as markdown          |
//! | POST   | `/memory/import`          | Import from mem0 / Zep / native JSON |
//! | POST   | `/memory/pin`             | Pin a memory (exempt from decay)     |
//! | POST   | `/memory/unpin`           | Unpin a memory (resume decay)        |
//! | POST   | `/memory/delete`          | Delete a memory permanently          |
//! | POST   | `/memory/chunk`           | Ingest text as verbatim 800-char chunks |
//! | GET    | `/memory/drawers/stats`   | Drawer count + Wing/Room distribution |
//! | POST   | `/memory/tunnel`          | Create a cross-project waypoint      |
//! | POST   | `/memory/auto-tunnel`     | Auto-detect and create tunnel waypoints |
//! | POST   | `/memory/context`         | Get 4-layer agent context block      |
//! | GET    | `/memory/benchmark`       | LongMemEval recall@K (default k=5)   |
//!
//! # Usage
//!
//! ```bash
//! vibecli --serve --host 0.0.0.0 --port 7878 --provider ollama
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
    convert::Infallible,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use rand::Rng;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use vibe_ai::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, Message, MessageRole};
use vibe_ai::provider::AIProvider;
use vibe_collab::{CollabMessage, CollabServer, PeerInfo, SyncBroadcast};
use crate::session_store::{SessionStore, render_session_html, render_sessions_index_html};
use crate::job_manager::{JobManager, CreateJobReq, JobStatus};
// Re-export so external callers that imported these via `crate::serve::*`
// keep compiling. `JobManager` owns the canonical definitions now.
pub use crate::job_manager::{JobRecord, AgentEventPayload};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Shared server state ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ServeState {
    pub provider: Arc<dyn AIProvider>,
    pub approval: ApprovalPolicy,
    pub workspace_root: PathBuf,
    /// Durable job queue + live event streams (SQLite-backed, M1+).
    pub job_manager: Arc<JobManager>,
    /// Legacy directory kept for per-job side-car files (`{id}.feedback.json`,
    /// `{id}.intervene.json`) and for machine_id hashing. Job records
    /// themselves live in `~/.vibecli/jobs.db` via `job_manager`.
    pub jobs_dir: PathBuf,
    pub provider_name: String,
    /// Bearer token required on all non-health/non-viewer endpoints.
    /// Generated randomly on daemon startup and printed to stderr.
    pub api_token: String,
    /// CRDT collaboration server for multiplayer editing.
    pub collab_server: Arc<CollabServer>,
    /// GitHub App webhook config for CI/CD review bot.
    pub github_app_config: crate::github_app::GithubAppConfig,
    /// Daemon startup time — used to compute uptime_secs in the beacon endpoint.
    pub started_at: std::time::Instant,
    /// Cached public URL (ngrok / Tailscale Funnel) — populated in the
    /// background shortly after daemon startup.  The beacon endpoint reads
    /// from here instead of blocking per-request.
    pub public_url_cache: Arc<std::sync::Mutex<Option<String>>>,
    /// Pluggable inference router (mistralrs vs ollama-proxy). `None` when
    /// the daemon is built without inference support — the `/api/*`
    /// handlers respond with 503 in that case so the rest of the daemon
    /// stays usable.
    pub inference_router: Option<Arc<crate::inference::Router>>,
    /// RL-OS — per-workspace run/episode/metric/artifact store. Backed by the
    /// same SQLite file as `WorkspaceStore` (slice 1: `docs/design/rl-os/01-persistence.md`).
    pub rl_run_store: Arc<crate::rl_runs::RunStore>,
    /// RL-OS — training executor. Slice 2 ships the Python sidecar variant;
    /// slice 7d swaps in a native runtime for inference workloads.
    pub rl_executor: Arc<dyn crate::rl_executor::TrainingExecutor>,
    /// RL-OS slice 6.5 — pool of long-lived inference runtimes, one per
    /// active deployment. Lazy-loaded on first /act call; explicit
    /// `/stop` drops the runtime.
    pub rl_runtime_pool: Arc<crate::rl_runtime::RuntimePool>,
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
    /// Override the model for this request (e.g. "llama3.2", "gpt-4o").
    #[serde(default)]
    #[allow(dead_code)]
    pub model: Option<String>,
    /// Override the provider for this request (e.g. "ollama", "claude").
    #[serde(default)]
    #[allow(dead_code)]
    pub provider: Option<String>,
    /// Phase 7 S3: optional per-request context negotiation for mobile
    /// / watch / IDE clients. When `Some`, the daemon picks a budget
    /// shape (`AgentKind`) and threads `MemoryToggles` overrides into
    /// the assembler so the spawned agent sees the right memory mix.
    /// When `None`, the daemon falls back to its existing behavior
    /// (empty AgentContext) — backward compatible with all existing
    /// clients.
    #[serde(default)]
    pub context_request: Option<ContextRequest>,
}

/// Wire shape for per-request context negotiation. All fields optional
/// so a minimal `{}` still parses cleanly. Daemon-side defaults: kind
/// = `CodingAgent` (the daemon's primary use case), openmemory toggles
/// = `MemoryToggles::default()` values (both true).
#[derive(Debug, Deserialize)]
pub struct ContextRequest {
    /// One of `Chat | CodingAgent | ResearchAgent | BackgroundJob`.
    /// `None` → daemon picks `CodingAgent`. Unknown kind → 400.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub openmemory_enabled: Option<bool>,
    #[serde(default)]
    pub openmemory_auto_inject: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AgentStartResponse {
    pub session_id: String,
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

/// Cached TurboQuant codec probe — runs once at startup (see
/// `serve()`). Reading from a static lets `/health` stay non-blocking
/// and stay correct even when called every second by a load balancer.
static CODEC_PROBE: std::sync::OnceLock<vibe_infer::kv_cache_tq::CodecProbeReport> =
    std::sync::OnceLock::new();

/// Snapshot of which AI providers have a stored API key in the encrypted
/// ProfileStore. Names only — never the values. Used by `/health` so any
/// feature that depends on "an AI provider is configured" (chat,
/// diffcomplete, recap LLM mode, …) inherits its readiness signal from
/// one canonical source instead of probing per-feature. Read fresh on
/// each call: ProfileStore is a local SQLite query, sub-millisecond.
fn configured_provider_names() -> Vec<String> {
    crate::profile_store::ProfileStore::new()
        .ok()
        .and_then(|s| s.list_api_key_providers("default").ok())
        .map(|mut v| {
            v.sort();
            v
        })
        .unwrap_or_default()
}

async fn health() -> impl IntoResponse {
    let hf_token_present = std::env::var("HF_TOKEN").map(|s| !s.is_empty()).unwrap_or(false);
    let codec_probe = CODEC_PROBE.get();
    let providers = configured_provider_names();
    let provider_count = providers.len();
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        // mistralrs picker default depends on whether the daemon can pull
        // gated meta-llama/* repos. Frontend reads this and overrides
        // PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"] to skip the 401 path.
        "hf_token_present": hf_token_present,
        "mistralrs_recommended_default": crate::inference::mistralrs::recommended_default_model(),
        // TurboQuant codec self-check — runs once at startup, cached. A
        // failed probe means the codec is producing degraded output;
        // operators should investigate before turning the codec on for
        // a real model. Null if the probe didn't run (very early /health
        // hit before serve() reached the probe call).
        "kv_cache_codec_probe": codec_probe,
        // Configured AI providers (by ProfileStore key presence). Names
        // only, never values. The canonical readiness signal for any
        // provider-dependent feature.
        "providers": {
            "configured_count": provider_count,
            "names": providers,
        },
        // Per-feature readiness. Each entry is { available, requires? }.
        // `requires` names the prerequisite the feature inherits — e.g.
        // diffcomplete needs at least one configured provider.
        "features": {
            "diffcomplete": {
                "available": provider_count > 0,
                "requires": "providers.configured_count > 0",
                "transport": "tauri-desktop",
            },
        },
    }))
}

/// List available models from Ollama (if reachable) plus the daemon's active provider.
async fn list_models(State(state): State<ServeState>) -> impl IntoResponse {
    let mut models: Vec<serde_json::Value> = Vec::new();

    // Active provider
    models.push(serde_json::json!({
        "id": state.provider.name(),
        "provider": state.provider_name,
        "active": true,
    }));

    // Try to list Ollama models
    if let Ok(ollama_models) = vibe_ai::providers::ollama::OllamaProvider::list_models(None).await {
        for m in ollama_models {
            let id = format!("ollama/{}", m);
            if !models.iter().any(|x| x["id"].as_str() == Some(&id)) {
                models.push(serde_json::json!({
                    "id": id,
                    "name": m,
                    "provider": "ollama",
                }));
            }
        }
    }

    Json(serde_json::json!({ "models": models }))
}

/// Serve the VibeCody Web client — browser-based zero-install mode.
async fn web_client_page() -> impl IntoResponse {
    let config = crate::web_client::WebClientConfig::default();
    let html = crate::web_client::web_client_html(&config);
    (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
}

/// Serve the favicon SVG for the web client.
async fn web_favicon() -> impl IntoResponse {
    let svg = crate::web_client::web_client_favicon_svg();
    (StatusCode::OK, [("content-type", "image/svg+xml")], svg)
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

/// Phase 7 S3: build an `AgentContext` for the in-process `/agent`
/// HTTP handler when the client supplied a `context_request`. Mirrors
/// the worker-loop helper at `main.rs::build_worker_agent_context` —
/// memory comes from the assembler, gated by the client's toggles.
///
/// `None` for `kind` defaults to `CodingAgent` (the daemon's primary
/// use case). Returns `Err` with a human-readable message for unknown
/// kinds; the handler maps that to HTTP 400.
///
/// Pure helper so unit tests can drive it with a tempdir workspace —
/// no `ServeState`, no AppHandle, no socket binding.
fn build_agent_context_from_request(
    workspace: std::path::PathBuf,
    task: &str,
    job_id: &str,
    request: &ContextRequest,
    git_branch: Option<String>,
) -> Result<AgentContext, String> {
    use crate::context_assembler::{
        AgentKind, ContextBudget, ContextPolicy, MemoryToggles, assemble_context,
        parse_agent_kind,
    };

    let kind = match request.kind.as_deref() {
        Some(s) => parse_agent_kind(s)?,
        None => AgentKind::CodingAgent,
    };

    let policy = match kind {
        AgentKind::Chat => ContextPolicy::Chat,
        _ => ContextPolicy::Agent {
            task: task.to_string(),
            job_id: Some(job_id.to_string()),
        },
    };

    let budget = ContextBudget::for_kind(kind);

    // Surface scratchpad whenever the policy is Agent — same logic as
    // the worker-loop helper. Chat policy doesn't read scratchpad
    // sections so the path is None there.
    let jobs_db_path = match kind {
        AgentKind::Chat => None,
        _ => Some(crate::job_manager::default_db_path()),
    };

    let defaults = MemoryToggles::default();
    let toggles = MemoryToggles {
        openmemory_enabled: request.openmemory_enabled.unwrap_or(defaults.openmemory_enabled),
        openmemory_auto_inject: request
            .openmemory_auto_inject
            .unwrap_or(defaults.openmemory_auto_inject),
        jobs_db_path,
    };

    let assembled = assemble_context(&workspace, &policy, &budget, &toggles);

    let project_summary = assembled
        .get("project_profile")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let memory_pieces: Vec<&str> = ["open_memory", "agent_scratchpad"]
        .into_iter()
        .filter_map(|name| assembled.get(name).filter(|s| !s.is_empty()))
        .collect();
    let memory_context = if memory_pieces.is_empty() {
        None
    } else {
        Some(memory_pieces.join("\n\n---\n\n"))
    };

    Ok(AgentContext {
        workspace_root: workspace,
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
        project_summary,
        // task_context_files left empty here — the relevance scanner
        // is workspace-CPU-heavy and the in-process daemon path is
        // not where we want that running synchronously inside an HTTP
        // handler. Worker-loop subprocess path populates it (S4).
        task_context_files: vec![],
        memory_context,
        auto_commit: false,
    })
}

async fn start_agent(
    State(state): State<ServeState>,
    Json(req): Json<AgentRequest>,
) -> Result<Json<AgentStartResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Phase 7 S3: validate context_request kind up-front so a bad
    // negotiation request never spawns a job. Production-friendly:
    // existing clients (no context_request) skip this branch entirely.
    if let Some(ref ctx_req) = req.context_request {
        if let Some(ref k) = ctx_req.kind {
            if let Err(e) = crate::context_assembler::parse_agent_kind(k) {
                return Err(json_error(StatusCode::BAD_REQUEST, e));
            }
        }
    }

    let session_id = state
        .job_manager
        .create(CreateJobReq {
            task: req.task.clone(),
            provider: state.provider_name.clone(),
            approval: req.approval.clone().unwrap_or_else(|| "auto".into()),
            workspace_root: state.workspace_root.to_string_lossy().to_string(),
            priority: 5,
            webhook_url: None,
            tags: vec![],
            quota_bucket: None,
        })
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state.job_manager.mark_running(&session_id).await;
    // Register the broadcast channel; events are persisted+fanned-out via
    // `job_manager.publish_event` below so reconnecting SSE clients can
    // replay with `?since_seq=N`.
    let _ = state.job_manager.open_stream(&session_id).await;

    let approval = match &req.approval {
        Some(s) => ApprovalPolicy::from_str(s),
        None => state.approval.clone(),
    };

    let task = req.task.clone();
    let sid = session_id.clone();
    let workspace_root = state.workspace_root.clone();
    let provider = state.provider.clone();
    let job_manager = state.job_manager.clone();
    // Move context_request into the spawned task. Empty default keeps
    // the existing-clients path zero-overhead.
    let ctx_request = req.context_request;

    tokio::spawn(async move {
        use crate::tool_executor::ToolExecutor;

        let executor = Arc::new(ToolExecutor::new(workspace_root.clone(), false));
        let agent = AgentLoop::new(provider, approval, executor);

        let git_branch = vibe_core::git::get_current_branch(&workspace_root).ok();

        // Phase 7 S3: when the client supplied a context_request, route
        // memory through the assembler. When None (every existing
        // mobile/IDE client today), keep the prior empty-context
        // behavior so this is purely additive.
        let context = match ctx_request {
            Some(ref cr) => match build_agent_context_from_request(
                workspace_root.clone(),
                &task,
                &sid,
                cr,
                git_branch.clone(),
            ) {
                Ok(c) => c,
                Err(_) => AgentContext {
                    workspace_root: workspace_root.clone(),
                    open_files: vec![],
                    git_branch: git_branch.clone(),
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
                    auto_commit: false,
                },
            },
            None => AgentContext {
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
                auto_commit: false,
            },
        };

        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentEvent>(64);

        tokio::spawn(async move {
            let _ = agent.run(&task, context, event_tx).await;
        });

        let mut completed = false;
        while let Some(event) = event_rx.recv().await {
            let payload = match event {
                AgentEvent::StreamChunk(text) => AgentEventPayload::chunk(text),
                AgentEvent::ToolCallExecuted(step) => {
                    AgentEventPayload::step(step.step_num, step.tool_call.name(), step.tool_result.success)
                }
                AgentEvent::Complete(summary) => {
                    let p = AgentEventPayload::complete(summary.clone());
                    let _ = job_manager.publish_event(&sid, p).await;
                    let _ = job_manager
                        .mark_terminal(&sid, JobStatus::Complete, Some(summary), None)
                        .await;
                    job_manager.close_stream(&sid).await;
                    completed = true;
                    break;
                }
                AgentEvent::Error(msg) => {
                    let p = AgentEventPayload::error(msg.clone());
                    let _ = job_manager.publish_event(&sid, p).await;
                    let _ = job_manager
                        .mark_terminal(&sid, JobStatus::Failed, Some(msg), None)
                        .await;
                    job_manager.close_stream(&sid).await;
                    completed = true;
                    break;
                }
                _ => continue,
            };
            let _ = job_manager.publish_event(&sid, payload).await;
        }

        // Fallback: if the agent task exited without sending Complete or Error
        // (e.g., panic, unexpected return, dropped channel), ensure the SSE
        // stream gets a completion event so clients don't hang forever.
        if !completed {
            let _ = job_manager
                .publish_event(&sid, AgentEventPayload::complete("Agent finished.".into()))
                .await;
            let _ = job_manager
                .mark_terminal(
                    &sid,
                    JobStatus::Complete,
                    Some("Agent finished.".into()),
                    None,
                )
                .await;
            job_manager.close_stream(&sid).await;
        }
    });

    Ok(Json(AgentStartResponse { session_id }))
}

// ── Job endpoints ─────────────────────────────────────────────────────────────

async fn list_jobs(
    State(state): State<ServeState>,
) -> Json<Vec<JobRecord>> {
    Json(state.job_manager.list().await)
}

async fn get_job(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<JobRecord>, (StatusCode, Json<serde_json::Value>)> {
    state
        .job_manager
        .get(&id)
        .await
        .map(Json)
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{}' not found", sanitize_user_input(&id))))
}

async fn cancel_job(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<JobRecord>, (StatusCode, Json<serde_json::Value>)> {
    let record = state
        .job_manager
        .cancel(&id, Some("user requested".into()))
        .await
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Job '{}' not found", sanitize_user_input(&id))))?;
    Ok(Json(record))
}

#[derive(Debug, Default, serde::Deserialize)]
struct StreamQuery {
    /// Replay events with `seq > since_seq` before switching to live.
    /// Omit to get live only. Use `0` to replay from the beginning.
    since_seq: Option<u64>,
}

async fn stream_agent(
    Path(session_id): Path<String>,
    Query(q): Query<StreamQuery>,
    State(state): State<ServeState>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)> {
    use tokio_stream::wrappers::BroadcastStream;
    use futures::StreamExt;

    // Replay first (if requested), so a client that reconnects with
    // `since_seq=N` gets every event persisted after N before the live
    // stream resumes. Running the replay query before subscribe() gives us
    // a small window where a fresh event could land in the broadcast but
    // not yet in our replayed snapshot — publish_event persists before it
    // broadcasts, so the event's seq will be <= the max in replay OR will
    // show up live, never lost. A reconnecting client that echoes the
    // highest seq it received will never see duplicates.
    let replay: Vec<AgentEventPayload> = match q.since_seq {
        Some(n) => state
            .job_manager
            .replay_events(&session_id, n)
            .await
            .into_iter()
            .map(|(_seq, p)| p)
            .collect(),
        None => Vec::new(),
    };

    // Subscribe to live *after* replay is fetched so we minimise the
    // overlap window; publish_event's persist-then-broadcast order means
    // any overlap shows up as a duplicate at worst — client tracks seq.
    let rx = state
        .job_manager
        .subscribe(&session_id)
        .await
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, format!("Session '{}' not found", sanitize_user_input(&session_id))))?;

    let replay_stream = futures::stream::iter(replay.into_iter().map(|payload| {
        let json = serde_json::to_string(&payload).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().data(json))
    }));

    let live_stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(payload) => {
                let json = serde_json::to_string(&payload).ok()?;
                Some(Ok(Event::default().data(json)))
            }
            Err(_) => None,
        }
    });

    let stream = replay_stream.chain(live_stream);

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
    let workspace = req.context
        .as_ref()
        .and_then(|c| c.workspace_root.clone())
        .unwrap_or_else(|| state.workspace_root.to_string_lossy().to_string());

    let session_id = match state
        .job_manager
        .create(CreateJobReq {
            task: req.task.clone(),
            provider: state.provider_name.clone(),
            approval: "auto".into(),
            workspace_root: workspace.clone(),
            priority: 5,
            webhook_url: None,
            tags: vec!["acp".into()],
            quota_bucket: None,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let status = crate::acp::AcpTaskStatus {
        id: session_id.clone(),
        status: "pending".to_string(),
        summary: Some(format!("Task queued: {}", req.task)),
        files_modified: Vec::new(),
        steps_completed: 0,
    };

    let provider = state.provider.clone();
    let task = req.task.clone();
    let sid = session_id.clone();
    let job_manager = state.job_manager.clone();

    tokio::spawn(async move {
        let _ = job_manager.mark_running(&sid).await;

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
        let _ = agent.run(&task, context, event_tx).await;

        let _ = job_manager
            .mark_terminal(
                &sid,
                JobStatus::Complete,
                Some("ACP task completed".into()),
                None,
            )
            .await;
    });

    (StatusCode::CREATED, Json(status)).into_response()
}

/// Get ACP task status.
async fn acp_get_task(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Some(job) = state.job_manager.get(&id).await {
        let status = crate::acp::AcpTaskStatus {
            id: job.session_id,
            status: job.status,
            summary: job.summary,
            files_modified: Vec::new(),
            steps_completed: job.steps_completed as usize,
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
#[allow(dead_code)] // Fields populated by serde deserialization
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
#[allow(dead_code)] // Fields populated by serde deserialization
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
#[allow(dead_code)] // Prepared for API key management endpoints
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
    let mode = req.mode.as_deref().unwrap_or("smart").to_string();
    let priority = req.priority.unwrap_or(5);
    let workspace = req
        .workspace
        .clone()
        .unwrap_or_else(|| state.workspace_root.to_string_lossy().to_string());

    let session_id = match state
        .job_manager
        .create(CreateJobReq {
            task: req.task.clone(),
            provider: state.provider_name.clone(),
            approval: req.approval.clone().unwrap_or_else(|| "auto".into()),
            workspace_root: workspace.clone(),
            priority,
            webhook_url: req.webhook_url.clone(),
            tags: req.tags.clone(),
            quota_bucket: None,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // Spawn background agent
    let provider = state.provider.clone();
    let task = req.task.clone();
    let sid = session_id.clone();
    let job_manager = state.job_manager.clone();
    let webhook_url = req.webhook_url.clone();
    let timeout = req.timeout_secs.unwrap_or(300);

    tokio::spawn(async move {
        let _ = job_manager.mark_running(&sid).await;

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
        )
        .await;

        let (status, summary) = match result {
            Ok(Ok(())) => (JobStatus::Complete, "Task completed successfully".to_string()),
            Ok(Err(e)) => (JobStatus::Failed, format!("Task failed: {e}")),
            Err(_) => (
                JobStatus::Failed,
                format!("Task timed out after {timeout}s"),
            ),
        };
        let _ = job_manager
            .mark_terminal(&sid, status, Some(summary.clone()), None)
            .await;

        // Fire webhook callback if configured. Uses the retry loop in
        // `JobManager::deliver_webhook` (exp-backoff up to 5 attempts,
        // final outcome persisted to `webhook_deliveries` for the
        // dead-letter queue).
        if let Some(url) = &webhook_url {
            let payload = serde_json::json!({
                "event": format!("task.{}", status.as_str()),
                "task_id": sid,
                "status": status.as_str(),
                "summary": summary,
                "finished_at": now_ms(),
            });
            let _ = job_manager.deliver_webhook(&sid, url, &payload).await;
        }
    });

    let status = V1TaskStatus {
        id: session_id,
        status: "queued".to_string(),
        task: req.task,
        mode,
        priority,
        tags: req.tags,
        created_at: now_ms(),
        started_at: None,
        finished_at: None,
        summary: None,
        steps_completed: 0,
        webhook_url: req.webhook_url,
    };

    (StatusCode::CREATED, Json(status)).into_response()
}

/// GET /v1/tasks — List all tasks.
async fn v1_list_tasks(
    State(state): State<ServeState>,
) -> impl IntoResponse {
    let tasks = state.job_manager.list().await;
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
    match state.job_manager.get(&id).await {
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

/// GET /v1/metrics/jobs — JobManager counters + queue-depth gauges.
async fn v1_jobs_metrics(State(state): State<ServeState>) -> impl IntoResponse {
    let snap = state.job_manager.metrics_snapshot().await;
    Json(snap)
}

/// POST /v1/tasks/:id/cancel — Cancel a running task.
async fn v1_cancel_task(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let existing = match state.job_manager.get(&id).await {
        Some(j) => j,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Task not found"})),
            )
                .into_response();
        }
    };
    if existing.status == "running" || existing.status == "queued" {
        let _ = state
            .job_manager
            .cancel(&id, Some("Cancelled by API request".into()))
            .await;
        (
            StatusCode::OK,
            Json(serde_json::json!({"id": id, "status": "cancelled"})),
        )
            .into_response()
    } else {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": format!("Task is already {}", existing.status)})),
        )
            .into_response()
    }
}

/// POST /v1/tasks/:id/feedback — Submit human feedback on a task.
async fn v1_task_feedback(
    State(state): State<ServeState>,
    Path(id): Path<String>,
    Json(feedback): Json<serde_json::Value>,
) -> impl IntoResponse {
    match state.job_manager.get(&id).await {
        Some(_) => {
            // Persist feedback alongside the job
            let feedback_path = state.jobs_dir.join(format!("{id}.feedback.json"));
            let _ = std::fs::create_dir_all(&state.jobs_dir);
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
    let session_id = match state
        .job_manager
        .create(CreateJobReq {
            task: format!("[browse] {} — {}", req.url, req.task),
            provider: state.provider_name.clone(),
            approval: "auto".into(),
            workspace_root: state.workspace_root.to_string_lossy().to_string(),
            priority: 5,
            webhook_url: req.webhook_url.clone(),
            tags: vec!["browse".into()],
            quota_bucket: None,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

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

    (StatusCode::CREATED, Json(status)).into_response()
}

/// GET /v1/browse/:id — Get browse task status.
async fn v1_get_browse(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.job_manager.get(&id).await {
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
    match state.job_manager.get(&id).await {
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
    match state.job_manager.get(&id).await {
        Some(_) => {
            // Store intervention action
            let intervene_path = state.jobs_dir.join(format!("{id}.intervene.json"));
            let _ = std::fs::create_dir_all(&state.jobs_dir);
            let _ = std::fs::write(&intervene_path, serde_json::to_string_pretty(&action).unwrap_or_default());
            (StatusCode::OK, Json(serde_json::json!({"status": "intervention_recorded", "id": id}))).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Browse task not found"}))).into_response(),
    }
}

// ── Recap & Resume — F1.2 HTTP surface ─────────────────────────────────────

/// POST /v1/recap request body. F1.2 only honors `kind = "session"` and
/// `generator = "heuristic"`. Other generators return 501 so clients
/// can probe support without guessing — the LLM/auto path lands in
/// its own slice with the prompt + provider routing.
#[derive(Debug, Deserialize)]
pub struct RecapRequest {
    /// One of `session | job | diff_chain`. F1.2 only supports
    /// `session`; the other two return 400 until their own slices land.
    pub kind: String,
    pub subject_id: String,
    /// When `false` (default), an existing recap for the same
    /// `(subject_id, last_message_id)` is returned unchanged. When
    /// `true`, the existing row is dropped and a fresh recap is
    /// generated.
    #[serde(default)]
    pub force: bool,
    /// `heuristic` (default for F1.2) | `llm` | `auto`. Anything but
    /// heuristic returns 501 in this slice.
    #[serde(default = "default_generator")]
    pub generator: String,
    /// Reserved for future LLM slice; F1.2 ignores these but accepts
    /// them so existing clients don't get rejected on field presence.
    #[serde(default)]
    #[allow(dead_code)]
    pub provider: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub model: Option<String>,
}

fn default_generator() -> String {
    "heuristic".to_string()
}

/// PATCH /v1/recap/:id request body. All three fields required — a
/// user edit replaces the prior heuristic/LLM output wholesale. The
/// row's id, subject_id, last_message_id, generated_at, and artifacts
/// are preserved by the daemon (artifacts are inferred from steps,
/// not from the recap body — they don't belong in a "user-edited
/// prose" surface).
#[derive(Debug, Deserialize)]
pub struct RecapPatch {
    pub headline: String,
    pub bullets: Vec<String>,
    pub next_actions: Vec<String>,
}

/// GET /v1/recap query params. `kind` + `subject_id` together select
/// the timeline for one subject; `limit` defaults to 20.
#[derive(Debug, Deserialize)]
pub struct RecapListQuery {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default = "default_recap_limit")]
    pub limit: usize,
}

fn default_recap_limit() -> usize {
    20
}

// ── Pure helpers for the recap routes ───────────────────────────────────────
//
// Each handler is a thin shell that opens `SessionStore::open_default()`
// and delegates to the matching `do_v1_recap_*` helper. The helpers take
// a borrowed `SessionStore` so unit tests can drive them with a tempdir-
// backed store — same pattern as QW2's `build_mobile_context_response`.

/// Pure builder for `POST /v1/recap`. Returns `(status, body)` so the
/// handler can `Json(...)` without re-implementing the error mapping.
pub(crate) fn do_v1_recap_post(
    store: &SessionStore,
    req: &RecapRequest,
) -> (StatusCode, serde_json::Value) {
    if req.kind != "session" {
        return (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!(
                    "kind {:?} not supported in F1.2; only \"session\" is implemented",
                    req.kind
                )
            }),
        );
    }
    if req.generator != "heuristic" {
        return (
            StatusCode::NOT_IMPLEMENTED,
            serde_json::json!({
                "error": format!(
                    "generator {:?} not implemented in F1.2; only \"heuristic\" is supported",
                    req.generator
                )
            }),
        );
    }

    let detail = match store.get_session_detail(&req.subject_id) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                serde_json::json!({
                    "error": format!("session {:?} not found", req.subject_id)
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("failed to load session: {e}")}),
            );
        }
    };

    // Idempotency / force=true logic. The (subject_id, last_message_id)
    // pair is the unique key. F1.1 already enforces it on insert.
    let last_message_id = detail.messages.last().map(|m| m.id);
    if req.force {
        // Drop any prior recap for this (subject, last_msg) so the
        // INSERT can proceed. Without this, the unique index conflicts.
        if let Ok(Some(existing)) =
            store.get_recap_by_subject_and_last_msg(&req.subject_id, last_message_id)
        {
            let _ = store.delete_recap(&existing.id);
        }
    }

    let recap = crate::recap::heuristic_recap(&detail);
    match store.insert_recap(&recap) {
        Ok(stored) => match serde_json::to_value(&stored) {
            Ok(v) => (StatusCode::OK, v),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("recap serialize: {e}")}),
            ),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap insert: {e}")}),
        ),
    }
}

pub(crate) fn do_v1_recap_get(
    store: &SessionStore,
    id: &str,
) -> (StatusCode, serde_json::Value) {
    match store.get_recap_by_id(id) {
        Ok(Some(r)) => match serde_json::to_value(&r) {
            Ok(v) => (StatusCode::OK, v),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("serialize: {e}")}),
            ),
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "recap not found"}),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap load: {e}")}),
        ),
    }
}

pub(crate) fn do_v1_recap_list(
    store: &SessionStore,
    q: &RecapListQuery,
) -> (StatusCode, serde_json::Value) {
    let kind = match q.kind.as_deref() {
        Some(k) => k,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": "missing required query param `kind`"}),
            );
        }
    };
    if kind != "session" {
        return (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!(
                    "kind {kind:?} not supported in F1.2; only \"session\" is implemented"
                )
            }),
        );
    }
    let subject_id = match q.subject_id.as_deref() {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": "missing required query param `subject_id`"}),
            );
        }
    };

    match store.list_recaps_for_subject(subject_id, q.limit) {
        Ok(rows) => {
            let count = rows.len();
            match serde_json::to_value(&rows) {
                Ok(v) => (
                    StatusCode::OK,
                    serde_json::json!({"recaps": v, "count": count}),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({"error": format!("serialize: {e}")}),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap list: {e}")}),
        ),
    }
}

pub(crate) fn do_v1_recap_patch(
    store: &SessionStore,
    id: &str,
    patch: &RecapPatch,
) -> (StatusCode, serde_json::Value) {
    // Load the prior row so we can preserve artifacts + resume_hint.
    // PATCH only edits the prose surface (headline, bullets,
    // next_actions); structured fields stay intact.
    let prior = match store.get_recap_by_id(id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                serde_json::json!({"error": "recap not found"}),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("recap load: {e}")}),
            );
        }
    };

    match store.update_recap(
        id,
        &patch.headline,
        &patch.bullets,
        &patch.next_actions,
        &prior.artifacts,
        prior.resume_hint.as_ref(),
    ) {
        Ok(Some(updated)) => match serde_json::to_value(&updated) {
            Ok(v) => (StatusCode::OK, v),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("serialize: {e}")}),
            ),
        },
        Ok(None) => (
            // The pre-flight load above already 404'd on missing rows,
            // so reaching here means the row vanished mid-write — surface
            // it as a 404 too.
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "recap not found"}),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap update: {e}")}),
        ),
    }
}

pub(crate) fn do_v1_recap_delete(
    store: &SessionStore,
    id: &str,
) -> (StatusCode, serde_json::Value) {
    match store.delete_recap(id) {
        Ok(()) => (StatusCode::NO_CONTENT, serde_json::json!({})),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap delete: {e}")}),
        ),
    }
}

// ── HTTP handlers (thin shells around the helpers) ─────────────────────────

fn open_default_or_500(
) -> Result<SessionStore, (StatusCode, Json<serde_json::Value>)> {
    SessionStore::open_default().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("session store unavailable: {e}")
            })),
        )
    })
}

// ── J1.3: Pure helpers for `kind=job` recap routes ─────────────────────────
//
// Mirrors the F1.2 `do_v1_recap_*` helpers but routes through `JobManager`
// instead of `SessionStore`. Same `(StatusCode, serde_json::Value)` return
// shape so the HTTP shell can `.json()` the result without re-implementing
// error mapping. Async because `JobManager` lives behind a tokio `Mutex`.

pub(crate) async fn do_v1_recap_post_job(
    jm: &JobManager,
    req: &RecapRequest,
) -> (StatusCode, serde_json::Value) {
    if req.kind != "job" {
        return (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!(
                    "do_v1_recap_post_job called with kind {:?}; routing bug",
                    req.kind
                )
            }),
        );
    }
    if req.generator != "heuristic" {
        return (
            StatusCode::NOT_IMPLEMENTED,
            serde_json::json!({
                "error": format!(
                    "generator {:?} not implemented in J1.3; only \"heuristic\" is supported",
                    req.generator
                )
            }),
        );
    }

    let (job, events) = match jm.fetch_job_with_events(&req.subject_id).await {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                serde_json::json!({
                    "error": format!("job {:?} not found", req.subject_id)
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("failed to load job: {e}")}),
            );
        }
    };

    // Idempotency: J1.1's insert is upsert on (subject_id, last_event_seq).
    // `force=true` deletes any prior recap for that (subject, seq) so a
    // fresh row is generated.
    let last_event_seq = events.last().map(|(s, _)| *s as i64);
    if req.force {
        if let Ok(Some(existing)) = jm
            .get_job_recap_by_subject_and_seq(&req.subject_id, last_event_seq)
            .await
        {
            let _ = jm.delete_job_recap(&existing.id).await;
        }
    }

    let recap = crate::recap::heuristic_job_recap(&job, &events);
    let stored_id = match jm.insert_job_recap(&recap).await {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("recap insert: {e}")}),
            );
        }
    };
    // Re-load via id so the response carries the persisted shape (the
    // freshly-generated `recap.id` is replaced by the existing row's id
    // when J1.1's idempotency hit).
    match jm.get_job_recap_by_id(&stored_id).await {
        Ok(Some(stored)) => match serde_json::to_value(&stored) {
            Ok(v) => (StatusCode::OK, v),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("recap serialize: {e}")}),
            ),
        },
        Ok(None) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": "recap vanished after insert"}),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap reload: {e}")}),
        ),
    }
}

pub(crate) async fn do_v1_recap_get_job(
    jm: &JobManager,
    id: &str,
) -> (StatusCode, serde_json::Value) {
    match jm.get_job_recap_by_id(id).await {
        Ok(Some(r)) => match serde_json::to_value(&r) {
            Ok(v) => (StatusCode::OK, v),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("serialize: {e}")}),
            ),
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "recap not found"}),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap load: {e}")}),
        ),
    }
}

pub(crate) async fn do_v1_recap_list_job(
    jm: &JobManager,
    q: &RecapListQuery,
) -> (StatusCode, serde_json::Value) {
    let subject_id = match q.subject_id.as_deref() {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": "missing required query param `subject_id`"}),
            );
        }
    };
    match jm.list_job_recaps_for_subject(subject_id, q.limit).await {
        Ok(rows) => {
            let count = rows.len();
            match serde_json::to_value(&rows) {
                Ok(v) => (
                    StatusCode::OK,
                    serde_json::json!({"recaps": v, "count": count}),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({"error": format!("serialize: {e}")}),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap list: {e}")}),
        ),
    }
}

pub(crate) async fn do_v1_recap_delete_job(
    jm: &JobManager,
    id: &str,
) -> (StatusCode, serde_json::Value) {
    match jm.delete_job_recap(id).await {
        Ok(()) => (StatusCode::NO_CONTENT, serde_json::json!({})),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({"error": format!("recap delete: {e}")}),
        ),
    }
}

async fn v1_recap_post(
    State(state): State<ServeState>,
    Json(req): Json<RecapRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (status, body) = match req.kind.as_str() {
        "session" => {
            let store = match open_default_or_500() {
                Ok(s) => s,
                Err(e) => return e,
            };
            do_v1_recap_post(&store, &req)
        }
        "job" => do_v1_recap_post_job(&state.job_manager, &req).await,
        other => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!(
                    "kind {other:?} not supported; valid: \"session\" | \"job\""
                )
            }),
        ),
    };
    (status, Json(body))
}

async fn v1_recap_get(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Try session store first; if not found, fall through to JobsDb. Recap
    // ids are UUIDs across both stores, so there is no collision risk.
    let store = match open_default_or_500() {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (status, body) = do_v1_recap_get(&store, &id);
    if status == StatusCode::NOT_FOUND {
        let (status, body) = do_v1_recap_get_job(&state.job_manager, &id).await;
        return (status, Json(body));
    }
    (status, Json(body))
}

async fn v1_recap_list(
    State(state): State<ServeState>,
    Query(q): Query<RecapListQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let kind = q.kind.as_deref().unwrap_or("session");
    let (status, body) = match kind {
        "session" => {
            let store = match open_default_or_500() {
                Ok(s) => s,
                Err(e) => return e,
            };
            do_v1_recap_list(&store, &q)
        }
        "job" => do_v1_recap_list_job(&state.job_manager, &q).await,
        other => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!(
                    "kind {other:?} not supported; valid: \"session\" | \"job\""
                )
            }),
        ),
    };
    (status, Json(body))
}

async fn v1_recap_patch(
    Path(id): Path<String>,
    Json(patch): Json<RecapPatch>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = match open_default_or_500() {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (status, body) = do_v1_recap_patch(&store, &id, &patch);
    (status, Json(body))
}

async fn v1_recap_delete(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Both stores have idempotent deletes and disjoint id spaces (UUIDs).
    // Issue the delete to both; succeed if either store reports success.
    let store = match open_default_or_500() {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (s1, _) = do_v1_recap_delete(&store, &id);
    let (s2, _) = do_v1_recap_delete_job(&state.job_manager, &id).await;
    let final_status = if s1 == StatusCode::NO_CONTENT || s2 == StatusCode::NO_CONTENT {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (final_status, Json(serde_json::json!({})))
}

// ── Recap & Resume — F1.3 /v1/resume routes ────────────────────────────────

fn helper_outcome_to_response(
    out: crate::resume::HelperOutcome,
) -> (StatusCode, Json<serde_json::Value>) {
    let status = StatusCode::from_u16(out.status)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(out.body))
}

async fn v1_resume_post(
    State(state): State<ServeState>,
    Json(req): Json<crate::resume::ResumeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let registry = crate::resume::global_registry();
    // J1.3b: dispatch on `kind`. When the body says kind=job we route to
    // the JobManager-backed helper. Default + explicit "session" stay on
    // the F1.3 path. When kind is omitted but `from_recap_id` is set, we
    // probe the job recap store first — a hit means it's a job recap and
    // we route accordingly. This lets clients pass just a recap id
    // without knowing which store owns it.
    let routed_to_job = match req.kind.as_deref() {
        Some("job") => true,
        Some(_) => false,
        None => {
            if let Some(rid) = req.from_recap_id.as_deref() {
                state
                    .job_manager
                    .get_job_recap_by_id(rid)
                    .await
                    .map(|opt| opt.is_some())
                    .unwrap_or(false)
            } else {
                false
            }
        }
    };
    let out = if routed_to_job {
        crate::resume::do_v1_resume_post_job(&state.job_manager, registry, &req).await
    } else {
        let store = match open_default_or_500() {
            Ok(s) => s,
            Err(e) => return e,
        };
        crate::resume::do_v1_resume_post(&store, registry, &req)
    };
    helper_outcome_to_response(out)
}

async fn v1_resume_get(
    Path(handle): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let registry = crate::resume::global_registry();
    let out = crate::resume::do_v1_resume_get(registry, &handle);
    helper_outcome_to_response(out)
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
            // Phase 7 S3: advertise context-assembler shape so mobile /
            // watch / IDE clients can negotiate via `/agent`'s
            // `context_request` field. The kinds here mirror the
            // `AgentKind` enum (one source of truth in
            // `context_assembler::parse_agent_kind`); the sections
            // mirror `KNOWN_SECTION_NAMES`. A client that receives an
            // unknown kind/section here can fall back to defaults
            // without needing a daemon redeploy.
            "context": {
                "description": "Per-request memory negotiation: pick a budget shape and toggles",
                "kinds": ["Chat", "CodingAgent", "ResearchAgent", "BackgroundJob"],
                "sections": crate::context_assembler::KNOWN_SECTION_NAMES,
                "request_field": "context_request",
                "toggles": ["openmemory_enabled", "openmemory_auto_inject"],
            },
        },
    }))
}

// ── RL-OS: /v1/rl/runs ────────────────────────────────────────────────────────
//
// Slice 1 — see docs/design/rl-os/01-persistence.md. These handlers are thin
// wrappers around `RunStore`: validate, call, map errors to HTTP. The
// executor (slice 2) plugs in at `start_run`; until it ships, that handler
// records a "no executor" failure on the run so the panel surfaces an
// honest message instead of pretending to train.

fn rl_err_to_http(e: crate::rl_runs::RunError) -> (StatusCode, Json<serde_json::Value>) {
    use crate::rl_runs::RunError;
    let status = match &e {
        RunError::NotFound(_) => StatusCode::NOT_FOUND,
        RunError::Invalid(_) => StatusCode::BAD_REQUEST,
        RunError::IllegalTransition { .. } => StatusCode::CONFLICT,
        RunError::DeleteWhileActive(_) => StatusCode::CONFLICT,
        RunError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    json_error(status, e.to_string())
}

#[derive(Debug, Deserialize)]
struct RlRunListQuery {
    kind: Option<String>,
    status: Option<String>,
    algorithm: Option<String>,
    limit: Option<i64>,
}

async fn rl_create_run(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_runs::CreateRunRequest>,
) -> Result<Json<crate::rl_runs::Run>, (StatusCode, Json<serde_json::Value>)> {
    state
        .rl_run_store
        .create(req)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_list_runs_h(
    State(state): State<ServeState>,
    Query(q): Query<RlRunListQuery>,
) -> Result<Json<Vec<crate::rl_runs::Run>>, (StatusCode, Json<serde_json::Value>)> {
    let filter = crate::rl_runs::RunFilter {
        kind: q.kind.as_deref().and_then(crate::rl_runs::RunKind::from_str),
        status: q
            .status
            .as_deref()
            .and_then(crate::rl_runs::RunStatus::from_str),
        algorithm: q.algorithm,
        limit: q.limit,
    };
    state
        .rl_run_store
        .list(filter)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_get_run(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_runs::Run>, (StatusCode, Json<serde_json::Value>)> {
    match state.rl_run_store.get(&id).map_err(rl_err_to_http)? {
        Some(run) => Ok(Json(run)),
        None => Err(json_error(
            StatusCode::NOT_FOUND,
            format!("run '{}' not found", sanitize_user_input(&id)),
        )),
    }
}

async fn rl_start_run(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_runs::Run>, (StatusCode, Json<serde_json::Value>)> {
    // Slice 2: spawn the Python sidecar. The executor flips the row
    // through Created → Queued → Running on its `started` heartbeat; we
    // return immediately with whatever the row looks like right after
    // spawn (typically Queued; reader task takes it to Running on the
    // first sidecar JSON-Line).
    state
        .rl_executor
        .start(&id)
        .await
        .map_err(rl_err_to_http)?;
    let run = state
        .rl_run_store
        .get(&id)
        .map_err(rl_err_to_http)?
        .ok_or_else(|| {
            json_error(
                StatusCode::NOT_FOUND,
                format!("run '{}' not found", sanitize_user_input(&id)),
            )
        })?;
    Ok(Json(run))
}

async fn rl_stop_run(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_runs::Run>, (StatusCode, Json<serde_json::Value>)> {
    // Executor sends SIGTERM and transitions through Stopping → Stopped
    // when the sidecar's atexit handler emits the final `finished` line.
    state
        .rl_executor
        .stop(&id)
        .await
        .map_err(rl_err_to_http)?;
    let run = state
        .rl_run_store
        .get(&id)
        .map_err(rl_err_to_http)?
        .ok_or_else(|| {
            json_error(
                StatusCode::NOT_FOUND,
                format!("run '{}' not found", sanitize_user_input(&id)),
            )
        })?;
    Ok(Json(run))
}

async fn rl_cancel_run(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_runs::Run>, (StatusCode, Json<serde_json::Value>)> {
    state
        .rl_executor
        .cancel(&id)
        .await
        .map_err(rl_err_to_http)?;
    let run = state
        .rl_run_store
        .get(&id)
        .map_err(rl_err_to_http)?
        .ok_or_else(|| {
            json_error(
                StatusCode::NOT_FOUND,
                format!("run '{}' not found", sanitize_user_input(&id)),
            )
        })?;
    Ok(Json(run))
}

async fn rl_delete_run(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    state
        .rl_run_store
        .delete(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(rl_err_to_http)
}

#[derive(Debug, Deserialize)]
struct SinceTickQuery {
    since: Option<i64>,
    limit: Option<i64>,
}

async fn rl_get_metrics(
    Path(id): Path<String>,
    Query(q): Query<SinceTickQuery>,
    State(state): State<ServeState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let since = q.since.unwrap_or(0);
    let metrics = state
        .rl_run_store
        .list_metrics(&id, since)
        .map_err(rl_err_to_http)?;
    Ok(Json(serde_json::json!({
        "run_id": id,
        "since": since,
        "metrics": metrics,
    })))
}

async fn rl_get_episodes(
    Path(id): Path<String>,
    Query(q): Query<SinceTickQuery>,
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_runs::EpisodeRow>>, (StatusCode, Json<serde_json::Value>)> {
    let since = q.since.unwrap_or(0);
    let limit = q.limit.unwrap_or(500);
    state
        .rl_run_store
        .list_episodes(&id, since, limit)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_get_artifacts(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_runs::Artifact>>, (StatusCode, Json<serde_json::Value>)> {
    state
        .rl_run_store
        .list_artifacts(&id)
        .map(Json)
        .map_err(rl_err_to_http)
}

// ── RL-OS: /v1/rl/envs (slice 3) ─────────────────────────────────────────────
//
// See `docs/design/rl-os/03-environments.md`. Slice 3 reads + writes the
// `rl_environments` table (created by slice 1's schema), seeds a small set
// of bundled defaults so the panel is never empty, and exposes a refresh
// endpoint that asks the sidecar to enumerate Gymnasium envs.

#[derive(Debug, Deserialize)]
struct RlEnvListQuery {
    source: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
}

fn open_env_store_from_state(state: &ServeState) -> Result<crate::rl_envs::EnvStore, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_envs::EnvStore::open(&state.workspace_root).map_err(rl_err_to_http)
}

async fn rl_list_envs_h(
    State(state): State<ServeState>,
    Query(q): Query<RlEnvListQuery>,
) -> Result<Json<Vec<crate::rl_envs::Environment>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_env_store_from_state(&state)?;
    if store.count("gymnasium").unwrap_or(0) == 0 {
        // First-touch seeding: drop a small set of bundled defaults so the
        // panel renders something even before `vibe-rl-py` is installed.
        let _ = store.seed_defaults();
    }
    store
        .list(crate::rl_envs::EnvFilter {
            source: q.source,
            search: q.search,
            limit: q.limit,
        })
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_get_env_h(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_envs::Environment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_env_store_from_state(&state)?;
    match store.get(&id).map_err(rl_err_to_http)? {
        Some(env) => Ok(Json(env)),
        None => Err(json_error(
            StatusCode::NOT_FOUND,
            format!("environment '{}' not found", sanitize_user_input(&id)),
        )),
    }
}

async fn rl_refresh_envs_h(
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_envs::RefreshReport>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_env_store_from_state(&state)?;
    let cfg = crate::rl_executor::ExecutorConfig::from_env();
    store
        .refresh_from_sidecar(&cfg)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_register_custom_env_h(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_envs::CustomEnvRequest>,
) -> Result<Json<crate::rl_envs::Environment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_env_store_from_state(&state)?;
    let cfg = crate::rl_executor::ExecutorConfig::from_env();
    store
        .register_custom(req, &cfg)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_delete_env_h(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = open_env_store_from_state(&state)?;
    store
        .delete(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(rl_err_to_http)
}

// ── RL-OS: /v1/rl/eval (slice 4) ──────────────────────────────────────────────

fn open_eval_store_from_state(state: &ServeState) -> Result<crate::rl_eval::EvalStore, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_eval::EvalStore::open(&state.workspace_root).map_err(rl_err_to_http)
}

async fn rl_eval_create_suite(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_eval::CreateSuiteRequest>,
) -> Result<Json<crate::rl_eval::EvalSuite>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    store.create_suite(req).map(Json).map_err(rl_err_to_http)
}

async fn rl_eval_list_suites(
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_eval::EvalSuite>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    store.list_suites().map(Json).map_err(rl_err_to_http)
}

async fn rl_eval_get_suite(
    Path(suite_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_eval::EvalSuite>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    match store.get_suite(&suite_id).map_err(rl_err_to_http)? {
        Some(s) => Ok(Json(s)),
        None => Err(json_error(
            StatusCode::NOT_FOUND,
            format!("eval suite '{}' not found", sanitize_user_input(&suite_id)),
        )),
    }
}

async fn rl_eval_delete_suite(
    Path(suite_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    store
        .delete_suite(&suite_id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(rl_err_to_http)
}

#[derive(Debug, Deserialize)]
struct EvalResultsQuery {
    run_id: Option<String>,
    suite_id: Option<String>,
}

async fn rl_eval_list_results(
    State(state): State<ServeState>,
    Query(q): Query<EvalResultsQuery>,
) -> Result<Json<Vec<crate::rl_eval::EvalResult>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    let rows = match (q.run_id, q.suite_id) {
        (Some(r), _) => store.list_results_for_run(&r),
        (None, Some(s)) => store.list_results_for_suite(&s),
        (None, None) => Err(crate::rl_runs::RunError::Invalid(
            "either run_id or suite_id must be provided".into(),
        )),
    }
    .map_err(rl_err_to_http)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct CompareRequest {
    run_a: String,
    run_b: String,
    #[serde(default)]
    suite_id: Option<String>,
}

async fn rl_eval_compare(
    State(state): State<ServeState>,
    Json(req): Json<CompareRequest>,
) -> Result<Json<crate::rl_eval::ComparisonReport>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_eval_store_from_state(&state)?;
    store
        .compare(&req.run_a, &req.run_b, req.suite_id.as_deref())
        .map(Json)
        .map_err(rl_err_to_http)
}

// ── RL-OS: /v1/rl/policies (slice 5) ──────────────────────────────────────────

fn open_policy_store_from_state(
    state: &ServeState,
) -> Result<crate::rl_policies::PolicyStore, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_policies::PolicyStore::open(&state.workspace_root).map_err(rl_err_to_http)
}

async fn rl_register_policy(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_policies::RegisterRequest>,
) -> Result<Json<crate::rl_policies::Policy>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_policy_store_from_state(&state)?;
    store
        .register(req, state.rl_run_store.as_ref())
        .map(Json)
        .map_err(rl_err_to_http)
}

#[derive(Debug, Deserialize)]
struct PolicyListQuery {
    name: Option<String>,
}

async fn rl_list_policies_h(
    State(state): State<ServeState>,
    Query(q): Query<PolicyListQuery>,
) -> Result<Json<Vec<crate::rl_policies::Policy>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_policy_store_from_state(&state)?;
    store
        .list(q.name.as_deref())
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_get_policy(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_policies::Policy>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_policy_store_from_state(&state)?;
    match store.get(&id).map_err(rl_err_to_http)? {
        Some(p) => Ok(Json(p)),
        None => Err(json_error(
            StatusCode::NOT_FOUND,
            format!("policy '{}' not found", sanitize_user_input(&id)),
        )),
    }
}

async fn rl_delete_policy(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let store = open_policy_store_from_state(&state)?;
    store
        .delete(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(rl_err_to_http)
}

#[derive(Debug, Deserialize)]
struct LineageQuery {
    depth: Option<usize>,
}

async fn rl_get_policy_lineage(
    Path(id): Path<String>,
    Query(q): Query<LineageQuery>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_policies::LineageGraph>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_policy_store_from_state(&state)?;
    let depth = q.depth.unwrap_or(3);
    store.lineage(&id, depth).map(Json).map_err(rl_err_to_http)
}

async fn rl_get_policy_card(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    use axum::response::IntoResponse;
    let store = open_policy_store_from_state(&state)?;
    let card = store.card(&id).map_err(rl_err_to_http)?;
    Ok((
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        card,
    )
        .into_response())
}

async fn rl_get_reward_components(
    Path(run_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_policies::RewardComponentRow>>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state
        .rl_run_store
        .reward_decomposition(&run_id)
        .map_err(rl_err_to_http)?;
    let report: Vec<crate::rl_policies::RewardComponentRow> = rows
        .into_iter()
        .map(|(component, mean, total, n)| crate::rl_policies::RewardComponentRow {
            component,
            mean,
            total,
            n_episodes: n,
        })
        .collect();
    Ok(Json(report))
}

// ── RL-OS: /v1/rl/serve (slice 6 — deployment management) ────────────────────

fn open_deploy_store_from_state(
    state: &ServeState,
) -> Result<crate::rl_deploy::DeploymentStore, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_deploy::DeploymentStore::open(&state.workspace_root).map_err(rl_err_to_http)
}

async fn rl_create_deployment(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_deploy::CreateDeploymentRequest>,
) -> Result<Json<crate::rl_deploy::Deployment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.create(req).map(Json).map_err(rl_err_to_http)
}

async fn rl_list_deployments_h(
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_deploy::Deployment>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.list().map(Json).map_err(rl_err_to_http)
}

async fn rl_get_deployment(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_deploy::Deployment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    match store.get(&id).map_err(rl_err_to_http)? {
        Some(d) => Ok(Json(d)),
        None => Err(json_error(
            StatusCode::NOT_FOUND,
            format!("deployment '{}' not found", sanitize_user_input(&id)),
        )),
    }
}

async fn rl_promote_deployment(
    Path(id): Path<String>,
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_deploy::PromoteRequest>,
) -> Result<Json<crate::rl_deploy::Deployment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.promote(&id, req).map(Json).map_err(rl_err_to_http)
}

async fn rl_rollback_deployment(
    Path(id): Path<String>,
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_deploy::RollbackRequest>,
) -> Result<Json<crate::rl_deploy::Deployment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.rollback(&id, req).map(Json).map_err(rl_err_to_http)
}

async fn rl_stop_deployment(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_deploy::Deployment>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.stop(&id).map(Json).map_err(rl_err_to_http)
}

async fn rl_get_deployment_health_h(
    Path(id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<crate::rl_deploy::HealthSnapshot>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    store.health(&id).map(Json).map_err(rl_err_to_http)
}

/// Slice 6.5 — real inference path. Looks up the deployment by name
/// (or by id; we accept both for the URL slug to keep the /act path
/// stable across renames), resolves its primary artifact, lazy-spawns
/// the inference sidecar in the runtime pool, and round-trips the obs.
///
/// Only deployments in `staging` / `canary` / `production` are
/// servable; rolled-back / stopped slugs return 409.
async fn rl_serve_act(
    Path(name): Path<String>,
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_runtime::ActRequest>,
) -> Result<Json<crate::rl_runtime::ActResponse>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_deploy_store_from_state(&state)?;
    // Lookup: name OR id, since the panel may pass either.
    let deployment = {
        let by_id = store.get(&name).map_err(rl_err_to_http)?;
        match by_id {
            Some(d) => d,
            None => store
                .list()
                .map_err(rl_err_to_http)?
                .into_iter()
                .find(|d| d.name == name)
                .ok_or_else(|| {
                    json_error(
                        StatusCode::NOT_FOUND,
                        format!("deployment '{}' not found", sanitize_user_input(&name)),
                    )
                })?,
        }
    };

    use crate::rl_deploy::DeploymentStatus;
    if matches!(
        deployment.status,
        DeploymentStatus::RolledBack | DeploymentStatus::Stopped
    ) {
        return Err(json_error(
            StatusCode::CONFLICT,
            format!(
                "deployment '{}' is in status '{}' and cannot serve",
                deployment.name,
                deployment.status.as_str()
            ),
        ));
    }
    if !matches!(deployment.runtime.as_str(), "python" | "onnx") {
        return Err(json_error(
            StatusCode::NOT_IMPLEMENTED,
            format!(
                "runtime '{}' is not yet wired (supports: python, onnx). \
                 native_candle ships in slice 7d.",
                deployment.runtime
            ),
        ));
    }

    // Resolve the primary artifact's workspace-relative path.
    let artifact = state
        .rl_run_store
        .find_artifact_by_id(&deployment.artifact_id)
        .map_err(rl_err_to_http)?
        .ok_or_else(|| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "deployment '{}' references artifact '{}' which is missing",
                    deployment.name, deployment.artifact_id
                ),
            )
        })?;

    let cfg = crate::rl_executor::ExecutorConfig::from_env();
    let runtime = state
        .rl_runtime_pool
        .get_or_spawn(
            &cfg,
            &deployment.deployment_id,
            &artifact.rel_path,
            &state.workspace_root,
            &deployment.runtime,
        )
        .await
        .map_err(rl_err_to_http)?;

    let started = std::time::Instant::now();
    let action = runtime.act(req.obs).await.map_err(rl_err_to_http)?;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(crate::rl_runtime::ActResponse {
        action,
        deployment: deployment.name,
        policy_id: None,
        latency_ms,
    }))
}

// ── RL-OS: /v1/rl/rlhf, /v1/rl/optimization, /v1/rl/multi-agent (slice 7) ────

fn open_pref_store_from_state(
    state: &ServeState,
) -> Result<crate::rl_advanced::PreferenceStore, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_advanced::PreferenceStore::open(&state.workspace_root).map_err(rl_err_to_http)
}

async fn rl_create_preference(
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_advanced::CreatePreferenceRequest>,
) -> Result<Json<crate::rl_advanced::Preference>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_pref_store_from_state(&state)?;
    store.create(req).map(Json).map_err(rl_err_to_http)
}

async fn rl_judge_preference(
    Path(pref_id): Path<String>,
    State(state): State<ServeState>,
    Json(req): Json<crate::rl_advanced::JudgePreferenceRequest>,
) -> Result<Json<crate::rl_advanced::Preference>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_pref_store_from_state(&state)?;
    store
        .judge(&pref_id, req)
        .map(Json)
        .map_err(rl_err_to_http)
}

#[derive(Debug, Deserialize)]
struct PreferenceListQuery {
    suite_id: Option<String>,
}

async fn rl_list_preferences(
    State(state): State<ServeState>,
    Query(q): Query<PreferenceListQuery>,
) -> Result<Json<Vec<crate::rl_advanced::Preference>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_pref_store_from_state(&state)?;
    store
        .list(q.suite_id.as_deref())
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_alignment_metrics(
    Path(run_id): Path<String>,
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_advanced::AlignmentScoreRow>>, (StatusCode, Json<serde_json::Value>)> {
    let store = open_pref_store_from_state(&state)?;
    store
        .alignment_scores(&run_id)
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_optimization_runs_h(
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_advanced::OptimizationRunSummary>>, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_advanced::list_optimization_runs(state.rl_run_store.as_ref())
        .map(Json)
        .map_err(rl_err_to_http)
}

async fn rl_multi_agent_runs_h(
    State(state): State<ServeState>,
) -> Result<Json<Vec<crate::rl_advanced::MultiAgentRunSummary>>, (StatusCode, Json<serde_json::Value>)> {
    crate::rl_advanced::list_multi_agent_runs(state.rl_run_store.as_ref())
        .map(Json)
        .map_err(rl_err_to_http)
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
        // Vite dev server origins (VibeUI :1420, VibeCLI app :1421)
        "http://localhost:1420".to_string(),
        "http://localhost:1421".to_string(),
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
        .route("/memory/import", post(memory_import))
        .route("/memory/pin", post(memory_pin))
        .route("/memory/unpin", post(memory_unpin))
        .route("/memory/delete", post(memory_delete))
        // MemPalace verbatim drawer + benchmark endpoints
        .route("/memory/chunk", post(memory_chunk))
        .route("/memory/drawers/stats", get(memory_drawers_stats))
        .route("/memory/tunnel", post(memory_tunnel))
        .route("/memory/auto-tunnel", post(memory_auto_tunnel))
        .route("/memory/context", post(memory_context))
        .route("/memory/benchmark", get(memory_benchmark))
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
        .route("/v1/metrics/jobs", get(v1_jobs_metrics))
        .route("/v1/browse", post(v1_create_browse))
        .route("/v1/browse/:id", get(v1_get_browse))
        .route("/v1/browse/:id/screenshots", get(v1_browse_screenshots))
        .route("/v1/browse/:id/intervene", post(v1_browse_intervene))
        // RL-OS v1 — slice 1 (persistence + run lifecycle)
        // See docs/design/rl-os/01-persistence.md
        .route("/v1/rl/runs", post(rl_create_run))
        .route("/v1/rl/runs", get(rl_list_runs_h))
        .route("/v1/rl/runs/:id", get(rl_get_run))
        .route("/v1/rl/runs/:id", axum::routing::delete(rl_delete_run))
        .route("/v1/rl/runs/:id/start", post(rl_start_run))
        .route("/v1/rl/runs/:id/stop", post(rl_stop_run))
        .route("/v1/rl/runs/:id/cancel", post(rl_cancel_run))
        .route("/v1/rl/runs/:id/metrics", get(rl_get_metrics))
        .route("/v1/rl/runs/:id/episodes", get(rl_get_episodes))
        .route("/v1/rl/runs/:id/artifacts", get(rl_get_artifacts))
        // RL-OS slice 3 — environment registry
        .route("/v1/rl/envs", get(rl_list_envs_h))
        .route("/v1/rl/envs/:id", get(rl_get_env_h))
        .route("/v1/rl/envs/:id", axum::routing::delete(rl_delete_env_h))
        .route("/v1/rl/envs/refresh", post(rl_refresh_envs_h))
        .route("/v1/rl/envs/custom", post(rl_register_custom_env_h))
        // RL-OS slice 4 — eval suites + results + compare
        .route("/v1/rl/eval/suites", post(rl_eval_create_suite))
        .route("/v1/rl/eval/suites", get(rl_eval_list_suites))
        .route("/v1/rl/eval/suites/:id", get(rl_eval_get_suite))
        .route("/v1/rl/eval/suites/:id", axum::routing::delete(rl_eval_delete_suite))
        .route("/v1/rl/eval/results", get(rl_eval_list_results))
        .route("/v1/rl/eval/compare", post(rl_eval_compare))
        // RL-OS slice 5 — policy registry + lineage + reward decomposition
        .route("/v1/rl/policies", post(rl_register_policy))
        .route("/v1/rl/policies", get(rl_list_policies_h))
        .route("/v1/rl/policies/:id", get(rl_get_policy))
        .route("/v1/rl/policies/:id", axum::routing::delete(rl_delete_policy))
        .route("/v1/rl/policies/:id/lineage", get(rl_get_policy_lineage))
        .route("/v1/rl/policies/:id/card", get(rl_get_policy_card))
        .route(
            "/v1/rl/runs/:id/reward-components",
            get(rl_get_reward_components),
        )
        // RL-OS slice 6 — deployment management (inference wired in 6.5)
        .route("/v1/rl/serve/deployments", post(rl_create_deployment))
        .route("/v1/rl/serve/deployments", get(rl_list_deployments_h))
        .route("/v1/rl/serve/deployments/:id", get(rl_get_deployment))
        .route("/v1/rl/serve/deployments/:id/promote", post(rl_promote_deployment))
        .route("/v1/rl/serve/deployments/:id/rollback", post(rl_rollback_deployment))
        .route("/v1/rl/serve/deployments/:id/stop", post(rl_stop_deployment))
        .route("/v1/rl/serve/deployments/:id/health", get(rl_get_deployment_health_h))
        .route("/v1/rl/serve/:name/act", post(rl_serve_act))
        // RL-OS slice 7 — RLHF + Optimization + Multi-Agent
        .route("/v1/rl/rlhf/preferences", post(rl_create_preference))
        .route("/v1/rl/rlhf/preferences", get(rl_list_preferences))
        .route("/v1/rl/rlhf/preferences/:id/judge", post(rl_judge_preference))
        .route("/v1/rl/rlhf/runs/:id/alignment", get(rl_alignment_metrics))
        .route("/v1/rl/optimization/runs", get(rl_optimization_runs_h))
        .route("/v1/rl/multi-agent/runs", get(rl_multi_agent_runs_h))
        // Recap & Resume v1 — F1.2 (Session-only, heuristic-only)
        .route("/v1/recap", post(v1_recap_post))
        .route("/v1/recap", get(v1_recap_list))
        .route("/v1/recap/:id", get(v1_recap_get))
        .route("/v1/recap/:id", axum::routing::patch(v1_recap_patch))
        .route("/v1/recap/:id", axum::routing::delete(v1_recap_delete))
        // Recap & Resume v1 — F1.3 (resume handles)
        .route("/v1/resume", post(v1_resume_post))
        .route("/v1/resume/:handle", get(v1_resume_get))
        // Mobile Gateway — machine registration & dispatch (iOS/Android remote management)
        .route("/mobile/machines", get(mobile_list_machines))
        .route("/mobile/machines", post(mobile_register_machine))
        .route("/mobile/machines/:id", get(mobile_get_machine))
        .route("/mobile/machines/:id", axum::routing::delete(mobile_unregister_machine))
        .route("/mobile/machines/:id/heartbeat", post(mobile_heartbeat))
        .route("/mobile/pairing", post(mobile_create_pairing))
        .route("/mobile/pairing/:id/accept", post(mobile_accept_pairing))
        .route("/mobile/pairing/:id/verify", post(mobile_verify_pin))
        .route("/mobile/pairing/:id/reject", post(mobile_reject_pairing))
        .route("/mobile/devices", get(mobile_list_devices))
        .route("/mobile/devices/:id/push-token", post(mobile_update_push_token))
        .route("/mobile/devices/:device_id/machines/:machine_id/unpair", post(mobile_unpair))
        .route("/mobile/dispatch", post(mobile_dispatch))
        .route("/mobile/dispatch/:id", get(mobile_get_dispatch))
        .route("/mobile/dispatch/:id/cancel", post(mobile_cancel_dispatch))
        .route("/mobile/dispatch/:id/update", post(mobile_update_dispatch))
        .route("/mobile/dispatches/machine/:id", get(mobile_machine_dispatches))
        .route("/mobile/dispatches/device/:id", get(mobile_device_dispatches))
        .route("/mobile/notifications/:device_id", get(mobile_notifications))
        .route("/mobile/stats", get(mobile_stats))
        .route("/mobile/sessions", get(mobile_sessions))
        .route("/mobile/sessions/:id/context", get(mobile_session_context))
        .route_layer(middleware::from_fn_with_state(limiter, rate_limit))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // More restrictive rate limiter for public routes: 10 requests per 60 seconds
    let public_limiter = Arc::new(RateLimiter::new(10, Duration::from_secs(60)));

    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/models", get(list_models))
        .route("/web", get(web_client_page))
        .route("/favicon.svg", get(web_favicon))
        .route("/webhook/github", post(github_webhook))
        .route("/pair", get(pairing_handler))
        .route("/acp/v1/capabilities", get(acp_capabilities))
        .route("/v1/capabilities", get(v1_capabilities))
        .route("/ws/collab/:room_id", get(ws_collab_handler))
        .route("/mobile/beacon", get(mobile_beacon))
        .route_layer(middleware::from_fn_with_state(public_limiter, rate_limit));

    // Watch routes (/watch/*) — separate state, no bearer auth required on challenge/register
    let watch_session_db = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".vibecli")
        .join("sessions.db");
    // machine_id must be stable across restarts so Watch JWTs (which embed it)
    // remain valid after daemon restarts. Load from ProfileStore, generate once.
    let watch_machine_id = std::env::var("VIBECLI_MACHINE_ID").unwrap_or_else(|_| {
        crate::profile_store::ProfileStore::new()
            .ok()
            .and_then(|s| s.get_api_key("default", "watch.machine_id").ok().flatten())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                let id = format!("{:016x}", rand::rng().random::<u64>());
                if let Ok(s) = crate::profile_store::ProfileStore::new() {
                    let _ = s.set_api_key("default", "watch.machine_id", &id);
                }
                id
            })
    });
    let watch_router = crate::watch_bridge::WatchBridgeState::new(
        state.api_token.clone(),
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        watch_machine_id,
        None,
        Some(watch_session_db),
        Some(state.provider.clone()),
        state.provider_name.clone(),
    )
    .map(|s| crate::watch_bridge::build_watch_router(s).with_state(()))
    .unwrap_or_else(|e| {
        eprintln!("[watch] Failed to init WatchBridgeState: {e}");
        axum::Router::new()
    });

    // Ollama-compatible inference routes (/api/chat, /api/generate, /api/tags,
    // /api/pull, /api/show). Mounted under bearer auth + rate limiting,
    // since the daemon's `/api/*` surface is not the same security boundary
    // as ollama's plain-HTTP localhost socket. See `inference_routes.rs`
    // for the full handler set.
    let inference_limiter = Arc::new(RateLimiter::new(60, Duration::from_secs(60)));
    let inference_routes = crate::inference_routes::build_routes()
        .route_layer(middleware::from_fn_with_state(inference_limiter, rate_limit))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Public routes (health check, GitHub webhook with HMAC, pairing, collab WS, ACP discovery)
    public_routes
        .merge(authed_routes)
        .merge(inference_routes)
        .nest("/watch", watch_router)
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
/// Hydrate `HF_TOKEN` from the encrypted ProfileStore so downstream libs
/// (`hf-hub`, `candle`, mistralrs) pick it up without the user having to
/// `export HF_TOKEN=...`. ProfileStore is the canonical store per
/// AGENTS.md → Zero-Config First; env-var fallback is kept for
/// compatibility with toolchains the user may already have configured.
///
/// Order of precedence — first match wins:
///   1. Existing `HF_TOKEN` in env (developer override or system config).
///   2. Value at `huggingface` in ProfileStore (`vibecli set-key huggingface ...`).
///   3. Unset — daemon logs a startup advisory and the mistralrs backend
///      auto-falls-back to an ungated model.
fn hydrate_hf_token_from_profile_store() {
    // Already set in env? Honour it — covers developer overrides and
    // shells that source HF's own credential helpers.
    if std::env::var("HF_TOKEN").map(|s| !s.is_empty()).unwrap_or(false) {
        return;
    }
    let Ok(store) = crate::profile_store::ProfileStore::new() else {
        // Store unavailable (first run, permission issue, etc.). The
        // mistralrs fallback path will handle it gracefully.
        return;
    };
    let Ok(Some(token)) = store.get_api_key("default", "huggingface") else {
        return;
    };
    if token.is_empty() {
        return;
    }
    std::env::set_var("HF_TOKEN", token);
    tracing::info!(
        "vibecli serve: HF_TOKEN hydrated from ProfileStore (vibecli set-key huggingface)"
    );
}

pub async fn serve(
    provider: Arc<dyn AIProvider>,
    provider_name: String,
    approval: ApprovalPolicy,
    workspace_root: PathBuf,
    port: u16,
    host: String,
) -> Result<()> {
    // Pull HF_TOKEN out of the encrypted store before anything that might
    // need it spins up. Zero-config goal: a user who ran
    // `vibecli set-key huggingface hf_...` shouldn't have to also export
    // the env var. AGENTS.md → Zero-Config First.
    hydrate_hf_token_from_profile_store();

    // Run the TurboQuant codec self-check once. Surfaced via /health so
    // operators can detect kernel regressions without needing to load a
    // model. Probe takes microseconds; failure here is informational
    // (we don't refuse to start) — the codec only matters when the user
    // actively opts into TurboQuant via VIBE_INFER_KV_CACHE.
    let probe = vibe_infer::kv_cache_tq::run_codec_probe();
    if !probe.passed {
        tracing::warn!(
            "vibecli serve: TurboQuant codec probe FAILED \
             (mean_cosine={:.4}, hash=0x{:08x}, {}µs) — codec output diverges from reference",
            probe.mean_cosine, probe.output_hash, probe.elapsed_us,
        );
        eprintln!(
            "[vibecli serve] kv-cache codec probe FAILED — see /health.kv_cache_codec_probe"
        );
    } else {
        tracing::info!(
            "vibecli serve: TurboQuant codec probe ok \
             (mean_cosine={:.4}, hash=0x{:08x}, {}µs)",
            probe.mean_cosine, probe.output_hash, probe.elapsed_us,
        );
    }
    let _ = CODEC_PROBE.set(probe);

    // Legacy jobs directory — kept for feedback/intervene side-car files and
    // one-shot migration. Job records themselves now live in
    // ~/.vibecli/jobs.db via `JobManager`.
    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("jobs");
    std::fs::create_dir_all(&jobs_dir)?;

    // Durable job queue (SQLite) + in-memory SSE fan-out. On boot we run a
    // one-shot migration of any pre-M1 JSON records and sweep any rows left
    // in queued/running from a prior process into the failed state.
    let job_manager = Arc::new(
        JobManager::new(&crate::job_manager::default_db_path())
            .map_err(|e| anyhow::anyhow!("jobs db: {e}"))?,
    );
    match job_manager.migrate_json_jobs(&jobs_dir).await {
        Ok(rep) if rep.imported > 0 => {
            eprintln!(
                "[jobs] migrated {} JSON record(s) into jobs.db (backup: {:?})",
                rep.imported, rep.backed_up_dir
            );
        }
        Ok(_) => {}
        Err(e) => eprintln!("[jobs] migration warning: {e}"),
    }
    match job_manager.recover_interrupted().await {
        Ok(n) if n > 0 => {
            eprintln!("[jobs] recovered {n} interrupted job(s) → failed");
        }
        Ok(_) => {}
        Err(e) => eprintln!("[jobs] recovery warning: {e}"),
    }

    // Generate a random bearer token for this daemon session
    let api_token = format!("{:032x}", rand::rng().random::<u128>());

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

    let public_url_cache: Arc<std::sync::Mutex<Option<String>>> =
        Arc::new(std::sync::Mutex::new(None));

    // RL-OS slice 1 — open the per-workspace run store. Same SQLite file as
    // WorkspaceStore (`<workspace>/.vibecli/workspace.db`), separate connection.
    // See docs/design/rl-os/01-persistence.md.
    let rl_run_store = Arc::new(
        crate::rl_runs::RunStore::open(&workspace_root)
            .map_err(|e| anyhow::anyhow!("rl run store: {e}"))?,
    );
    // RL-OS slice 2 — Python-sidecar training executor.
    // See docs/design/rl-os/02-training-executor.md.
    let rl_python_executor = Arc::new(crate::rl_executor::PythonExecutor::new(
        crate::rl_executor::ExecutorConfig::from_env(),
        rl_run_store.clone(),
    ));
    // Sweep stale Running / Stopping rows from a prior daemon process so the
    // dashboard never claims a long-dead run is still active.
    if let Err(e) = rl_python_executor.recover_stale_runs().await {
        eprintln!("[rl] recover_stale_runs: {e}");
    }
    // Unsizing coercion: `Arc<PythonExecutor>` → `Arc<dyn TrainingExecutor>`
    // happens at the assignment to the type-annotated binding.
    let rl_executor: Arc<dyn crate::rl_executor::TrainingExecutor> = rl_python_executor;

    let state = ServeState {
        provider,
        approval,
        workspace_root,
        job_manager,
        jobs_dir,
        provider_name,
        api_token: api_token.clone(),
        collab_server,
        github_app_config: gh_app_config,
        started_at: std::time::Instant::now(),
        public_url_cache: Arc::clone(&public_url_cache),
        inference_router: Some(Arc::new(crate::inference::Router::from_env())),
        rl_run_store,
        rl_executor,
        rl_runtime_pool: Arc::new(crate::rl_runtime::RuntimePool::new()),
    };

    // Background task: detect or start an ngrok / Tailscale Funnel tunnel and
    // cache the resulting public URL so the beacon can serve it instantly.
    {
        let cache = Arc::clone(&public_url_cache);
        let port_copy = port;
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("config.toml");
        tokio::spawn(async move {
            // 1. Check for an already-running ngrok agent (fast, no side-effects).
            let detected = tokio::task::spawn_blocking(move || {
                crate::ngrok::detect_tunnel(port_copy)
            })
            .await
            .unwrap_or(None);

            if let Some(url) = detected {
                eprintln!("[tunnel] ngrok tunnel detected: {url}");
                if let Ok(mut guard) = cache.lock() {
                    *guard = Some(url);
                }
                return;
            }

            // 2. Load [tunnel] config.
            let tunnel_cfg = std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str::<crate::config::Config>(&s).ok())
                .map(|c| c.tunnel)
                .unwrap_or_default();

            // 3. Tailscale Funnel (opt-in).
            if tunnel_cfg.tailscale_funnel {
                match crate::tailscale::serve_via_funnel(port_copy).await {
                    Ok(_child) => {
                        // Poll until tailscale reports a DNS name with an active funnel
                        // (up to 20 s).  `tailscale status --json` exposes:
                        //   Self.DNSName  → "<machine>.<tailnet>.ts.net."
                        //   Self.FunnelPorts → [443, ...]  when funnel is active
                        // Tailscale Funnel always uses HTTPS on port 443; the daemon
                        // port is accessible via the funnel URL path (no port in URL).
                        for _ in 0..10u32 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            let funnel_url =
                                crate::tailscale::tailscale_funnel_url(port_copy);
                            if let Some(url) = funnel_url {
                                eprintln!("[tunnel] Tailscale Funnel active: {url}");
                                if let Ok(mut guard) = cache.lock() {
                                    *guard = Some(url);
                                }
                                return;
                            }
                        }
                        eprintln!("[tunnel] Tailscale Funnel started but URL not yet available");
                    }
                    Err(e) => eprintln!("[tunnel] tailscale funnel failed to start: {e}"),
                }
            }

            // 4. ngrok auto-start (opt-in).
            if tunnel_cfg.ngrok_auto_start {
                // Prefer env var; fall back to config file value.
                let token_str = std::env::var("NGROK_AUTHTOKEN")
                    .ok()
                    .or_else(|| tunnel_cfg.ngrok_auth_token.clone());
                match crate::ngrok::start_tunnel(port_copy, token_str.as_deref()).await {
                    Ok(url) => {
                        eprintln!("[tunnel] ngrok tunnel started: {url}");
                        if let Ok(mut guard) = cache.lock() {
                            *guard = Some(url);
                        }
                    }
                    Err(e) => eprintln!("[tunnel] ngrok start failed: {e}"),
                }
            }
        });
    }

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

    let addr = format!("{host}:{port}");
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

    // mistralrs gating advisory — meta-llama/* repos require an HF token
    // and license acceptance. By this point hydrate_hf_token_from_profile_store
    // has already run, so a missing env value here means "not in env AND not
    // in ProfileStore." Surface the canonical fix (encrypted store) first, env
    // export second; gated-load failures are auto-substituted at request time
    // either way.
    if std::env::var("HF_TOKEN").map(|s| s.is_empty()).unwrap_or(true) {
        eprintln!(
            "[vibecli serve] HF_TOKEN not configured — gated mistralrs models (meta-llama/*) cannot be pulled."
        );
        eprintln!(
            "[vibecli serve]   Falling back to {} for the picker default.",
            crate::inference::mistralrs::UNGATED_FALLBACK_MODEL
        );
        eprintln!(
            "[vibecli serve]   To enable Llama-3.x:"
        );
        eprintln!(
            "[vibecli serve]     1. Accept the license at https://huggingface.co/meta-llama"
        );
        eprintln!(
            "[vibecli serve]     2. Run: vibecli set-key huggingface hf_...   (preferred — encrypted store)"
        );
        eprintln!(
            "[vibecli serve]        OR: export HF_TOKEN=hf_...                  (env-var fallback)"
        );
    }

    // Start zero-config mDNS announcer — announces _vibecli._tcp.local. so
    // the mobile app discovers this daemon on any LAN without special flags.
    {
        let machine_id = std::env::var("VIBECLI_MACHINE_ID")
            .unwrap_or_else(|_| format!("{:016x}", rand::rng().random::<u64>()));
        crate::mdns_announce::start(port, machine_id);
        eprintln!("[vibecli serve] mDNS announcing _vibecli._tcp.local. on port {port}");
    }

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
        .unwrap_or_else(|| format!("{:016x}", rand::rng().random::<u64>()));
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
    let peer_id = format!("{:016x}", rand::rng().random::<u64>());
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
        "total_drawers": store.drawer_store().len(),
        "encryption": false,
        "sectors": sectors,
        "embedding_dim": store.embedding_dim(),
        "embedding_compression_ratio": store.embedding_compression_ratio(),
        "embedding_backend": "turboquant",
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

#[derive(Deserialize)]
struct MemoryIdRequest {
    id: String,
}

async fn memory_pin(
    _state: State<ServeState>,
    Json(req): Json<MemoryIdRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    if store.pin(&req.id) {
        let _ = store.save();
        (StatusCode::OK, Json(serde_json::json!({ "pinned": true, "id": req.id })))
    } else {
        json_error(StatusCode::NOT_FOUND, format!("memory '{}' not found or already pinned", req.id))
    }
}

async fn memory_unpin(
    _state: State<ServeState>,
    Json(req): Json<MemoryIdRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    if store.unpin(&req.id) {
        let _ = store.save();
        (StatusCode::OK, Json(serde_json::json!({ "pinned": false, "id": req.id })))
    } else {
        json_error(StatusCode::NOT_FOUND, format!("memory '{}' not found or not pinned", req.id))
    }
}

async fn memory_delete(
    _state: State<ServeState>,
    Json(req): Json<MemoryIdRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    if store.delete(&req.id) {
        let _ = store.save();
        (StatusCode::OK, Json(serde_json::json!({ "deleted": true, "id": req.id })))
    } else {
        json_error(StatusCode::NOT_FOUND, format!("memory '{}' not found", req.id))
    }
}

#[derive(Deserialize)]
struct MemoryImportRequest {
    /// JSON content to import.
    content: String,
    /// Format hint: "mem0", "zep", "openmemory", or "auto" (default).
    #[serde(default = "default_import_format")]
    format: String,
}

fn default_import_format() -> String { "auto".to_string() }

async fn memory_import(
    _state: State<ServeState>,
    Json(req): Json<MemoryImportRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    let result = match req.format.as_str() {
        "mem0" => crate::open_memory::import_from_mem0(&mut store, &req.content),
        "zep"  => crate::open_memory::import_from_zep(&mut store, &req.content),
        "openmemory" => store.import_openmemory_json(&req.content),
        _ => {
            // auto-detect: try each format in order
            crate::open_memory::import_from_auto_memory(&mut store, &req.content)
        }
    };
    match result {
        Ok(imported) => {
            let _ = store.save();
            (StatusCode::OK, Json(serde_json::json!({
                "imported": imported,
                "total_memories": store.total_memories(),
                "format_used": req.format,
            })))
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, format!("import failed: {e}")),
    }
}

// ── MemPalace verbatim drawer endpoints ───────────────────────────────────────

#[derive(Deserialize)]
struct MemoryChunkRequest {
    /// Raw text to ingest as verbatim 800-char chunks.
    content: String,
    /// Optional source label (e.g. filename or session ID).
    #[serde(default)]
    source: String,
}

async fn memory_chunk(
    _state: State<ServeState>,
    Json(req): Json<MemoryChunkRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    let source = if req.source.is_empty() { "api".to_string() } else { req.source };
    let created = store.ingest_conversation_chunks(&req.content, &source);
    match store.save() {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({
            "chunks_created": created,
            "total_drawers": store.drawer_store().len(),
        }))),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("save failed: {e}")),
    }
}

async fn memory_drawers_stats(_state: State<ServeState>) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let ds = store.drawer_store();
    let total = ds.len();

    // Wing and Room distributions from the raw drawers
    let mut wings: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut rooms: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for d in ds.drawers() {
        *wings.entry(d.wing.clone()).or_default() += 1;
        *rooms.entry(d.room.clone()).or_default() += 1;
    }
    let wing_list: Vec<serde_json::Value> = wings.into_iter()
        .map(|(w, c)| serde_json::json!({"wing": w, "count": c}))
        .collect();
    let room_list: Vec<serde_json::Value> = rooms.into_iter()
        .map(|(r, c)| serde_json::json!({"room": r, "count": c}))
        .collect();

    Json(serde_json::json!({
        "total_drawers": total,
        "wings": wing_list,
        "rooms": room_list,
    }))
}

#[derive(Deserialize)]
struct MemoryTunnelRequest {
    src_id: String,
    dst_id: String,
    #[serde(default = "default_tunnel_weight")]
    weight: f64,
}

fn default_tunnel_weight() -> f64 { 0.8 }

async fn memory_tunnel(
    _state: State<ServeState>,
    Json(req): Json<MemoryTunnelRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut store = load_memory_store();
    let created = store.add_cross_project_waypoint(&req.src_id, &req.dst_id, req.weight);
    match store.save() {
        Ok(_) => {
            let status = if created { StatusCode::CREATED } else { StatusCode::OK };
            (status, Json(serde_json::json!({
                "created": created,
                "src_id": req.src_id,
                "dst_id": req.dst_id,
                "weight": req.weight,
            })))
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("save failed: {e}")),
    }
}

#[derive(Deserialize)]
struct MemoryAutoTunnelRequest {
    #[serde(default = "default_auto_tunnel_threshold")]
    threshold: f64,
}

fn default_auto_tunnel_threshold() -> f64 { 0.75 }

async fn memory_auto_tunnel(
    _state: State<ServeState>,
    Json(req): Json<MemoryAutoTunnelRequest>,
) -> Json<serde_json::Value> {
    // Auto-tunnel between the global default store and the project-scoped store
    let global = load_memory_store();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project = crate::open_memory::project_scoped_store(&cwd);
    let created = crate::open_memory::OpenMemoryStore::tunnel_across_stores(
        &mut [global, project],
        req.threshold,
    );
    Json(serde_json::json!({
        "tunnels_created": created,
        "threshold": req.threshold,
    }))
}

#[derive(Deserialize)]
struct MemoryContextRequest {
    query: String,
    #[serde(default = "default_l1_tokens")]
    l1_tokens: usize,
    #[serde(default = "default_l2_limit")]
    l2_limit: usize,
    #[serde(default = "default_l3_threshold")]
    l3_threshold: usize,
}

fn default_l1_tokens() -> usize { 700 }
fn default_l2_limit() -> usize { 8 }
fn default_l3_threshold() -> usize { 3 }

async fn memory_context(
    _state: State<ServeState>,
    Json(req): Json<MemoryContextRequest>,
) -> Json<serde_json::Value> {
    let store = load_memory_store();
    let ctx = store.get_layered_context(&req.query, req.l1_tokens, req.l2_limit, req.l3_threshold);
    Json(serde_json::json!({
        "query": req.query,
        "context": ctx,
        "total_memories": store.total_memories(),
        "total_drawers": store.drawer_store().len(),
    }))
}

#[derive(Deserialize)]
struct MemoryBenchmarkQuery {
    #[serde(default = "default_bench_k")]
    k: usize,
}

fn default_bench_k() -> usize { 5 }

async fn memory_benchmark(
    _state: State<ServeState>,
    Query(params): Query<MemoryBenchmarkQuery>,
) -> Json<serde_json::Value> {
    let cases = crate::mem_benchmark::default_benchmark_cases();
    let report = crate::mem_benchmark::run_benchmark(&cases, params.k);

    let cases_out: Vec<serde_json::Value> = report.cases.iter().map(|c| {
        serde_json::json!({
            "query": c.query,
            "expected_answer": c.expected_answer,
            "found_cognitive": c.found_cognitive,
            "found_verbatim": c.found_verbatim,
            "found_any": c.found_any,
        })
    }).collect();

    Json(serde_json::json!({
        "k": report.k,
        "total_cases": report.total_cases,
        "hits_cognitive": (report.recall_cognitive * report.total_cases as f64).round() as usize,
        "hits_verbatim":  (report.recall_verbatim  * report.total_cases as f64).round() as usize,
        "recall_cognitive": report.recall_cognitive,
        "recall_verbatim":  report.recall_verbatim,
        "recall_combined":  report.recall_combined,
        "cases": cases_out,
    }))
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

// ── Mobile Gateway Handlers ──────────────────────────────────────────────────

use std::sync::{Mutex as StdMutex, OnceLock};

fn mobile_gateway() -> &'static StdMutex<crate::mobile_gateway::MobileGateway> {
    static INSTANCE: OnceLock<StdMutex<crate::mobile_gateway::MobileGateway>> = OnceLock::new();
    INSTANCE.get_or_init(|| StdMutex::new(crate::mobile_gateway::MobileGateway::new()))
}

// -- Request/Response types --

#[derive(Deserialize)]
struct MobileRegisterRequest {
    name: String,
    hostname: String,
    port: u16,
    workspace_root: String,
    api_token: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    tailscale_ip: Option<String>,
    #[serde(default)]
    public_url: Option<String>,
}

#[derive(Deserialize)]
struct MobileHeartbeatRequest {
    #[serde(default)]
    cpu_usage_pct: f64,
    #[serde(default)]
    memory_used_gb: f64,
    #[serde(default)]
    memory_total_gb: f64,
    #[serde(default)]
    disk_used_gb: f64,
    #[serde(default)]
    disk_total_gb: f64,
    #[serde(default)]
    active_agent_sessions: usize,
    #[serde(default)]
    queued_tasks: usize,
    #[serde(default)]
    uptime_secs: u64,
    #[serde(default)]
    provider_name: String,
    #[serde(default)]
    provider_healthy: bool,
}

#[derive(Deserialize)]
struct MobilePairingRequest {
    machine_id: String,
    #[serde(default = "default_pairing_method")]
    method: String,
}

fn default_pairing_method() -> String { "qr_code".to_string() }

#[derive(Deserialize)]
struct MobileAcceptPairingRequest {
    device_id: String,
    device_name: String,
    #[serde(default = "default_platform")]
    platform: String,
    #[serde(default)]
    push_token: Option<String>,
    #[serde(default = "default_version")]
    app_version: String,
    #[serde(default = "default_version")]
    os_version: String,
}

fn default_platform() -> String { "apns".to_string() }
fn default_version() -> String { "1.0.0".to_string() }

#[derive(Deserialize)]
struct MobileVerifyPinRequest {
    pin: String,
}

#[derive(Deserialize)]
struct MobilePushTokenRequest {
    push_token: String,
}

#[derive(Deserialize)]
struct MobileDispatchRequest {
    device_id: String,
    machine_id: String,
    #[serde(default = "default_dispatch_type")]
    dispatch_type: String,
    payload: String,
}

fn default_dispatch_type() -> String { "chat".to_string() }

#[derive(Deserialize)]
struct MobileUpdateDispatchRequest {
    status: String,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

fn parse_pairing_method(s: &str) -> crate::mobile_gateway::PairingMethod {
    match s {
        "pin" => crate::mobile_gateway::PairingMethod::Pin,
        "tailscale" => crate::mobile_gateway::PairingMethod::Tailscale,
        "cloud_relay" => crate::mobile_gateway::PairingMethod::CloudRelay,
        _ => crate::mobile_gateway::PairingMethod::QrCode,
    }
}

fn parse_push_platform(s: &str) -> crate::mobile_gateway::PushPlatform {
    match s {
        "fcm" => crate::mobile_gateway::PushPlatform::Fcm,
        "webpush" => crate::mobile_gateway::PushPlatform::WebPush,
        _ => crate::mobile_gateway::PushPlatform::APNs,
    }
}

fn parse_dispatch_type(s: &str) -> crate::mobile_gateway::DispatchType {
    match s {
        "agent_task" => crate::mobile_gateway::DispatchType::AgentTask,
        "command" => crate::mobile_gateway::DispatchType::Command,
        "repl_command" => crate::mobile_gateway::DispatchType::ReplCommand,
        "file_op" => crate::mobile_gateway::DispatchType::FileOp,
        "git_op" => crate::mobile_gateway::DispatchType::GitOp,
        "cancel" => crate::mobile_gateway::DispatchType::Cancel,
        _ => crate::mobile_gateway::DispatchType::Chat,
    }
}

fn parse_dispatch_status(s: &str) -> crate::mobile_gateway::DispatchStatus {
    match s {
        "sent" => crate::mobile_gateway::DispatchStatus::Sent,
        "running" => crate::mobile_gateway::DispatchStatus::Running,
        "completed" => crate::mobile_gateway::DispatchStatus::Completed,
        "failed" => crate::mobile_gateway::DispatchStatus::Failed,
        "cancelled" => crate::mobile_gateway::DispatchStatus::Cancelled,
        "timed_out" => crate::mobile_gateway::DispatchStatus::TimedOut,
        _ => crate::mobile_gateway::DispatchStatus::Queued,
    }
}

// -- Handlers --

async fn mobile_list_machines(
    _state: State<ServeState>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let summaries: Vec<serde_json::Value> = gw.machine_summaries().iter().map(|s| {
        serde_json::json!({
            "machine_id": s.machine_id,
            "name": s.name,
            "os": s.os,
            "status": s.status,
            "active_tasks": s.active_tasks,
            "paired_devices": s.paired_devices,
            "last_heartbeat": s.last_heartbeat,
            "workspace": s.workspace,
        })
    }).collect();
    Json(serde_json::json!({ "machines": summaries }))
}

async fn mobile_register_machine(
    _state: State<ServeState>,
    Json(req): Json<MobileRegisterRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let machine = gw.register_machine(&req.name, &req.hostname, req.port, &req.workspace_root, &req.api_token);
    let mid = machine.machine_id.clone();

    // Apply optional fields.
    if !req.tags.is_empty() {
        let _ = gw.tag_machine(&mid, req.tags);
    }
    if let Some(ip) = req.tailscale_ip {
        if let Some(m) = gw.machines.get_mut(&mid) {
            m.tailscale_ip = Some(ip);
        }
    }
    if let Some(url) = req.public_url {
        if let Some(m) = gw.machines.get_mut(&mid) {
            m.public_url = Some(url);
        }
    }

    (StatusCode::CREATED, Json(serde_json::json!({
        "machine_id": mid,
        "status": "registered",
    })))
}

async fn mobile_get_machine(
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.get_machine(&id) {
        Some(m) => (StatusCode::OK, Json(serde_json::json!({
            "machine_id": m.machine_id,
            "name": m.name,
            "hostname": m.hostname,
            "os": m.os.to_string(),
            "arch": m.arch,
            "status": m.status.to_string(),
            "daemon_port": m.daemon_port,
            "daemon_version": m.daemon_version,
            "workspace_root": m.workspace_root,
            "capabilities": m.capabilities,
            "active_sessions": m.active_sessions,
            "max_sessions": m.max_sessions,
            "cpu_cores": m.cpu_cores,
            "memory_gb": m.memory_gb,
            "disk_free_gb": m.disk_free_gb,
            "registered_at": m.registered_at,
            "last_heartbeat": m.last_heartbeat,
            "tailscale_ip": m.tailscale_ip,
            "public_url": m.public_url,
            "tags": m.tags,
        }))),
        None => json_error(StatusCode::NOT_FOUND, "Machine not found"),
    }
}

async fn mobile_unregister_machine(
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.unregister_machine(&id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "unregistered" }))),
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_heartbeat(
    Path(id): Path<String>,
    Json(req): Json<MobileHeartbeatRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let metrics = crate::mobile_gateway::MachineMetrics {
        machine_id: id.clone(),
        timestamp: now,
        cpu_usage_pct: req.cpu_usage_pct,
        memory_used_gb: req.memory_used_gb,
        memory_total_gb: req.memory_total_gb,
        disk_used_gb: req.disk_used_gb,
        disk_total_gb: req.disk_total_gb,
        active_agent_sessions: req.active_agent_sessions,
        queued_tasks: req.queued_tasks,
        uptime_secs: req.uptime_secs,
        provider_name: req.provider_name,
        provider_healthy: req.provider_healthy,
    };
    match gw.heartbeat(&id, Some(metrics)) {
        Ok(_) => {
            // Also check for stale machines and timed-out dispatches.
            gw.check_stale_machines();
            gw.check_timeouts();
            let pending = gw.pending_dispatches(&id).len();
            (StatusCode::OK, Json(serde_json::json!({
                "status": "ok",
                "pending_dispatches": pending,
            })))
        }
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_create_pairing(
    Json(req): Json<MobilePairingRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let method = parse_pairing_method(&req.method);
    match gw.create_pairing(&req.machine_id, method) {
        Ok(p) => (StatusCode::CREATED, Json(serde_json::json!({
            "pairing_id": p.id,
            "pin": p.pin,
            "qr_data": p.qr_data,
            "expires_at": p.expires_at,
        }))),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e),
    }
}

async fn mobile_accept_pairing(
    Path(id): Path<String>,
    Json(req): Json<MobileAcceptPairingRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let platform = parse_push_platform(&req.platform);
    match gw.accept_pairing(&id, &req.device_id, &req.device_name, platform, req.push_token, &req.app_version, &req.os_version) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "paired" }))),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e),
    }
}

async fn mobile_verify_pin(
    Path(id): Path<String>,
    Json(req): Json<MobileVerifyPinRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.verify_pin(&id, &req.pin) {
        Ok(valid) => (StatusCode::OK, Json(serde_json::json!({ "valid": valid }))),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e),
    }
}

async fn mobile_reject_pairing(
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.reject_pairing(&id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "rejected" }))),
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_list_devices(
    _state: State<ServeState>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let devices: Vec<serde_json::Value> = gw.devices.values().map(|d| {
        serde_json::json!({
            "device_id": d.device_id,
            "device_name": d.device_name,
            "platform": d.platform.to_string(),
            "paired_machines": d.paired_machines,
            "paired_at": d.paired_at,
            "last_seen": d.last_seen,
            "app_version": d.app_version,
            "os_version": d.os_version,
            "has_push_token": d.push_token.is_some(),
        })
    }).collect();
    Json(serde_json::json!({ "devices": devices }))
}

async fn mobile_update_push_token(
    Path(id): Path<String>,
    Json(req): Json<MobilePushTokenRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.update_push_token(&id, &req.push_token) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "updated" }))),
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_unpair(
    Path((device_id, machine_id)): Path<(String, String)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.unpair_device(&device_id, &machine_id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "unpaired" }))),
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_dispatch(
    Json(req): Json<MobileDispatchRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let dtype = parse_dispatch_type(&req.dispatch_type);
    match gw.dispatch_task(&req.device_id, &req.machine_id, dtype, &req.payload) {
        Ok(t) => (StatusCode::CREATED, Json(serde_json::json!({
            "task_id": t.task_id,
            "status": t.status.to_string(),
        }))),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e),
    }
}

async fn mobile_get_dispatch(
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.get_dispatch(&id) {
        Some(t) => (StatusCode::OK, Json(serde_json::json!({
            "task_id": t.task_id,
            "machine_id": t.machine_id,
            "device_id": t.device_id,
            "dispatch_type": t.dispatch_type.to_string(),
            "payload": t.payload,
            "status": t.status.to_string(),
            "created_at": t.created_at,
            "started_at": t.started_at,
            "completed_at": t.completed_at,
            "result": t.result,
            "error": t.error,
            "session_id": t.session_id,
        }))),
        None => json_error(StatusCode::NOT_FOUND, "Dispatch not found"),
    }
}

async fn mobile_cancel_dispatch(
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    match gw.cancel_dispatch(&id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "cancelled" }))),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e),
    }
}

async fn mobile_update_dispatch(
    Path(id): Path<String>,
    Json(req): Json<MobileUpdateDispatchRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let status = parse_dispatch_status(&req.status);
    match gw.update_dispatch(&id, status, req.result, req.error, req.session_id) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "updated" }))),
        Err(e) => json_error(StatusCode::NOT_FOUND, e),
    }
}

async fn mobile_machine_dispatches(
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let tasks: Vec<serde_json::Value> = gw.list_dispatches_for_machine(&id).iter().map(|t| {
        serde_json::json!({
            "task_id": t.task_id,
            "dispatch_type": t.dispatch_type.to_string(),
            "payload": t.payload,
            "status": t.status.to_string(),
            "created_at": t.created_at,
            "result": t.result,
        })
    }).collect();
    Json(serde_json::json!({ "dispatches": tasks }))
}

async fn mobile_device_dispatches(
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let tasks: Vec<serde_json::Value> = gw.list_dispatches_for_device(&id).iter().map(|t| {
        serde_json::json!({
            "task_id": t.task_id,
            "machine_id": t.machine_id,
            "dispatch_type": t.dispatch_type.to_string(),
            "payload": t.payload,
            "status": t.status.to_string(),
            "created_at": t.created_at,
            "result": t.result,
        })
    }).collect();
    Json(serde_json::json!({ "dispatches": tasks }))
}

async fn mobile_notifications(
    Path(device_id): Path<String>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let notifs: Vec<serde_json::Value> = gw.unsent_notifications(&device_id).iter().map(|n| {
        serde_json::json!({
            "id": n.id,
            "title": n.title,
            "body": n.body,
            "category": n.category.to_string(),
            "data": n.data,
            "created_at": n.created_at,
        })
    }).collect();
    Json(serde_json::json!({ "notifications": notifs }))
}

async fn mobile_stats(
    _state: State<ServeState>,
) -> Json<serde_json::Value> {
    let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
    let s = gw.stats();
    Json(serde_json::json!({
        "total_machines": s.total_machines,
        "online_machines": s.online_machines,
        "total_devices": s.total_devices,
        "total_dispatches": s.total_dispatches,
        "active_dispatches": s.active_dispatches,
        "completed_dispatches": s.completed_dispatches,
        "failed_dispatches": s.failed_dispatches,
        "pending_notifications": s.pending_notifications,
        "pending_pairings": s.pending_pairings,
    }))
}

// ── Mobile Handoff Endpoints ──────────────────────────────────────────────────

/// Info about the currently active (or most recently finished) session.
#[derive(Debug, Serialize)]
struct ActiveSessionInfo {
    session_id: String,
    task: String,
    provider: String,
    status: String,
    started_at: u64,
    message_count: usize,
    summary: Option<String>,
}

/// Response for `GET /mobile/beacon` (no auth required).
#[derive(Debug, Serialize)]
struct BeaconResponse {
    machine_id: String,
    hostname: String,
    daemon_version: &'static str,
    port: u16,
    lan_ips: Vec<String>,
    tailscale_ip: Option<String>,
    public_url: Option<String>,
    uptime_secs: u64,
    active_session: Option<ActiveSessionInfo>,
}

/// `GET /mobile/beacon` — no auth required.
///
/// Fast probe endpoint used by the mobile app to discover the best URL,
/// detect machine presence, and check for an active session to hand off.
async fn mobile_beacon(State(state): State<ServeState>) -> Json<serde_json::Value> {
    // Hostname via process invocation (no extra dep).
    let hostname = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Derive primary LAN IP by binding a UDP socket and connecting to a public
    // address — no packet is actually sent; we just read the OS-chosen source IP.
    let lan_ips: Vec<String> = {
        let mut ips = Vec::new();
        if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if sock.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = sock.local_addr() {
                    ips.push(addr.ip().to_string());
                }
            }
        }
        if ips.is_empty() {
            ips.push("127.0.0.1".to_string());
        }
        ips
    };

    // Tailscale IP (best-effort; silently ignored if tailscale is not installed).
    let tailscale_ip = crate::tailscale::tailscale_status()
        .ok()
        .and_then(|s| s.tailscale_ip);

    // Public URL: prefer ngrok/Tailscale Funnel (cached at startup), fall back
    // to the pairing registry populated by the mobile gateway endpoint.
    let public_url = state
        .public_url_cache
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .or_else(|| {
            let gw = mobile_gateway().lock().unwrap_or_else(|e| e.into_inner());
            gw.machines.values().find_map(|m| m.public_url.clone())
        });

    // machine_id: stable identifier based on hostname + jobs_dir path hash.
    let machine_id = {
        let raw = format!("{}:{}", hostname, state.jobs_dir.display());
        format!("{:x}", {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in raw.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        })
    };

    let uptime_secs = state.started_at.elapsed().as_secs();

    // Find active session: prefer "running", fall back to a recently finished
    // job (within the last 15 minutes = 900_000 ms).
    let active_session: Option<ActiveSessionInfo> = {
        let jobs = state.job_manager.list().await;
        let now = now_ms();
        let fifteen_min_ms: u64 = 15 * 60 * 1_000;

        let chosen = jobs.iter().find(|j| j.status == "running").or_else(|| {
            jobs.iter().find(|j| {
                j.finished_at
                    .map(|fa| now.saturating_sub(fa) <= fifteen_min_ms)
                    .unwrap_or(false)
            })
        });

        chosen.map(|job| {
            // Try to get message count from SessionStore.
            let message_count = SessionStore::open_default()
                .ok()
                .and_then(|store| store.get_session_detail(&job.session_id).ok().flatten())
                .map(|d| d.messages.len())
                .unwrap_or(0);

            ActiveSessionInfo {
                session_id: job.session_id.clone(),
                task: job.task.clone(),
                provider: job.provider.clone(),
                status: job.status.clone(),
                started_at: job.started_at,
                message_count,
                summary: job.summary.clone(),
            }
        })
    };

    Json(serde_json::to_value(BeaconResponse {
        machine_id,
        hostname,
        daemon_version: env!("CARGO_PKG_VERSION"),
        port: 0, // port is not stored in state; 0 means "use the URL you connected to"
        lan_ips,
        tailscale_ip,
        public_url,
        uptime_secs,
        active_session,
    })
    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})))
}

/// Per-session record returned by `GET /mobile/sessions`.
#[derive(Debug, Serialize)]
struct MobileSessionRecord {
    session_id: String,
    task: String,
    provider: String,
    status: String,
    started_at: u64,
    finished_at: Option<u64>,
    summary: Option<String>,
    message_count: usize,
    last_message_preview: Option<String>,
}

/// `GET /mobile/sessions` — auth required.
///
/// Returns up to 50 sessions (most-recent first) with message previews,
/// suitable for rendering a "pick up where you left off" list in the mobile app.
async fn mobile_sessions(State(state): State<ServeState>) -> Json<serde_json::Value> {
    let jobs = state.job_manager.list().await;
    let limited: Vec<&JobRecord> = jobs.iter().take(50).collect();

    // Open the session store once for all lookups (best-effort).
    let store = SessionStore::open_default().ok();

    let sessions: Vec<MobileSessionRecord> = limited
        .into_iter()
        .map(|job| {
            let (message_count, last_message_preview) = store
                .as_ref()
                .and_then(|s| s.get_session_detail(&job.session_id).ok().flatten())
                .map(|d| {
                    let count = d.messages.len();
                    let preview = d.messages.last().map(|m| {
                        let s = m.content.chars().take(120).collect::<String>();
                        s
                    });
                    (count, preview)
                })
                .unwrap_or((0, None));

            MobileSessionRecord {
                session_id: job.session_id.clone(),
                task: job.task.clone(),
                provider: job.provider.clone(),
                status: job.status.clone(),
                started_at: job.started_at,
                finished_at: job.finished_at,
                summary: job.summary.clone(),
                message_count,
                last_message_preview,
            }
        })
        .collect();

    Json(serde_json::json!({ "sessions": sessions }))
}

/// `GET /mobile/sessions/:id/context` — auth required.
///
/// Returns a HandoffContext JSON bundle for the requested session so the
/// mobile app can "continue" it with full conversation history. Phase 7
/// quick-win: now also includes an `assembler` block with the project
/// memory + orchestration sections (built via the same context assembler
/// the REPL chat path uses), so mobile can show "what the agent knows"
/// instead of just the chat transcript.
async fn mobile_session_context(
    State(state): State<ServeState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("session store unavailable: {e}") })),
            );
        }
    };

    let detail = match store.get_session_detail(&id) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "session not found" })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("failed to load session: {e}") })),
            );
        }
    };

    // Pick the workspace for memory assembly: the session's recorded
    // project_path wins, otherwise fall back to the daemon's own
    // workspace_root (the directory the daemon was started in).
    let workspace = detail
        .session
        .project_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| state.workspace_root.clone());

    match build_mobile_context_response(&id, &detail, &workspace) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("serialization failed: {e}") })),
        ),
    }
}

/// Pure builder for the `/mobile/sessions/:id/context` response body.
///
/// Extracted from the handler so unit tests can exercise the merge of
/// HandoffContext (session transcript) + AssembledContext (project
/// memory + orchestration) without touching `SessionStore::open_default`
/// or any daemon-global state. The handler is the only production
/// caller; tests inject a `SessionDetail` directly.
fn build_mobile_context_response(
    session_id: &str,
    detail: &crate::session_store::SessionDetail,
    workspace: &std::path::Path,
) -> anyhow::Result<serde_json::Value> {
    // 1. Session transcript → HandoffContext (preserves existing payload
    //    shape so mobile clients on the old envelope keep working).
    let mut ctx = crate::context_handoff::HandoffContext::new(&detail.session.provider);
    if let Some(ref sys) = detail.session.summary {
        ctx = ctx.with_system(sys.clone());
    }
    for msg in &detail.messages {
        let handoff_msg = match msg.role.as_str() {
            "user" => crate::context_handoff::HandoffMessage::user(&msg.content),
            "assistant" => crate::context_handoff::HandoffMessage::assistant(&msg.content),
            "system" => crate::context_handoff::HandoffMessage::system(&msg.content),
            _ => crate::context_handoff::HandoffMessage::user(&msg.content),
        };
        ctx.push_message(handoff_msg);
    }
    let json_str = ctx
        .serialize()
        .map_err(|e| anyhow::anyhow!("HandoffContext serialize failed: {e}"))?;
    let inner: serde_json::Value =
        serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);

    // 2. Memory layer — Chat policy with a Chat-kind budget. Same
    //    assembler the REPL chat path uses (`main.rs:4135`), so mobile
    //    sees identical sections in identical priority order.
    let policy = crate::context_assembler::ContextPolicy::Chat;
    let budget = crate::context_assembler::ContextBudget::for_kind(
        crate::context_assembler::AgentKind::Chat,
    );
    let toggles = crate::context_assembler::MemoryToggles::default();
    let assembled =
        crate::context_assembler::assemble_context(workspace, &policy, &budget, &toggles);

    let assembler_payload = serde_json::json!({
        "total_chars": assembled.total_chars,
        "sections": assembled
            .sections
            .iter()
            .map(|s| serde_json::json!({
                "name": s.name,
                "content": s.content,
                "priority": s.priority,
                "truncated": s.truncated,
            }))
            .collect::<Vec<_>>(),
    });

    Ok(serde_json::json!({
        "context_type": "vibecody_session",
        "session_id": session_id,
        "context": inner,
        "assembler": assembler_payload,
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

    // ── Phase 7 quick-win: mobile context response includes assembler ──────

    /// Build a minimal `SessionDetail` for the mobile-context tests. Pure
    /// data, no DB. Fixture lives only in this module.
    fn fixture_detail(id: &str, project_path: Option<&str>) -> crate::session_store::SessionDetail {
        crate::session_store::SessionDetail {
            session: crate::session_store::SessionRow {
                id: id.to_string(),
                task: "explain the codebase".to_string(),
                provider: "mock".to_string(),
                model: "test-model".to_string(),
                started_at: 0,
                finished_at: None,
                status: "running".to_string(),
                summary: Some("system summary".to_string()),
                step_count: 0,
                parent_session_id: None,
                depth: 0,
                project_path: project_path.map(str::to_string),
            },
            messages: vec![
                crate::session_store::MessageRow {
                    id: 1,
                    session_id: id.to_string(),
                    role: "user".to_string(),
                    content: "hello".to_string(),
                    created_at: 0,
                },
                crate::session_store::MessageRow {
                    id: 2,
                    session_id: id.to_string(),
                    role: "assistant".to_string(),
                    content: "hi there".to_string(),
                    created_at: 0,
                },
            ],
            steps: vec![],
        }
    }

    #[test]
    fn mobile_context_response_preserves_existing_envelope() {
        // Backwards-compatibility: the original `context_type`,
        // `session_id`, `context` keys must still be present so existing
        // mobile clients don't break when we add the assembler block.
        let workspace = tempfile::TempDir::new().unwrap();
        let detail = fixture_detail("sess-abc", None);
        let value =
            build_mobile_context_response("sess-abc", &detail, workspace.path())
                .expect("build response");

        assert_eq!(value["context_type"], "vibecody_session");
        assert_eq!(value["session_id"], "sess-abc");
        assert!(value["context"].is_object(), "context must be a JSON object");
    }

    #[test]
    fn mobile_context_response_includes_assembler_block() {
        // The new contract: response gains an `assembler` key with the
        // sections + total_chars from the same context_assembler the
        // REPL chat path uses. Empty workspace → no project_memory →
        // assembler is present but with zero sections (Chat policy
        // doesn't pull anything else for an empty workspace).
        let workspace = tempfile::TempDir::new().unwrap();
        let detail = fixture_detail("sess-empty", None);
        let value = build_mobile_context_response(
            "sess-empty",
            &detail,
            workspace.path(),
        )
        .expect("build response");

        let assembler = value
            .get("assembler")
            .expect("response must include assembler block");
        assert!(
            assembler.get("sections").is_some(),
            "assembler must expose `sections`"
        );
        assert!(
            assembler.get("total_chars").is_some(),
            "assembler must expose `total_chars`"
        );
    }

    #[test]
    fn mobile_context_response_surfaces_project_memory_section() {
        // When the workspace has a hierarchical project memory file
        // (CLAUDE.md / VIBECLI.md / AGENTS.md), the assembler must lift
        // its content into the `project_memory` section so mobile shows
        // it as part of "what the agent knows."
        let workspace = tempfile::TempDir::new().unwrap();
        std::fs::write(
            workspace.path().join("CLAUDE.md"),
            "# Local Project Memory\n\nThis project uses Rust edition 2021.",
        )
        .expect("write CLAUDE.md");

        let detail = fixture_detail("sess-mem", None);
        let value = build_mobile_context_response(
            "sess-mem",
            &detail,
            workspace.path(),
        )
        .expect("build response");

        let sections = value["assembler"]["sections"]
            .as_array()
            .expect("sections array");
        let project_memory = sections
            .iter()
            .find(|s| s["name"] == "project_memory")
            .expect("project_memory section must be present");
        let content = project_memory["content"]
            .as_str()
            .expect("content is a string");
        assert!(
            content.contains("Rust edition 2021"),
            "project_memory must reflect CLAUDE.md content; got: {content}"
        );
    }

    // ── JobRecord serde backward-compat ────────────────────────────────────
    //
    // The durable JSON-per-job format was replaced by the SQLite-backed
    // `JobManager`. The tests below assert that a pre-M1 JobRecord JSON (the
    // old 7-field shape) still deserialises cleanly thanks to `#[serde(default)]`
    // on the new fields — the migration path in `JobManager::migrate_json_jobs`
    // depends on this invariant.

    #[test]
    fn job_record_deserialize_with_missing_optionals() {
        let json = r#"{"session_id":"x","task":"t","status":"running","provider":"p","started_at":0}"#;
        let job: JobRecord = serde_json::from_str(json).unwrap();
        assert!(job.finished_at.is_none());
        assert!(job.summary.is_none());
        // New fields fall back to serde defaults.
        assert_eq!(job.priority, 5);
        assert_eq!(job.queued_at, 0);
        assert!(job.tags.is_empty());
        assert_eq!(job.steps_completed, 0);
    }

    #[test]
    fn job_record_deserialize_pre_m1_full_shape() {
        let json = r#"{
            "session_id": "abc123",
            "task": "legacy",
            "status": "complete",
            "provider": "ollama",
            "started_at": 1700000000000,
            "finished_at": 1700000060000,
            "summary": "done"
        }"#;
        let job: JobRecord = serde_json::from_str(json).unwrap();
        assert_eq!(job.session_id, "abc123");
        assert_eq!(job.status, "complete");
        assert_eq!(job.finished_at, Some(1_700_000_060_000));
        assert_eq!(job.summary.as_deref(), Some("done"));
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
            let db = crate::job_manager::JobsDb::open_with(
                &tmp_dir.path().join("jobs.db"),
                [42u8; 32],
            )
            .unwrap();
            let state = ServeState {
                provider: Arc::new(MockProvider),
                approval: ApprovalPolicy::FullAuto,
                workspace_root: tmp_dir.path().to_path_buf(),
                job_manager: Arc::new(crate::job_manager::JobManager::new_with(db)),
                jobs_dir: tmp_dir.path().to_path_buf(),
                provider_name: "mock".to_string(),
                api_token: token.to_string(),
                collab_server: Arc::new(CollabServer::new(5)),
                github_app_config: crate::github_app::GithubAppConfig::default(),
                started_at: std::time::Instant::now(),
                public_url_cache: Arc::new(std::sync::Mutex::new(None)),
                inference_router: None,
                rl_run_store: {
                    let s = Arc::new(
                        crate::rl_runs::RunStore::open_with(&tmp_dir.path().join("rl.db"))
                            .unwrap(),
                    );
                    s
                },
                rl_executor: Arc::new(crate::rl_executor::PythonExecutor::new(
                    crate::rl_executor::ExecutorConfig::from_env(),
                    Arc::new(
                        crate::rl_runs::RunStore::open_with(&tmp_dir.path().join("rl_x.db"))
                            .unwrap(),
                    ),
                )),
                rl_runtime_pool: Arc::new(crate::rl_runtime::RuntimePool::new()),
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

        #[tokio::test]
        async fn health_exposes_providers_block_with_count_and_names() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            // The block must exist, even when no providers are configured.
            // configured_count is the canonical readiness signal for any
            // feature that depends on AI providers.
            assert!(json["providers"].is_object(), "providers block missing");
            assert!(
                json["providers"]["configured_count"].is_u64(),
                "providers.configured_count must be a non-negative integer"
            );
            assert!(
                json["providers"]["names"].is_array(),
                "providers.names must always be an array (empty when none)"
            );
        }

        #[tokio::test]
        async fn health_features_diffcomplete_inherits_from_providers() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            let dc = &json["features"]["diffcomplete"];
            assert!(dc.is_object(), "features.diffcomplete missing");
            assert!(
                dc["available"].is_boolean(),
                "features.diffcomplete.available must be a boolean"
            );
            assert_eq!(
                dc["transport"], "tauri-desktop",
                "diffcomplete is desktop-only by current design"
            );
            // Availability must follow the providers count rule.
            let count = json["providers"]["configured_count"].as_u64().unwrap();
            let available = dc["available"].as_bool().unwrap();
            assert_eq!(
                available,
                count > 0,
                "features.diffcomplete.available must equal providers.configured_count > 0"
            );
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

        // ── S3: HTTP context negotiation (capabilities + agent payload) ──

        #[tokio::test]
        async fn v1_capabilities_advertises_context_kinds_and_sections() {
            // Mobile/watch/IDE clients hit /v1/capabilities before
            // sending any /agent request to discover what the daemon
            // supports. This pins that the response includes a
            // `context.kinds` array with all four AgentKind variants
            // and a `context.sections` array covering every
            // assembler-emitted section name. Drift here breaks
            // negotiation silently — clients fall back to defaults
            // and lose memory features.
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/v1/capabilities")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            let context = json
                .get("capabilities")
                .and_then(|c| c.get("context"))
                .expect("capabilities.context block present");
            let kinds = context
                .get("kinds")
                .and_then(|k| k.as_array())
                .expect("context.kinds array");
            let kind_strs: Vec<&str> =
                kinds.iter().filter_map(|v| v.as_str()).collect();
            assert!(kind_strs.contains(&"Chat"));
            assert!(kind_strs.contains(&"CodingAgent"));
            assert!(kind_strs.contains(&"ResearchAgent"));
            assert!(kind_strs.contains(&"BackgroundJob"));

            let sections = context
                .get("sections")
                .and_then(|s| s.as_array())
                .expect("context.sections array");
            let section_strs: Vec<&str> =
                sections.iter().filter_map(|v| v.as_str()).collect();
            assert!(section_strs.contains(&"project_memory"));
            assert!(section_strs.contains(&"orchestration"));
            assert!(section_strs.contains(&"open_memory"));
            assert!(section_strs.contains(&"agent_scratchpad"));
        }

        #[tokio::test]
        async fn agent_with_unknown_context_kind_returns_400() {
            // Negotiation pre-flight: an unknown `kind` must reject
            // before the daemon spawns a job. Mirrors the same
            // canonical valid-list as parse_agent_kind. The error
            // body must echo the bad kind back so the client can log
            // it; this is the contract `parse_agent_kind` advertises.
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .method("POST")
                .uri("/agent")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"task":"do stuff","context_request":{"kind":"Sycophant"}}"#,
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "unknown kind must reject before spawn"
            );
            let body = body_string(resp.into_body()).await;
            assert!(
                body.contains("Sycophant"),
                "error must echo bad kind; got: {body}"
            );
        }

        #[tokio::test]
        async fn agent_without_context_request_remains_backward_compatible() {
            // The pre-S3 payload (no context_request field) must still
            // accept and spawn. Pinning this so the additive change
            // doesn't accidentally tighten the schema for existing
            // mobile/watch clients that haven't been updated.
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .method("POST")
                .uri("/agent")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"task":"plain old request"}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            // 200 OK with a session_id payload, just like before S3.
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "missing context_request must not regress existing clients"
            );
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(
                json.get("session_id").is_some(),
                "response must include session_id; got: {body}"
            );
        }

        #[tokio::test]
        async fn agent_with_valid_context_request_spawns_job() {
            // Happy path: a valid kind + toggles negotiation parses
            // and the daemon accepts it. The actual memory wiring
            // happens inside the spawned task; this test just pins
            // the request-acceptance contract so the negotiation
            // surface is stable.
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .method("POST")
                .uri("/agent")
                .header("authorization", "Bearer test-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"task":"survey the codebase","context_request":{"kind":"ResearchAgent","openmemory_enabled":true}}"#,
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(
                json.get("session_id").is_some(),
                "valid context_request must produce a session_id; got: {body}"
            );
        }

        #[test]
        fn build_agent_context_from_request_defaults_kind_to_coding_agent() {
            // When `kind` is omitted entirely, the daemon must
            // default to `CodingAgent` (its primary use case). Pinning
            // this so a future refactor doesn't silently change which
            // budget shape unspecified requests fall into.
            let workspace = tempfile::TempDir::new().unwrap();
            let cr = ContextRequest {
                kind: None,
                openmemory_enabled: None,
                openmemory_auto_inject: None,
            };
            let ctx = build_agent_context_from_request(
                workspace.path().to_path_buf(),
                "do something",
                "session-S3",
                &cr,
                None,
            )
            .expect("default kind must succeed");
            assert_eq!(ctx.workspace_root, workspace.path());
            // The contract is "no panic, valid context returned" — the
            // exact memory_context value depends on global state we
            // don't want to assert on here.
            let _ = ctx.memory_context;
        }

        #[test]
        fn build_agent_context_from_request_rejects_unknown_kind() {
            let workspace = tempfile::TempDir::new().unwrap();
            let cr = ContextRequest {
                kind: Some("Sycophant".to_string()),
                openmemory_enabled: None,
                openmemory_auto_inject: None,
            };
            let err = build_agent_context_from_request(
                workspace.path().to_path_buf(),
                "x",
                "y",
                &cr,
                None,
            )
            .expect_err("unknown kind must error");
            assert!(err.contains("Sycophant"));
        }

        // ── Inference (Ollama-compat) routes ──────────────────────────────

        /// Variant of `test_app` with an inference Router wired up.
        /// Both backends fail at request time in tests (no ollama daemon
        /// running, and either no `mistralrs_enabled` cfg or no model
        /// cache available) — these tests verify routing / auth / error
        /// mapping, not real inference.
        fn test_app_with_inference(token: &str) -> (Router, tempfile::TempDir) {
            let tmp_dir = tempfile::tempdir().unwrap();
            let db = crate::job_manager::JobsDb::open_with(
                &tmp_dir.path().join("jobs.db"),
                [42u8; 32],
            )
            .unwrap();
            let state = ServeState {
                provider: Arc::new(MockProvider),
                approval: ApprovalPolicy::FullAuto,
                workspace_root: tmp_dir.path().to_path_buf(),
                job_manager: Arc::new(crate::job_manager::JobManager::new_with(db)),
                jobs_dir: tmp_dir.path().to_path_buf(),
                provider_name: "mock".to_string(),
                api_token: token.to_string(),
                collab_server: Arc::new(CollabServer::new(5)),
                github_app_config: crate::github_app::GithubAppConfig::default(),
                started_at: std::time::Instant::now(),
                public_url_cache: Arc::new(std::sync::Mutex::new(None)),
                inference_router: Some(Arc::new(crate::inference::Router::new(
                    crate::inference::backend::BackendKind::Ollama,
                ))),
                rl_run_store: Arc::new(
                    crate::rl_runs::RunStore::open_with(&tmp_dir.path().join("rl.db")).unwrap(),
                ),
                rl_executor: Arc::new(crate::rl_executor::PythonExecutor::new(
                    crate::rl_executor::ExecutorConfig::from_env(),
                    Arc::new(
                        crate::rl_runs::RunStore::open_with(
                            &tmp_dir.path().join("rl-exec.db"),
                        )
                        .unwrap(),
                    ),
                )),
                rl_runtime_pool: Arc::new(crate::rl_runtime::RuntimePool::new()),
            };
            (build_router(state, 7878), tmp_dir)
        }

        #[tokio::test]
        async fn api_chat_without_auth_returns_401() {
            let (app, _tmp) = test_app_with_inference("secret-token");
            let req = Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"model":"qwen2.5:0.5b","messages":[{"role":"user","content":"hi"}]}"#,
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn api_tags_without_auth_returns_401() {
            let (app, _tmp) = test_app_with_inference("secret-token");
            let req = Request::builder().uri("/api/tags").body(Body::empty()).unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn api_tags_returns_503_when_router_absent() {
            // `test_app` (no inference router) — handler must return 503 not panic.
            let (app, _tmp) = test_app("secret-token");
            let req = Request::builder()
                .uri("/api/tags")
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        }

        #[tokio::test]
        async fn api_chat_with_mistralrs_header_routes_to_backend() {
            // Two cfg modes (set by build.rs):
            //   - cfg(not(mistralrs_enabled)) — Linux/Windows w/o feature:
            //     MistralRsBackend surfaces Unavailable → handler maps to 503.
            //   - cfg(mistralrs_enabled) — macOS or feature on: backend
            //     attempts a real model load, which fails in CI (no
            //     network / model cache) → non-200 status.
            let (app, _tmp) = test_app_with_inference("secret-token");
            let req = Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("authorization", "Bearer secret-token")
                .header("x-vibecli-backend", "mistralrs")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"model":"Qwen/Qwen2.5-0.5B-Instruct","messages":[{"role":"user","content":"hi"}]}"#,
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            #[cfg(not(mistralrs_enabled))]
            assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
            // With mistralrs_enabled, the backend is actually invoked.
            // Outcome depends on whether the model is cached: 200 if it is,
            // 502/500 if it has to fault the network and that fails.
            // Either way, it must NOT be 503 — that would mean we fell
            // through to the canned "feature off" path.
            #[cfg(mistralrs_enabled)]
            assert_ne!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
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

        // ── GET /v1/metrics/jobs ─────────────────────────────────────────

        #[tokio::test]
        async fn metrics_jobs_requires_auth() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/v1/metrics/jobs")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn metrics_jobs_returns_zero_snapshot_on_fresh_daemon() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/v1/metrics/jobs")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let snap: crate::job_manager::JobManagerMetrics =
                serde_json::from_str(&body).expect("snapshot deserializes");
            assert_eq!(snap, crate::job_manager::JobManagerMetrics::default());
        }

        // ── GET /memory/stats includes TurboQuant index info ─────────────
        // Locks the contract added in 3a57b789 so mobile/watch/SDK clients
        // can rely on these fields being present.
        #[tokio::test]
        async fn memory_stats_includes_turboquant_index_fields() {
            let (app, _tmp) = test_app("tok");
            let req = Request::builder()
                .uri("/memory/stats")
                .header("authorization", "Bearer tok")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = body_string(resp.into_body()).await;
            let v: serde_json::Value =
                serde_json::from_str(&body).expect("response is JSON");

            let dim = v.get("embedding_dim").and_then(|x| x.as_u64())
                .expect("embedding_dim field present and numeric");
            assert!(dim > 0, "embedding_dim should be positive, got {dim}");

            let ratio = v.get("embedding_compression_ratio").and_then(|x| x.as_f64())
                .expect("embedding_compression_ratio field present and numeric");
            assert!(
                ratio > 1.0,
                "compression ratio should beat raw f32, got {ratio}"
            );

            let backend = v.get("embedding_backend").and_then(|x| x.as_str())
                .expect("embedding_backend field present and string");
            assert_eq!(backend, "turboquant");
        }

        // ── F1.2: /v1/recap auth enforcement ───────────────────────────────

        #[tokio::test]
        async fn recap_post_without_auth_returns_401() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .method("POST")
                .uri("/v1/recap")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"session","subject_id":"x"}"#))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn recap_get_without_auth_returns_401() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/v1/recap/some-id")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn recap_list_without_auth_returns_401() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/v1/recap?kind=session&subject_id=x")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        // ── F1.3: /v1/resume auth enforcement ───────────────────────────

        #[tokio::test]
        async fn resume_post_without_auth_returns_401() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .method("POST")
                .uri("/v1/resume")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"from_subject_id":"x","kind":"session"}"#,
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn resume_get_without_auth_returns_401() {
            let (app, _tmp) = test_app("test-token");
            let req = Request::builder()
                .uri("/v1/resume/some-handle")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
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

    // ── MemPalace endpoint request structs ────────────────────────────────

    #[test]
    fn memory_chunk_request_defaults_source_to_empty() {
        let json = r#"{"content":"hello world"}"#;
        let req: MemoryChunkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "hello world");
        assert_eq!(req.source, "");
    }

    #[test]
    fn memory_chunk_request_accepts_explicit_source() {
        let json = r#"{"content":"some text","source":"runbook.txt"}"#;
        let req: MemoryChunkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.source, "runbook.txt");
    }

    #[test]
    fn memory_tunnel_request_defaults_weight() {
        let json = r#"{"src_id":"mem_aaa","dst_id":"mem_bbb"}"#;
        let req: MemoryTunnelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.src_id, "mem_aaa");
        assert_eq!(req.dst_id, "mem_bbb");
        assert!((req.weight - 0.8).abs() < 1e-9, "default weight should be 0.8");
    }

    #[test]
    fn memory_tunnel_request_accepts_custom_weight() {
        let json = r#"{"src_id":"a","dst_id":"b","weight":0.95}"#;
        let req: MemoryTunnelRequest = serde_json::from_str(json).unwrap();
        assert!((req.weight - 0.95).abs() < 1e-9);
    }

    #[test]
    fn memory_auto_tunnel_request_defaults_threshold() {
        let json = r#"{}"#;
        let req: MemoryAutoTunnelRequest = serde_json::from_str(json).unwrap();
        assert!((req.threshold - 0.75).abs() < 1e-9, "default threshold should be 0.75");
    }

    #[test]
    fn memory_context_request_defaults() {
        let json = r#"{"query":"deploy process"}"#;
        let req: MemoryContextRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "deploy process");
        assert_eq!(req.l1_tokens, 700);
        assert_eq!(req.l2_limit, 8);
        assert_eq!(req.l3_threshold, 3);
    }

    #[test]
    fn memory_benchmark_query_defaults_k_to_5() {
        let query: MemoryBenchmarkQuery = serde_json::from_str(r#"{"k":5}"#).unwrap();
        assert_eq!(query.k, 5);
        let default_query: MemoryBenchmarkQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(default_query.k, 5);
    }

    #[test]
    fn memory_benchmark_query_accepts_custom_k() {
        let query: MemoryBenchmarkQuery = serde_json::from_str(r#"{"k":10}"#).unwrap();
        assert_eq!(query.k, 10);
    }

    #[test]
    fn default_helpers_return_expected_values() {
        assert_eq!(default_tunnel_weight(), 0.8);
        assert_eq!(default_auto_tunnel_threshold(), 0.75);
        assert_eq!(default_l1_tokens(), 700);
        assert_eq!(default_l2_limit(), 8);
        assert_eq!(default_l3_threshold(), 3);
        assert_eq!(default_bench_k(), 5);
        assert_eq!(default_import_format(), "auto");
    }

    #[test]
    fn memory_import_request_defaults_format_to_auto() {
        let json = r#"{"content":"{\"memories\":[]}"}"#;
        let req: MemoryImportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.format, "auto");
    }

    #[test]
    fn memory_import_request_accepts_explicit_format() {
        for fmt in &["mem0", "zep", "openmemory", "auto"] {
            let json = format!(r#"{{"content":"[]","format":"{}"}}"#, fmt);
            let req: MemoryImportRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(&req.format, fmt);
        }
    }

    #[test]
    fn memory_id_request_roundtrip() {
        let json = r#"{"id":"mem_abc123"}"#;
        let req: MemoryIdRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "mem_abc123");
    }

    #[test]
    fn memory_id_request_accepts_prefix_ids() {
        // Confirm short prefix IDs (as typed in the REPL) deserialize correctly
        for id in &["mem_a3f8", "mem_b7e4", "abc", "mem_00000000"] {
            let json = format!(r#"{{"id":"{}"}}"#, id);
            let req: MemoryIdRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(&req.id, id);
        }
    }

    #[test]
    fn memory_id_request_requires_id_field() {
        // Missing "id" field must fail deserialization
        let result: Result<MemoryIdRequest, _> = serde_json::from_str(r#"{}"#);
        assert!(result.is_err(), "MemoryIdRequest with no id should fail");
    }

    // ── F1.2: /v1/recap helpers ──────────────────────────────────────────

    /// Build a tempdir-scoped SessionStore with one session + one
    /// user message. Mirrors the F1.1 fixture in session_store::tests
    /// but kept local so this module doesn't reach across the
    /// `#[cfg(test)]` boundary.
    fn recap_route_fixture() -> (
        crate::session_store::SessionStore,
        String,
        tempfile::TempDir,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::session_store::SessionStore::open(
            dir.path().join("sessions.db"),
        )
        .unwrap();
        let sid = "F12-route-test".to_string();
        store
            .insert_session_with_parent(
                &sid,
                "Refactor the auth middleware",
                "mock",
                "test-model",
                None,
                0,
            )
            .unwrap();
        store
            .insert_message(&sid, "user", "Refactor the auth middleware")
            .unwrap();
        (store, sid, dir)
    }

    #[test]
    fn recap_post_session_heuristic_returns_recap_shape() {
        // Happy path: kind=session, generator=heuristic. The store
        // round-trips the recap and the response payload mirrors the
        // F1.1 wire shape — schema_version=1, kind=session, generator
        // discriminator type=heuristic.
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (status, body) = do_v1_recap_post(&store, &req);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "session");
        assert_eq!(body["subject_id"], sid);
        assert_eq!(body["schema_version"], 1);
        assert_eq!(body["generator"]["type"], "heuristic");
        assert!(body["headline"].as_str().is_some_and(|h| !h.is_empty()));
    }

    #[test]
    fn recap_post_idempotent_when_force_false() {
        // Two POSTs with force=false against the same session must
        // return the same recap id — the F1.1 idempotency rule
        // surfaces through the route layer unchanged.
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (s1, b1) = do_v1_recap_post(&store, &req);
        let (s2, b2) = do_v1_recap_post(&store, &req);
        assert_eq!(s1, StatusCode::OK);
        assert_eq!(s2, StatusCode::OK);
        assert_eq!(
            b1["id"], b2["id"],
            "force=false must return the same recap; got: {b1:?} vs {b2:?}"
        );
    }

    #[test]
    fn recap_post_force_true_replaces_prior_recap() {
        // force=true must drop the prior row (same subject + same
        // last_message_id) and insert a fresh one with a new id.
        // Without the route's pre-delete, the unique index would
        // conflict and the second insert would 500.
        let (store, sid, _dir) = recap_route_fixture();
        let mut req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, b1) = do_v1_recap_post(&store, &req);
        req.force = true;
        let (s2, b2) = do_v1_recap_post(&store, &req);
        assert_eq!(s2, StatusCode::OK);
        assert_ne!(
            b1["id"], b2["id"],
            "force=true must produce a fresh recap id; got: {b1:?} vs {b2:?}"
        );
    }

    #[test]
    fn recap_post_unknown_kind_returns_400() {
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "diff_chain".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (status, body) = do_v1_recap_post(&store, &req);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"].as_str().is_some_and(|s| s.contains("diff_chain")),
            "error must echo the unsupported kind back; got: {body}"
        );
    }

    #[test]
    fn recap_post_llm_generator_returns_501() {
        // F1.2 deliberately rejects the LLM path with 501 so clients
        // can probe support without guessing. The LLM slice (its own
        // task) flips this to 200.
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "llm".to_string(),
            provider: Some("anthropic".to_string()),
            model: Some("claude-opus-4-7".to_string()),
        };
        let (status, body) = do_v1_recap_post(&store, &req);
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert!(
            body["error"].as_str().is_some_and(|s| s.contains("llm")),
            "error must mention the unsupported generator; got: {body}"
        );
    }

    #[test]
    fn recap_post_missing_session_returns_404() {
        let (store, _sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: "does-not-exist".to_string(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (status, body) = do_v1_recap_post(&store, &req);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body["error"].as_str().is_some_and(|s| s.contains("does-not-exist")));
    }

    #[test]
    fn recap_get_returns_stored_recap() {
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, body) = do_v1_recap_post(&store, &req);
        let id = body["id"].as_str().unwrap().to_string();
        let (status, fetched) = do_v1_recap_get(&store, &id);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fetched["id"], id);
    }

    #[test]
    fn recap_get_missing_id_returns_404() {
        let (store, _sid, _dir) = recap_route_fixture();
        let (status, body) = do_v1_recap_get(&store, "no-such-recap");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "recap not found");
    }

    #[test]
    fn recap_list_requires_kind_and_subject_id() {
        // Both are mandatory — F1.2 doesn't support cross-subject
        // browse. Pinning the explicit-required contract so a future
        // refactor doesn't silently broaden the surface.
        let (store, sid, _dir) = recap_route_fixture();
        let q_no_kind = RecapListQuery {
            kind: None,
            subject_id: Some(sid.clone()),
            limit: 20,
        };
        let (s1, b1) = do_v1_recap_list(&store, &q_no_kind);
        assert_eq!(s1, StatusCode::BAD_REQUEST);
        assert!(b1["error"].as_str().is_some_and(|s| s.contains("kind")));

        let q_no_sid = RecapListQuery {
            kind: Some("session".to_string()),
            subject_id: None,
            limit: 20,
        };
        let (s2, b2) = do_v1_recap_list(&store, &q_no_sid);
        assert_eq!(s2, StatusCode::BAD_REQUEST);
        assert!(b2["error"].as_str().is_some_and(|s| s.contains("subject_id")));
    }

    #[test]
    fn recap_list_returns_count_and_recaps_array() {
        let (store, sid, _dir) = recap_route_fixture();
        // Empty path first.
        let q = RecapListQuery {
            kind: Some("session".to_string()),
            subject_id: Some(sid.clone()),
            limit: 20,
        };
        let (s_empty, b_empty) = do_v1_recap_list(&store, &q);
        assert_eq!(s_empty, StatusCode::OK);
        assert_eq!(b_empty["count"], 0);
        assert!(b_empty["recaps"].as_array().is_some_and(|a| a.is_empty()));

        // Now one row.
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let _ = do_v1_recap_post(&store, &req);
        let (s_one, b_one) = do_v1_recap_list(&store, &q);
        assert_eq!(s_one, StatusCode::OK);
        assert_eq!(b_one["count"], 1);
        assert_eq!(b_one["recaps"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn recap_patch_marks_generator_user_edited() {
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, posted) = do_v1_recap_post(&store, &req);
        let id = posted["id"].as_str().unwrap().to_string();
        let patch = RecapPatch {
            headline: "Hand-written headline".to_string(),
            bullets: vec!["I edited this".to_string()],
            next_actions: vec!["Ship it".to_string()],
        };
        let (status, body) = do_v1_recap_patch(&store, &id, &patch);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["headline"], "Hand-written headline");
        assert_eq!(body["generator"]["type"], "user_edited");
        assert_eq!(
            body["bullets"].as_array().unwrap().len(),
            1,
            "bullets must reflect the patch; got: {}",
            body["bullets"]
        );
    }

    #[test]
    fn recap_patch_missing_id_returns_404() {
        let (store, _sid, _dir) = recap_route_fixture();
        let patch = RecapPatch {
            headline: "x".to_string(),
            bullets: vec![],
            next_actions: vec![],
        };
        let (status, body) =
            do_v1_recap_patch(&store, "no-such-recap", &patch);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "recap not found");
    }

    #[test]
    fn recap_delete_returns_204_and_removes_row() {
        let (store, sid, _dir) = recap_route_fixture();
        let req = RecapRequest {
            kind: "session".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, body) = do_v1_recap_post(&store, &req);
        let id = body["id"].as_str().unwrap().to_string();

        let (status, _) = do_v1_recap_delete(&store, &id);
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (after, _) = do_v1_recap_get(&store, &id);
        assert_eq!(after, StatusCode::NOT_FOUND);
    }

    #[test]
    fn recap_delete_idempotent_on_unknown_id() {
        // Pin: delete on a non-existent id is 204, not 404. Clients
        // shouldn't have to swallow a 404 just because they retried
        // (the design "Forget a recap" semantics).
        let (store, _sid, _dir) = recap_route_fixture();
        let (status, _) = do_v1_recap_delete(&store, "never-existed");
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    // ── J1.3: kind=job recap-route helper tests ────────────────────────────

    async fn job_route_fixture(
        task: &str,
    ) -> (Arc<crate::job_manager::JobManager>, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::job_manager::JobsDb::open_with(
            &dir.path().join("jobs.db"),
            [42u8; 32],
        )
        .unwrap();
        let mgr = Arc::new(crate::job_manager::JobManager::new_with(db));
        let sid = mgr
            .create(crate::job_manager::CreateJobReq {
                task: task.into(),
                provider: "mock".into(),
                approval: "auto".into(),
                workspace_root: "/tmp/ws".into(),
                priority: 5,
                webhook_url: None,
                tags: vec![],
                quota_bucket: None,
            })
            .await
            .unwrap();
        // Seed a couple of step events so the heuristic produces bullets.
        mgr.publish_event(
            &sid,
            crate::job_manager::AgentEventPayload::step(1, "shell", true),
        )
        .await;
        mgr.publish_event(
            &sid,
            crate::job_manager::AgentEventPayload::step(2, "edit", true),
        )
        .await;
        (mgr, sid, dir)
    }

    #[tokio::test]
    async fn recap_post_job_heuristic_returns_recap_shape() {
        let (mgr, sid, _dir) = job_route_fixture("Refactor SSRF guard").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (status, body) = do_v1_recap_post_job(&mgr, &req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "job");
        assert_eq!(body["subject_id"], sid);
        assert_eq!(body["schema_version"], 1);
        assert_eq!(body["generator"]["type"], "heuristic");
        assert!(body["headline"].as_str().unwrap().starts_with("Refactor"));
    }

    #[tokio::test]
    async fn recap_post_job_returns_404_on_missing_subject() {
        let (mgr, _sid, _dir) = job_route_fixture("Task").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: "no-such-job".into(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (status, _) = do_v1_recap_post_job(&mgr, &req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn recap_post_job_idempotent_on_repeat() {
        // J1.1's (subject_id, last_event_seq) upsert means a second POST
        // with no new events returns the existing row id unchanged.
        let (mgr, sid, _dir) = job_route_fixture("Same job").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (s1, b1) = do_v1_recap_post_job(&mgr, &req).await;
        let (s2, b2) = do_v1_recap_post_job(&mgr, &req).await;
        assert_eq!(s1, StatusCode::OK);
        assert_eq!(s2, StatusCode::OK);
        assert_eq!(
            b1["id"], b2["id"],
            "second POST must reuse the same recap row"
        );
    }

    #[tokio::test]
    async fn recap_post_job_force_drops_existing_and_regens() {
        // force=true must produce a different recap id even when
        // (subject, last_event_seq) is unchanged.
        let (mgr, sid, _dir) = job_route_fixture("Forced regen").await;
        let req_first = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, b1) = do_v1_recap_post_job(&mgr, &req_first).await;
        let req_force = RecapRequest {
            force: true,
            ..req_first
        };
        let (s2, b2) = do_v1_recap_post_job(&mgr, &req_force).await;
        assert_eq!(s2, StatusCode::OK);
        assert_ne!(b1["id"], b2["id"], "force=true must produce a new id");
    }

    #[tokio::test]
    async fn recap_post_job_rejects_llm_generator() {
        let (mgr, sid, _dir) = job_route_fixture("Task").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid,
            force: false,
            generator: "llm".to_string(),
            provider: None,
            model: None,
        };
        let (status, _) = do_v1_recap_post_job(&mgr, &req).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn recap_get_job_returns_stored_row() {
        let (mgr, sid, _dir) = job_route_fixture("Get me").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, b1) = do_v1_recap_post_job(&mgr, &req).await;
        let id = b1["id"].as_str().unwrap();
        let (status, b2) = do_v1_recap_get_job(&mgr, id).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(b1["id"], b2["id"]);
    }

    #[tokio::test]
    async fn recap_get_job_returns_404_on_missing_id() {
        let (mgr, _sid, _dir) = job_route_fixture("Task").await;
        let (status, _) = do_v1_recap_get_job(&mgr, "no-such-recap").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn recap_list_job_returns_recaps_for_subject() {
        let (mgr, sid, _dir) = job_route_fixture("List me").await;
        let post_req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid.clone(),
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let _ = do_v1_recap_post_job(&mgr, &post_req).await;
        let q = RecapListQuery {
            kind: Some("job".into()),
            subject_id: Some(sid.clone()),
            limit: 20,
        };
        let (status, body) = do_v1_recap_list_job(&mgr, &q).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["count"], 1);
        assert_eq!(body["recaps"][0]["subject_id"], sid);
    }

    #[tokio::test]
    async fn recap_list_job_400s_without_subject_id() {
        let (mgr, _sid, _dir) = job_route_fixture("Task").await;
        let q = RecapListQuery {
            kind: Some("job".into()),
            subject_id: None,
            limit: 20,
        };
        let (status, _) = do_v1_recap_list_job(&mgr, &q).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn recap_delete_job_removes_row() {
        let (mgr, sid, _dir) = job_route_fixture("Delete me").await;
        let req = RecapRequest {
            kind: "job".to_string(),
            subject_id: sid,
            force: false,
            generator: "heuristic".to_string(),
            provider: None,
            model: None,
        };
        let (_, b) = do_v1_recap_post_job(&mgr, &req).await;
        let id = b["id"].as_str().unwrap().to_string();
        let (status, _) = do_v1_recap_delete_job(&mgr, &id).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (after, _) = do_v1_recap_get_job(&mgr, &id).await;
        assert_eq!(after, StatusCode::NOT_FOUND);
    }

    // F1.2 HTTP auth tests live in `http_integration` (where `test_app`
    // is defined). Pure helper tests above don't need it.
}
