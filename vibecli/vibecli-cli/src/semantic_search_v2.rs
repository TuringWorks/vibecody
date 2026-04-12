#![allow(dead_code)]
//! Semantic search v2 — ranked multi-strategy code search with hybrid scoring.
//! FIT-GAP v11 Phase 48 — closes gap vs all major competitors.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Search strategy used to find a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchStrategy {
    Lexical,    // keyword / regex match
    Structural, // AST / symbol match
    Semantic,   // embedding similarity
    Hybrid,     // combination of multiple strategies
}

impl SearchStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Lexical => "lexical",
            Self::Structural => "structural",
            Self::Semantic => "semantic",
            Self::Hybrid => "hybrid",
        }
    }
}

/// A single indexed code chunk.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub id: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub language: String,
    /// Pre-computed keyword tokens for lexical search.
    pub tokens: Vec<String>,
    /// Mock embedding vector (cosine similarity in tests).
    pub embedding: Vec<f32>,
}

impl CodeChunk {
    pub fn new(
        id: impl Into<String>,
        file: impl Into<String>,
        start: usize,
        end: usize,
        content: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        let content = content.into();
        let tokens = tokenize(&content);
        Self {
            id: id.into(),
            file: file.into(),
            start_line: start,
            end_line: end,
            content,
            language: language.into(),
            tokens,
            embedding: Vec::new(),
        }
    }

    pub fn with_embedding(mut self, v: Vec<f32>) -> Self {
        self.embedding = v;
        self
    }
}

/// A ranked search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk: CodeChunk,
    pub score: f32,
    pub strategy: SearchStrategy,
    pub matched_terms: Vec<String>,
}

/// Search configuration.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub max_results: usize,
    pub min_score: f32,
    pub strategies: Vec<SearchStrategy>,
    pub language_filter: Option<String>,
    pub file_filter: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 20,
            min_score: 0.1,
            strategies: vec![SearchStrategy::Hybrid],
            language_filter: None,
            file_filter: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

/// In-memory search index.
#[derive(Debug, Default)]
pub struct SearchIndex {
    chunks: Vec<CodeChunk>,
    /// token → list of chunk indices (inverted index)
    inverted: HashMap<String, Vec<usize>>,
}

impl SearchIndex {
    pub fn new() -> Self { Self::default() }

    /// Add a chunk to the index.
    pub fn index(&mut self, chunk: CodeChunk) {
        let idx = self.chunks.len();
        for token in &chunk.tokens {
            self.inverted.entry(token.clone()).or_default().push(idx);
        }
        self.chunks.push(chunk);
    }

    /// Remove all chunks for a given file.
    pub fn remove_file(&mut self, file: &str) {
        // Rebuild without the file's chunks (simple approach for small indexes).
        let kept: Vec<CodeChunk> = self.chunks.drain(..).filter(|c| c.file != file).collect();
        self.inverted.clear();
        for chunk in kept {
            let idx = self.chunks.len();
            for token in &chunk.tokens {
                self.inverted.entry(token.clone()).or_default().push(idx);
            }
            self.chunks.push(chunk);
        }
    }

    pub fn chunk_count(&self) -> usize { self.chunks.len() }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Multi-strategy semantic search engine.
pub struct SemanticSearchV2 {
    index: SearchIndex,
}

impl SemanticSearchV2 {
    pub fn new() -> Self {
        Self { index: SearchIndex::new() }
    }

    pub fn index_chunk(&mut self, chunk: CodeChunk) {
        self.index.index(chunk);
    }

    pub fn remove_file(&mut self, file: &str) {
        self.index.remove_file(file);
    }

