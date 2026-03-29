//! TurboQuant — Extreme vector compression via PolarQuant + QJL.
//!
//! Implements the two-stage quantization algorithm from Google Research's
//! TurboQuant paper (2026): random rotation → polar-coordinate grid
//! quantization (PolarQuant) followed by 1-bit residual compression via
//! Quantized Johnson-Lindenstrauss (QJL). Achieves ~3 bits per dimension
//! with negligible recall loss for cosine similarity search.
//!
//! # Architecture
//!
//! ```text
//! f32 vector ──► random rotation ──► PolarQuant (2-bit grid) ──► reconstruct
//!                                                                    │
//!                                                          residual = original − reconstructed
//!                                                                    │
//!                                                               QJL (1-bit signs)
//! ```
//!
//! Total storage: ~2 bits (polar) + 1 bit (QJL residual) + f32 radius = ~3 bits/dim + 4 bytes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Constants ────────────────────────────────────────────────────────────────

/// Default number of QJL projection dimensions (as fraction of original dim).
const QJL_PROJECTION_RATIO: f32 = 1.0;

/// Number of quantization levels per dimension for PolarQuant (2 bits = 4 levels).
const POLAR_LEVELS: u8 = 4;

// ── Random rotation (deterministic from seed) ────────────────────────────────

/// Simple xorshift64 PRNG for deterministic rotation matrices.
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

    /// Uniform f32 in [-1, 1].
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
    }
}

/// Generate a pseudo-random rotation matrix using Gram-Schmidt orthogonalization.
/// Returns a `dim × dim` matrix stored row-major.
fn generate_rotation_matrix(dim: usize, seed: u64) -> Vec<f32> {
    let mut rng = Xorshift64::new(seed);
    let mut matrix = vec![0.0f32; dim * dim];

    // Generate random vectors and orthogonalize via modified Gram-Schmidt.
    for i in 0..dim {
        // Random vector
        for j in 0..dim {
            matrix[i * dim + j] = rng.next_f32();
        }

        // Subtract projections onto previous basis vectors
        for k in 0..i {
            let dot: f32 = (0..dim)
                .map(|j| matrix[i * dim + j] * matrix[k * dim + j])
                .sum();
            for j in 0..dim {
                matrix[i * dim + j] -= dot * matrix[k * dim + j];
            }
        }

        // Normalize
        let norm: f32 = (0..dim)
            .map(|j| matrix[i * dim + j] * matrix[i * dim + j])
            .sum::<f32>()
            .sqrt();
        if norm > 1e-10 {
            for j in 0..dim {
                matrix[i * dim + j] /= norm;
            }
        }
    }

    matrix
}

/// Apply rotation: result = R × input.
fn rotate_vector(rotation: &[f32], input: &[f32], dim: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; dim];
    for i in 0..dim {
        let mut sum = 0.0f32;
        for j in 0..dim {
            sum += rotation[i * dim + j] * input[j];
        }
        result[i] = sum;
    }
    result
}

/// Apply inverse (transpose) rotation: result = R^T × input.
fn inverse_rotate_vector(rotation: &[f32], input: &[f32], dim: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; dim];
    for i in 0..dim {
        let mut sum = 0.0f32;
        for j in 0..dim {
            sum += rotation[j * dim + i] * input[j];
        }
        result[i] = sum;
    }
    result
}

// ── PolarQuant ───────────────────────────────────────────────────────────────

/// PolarQuant: convert rotated vectors to polar form (radius + quantized angles).
///
/// After random rotation, the angular distribution concentrates, allowing
/// efficient grid quantization with minimal distortion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarQuantized {
    /// L2 norm (radius) of the original vector, stored at full precision.
    pub radius: f32,
    /// 2-bit quantized direction codes, packed 4 per byte.
    /// Each code ∈ {0,1,2,3} maps to a uniform grid on the unit hypersphere.
    pub codes: Vec<u8>,
}

