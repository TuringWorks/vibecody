//! # fluxo-server
//!
//! An Axum HTTP API over a [`fluxo_engine::Engine`]:
//!
//! | Method & path | Purpose |
//! |---|---|
//! | `POST /workflow` | Register a workflow definition |
//! | `GET  /workflow` | List `(name, version)` pairs |
//! | `GET  /workflow/{name}?version=` | Fetch a definition (latest unless pinned) |
//! | `POST /workflow/{name}/execute` | Start a run |
//! | `GET  /workflow/run/{id}` | Fetch a run |
//! | `POST /workflow/run/{id}/{pause,resume,terminate,signal}` | Control a run |
//! | `GET  /workflow/run/{id}/stream` | SSE timeline of the run |
//! | `GET  /runs?status=` | List runs |
//! | `GET  /tasks/poll/{task_type}?workerId=` | Claim a worker task |
//! | `POST /tasks/{task_id}/{complete,fail}` | Report a task result |
//!
//! The router is generic over the [`Store`] backend, so the same API serves SQLite locally
//! or Postgres at scale.

#![forbid(unsafe_code)]

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use fluxo_core::run::TaskStatus;
use fluxo_core::WorkflowDef;
use fluxo_engine::{Engine, EngineError};
use fluxo_store::{Store, StoreError};
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Build the API router over a shared engine.
pub fn router<S: Store + 'static>(engine: Arc<Engine<S>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/workflow", post(register::<S>).get(list_defs::<S>))
        .route("/workflow/{name}", get(get_def::<S>))
        .route("/workflow/{name}/execute", post(execute::<S>))
        .route("/workflow/run/{id}", get(get_run::<S>))
        .route("/workflow/run/{id}/pause", post(pause::<S>))
        .route("/workflow/run/{id}/resume", post(resume::<S>))
        .route("/workflow/run/{id}/terminate", post(terminate::<S>))
        .route("/workflow/run/{id}/signal", post(signal::<S>))
        .route("/workflow/run/{id}/stream", get(stream_run::<S>))
        .route("/runs", get(list_runs::<S>))
        .route("/tasks/poll/{task_type}", get(poll_task::<S>))
        .route("/tasks/{task_id}/complete", post(complete_task::<S>))
        .route("/tasks/{task_id}/fail", post(fail_task::<S>))
        .with_state(engine)
}

/// Bind `addr` and serve the API until the process exits.
pub async fn serve<S: Store + 'static>(engine: Engine<S>, addr: SocketAddr) -> std::io::Result<()> {
    let app = router(Arc::new(engine));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// An HTTP-shaped error with a status code and message.
