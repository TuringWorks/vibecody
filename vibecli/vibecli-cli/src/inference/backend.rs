//! The [`Backend`] trait every inference engine implements, plus the wire
//! types that mirror Ollama's HTTP API closely enough to serialize
//! round-trip without custom adapters.
//!
//! ## Why mirror Ollama's schema?
//!
//! The daemon's one external contract is "speak Ollama". Any deserialization
//! layer between the HTTP route and the backend would have to round-trip
//! through a neutral type anyway, so these types *are* the neutral type.
//! Ollama's schema is the neutral schema.

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

/// Enum tag naming each implementation. Used by [`Router`] for the
/// per-request override path and by [`ModelInfo::backend`] so clients see
/// where a model lives in `/api/tags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    /// Reverse-proxy to `ollama serve`. Model management (pull/delete/show)
    /// is delegated to the ollama daemon.
    Ollama,
    /// In-process `vibe-infer::MistralGenerator` with optional TurboQuant
    /// KV-cache codec. Pulls from Hugging Face via hf-hub.
    Mistralrs,
}

impl BackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Mistralrs => "mistralrs",
        }
    }
}

// ---------------------------------------------------------------------------
// Request / response wire types (Ollama-compatible)
// ---------------------------------------------------------------------------

/// One message in a `/api/chat` conversation. Matches Ollama's schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

/// `/api/chat` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    /// VibeCLI extension: lets the client pin a backend per request. Not
    /// part of upstream Ollama's schema — ignored by ollama itself.
    #[serde(default)]
    pub backend: Option<BackendKind>,
}

/// One NDJSON frame streamed back from `/api/chat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub model: String,
    pub created_at: String,
    pub message: ChatMessage,
    #[serde(default)]
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,
    /// Token counts. Populated on the final (`done: true`) frame when the
    /// backend reports them — mistralrs always does, ollama-proxy passes
    /// through upstream's `prompt_eval_count`/`eval_count` automatically
    /// thanks to the rename below.
    ///
    /// Wire serialization uses Ollama's field names (`prompt_eval_count`,
    /// `eval_count`) for compatibility with `/api/chat` clients; the
    /// `prompt_tokens`/`completion_tokens` aliases keep the deserializer
    /// accepting OpenAI/Anthropic-shaped counts as well. Routes that need
    /// to expose `usage` (notably `/v1/messages`) read these fields by
    /// their Rust names regardless of how the wire was tagged.
    #[serde(
        default,
        rename = "prompt_eval_count",
        alias = "prompt_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_tokens: Option<u32>,
    #[serde(
        default,
        rename = "eval_count",
        alias = "completion_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub completion_tokens: Option<u32>,
}

/// `/api/generate` request body (raw prompt, no chat template).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    #[serde(default)]
    pub backend: Option<BackendKind>,
}

/// One NDJSON frame streamed back from `/api/generate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateChunk {
    pub model: String,
    pub created_at: String,
    pub response: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,
}

/// `/api/tags` entry. One per loaded/cached model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    /// ISO-8601 timestamp of when the blob was last touched.
    pub modified_at: String,
    pub size: u64,
    /// VibeCLI extension — tells the client which backend serves this model.
    pub backend: BackendKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// `/api/pull` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// Ollama tag (`qwen2.5:0.5b`) or HF repo id (`Qwen/Qwen2.5-0.5B-Instruct`).
    pub name: String,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub backend: Option<BackendKind>,
}

/// One NDJSON frame streamed back during `/api/pull`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

// ---------------------------------------------------------------------------
// Error type — one shape across both backends so the HTTP layer can map
// cleanly to status codes without knowing which backend failed.
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type BackendResult<T> = std::result::Result<T, BackendError>;

// ---------------------------------------------------------------------------
// The trait
// ---------------------------------------------------------------------------

