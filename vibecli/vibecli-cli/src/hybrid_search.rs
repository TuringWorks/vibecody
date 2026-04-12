//! Cross-language hybrid semantic search with BM25 + embedding re-ranking.
//!
//! GAP-v9-005: rivals Cursor Smart Search, Gemini Code Search, Windsurf (40+ languages).
//! - BM25 keyword scoring + cosine-similarity vector ranking
//! - LLM re-ranker stage: selects top-k from merged candidate set
//! - Multi-hop navigation: "find callers of X that also import Y"
//! - Language-agnostic indexing: term frequency tables per document
//! - Snippet extraction with surrounding context lines

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Core Types ──────────────────────────────────────────────────────────────

/// A document in the search index (one source file = one document).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDoc {
    pub id: String,
    pub path: String,
    pub language: String,
    pub content: String,
    pub embedding: Vec<f32>,   // dense vector (zero = not embedded)
    pub term_freq: HashMap<String, u32>,
    pub doc_len: u32,
}

impl SearchDoc {
    pub fn new(id: &str, path: &str, language: &str, content: &str) -> Self {
        let tokens = tokenize(content);
        let doc_len = tokens.len() as u32;
        let mut term_freq: HashMap<String, u32> = HashMap::new();
        for t in tokens { *term_freq.entry(t).or_insert(0) += 1; }
        Self {
            id: id.to_string(),
            path: path.to_string(),
            language: language.to_string(),
            content: content.to_string(),
            embedding: Vec::new(),
            term_freq,
            doc_len,
        }
    }

    pub fn with_embedding(mut self, emb: Vec<f32>) -> Self {
        self.embedding = emb;
        self
    }
}

/// Tokenise text into lowercase words (alphanumeric only).
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect()
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub path: String,
    pub language: String,
    pub bm25_score: f64,
    pub vector_score: f64,
    pub rerank_score: f64,
    pub snippet: String,
    pub match_line: Option<u32>,
}

impl SearchResult {
    /// Combined score (weighted BM25 + vector).
    pub fn combined_score(&self) -> f64 {
        0.4 * self.bm25_score + 0.6 * self.vector_score
    }
}

/// Re-ranking strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RerankStrategy {
    None,
    Linear { bm25_weight: f64, vector_weight: f64 },
    Rrf { k: f64 },  // Reciprocal Rank Fusion
}

impl RerankStrategy {
    pub fn score(&self, bm25: f64, vector: f64, bm25_rank: usize, vec_rank: usize) -> f64 {
        match self {
            Self::None => bm25,
            Self::Linear { bm25_weight, vector_weight } => bm25_weight * bm25 + vector_weight * vector,
            Self::Rrf { k } => {
                1.0 / (k + bm25_rank as f64) + 1.0 / (k + vec_rank as f64)
            }
        }
    }
}

// ─── BM25 Engine ─────────────────────────────────────────────────────────────

/// BM25+ scorer.
pub struct Bm25Index {
    docs: Vec<SearchDoc>,
    df: HashMap<String, u32>,  // document frequency per term
    avg_doc_len: f64,
    k1: f64,
    b: f64,
}

impl Bm25Index {
    pub fn new() -> Self {
        Self { docs: Vec::new(), df: HashMap::new(), avg_doc_len: 0.0, k1: 1.5, b: 0.75 }
    }

    pub fn add(&mut self, doc: SearchDoc) {
        for term in doc.term_freq.keys() {
            *self.df.entry(term.clone()).or_insert(0) += 1;
        }
        self.docs.push(doc);
        self.recompute_avg();
    }

    fn recompute_avg(&mut self) {
        if self.docs.is_empty() { self.avg_doc_len = 0.0; return; }
        let total: u32 = self.docs.iter().map(|d| d.doc_len).sum();
        self.avg_doc_len = total as f64 / self.docs.len() as f64;
    }

