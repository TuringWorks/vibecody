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
use crate::inference::backend_override::override_kind;
use crate::serve::ServeState;

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

    // Forward `max_tokens` into the Ollama-shape options blob so
    // `inference::mistralrs::sampler_from_options` and
    // `inference::ollama::OllamaProxyBackend` both pick it up via the
    // existing `num_predict` key. Without this the Anthropic spec field
    // is silently dropped, which surprises clients that rely on it as
    // an output cap (Copilot review comment #2 on PR #8).
    let options = req.max_tokens.map(|n| {
        serde_json::json!({ "num_predict": n })
    });

    let chat_req = ChatRequest {
        model: req.model.clone(),
        messages: chat_messages,
        stream: Some(false),
        options,
        backend: req.backend,
    };

    // Total input chars across system + every message content. Used as
    // the denominator in the char-based fallback estimate when the
    // backend doesn't report `prompt_tokens` itself.
    let input_chars: usize = chat_req
        .messages
        .iter()
        .map(|m| m.content.chars().count())
        .sum();

    let backend = router.resolve(&chat_req.model, override_kind(&headers, chat_req.backend));
    let mut stream = match backend.chat(chat_req).await {
        Ok(s) => s,
        Err(e) => return err_response(e),
    };

    let mut combined = String::new();
    let mut last_model = req.model;
    let mut done_reason: Option<String> = None;
    let mut backend_prompt_tokens: Option<u32> = None;
    let mut backend_completion_tokens: Option<u32> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                last_model = chunk.model;
                combined.push_str(&chunk.message.content);
                if chunk.done {
                    done_reason = chunk.done_reason;
                    backend_prompt_tokens = chunk.prompt_tokens;
                    backend_completion_tokens = chunk.completion_tokens;
                }
            }
            Err(e) => return err_response(e),
        }
    }

    let stop_reason = done_reason.unwrap_or_else(|| "end_turn".to_string());
    let usage = Usage::resolve(
        backend_prompt_tokens,
        backend_completion_tokens,
        input_chars,
        combined.chars().count(),
    );
    let resp = MessagesResponse {
        id: format!("msg_{}", Uuid::new_v4().simple()),
        kind: "message",
        role: "assistant",
        content: vec![ContentBlockOut::Text { text: combined }],
        model: last_model,
        stop_reason,
        stop_sequence: None,
        usage,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

impl Usage {
    /// Build a `usage` block for the response, preferring real backend
    /// counts when present (mistralrs always reports them) and falling
    /// back to a coarse char/4 estimate when the backend doesn't.
    ///
    /// Rationale (Copilot review comment #3 on PR #8): the Anthropic
    /// Messages spec marks `usage` as required and downstream clients
    /// gate budgeting / telemetry on it. Returning `0`s is worse than a
    /// rough estimate because callers can't distinguish "we don't know"
    /// from "the prompt was empty." chars/4 is the standard heuristic
    /// for English; non-Latin scripts will under-count, but a finger-in-
    /// the-air number is better than a misleading zero.
    fn resolve(
        backend_prompt_tokens: Option<u32>,
        backend_completion_tokens: Option<u32>,
        input_chars: usize,
        output_chars: usize,
    ) -> Self {
        fn estimate(chars: usize) -> u32 {
            // Round up so a 1-char input doesn't report 0 tokens.
            let est = (chars as u64 + 3) / 4;
            u32::try_from(est).unwrap_or(u32::MAX)
        }
        Usage {
            input_tokens: backend_prompt_tokens.unwrap_or_else(|| estimate(input_chars)),
            output_tokens: backend_completion_tokens.unwrap_or_else(|| estimate(output_chars)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_prefers_backend_counts_when_present() {
        let u = Usage::resolve(Some(123), Some(45), 999_999, 999_999);
        assert_eq!(u.input_tokens, 123);
        assert_eq!(u.output_tokens, 45);
    }

    #[test]
    fn usage_falls_back_to_char_estimate_when_backend_silent() {
        // 16 chars / 4 = 4 tokens; 8 chars / 4 = 2 tokens.
        let u = Usage::resolve(None, None, 16, 8);
        assert_eq!(u.input_tokens, 4);
        assert_eq!(u.output_tokens, 2);
    }

    #[test]
    fn usage_estimate_rounds_up_so_short_inputs_arent_zero() {
        // A 1-character input would round down to 0 tokens with a naive
        // /4 — round up so callers always see at least 1 token.
        let u = Usage::resolve(None, None, 1, 1);
        assert_eq!(u.input_tokens, 1);
        assert_eq!(u.output_tokens, 1);
    }

    #[test]
    fn usage_mixed_backend_count_and_estimate() {
        // Backend gave us prompt_tokens but not completion_tokens —
        // mix and match per side.
        let u = Usage::resolve(Some(50), None, 999, 100);
        assert_eq!(u.input_tokens, 50);
        assert_eq!(u.output_tokens, 25); // 100/4
    }
}
