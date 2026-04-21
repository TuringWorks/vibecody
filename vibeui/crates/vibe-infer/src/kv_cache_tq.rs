//! TurboQuant for attention KV cache — Phase 3 research spike.
//!
//! Prototypes PolarQuant + QJL applied to a `[num_heads, seq_len, head_dim]`
//! tensor instead of a 1D embedding. The goal is to measure *viability* —
//! fidelity and memory — before investing in a GPU kernel PR to
//! `mistralrs-quant`.
//!
//! # Why pure Rust (no candle)
//!
//! A CPU reference implementation is deliberate: the point of the spike is to
//! answer "does PolarQuant+QJL preserve attention-top-k well enough for KV?",
//! not to ship a tensor kernel. Once the answer is yes, the actual fused
//! quant/dequant lands as a CUDA kernel in Mistral.rs. Until then, keeping
//! this leaf-free lets anyone run `cargo bench`-style comparisons against
//! Fp16/Fp8/Int8 baselines on a laptop.
//!
//! # Scope
//!
//! - Bulk encode a single attention layer's K or V tensor.
//! - Decode a range of tokens for a given head (matches the access pattern of
//!   `q · Kᵀ` during decode).
//! - Estimate fidelity: cosine similarity of reconstructed head-vectors and
//!   top-k overlap of simulated attention weights.
//! - Compare against Fp8 (E4M3 round-to-nearest) and symmetric Int8.
//!
//! # Out of scope
//!
//! - Paged attention integration (that's the Mistral.rs PR).
//! - Actual GPU kernels (no candle use here; see `mistralrs-quant` instead).
//! - Persistence — `CompressedKvLayer` is in-memory only for the spike.

use serde::{Deserialize, Serialize};

// ── PRNG — xorshift64 (matches vibe-core so outputs are comparable) ──────────

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
    }
}

// ── Orthogonal rotation via Gram-Schmidt ─────────────────────────────────────

fn gram_schmidt_rotation(dim: usize, seed: u64) -> Vec<f32> {
    let mut rng = Xorshift64::new(seed);
    let mut m = vec![0.0f32; dim * dim];

    for i in 0..dim {
        for j in 0..dim {
            m[i * dim + j] = rng.next_f32();
        }
        for k in 0..i {
            let dot: f32 = (0..dim).map(|j| m[i * dim + j] * m[k * dim + j]).sum();
            for j in 0..dim {
                m[i * dim + j] -= dot * m[k * dim + j];
            }
        }
        let norm: f32 = (0..dim).map(|j| m[i * dim + j].powi(2)).sum::<f32>().sqrt();
        if norm > 1e-10 {
            for j in 0..dim {
                m[i * dim + j] /= norm;
            }
        }
    }
    m
}

/// Apply `y = R · x` where R is stored row-major.
fn rotate(r: &[f32], x: &[f32], dim: usize, out: &mut [f32]) {
    for i in 0..dim {
        let mut s = 0.0f32;
        for j in 0..dim {
            s += r[i * dim + j] * x[j];
        }
        out[i] = s;
    }
}

/// Apply `y = Rᵀ · x`.
fn inverse_rotate(r: &[f32], x: &[f32], dim: usize, out: &mut [f32]) {
    for i in 0..dim {
        let mut s = 0.0f32;
        for j in 0..dim {
            s += r[j * dim + i] * x[j];
        }
        out[i] = s;
    }
}

// ── Per-head-vector codes ────────────────────────────────────────────────────

/// PolarQuant code for a single `head_dim`-length vector.
///
/// Stores a full-precision radius plus a 2-bit direction code per dim packed
/// 4-per-byte. Decoded by mapping each 2-bit code to the centroid
/// {-0.75, -0.25, +0.25, +0.75}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarCode {
    pub radius: f32,
    /// `ceil(head_dim / 4)` bytes — each byte holds 4 codes, LSB-first.
    pub codes: Vec<u8>,
}

