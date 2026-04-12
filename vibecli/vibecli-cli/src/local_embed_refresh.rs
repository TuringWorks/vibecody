//! Quantized local embedding refresh on file save.
//!
//! GAP-v9-019: rivals Continue.dev local embeddings, Cursor offline RAG, Cody offline search.
//! - Int8 scalar quantization for compact embedding storage (~75% size reduction)
//! - File-level embedding index with change detection (content hash)
//! - Incremental refresh: only re-encode changed files
//! - BM25-compatible keyword index as fallback when embeddings are stale
//! - Similarity search with decoded float32 comparison
//! - Index persistence: serialize/deserialize to bytes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Quantization ────────────────────────────────────────────────────────────

/// Scalar int8 quantization of a float32 embedding vector.
/// Maps [min, max] → [-127, 127].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedVector {
    pub values: Vec<i8>,
    pub scale: f32,
    pub zero_point: f32,
}

impl QuantizedVector {
    /// Quantize a float32 vector.
    pub fn from_floats(floats: &[f32]) -> Self {
        if floats.is_empty() {
            return Self { values: vec![], scale: 1.0, zero_point: 0.0 };
        }
        let min = floats.iter().cloned().fold(f32::MAX, f32::min);
        let max = floats.iter().cloned().fold(f32::MIN, f32::max);
        let range = (max - min).max(1e-8);
        let scale = range / 254.0;
        let zero_point = min + range / 2.0;
        let values = floats.iter().map(|&v| {
            ((v - zero_point) / scale).round().clamp(-127.0, 127.0) as i8
        }).collect();
        Self { values, scale, zero_point }
    }

    /// Dequantize back to float32.
    pub fn to_floats(&self) -> Vec<f32> {
        self.values.iter().map(|&q| q as f32 * self.scale + self.zero_point).collect()
    }

    /// Cosine similarity between two quantized vectors (dequantized).
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let a = self.to_floats();
        let b = other.to_floats();
        let len = a.len().min(b.len());
        if len == 0 { return 0.0; }
        let dot: f32 = a[..len].iter().zip(&b[..len]).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a < 1e-8 || norm_b < 1e-8 { return 0.0; }
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }

    /// Storage bytes used.
    pub fn byte_size(&self) -> usize { self.values.len() }

    /// Size reduction vs float32 (8-bit / 32-bit = 75% savings).
    pub fn compression_ratio() -> f32 { 4.0 }
}

// ─── File Entry ──────────────────────────────────────────────────────────────

/// FNV-1a-like hash for content change detection (no external deps).
pub fn content_hash(content: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in content.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub content_hash: u64,
    pub embedding: QuantizedVector,
    /// BM25 term-frequency map (keyword → tf score).
    pub tf_map: HashMap<String, f32>,
}

impl IndexedFile {
    pub fn new(path: impl Into<String>, content: &str, embedding: QuantizedVector) -> Self {
        let hash = content_hash(content);
        let tf_map = compute_tf(content);
        Self { path: path.into(), content_hash: hash, embedding, tf_map }
    }
}

/// Compute term frequencies from text.
fn compute_tf(text: &str) -> HashMap<String, f32> {
    let tokens: Vec<String> = text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect();
    let total = tokens.len() as f32;
    if total == 0.0 { return HashMap::new(); }
    let mut counts: HashMap<String, u32> = HashMap::new();
    for t in &tokens { *counts.entry(t.clone()).or_insert(0) += 1; }
    counts.into_iter().map(|(k, c)| (k, c as f32 / total)).collect()
}

// ─── Embedding Index ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingIndex {
    pub files: HashMap<String, IndexedFile>,
    /// IDF table computed over the entire corpus.
    pub idf: HashMap<String, f32>,
    /// Total files indexed.
    pub file_count: usize,
}

impl EmbeddingIndex {
    pub fn new() -> Self {
        Self { files: HashMap::new(), idf: HashMap::new(), file_count: 0 }
    }

    /// Upsert a file: only re-encode if content hash changed.
    /// Returns true if the file was actually updated.
    pub fn upsert(&mut self, path: &str, content: &str, embedding: QuantizedVector) -> bool {
        let hash = content_hash(content);
        if let Some(existing) = self.files.get(path) {
            if existing.content_hash == hash { return false; } // no change
        }
        let entry = IndexedFile::new(path, content, embedding);
        self.files.insert(path.to_string(), entry);
        self.file_count = self.files.len();
        self.recompute_idf();
        true
    }

    /// Remove a file from the index.
    pub fn remove(&mut self, path: &str) -> bool {
        let removed = self.files.remove(path).is_some();
        if removed {
            self.file_count = self.files.len();
            self.recompute_idf();
        }
        removed
    }

