//! In-process Mistral.rs backend — TurboQuant-aware local inference via
//! `vibe-infer::MistralGenerator`.
//!
//! ## Build mode
//!
//! Real implementation is gated behind vibecli's `vibe-mistralrs` feature
//! (which turns on `vibe-infer/mistralrs`). Without that feature, every
//! method returns [`BackendError::Unavailable`] with a recompile hint —
//! the daemon still builds and the ollama backend still works.
//!
//! ## Lifecycle
//!
//! Generators are loaded lazily on first request and cached by model id in
//! a `RwLock<HashMap<String, Arc<MistralGenerator>>>`. Loading triggers
//! hf-hub download into `~/.cache/huggingface/hub` (mistral.rs's default).
//! KV-cache mode is read from `VIBE_INFER_KV_CACHE` (see
//! [`vibe_infer::mistral::KvCacheMode::from_env`]) — set to `turboquant` to
//! exercise the native CUDA / Metal codec.
//!
//! ## Streaming
//!
//! `vibe_infer::TextGenerator` exposes a unary `generate(req) -> response`
//! API. We synthesize a 2-frame Ollama NDJSON stream from the unary result
//! (one content frame, one `done: true` frame). Real per-token streaming
//! requires going around the `TextGenerator` trait into mistralrs's native
//! streaming surface — deferred to a follow-up.

use async_trait::async_trait;
use futures::stream::BoxStream;
#[cfg(feature = "vibe-mistralrs")]
use futures::stream;

#[cfg(feature = "vibe-mistralrs")]
use super::backend::ChatMessage;
use super::backend::{
    Backend, BackendError, BackendKind, BackendResult, ChatChunk, ChatRequest,
    GenerateChunk, GenerateRequest, ModelInfo, PullProgress, PullRequest,
};

#[cfg(feature = "vibe-mistralrs")]
use std::collections::HashMap;
#[cfg(feature = "vibe-mistralrs")]
use std::sync::Arc;
#[cfg(feature = "vibe-mistralrs")]
use tokio::sync::RwLock;
#[cfg(feature = "vibe-mistralrs")]
use vibe_infer::{
    mistral::{KvCacheMode, MistralGenerator},
    GenerationRequest, InferenceError, TextGenerator,
};

/// In-process text-generation backed by `vibe-infer::MistralGenerator`.
pub struct MistralRsBackend {
    #[cfg(feature = "vibe-mistralrs")]
    cache: Arc<RwLock<HashMap<String, Arc<MistralGenerator>>>>,
    #[cfg(not(feature = "vibe-mistralrs"))]
    _private: (),
}

impl Default for MistralRsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MistralRsBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "vibe-mistralrs")]
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(not(feature = "vibe-mistralrs"))]
            _private: (),
        }
    }
}

#[cfg(feature = "vibe-mistralrs")]
impl MistralRsBackend {
    async fn get_or_load(&self, model_id: &str) -> BackendResult<Arc<MistralGenerator>> {
        if let Some(g) = self.cache.read().await.get(model_id) {
            return Ok(Arc::clone(g));
        }
        let kv_mode = KvCacheMode::from_env();
        tracing::info!(
            "vibecli inference: loading mistralrs model {model_id} (kv_cache={kv_mode:?})"
        );
        let gen = MistralGenerator::load_with_kv_cache(model_id, kv_mode)
            .await
            .map_err(map_infer_err)?;
        let arc = Arc::new(gen);
        self.cache
            .write()
            .await
            .insert(model_id.to_string(), Arc::clone(&arc));
        Ok(arc)
    }
}

#[cfg(feature = "vibe-mistralrs")]
fn map_infer_err(e: InferenceError) -> BackendError {
    match e {
        InferenceError::ModelNotFound(name, _) => BackendError::ModelNotFound(name),
        InferenceError::BackendNotEnabled(feat) => BackendError::Unavailable(format!(
            "vibe-infer feature `{feat}` not built into this binary"
        )),
        other => BackendError::Upstream(other.to_string()),
    }
}

#[cfg(feature = "vibe-mistralrs")]
fn finish_label(reason: vibe_infer::FinishReason) -> &'static str {
    match reason {
        vibe_infer::FinishReason::Stop => "stop",
        vibe_infer::FinishReason::Length => "length",
        vibe_infer::FinishReason::Error => "error",
    }
}