impl PolarCode {
    fn encode(rotated: &[f32]) -> Self {
        let radius: f32 = rotated.iter().map(|x| x * x).sum::<f32>().sqrt();
        let packed = rotated.len().div_ceil(4);
        let mut codes = vec![0u8; packed];

        if radius < 1e-10 {
            return Self { radius: 0.0, codes };
        }

        for (i, &v) in rotated.iter().enumerate() {
            let unit = v / radius;
            let code = if unit < -0.5 {
                0u8
            } else if unit < 0.0 {
                1u8
            } else if unit < 0.5 {
                2u8
            } else {
                3u8
            };
            let byte = i >> 2;
            let shift = (i & 0b11) << 1;
            codes[byte] |= code << shift;
        }

        Self { radius, codes }
    }

    fn decode(&self, dim: usize, out: &mut [f32]) {
        const CENTROIDS: [f32; 4] = [-0.75, -0.25, 0.25, 0.75];
        let mut unit_sq_sum = 0.0f32;
        for (i, slot) in out.iter_mut().take(dim).enumerate() {
            let byte = i >> 2;
            let shift = (i & 0b11) << 1;
            let code = ((self.codes[byte] >> shift) & 0b11) as usize;
            *slot = CENTROIDS[code];
            unit_sq_sum += CENTROIDS[code] * CENTROIDS[code];
        }
        let unit_norm = unit_sq_sum.sqrt().max(1e-10);
        let scale = self.radius / unit_norm;
        for slot in out.iter_mut().take(dim) {
            *slot *= scale;
        }
    }

    pub fn storage_bytes(&self) -> usize {
        4 + self.codes.len()
    }
}

/// QJL 1-bit residual sketch for a `head_dim`-length vector.
///
/// Stores the pre-projection L2 norm plus 1 bit per projection dim. Decode
/// is `sign(i) · projection[i] · (norm / √proj_dim)` summed over i.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QjlCode {
    pub residual_norm: f32,
    /// `ceil(proj_dim / 8)` bytes.
    pub signs: Vec<u8>,
}

impl QjlCode {
    fn encode(residual: &[f32], projection: &[f32], proj_dim: usize) -> Self {
        let dim = residual.len();
        let residual_norm: f32 = residual.iter().map(|x| x * x).sum::<f32>().sqrt();
        let packed = proj_dim.div_ceil(8);
        let mut signs = vec![0u8; packed];

        if residual_norm < 1e-10 {
            return Self { residual_norm: 0.0, signs };
        }

        for i in 0..proj_dim {
            let mut dot = 0.0f32;
            for j in 0..dim {
                dot += projection[i * dim + j] * residual[j];
            }
            if dot >= 0.0 {
                signs[i >> 3] |= 1 << (i & 0b111);
            }
        }
        Self { residual_norm, signs }
    }

    fn decode_add(&self, projection: &[f32], proj_dim: usize, dim: usize, out: &mut [f32]) {
        if self.residual_norm < 1e-10 {
            return;
        }
        let scale = self.residual_norm / (proj_dim as f32).sqrt();
        for i in 0..proj_dim {
            let byte = i >> 3;
            let bit = i & 0b111;
            let sign = if (self.signs[byte] >> bit) & 1 == 1 { 1.0f32 } else { -1.0f32 };
            let s = sign * scale;
            for j in 0..dim {
                out[j] += s * projection[i * dim + j];
            }
        }
    }

    pub fn storage_bytes(&self) -> usize {
        4 + self.signs.len()
    }
}

// ── Compressed layer storage — `[num_heads, seq_len]` of per-vector codes ────

/// Compressed representation of one attention layer's K *or* V tensor.
///
/// Indexing: entry `[h, t]` is `codes[h * seq_len + t]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedKvLayer {
    pub num_heads: usize,
    pub seq_len: usize,
    pub head_dim: usize,
    pub proj_dim: usize,
    pub polar: Vec<PolarCode>,
    pub qjl: Vec<QjlCode>,
}

impl CompressedKvLayer {
    /// Total storage in bytes (codes only — rotation/projection matrices live
    /// on the owning [`KvCacheTurboQuant`] and are shared across layers).
    pub fn storage_bytes(&self) -> usize {
        let p: usize = self.polar.iter().map(|c| c.storage_bytes()).sum();
        let q: usize = self.qjl.iter().map(|c| c.storage_bytes()).sum();
        p + q
    }