    /// Recompute IDF scores across corpus.
    fn recompute_idf(&mut self) {
        let n = self.files.len() as f32;
        if n == 0.0 { self.idf.clear(); return; }
        let mut df: HashMap<String, u32> = HashMap::new();
        for f in self.files.values() {
            for term in f.tf_map.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }
        self.idf = df.into_iter().map(|(t, d)| (t, ((n / d as f32) + 1.0).ln())).collect();
    }

    /// Vector similarity search. Returns top-k (path, score) pairs.
    pub fn vector_search(&self, query_embedding: &QuantizedVector, top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self.files.iter()
            .map(|(path, file)| (path.clone(), query_embedding.cosine_similarity(&file.embedding)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// BM25 keyword search. Returns top-k (path, score) pairs.
    pub fn keyword_search(&self, query_terms: &[&str], top_k: usize) -> Vec<(String, f32)> {
        let k1 = 1.5_f32;
        let b  = 0.75_f32;
        let avg_dl = if self.files.is_empty() { 1.0 } else {
            self.files.values().map(|f| f.tf_map.len() as f32).sum::<f32>() / self.files.len() as f32
        };

        let mut scored: Vec<(String, f32)> = self.files.iter().map(|(path, file)| {
            let dl = file.tf_map.len() as f32;
            let score: f32 = query_terms.iter().map(|&term| {
                let term_l = term.to_lowercase();
                let tf = file.tf_map.get(&term_l).copied().unwrap_or(0.0) * dl;
                let idf = self.idf.get(&term_l).copied().unwrap_or(0.0);
                let tf_norm = tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * dl / avg_dl));
                idf * tf_norm
            }).sum();
            (path.clone(), score)
        }).filter(|(_, s)| *s > 0.0).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Hybrid search: combine vector and keyword scores with alpha blending.
    /// alpha=1.0 → pure vector; alpha=0.0 → pure keyword.
    pub fn hybrid_search(
        &self,
        query_embedding: &QuantizedVector,
        query_terms: &[&str],
        top_k: usize,
        alpha: f32,
    ) -> Vec<(String, f32)> {
        let vec_results: HashMap<String, f32> = self.vector_search(query_embedding, self.files.len()).into_iter().collect();
        let kw_results:  HashMap<String, f32> = self.keyword_search(query_terms, self.files.len()).into_iter().collect();

        let mut all_paths: Vec<String> = vec_results.keys().chain(kw_results.keys()).cloned().collect();
        all_paths.sort();
        all_paths.dedup();

        // Normalise scores independently
        let vec_max: f32 = vec_results.values().cloned().fold(0.0_f32, f32::max).max(1e-8);
        let kw_max:  f32 = kw_results.values().cloned().fold(0.0_f32, f32::max).max(1e-8);

        let mut scored: Vec<(String, f32)> = all_paths.into_iter().map(|path| {
            let v = vec_results.get(&path).copied().unwrap_or(0.0) / vec_max;
            let k = kw_results.get(&path).copied().unwrap_or(0.0) / kw_max;
            (path, alpha * v + (1.0 - alpha) * k)
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Check whether a file is stale (content hash mismatch).
    pub fn is_stale(&self, path: &str, current_content: &str) -> bool {
        match self.files.get(path) {
            None => true,
            Some(entry) => entry.content_hash != content_hash(current_content),
        }
    }

    /// Summary stats for the index.
    pub fn stats(&self) -> IndexStats {
        let total_bytes: usize = self.files.values().map(|f| f.embedding.byte_size()).sum();
        let float_bytes = total_bytes * 4; // what float32 would cost
        IndexStats {
            files: self.files.len(),
            total_embedding_bytes: total_bytes,
            compression_saving_bytes: float_bytes.saturating_sub(total_bytes),
        }
    }
}

impl Default for EmbeddingIndex {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub files: usize,
    pub total_embedding_bytes: usize,
    pub compression_saving_bytes: usize,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn embed(vals: &[f32]) -> QuantizedVector { QuantizedVector::from_floats(vals) }
    fn unit_embed(n: usize) -> QuantizedVector {
        let vals: Vec<f32> = (0..n).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        embed(&vals)
    }

    #[test]
    fn test_quantize_roundtrip_close() {
        let orig = vec![0.1, 0.5, -0.3, 0.8, -0.9];
        let q = embed(&orig);
        let back = q.to_floats();
        for (o, b) in orig.iter().zip(&back) {
            assert!((o - b).abs() < 0.02, "roundtrip error: {o} vs {b}");
        }
    }

    #[test]
    fn test_quantize_empty() {
        let q = embed(&[]);
        assert!(q.values.is_empty());
        assert!(q.to_floats().is_empty());
    }

    #[test]
    fn test_quantize_single_value() {
        let q = embed(&[0.5]);
        let back = q.to_floats();
        assert!((back[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let q = embed(&[0.6, 0.8]);
        assert!((q.cosine_similarity(&q) - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = embed(&[1.0, 0.0]);
        let b = embed(&[0.0, 1.0]);
        assert!(a.cosine_similarity(&b).abs() < 0.15);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a = embed(&[]);
        assert_eq!(a.cosine_similarity(&embed(&[])), 0.0);
    }

    #[test]
    fn test_compression_ratio() {
        assert_eq!(QuantizedVector::compression_ratio(), 4.0);
    }

    #[test]
    fn test_byte_size() {
        let q = embed(&[1.0, 2.0, 3.0]);
        assert_eq!(q.byte_size(), 3);
    }

    #[test]
    fn test_content_hash_stable() {
        assert_eq!(content_hash("hello"), content_hash("hello"));
    }

    #[test]
    fn test_content_hash_differs() {
        assert_ne!(content_hash("hello"), content_hash("world"));
    }

    #[test]
    fn test_upsert_adds_file() {
        let mut idx = EmbeddingIndex::new();
        let added = idx.upsert("a.rs", "fn foo() {}", unit_embed(4));
        assert!(added);
        assert!(idx.files.contains_key("a.rs"));
    }

    #[test]
    fn test_upsert_no_change_skips() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "fn foo() {}", unit_embed(4));
        let added = idx.upsert("a.rs", "fn foo() {}", unit_embed(4));
        assert!(!added);
        assert_eq!(idx.files.len(), 1);
    }

    #[test]
    fn test_upsert_changed_content_updates() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "fn foo() {}", unit_embed(4));
        let added = idx.upsert("a.rs", "fn bar() {}", unit_embed(4));
        assert!(added);
    }

    #[test]
    fn test_remove_file() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "content", unit_embed(4));
        assert!(idx.remove("a.rs"));
        assert!(!idx.files.contains_key("a.rs"));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut idx = EmbeddingIndex::new();
        assert!(!idx.remove("nonexistent.rs"));
    }

    #[test]
    fn test_is_stale_new_file() {
        let idx = EmbeddingIndex::new();
        assert!(idx.is_stale("x.rs", "some content"));
    }

    #[test]
    fn test_is_stale_unchanged() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("x.rs", "content", unit_embed(4));
        assert!(!idx.is_stale("x.rs", "content"));
    }

    #[test]
    fn test_is_stale_changed_content() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("x.rs", "content", unit_embed(4));
        assert!(idx.is_stale("x.rs", "different content"));
    }

    #[test]
    fn test_vector_search_returns_top_k() {
        let mut idx = EmbeddingIndex::new();
        let q1 = embed(&[1.0, 0.0]);
        let q2 = embed(&[0.0, 1.0]);
        let q3 = embed(&[0.9, 0.1]);
        idx.upsert("a.rs", "fn alpha", q1.clone());
        idx.upsert("b.rs", "fn beta", q2);
        idx.upsert("c.rs", "fn gamma", q3);
        let results = idx.vector_search(&q1, 2);
        assert_eq!(results.len(), 2);
        // a.rs should rank highest (query == q1)
        assert_eq!(results[0].0, "a.rs");
    }

    #[test]
    fn test_keyword_search_finds_term() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "alpha beta gamma", unit_embed(4));
        idx.upsert("b.rs", "delta epsilon", unit_embed(4));
        let results = idx.keyword_search(&["alpha"], 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "a.rs");
    }

