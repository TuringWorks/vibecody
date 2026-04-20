//! all-MiniLM-L6-v2 embedder backed by candle.
//!
//! Loads `sentence-transformers/all-MiniLM-L6-v2` (6-layer BERT, 384-dim)
//! from the Hugging Face cache on first call and keeps the model + tokenizer
//! resident for subsequent embeddings.
//!
//! The model weights (~22 MB safetensors) are fetched via `hf-hub`. After
//! the first download the cache is offline; set `HF_HOME` to control the
//! location (defaults to `~/.cache/huggingface`).
//!
//! # Example
//! ```no_run
//! # #[cfg(feature = "candle")]
//! # async fn demo() -> vibe_infer::Result<()> {
//! use vibe_infer::{Embedder, minilm::MiniLmEmbedder};
//! let embedder = MiniLmEmbedder::load().await?;
//! let v = embedder.embed("hello world").await?;
//! assert_eq!(v.len(), 384);
//! # Ok(()) }
//! ```

use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, HiddenAct, DTYPE};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;

use crate::{Embedder, InferenceError, Result};

const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
const MODEL_REVISION: &str = "main";
const EMBEDDING_DIM: usize = 384;

pub struct MiniLmEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl MiniLmEmbedder {
    /// Load model weights + tokenizer from the Hugging Face cache. Downloads
    /// on first call; subsequent calls are offline.
    pub async fn load() -> Result<Self> {
        let device = best_device();
        let api = Api::new().map_err(be)?;
        let repo = api.repo(Repo::with_revision(
            MODEL_ID.to_string(),
            RepoType::Model,
            MODEL_REVISION.to_string(),
        ));

        let config_path = repo.get("config.json").await.map_err(be)?;
        let tokenizer_path = repo.get("tokenizer.json").await.map_err(be)?;
        let weights_path = repo.get("model.safetensors").await.map_err(be)?;

        let config: Config = {
            let bytes = std::fs::read(&config_path).map_err(be)?;
            let mut cfg: Config = serde_json::from_slice(&bytes).map_err(be)?;
            // MiniLM uses GELU but the default Config deserialization picks
            // HiddenAct::Gelu which matches; explicit for clarity.
            cfg.hidden_act = HiddenAct::Gelu;
            cfg
        };

        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(be)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device).map_err(be)?
        };
        let model = BertModel::load(vb, &config).map_err(be)?;

        Ok(Self { model, tokenizer, device })
    }

    fn forward(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| InferenceError::Backend(format!("tokenize: {e}")))?;
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();

        let input_ids = Tensor::new(ids, &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(be)?;
        let attn_mask = Tensor::new(mask, &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(be)?;
        let token_type_ids = input_ids.zeros_like().map_err(be)?;

        // [1, seq, hidden]
        let hidden = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attn_mask))
            .map_err(be)?;
        let pooled = mean_pool(&hidden, &attn_mask)?;
        let normed = l2_normalize(&pooled)?;

        // [1, 384] -> Vec<f32>
        let v: Vec<f32> = normed.squeeze(0).and_then(|t| t.to_vec1()).map_err(be)?;
        Ok(v)
    }
}

#[async_trait]
impl Embedder for MiniLmEmbedder {
    fn dim(&self) -> usize {
        EMBEDDING_DIM
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // candle's forward pass is CPU/GPU-bound and short (~ms for one
        // sentence). Run inline rather than spawn_blocking — the latter
        // requires `'static` and forces the embedder to be Arc'd everywhere.
        // If batch sizes grow, override `embed_batch` and dispatch there.
        self.forward(text)
    }
}

fn mean_pool(hidden: &Tensor, attn_mask: &Tensor) -> Result<Tensor> {
    // Mean over non-padding positions. Matches sentence-transformers default.
    let mask = attn_mask
        .to_dtype(DType::F32)
        .and_then(|m| m.unsqueeze(2))
        .map_err(be)?;
    let masked = hidden.broadcast_mul(&mask).map_err(be)?;
    let summed = masked.sum(1).map_err(be)?;
    let counts = mask.sum(1).map_err(be)?;
    summed.broadcast_div(&counts).map_err(be)
}

fn l2_normalize(t: &Tensor) -> Result<Tensor> {
    let sq = t.sqr().map_err(be)?;
    let sum = sq.sum_keepdim(1).map_err(be)?;
    let norm = sum.sqrt().map_err(be)?;
    t.broadcast_div(&norm).map_err(be)
}

fn best_device() -> Device {
    #[cfg(all(target_os = "macos", feature = "candle-metal"))]
    {
        if let Ok(d) = Device::new_metal(0) {
            return d;
        }
    }
    Device::Cpu
}

fn be<E: std::fmt::Display>(e: E) -> InferenceError {
    InferenceError::Backend(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Marked #[ignore] because first run downloads ~22 MB from Hugging Face.
    /// Run with: cargo test -p vibe-infer --features candle -- --ignored
    #[tokio::test]
    #[ignore]
    async fn minilm_embeds_384_dim_l2_normalized() {
        let embedder = MiniLmEmbedder::load().await.expect("load model");
        assert_eq!(embedder.dim(), 384);

        let v = embedder.embed("hello world").await.expect("embed");
        assert_eq!(v.len(), 384);

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "expected L2-norm ~1.0, got {norm}");
    }

    #[tokio::test]
    #[ignore]
    async fn similar_sentences_score_higher_than_unrelated() {
        let embedder = MiniLmEmbedder::load().await.expect("load model");
        let a = embedder.embed("a cat sat on the mat").await.unwrap();
        let b = embedder.embed("a kitten rested on a rug").await.unwrap();
        let c = embedder.embed("quarterly earnings exceeded forecasts").await.unwrap();

        let ab: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let ac: f32 = a.iter().zip(&c).map(|(x, y)| x * y).sum();
        assert!(ab > ac, "related pair ({ab}) should outscore unrelated ({ac})");
    }
}
