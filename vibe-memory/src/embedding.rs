//! How memories get their vectors.
//!
//! Until now this crate had no embedding model at all: `generate_embedding`
//! hashed words into buckets with a positional weight and called the result an
//! embedding. It works as a crude lexical signal, and it is free, so it stays
//! — as the **default**, named honestly, behind the same
//! [`Embedder`](vibe_embed::Embedder) trait every real model implements.
//!
//! A caller who wants real semantic recall passes any other embedder
//! ([`GlobalMemStore::with_embedder`](crate::GlobalMemStore::with_embedder)):
//! Ollama, OpenAI, Voyage, Cohere, Gemini, or an in-process candle model.
//!
//! # Why the model identity is stored per row
//!
//! Vectors from two models are not comparable, and cosine similarity does not
//! say so — it returns `0.0` for a length mismatch and a meaningless number
//! for a same-length mismatch. Before this, changing `VIBE_MEMORY_DIM` made
//! every existing memory score `0.0` and silently vanish from every search,
//! with the rows still sitting in the database. Each row now records which
//! model produced its vector, and search compares only rows it can compare —
//! reporting the rest as [`SearchDiagnostics::skipped_other_model`] instead of
//! hiding them.

use vibe_embed::{EmbedKind, Embedder, ModelRef, ProviderKind, SharedEmbedder};

/// Model id reported by the built-in hash engine.
///
/// Deliberately not the name of any real model: this produces a lexical
/// fingerprint, not a semantic embedding, and an index built from it must
/// never be mistaken for one built by a trained model.
pub const HASH_MODEL_ID: &str = "vibe-memory-hash";

/// The zero-dependency default: words hashed into `dim` buckets with a
/// positional decay, L2-normalised.
///
/// Bit-for-bit identical to the `generate_embedding` this replaced, so
/// existing databases keep working without a re-embed.
pub struct HashEmbedder {
    dim: usize,
    model: ModelRef,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        // A zero-bucket engine would divide by zero on the first token.
        let dim = dim.max(1);
        Self {
            dim,
            model: ModelRef::new(ProviderKind::Local, HASH_MODEL_ID)
                .with_dimensions(Some(dim)),
        }
    }

    pub fn shared(dim: usize) -> SharedEmbedder {
        std::sync::Arc::new(Self::new(dim))
    }

    pub fn embed_sync(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.dim];
        let lower_text = text.to_lowercase();
        for (i, word) in lower_text.split_whitespace().enumerate() {
            let idx = (simple_hash(word) % self.dim as u64) as usize;
            embedding[idx] += 1.0f32 / (1.0 + (i as f32 * 0.1));
        }
        let magnitude = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            embedding.iter_mut().for_each(|v| *v /= magnitude);
        }
        embedding
    }
}

#[async_trait::async_trait]
impl Embedder for HashEmbedder {
    fn model(&self) -> &ModelRef {
        &self.model
    }

    fn dim(&self) -> Option<usize> {
        Some(self.dim)
    }

    async fn embed_batch(
        &self,
        texts: &[String],
        _kind: EmbedKind,
    ) -> vibe_embed::Result<Vec<Vec<f32>>> {
        // Symmetric: there is no query/document asymmetry in a bag of hashes.
        Ok(texts.iter().map(|t| self.embed_sync(t)).collect())
    }
}

fn simple_hash(s: &str) -> u64 {
    s.bytes()
        .fold(5381u64, |h, c| h.wrapping_mul(33).wrapping_add(c as u64))
}

// ---------------------------------------------------------------------------
// Row-level model tagging
// ---------------------------------------------------------------------------

/// The identity written alongside every stored vector.
///
/// `slug` is [`ModelRef::slug`], which already folds in the dimension for
/// Matryoshka variants; `dim` is the length actually stored, measured rather
/// than assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorTag {
    pub slug: String,
    pub dim: usize,
}

impl VectorTag {
    pub fn of(model: &ModelRef, dim: usize) -> Self {
        Self {
            slug: model.slug(),
            dim,
        }
    }

    /// Whether a stored row can be compared with a vector carrying this tag.
    ///
    /// Rows written before model tagging existed have no slug. They are
    /// treated as comparable *only* when their length matches — the best
    /// available evidence, and the behaviour those rows had before, so an
    /// upgrade does not make an existing memory store look empty.
    pub fn accepts(&self, row_slug: Option<&str>, row_len: usize) -> bool {
        match row_slug {
            Some(s) => s == self.slug && row_len == self.dim,
            None => row_len == self.dim,
        }
    }
}