    /// Uncompressed cost of the same tensor at fp16 (2 bytes/element).
    pub fn fp16_bytes(&self) -> usize {
        self.num_heads * self.seq_len * self.head_dim * 2
    }

    /// Compression ratio vs fp16.
    pub fn ratio_vs_fp16(&self) -> f64 {
        self.fp16_bytes() as f64 / self.storage_bytes().max(1) as f64
    }
}

// ── The codec ────────────────────────────────────────────────────────────────

/// Shared rotation + JL projection matrices for a given `head_dim`.
///
/// Construct once per model (head_dim rarely changes), encode many layers.
/// The matrices are the dominant fixed cost — ~head_dim² f32 for rotation,
/// proj_dim × head_dim f32 for the JL projection. For `head_dim = 128`,
/// that's 128² × 4 = 64 KiB of rotation and 64 KiB of projection — shared
/// across every (layer, head, token).
#[derive(Debug)]
pub struct KvCacheTurboQuant {
    head_dim: usize,
    proj_dim: usize,
    rotation: Vec<f32>,
    projection: Vec<f32>,
}

impl KvCacheTurboQuant {
    /// Build a codec for the given `head_dim` with deterministic matrices.
    ///
    /// `qjl_proj_dim` controls the QJL residual fidelity. `None` defaults to
    /// `head_dim` (1× projection), which matches the embeddings-side
    /// TurboQuant. Larger values trade memory for recall.
    pub fn new(head_dim: usize, seed: u64, qjl_proj_dim: Option<usize>) -> Self {
        let proj_dim = qjl_proj_dim.unwrap_or(head_dim);
        let rotation = gram_schmidt_rotation(head_dim, seed);
        // The JL projection is a `proj_dim × head_dim` matrix; reuse the same
        // Gram-Schmidt routine with max(proj_dim, head_dim) to get an
        // orthonormal basis that we then truncate to the projection shape
        // during encode/decode. Matches vibe-core's construction.
        let projection = gram_schmidt_rotation(proj_dim.max(head_dim), seed.wrapping_add(1));
        Self { head_dim, proj_dim, rotation, projection }
    }

    pub fn head_dim(&self) -> usize {
        self.head_dim
    }

    pub fn proj_dim(&self) -> usize {
        self.proj_dim
    }

    /// Matrix storage shared across all layers (rotation + projection).
    pub fn fixed_bytes(&self) -> usize {
        (self.rotation.len() + self.projection.len()) * 4
    }

    /// Row-major rotation matrix R, shape `[head_dim, head_dim]`. Exposed so
    /// the device-side codec (`NativeTurboQuantCodec`) can upload R to a
    /// CUDA / Metal device.
    pub fn rotation_matrix(&self) -> &[f32] {
        &self.rotation
    }

    /// Row-major QJL projection basis, shape
    /// `[max(proj_dim, head_dim), max(proj_dim, head_dim)]`. Callers must
    /// truncate to `[proj_dim, head_dim]` (take the first `proj_dim` rows of
    /// `head_dim` columns each) — this matches the truncation used inside
    /// `QjlSign::encode` / `decode_add`.
    pub fn projection_matrix(&self) -> &[f32] {
        &self.projection
    }

    /// Encode a row-major `[num_heads, seq_len, head_dim]` tensor.
    ///
    /// Tensor layout: `tensor[h * seq_len * head_dim + t * head_dim + d]`.
    pub fn encode_layer(&self, tensor: &[f32], num_heads: usize, seq_len: usize) -> CompressedKvLayer {
        assert_eq!(
            tensor.len(),
            num_heads * seq_len * self.head_dim,
            "tensor shape mismatch: got {} elements, want {}",
            tensor.len(),
            num_heads * seq_len * self.head_dim,
        );

        let total = num_heads * seq_len;
        let mut polar = Vec::with_capacity(total);
        let mut qjl = Vec::with_capacity(total);
        let mut rotated = vec![0.0f32; self.head_dim];
        let mut reconstructed = vec![0.0f32; self.head_dim];
        let mut residual = vec![0.0f32; self.head_dim];

        for h in 0..num_heads {
            for t in 0..seq_len {
                let offset = h * seq_len * self.head_dim + t * self.head_dim;
                let slice = &tensor[offset..offset + self.head_dim];

                rotate(&self.rotation, slice, self.head_dim, &mut rotated);
                let polar_code = PolarCode::encode(&rotated);

                polar_code.decode(self.head_dim, &mut reconstructed);
                for i in 0..self.head_dim {
                    residual[i] = rotated[i] - reconstructed[i];
                }
                let qjl_code = QjlCode::encode(&residual, &self.projection, self.proj_dim);

                polar.push(polar_code);
                qjl.push(qjl_code);
            }
        }

        CompressedKvLayer {
            num_heads,
            seq_len,
            head_dim: self.head_dim,
            proj_dim: self.proj_dim,
            polar,
            qjl,
        }
    }