    #[test]
    fn test_keyword_search_no_match() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "fn foo", unit_embed(4));
        let results = idx.keyword_search(&["zorgblat"], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hybrid_search_blends_scores() {
        let mut idx = EmbeddingIndex::new();
        let qv = embed(&[1.0, 0.0]);
        idx.upsert("a.rs", "alpha beta", qv.clone());
        idx.upsert("b.rs", "gamma delta", embed(&[0.0, 1.0]));
        let results = idx.hybrid_search(&qv, &["alpha"], 2, 0.5);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "a.rs"); // best vector + keyword match
    }

    #[test]
    fn test_stats_compression() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "content", embed(&[0.1, 0.2, 0.3, 0.4]));
        let stats = idx.stats();
        assert_eq!(stats.files, 1);
        assert_eq!(stats.total_embedding_bytes, 4); // 4 int8 values
        assert_eq!(stats.compression_saving_bytes, 12); // saved vs float32 (4 * 4 - 4 = 12)
    }

    #[test]
    fn test_index_file_count_updates() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "a", unit_embed(2));
        idx.upsert("b.rs", "b", unit_embed(2));
        assert_eq!(idx.file_count, 2);
        idx.remove("a.rs");
        assert_eq!(idx.file_count, 1);
    }

    #[test]
    fn test_idf_populated_after_upsert() {
        let mut idx = EmbeddingIndex::new();
        idx.upsert("a.rs", "hello world", unit_embed(2));
        assert!(!idx.idf.is_empty());
    }

    #[test]
    fn test_compute_tf_filters_short_tokens() {
        let tf = compute_tf("a bb ccc");
        assert!(!tf.contains_key("a")); // len < 2 filtered
        assert!(tf.contains_key("bb"));
        assert!(tf.contains_key("ccc"));
    }
}
