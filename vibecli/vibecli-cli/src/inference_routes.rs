//! Ollama-compatible HTTP handlers (`/api/chat`, `/api/generate`, `/api/tags`,
//! `/api/pull`, `/api/show`).
//!
//! These mount alongside the existing daemon routes in [`crate::serve`].
//! The handlers are thin: each one parses the request, asks
//! [`crate::inference::Router`] for a backend, and either returns an NDJSON
//! body (streaming) or collapses the backend's stream into a single JSON
//! response (non-streaming, when the client opts out via `stream: false`).
//!
//! Backend selection precedence (highest first):
//!   1. `X-VibeCLI-Backend: mistralrs|ollama` request header
//!   2. `"backend": "mistralrs"` field in the JSON body
//!   3. Per-model pin (env or config — see [`crate::inference::router`])
//!   4. Daemon default
//!
//! Header beats body on purpose — it lets a thin debug client pin a backend
//! without rewriting the body the upstream service expects.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::stream::StreamExt;
use serde::Serialize;

use crate::inference::backend::{
    BackendError, BackendKind, ChatChunk, ChatMessage, ChatRequest, GenerateChunk,
    GenerateRequest, ModelInfo, PullProgress, PullRequest,
};
use crate::serve::ServeState;

const HEADER_BACKEND: &str = "x-vibecli-backend";

/// Resolve an explicit backend choice from header + body, with header winning.
fn override_kind(headers: &HeaderMap, body_kind: Option<BackendKind>) -> Option<BackendKind> {
    if let Some(v) = headers.get(HEADER_BACKEND) {
        if let Ok(s) = v.to_str() {
            return parse_kind(s).or(body_kind);
        }
    }
    body_kind
}

fn parse_kind(s: &str) -> Option<BackendKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mistralrs" => Some(BackendKind::Mistralrs),
        "ollama" => Some(BackendKind::Ollama),
        _ => None,
    }
}

/// Map [`BackendError`] to an HTTP status + JSON body.
fn err_response(e: BackendError) -> Response {
    let (status, msg) = match &e {
        BackendError::ModelNotFound(_) => (StatusCode::NOT_FOUND, e.to_string()),
        BackendError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, e.to_string()),
        BackendError::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()),
        BackendError::Upstream(_) | BackendError::Other(_) => {
            (StatusCode::BAD_GATEWAY, e.to_string())
        }
    };
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

/// Turn a `BoxStream<BackendResult<T>>` into an NDJSON response body.
/// Each frame becomes one JSON object + `\n`. Errors mid-stream are
/// flushed as `{"error": "..."}` frames so clients see them inline.
fn ndjson_response<T>(
    stream: futures::stream::BoxStream<
        'static,
        crate::inference::backend::BackendResult<T>,
    >,
) -> Response
where
    T: Serialize + Send + 'static,
{
    let bytes_stream = stream.map(|item| {
        let json = match item {
            Ok(frame) => serde_json::to_string(&frame)
                .unwrap_or_else(|e| format!(r#"{{"error":"serialize: {e}"}}"#)),
            Err(e) => serde_json::to_string(&serde_json::json!({"error": e.to_string()}))
                .unwrap_or_else(|_| String::from(r#"{"error":"serialize"}"#)),
        };
        Ok::<_, std::io::Error>(format!("{json}\n").into_bytes())
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(bytes_stream))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "stream build failed").into_response()
        })
}

// ---------------------------------------------------------------------------
// /api/chat
// ---------------------------------------------------------------------------

pub async fn chat(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(body): Json<ChatRequest>,
) -> Response {
    let router = match &state.inference_router {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "inference router not initialized"})),
            )
                .into_response();
        }
    };
    let backend = router.resolve(&body.model, override_kind(&headers, body.backend));
    let stream_flag = body.stream.unwrap_or(true);
    match backend.chat(body).await {
        Err(e) => err_response(e),
        Ok(stream) if stream_flag => ndjson_response::<ChatChunk>(stream),
        Ok(stream) => collapse_chat(stream).await,
    }
}

/// Drain a chat stream into a single Ollama-style aggregated response.
async fn collapse_chat(
    mut stream: futures::stream::BoxStream<
        'static,
        crate::inference::backend::BackendResult<ChatChunk>,
    >,
) -> Response {
    let mut combined = String::new();
    let mut last_model = String::new();
    let mut last_created_at = String::new();
    let mut done_reason: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                last_model = chunk.model;
                last_created_at = chunk.created_at;
                combined.push_str(&chunk.message.content);
                if chunk.done {
                    done_reason = chunk.done_reason;
                }
            }
            Err(e) => return err_response(e),
        }
    }
    let final_chunk = ChatChunk {
        model: last_model,
        created_at: last_created_at,
        message: ChatMessage {
            role: "assistant".into(),
            content: combined,
            images: None,
        },
        done: true,
        done_reason,
    };
    (StatusCode::OK, Json(final_chunk)).into_response()
}