/// One pluggable inference engine. Implementations must be `Send + Sync` and
/// cheap to clone via `Arc` — the router holds them for the daemon's lifetime.
///
/// All streaming methods return a [`BoxStream`] of NDJSON frames. Non-streaming
/// requests are handled by the HTTP layer collapsing the stream into a single
/// response before serializing — backends always produce streams.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Which kind this is — used by the router for tagging and by
    /// `list_models` to populate `ModelInfo::backend`.
    fn kind(&self) -> BackendKind;

    /// Streaming `/api/chat`. Each item is one Ollama NDJSON frame.
    async fn chat(
        &self,
        req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>>;

    /// Streaming `/api/generate`. Each item is one Ollama NDJSON frame.
    async fn generate(
        &self,
        req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>>;

    /// `/api/tags` — list models this backend knows about. May include
    /// cached-but-not-loaded entries.
    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>>;

    /// `/api/pull` — download a model. Streams progress events.
    /// Backends that cannot pull (e.g. mistralrs lazy-loads via hf-hub on
    /// first use) may return a single "status: already cached" frame or
    /// `BackendError::InvalidRequest`.
    async fn pull(
        &self,
        req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>>;

    /// `/api/show` — metadata for a single model. `BackendError::ModelNotFound`
    /// if the backend doesn't know the model.
    async fn show(&self, name: &str) -> BackendResult<ModelInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_chunk() -> ChatChunk {
        ChatChunk {
            model: "m".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            message: ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                images: None,
            },
            done: true,
            done_reason: None,
            prompt_tokens: None,
            completion_tokens: None,
        }
    }

    #[test]
    fn chat_chunk_serializes_using_ollama_field_names() {
        // mistralrs / ollama-proxy populate the done frame with token counts.
        // /api/chat clients expect Ollama's wire spelling
        // (prompt_eval_count / eval_count). Confirm the rename is wired.
        let mut chunk = empty_chunk();
        chunk.prompt_tokens = Some(50);
        chunk.completion_tokens = Some(12);
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(
            json.contains("\"prompt_eval_count\":50"),
            "expected Ollama-shaped prompt_eval_count, got {json}"
        );
        assert!(
            json.contains("\"eval_count\":12"),
            "expected Ollama-shaped eval_count, got {json}"
        );
        // OpenAI/Anthropic-shaped names are deserialize-only aliases —
        // they must not appear in the serialized output.
        assert!(!json.contains("prompt_tokens"));
        assert!(!json.contains("completion_tokens"));
    }

    #[test]
    fn chat_chunk_deserializes_ollama_field_names() {
        // ollama-proxy yields raw upstream NDJSON. Confirm we pick up
        // the Ollama-shaped names without translation.
        let raw = r#"{
            "model":"m","created_at":"2026-01-01T00:00:00Z",
            "message":{"role":"assistant","content":""},
            "done":true,"prompt_eval_count":50,"eval_count":12
        }"#;
        let chunk: ChatChunk = serde_json::from_str(raw).unwrap();
        assert_eq!(chunk.prompt_tokens, Some(50));
        assert_eq!(chunk.completion_tokens, Some(12));
    }

    #[test]
    fn chat_chunk_deserializes_openai_aliases() {
        // Backwards-compat with the pre-rename shape: anything that
        // serialized prompt_tokens/completion_tokens (e.g. clients we
        // shipped before this change) still round-trips into a chunk
        // via the alias.
        let raw = r#"{
            "model":"m","created_at":"2026-01-01T00:00:00Z",
            "message":{"role":"assistant","content":""},
            "done":true,"prompt_tokens":7,"completion_tokens":3
        }"#;
        let chunk: ChatChunk = serde_json::from_str(raw).unwrap();
        assert_eq!(chunk.prompt_tokens, Some(7));
        assert_eq!(chunk.completion_tokens, Some(3));
    }

    #[test]
    fn chat_chunk_omits_token_fields_when_none() {
        // Streaming content frames send None for both. The skip_serializing_if
        // keeps the wire shape clean — no useless null/0 fields.
        let chunk = empty_chunk();
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(!json.contains("prompt_eval_count"));
        assert!(!json.contains("eval_count"));
    }
}