/// What a search could and could not compare.
///
/// Returned alongside results so a caller can say "12 memories are stored
/// with a different embedding model and were not searched" instead of
/// presenting a truncated result set as if it were complete.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SearchDiagnostics {
    /// Rows compared against the query.
    pub compared: usize,
    /// Rows skipped because another model produced their vectors.
    pub skipped_other_model: usize,
    /// Rows skipped because they have no vector at all.
    pub skipped_no_vector: usize,
}

impl SearchDiagnostics {
    pub fn is_complete(&self) -> bool {
        self.skipped_other_model == 0 && self.skipped_no_vector == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The refactor must be bit-identical to the old `generate_embedding`, or
    /// every existing database silently loses recall.
    #[test]
    fn hash_embedding_matches_the_original_algorithm() {
        let e = HashEmbedder::new(768);
        let got = e.embed_sync("the quick brown fox");

        // Reference implementation, transcribed from the code this replaced.
        let dim = 768usize;
        let mut want = vec![0.0f32; dim];
        let lower = "the quick brown fox".to_lowercase();
        for (i, word) in lower.split_whitespace().enumerate() {
            let hash = simple_hash(word);
            want[(hash % dim as u64) as usize] += 1.0f32 / (1.0 + (i as f32 * 0.1));
        }
        let mag = want.iter().map(|v| v * v).sum::<f32>().sqrt();
        want.iter_mut().for_each(|v| *v /= mag);

        assert_eq!(got, want);
    }

    #[test]
    fn hash_embedding_is_unit_length() {
        let v = HashEmbedder::new(64).embed_sync("hello world");
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn empty_text_yields_a_zero_vector_not_a_nan_vector() {
        let v = HashEmbedder::new(32).embed_sync("   ");
        assert_eq!(v.len(), 32);
        assert!(v.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn zero_dimension_is_clamped_rather_than_dividing_by_zero() {
        let e = HashEmbedder::new(0);
        assert_eq!(e.dim(), Some(1));
        assert_eq!(e.embed_sync("word").len(), 1);
    }

    #[test]
    fn identity_names_the_hash_engine_not_a_model() {
        let e = HashEmbedder::new(768);
        assert_eq!(e.model().provider, ProviderKind::Local);
        assert_eq!(e.model().model, HASH_MODEL_ID);
    }

    /// Dimension is part of the identity: 768-bucket and 512-bucket vectors
    /// are not comparable even though the algorithm is the same.
    #[test]
    fn different_dimensions_are_different_models() {
        assert_ne!(
            HashEmbedder::new(768).model().slug(),
            HashEmbedder::new(512).model().slug()
        );
    }

    #[tokio::test]
    async fn batch_matches_single() {
        let e = HashEmbedder::new(128);
        let texts = vec!["alpha beta".to_string(), "gamma".to_string()];
        let batch = e
            .embed_batch(&texts, EmbedKind::Document)
            .await
            .expect("batch");
        assert_eq!(batch[0], e.embed_sync("alpha beta"));
        assert_eq!(batch[1], e.embed_sync("gamma"));
    }

    // ── VectorTag ────────────────────────────────────────────────────────────

    #[test]
    fn tag_accepts_its_own_rows() {
        let e = HashEmbedder::new(768);
        let tag = VectorTag::of(e.model(), 768);
        assert!(tag.accepts(Some(&tag.slug), 768));
    }

    #[test]
    fn tag_rejects_another_models_rows() {
        let tag = VectorTag::of(HashEmbedder::new(768).model(), 768);
        let other = ModelRef::new(ProviderKind::Ollama, "nomic-embed-text");
        assert!(!tag.accepts(Some(&other.slug()), 768));
    }

    /// Same length, different model, is the dangerous case: cosine returns a
    /// plausible-looking number instead of failing.
    #[test]
    fn tag_rejects_a_same_length_different_model() {
        let tag = VectorTag::of(&ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"), 768);
        let impostor = ModelRef::new(ProviderKind::Gemini, "text-embedding-004"); // also 768
        assert!(!tag.accepts(Some(&impostor.slug()), 768));
    }

    /// Pre-tagging rows must keep working, or upgrading empties the store.
    #[test]
    fn tag_accepts_untagged_rows_of_the_right_length() {
        let tag = VectorTag::of(HashEmbedder::new(768).model(), 768);
        assert!(tag.accepts(None, 768));
        assert!(!tag.accepts(None, 512));
    }

    #[test]
    fn diagnostics_report_completeness() {
        assert!(SearchDiagnostics::default().is_complete());
        let partial = SearchDiagnostics {
            compared: 5,
            skipped_other_model: 2,
            skipped_no_vector: 0,
        };
        assert!(!partial.is_complete());
    }
}
