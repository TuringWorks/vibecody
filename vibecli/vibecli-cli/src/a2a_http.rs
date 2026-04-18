//! Real HTTP + SSE transport for the A2A protocol (US-002).
//!
//! The existing [`crate::a2a_protocol`] module carries the data model (agent
//! cards, task types, event stream). This module wraps that model with an
//! axum-based server and a reqwest-based client so VibeCody agents can discover
//! and delegate work over the wire.
//!
//! Server routes:
//! - `GET  /a2a/card`         → the local [`AgentCard`] as JSON
//! - `POST /a2a/tasks`        → submit a [`TaskInput`], returns `{"task_id": "…"}`
//! - `GET  /a2a/tasks/:id`    → current [`A2aTask`] state
//! - `GET  /a2a/events`       → SSE stream of [`A2aEvent`]s emitted since server start
//!
//! The server shares a `Arc<Mutex<A2aServer>>` with the rest of the agent so
//! in-process code (REPL, Tauri, worktree dispatcher) and remote peers both
//! see the same task queue + event stream.

use crate::a2a_protocol::{A2aEvent, A2aServer, A2aTask, AgentCard, TaskInput};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Sse, sse::Event as SseEvent},
    routing::{get, post},
};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};

type Shared = Arc<Mutex<A2aServer>>;

// ── Broadcast channel for SSE ───────────────────────────────────────────────

/// Runtime state attached to the HTTP layer: the shared agent + a live
/// broadcast channel for SSE subscribers. Events already buffered on
/// [`A2aServer::event_stream`] are replayed first; new events arrive via the
/// broadcast channel.
#[derive(Clone)]
struct HttpState {
    agent: Shared,
    bus: broadcast::Sender<A2aEvent>,
}

/// Handle returned from [`serve_agent`]; owns the bound address + abort handle.
#[derive(Debug)]
pub struct ServerHandle {
    pub addr: SocketAddr,
    pub _task: tokio::task::JoinHandle<()>,
}

impl ServerHandle {
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

/// Spawn an A2A HTTP server on an ephemeral port bound to the shared
/// [`A2aServer`] state. The caller keeps the returned handle alive for the
/// duration of the test or process.
pub async fn serve_agent(agent: Shared) -> Result<ServerHandle, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("bind failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("local_addr failed: {e}"))?;

    let (tx, _) = broadcast::channel::<A2aEvent>(256);
    let state = HttpState {
        agent,
        bus: tx.clone(),
    };

    let app = Router::new()
        .route("/a2a/card", get(get_card))
        .route("/a2a/tasks", post(submit_task))
        .route("/a2a/tasks/:id", get(get_task))
        .route("/a2a/events", get(sse_events))
        .with_state(state);

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(ServerHandle { addr, _task: task })
}

// ── Handlers ────────────────────────────────────────────────────────────────

async fn get_card(State(s): State<HttpState>) -> impl IntoResponse {
    let agent = s.agent.lock().await;
    (StatusCode::OK, Json(agent.agent_card.clone()))
}

async fn submit_task(
    State(s): State<HttpState>,
    Json(input): Json<TaskInput>,
) -> impl IntoResponse {
    let mut agent = s.agent.lock().await;
    match agent.submit_task(input) {
        Ok(id) => {
            // Replay the freshly recorded event to SSE subscribers.
            if let Some(latest) = agent.get_events().all_events().last() {
                let _ = s.bus.send(latest.clone());
            }
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"task_id": id})),
            )
                .into_response()
        }
        Err(e) => (StatusCode::CONFLICT, Json(serde_json::json!({"error": e})))
            .into_response(),
    }
}

async fn get_task(
    State(s): State<HttpState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent = s.agent.lock().await;
    match agent.active_tasks.iter().find(|t| t.id == id) {
        Some(task) => (StatusCode::OK, Json(task.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("task {id} not found")})),
        )
            .into_response(),
    }
}

async fn sse_events(
    State(s): State<HttpState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let snapshot: Vec<A2aEvent> = {
        let agent = s.agent.lock().await;
        agent.get_events().all_events().to_vec()
    };
    let live = tokio_stream::wrappers::BroadcastStream::new(s.bus.subscribe());
    let replay = futures::stream::iter(snapshot.into_iter().map(Ok::<_, broadcast::error::RecvError>));

    let merged = replay.chain(live.map(|r| r.map_err(|_| broadcast::error::RecvError::Closed)))
        .filter_map(|r| async move { r.ok() })
        .map(|ev| {
            let kind = format!("{:?}", ev.event_type);
            Ok(SseEvent::default()
                .event(kind)
                .json_data(ev)
                .unwrap_or_else(|_| SseEvent::default()))
        });

    Sse::new(merged).keep_alive(
        axum::response::sse::KeepAlive::new().interval(Duration::from_secs(30)),
    )
}

// ── Client ──────────────────────────────────────────────────────────────────

/// HTTP + SSE client for talking to an A2A peer.
pub struct A2aHttpClient {
    client: reqwest::Client,
}

