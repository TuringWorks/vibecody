//! Anthropic Messages API-compatible POST `/v1/messages` route.
//!
//! Mirror of Ollama 0.22.x's `/v1/messages` surface so clients written
//! against the Anthropic Messages API (notably Claude Code's gateway and
//! Bedrock paths) can drive the daemon's local + remote backends through
//! the same dispatch layer that powers `/api/chat`.
//!
//! Request shape: a minimal subset of the Anthropic spec — `model`,
//! `messages`, optional `system`, `max_tokens`, `stream`. Content blocks
//! support `string` shorthand and the `text` block type. Vision, tool
//! use, tool result, and document blocks are out of scope for this slice
//! and would surface as a 400 via serde rejection.
//!
//! Streaming is intentionally rejected with 501 — Anthropic uses an SSE
//! event stream (`message_start`, `content_block_start`,
//! `content_block_delta`, `content_block_stop`, `message_stop`) distinct
//! from `/api/chat`'s NDJSON, and folding the two is a separate
//! workstream. Non-streaming clients (`stream: false` or omitted) get
//! the full response in one Anthropic-shape JSON body.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::inference::backend::{BackendError, BackendKind, ChatMessage, ChatRequest};
use crate::serve::ServeState;

const HEADER_BACKEND: &str = "x-vibecli-backend";

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesRequest {
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub system: Option<String>,
    pub messages: Vec<MessageInput>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub backend: Option<BackendKind>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageInput {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    String(String),
    Blocks(Vec<ContentBlockIn>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockIn {
    Text { text: String },
}

impl MessageContent {
    fn flatten(self) -> String {
        match self {
            MessageContent::String(s) => s,
            MessageContent::Blocks(blocks) => blocks
                .into_iter()
                .map(|b| match b {
                    ContentBlockIn::Text { text } => text,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub role: &'static str,
    pub content: Vec<ContentBlockOut>,
    pub model: String,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockOut {
    Text { text: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

fn parse_kind(s: &str) -> Option<BackendKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mistralrs" => Some(BackendKind::Mistralrs),
        "ollama" => Some(BackendKind::Ollama),
        _ => None,
    }
}

fn override_kind(headers: &HeaderMap, body_kind: Option<BackendKind>) -> Option<BackendKind> {
    if let Some(v) = headers.get(HEADER_BACKEND) {
        if let Ok(s) = v.to_str() {
            return parse_kind(s).or(body_kind);
        }
    }
    body_kind
}

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

pub async fn messages(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(req): Json<MessagesRequest>,
) -> Response {
    if req.stream.unwrap_or(false) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "/v1/messages streaming not yet implemented; pass stream: false"
            })),
        )
            .into_response();
    }

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

    let mut chat_messages: Vec<ChatMessage> = Vec::new();
    if let Some(sys) = req.system {
        chat_messages.push(ChatMessage {
            role: "system".to_string(),
            content: sys,
            images: None,
        });
    }
    for m in req.messages {
        chat_messages.push(ChatMessage {
            role: m.role,
            content: m.content.flatten(),
            images: None,
        });
    }

    let chat_req = ChatRequest {
        model: req.model.clone(),
        messages: chat_messages,
        stream: Some(false),
        options: None,
        backend: req.backend,
    };

    let backend = router.resolve(&chat_req.model, override_kind(&headers, chat_req.backend));
    let mut stream = match backend.chat(chat_req).await {
        Ok(s) => s,
        Err(e) => return err_response(e),
    };

    let mut combined = String::new();
    let mut last_model = req.model;
    let mut done_reason: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                last_model = chunk.model;
                combined.push_str(&chunk.message.content);
                if chunk.done {
                    done_reason = chunk.done_reason;
                }
            }
            Err(e) => return err_response(e),
        }
    }

    let stop_reason = done_reason.unwrap_or_else(|| "end_turn".to_string());
    let resp = MessagesResponse {
        id: format!("msg_{}", Uuid::new_v4().simple()),
        kind: "message",
        role: "assistant",
        content: vec![ContentBlockOut::Text { text: combined }],
        model: last_model,
        stop_reason,
        stop_sequence: None,
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    };
    (StatusCode::OK, Json(resp)).into_response()
}