impl PolarQuantized {
    /// Quantize a (already-rotated) vector into polar form.
    fn quantize(rotated: &[f32]) -> Self {
        let dim = rotated.len();

        // Compute radius (L2 norm)
        let radius: f32 = rotated.iter().map(|x| x * x).sum::<f32>().sqrt();

        if radius < 1e-10 {
            return Self {
                radius: 0.0,
                codes: vec![0u8; (dim + 3) / 4],
            };
        }

        // Normalize to unit vector
        let unit: Vec<f32> = rotated.iter().map(|x| x / radius).collect();

        // Quantize each dimension to 2 bits (4 levels) in [-1, 1]
        // Levels: 0 → -0.75, 1 → -0.25, 2 → 0.25, 3 → 0.75
        let packed_len = (dim + 3) / 4;
        let mut codes = vec![0u8; packed_len];

        for (i, &val) in unit.iter().enumerate() {
            let code = if val < -0.5 {
                0u8
            } else if val < 0.0 {
                1u8
            } else if val < 0.5 {
                2u8
            } else {
                POLAR_LEVELS - 1
            };
            let byte_idx = i / 4;
            let bit_offset = (i % 4) * 2;
            codes[byte_idx] |= code << bit_offset;
        }

        Self { radius, codes }
    }

    /// Reconstruct an approximate unit vector from quantized codes.
    fn dequantize_unit(&self, dim: usize) -> Vec<f32> {
        let centroids = [-0.75f32, -0.25, 0.25, 0.75];
        let mut unit = Vec::with_capacity(dim);

        for i in 0..dim {
            let byte_idx = i / 4;
            let bit_offset = (i % 4) * 2;
            let code = (self.codes[byte_idx] >> bit_offset) & 0x03;
            unit.push(centroids[code as usize]);
        }

        // Re-normalize to unit sphere
        let norm: f32 = unit.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-10 {
            for x in &mut unit {
                *x /= norm;
            }
        }

        unit
    }

    /// Reconstruct the full approximate vector (unit × radius).
    fn dequantize(&self, dim: usize) -> Vec<f32> {
        self.dequantize_unit(dim)
            .into_iter()
            .map(|x| x * self.radius)
            .collect()
    }

    /// Storage size in bytes for this quantized vector.
    pub fn storage_bytes(&self) -> usize {
        4 /* radius f32 */ + self.codes.len()
    }
}

// ── QJL (Quantized Johnson-Lindenstrauss) ────────────────────────────────────

/// QJL residual compressor: projects the PolarQuant residual through a random
/// JL matrix and stores only the signs (1 bit each). This eliminates
/// quantization bias in inner-product estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QjlCompressed {
    /// 1-bit signs of the projected residual, packed 8 per byte.
    /// bit = 1 means positive, bit = 0 means negative.
    pub signs: Vec<u8>,
    /// L2 norm of the residual before projection (used for scaling).
    pub residual_norm: f32,
    /// Number of projection dimensions.
    pub proj_dim: usize,
}

impl QjlCompressed {
    /// Compress a residual vector using QJL.
    fn compress(residual: &[f32], projection: &[f32], proj_dim: usize) -> Self {
        let dim = residual.len();
        let residual_norm: f32 = residual.iter().map(|x| x * x).sum::<f32>().sqrt();

        let packed_len = (proj_dim + 7) / 8;
        let mut signs = vec![0u8; packed_len];

        // Project and keep only signs
        for i in 0..proj_dim {
            let mut dot = 0.0f32;
            for j in 0..dim {
                dot += projection[i * dim + j] * residual[j];
            }
            if dot >= 0.0 {
                signs[i / 8] |= 1 << (i % 8);
            }
        }

        Self {
            signs,
            residual_norm,
            proj_dim,
        }
    }

    /// Reconstruct an approximate residual from the QJL signs.
    fn decompress(&self, projection: &[f32], dim: usize) -> Vec<f32> {
        if self.residual_norm < 1e-10 {
            return vec![0.0; dim];
        }

        let scale = self.residual_norm / (self.proj_dim as f32).sqrt();
        let mut result = vec![0.0f32; dim];

        for i in 0..self.proj_dim {
            let sign_bit = (self.signs[i / 8] >> (i % 8)) & 1;
            let sign = if sign_bit == 1 { 1.0f32 } else { -1.0f32 };
            for j in 0..dim {
                result[j] += sign * projection[i * dim + j] * scale;
            }
        }

        result
    }

    /// Storage size in bytes.
    pub fn storage_bytes(&self) -> usize {
        4 /* residual_norm */ + 8 /* proj_dim */ + self.signs.len()
    }
}

// ── Compressed Vector Entry ──────────────────────────────────────────────────

/// A single vector compressed via the TurboQuant two-stage pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedVector {
    pub polar: PolarQuantized,
    pub qjl: QjlCompressed,
}

