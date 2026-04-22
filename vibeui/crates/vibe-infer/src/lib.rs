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

/// Produces a fixed-dim L2-normalised embedding for a text input.
///
/// Implementations MUST return `vector.len() == self.dim()` for every call,
/// so callers can pre-allocate index storage.
#[async_trait]
pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Default batch implementation calls `embed` sequentially. Backends with
    /// real batching (candle GPU) should override.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t).await?);
        }
        Ok(out)
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
    fn dim(&self) -> usize {
        0
    }
    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        Err(InferenceError::BackendNotEnabled("candle"))
    }
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

    #[tokio::test]
    async fn stub_embedder_errors() {
        let r = StubBackend.embed("hello").await;
        assert!(matches!(r, Err(InferenceError::BackendNotEnabled("candle"))));
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
        assert!(matches!(r, Err(InferenceError::BackendNotEnabled("candle"))));
    }
}