    /// Execute a search with the given query and config.
    pub fn search(&self, query: &str, config: &SearchConfig) -> Vec<SearchResult> {
        let query_tokens = tokenize(query);
        let query_embedding = mock_embed(query);

        let mut scored: Vec<(usize, f32, SearchStrategy, Vec<String>)> = Vec::new();

        for (idx, chunk) in self.index.chunks.iter().enumerate() {
            // Language filter
            if let Some(ref lang) = config.language_filter {
                if &chunk.language != lang { continue; }
            }
            // File filter
            if let Some(ref filt) = config.file_filter {
                if !chunk.file.contains(filt.as_str()) { continue; }
            }

            let mut best_score = 0.0f32;
            let mut best_strategy = SearchStrategy::Lexical;
            let mut matched = Vec::new();

            for &strategy in &config.strategies {
                let (score, terms) = match strategy {
                    SearchStrategy::Lexical => lexical_score(chunk, &query_tokens),
                    SearchStrategy::Structural => structural_score(chunk, query),
                    SearchStrategy::Semantic => semantic_score(chunk, &query_embedding),
                    SearchStrategy::Hybrid => {
                        let (ls, lt) = lexical_score(chunk, &query_tokens);
                        let (ss, _) = semantic_score(chunk, &query_embedding);
                        let (strs, _) = structural_score(chunk, query);
                        (ls * 0.4 + ss * 0.4 + strs * 0.2, lt)
                    }
                };
                if score > best_score {
                    best_score = score;
                    best_strategy = strategy;
                    matched = terms;
                }
            }

            if best_score >= config.min_score {
                scored.push((idx, best_score, best_strategy, matched));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(config.max_results);

        scored.into_iter().map(|(idx, score, strategy, matched_terms)| {
            SearchResult {
                chunk: self.index.chunks[idx].clone(),
                score,
                strategy,
                matched_terms,
            }
        }).collect()
    }

    /// Build a context window from top results.
    pub fn build_context(&self, results: &[SearchResult], max_chars: usize) -> String {
        let mut buf = String::new();
        for r in results {
            let snippet = format!(
                "// {} [{}:{}–{}] score={:.2}\n{}\n\n",
                r.chunk.file,
                r.chunk.language,
                r.chunk.start_line,
                r.chunk.end_line,
                r.score,
                r.chunk.content
            );
            if buf.len() + snippet.len() > max_chars { break; }
            buf.push_str(&snippet);
        }
        buf
    }

    pub fn chunk_count(&self) -> usize { self.index.chunk_count() }
}

impl Default for SemanticSearchV2 {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect()
}

fn lexical_score(chunk: &CodeChunk, query_tokens: &[String]) -> (f32, Vec<String>) {
    let mut matched = Vec::new();
    for qt in query_tokens {
        if chunk.tokens.iter().any(|t| t == qt) {
            matched.push(qt.clone());
        }
    }
    let score = if query_tokens.is_empty() {
        0.0
    } else {
        matched.len() as f32 / query_tokens.len() as f32
    };
    (score, matched)
}

fn structural_score(chunk: &CodeChunk, query: &str) -> (f32, Vec<String>) {
    // Structural: check if query looks like a symbol name and appears in content.
    let score = if chunk.content.contains(query) { 0.8 }
    else if chunk.content.to_lowercase().contains(&query.to_lowercase()) { 0.5 }
    else { 0.0 };
    (score, Vec::new())
}

fn semantic_score(chunk: &CodeChunk, query_embedding: &[f32]) -> (f32, Vec<String>) {
    if chunk.embedding.is_empty() || query_embedding.is_empty() {
        return (0.0, Vec::new());
    }
    let score = cosine_similarity(query_embedding, &chunk.embedding);
    (score.max(0.0), Vec::new())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let dot: f32 = a[..len].iter().zip(b[..len].iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

/// Mock embedding: bag-of-tokens using hash positions.
fn mock_embed(text: &str) -> Vec<f32> {
    let tokens = tokenize(text);
    let mut v = vec![0.0f32; 32];
    for (i, t) in tokens.iter().enumerate() {
        let h: usize = t.bytes().fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
        v[h % 32] += 1.0 / (i + 1) as f32;
    }
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &str, file: &str, content: &str, lang: &str) -> CodeChunk {
        CodeChunk::new(id, file, 1, 10, content, lang)
    }

    fn emb_chunk(id: &str, file: &str, content: &str, lang: &str) -> CodeChunk {
        let emb = mock_embed(content);
        CodeChunk::new(id, file, 1, 10, content, lang).with_embedding(emb)
    }

    #[test]
    fn test_index_and_count() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/lib.rs", "fn add(a: i32) -> i32 { a + 1 }", "rust"));
        assert_eq!(eng.chunk_count(), 1);
    }

    #[test]
    fn test_lexical_search() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/lib.rs", "fn add_numbers(a: i32, b: i32) -> i32 { a + b }", "rust"));
        eng.index_chunk(chunk("c2", "src/main.rs", "fn greet(name: &str) { println!(\"Hello\"); }", "rust"));
        let config = SearchConfig { strategies: vec![SearchStrategy::Lexical], ..Default::default() };
        // Use tokens that appear literally in c1: "add_numbers" and "i32"
        let results = eng.search("add_numbers i32", &config);
        assert!(!results.is_empty());
        assert_eq!(results[0].chunk.id, "c1");
    }

    #[test]
    fn test_structural_search() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/lib.rs", "pub fn serialize_json(data: &Value) -> String { }", "rust"));
        let config = SearchConfig { strategies: vec![SearchStrategy::Structural], ..Default::default() };
        let results = eng.search("serialize_json", &config);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_semantic_search() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(emb_chunk("c1", "src/auth.rs", "fn validate_token(token: &str) -> bool { true }", "rust"));
        let config = SearchConfig { strategies: vec![SearchStrategy::Semantic], ..Default::default() };
        let results = eng.search("validate token auth", &config);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hybrid_search() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(emb_chunk("c1", "src/db.rs", "fn query_users(filter: &str) -> Vec<User> { vec![] }", "rust"));
        let config = SearchConfig::default();
        let results = eng.search("query users database", &config);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_language_filter() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/lib.rs", "fn add() {}", "rust"));
        eng.index_chunk(chunk("c2", "src/lib.ts", "function add() {}", "typescript"));
        let config = SearchConfig {
            strategies: vec![SearchStrategy::Lexical],
            language_filter: Some("typescript".to_string()),
            ..Default::default()
        };
        let results = eng.search("add", &config);
        assert!(results.iter().all(|r| r.chunk.language == "typescript"));
    }