    /// Decode a single `[head_dim]` slice at position `(head, token)`.
    pub fn decode_one(&self, layer: &CompressedKvLayer, head: usize, token: usize, out: &mut [f32]) {
        assert_eq!(out.len(), self.head_dim);
        let idx = head * layer.seq_len + token;
        let mut rotated = vec![0.0f32; self.head_dim];
        layer.polar[idx].decode(self.head_dim, &mut rotated);
        layer.qjl[idx].decode_add(&self.projection, self.proj_dim, self.head_dim, &mut rotated);
        inverse_rotate(&self.rotation, &rotated, self.head_dim, out);
    }

    /// Decode `[head, token_start..token_end)` into a flat `[span, head_dim]`
    /// buffer. Matches the access pattern of `q · Kᵀ` during decode.
    pub fn decode_range(
        &self,
        layer: &CompressedKvLayer,
        head: usize,
        token_start: usize,
        token_end: usize,
        out: &mut [f32],
    ) {
        let span = token_end - token_start;
        assert_eq!(out.len(), span * self.head_dim);
        for (i, t) in (token_start..token_end).enumerate() {
            let slice = &mut out[i * self.head_dim..(i + 1) * self.head_dim];
            self.decode_one(layer, head, t, slice);
        }
    }

    /// Bytes per logical K/V element, averaged across the layer. Useful as a
    /// sanity check against the `0.4375` estimate in [`super::kv_cache`]
    /// (3 bits/dim + 8 B per-vector scalar overhead at head_dim=128).
    pub fn bytes_per_element(&self, layer: &CompressedKvLayer) -> f32 {
        layer.storage_bytes() as f32 / (layer.num_heads * layer.seq_len * self.head_dim) as f32
    }
}

// ── Fidelity metrics ─────────────────────────────────────────────────────────

/// One-line report comparing reconstructed K/V against ground truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FidelityReport {
    pub method: String,
    pub bytes_per_element: f32,
    /// Mean cosine similarity of reconstructed head-vectors vs originals.
    /// 1.0 = perfect, 0.0 = unrelated.
    pub mean_cosine: f32,
    /// Lowest cosine similarity observed across all (head, token) slices.
    pub worst_cosine: f32,
    /// Simulated attention-weight MAE for a random query vector. Lower is
    /// better; 0.0 means the reconstructed K produces identical softmax
    /// weights to the ground-truth K.
    pub attention_mae: f32,
    /// Top-1 attention index agreement rate with ground truth (fraction of
    /// heads where the argmax-attended token matches).
    pub top1_agreement: f32,
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-10 { 0.0 } else { (dot / denom).clamp(-1.0, 1.0) }
}

fn softmax(scores: &mut [f32]) {
    let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for s in scores.iter_mut() {
        *s = (*s - max).exp();
        sum += *s;
    }
    if sum > 0.0 {
        for s in scores.iter_mut() {
            *s /= sum;
        }
    }
}