#[cfg(feature = "vibe-mistralrs")]
fn flatten_messages(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pull `num_predict` (tokens) and `temperature` out of an Ollama-style
/// `options` blob. Ollama uses `num_predict` for the cap; OpenAI uses
/// `max_tokens`. We accept either so curl examples written for the OpenAI
/// world still work.
#[cfg(feature = "vibe-mistralrs")]
fn sampler_from_options(opts: Option<&serde_json::Value>) -> (usize, f32) {
    let max_tokens = opts
        .and_then(|v| v.get("num_predict").or_else(|| v.get("max_tokens")))
        .and_then(|v| v.as_u64())
        .unwrap_or(512) as usize;
    let temperature = opts
        .and_then(|v| v.get("temperature"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7) as f32;
    (max_tokens, temperature)
}

#[async_trait]
impl Backend for MistralRsBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Mistralrs
    }

    #[cfg(feature = "vibe-mistralrs")]
    async fn chat(
        &self,
        req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>> {
        let gen = self.get_or_load(&req.model).await?;
        let prompt = flatten_messages(&req.messages);
        let (max_tokens, temperature) = sampler_from_options(req.options.as_ref());

        let resp = gen
            .generate(GenerationRequest {
                prompt,
                max_tokens,
                temperature,
                stop: vec![],
            })
            .await
            .map_err(map_infer_err)?;

        let model = req.model.clone();
        let now = chrono::Utc::now().to_rfc3339();
        let content_frame = ChatChunk {
            model: model.clone(),
            created_at: now.clone(),
            message: ChatMessage {
                role: "assistant".into(),
                content: resp.text,
                images: None,
            },
            done: false,
            done_reason: None,
        };
        let done_frame = ChatChunk {
            model,
            created_at: now,
            message: ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                images: None,
            },
            done: true,
            done_reason: Some(finish_label(resp.finish_reason).to_string()),
        };
        Ok(Box::pin(stream::iter(vec![Ok(content_frame), Ok(done_frame)])))
    }

    #[cfg(not(feature = "vibe-mistralrs"))]
    async fn chat(
        &self,
        _req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(feature = "vibe-mistralrs")]
    async fn generate(
        &self,
        req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>> {
        let gen = self.get_or_load(&req.model).await?;
        let (max_tokens, temperature) = sampler_from_options(req.options.as_ref());

        let resp = gen
            .generate(GenerationRequest {
                prompt: req.prompt,
                max_tokens,
                temperature,
                stop: vec![],
            })
            .await
            .map_err(map_infer_err)?;

        let model = req.model.clone();
        let now = chrono::Utc::now().to_rfc3339();
        let content_frame = GenerateChunk {
            model: model.clone(),
            created_at: now.clone(),
            response: resp.text,
            done: false,
            done_reason: None,
        };
        let done_frame = GenerateChunk {
            model,
            created_at: now,
            response: String::new(),
            done: true,
            done_reason: Some(finish_label(resp.finish_reason).to_string()),
        };
        Ok(Box::pin(stream::iter(vec![
            Ok(content_frame),
            Ok(done_frame),
        ])))
    }

    #[cfg(not(feature = "vibe-mistralrs"))]
    async fn generate(
        &self,
        _req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(feature = "vibe-mistralrs")]
    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        let cache = self.cache.read().await;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(cache
            .keys()
            .map(|name| ModelInfo {
                name: name.clone(),
                modified_at: now.clone(),
                size: 0,
                backend: BackendKind::Mistralrs,
                digest: None,
            })
            .collect())
    }

    #[cfg(not(feature = "vibe-mistralrs"))]
    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        Ok(Vec::new())
    }

    #[cfg(feature = "vibe-mistralrs")]
    async fn pull(
        &self,
        req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>> {
        // mistralrs lazy-loads on first use, so "pull" = "load and cache."
        // We surface this as a 2-frame stream (downloading → success) so
        // ollama-compat clients can poll the same shape they expect.
        let _ = self.get_or_load(&req.name).await?;
        let progress = vec![
            Ok(PullProgress {
                status: format!("loaded {} into mistralrs cache", req.name),
                digest: None,
                total: None,
                completed: None,
            }),
            Ok(PullProgress {
                status: "success".into(),
                digest: None,
                total: None,
                completed: None,
            }),
        ];
        Ok(Box::pin(stream::iter(progress)))
    }

    #[cfg(not(feature = "vibe-mistralrs"))]
    async fn pull(
        &self,
        _req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(feature = "vibe-mistralrs")]
    async fn show(&self, name: &str) -> BackendResult<ModelInfo> {
        let cache = self.cache.read().await;
        if cache.contains_key(name) {
            Ok(ModelInfo {
                name: name.to_string(),
                modified_at: chrono::Utc::now().to_rfc3339(),
                size: 0,
                backend: BackendKind::Mistralrs,
                digest: None,
            })
        } else {
            Err(BackendError::ModelNotFound(name.into()))
        }
    }

    #[cfg(not(feature = "vibe-mistralrs"))]
    async fn show(&self, name: &str) -> BackendResult<ModelInfo> {
        Err(BackendError::ModelNotFound(name.into()))
    }
}
