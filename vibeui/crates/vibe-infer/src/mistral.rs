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

use crate::{FinishReason, GenerationRequest, GenerationResponse, InferenceError, Result, TextGenerator};

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
    /// — use [`Self::load_isq`] when you want on-load quantization.
    pub async fn load(model_id: &str) -> Result<Self> {
        let model = TextModelBuilder::new(model_id.to_string())
            .build()
            .await
            .map_err(be)?;
        Ok(Self {
            model,
            model_id: model_id.to_string(),
        })
    }

    /// Load with in-situ quantization applied during weight loading. `Q4K`
    /// (4-bit K-quants) is a reasonable default that roughly halves VRAM with
    /// minimal quality loss.
    pub async fn load_isq(model_id: &str, isq: IsqType) -> Result<Self> {
        let model = TextModelBuilder::new(model_id.to_string())
            .with_isq(isq)
            .build()
            .await
            .map_err(be)?;
        Ok(Self {
            model,
            model_id: model_id.to_string(),
        })
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
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