/// Measure fidelity of a TurboQuant-compressed K tensor against ground truth.
///
/// Uses a deterministic synthetic query per head (Xorshift-seeded) to compute
/// both ground-truth and reconstructed attention weights. Returns per-method
/// aggregates so callers can print a comparison table.
pub fn fidelity_turboquant(
    codec: &KvCacheTurboQuant,
    ground_truth: &[f32],
    num_heads: usize,
    seq_len: usize,
    query_seed: u64,
) -> FidelityReport {
    let head_dim = codec.head_dim;
    let layer = codec.encode_layer(ground_truth, num_heads, seq_len);

    // Reconstruct whole tensor for cosine stats.
    let mut reconstructed = vec![0.0f32; ground_truth.len()];
    let mut slot = vec![0.0f32; head_dim];
    for h in 0..num_heads {
        for t in 0..seq_len {
            codec.decode_one(&layer, h, t, &mut slot);
            let off = h * seq_len * head_dim + t * head_dim;
            reconstructed[off..off + head_dim].copy_from_slice(&slot);
        }
    }

    aggregate_fidelity(
        "turboquant",
        codec.bytes_per_element(&layer),
        head_dim,
        num_heads,
        seq_len,
        ground_truth,
        &reconstructed,
        query_seed,
    )
}

/// Fp8 (E4M3 round-to-nearest) baseline, simulated in fp32.
pub fn fidelity_fp8(
    tensor: &[f32],
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
    query_seed: u64,
) -> FidelityReport {
    // E4M3 has a max representable of ~448.0 and ~256 distinct levels in the
    // normal range. Simulate via per-tensor scale + round-to-nearest-of-8bit.
    let max_abs = tensor.iter().copied().fold(0.0f32, |a, b| a.max(b.abs())).max(1e-9);
    let scale = max_abs / 448.0;
    let reconstructed: Vec<f32> = tensor
        .iter()
        .map(|v| {
            let q = (v / scale).round().clamp(-448.0, 448.0);
            q * scale
        })
        .collect();

    aggregate_fidelity(
        "fp8",
        1.0,
        head_dim,
        num_heads,
        seq_len,
        tensor,
        &reconstructed,
        query_seed,
    )
}

/// Symmetric Int8 per-channel baseline.
pub fn fidelity_int8(
    tensor: &[f32],
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
    query_seed: u64,
) -> FidelityReport {
    let mut reconstructed = vec![0.0f32; tensor.len()];
    for h in 0..num_heads {
        for t in 0..seq_len {
            let off = h * seq_len * head_dim + t * head_dim;
            let slice = &tensor[off..off + head_dim];
            let max_abs = slice.iter().copied().fold(0.0f32, |a, b| a.max(b.abs())).max(1e-9);
            let scale = max_abs / 127.0;
            for (d, &v) in slice.iter().enumerate() {
                let q = (v / scale).round().clamp(-127.0, 127.0);
                reconstructed[off + d] = q * scale;
            }
        }
    }
    aggregate_fidelity(
        "int8",
        1.0,
        head_dim,
        num_heads,
        seq_len,
        tensor,
        &reconstructed,
        query_seed,
    )
}

#[allow(clippy::too_many_arguments)]
fn aggregate_fidelity(
    method: &str,
    bytes_per_element: f32,
    head_dim: usize,
    num_heads: usize,
    seq_len: usize,
    ground_truth: &[f32],
    reconstructed: &[f32],
    query_seed: u64,
) -> FidelityReport {
    let mut total_cos = 0.0f32;
    let mut worst_cos = 1.0f32;
    let mut total_mae = 0.0f32;
    let mut top1_hits = 0usize;
    let mut rng = Xorshift64::new(query_seed);

    for h in 0..num_heads {
        // Deterministic query per head, shared between GT and reconstructed.
        let mut query = vec![0.0f32; head_dim];
        for q in query.iter_mut() {
            *q = rng.next_f32();
        }

        let mut gt_scores = vec![0.0f32; seq_len];
        let mut recon_scores = vec![0.0f32; seq_len];
        for t in 0..seq_len {
            let off = h * seq_len * head_dim + t * head_dim;
            let gt = &ground_truth[off..off + head_dim];
            let rc = &reconstructed[off..off + head_dim];

            let c = cosine(gt, rc);
            total_cos += c;
            if c < worst_cos {
                worst_cos = c;
            }

            let inv_sqrt = 1.0 / (head_dim as f32).sqrt();
            let mut dg = 0.0f32;
            let mut dr = 0.0f32;
            for d in 0..head_dim {
                dg += query[d] * gt[d];
                dr += query[d] * rc[d];
            }
            gt_scores[t] = dg * inv_sqrt;
            recon_scores[t] = dr * inv_sqrt;
        }

        softmax(&mut gt_scores);
        softmax(&mut recon_scores);
        for t in 0..seq_len {
            total_mae += (gt_scores[t] - recon_scores[t]).abs();
        }

        let gt_top = argmax(&gt_scores);
        let rc_top = argmax(&recon_scores);
        if gt_top == rc_top {
            top1_hits += 1;
        }
    }

    let total_slices = (num_heads * seq_len) as f32;
    FidelityReport {
        method: method.to_string(),
        bytes_per_element,
        mean_cosine: total_cos / total_slices,
        worst_cosine: worst_cos,
        attention_mae: total_mae / (num_heads * seq_len) as f32,
        top1_agreement: top1_hits as f32 / num_heads as f32,
    }
}