    #[test]
    fn test_file_filter() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/auth.rs", "fn authenticate() {}", "rust"));
        eng.index_chunk(chunk("c2", "src/db.rs", "fn authenticate() {}", "rust"));
        let config = SearchConfig {
            strategies: vec![SearchStrategy::Lexical],
            file_filter: Some("auth".to_string()),
            ..Default::default()
        };
        let results = eng.search("authenticate", &config);
        assert!(results.iter().all(|r| r.chunk.file.contains("auth")));
    }

    #[test]
    fn test_min_score_filter() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/unrelated.rs", "fn something_else() {}", "rust"));
        let config = SearchConfig {
            strategies: vec![SearchStrategy::Lexical],
            min_score: 0.9, // Very high threshold
            ..Default::default()
        };
        let results = eng.search("query_users", &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_max_results() {
        let mut eng = SemanticSearchV2::new();
        for i in 0..20 {
            eng.index_chunk(chunk(&format!("c{}", i), "src/lib.rs", "fn compute() {}", "rust"));
        }
        let config = SearchConfig {
            max_results: 5,
            strategies: vec![SearchStrategy::Lexical],
            ..Default::default()
        };
        let results = eng.search("compute", &config);
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_remove_file() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/auth.rs", "fn login() {}", "rust"));
        eng.index_chunk(chunk("c2", "src/db.rs", "fn login() {}", "rust"));
        eng.remove_file("src/auth.rs");
        assert_eq!(eng.chunk_count(), 1);
    }

    #[test]
    fn test_build_context() {
        let mut eng = SemanticSearchV2::new();
        eng.index_chunk(chunk("c1", "src/lib.rs", "fn add(a: i32) -> i32 { a + 1 }", "rust"));
        let config = SearchConfig::default();
        let results = eng.search("add", &config);
        let ctx = eng.build_context(&results, 10000);
        assert!(ctx.contains("src/lib.rs"));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 1.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn test_tokenize() {
        let t = tokenize("fn add_numbers(a: i32)");
        assert!(t.contains(&"fn".to_string()));
        assert!(t.contains(&"add_numbers".to_string()));
    }
}
