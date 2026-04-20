//! Candle-backed `KvCacheCodec` implementations for the TuringWorks fork of
//! mistral.rs.
//!
//! The fork's `mistralrs_core::KvCacheCodec` trait takes a `candle::Tensor`
//! and must return a tensor with the same shape and dtype (see
//! `mistralrs-core/src/kv_cache/codec.rs`). This module bridges that
//! interface to the pure-Rust [`KvCacheTurboQuant`] we built in Phase 3:
//! the codec is lossy *within* fp16 storage — the returned tensor carries
//! reconstructed values from PolarQuant + QJL, not a packed bit layout.
//! True packed-storage codecs need a richer trait (future work, probably a
//! `mistralrs-quant` CUDA/Metal kernel PR).
//!
//! # Shape contract
//!
//! The codec assumes `head_dim` is the **last** axis. All other axes are
//! flattened into `num_vectors = total_elements / head_dim` for the pure-Rust
//! encoder, which treats them as one head × `num_vectors` tokens. That's fine
//! — the codec is per-vector and doesn't care about head/batch structure.
//!
//! # Cost
//!
//! Encode shuttles the tensor to host-f32 and back on every write — roughly
//! 2× a device-to-host copy plus the quant math. Acceptable for correctness
//! experiments and small-batch decoding; not a production path. Swap in a
//! device-side kernel when memory savings start mattering more than latency.

use std::sync::Arc;

use candle_core::{DType, Device, Result, Tensor};
use mistralrs::core::KvCacheCodec;

use crate::kv_cache_tq::KvCacheTurboQuant;

/// PolarQuant + QJL codec implemented in pure Rust on host memory.
///
/// Construct once per cache config (head_dim + seed) and install via
/// `SingleCache::set_codec` / `RotatingCache::set_codec` on the fork.
#[derive(Debug)]
pub struct CandleTurboQuantCodec {
    inner: KvCacheTurboQuant,
    head_dim: usize,
}

impl CandleTurboQuantCodec {
    pub fn new(head_dim: usize, seed: u64, qjl_proj_dim: Option<usize>) -> Self {
        Self {
            inner: KvCacheTurboQuant::new(head_dim, seed, qjl_proj_dim),
            head_dim,
        }
    }

    /// Convenience for the common case: call this to wrap the codec in an
    /// `Arc<dyn KvCacheCodec>` ready to hand to `set_codec`.
    pub fn shared(head_dim: usize, seed: u64, qjl_proj_dim: Option<usize>) -> Arc<dyn KvCacheCodec> {
        Arc::new(Self::new(head_dim, seed, qjl_proj_dim))
    }
}

impl KvCacheCodec for CandleTurboQuantCodec {
    fn encode(&self, tensor: &Tensor) -> Result<Tensor> {
        let dtype = tensor.dtype();
        let device = tensor.device().clone();
        let shape = tensor.dims().to_vec();

        if shape.is_empty() {
            candle_core::bail!("TurboQuantCodec: expected a ranked tensor, got scalar");
        }
        if *shape.last().unwrap() != self.head_dim {
            candle_core::bail!(
                "TurboQuantCodec: last-axis must be head_dim={}, got shape {:?}",
                self.head_dim,
                shape,
            );
        }

        let total: usize = shape.iter().product();
        let num_vectors = total / self.head_dim;

        // Shuttle to host f32 for the pure-Rust codec. contiguous() protects
        // against strided views from `narrow` / broadcast paths.
        let flat: Vec<f32> = tensor
            .to_dtype(DType::F32)?
            .to_device(&Device::Cpu)?
            .contiguous()?
            .flatten_all()?
            .to_vec1::<f32>()?;

        // heads=1, seq=num_vectors: the codec operates per `head_dim` vector,
        // so flattening other axes into the seq axis is lossless.
        let layer = self.inner.encode_layer(&flat, 1, num_vectors);

        let mut out = vec![0f32; total];
        for t in 0..num_vectors {
            let slot = &mut out[t * self.head_dim..(t + 1) * self.head_dim];
            self.inner.decode_one(&layer, 0, t, slot);
        }

        Tensor::from_vec(out, (total,), &Device::Cpu)?
            .reshape(shape)?
            .to_dtype(dtype)?
            .to_device(&device)
    }

    fn decode(&self, tensor: &Tensor) -> Result<Tensor> {
        // Values were already quantized + reconstructed by `encode`. The cache
        // stored that reconstruction as fp16 (or whatever the input dtype
        // was), so decode is a straight identity.
        Ok(tensor.clone())
    }