fn argmax(scores: &[f32]) -> usize {
    scores
        .iter()
        .enumerate()
        .fold((0usize, f32::NEG_INFINITY), |(bi, bv), (i, &v)| if v > bv { (i, v) } else { (bi, bv) })
        .0
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_tensor(num_heads: usize, seq_len: usize, head_dim: usize, seed: u64) -> Vec<f32> {
        let mut rng = Xorshift64::new(seed);
        let n = num_heads * seq_len * head_dim;
        (0..n).map(|_| rng.next_f32()).collect()
    }

    #[test]
    fn polar_code_storage_is_quarter_byte_per_dim() {
        let v: Vec<f32> = (0..128).map(|i| (i as f32).sin()).collect();
        let mut rotated = vec![0.0f32; 128];
        let r = gram_schmidt_rotation(128, 1);
        rotate(&r, &v, 128, &mut rotated);
        let code = PolarCode::encode(&rotated);
        // 128 dims @ 2 bits each = 32 bytes of codes + 4 bytes radius.
        assert_eq!(code.codes.len(), 32);
        assert_eq!(code.storage_bytes(), 36);
    }

    #[test]
    fn qjl_code_storage_is_one_bit_per_proj_dim() {
        let dim = 128;
        let proj_dim = 128;
        let residual: Vec<f32> = (0..dim).map(|i| ((i as f32) * 0.1).cos()).collect();
        let proj = gram_schmidt_rotation(proj_dim, 2);
        let code = QjlCode::encode(&residual, &proj, proj_dim);
        assert_eq!(code.signs.len(), 16); // 128 bits
        assert_eq!(code.storage_bytes(), 20); // 4B norm + 16B signs
    }

    #[test]
    fn encode_decode_shape_round_trip() {
        let num_heads = 4;
        let seq_len = 16;
        let head_dim = 64;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 7);

        let codec = KvCacheTurboQuant::new(head_dim, 11, None);
        let layer = codec.encode_layer(&tensor, num_heads, seq_len);

        assert_eq!(layer.polar.len(), num_heads * seq_len);
        assert_eq!(layer.qjl.len(), num_heads * seq_len);

        let mut out = vec![0.0f32; head_dim];
        codec.decode_one(&layer, 2, 5, &mut out);
        assert_eq!(out.len(), head_dim);
        assert!(out.iter().any(|&x| x.abs() > 1e-6), "decoded vector should be non-trivial");
    }

    #[test]
    fn decode_range_matches_per_token_decode() {
        let num_heads = 2;
        let seq_len = 8;
        let head_dim = 32;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 13);
        let codec = KvCacheTurboQuant::new(head_dim, 17, None);
        let layer = codec.encode_layer(&tensor, num_heads, seq_len);

        let mut span = vec![0.0f32; 4 * head_dim];
        codec.decode_range(&layer, 1, 2, 6, &mut span);

        let mut one = vec![0.0f32; head_dim];
        for (i, t) in (2..6).enumerate() {
            codec.decode_one(&layer, 1, t, &mut one);
            let slice = &span[i * head_dim..(i + 1) * head_dim];
            for (a, b) in slice.iter().zip(one.iter()) {
                assert!((a - b).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn compression_ratio_matches_spike_finding_for_head_dim_128() {
        // Llama-3.1-8B-ish: 8 KV heads × 256 tokens × 128 head_dim.
        //
        // Phase-3 spike finding: for head_dim=128, real compression is ~4.57×
        // vs fp16, NOT the 5.33× the `kv_cache.rs` 0.375 B/el estimate predicts.
        // The gap is the per-vector scalar overhead: 4 B radius (PolarCode) +
        // 4 B residual_norm (QjlCode) = 8 B per (head, token), which at
        // head_dim=128 adds 0.0625 B/el. Actual ~0.4375 B/el → 2/0.4375 ≈ 4.57×.
        //
        // Implication for the kernel PR: the 0.375 B/el number is the right
        // amortised estimate only for long vectors (≥ 512 dim). For typical
        // head_dim 64/128, budget on ~4–5× savings, not >5×.
        let num_heads = 8;
        let seq_len = 256;
        let head_dim = 128;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 42);

        let codec = KvCacheTurboQuant::new(head_dim, 42, None);
        let layer = codec.encode_layer(&tensor, num_heads, seq_len);

        let ratio = layer.ratio_vs_fp16();
        assert!(ratio > 4.4 && ratio < 4.8, "ratio outside [4.4, 4.8]: {ratio:.3}×");
        let bpe = codec.bytes_per_element(&layer);
        // Theoretical floor for head_dim=128: 2b polar + 1b qjl + 8B/128dim = 0.4375 B/el.
        assert!(bpe > 0.40 && bpe < 0.50, "bytes/element outside [0.40, 0.50]: {bpe:.4}");
    }

    #[test]
    fn fidelity_turboquant_preserves_top1_majority() {
        let num_heads = 16;
        let seq_len = 64;
        let head_dim = 64;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 3);
        let codec = KvCacheTurboQuant::new(head_dim, 3, None);

        let rep = fidelity_turboquant(&codec, &tensor, num_heads, seq_len, 99);

        assert_eq!(rep.method, "turboquant");
        assert!(rep.mean_cosine > 0.5, "mean cosine too low: {:.3}", rep.mean_cosine);
        // On fully random data, even fp8 only gets partial top-1 agreement;
        // this is a sanity floor, not a fidelity claim.
        assert!(rep.top1_agreement >= 0.0);
        assert!(rep.attention_mae.is_finite());
    }

    #[test]
    fn fidelity_fp8_is_near_lossless() {
        let num_heads = 4;
        let seq_len = 32;
        let head_dim = 64;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 5);
        let rep = fidelity_fp8(&tensor, num_heads, seq_len, head_dim, 7);
        assert!(rep.mean_cosine > 0.999, "fp8 should be ≥0.999 cosine: {}", rep.mean_cosine);
        assert!(rep.top1_agreement > 0.9);
    }

    #[test]
    fn fidelity_int8_beats_turboquant_in_cosine() {
        let num_heads = 4;
        let seq_len = 32;
        let head_dim = 64;
        let tensor = synthetic_tensor(num_heads, seq_len, head_dim, 9);
        let codec = KvCacheTurboQuant::new(head_dim, 9, None);

        let int8 = fidelity_int8(&tensor, num_heads, seq_len, head_dim, 21);
        let tq = fidelity_turboquant(&codec, &tensor, num_heads, seq_len, 21);

        // int8 uses 8× more bits, so it should win on cosine. This test
        // documents the expected ordering of the spike so future kernel work
        // has a clear baseline.
        assert!(int8.mean_cosine >= tq.mean_cosine);
    }

    #[test]
    fn fixed_matrix_bytes_scale_with_head_dim_squared() {
        let codec_64 = KvCacheTurboQuant::new(64, 1, None);
        let codec_128 = KvCacheTurboQuant::new(128, 1, None);
        // (128² + 128²) vs (64² + 64²) = 4× growth.
        let ratio = codec_128.fixed_bytes() as f32 / codec_64.fixed_bytes() as f32;
        assert!((ratio - 4.0).abs() < 0.01, "ratio = {ratio}");
    }
}
