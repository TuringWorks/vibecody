//! Pure-Rust local inference for VibeCody.
//!
//! Two trait surfaces:
//! - [`Embedder`] — text → fixed-dim vector. Consumed by OpenMemory's
//!   [`compressed_hnsw`](../../../../vibecli/vibecli-cli/src/compressed_hnsw.rs)
//!   index when the user opts into a real embedding model instead of the
//!   built-in feature-hashing engine.
//! - [`TextGenerator`] — chat / completion. Long-term replacement for the
//!   `vllm`-orchestration in `inference_server.rs` when the user wants a
//!   process-local model rather than a sidecar HTTP server.
//!
//! ## Backend choice (candle vs vLLM sidecar)
//!
//! VibeCody already orchestrates external servers (vLLM, TGI, llama.cpp …)
//! via `vibecli/vibecli-cli/src/inference_server.rs`. That stays — it is the
//! right answer for big models on dedicated GPUs.
//!
//! `vibe-infer` covers the *in-process* case: small embedding models and
//! sub-3B chat models that ship inside the binary, run on CPU/Metal, and
//! need zero subprocess management. We pick **candle** because:
//!   - pure Rust, no C++/Python toolchain at build time,
//!   - tight workspace integration (shares `tokio` / `serde` / `tracing`),
//!   - Metal + CUDA backends without re-compiling the host crate.
//!
//! Candle is gated behind the `candle` feature so default builds (CI, Tauri
//! shell, mobile bridge) stay fast. Until that feature is enabled, calls
//! resolve to [`StubBackend`], which returns
//! [`InferenceError::BackendNotEnabled`].

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("backend `{0}` not enabled — rebuild with --features {0}")]
    BackendNotEnabled(&'static str),

    #[error("model `{0}` not found at {1}")]
    ModelNotFound(String, String),

    #[error("dimension mismatch: model produces {model}, caller expected {expected}")]
    DimensionMismatch { model: usize, expected: usize },

    #[error("backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, InferenceError>;

// ---------------------------------------------------------------------------
// Embedder
// ---------------------------------------------------------------------------

/// The embedding trait lives in [`vibe_embed`], not here.
///
/// There used to be two: this crate's `Embedder` (implemented only by the
/// candle backend) and a two-variant `EmbeddingProvider` enum inside the code
/// index. Nothing bridged them, so a local model and a cloud model were not
/// interchangeable anywhere. One trait now serves both, and
/// [`MiniLmEmbedder`](minilm::MiniLmEmbedder) is simply another implementor
/// alongside the HTTP backends.
pub use vibe_embed::{EmbedKind, Embedder, ModelRef, ProviderKind, SharedEmbedder};

/// Bridge this crate's error type into the shared one.
impl From<InferenceError> for vibe_embed::EmbeddingError {
    fn from(e: InferenceError) -> Self {
        match e {
            InferenceError::BackendNotEnabled(b) => Self::BackendNotEnabled(b),
            other => Self::Backend(other.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Text generation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub stop: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub text: String,
    pub tokens_generated: usize,
    /// Prompt-side token count. Populated when the underlying backend
    /// reports it (mistralrs always does); 0 when unknown. Surfaced via
    /// `ChatChunk` so HTTP routes (e.g. `/v1/messages`) can populate
    /// `usage.input_tokens` instead of returning 0 — Anthropic clients
    /// that gate on usage telemetry need a real number, not a sentinel.
    #[serde(default)]
    pub prompt_tokens: usize,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    Error,
}

/// Role tag on a chat turn. Maps 1:1 to OpenAI / Ollama wire roles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Multi-turn chat request — preserves message structure so backends can
/// apply the model's own chat template (Qwen ChatML, Llama-3 instruct,
/// etc.) instead of receiving a pre-flattened blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub stop: Vec<String>,
}

#[async_trait]
pub trait TextGenerator: Send + Sync {
    async fn generate(&self, req: GenerationRequest) -> Result<GenerationResponse>;

    /// Chat-aware generation. Default impl flattens to a single `prompt`
    /// (content-only join, no role prefix) so legacy backends keep working
    /// — but real implementations should override and pass each turn to
    /// the underlying engine so the model's chat template is applied per
    /// message. The flatten default is a correctness fallback, not a
    /// quality target: stuffing a multi-turn conversation through a
    /// single-prompt API loses the role boundaries the template needs.
    async fn generate_chat(&self, req: ChatRequest) -> Result<GenerationResponse> {
        let prompt = req
            .messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        self.generate(GenerationRequest {
            prompt,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stop: req.stop,
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// Stub backend (default — no ML deps)
// ---------------------------------------------------------------------------

/// Zero-cost placeholder used when no inference feature is enabled. Every
/// call returns [`InferenceError::BackendNotEnabled`] so callers fail loudly
/// instead of silently degrading to fake outputs.
pub struct StubBackend;

#[async_trait]
impl Embedder for StubBackend {
    fn model(&self) -> &ModelRef {
        // A stub still has to answer "which model are you?" — it names the
        // model it *would* run, so an index header built against it is not
        // mislabelled if the backend is later compiled in.
        static MODEL: std::sync::OnceLock<ModelRef> = std::sync::OnceLock::new();
        MODEL.get_or_init(|| ModelRef::new(ProviderKind::Local, minilm_model_id()))
    }

    fn dim(&self) -> Option<usize> {
        None
    }

    async fn embed_batch(
        &self,
        _texts: &[String],
        _kind: EmbedKind,
    ) -> vibe_embed::Result<Vec<Vec<f32>>> {
        Err(vibe_embed::EmbeddingError::BackendNotEnabled("candle"))
    }
}

/// Catalog id of the in-process model. Shared by the stub and the real
/// backend so both report the same identity.
pub const fn minilm_model_id() -> &'static str {
    "all-MiniLM-L6-v2"
}

#[async_trait]
impl TextGenerator for StubBackend {
    async fn generate(&self, _req: GenerationRequest) -> Result<GenerationResponse> {
        Err(InferenceError::BackendNotEnabled("candle"))
    }
}

// ---------------------------------------------------------------------------
// Candle backend (gated — only compiled with --features candle)
// ---------------------------------------------------------------------------

#[cfg(feature = "candle")]
pub mod minilm;

#[cfg(feature = "mistralrs")]
pub mod mistral;

pub mod kv_cache;
pub mod kv_cache_tq;

#[cfg(feature = "mistralrs")]
pub mod kv_cache_codec;

#[cfg(test)]
mod tests {
    use super::*;

    /// The stub must still name the model it stands in for, so an index
    /// header built in a non-candle build is not mislabelled.
    #[test]
    fn stub_reports_the_local_model_identity() {
        assert_eq!(StubBackend.model().provider, ProviderKind::Local);
        assert_eq!(StubBackend.model().model, minilm_model_id());
        assert_eq!(StubBackend.dim(), None);
    }

    #[tokio::test]
    async fn stub_embedder_errors() {
        let r = StubBackend.embed("hello", EmbedKind::Document).await;
        assert!(matches!(
            r,
            Err(vibe_embed::EmbeddingError::BackendNotEnabled("candle"))
        ));
    }

    #[tokio::test]
    async fn stub_generator_errors() {
        let r = StubBackend
            .generate(GenerationRequest {
                prompt: "hi".into(),
                max_tokens: 16,
                temperature: 0.0,
                stop: vec![],
            })
            .await;
        assert!(matches!(
            r,
            Err(InferenceError::BackendNotEnabled("candle"))
        ));
    }
}
