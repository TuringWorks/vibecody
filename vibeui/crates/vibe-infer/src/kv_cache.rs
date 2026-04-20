//! KV-cache compression surface — the experimentation harness for swapping
//! attention key/value storage strategies at runtime.
//!
//! The motivation: we want to evaluate TurboQuant (PolarQuant + QJL, ~3 bits/dim,
//! from Google's paper), DeepSeek-style MLA, KIVI, GEAR, H2O, and Fp8 against
//! each other on the same prompts and measure tokens/sec + quality + memory.
//!
//! This module defines *declarative* types — the runtime that actually owns
//! the KV cache (e.g. Mistral.rs' paged attention allocator) decides how to
//! honour them. Today only [`KvCacheMethod::Fp16`] is a guaranteed no-op pass-
//! through; everything else is a **request** that may or may not be honoured
//! by the chosen backend. Callers should consult
//! [`KvCacheBackend::supports`] before assuming a method is live.
//!
//! # Not here
//!
//! Actual tensor manipulation lives in two places:
//! - `vibeui/crates/vibe-core/src/index/turboquant.rs` — the reference
//!   PolarQuant + QJL encoder for 1D vectors (OpenMemory's embedding index).
//! - A future `mistralrs-quant` PR — the 3D `[tokens, heads, head_dim]`
//!   kernel, once we've prototyped it in Phase 3.
//!
//! Both are intentionally out-of-scope for this file; it is glue, not math.

use serde::{Deserialize, Serialize};

/// One of a fixed menu of KV-cache storage strategies a backend *may* support.
///
/// `Custom(name)` is the escape hatch for registering an experimental
/// strategy at runtime without changing the enum. Use it for research spikes;
/// graduate to a named variant once the method stabilises.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvCacheMethod {
    /// fp16 — the default, baseline, no compression.
    Fp16,
    /// fp8 (E4M3 / E5M2) — ~2× smaller, ~lossless on most models. Supported
    /// natively in vLLM, Mistral.rs with `cuda` feature.
    Fp8,
    /// Symmetric int8 per-channel. Cheap, widely supported.
    Int8,
    /// TurboQuant (PolarQuant + QJL, ~3 bits/dim). Experimental — requires
    /// a backend that has the grouped-quantize kernel wired up. See Phase 3.
    TurboQuant,
    /// Escape hatch — a runtime-registered method keyed by name. Unknown to
    /// backends other than the one that registered it.
    Custom(String),
}

impl KvCacheMethod {
    /// Canonical serialisation used in CLI flags, config files, and the
    /// `/inference/config` HTTP surface. Matches the vLLM `--kv-cache-dtype`
    /// vocabulary where it overlaps.
    pub fn as_flag(&self) -> &str {
        match self {
            Self::Fp16 => "fp16",
            Self::Fp8 => "fp8",
            Self::Int8 => "int8",
            Self::TurboQuant => "turboquant",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Expected bytes per stored element. Used by
    /// [`KvCacheReport::estimate_memory`] and by pod-sizing heuristics upstream.
    pub fn bytes_per_element(&self) -> f32 {
        match self {
            Self::Fp16 => 2.0,
            Self::Fp8 => 1.0,
            Self::Int8 => 1.0,
            // PolarQuant 2-bit code + QJL 1-bit residual ≈ 3 bits/dim asymptotic.
            // Phase 3 measured 0.4375 B/el at head_dim=128 once the per-vector
            // scalar overhead (4B radius + 4B residual_norm) is amortised:
            // 3 bits/dim + 8 B / 128 dim = 0.4375 B/el → ~4.57× vs fp16. See
            // `kv_cache_tq::tests::compression_ratio_matches_spike_finding_for_head_dim_128`.
            Self::TurboQuant => 0.4375,
            // No way to know for a Custom method — callers should supply the
            // number directly via the benchmark harness.
            Self::Custom(_) => f32::NAN,
        }
    }
}

/// Capability probe — each backend advertises which methods it honours.
pub trait KvCacheBackend {
    /// Human-friendly name, e.g. `"mistralrs"`, `"vllm"`, `"llama.cpp"`.
    fn name(&self) -> &str;

    /// Whether this backend will actually apply `method`, vs silently falling
    /// back to fp16.
    fn supports(&self, method: &KvCacheMethod) -> bool;

    /// Ordered list of methods this backend implements, best-first.
    fn methods(&self) -> Vec<KvCacheMethod>;
}

/// A single run of the benchmark harness: `(backend, method, workload) → results`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvCacheReport {
    pub backend: String,
    pub method: KvCacheMethod,
    pub tokens_per_sec_prefill: f32,
    pub tokens_per_sec_decode: f32,
    /// Resident KV-cache bytes at end-of-run. A backend that doesn't track
    /// this precisely may return an upper-bound estimate.
    pub kv_cache_bytes: u64,
    /// Perplexity on a held-out eval set if the harness measured it; `None`
    /// for tokens-only runs.
    pub eval_perplexity: Option<f32>,
    /// Optional: recall@k against an fp16 reference run, for when perplexity
    /// is unavailable but the harness can compare logits/choices.
    pub recall_at_k: Option<f32>,
}

impl KvCacheReport {
    /// Napkin-math memory estimate for a given sequence length + model shape.
    /// Ignores backend-specific alignment / paging overhead.
    pub fn estimate_memory(method: &KvCacheMethod, tokens: u64, num_layers: u32, num_kv_heads: u32, head_dim: u32) -> u64 {
        let elements = tokens * num_layers as u64 * num_kv_heads as u64 * head_dim as u64 * 2; // K + V
        (elements as f32 * method.bytes_per_element()) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_flag_round_trip() {
        assert_eq!(KvCacheMethod::Fp16.as_flag(), "fp16");
        assert_eq!(KvCacheMethod::TurboQuant.as_flag(), "turboquant");
        assert_eq!(KvCacheMethod::Custom("kivi".into()).as_flag(), "kivi");
    }

    #[test]
    fn turboquant_estimate_matches_phase3_spike() {
        // 3 bits/dim + 8 B per-vector scalar overhead at head_dim=128.
        assert_eq!(KvCacheMethod::TurboQuant.bytes_per_element(), 0.4375);
    }

    #[test]
    fn fp16_baseline_doubles_fp8() {
        assert_eq!(KvCacheMethod::Fp16.bytes_per_element(), 2.0 * KvCacheMethod::Fp8.bytes_per_element());
    }

    #[test]
    fn memory_estimate_turboquant_4x_to_5x_smaller_than_fp16() {
        // 32K context × 32 layers × 8 KV heads × 128 head_dim.
        // Phase-3-measured ratio at head_dim=128: 2.0 / 0.4375 ≈ 4.57×.
        let fp16 = KvCacheReport::estimate_memory(&KvCacheMethod::Fp16, 32_768, 32, 8, 128);
        let tq = KvCacheReport::estimate_memory(&KvCacheMethod::TurboQuant, 32_768, 32, 8, 128);
        let ratio = (fp16 as f32) / (tq as f32);
        assert!(ratio > 4.4 && ratio < 4.8, "ratio = {ratio:.2}×");
    }
}
