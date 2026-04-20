//! TurboQuant-backed memory index — wraps `vibe_core::index::turboquant::TurboQuantIndex`
//! with the small surface OpenMemory needs (insert / search / size / ratio).
//!
//! The underlying TurboQuantIndex stores ~3 bits/dim (PolarQuant 2 bits + QJL
//! residual 1 bit + f32 radius), decoding on each search query. This module
//! deliberately stays thin: it picks reasonable defaults (deterministic seed,
//! `qjl_proj_dim = None` so the projection ratio falls back to the configured
//! default) and exposes `dimension()` so test harnesses don't need to track
//! it separately.

use std::collections::HashMap;

use vibe_core::index::turboquant::{
    TurboQuantConfig, TurboQuantIndex, TurboQuantSearchResult,
};

/// Default seed for the random rotation matrix. Stable across runs so two
/// indexes with the same data produce identical compressed representations.
const DEFAULT_SEED: u64 = 0xC0DE_F00D_DEAD_BEEF;

/// A single search hit decoded back from the compressed store.
#[derive(Debug, Clone)]
pub struct MemoryHit {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

impl From<TurboQuantSearchResult> for MemoryHit {
    fn from(r: TurboQuantSearchResult) -> Self {
        Self {
            id: r.id,
            score: r.score,
            metadata: r.metadata,
        }
    }
}

/// Compressed embedding index. Inserted vectors are stored at ~3 bits/dim;
/// `search` decompresses on the fly and returns the top-k cosine matches.
pub struct CompressedMemoryIndex {
    inner: TurboQuantIndex,
    dimension: usize,
}

impl CompressedMemoryIndex {
    pub fn new(dimension: usize) -> Self {
        let cfg = TurboQuantConfig {
            dimension,
            seed: DEFAULT_SEED,
            qjl_proj_dim: None,
        };
        Self {
            inner: TurboQuantIndex::new(cfg),
            dimension,
        }
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn insert(&mut self, id: String, vector: &[f32], metadata: HashMap<String, String>) {
        let _ = self.inner.insert(id, vector, metadata);
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<MemoryHit> {
        self.inner
            .search(query, top_k)
            .into_iter()
            .map(MemoryHit::from)
            .collect()
    }

    /// Cosine-style query that mirrors the legacy `HnswIndex::query` shape so
    /// callers in `OpenMemoryStore` can use this index as a drop-in.
    pub fn query(&self, vector: &[f32], k: usize) -> Vec<(String, f64)> {
        self.search(vector, k)
            .into_iter()
            .map(|h| (h.id, h.score as f64))
            .collect()
    }

    /// Remove a vector by id. Returns `true` if a matching entry was removed.
    pub fn delete(&mut self, id: &str) -> bool {
        self.inner.delete(id)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn compression_ratio(&self) -> f64 {
        self.inner.compression_ratio()
    }
}