// ---------------------------------------------------------------------------
// /api/generate
// ---------------------------------------------------------------------------

pub async fn generate(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(body): Json<GenerateRequest>,
) -> Response {
    let router = match &state.inference_router {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "inference router not initialized"})),
            )
                .into_response();
        }
    };
    let backend = router.resolve(&body.model, override_kind(&headers, body.backend));
    let stream_flag = body.stream.unwrap_or(true);
    match backend.generate(body).await {
        Err(e) => err_response(e),
        Ok(stream) if stream_flag => ndjson_response::<GenerateChunk>(stream),
        Ok(stream) => collapse_generate(stream).await,
    }
}

async fn collapse_generate(
    mut stream: futures::stream::BoxStream<
        'static,
        crate::inference::backend::BackendResult<GenerateChunk>,
    >,
) -> Response {
    let mut combined = String::new();
    let mut last_model = String::new();
    let mut last_created_at = String::new();
    let mut done_reason: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                last_model = chunk.model;
                last_created_at = chunk.created_at;
                combined.push_str(&chunk.response);
                if chunk.done {
                    done_reason = chunk.done_reason;
                }
            }
            Err(e) => return err_response(e),
        }
    }
    let final_chunk = GenerateChunk {
        model: last_model,
        created_at: last_created_at,
        response: combined,
        done: true,
        done_reason,
    };
    (StatusCode::OK, Json(final_chunk)).into_response()
}

// ---------------------------------------------------------------------------
// /api/tags
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

pub async fn tags(State(state): State<ServeState>, headers: HeaderMap) -> Response {
    let router = match &state.inference_router {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "inference router not initialized"})),
            )
                .into_response();
        }
    };
    // Allow scoping the listing via the `X-VibeCLI-Backend` header. Without
    // it we union both backends — that's the natural answer to "what
    // models can I use?" from a fresh client.
    let kinds: Vec<BackendKind> = match override_kind(&headers, None) {
        Some(k) => vec![k],
        None => vec![BackendKind::Ollama, BackendKind::Mistralrs],
    };
    let mut models: Vec<ModelInfo> = Vec::new();
    for kind in kinds {
        match router.by_kind(kind).list_models().await {
            Ok(list) => models.extend(list),
            // Don't fail the whole listing if one backend is down — the
            // user still cares about the other.
            Err(e) => tracing::warn!("/api/tags: {} backend failed: {e}", kind.as_str()),
        }
    }
    (StatusCode::OK, Json(TagsResponse { models })).into_response()
}

// ---------------------------------------------------------------------------
// /api/pull
// ---------------------------------------------------------------------------

pub async fn pull(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(body): Json<PullRequest>,
) -> Response {
    let router = match &state.inference_router {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "inference router not initialized"})),
            )
                .into_response();
        }
    };
    let backend = router.resolve(&body.name, override_kind(&headers, body.backend));
    let stream_flag = body.stream.unwrap_or(true);
    match backend.pull(body).await {
        Err(e) => err_response(e),
        Ok(stream) if stream_flag => ndjson_response::<PullProgress>(stream),
        Ok(stream) => collapse_pull(stream).await,
    }
}

async fn collapse_pull(
    mut stream: futures::stream::BoxStream<
        'static,
        crate::inference::backend::BackendResult<PullProgress>,
    >,
) -> Response {
    let mut last: Option<PullProgress> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(p) => last = Some(p),
            Err(e) => return err_response(e),
        }
    }
    match last {
        Some(p) => (StatusCode::OK, Json(p)).into_response(),
        None => (
            StatusCode::OK,
            Json(PullProgress {
                status: "no progress frames".into(),
                digest: None,
                total: None,
                completed: None,
            }),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// /api/show
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct ShowRequest {
    pub name: String,
    #[serde(default)]
    pub backend: Option<BackendKind>,
}

pub async fn show(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(body): Json<ShowRequest>,
) -> Response {
    let router = match &state.inference_router {
        Some(r) => r,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "inference router not initialized"})),
            )
                .into_response();
        }
    };
    let backend = router.resolve(&body.name, override_kind(&headers, body.backend));
    match backend.show(&body.name).await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => err_response(e),
    }
}

// ---------------------------------------------------------------------------
// Router builder — registered by `crate::serve::build_router`.
// ---------------------------------------------------------------------------

/// Build the `/api/*` route subtree. Caller wires the `ServeState` in via
/// `with_state(state)` — keeps state typing local to `serve.rs`.
pub fn build_routes() -> axum::Router<ServeState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/chat", post(chat))
        .route("/api/generate", post(generate))
        .route("/api/tags", get(tags))
        .route("/api/pull", post(pull))
        .route("/api/show", post(show))
}