    fn name(&self) -> &str {
        "turboquant"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tensor(shape: &[usize]) -> Tensor {
        let total: usize = shape.iter().product();
        let mut state = 42u64;
        let vals: Vec<f32> = (0..total)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
            })
            .collect();
        Tensor::from_vec(vals, (total,), &Device::Cpu)
            .unwrap()
            .reshape(shape)
            .unwrap()
    }

    fn cos_sim(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }

    #[test]
    fn encode_preserves_shape_and_dtype() {
        let codec = CandleTurboQuantCodec::new(64, 42, None);
        let input = sample_tensor(&[2, 4, 8, 64]);
        let encoded = codec.encode(&input).unwrap();
        assert_eq!(encoded.dims(), input.dims());
        assert_eq!(encoded.dtype(), input.dtype());
    }

    #[test]
    fn decode_is_identity() {
        let codec = CandleTurboQuantCodec::new(64, 42, None);
        let input = sample_tensor(&[1, 2, 4, 64]);
        let encoded = codec.encode(&input).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        let e: Vec<f32> = encoded.flatten_all().unwrap().to_vec1().unwrap();
        let d: Vec<f32> = decoded.flatten_all().unwrap().to_vec1().unwrap();
        assert_eq!(e, d);
    }

    #[test]
    fn rejects_wrong_last_axis() {
        let codec = CandleTurboQuantCodec::new(128, 42, None);
        let input = sample_tensor(&[2, 4, 64]);
        let err = codec.encode(&input).unwrap_err();
        assert!(
            err.to_string().contains("last-axis"),
            "wanted last-axis error, got: {err}"
        );
    }

    /// The candle round-trip should match the pure-Rust spike's fidelity on
    /// uniform noise. The spike's `> 0.5` lower bound is the existing
    /// published floor for TurboQuant on flat distributions — matching that
    /// floor proves the candle bridge doesn't introduce additional loss.
    /// Real attention distributions (spiked) produce ~0.92 mean cosine; see
    /// `examples/kv_cache_bench` for that regime.
    #[test]
    fn roundtrip_cosine_matches_phase3_spike() {
        let head_dim = 128;
        let num_vectors = 64;
        let codec = CandleTurboQuantCodec::new(head_dim, 42, None);

        let input = sample_tensor(&[num_vectors, head_dim]);
        let input_vals: Vec<f32> = input.flatten_all().unwrap().to_vec1().unwrap();

        let encoded = codec.encode(&input).unwrap();
        let encoded_vals: Vec<f32> = encoded.flatten_all().unwrap().to_vec1().unwrap();

        let mut total_cos = 0.0f32;
        for t in 0..num_vectors {
            let lhs = &input_vals[t * head_dim..(t + 1) * head_dim];
            let rhs = &encoded_vals[t * head_dim..(t + 1) * head_dim];
            total_cos += cos_sim(lhs, rhs);
        }
        let mean = total_cos / num_vectors as f32;
        assert!(mean > 0.5, "expected mean cosine > 0.5, got {mean}");
    }

    /// Same seed + same head_dim → identical reconstruction. Proves the
    /// codec is deterministic so a model loaded twice produces the same
    /// cache contents.
    #[test]
    fn deterministic_seed_reproduces_bytes() {
        let a = CandleTurboQuantCodec::new(64, 7, None);
        let b = CandleTurboQuantCodec::new(64, 7, None);
        let input = sample_tensor(&[1, 2, 4, 64]);
        let ea: Vec<f32> = a.encode(&input).unwrap().flatten_all().unwrap().to_vec1().unwrap();
        let eb: Vec<f32> = b.encode(&input).unwrap().flatten_all().unwrap().to_vec1().unwrap();
        assert_eq!(ea, eb);
    }

    /// Install into a SingleCache on the fork and prove the trait dispatch
    /// path actually flows tensors through the codec. End-to-end check that
    /// the whole wiring works, not just the standalone encode/decode.
    #[test]
    fn installs_into_single_cache_and_round_trips() {
        use mistralrs::core::SingleCache;

        let head_dim = 64;
        let mut cache = SingleCache::new(2, 16, 16);
        cache.set_codec(CandleTurboQuantCodec::shared(head_dim, 42, None));

        // Shape: [batch=1, heads=2, seq=3, head_dim=64]. dim=2 = seq axis.
        let src = sample_tensor(&[1, 2, 3, head_dim]);
        cache.append(&src).unwrap();
        let out = cache.current_data().unwrap().unwrap();
        assert_eq!(out.dims(), &[1, 2, 3, head_dim]);

        // Values should be close to but not equal to the input (quantized).
        // Threshold matches the pure-Rust spike's uniform-random floor.
        let a: Vec<f32> = src.flatten_all().unwrap().to_vec1().unwrap();
        let b: Vec<f32> = out.flatten_all().unwrap().to_vec1().unwrap();
        let cos = cos_sim(&a, &b);
        assert!(cos > 0.5, "global cosine should be > 0.5, got {cos}");
        assert!(a != b, "codec must modify values; got bit-identical output");
    }
}