struct AppError(StatusCode, String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<EngineError> for AppError {
    fn from(e: EngineError) -> Self {
        let code = match &e {
            EngineError::NotFound(_) => StatusCode::NOT_FOUND,
            EngineError::Invalid(_) => StatusCode::BAD_REQUEST,
            EngineError::Core(fluxo_core::FluxoError::InvalidDefinition(_))
            | EngineError::Core(fluxo_core::FluxoError::UnsupportedTaskType(_)) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        AppError(code, e.to_string())
    }
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        let code = match &e {
            StoreError::NotFound(_) => StatusCode::NOT_FOUND,
            StoreError::Conflict(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        AppError(code, e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Request / response DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteRequest {
    #[serde(default)]
    input: Value,
    version: Option<u32>,
    correlation_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteResponse {
    workflow_id: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    name: String,
    version: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollQuery {
    worker_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteRequest {
    workflow_id: String,
    #[serde(default)]
    output: Value,
    status: Option<TaskStatus>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FailRequest {
    workflow_id: String,
    #[serde(default)]
    output: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalRequest {
    reference_name: String,
    #[serde(default)]
    output: Value,
}

#[derive(Deserialize)]
struct TerminateRequest {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct StatusQuery {
    status: Option<fluxo_core::run::WorkflowStatus>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "fluxo-server" }))
}

async fn register<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Json(def): Json<WorkflowDef>,
) -> std::result::Result<Json<RegisterResponse>, AppError> {
    engine.register(&def).await?;
    Ok(Json(RegisterResponse { name: def.name, version: def.version }))
}

async fn list_defs<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
) -> std::result::Result<Json<Value>, AppError> {
    let defs = engine.store().list_workflow_defs().await?;
    let items: Vec<Value> = defs
        .into_iter()
        .map(|(name, version)| json!({ "name": name, "version": version }))
        .collect();
    Ok(Json(json!({ "workflows": items })))
}

async fn get_def<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(name): Path<String>,
    Query(q): Query<VersionQuery>,
) -> std::result::Result<Json<WorkflowDef>, AppError> {
    engine
        .store()
        .get_workflow_def(&name, q.version)
        .await?
        .map(Json)
        .ok_or_else(|| AppError(StatusCode::NOT_FOUND, format!("workflow '{}' not found", name)))
}

async fn execute<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(name): Path<String>,
    Json(req): Json<ExecuteRequest>,
) -> std::result::Result<Json<ExecuteResponse>, AppError> {
    let id = engine.start(&name, req.version, req.input, req.correlation_id).await?;
    Ok(Json(ExecuteResponse { workflow_id: id }))
}

async fn get_run<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
) -> std::result::Result<Json<fluxo_core::WorkflowRun>, AppError> {
    Ok(Json(engine.get_run(&id).await?))
}

async fn list_runs<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Query(q): Query<StatusQuery>,
) -> std::result::Result<Json<Value>, AppError> {
    let runs = engine.store().list_runs(q.status).await?;
    Ok(Json(json!({ "runs": runs })))
}

async fn pause<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
) -> std::result::Result<StatusCode, AppError> {
    engine.pause(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resume<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
) -> std::result::Result<StatusCode, AppError> {
    engine.resume(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn terminate<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
    Json(req): Json<TerminateRequest>,
) -> std::result::Result<StatusCode, AppError> {
    engine.terminate(&id, req.reason).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn signal<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
    Json(req): Json<SignalRequest>,
) -> std::result::Result<StatusCode, AppError> {
    engine.signal(&id, &req.reference_name, req.output).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn poll_task<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(task_type): Path<String>,
    Query(q): Query<PollQuery>,
) -> std::result::Result<Response, AppError> {
    let worker = q.worker_id.unwrap_or_else(|| "anonymous".to_string());
    match engine.poll(&task_type, &worker).await? {
        Some(polled) => Ok(Json(json!({
            "workflowId": polled.workflow_id,
            "task": polled.task,
        }))
        .into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

async fn complete_task<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(task_id): Path<String>,
    Json(req): Json<CompleteRequest>,
) -> std::result::Result<StatusCode, AppError> {
    let status = req.status.unwrap_or(TaskStatus::Completed);
    engine.complete_task(&req.workflow_id, &task_id, status, req.output).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fail_task<S: Store>(
    State(engine): State<Arc<Engine<S>>>,
    Path(task_id): Path<String>,
    Json(req): Json<FailRequest>,
) -> std::result::Result<StatusCode, AppError> {
    engine
        .complete_task(&req.workflow_id, &task_id, TaskStatus::Failed, req.output)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// SSE timeline: emits a run snapshot whenever it changes, then closes when terminal.
///
/// v1 polls the store (~400ms); it upgrades to push once the engine exposes a transition
/// broadcast channel.
async fn stream_run<S: Store + 'static>(
    State(engine): State<Arc<Engine<S>>>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    struct StreamState<S: Store> {
        engine: Arc<Engine<S>>,
        id: String,
        last_updated: i64,
        first: bool,
        done: bool,
    }

    let init = StreamState { engine, id, last_updated: i64::MIN, first: true, done: false };

    let stream = futures::stream::unfold(init, |mut st| async move {
        if st.done {
            return None;
        }
        loop {
            match st.engine.get_run(&st.id).await {
                Ok(run) => {
                    if st.first || run.updated_at != st.last_updated {
                        st.first = false;
                        st.last_updated = run.updated_at;
                        if run.status.is_terminal() {
                            st.done = true;
                        }
                        let payload = json!({
                            "workflowId": run.workflow_id,
                            "status": run.status,
                            "output": run.output,
                            "tasks": run.tasks.iter().map(|t| json!({
                                "taskId": t.task_id,
                                "ref": t.reference_name,
                                "type": t.task_type,
                                "status": t.status,
                            })).collect::<Vec<_>>(),
                        });
                        let event = Event::default()
                            .event("run")
                            .json_data(payload)
                            .unwrap_or_else(|_| Event::default().data("{}"));
                        return Some((Ok(event), st));
                    }
                }
                Err(_) => {
                    st.done = true;
                    let event = Event::default().event("error").data("run not found");
                    return Some((Ok(event), st));
                }
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