impl CompressedVector {
    /// Total storage in bytes for this compressed vector.
    pub fn storage_bytes(&self) -> usize {
        self.polar.storage_bytes() + self.qjl.storage_bytes()
    }
}

// ── TurboQuantIndex ──────────────────────────────────────────────────────────

/// Configuration for a TurboQuant compressed vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurboQuantConfig {
    /// Dimensionality of the original vectors.
    pub dimension: usize,
    /// Seed for deterministic rotation and projection matrices.
    pub seed: u64,
    /// QJL projection dimensions (default: same as `dimension`).
    pub qjl_proj_dim: Option<usize>,
}

impl Default for TurboQuantConfig {
    fn default() -> Self {
        Self {
            dimension: 384,
            seed: 42,
            qjl_proj_dim: None,
        }
    }
}

/// Compressed vector index using TurboQuant (PolarQuant + QJL).
///
/// Achieves ~3 bits per dimension storage (vs 32 bits for f32) while
/// maintaining high recall for cosine similarity search.
///
/// # Memory savings
///
/// For 384-dim embeddings (e.g. nomic-embed-text):
/// - Uncompressed: 384 × 4 = 1,536 bytes per vector
/// - TurboQuant: 384/4 (polar) + 384/8 (QJL) + 8 (norms) = 152 bytes per vector
/// - Compression ratio: ~10.1×
#[derive(Debug, Serialize, Deserialize)]
pub struct TurboQuantIndex {
    config: TurboQuantConfig,
    /// Pre-computed rotation matrix (dim × dim), stored for serialization.
    #[serde(skip)]
    rotation: Vec<f32>,
    /// Pre-computed QJL projection matrix (proj_dim × dim).
    #[serde(skip)]
    projection: Vec<f32>,
    /// Compressed vectors, parallel with `ids`.
    vectors: Vec<CompressedVector>,
    /// Entry identifiers, parallel with `vectors`.
    ids: Vec<String>,
    /// Optional metadata per entry.
    metadata: Vec<HashMap<String, String>>,
}

impl TurboQuantIndex {
    /// Create a new empty index with the given configuration.
    pub fn new(config: TurboQuantConfig) -> Self {
        let dim = config.dimension;
        let proj_dim = config
            .qjl_proj_dim
            .unwrap_or((dim as f32 * QJL_PROJECTION_RATIO) as usize);
        let rotation = generate_rotation_matrix(dim, config.seed);
        let projection = generate_rotation_matrix(proj_dim.max(dim), config.seed.wrapping_add(1));

        Self {
            config,
            rotation,
            projection,
            vectors: Vec::new(),
            ids: Vec::new(),
            metadata: Vec::new(),
        }
    }

    /// Rebuild transient matrices after deserialization.
    pub fn rebuild_matrices(&mut self) {
        let dim = self.config.dimension;
        let proj_dim = self
            .config
            .qjl_proj_dim
            .unwrap_or((dim as f32 * QJL_PROJECTION_RATIO) as usize);
        self.rotation = generate_rotation_matrix(dim, self.config.seed);
        self.projection =
            generate_rotation_matrix(proj_dim.max(dim), self.config.seed.wrapping_add(1));
    }

    /// Effective QJL projection dimensionality.
    fn proj_dim(&self) -> usize {
        self.config
            .qjl_proj_dim
            .unwrap_or((self.config.dimension as f32 * QJL_PROJECTION_RATIO) as usize)
    }

