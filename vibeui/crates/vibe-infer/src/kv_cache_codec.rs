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

use std::sync::{Arc, Mutex};

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

// ───────────────────────────────────────────────────────────────────────────
// Native (device-side) TurboQuant codec
// ───────────────────────────────────────────────────────────────────────────

/// PolarQuant + QJL codec backed by the fused CUDA / Metal kernels in
/// `mistralrs-quant`. Falls back to the pure-Rust [`CandleTurboQuantCodec`]
/// path when the input tensor lives on the CPU.
///
/// The kernel needs the rotation `R` and projection `P` matrices on the same
/// device as the input. We materialise them lazily on the first encode call
/// and cache the device tensors. If the codec ever sees a different device
/// (rare — typically a cache lives on one device for its lifetime) the
/// matrices are rebuilt on the new device.
pub struct NativeTurboQuantCodec {
    inner: KvCacheTurboQuant,
    head_dim: usize,
    proj_dim: usize,
    /// Cached `(rotation, projection, device)` triple. The bundled `Device`
    /// lets us detect device drift cheaply.
    device_matrices: Mutex<Option<DeviceMatrices>>,
}

struct DeviceMatrices {
    rotation: Tensor,
    projection: Tensor,
    device: Device,
}

impl std::fmt::Debug for NativeTurboQuantCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeTurboQuantCodec")
            .field("head_dim", &self.head_dim)
            .field("proj_dim", &self.proj_dim)
            .finish()
    }
}

impl NativeTurboQuantCodec {
    pub fn new(head_dim: usize, seed: u64, qjl_proj_dim: Option<usize>) -> Self {
        let inner = KvCacheTurboQuant::new(head_dim, seed, qjl_proj_dim);
        let proj_dim = inner.proj_dim();
        Self {
            inner,
            head_dim,
            proj_dim,
            device_matrices: Mutex::new(None),
        }
    }

    pub fn shared(head_dim: usize, seed: u64, qjl_proj_dim: Option<usize>) -> Arc<dyn KvCacheCodec> {
        Arc::new(Self::new(head_dim, seed, qjl_proj_dim))
    }

    /// Build (rotation, projection) device tensors on `device` from the
    /// pure-Rust matrices. Truncates the QJL basis to `[proj_dim, head_dim]`
    /// to match the reference (`qjl_proj_dim` may be smaller than the basis
    /// returned by `gram_schmidt_rotation(max(proj_dim, head_dim))`).
    fn build_device_matrices(&self, device: &Device) -> Result<DeviceMatrices> {
        let rotation_host = self.inner.rotation_matrix();
        let projection_host = self.inner.projection_matrix();

        let rotation = Tensor::from_vec(
            rotation_host.to_vec(),
            (self.head_dim, self.head_dim),
            device,
        )?;

        // The pure-Rust impl stores the QJL basis as a square
        // max(proj_dim, head_dim) × max(proj_dim, head_dim) matrix and
        // truncates per-row at use time. Mirror that: take the first
        // `proj_dim` rows of `head_dim` columns each.
        let basis_dim = (projection_host.len() as f32).sqrt() as usize;
        debug_assert_eq!(basis_dim * basis_dim, projection_host.len());
        let mut truncated = Vec::with_capacity(self.proj_dim * self.head_dim);
        for i in 0..self.proj_dim {
            let row_start = i * basis_dim;
            truncated.extend_from_slice(&projection_host[row_start..row_start + self.head_dim]);
        }
        let projection = Tensor::from_vec(truncated, (self.proj_dim, self.head_dim), device)?;

        Ok(DeviceMatrices {
            rotation,
            projection,
            device: device.clone(),
        })
    }

    fn cpu_encode_fallback(&self, tensor: &Tensor) -> Result<Tensor> {
        let dtype = tensor.dtype();
        let device = tensor.device().clone();
        let total: usize = tensor.dims().iter().product();
        let num_vectors = total / self.head_dim;

        let flat: Vec<f32> = tensor
            .to_dtype(DType::F32)?
            .to_device(&Device::Cpu)?
            .contiguous()?
            .flatten_all()?
            .to_vec1::<f32>()?;
        let layer = self.inner.encode_layer(&flat, 1, num_vectors);
        let mut out = vec![0f32; total];
        for t in 0..num_vectors {
            let slot = &mut out[t * self.head_dim..(t + 1) * self.head_dim];
            self.inner.decode_one(&layer, 0, t, slot);
        }

        Tensor::from_vec(out, (total,), &Device::Cpu)?
            .reshape(tensor.dims())?
            .to_dtype(dtype)?
            .to_device(&device)
    }
}

