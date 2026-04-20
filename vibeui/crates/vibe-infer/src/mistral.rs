//! Mistral.rs text-generation backend.
//!
//! Wraps the `mistralrs` crate (https://github.com/EricLBuehler/mistral.rs) so
//! VibeCody can run LLM inference in-process without spawning a vLLM sidecar.
//! Mistral.rs brings PagedAttention, in-situ quantization (ISQ), LoRA, and an
//! OpenAI-compatible request surface — a natural home for the Rust-side
//! inference experiments we want to run (TurboQuant-for-KV-cache being the
//! motivating one).
//!
//! ## Feature gating
//!
//! - `mistralrs`        — base CPU build. Pulls candle 0.10.x transitively.
//! - `mistralrs-cuda`   — NVIDIA GPU acceleration.
//! - `mistralrs-metal`  — Apple GPU acceleration.
//! - `mistralrs-flash-attn` — FlashAttention-2 on Ampere+; implies cuda.
//!
//! None of these are in `default` — builds stay fast until someone opts in.
//!
//! ## Example
//! ```no_run
//! # #[cfg(feature = "mistralrs")]
//! # async fn demo() -> vibe_infer::Result<()> {
//! use vibe_infer::{GenerationRequest, TextGenerator, mistral::MistralGenerator};
//! let gen = MistralGenerator::load("Qwen/Qwen2.5-0.5B-Instruct").await?;
//! let out = gen
//!     .generate(GenerationRequest {
//!         prompt: "Say hi in one word.".into(),
//!         max_tokens: 16,
//!         temperature: 0.0,
//!         stop: vec![],
//!     })
//!     .await?;
//! println!("{}", out.text);
//! # Ok(()) }
//! ```

use async_trait::async_trait;
use mistralrs::{
    IsqType, Model, RequestBuilder, TextMessageRole, TextMessages, TextModelBuilder,
};

use crate::kv_cache_codec::CandleTurboQuantCodec;
use crate::{FinishReason, GenerationRequest, GenerationResponse, InferenceError, Result, TextGenerator};

/// KV-cache storage mode selected at load time.
///
/// `Fp16` is bit-exact with upstream mistral.rs (no codec installed).
/// `TurboQuant` reconstructs values through PolarQuant + QJL on every
/// write — same wire dtype as `Fp16`, but lossy reconstruction lets us
/// measure fidelity-vs-savings on real workloads. Future packed-storage
/// codecs will need a richer trait surface; this mode exists today only
/// as a correctness experiment.
#[derive(Debug, Clone)]
pub enum KvCacheMode {
    Fp16,
    TurboQuant {
        seed: u64,
        /// QJL projection dim. `None` defaults to `head_dim` (cosine fidelity
        /// floor matches the published Phase-3 spike).
        qjl_proj_dim: Option<usize>,
    },
}

impl KvCacheMode {
    /// Resolve `KvCacheMode` from `VIBE_INFER_KV_CACHE` so the UI can flip
    /// modes without a recompile. Recognized values:
    /// - unset / `"fp16"` / `"none"` → [`Self::Fp16`]
    /// - `"turboquant"` → [`Self::TurboQuant`] with seed from
    ///   `VIBE_INFER_KV_CACHE_SEED` (default `42`) and `qjl_proj_dim` from
    ///   `VIBE_INFER_KV_CACHE_QJL_DIM` (default `None`, i.e. `head_dim`).
    pub fn from_env() -> Self {
        let raw = std::env::var("VIBE_INFER_KV_CACHE")
            .unwrap_or_default()
            .to_ascii_lowercase();
        match raw.as_str() {
            "turboquant" | "turbo_quant" | "tq" => {
                let seed = std::env::var("VIBE_INFER_KV_CACHE_SEED")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(42);
                let qjl_proj_dim = std::env::var("VIBE_INFER_KV_CACHE_QJL_DIM")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok());
                Self::TurboQuant { seed, qjl_proj_dim }
            }
            _ => Self::Fp16,
        }
    }
}

/// An in-process Mistral.rs text-generation backend.
///
/// `Model` is `Send + Sync`; a single instance may be shared across tasks.
pub struct MistralGenerator {
    model: Model,
    model_id: String,
}

impl MistralGenerator {
    /// Load a model by Hugging Face repo id (e.g. `"Qwen/Qwen2.5-0.5B-Instruct"`).
    ///
    /// Weights are fetched on first call into the `hf-hub` cache. No ISQ applied
    /// — use [`Self::load_isq`] when you want on-load quantization. KV-cache
    /// mode is read from `VIBE_INFER_KV_CACHE` (see [`KvCacheMode::from_env`]).
    pub async fn load(model_id: &str) -> Result<Self> {
        Self::load_with_kv_cache(model_id, KvCacheMode::from_env()).await
    }