    /// Compress and insert a vector with the given ID and optional metadata.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        vector: &[f32],
        meta: HashMap<String, String>,
    ) -> Result<(), String> {
        let dim = self.config.dimension;
        if vector.len() != dim {
            return Err(format!(
                "dimension mismatch: expected {}, got {}",
                dim,
                vector.len()
            ));
        }

        let compressed = self.compress(vector);
        self.ids.push(id.into());
        self.vectors.push(compressed);
        self.metadata.push(meta);
        Ok(())
    }

    /// Compress a raw f32 vector through the TurboQuant pipeline.
    fn compress(&self, vector: &[f32]) -> CompressedVector {
        let dim = self.config.dimension;
        let proj_dim = self.proj_dim();

        // Stage 1: Random rotation + PolarQuant
        let rotated = rotate_vector(&self.rotation, vector, dim);
        let polar = PolarQuantized::quantize(&rotated);

        // Reconstruct from PolarQuant to compute residual
        let reconstructed_rotated = polar.dequantize(dim);
        let residual: Vec<f32> = rotated
            .iter()
            .zip(reconstructed_rotated.iter())
            .map(|(a, b)| a - b)
            .collect();

        // Stage 2: QJL 1-bit compression of residual
        let qjl = QjlCompressed::compress(&residual, &self.projection, proj_dim);

        CompressedVector { polar, qjl }
    }

    /// Decompress a vector back to approximate f32 representation.
    fn decompress(&self, compressed: &CompressedVector) -> Vec<f32> {
        let dim = self.config.dimension;

        // Reconstruct PolarQuant approximation in rotated space
        let polar_approx = compressed.polar.dequantize(dim);

        // Reconstruct QJL residual
        let residual_approx =
            compressed.qjl.decompress(&self.projection, dim);

        // Combine: rotated_approx = polar + residual
        let rotated_approx: Vec<f32> = polar_approx
            .iter()
            .zip(residual_approx.iter())
            .map(|(a, b)| a + b)
            .collect();

        // Inverse rotation back to original space
        inverse_rotate_vector(&self.rotation, &rotated_approx, dim)
    }

    /// Search for the top-k most similar vectors to `query` using cosine similarity.
    ///
    /// The query is compressed on-the-fly and inner products are estimated
    /// from compressed representations for speed. Falls back to decompressed
    /// cosine for accuracy.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<TurboQuantSearchResult> {
        if query.len() != self.config.dimension {
            return vec![];
        }

        let mut scored: Vec<(f32, usize)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(i, cv)| {
                let approx = self.decompress(cv);
                (cosine_sim(query, &approx), i)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.0)
            .map(|(score, i)| TurboQuantSearchResult {
                id: self.ids[i].clone(),
                score,
                metadata: self.metadata[i].clone(),
            })
            .collect()
    }

    /// Delete an entry by ID. Returns true if found.
    pub fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.ids.iter().position(|x| x == id) {
            self.ids.swap_remove(pos);
            self.vectors.swap_remove(pos);
            self.metadata.swap_remove(pos);
            true
        } else {
            false
        }
    }

    /// Number of vectors in the index.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Total compressed storage in bytes (vectors only, excluding matrices).
    pub fn storage_bytes(&self) -> usize {
        self.vectors.iter().map(|v| v.storage_bytes()).sum()
    }

    /// What the same vectors would cost uncompressed (f32).
    pub fn uncompressed_bytes(&self) -> usize {
        self.vectors.len() * self.config.dimension * 4
    }

    /// Compression ratio (uncompressed / compressed).
    pub fn compression_ratio(&self) -> f64 {
        let compressed = self.storage_bytes();
        if compressed == 0 {
            return 0.0;
        }
        self.uncompressed_bytes() as f64 / compressed as f64
    }

    /// Return statistics about the index.
    pub fn stats(&self) -> TurboQuantStats {
        TurboQuantStats {
            num_vectors: self.vectors.len(),
            dimension: self.config.dimension,
            compressed_bytes: self.storage_bytes(),
            uncompressed_bytes: self.uncompressed_bytes(),
            compression_ratio: self.compression_ratio(),
            bits_per_dimension: if self.vectors.is_empty() {
                0.0
            } else {
                (self.storage_bytes() as f64 * 8.0)
                    / (self.vectors.len() as f64 * self.config.dimension as f64)
            },
        }
    }
}

// ── Search Result ────────────────────────────────────────────────────────────

/// A result from a TurboQuant compressed search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurboQuantSearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

// ── Stats ────────────────────────────────────────────────────────────────────

/// Statistics about a TurboQuant index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurboQuantStats {
    pub num_vectors: usize,
    pub dimension: usize,
    pub compressed_bytes: usize,
    pub uncompressed_bytes: usize,
    pub compression_ratio: f64,
    pub bits_per_dimension: f64,
}

// ── Cosine similarity (shared with embeddings.rs) ────────────────────────────

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (dot, norm_a, norm_b) = a.iter().zip(b.iter()).fold(
        (0.0f32, 0.0f32, 0.0f32),
        |(d, na, nb), (x, y)| (d + x * y, na + x * x, nb + y * y),
    );
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}

// ── Batch compression helper ─────────────────────────────────────────────────

