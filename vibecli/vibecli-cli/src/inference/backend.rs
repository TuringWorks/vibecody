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