    /// Load with in-situ quantization applied during weight loading. `Q4K`
    /// (4-bit K-quants) is a reasonable default that roughly halves VRAM with
    /// minimal quality loss.
    pub async fn load_isq(model_id: &str, isq: IsqType) -> Result<Self> {
        Self::load_with_kv_cache_isq(model_id, isq, KvCacheMode::from_env()).await
    }

    /// Load + explicitly choose the KV-cache mode. Prefer this in tests so the
    /// outcome doesn't depend on ambient env vars.
    pub async fn load_with_kv_cache(model_id: &str, kv_cache: KvCacheMode) -> Result<Self> {
        let model = TextModelBuilder::new(model_id.to_string())
            .build()
            .await
            .map_err(be)?;
        install_codec_after_load(&model, &kv_cache).await?;
        Ok(Self {
            model,
            model_id: model_id.to_string(),
        })
    }

    /// Load + ISQ + explicit KV-cache mode.
    pub async fn load_with_kv_cache_isq(
        model_id: &str,
        isq: IsqType,
        kv_cache: KvCacheMode,
    ) -> Result<Self> {
        let model = TextModelBuilder::new(model_id.to_string())
            .with_isq(isq)
            .build()
            .await
            .map_err(be)?;
        install_codec_after_load(&model, &kv_cache).await?;
        Ok(Self {
            model,
            model_id: model_id.to_string(),
        })
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Install the codec selected by `mode` onto every attention layer of `model`.
/// No-op for [`KvCacheMode::Fp16`]. Logs the layer count for observability —
/// this is the only signal the user gets that the codec actually landed,
/// since the codec ride-along is invisible at the request surface.
async fn install_codec_after_load(model: &Model, mode: &KvCacheMode) -> Result<()> {
    let KvCacheMode::TurboQuant { seed, qjl_proj_dim } = mode else {
        return Ok(());
    };

    // `kv_head_dims` returns `(k_head_dim, v_head_dim)`. We pick the larger
    // because the codec assumes a single head_dim and any head with extras
    // would otherwise be truncated by the encoder. Models where K and V
    // disagree (MLA, etc.) will need a richer codec API; flag here for now.
    let head_dim = match model.kv_head_dims().await.map_err(be)? {
        Some((k, v)) => {
            if k != v {
                tracing::warn!(
                    "vibe-infer: TurboQuant configured but k_head_dim ({k}) != v_head_dim ({v}); \
                     using max — fidelity may regress for MLA-style models"
                );
            }
            k.max(v)
        }
        None => {
            return Err(InferenceError::Backend(
                "TurboQuant requested but model exposes no head-dim metadata \
                 (likely a speech / diffusion pipeline)".into(),
            ));
        }
    };

    let codec = CandleTurboQuantCodec::shared(head_dim, *seed, *qjl_proj_dim);
    let installed = model.set_kv_cache_codec(codec).await.map_err(be)?;
    tracing::info!(
        "vibe-infer: installed TurboQuant KV-cache codec on {installed} layer(s) \
         (head_dim={head_dim}, seed={seed}, qjl_proj_dim={qjl_proj_dim:?})"
    );
    Ok(())
}

#[async_trait]
impl TextGenerator for MistralGenerator {
    async fn generate(&self, req: GenerationRequest) -> Result<GenerationResponse> {
        let messages = TextMessages::new().add_message(TextMessageRole::User, &req.prompt);
        let builder = RequestBuilder::from(messages)
            .set_sampler_max_len(req.max_tokens)
            .set_sampler_temperature(req.temperature as f64);

        let response = self.model.send_chat_request(builder).await.map_err(be)?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| InferenceError::Backend("mistralrs: no choices in response".into()))?;

        let text = choice
            .message
            .content
            .clone()
            .unwrap_or_default();

        let finish = match choice.finish_reason.as_str() {
            "stop" => FinishReason::Stop,
            "length" => FinishReason::Length,
            _ => FinishReason::Stop,
        };

        let tokens_generated = response.usage.completion_tokens as usize;

        Ok(GenerationResponse {
            text,
            tokens_generated,
            finish_reason: finish,
        })
    }
}

fn be<E: std::fmt::Display>(e: E) -> InferenceError {
    InferenceError::Backend(format!("mistralrs: {e}"))
}