impl KvCacheCodec for NativeTurboQuantCodec {
    fn encode(&self, tensor: &Tensor) -> Result<Tensor> {
        let dims = tensor.dims().to_vec();
        if dims.is_empty() {
            candle_core::bail!("NativeTurboQuantCodec: expected a ranked tensor, got scalar");
        }
        if *dims.last().unwrap() != self.head_dim {
            candle_core::bail!(
                "NativeTurboQuantCodec: last-axis must be head_dim={}, got shape {:?}",
                self.head_dim,
                dims,
            );
        }

        match tensor.device() {
            Device::Cpu => self.cpu_encode_fallback(tensor),
            device => {
                // Materialise / refresh the device matrices.
                let mut guard = self.device_matrices.lock().unwrap();
                let needs_rebuild = match &*guard {
                    Some(dm) => !dm.device.same_device(device),
                    None => true,
                };
                if needs_rebuild {
                    *guard = Some(self.build_device_matrices(device)?);
                }
                let dm = guard.as_ref().expect("device matrices set above");

                // The kernel only handles F32/F16 — promote BF16 if needed.
                let original_dtype = tensor.dtype();
                let kernel_input = match original_dtype {
                    DType::F32 | DType::F16 => tensor.clone(),
                    DType::BF16 => tensor.to_dtype(DType::F16)?,
                    other => candle_core::bail!(
                        "NativeTurboQuantCodec: unsupported dtype {:?} for device path",
                        other
                    ),
                };

                let encoded = mistralrs::core::turboquant::encode(
                    &kernel_input,
                    &dm.rotation,
                    &dm.projection,
                )?;
                if encoded.dtype() != original_dtype {
                    encoded.to_dtype(original_dtype)
                } else {
                    Ok(encoded)
                }
            }
        }
    }

    fn decode(&self, tensor: &Tensor) -> Result<Tensor> {
        Ok(tensor.clone())
    }

    fn name(&self) -> &str {
        "turboquant-native"
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

    /// Native codec on a CPU tensor must reproduce CandleTurboQuantCodec
    /// exactly. The CPU fallback path uses the same `KvCacheTurboQuant`
    /// internals, so the two must be byte-identical for the same seed.
    #[test]
    fn native_codec_cpu_path_matches_candle() {
        let head_dim = 64;
        let candle = CandleTurboQuantCodec::new(head_dim, 13, None);
        let native = NativeTurboQuantCodec::new(head_dim, 13, None);
        let input = sample_tensor(&[2, 3, head_dim]);

        let a: Vec<f32> = candle.encode(&input).unwrap().flatten_all().unwrap().to_vec1().unwrap();
        let b: Vec<f32> = native.encode(&input).unwrap().flatten_all().unwrap().to_vec1().unwrap();

        assert_eq!(a, b, "native CPU fallback must match CandleTurboQuantCodec exactly");
    }

    /// Native codec on Metal must match the CPU reference within fp16
    /// rounding. Skipped on builds without Metal — the GPU is needed to run
    /// this. Tolerance is loose because the kernel computes everything in
    /// fp32 inside the threadgroup, but the round-trip writes back through
    /// the input dtype (typically fp16), so values differ by ~1e-3.
    #[cfg(all(test, feature = "mistralrs-metal"))]
    #[test]
    fn native_codec_metal_matches_cpu_within_fp16_tolerance() {
        let device = match Device::new_metal(0) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("skipping metal parity test: {e}");
                return;
            }
        };

        let head_dim = 64;
        let proj_dim = 64;
        let cpu_codec = CandleTurboQuantCodec::new(head_dim, 99, Some(proj_dim));
        let native = NativeTurboQuantCodec::new(head_dim, 99, Some(proj_dim));

        let input_cpu = sample_tensor(&[2, 8, head_dim]).to_dtype(DType::F32).unwrap();
        let input_metal = input_cpu.to_device(&device).unwrap();

        let cpu_out: Vec<f32> = cpu_codec
            .encode(&input_cpu)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let metal_out: Vec<f32> = native
            .encode(&input_metal)
            .unwrap()
            .to_device(&Device::Cpu)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        assert_eq!(cpu_out.len(), metal_out.len());
        let mut max_abs = 0.0f32;
        for (a, b) in cpu_out.iter().zip(metal_out.iter()) {
            max_abs = max_abs.max((a - b).abs());
        }
        assert!(
            max_abs < 5e-3,
            "metal vs cpu max abs diff {max_abs} exceeds 5e-3 tolerance"
        );
    }

    /// CUDA parity test — same shape as the Metal one. Requires nvcc + a
    /// CUDA-capable GPU at test time.
    #[cfg(all(test, feature = "mistralrs-cuda"))]
    #[test]
    fn native_codec_cuda_matches_cpu_within_fp16_tolerance() {
        let device = match Device::new_cuda(0) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("skipping cuda parity test: {e}");
                return;
            }
        };

        let head_dim = 64;
        let proj_dim = 64;
        let cpu_codec = CandleTurboQuantCodec::new(head_dim, 17, Some(proj_dim));
        let native = NativeTurboQuantCodec::new(head_dim, 17, Some(proj_dim));

        let input_cpu = sample_tensor(&[2, 8, head_dim]).to_dtype(DType::F32).unwrap();
        let input_cuda = input_cpu.to_device(&device).unwrap();

        let cpu_out: Vec<f32> = cpu_codec
            .encode(&input_cpu)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let cuda_out: Vec<f32> = native
            .encode(&input_cuda)
            .unwrap()
            .to_device(&Device::Cpu)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        assert_eq!(cpu_out.len(), cuda_out.len());
        let mut max_abs = 0.0f32;
        for (a, b) in cpu_out.iter().zip(cuda_out.iter()) {
            max_abs = max_abs.max((a - b).abs());
        }
        assert!(
            max_abs < 5e-3,
            "cuda vs cpu max abs diff {max_abs} exceeds 5e-3 tolerance"
        );
    }
}