impl A2aHttpClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// GET `/a2a/card` and deserialize the response.
    pub async fn fetch_card(&self, base_url: &str) -> Result<AgentCard, String> {
        let url = format!("{}/a2a/card", base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("fetch_card: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("fetch_card read: {e}"))?;
        if !status.is_success() {
            return Err(format!("fetch_card HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| format!("fetch_card parse: {e}"))
    }

    /// POST `/a2a/tasks` with a [`TaskInput`] body, returning the server-issued task id.
    pub async fn submit_task(
        &self,
        base_url: &str,
        input: TaskInput,
    ) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Resp {
            task_id: String,
        }
        let url = format!("{}/a2a/tasks", base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(url)
            .json(&input)
            .send()
            .await
            .map_err(|e| format!("submit_task: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("submit_task read: {e}"))?;
        if !status.is_success() {
            return Err(format!("submit_task HTTP {status}: {text}"));
        }
        let parsed: Resp =
            serde_json::from_str(&text).map_err(|e| format!("submit_task parse: {e}"))?;
        Ok(parsed.task_id)
    }

    /// GET `/a2a/tasks/:id` and return the current task snapshot.
    pub async fn get_task(&self, base_url: &str, id: &str) -> Result<A2aTask, String> {
        let url = format!(
            "{}/a2a/tasks/{}",
            base_url.trim_end_matches('/'),
            urlencoding::encode(id)
        );
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("get_task: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("get_task read: {e}"))?;
        if !status.is_success() {
            return Err(format!("get_task HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| format!("get_task parse: {e}"))
    }

    /// Stream at most `max_events` Server-Sent Events from `/a2a/events`.
    /// Returns a vec of `(event_name, data_json)` tuples.
    pub async fn read_events(
        &self,
        base_url: &str,
        max_events: usize,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!("{}/a2a/events", base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| format!("read_events: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("read_events HTTP {}", resp.status()));
        }
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut events: Vec<(String, String)> = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, stream.next()).await {
                Err(_) => break,
                Ok(None) => break,
                Ok(Some(Err(e))) => return Err(format!("sse chunk: {e}")),
                Ok(Some(Ok(chunk))) => {
                    buf.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(end) = buf.find("\n\n") {
                        let frame = buf[..end].to_string();
                        buf.drain(..end + 2);
                        let mut event_name = String::new();
                        let mut data = String::new();
                        for line in frame.lines() {
                            if let Some(rest) = line.strip_prefix("event:") {
                                event_name = rest.trim().to_string();
                            } else if let Some(rest) = line.strip_prefix("data:") {
                                if !data.is_empty() {
                                    data.push('\n');
                                }
                                data.push_str(rest.trim_start());
                            }
                        }
                        if !event_name.is_empty() {
                            events.push((event_name, data));
                            if events.len() >= max_events {
                                return Ok(events);
                            }
                        }
                    }
                }
            }
        }
        Ok(events)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a_protocol::{AgentCapability, TaskStatus};

    fn card() -> AgentCard {
        AgentCard::new("unit-bot", "t", "http://x", "1.0.0")
            .with_capabilities(vec![AgentCapability::CodeGeneration])
    }

    async fn spawn() -> (ServerHandle, Shared) {
        let agent = Arc::new(Mutex::new(A2aServer::new("127.0.0.1", 0, card())));
        let handle = serve_agent(agent.clone()).await.expect("serve");
        (handle, agent)
    }

    #[tokio::test]
    async fn fetch_card_returns_advertised_card() {
        let (handle, _agent) = spawn().await;
        let client = A2aHttpClient::new(reqwest::Client::new());
        let got = client.fetch_card(&handle.base_url()).await.expect("fetch");
        assert_eq!(got.name, "unit-bot");
        assert_eq!(got.capabilities.len(), 1);
    }

    #[tokio::test]
    async fn submit_task_returns_task_id_and_records_state() {
        let (handle, agent) = spawn().await;
        let client = A2aHttpClient::new(reqwest::Client::new());
        let id = client
            .submit_task(&handle.base_url(), TaskInput::text("hello"))
            .await
            .expect("submit");
        assert!(id.starts_with("srv-task-"));
        let guard = agent.lock().await;
        assert_eq!(guard.active_task_count(), 1);
    }

    #[tokio::test]
    async fn get_task_returns_submitted_status() {
        let (handle, _agent) = spawn().await;
        let client = A2aHttpClient::new(reqwest::Client::new());
        let id = client
            .submit_task(&handle.base_url(), TaskInput::text("x"))
            .await
            .unwrap();
        let task = client.get_task(&handle.base_url(), &id).await.expect("get");
        assert_eq!(task.status, TaskStatus::Submitted);
    }

    #[tokio::test]
    async fn get_task_404_for_missing_id() {
        let (handle, _agent) = spawn().await;
        let client = A2aHttpClient::new(reqwest::Client::new());
        let err = client
            .get_task(&handle.base_url(), "nope")
            .await
            .unwrap_err();
        assert!(err.contains("404"));
    }

    #[tokio::test]
    async fn sse_stream_replays_buffered_events() {
        let (handle, _agent) = spawn().await;
        let client = A2aHttpClient::new(reqwest::Client::new());
        // Submit first so an event exists before we subscribe.
        let _ = client
            .submit_task(&handle.base_url(), TaskInput::text("y"))
            .await
            .unwrap();
        let events = client
            .read_events(&handle.base_url(), 1)
            .await
            .expect("sse");
        assert!(!events.is_empty(), "expected at least one replayed event");
        assert_eq!(events[0].0, "TaskCreated");
    }
}