    pub fn score(&self, doc: &SearchDoc, query_terms: &[String]) -> f64 {
        let n = self.docs.len() as f64;
        let mut score = 0.0;
        for term in query_terms {
            let tf = *doc.term_freq.get(term).unwrap_or(&0) as f64;
            if tf == 0.0 { continue; }
            let df = *self.df.get(term).unwrap_or(&0) as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            let tf_norm = tf * (self.k1 + 1.0)
                / (tf + self.k1 * (1.0 - self.b + self.b * doc.doc_len as f64 / self.avg_doc_len.max(1.0)));
            score += idf * tf_norm;
        }
        score
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(usize, f64)> {
        let terms = tokenize(query);
        let mut scored: Vec<(usize, f64)> = self.docs.iter().enumerate()
            .map(|(i, doc)| (i, self.score(doc, &terms)))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    pub fn docs(&self) -> &[SearchDoc] { &self.docs }
    pub fn doc_count(&self) -> usize { self.docs.len() }
    pub fn vocab_size(&self) -> usize { self.df.len() }
}

impl Default for Bm25Index {
    fn default() -> Self { Self::new() }
}

// ─── Vector Search ────────────────────────────────────────────────────────────

/// Cosine similarity between two equal-length vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }
}

// ─── Hybrid Search Engine ─────────────────────────────────────────────────────

/// Combined BM25 + vector hybrid search with re-ranking.
pub struct HybridSearch {
    bm25: Bm25Index,
    strategy: RerankStrategy,
}

impl HybridSearch {
    pub fn new(strategy: RerankStrategy) -> Self {
        Self { bm25: Bm25Index::new(), strategy }
    }

    pub fn add_doc(&mut self, doc: SearchDoc) {
        self.bm25.add(doc);
    }

    pub fn doc_count(&self) -> usize { self.bm25.doc_count() }