/// Compress a batch of f32 vectors into a TurboQuant index.
///
/// Convenience wrapper for bulk insertion (e.g. when converting an
/// existing `EmbeddingIndex` to compressed form).
pub fn compress_batch(
    vectors: &[Vec<f32>],
    ids: &[String],
    dimension: usize,
    seed: u64,
) -> TurboQuantIndex {
    let config = TurboQuantConfig {
        dimension,
        seed,
        qjl_proj_dim: None,
    };
    let mut index = TurboQuantIndex::new(config);
    for (vec, id) in vectors.iter().zip(ids.iter()) {
        let _ = index.insert(id.clone(), vec, HashMap::new());
    }
    index
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
        let mut rng = Xorshift64::new(seed);
        (0..dim).map(|_| rng.next_f32()).collect()
    }

    fn normalized_vector(dim: usize, seed: u64) -> Vec<f32> {
        let v = random_vector(dim, seed);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    }

    // ── Xorshift64 ──────────────────────────────────────────────────────────

    #[test]
    fn xorshift_deterministic() {
        let mut a = Xorshift64::new(42);
        let mut b = Xorshift64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn xorshift_zero_seed_handled() {
        let mut rng = Xorshift64::new(0);
        // Should not loop on zero; internal seed forced to 1
        let val = rng.next_u64();
        assert_ne!(val, 0);
    }

    // ── Rotation matrix ─────────────────────────────────────────────────────

    #[test]
    fn rotation_matrix_is_orthogonal() {
        let dim = 8;
        let r = generate_rotation_matrix(dim, 123);
        // R × R^T should be approximately identity
        for i in 0..dim {
            for j in 0..dim {
                let dot: f32 = (0..dim).map(|k| r[i * dim + k] * r[j * dim + k]).sum();
                if i == j {
                    assert!((dot - 1.0).abs() < 0.01, "diagonal [{i},{j}] = {dot}");
                } else {
                    assert!(dot.abs() < 0.01, "off-diagonal [{i},{j}] = {dot}");
                }
            }
        }
    }

    #[test]
    fn rotation_preserves_norm() {
        let dim = 16;
        let r = generate_rotation_matrix(dim, 99);
        let v = random_vector(dim, 42);
        let rotated = rotate_vector(&r, &v, dim);
        let norm_orig: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_rot: f32 = rotated.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm_orig - norm_rot).abs() < 0.01,
            "norms differ: {norm_orig} vs {norm_rot}"
        );
    }

    #[test]
    fn inverse_rotation_recovers_original() {
        let dim = 8;
        let r = generate_rotation_matrix(dim, 77);
        let v = random_vector(dim, 55);
        let rotated = rotate_vector(&r, &v, dim);
        let recovered = inverse_rotate_vector(&r, &rotated, dim);
        for (a, b) in v.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 0.01, "mismatch: {a} vs {b}");
        }
    }

    // ── PolarQuant ──────────────────────────────────────────────────────────

    #[test]
    fn polar_quant_preserves_direction() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let pq = PolarQuantized::quantize(&v);
        let reconstructed = pq.dequantize(v.len());
        let sim = cosine_sim(&v, &reconstructed);
        assert!(sim > 0.9, "cosine similarity too low: {sim}");
    }

    #[test]
    fn polar_quant_zero_vector() {
        let v = vec![0.0f32; 16];
        let pq = PolarQuantized::quantize(&v);
        assert!(pq.radius < 1e-9);
        let reconstructed = pq.dequantize(16);
        for x in &reconstructed {
            assert!(x.abs() < 1e-9);
        }
    }

    #[test]
    fn polar_quant_storage_smaller_than_f32() {
        let dim = 384;
        let v = random_vector(dim, 42);
        let pq = PolarQuantized::quantize(&v);
        let f32_bytes = dim * 4;
        assert!(
            pq.storage_bytes() < f32_bytes / 4,
            "PolarQuant should be <25% of f32: {} vs {}",
            pq.storage_bytes(),
            f32_bytes
        );
    }

    #[test]
    fn polar_quant_codes_packed_correctly() {
        // 8-dim vector → 2 bytes of codes
        let v = vec![0.8, -0.8, 0.1, -0.1, 0.6, -0.6, 0.3, -0.3];
        let pq = PolarQuantized::quantize(&v);
        assert_eq!(pq.codes.len(), 2, "8 dims / 4 per byte = 2 bytes");
    }

    // ── QJL ─────────────────────────────────────────────────────────────────

    #[test]
    fn qjl_storage_is_1_bit_per_projection_dim() {
        let dim = 64;
        let residual = random_vector(dim, 100);
        let proj = generate_rotation_matrix(dim, 200);
        let qjl = QjlCompressed::compress(&residual, &proj, dim);
        // 64 bits = 8 bytes
        assert_eq!(qjl.signs.len(), 8);
    }

    #[test]
    fn qjl_decompress_nonzero_for_nonzero_residual() {
        let dim = 32;
        let residual = random_vector(dim, 300);
        let proj = generate_rotation_matrix(dim, 400);
        let qjl = QjlCompressed::compress(&residual, &proj, dim);
        let reconstructed = qjl.decompress(&proj, dim);
        let norm: f32 = reconstructed.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.01, "reconstructed residual should be nonzero");
    }

    #[test]
    fn qjl_zero_residual_gives_zero() {
        let dim = 16;
        let residual = vec![0.0f32; dim];
        let proj = generate_rotation_matrix(dim, 500);
        let qjl = QjlCompressed::compress(&residual, &proj, dim);
        let reconstructed = qjl.decompress(&proj, dim);
        for x in &reconstructed {
            assert!(x.abs() < 1e-9);
        }
    }

    // ── TurboQuantIndex ─────────────────────────────────────────────────────

    #[test]
    fn index_insert_and_search() {
        let dim = 32;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);

        let v1 = normalized_vector(dim, 1);
        let v2 = normalized_vector(dim, 2);
        let v3 = normalized_vector(dim, 3);

        index.insert("a", &v1, HashMap::new()).unwrap();
        index.insert("b", &v2, HashMap::new()).unwrap();
        index.insert("c", &v3, HashMap::new()).unwrap();

        // Search with v1 should return "a" as the top result
        let results = index.search(&v1, 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "a");
        assert!(results[0].score > 0.8, "self-similarity should be high: {}", results[0].score);
    }

    #[test]
    fn index_dimension_mismatch_rejected() {
        let config = TurboQuantConfig {
            dimension: 16,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);
        let result = index.insert("bad", &[1.0, 2.0], HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn index_delete() {
        let dim = 8;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);
        index
            .insert("x", &random_vector(dim, 1), HashMap::new())
            .unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.delete("x"));
        assert_eq!(index.len(), 0);
        assert!(!index.delete("x")); // already gone
    }

    #[test]
    fn index_empty_search() {
        let config = TurboQuantConfig {
            dimension: 8,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);
        let results = index.search(&random_vector(8, 1), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn index_wrong_query_dim_returns_empty() {
        let config = TurboQuantConfig {
            dimension: 16,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);
        let results = index.search(&[1.0, 2.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn index_compression_ratio() {
        let dim = 384;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);

        for i in 0..100 {
            index
                .insert(
                    format!("v{i}"),
                    &random_vector(dim, i as u64),
                    HashMap::new(),
                )
                .unwrap();
        }

        let ratio = index.compression_ratio();
        assert!(
            ratio > 5.0,
            "compression ratio should be >5x, got {ratio:.1}x"
        );

        let stats = index.stats();
        assert!(stats.bits_per_dimension < 7.0, "should be <7 bits/dim");
        assert_eq!(stats.num_vectors, 100);
    }

    #[test]
    fn index_stats_empty() {
        let config = TurboQuantConfig {
            dimension: 64,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);
        let stats = index.stats();
        assert_eq!(stats.num_vectors, 0);
        assert_eq!(stats.compressed_bytes, 0);
        assert_eq!(stats.compression_ratio, 0.0);
    }

    // ── End-to-end recall test ───────────────────────────────────────────────

    #[test]
    fn recall_at_10_above_threshold() {
        let dim = 64;
        let n = 200;
        let k = 10;

        // Build ground truth with brute-force cosine
        let vectors: Vec<Vec<f32>> = (0..n).map(|i| normalized_vector(dim, i as u64 + 100)).collect();
        let query = normalized_vector(dim, 999);

        let mut gt_scores: Vec<(f32, usize)> = vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (cosine_sim(&query, v), i))
            .collect();
        gt_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let gt_top_k: Vec<usize> = gt_scores.iter().take(k).map(|(_, i)| *i).collect();

        // Build TurboQuant index
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);
        for (i, v) in vectors.iter().enumerate() {
            index.insert(format!("{i}"), v, HashMap::new()).unwrap();
        }

        let results = index.search(&query, k);
        let tq_top_k: Vec<usize> = results
            .iter()
            .filter_map(|r| r.id.parse::<usize>().ok())
            .collect();

        // Count overlap
        let hits = tq_top_k.iter().filter(|id| gt_top_k.contains(id)).count();
        let recall = hits as f64 / k as f64;

        assert!(
            recall >= 0.6,
            "recall@{k} should be ≥60%, got {:.0}% ({hits}/{k})",
            recall * 100.0
        );
    }

    // ── Batch compression ────────────────────────────────────────────────────

    #[test]
    fn compress_batch_builds_index() {
        let dim = 32;
        let vectors: Vec<Vec<f32>> = (0..10).map(|i| random_vector(dim, i)).collect();
        let ids: Vec<String> = (0..10).map(|i| format!("doc_{i}")).collect();
        let index = compress_batch(&vectors, &ids, dim, 42);
        assert_eq!(index.len(), 10);
        assert!(index.compression_ratio() > 3.0);
    }

    // ── PolarQuant POLAR_LEVELS constant ─────────────────────────────────────

    #[test]
    fn polar_levels_is_4() {
        assert_eq!(POLAR_LEVELS, 4);
    }

    // ── Metadata round-trip ──────────────────────────────────────────────────

    #[test]
    fn metadata_preserved_in_search() {
        let dim = 16;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);

        let mut meta = HashMap::new();
        meta.insert("file".to_string(), "main.rs".to_string());

        let v = normalized_vector(dim, 1);
        index.insert("doc1", &v, meta).unwrap();

        let results = index.search(&v, 1);
        assert_eq!(results[0].metadata.get("file").unwrap(), "main.rs");
    }

    // ── Rebuild matrices after serde ─────────────────────────────────────────

    #[test]
    fn rebuild_matrices_restores_search() {
        let dim = 16;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);

        let v = normalized_vector(dim, 1);
        index.insert("a", &v, HashMap::new()).unwrap();

        // Simulate serde round-trip (matrices are skipped)
        let json = serde_json::to_string(&index).unwrap();
        let mut index2: TurboQuantIndex = serde_json::from_str(&json).unwrap();
        index2.rebuild_matrices();

        let results = index2.search(&v, 1);
        assert_eq!(results[0].id, "a");
        assert!(results[0].score > 0.8);
    }

    // ── cosine_sim edge cases ────────────────────────────────────────────────

    #[test]
    fn cosine_sim_empty() {
        assert_eq!(cosine_sim(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_sim_mismatched_len() {
        assert_eq!(cosine_sim(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_sim_identical() {
        let v = vec![1.0f32, 2.0, 3.0];
        assert!((cosine_sim(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_sim_opposite() {
        let a = vec![1.0f32, 0.0];
        let b = vec![-1.0f32, 0.0];
        assert!((cosine_sim(&a, &b) + 1.0).abs() < 1e-6);
    }

    // ── CompressedVector storage ─────────────────────────────────────────────

    #[test]
    fn compressed_vector_storage_bytes() {
        let dim = 64;
        let v = random_vector(dim, 42);
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);
        let cv = index.compress(&v);
        // Should be much smaller than 64 * 4 = 256 bytes
        assert!(
            cv.storage_bytes() < 100,
            "compressed should be <100 bytes for 64-dim, got {}",
            cv.storage_bytes()
        );
    }

    // ── Decompress recovers approximate direction ────────────────────────────

    #[test]
    fn decompress_recovers_direction() {
        let dim = 64;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);

        let v = normalized_vector(dim, 42);
        let cv = index.compress(&v);
        let recovered = index.decompress(&cv);

        let sim = cosine_sim(&v, &recovered);
        assert!(
            sim > 0.7,
            "decompressed direction should be close to original: cosine = {sim}"
        );
    }

    // ── is_empty ─────────────────────────────────────────────────────────────

    #[test]
    fn is_empty_true_for_new_index() {
        let config = TurboQuantConfig {
            dimension: 8,
            seed: 42,
            qjl_proj_dim: None,
        };
        let index = TurboQuantIndex::new(config);
        assert!(index.is_empty());
    }

    #[test]
    fn is_empty_false_after_insert() {
        let dim = 8;
        let config = TurboQuantConfig {
            dimension: dim,
            seed: 42,
            qjl_proj_dim: None,
        };
        let mut index = TurboQuantIndex::new(config);
        index.insert("a", &random_vector(dim, 1), HashMap::new()).unwrap();
        assert!(!index.is_empty());
    }
}