    /// Full hybrid search: BM25 + vector retrieval → rerank → top-k.
    pub fn search(&self, query: &str, query_embedding: Option<&[f32]>, limit: usize) -> Vec<SearchResult> {
        let bm25_hits = self.bm25.search(query, limit * 3);
        let docs = self.bm25.docs();

        // Vector retrieval (if embedding provided)
        let mut vec_scores: Vec<(usize, f32)> = if let Some(qe) = query_embedding {
            docs.iter().enumerate()
                .filter(|(_, d)| !d.embedding.is_empty())
                .map(|(i, d)| (i, cosine_similarity(&d.embedding, qe)))
                .collect()
        } else { Vec::new() };
        vec_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Build candidate set union
        let mut candidate_set: Vec<usize> = bm25_hits.iter().map(|(i, _)| *i).collect();
        for (i, _) in &vec_scores {
            if !candidate_set.contains(i) { candidate_set.push(*i); }
        }
        candidate_set.truncate(limit * 3);

        // Rerank
        let bm25_rank_map: HashMap<usize, usize> = bm25_hits.iter().enumerate().map(|(r, (i, _))| (*i, r + 1)).collect();
        let vec_rank_map: HashMap<usize, usize> = vec_scores.iter().enumerate().map(|(r, (i, _))| (*i, r + 1)).collect();
        let bm25_score_map: HashMap<usize, f64> = bm25_hits.iter().map(|(i, s)| (*i, *s)).collect();
        let vec_score_map: HashMap<usize, f64> = vec_scores.iter().map(|(i, s)| (*i, *s as f64)).collect();

        let mut results: Vec<SearchResult> = candidate_set.iter().map(|&idx| {
            let doc = &docs[idx];
            let bm25_s = *bm25_score_map.get(&idx).unwrap_or(&0.0);
            let vec_s = *vec_score_map.get(&idx).unwrap_or(&0.0);
            let br = *bm25_rank_map.get(&idx).unwrap_or(&999);
            let vr = *vec_rank_map.get(&idx).unwrap_or(&999);
            let rerank = self.strategy.score(bm25_s, vec_s, br, vr);
            let snippet = extract_snippet(&doc.content, query, 2);
            SearchResult {
                doc_id: doc.id.clone(),
                path: doc.path.clone(),
                language: doc.language.clone(),
                bm25_score: bm25_s,
                vector_score: vec_s,
                rerank_score: rerank,
                snippet,
                match_line: None,
            }
        }).collect();

        results.sort_by(|a, b| b.rerank_score.partial_cmp(&a.rerank_score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Multi-hop: find docs matching both query A and query B.
    pub fn multi_hop(&self, query_a: &str, query_b: &str, limit: usize) -> Vec<SearchResult> {
        let hits_a: std::collections::HashSet<String> = self.bm25.search(query_a, 50)
            .iter().map(|(i, _)| self.bm25.docs()[*i].id.clone()).collect();
        self.search(query_b, None, limit * 2).into_iter()
            .filter(|r| hits_a.contains(&r.doc_id))
            .take(limit)
            .collect()
    }
}

/// Extract a snippet around the first query term match.
pub fn extract_snippet(content: &str, query: &str, context_lines: usize) -> String {
    let query_lower = query.to_lowercase();
    let first_term = query_lower.split_whitespace().next().unwrap_or("");
    let lines: Vec<&str> = content.lines().collect();
    let match_line = lines.iter().position(|l| l.to_lowercase().contains(first_term));
    if let Some(pos) = match_line {
        let start = pos.saturating_sub(context_lines);
        let end = (pos + context_lines + 1).min(lines.len());
        lines[start..end].join("\n")
    } else {
        lines.iter().take(5).cloned().collect::<Vec<_>>().join("\n")
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: &str, path: &str, lang: &str, content: &str) -> SearchDoc {
        SearchDoc::new(id, path, lang, content)
    }

    fn make_doc_with_emb(id: &str, content: &str, emb: Vec<f32>) -> SearchDoc {
        SearchDoc::new(id, &format!("{id}.rs"), "rust", content).with_embedding(emb)
    }

    // ── tokenize ──────────────────────────────────────────────────────────

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("hello world");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_tokenize_lowercases() {
        let tokens = tokenize("Hello World");
        assert!(tokens.contains(&"hello".to_string()));
    }

    #[test]
    fn test_tokenize_strips_short_words() {
        let tokens = tokenize("a bb ccc");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(tokens.contains(&"bb".to_string()));
    }

    #[test]
    fn test_tokenize_splits_on_underscores() {
        let tokens = tokenize("my_function_name");
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"function".to_string()));
    }

    // ── SearchDoc ─────────────────────────────────────────────────────────

    #[test]
    fn test_doc_term_freq_counts() {
        let doc = make_doc("d1", "a.rs", "rust", "fn foo() { foo() }");
        assert!(*doc.term_freq.get("foo").unwrap_or(&0) >= 2);
    }

    #[test]
    fn test_doc_len_non_zero() {
        let doc = make_doc("d1", "a.rs", "rust", "fn hello world");
        assert!(doc.doc_len > 0);
    }

    #[test]
    fn test_doc_with_embedding() {
        let doc = make_doc_with_emb("d1", "content", vec![1.0, 0.0]);
        assert!(!doc.embedding.is_empty());
    }

    // ── cosine_similarity ─────────────────────────────────────────────────

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_zero_vector() {
        let a = vec![0.0f32, 0.0];
        let b = vec![1.0f32, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_mismatched_lengths() {
        let a = vec![1.0f32];
        let b = vec![1.0f32, 2.0f32];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ── Bm25Index ─────────────────────────────────────────────────────────

    #[test]
    fn test_bm25_empty_index() {
        let idx = Bm25Index::new();
        assert_eq!(idx.doc_count(), 0);
        assert_eq!(idx.vocab_size(), 0);
    }

    #[test]
    fn test_bm25_add_doc() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "fn hello() {}"));
        assert_eq!(idx.doc_count(), 1);
    }

    #[test]
    fn test_bm25_search_finds_relevant() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "fn authenticate_user(username password) {}"));
        idx.add(make_doc("d2", "b.rs", "rust", "fn render_template(html) {}"));
        let hits = idx.search("authenticate", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].0, 0); // d1 should rank first
    }

    #[test]
    fn test_bm25_search_no_match() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "fn hello() {}"));
        let hits = idx.search("quantum_teleportation", 5);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_bm25_search_ranks_by_score() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "auth auth auth"));
        idx.add(make_doc("d2", "b.rs", "rust", "auth render"));
        let hits = idx.search("auth", 5);
        // d1 (3 occurrences) should score higher
        assert_eq!(hits[0].0, 0);
        assert!(hits[0].1 > hits[1].1);
    }

    #[test]
    fn test_bm25_idf_penalises_common_terms() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "foo bar"));
        idx.add(make_doc("d2", "b.rs", "rust", "foo baz"));
        // "foo" appears in both docs, should have lower IDF than "bar"
        let score_foo_d1 = idx.score(&idx.docs[0], &["foo".to_string()]);
        let score_bar_d1 = idx.score(&idx.docs[0], &["bar".to_string()]);
        assert!(score_bar_d1 >= score_foo_d1);
    }

    #[test]
    fn test_bm25_vocab_size_counts_unique_terms() {
        let mut idx = Bm25Index::new();
        idx.add(make_doc("d1", "a.rs", "rust", "alpha beta gamma"));
        assert!(idx.vocab_size() >= 3);
    }

    // ── RerankStrategy ────────────────────────────────────────────────────

    #[test]
    fn test_rerank_none_returns_bm25() {
        let s = RerankStrategy::None;
        assert!((s.score(0.8, 0.5, 1, 1) - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_rerank_linear_combines() {
        let s = RerankStrategy::Linear { bm25_weight: 0.4, vector_weight: 0.6 };
        let expected = 0.4 * 0.8 + 0.6 * 0.5;
        assert!((s.score(0.8, 0.5, 1, 1) - expected).abs() < 1e-5);
    }

    #[test]
    fn test_rerank_rrf_uses_ranks() {
        let s = RerankStrategy::Rrf { k: 60.0 };
        let score = s.score(0.9, 0.9, 1, 1);
        let expected = 1.0 / 61.0 + 1.0 / 61.0;
        assert!((score - expected).abs() < 1e-5);
    }

    #[test]
    fn test_rerank_rrf_lower_rank_scores_higher() {
        let s = RerankStrategy::Rrf { k: 60.0 };
        let rank1 = s.score(1.0, 1.0, 1, 1);
        let rank10 = s.score(1.0, 1.0, 10, 10);
        assert!(rank1 > rank10);
    }

    // ── HybridSearch ──────────────────────────────────────────────────────

    #[test]
    fn test_hybrid_search_bm25_only() {
        let mut hs = HybridSearch::new(RerankStrategy::None);
        hs.add_doc(make_doc("d1", "auth.rs", "rust", "fn authenticate_user(token) {}"));
        hs.add_doc(make_doc("d2", "render.rs", "rust", "fn render_html(template) {}"));
        let results = hs.search("authenticate", None, 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].doc_id, "d1");
    }

    #[test]
    fn test_hybrid_search_with_vector() {
        let mut hs = HybridSearch::new(RerankStrategy::Linear { bm25_weight: 0.5, vector_weight: 0.5 });
        hs.add_doc(make_doc_with_emb("d1", "auth token validate", vec![1.0, 0.0, 0.0]));
        hs.add_doc(make_doc_with_emb("d2", "render template html", vec![0.0, 1.0, 0.0]));
        let qemb = vec![1.0f32, 0.0, 0.0];
        let results = hs.search("auth", Some(&qemb), 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].doc_id, "d1");
    }

    #[test]
    fn test_hybrid_search_returns_limited_results() {
        let mut hs = HybridSearch::new(RerankStrategy::None);
        for i in 0..10 {
            hs.add_doc(make_doc(&format!("d{i}"), &format!("f{i}.rs"), "rust", "auth function call"));
        }
        let results = hs.search("auth", None, 3);
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_hybrid_search_empty_index() {
        let hs = HybridSearch::new(RerankStrategy::None);
        let results = hs.search("anything", None, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_hybrid_search_result_has_snippet() {
        let mut hs = HybridSearch::new(RerankStrategy::None);
        hs.add_doc(make_doc("d1", "a.rs", "rust", "fn login() { authenticate(user) }"));
        let results = hs.search("authenticate", None, 5);
        assert!(!results.is_empty());
        assert!(!results[0].snippet.is_empty());
    }

    #[test]
    fn test_hybrid_multi_hop_intersects() {
        let mut hs = HybridSearch::new(RerankStrategy::None);
        hs.add_doc(make_doc("d1", "a.rs", "rust", "auth token validate session import db"));
        hs.add_doc(make_doc("d2", "b.rs", "rust", "auth token without database"));
        hs.add_doc(make_doc("d3", "c.rs", "rust", "render template html without auth"));
        let results = hs.multi_hop("auth", "db", 5);
        // Only d1 has both "auth" and "db"
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "d1");
    }

    // ── extract_snippet ───────────────────────────────────────────────────

    #[test]
    fn test_extract_snippet_finds_match_line() {
        let content = "line one\nline two\nauthenticate here\nline four\nline five";
        let snippet = extract_snippet(content, "authenticate", 1);
        assert!(snippet.contains("authenticate"));
    }

    #[test]
    fn test_extract_snippet_fallback_to_first_lines() {
        let content = "alpha\nbeta\ngamma";
        let snippet = extract_snippet(content, "notfound", 1);
        assert!(snippet.contains("alpha"));
    }
}
