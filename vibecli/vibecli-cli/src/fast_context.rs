//! Optimized code context finder — Fast Context / SWE-grep.
//!
//! Provides rapid code search and context retrieval via trigram indexing,
//! symbol-aware search, structural queries (trait implementations, callers),
//! and LRU-cached ranked results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Core types ──────────────────────────────────────────────────────────────

/// How a result was matched.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchType {
    Exact,
    Fuzzy,
    Semantic,
    Structural,
    Symbol,
}

impl MatchType {
    /// Lower is better — used for default ranking.
    fn rank(&self) -> u32 {
        match self {
            Self::Exact => 0,
            Self::Symbol => 1,
            Self::Structural => 2,
            Self::Fuzzy => 3,
            Self::Semantic => 4,
        }
    }
}

/// A single search result with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResult {
    pub file_path: String,
    pub line_range: (usize, usize),
    pub snippet: String,
    pub relevance_score: f64,
    pub match_type: MatchType,
}

/// Metadata about a single indexed file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub file_path: String,
    pub symbols: Vec<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub last_modified: u64,
    pub file_hash: String,
    pub line_count: usize,
}

/// Maps file paths to their index entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileIndex {
    entries: HashMap<String, IndexEntry>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.file_path.clone(), entry);
    }

    pub fn remove(&mut self, path: &str) -> Option<IndexEntry> {
        self.entries.remove(path)
    }

    pub fn lookup(&self, path: &str) -> Option<&IndexEntry> {
        self.entries.get(path)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(|s| s.as_str())
    }

    pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.entries.values()
    }
}

// ── Trigram index ───────────────────────────────────────────────────────────

/// A trigram (3-character substring) used for fast substring matching.
pub type Trigram = [u8; 3];

/// Extracts trigrams from a string, lowercased.
fn extract_trigrams(s: &str) -> Vec<Trigram> {
    let lower = s.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    if bytes.len() < 3 {
        return Vec::new();
    }
    (0..bytes.len() - 2)
        .map(|i| [bytes[i], bytes[i + 1], bytes[i + 2]])
        .collect()
}

/// Maps trigrams to the set of file paths that contain them.
#[derive(Debug, Clone, Default)]
pub struct TrigramIndex {
    /// trigram → list of file paths
    map: HashMap<Trigram, Vec<String>>,
}

impl TrigramIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Index all symbols from an entry.
    pub fn add_entry(&mut self, entry: &IndexEntry) {
        let mut seen = std::collections::HashSet::new();
        for sym in &entry.symbols {
            for tri in extract_trigrams(sym) {
                if seen.insert(tri) {
                    self.map
                        .entry(tri)
                        .or_default()
                        .push(entry.file_path.clone());
                }
            }
        }
    }

    /// Remove all references to a file path.
    pub fn remove_path(&mut self, path: &str) {
        for paths in self.map.values_mut() {
            paths.retain(|p| p != path);
        }
        self.map.retain(|_, v| !v.is_empty());
    }

    /// Find files whose symbols match the query via trigram intersection.
    pub fn search(&self, query: &str) -> Vec<String> {
        let trigrams = extract_trigrams(query);
        if trigrams.is_empty() {
            return Vec::new();
        }

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for tri in &trigrams {
            if let Some(paths) = self.map.get(tri) {
                for p in paths {
                    *counts.entry(p.as_str()).or_default() += 1;
                }
            }
        }

        // Files matching all trigrams first, then by count descending.
        let total = trigrams.len();
        let mut results: Vec<(&str, usize)> = counts.into_iter().collect();
        results.sort_by(|a, b| {
            let a_full = (a.1 >= total) as u8;
            let b_full = (b.1 >= total) as u8;
            b_full.cmp(&a_full).then(b.1.cmp(&a.1))
        });
        results.into_iter().map(|(p, _)| p.to_string()).collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ── Search cache (LRU) ─────────────────────────────────────────────────────

/// Simple LRU cache for search results.
#[derive(Debug)]
pub struct SearchCache {
    capacity: usize,
    /// Ordered from oldest to newest.
    order: Vec<String>,
    entries: HashMap<String, Vec<ContextResult>>,
}

impl SearchCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: Vec::new(),
            entries: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&Vec<ContextResult>> {
        if self.entries.contains_key(key) {
            // Move to end (most recent).
            self.order.retain(|k| k != key);
            self.order.push(key.to_string());
            self.entries.get(key)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, results: Vec<ContextResult>) {
        if self.entries.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.first().cloned() {
                self.order.remove(0);
                self.entries.remove(&oldest);
            }
        }
        self.order.push(key.clone());
        self.entries.insert(key, results);
    }

    pub fn invalidate(&mut self, key: &str) {
        self.entries.remove(key);
        self.order.retain(|k| k != key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── FastGrep ────────────────────────────────────────────────────────────────

/// Optimized search engine with symbol-aware, structural, and trigram-based
/// search plus LRU caching.
#[derive(Debug)]
pub struct FastGrep {
    trigram_index: TrigramIndex,
    cache: SearchCache,
}

impl FastGrep {
    pub fn new(cache_capacity: usize) -> Self {
        Self {
            trigram_index: TrigramIndex::new(),
            cache: SearchCache::new(cache_capacity),
        }
    }

    pub fn index_entry(&mut self, entry: &IndexEntry) {
        self.trigram_index.add_entry(entry);
    }

    pub fn remove_path(&mut self, path: &str) {
        self.trigram_index.remove_path(path);
        self.cache.clear(); // invalidate all — path could appear in any cached result
    }

    /// Symbol-aware search: exact match on symbol names.
    pub fn search_symbol(
        &self,
        name: &str,
        index: &FileIndex,
    ) -> Vec<ContextResult> {
        let lower = name.to_ascii_lowercase();
        let mut results = Vec::new();

        for entry in index.entries() {
            for (i, sym) in entry.symbols.iter().enumerate() {
                let sym_lower = sym.to_ascii_lowercase();
                if sym_lower == lower {
                    results.push(ContextResult {
                        file_path: entry.file_path.clone(),
                        line_range: (i + 1, i + 1),
                        snippet: sym.clone(),
                        relevance_score: 1.0,
                        match_type: MatchType::Exact,
                    });
                } else if sym_lower.contains(&lower) {
                    results.push(ContextResult {
                        file_path: entry.file_path.clone(),
                        line_range: (i + 1, i + 1),
                        snippet: sym.clone(),
                        relevance_score: 0.7,
                        match_type: MatchType::Symbol,
                    });
                }
            }
        }

        Self::sort_results(&mut results);
        results
    }

    /// Structural search: find implementations of a trait.
    pub fn search_implementations(
        &self,
        trait_name: &str,
        index: &FileIndex,
    ) -> Vec<ContextResult> {
        let pattern_lower = format!("impl {}", trait_name.to_ascii_lowercase());
        let mut results = Vec::new();

        for entry in index.entries() {
            for (i, sym) in entry.symbols.iter().enumerate() {
                let sym_lower = sym.to_ascii_lowercase();
                if sym_lower.contains(&pattern_lower) || sym_lower == format!("impl_{}", trait_name.to_ascii_lowercase()) {
                    results.push(ContextResult {
                        file_path: entry.file_path.clone(),
                        line_range: (i + 1, i + 1),
                        snippet: sym.clone(),
                        relevance_score: 0.9,
                        match_type: MatchType::Structural,
                    });
                }
            }
        }

        Self::sort_results(&mut results);
        results
    }

    /// Structural search: find callers / references of a symbol.
    pub fn search_references(
        &self,
        symbol: &str,
        index: &FileIndex,
    ) -> Vec<ContextResult> {
        let lower = symbol.to_ascii_lowercase();
        let mut results = Vec::new();

        for entry in index.entries() {
            // Check imports for the symbol.
            let in_imports = entry.imports.iter().any(|imp| imp.to_ascii_lowercase().contains(&lower));
            let in_symbols = entry.symbols.iter().any(|s| {
                let sl = s.to_ascii_lowercase();
                sl != lower && sl.contains(&lower)
            });

            if in_imports || in_symbols {
                results.push(ContextResult {
                    file_path: entry.file_path.clone(),
                    line_range: (1, entry.line_count),
                    snippet: format!("references {}", symbol),
                    relevance_score: if in_imports { 0.85 } else { 0.75 },
                    match_type: MatchType::Structural,
                });
            }
        }

        Self::sort_results(&mut results);
        results
    }

    /// Trigram-based fuzzy search with cached results.
    pub fn search_fuzzy(
        &mut self,
        query: &str,
        index: &FileIndex,
        max_results: usize,
    ) -> Vec<ContextResult> {
        let cache_key = format!("fuzzy:{}", query);

        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.iter().take(max_results).cloned().collect();
        }

        let trigram_hits = self.trigram_index.search(query);
        let lower = query.to_ascii_lowercase();
        let mut results = Vec::new();

        for path in &trigram_hits {
            if let Some(entry) = index.lookup(path) {
                for (i, sym) in entry.symbols.iter().enumerate() {
                    let sym_lower = sym.to_ascii_lowercase();
                    if sym_lower == lower {
                        results.push(ContextResult {
                            file_path: path.clone(),
                            line_range: (i + 1, i + 1),
                            snippet: sym.clone(),
                            relevance_score: 1.0,
                            match_type: MatchType::Exact,
                        });
                    } else if sym_lower.contains(&lower) {
                        results.push(ContextResult {
                            file_path: path.clone(),
                            line_range: (i + 1, i + 1),
                            snippet: sym.clone(),
                            relevance_score: 0.7,
                            match_type: MatchType::Symbol,
                        });
                    } else {
                        // Fuzzy trigram match.
                        let score = trigram_similarity(sym, query);
                        if score > 0.3 {
                            results.push(ContextResult {
                                file_path: path.clone(),
                                line_range: (i + 1, i + 1),
                                snippet: sym.clone(),
                                relevance_score: score,
                                match_type: MatchType::Fuzzy,
                            });
                        }
                    }
                }
            }
        }

        Self::sort_results(&mut results);
        self.cache.insert(cache_key, results.clone());
        results.into_iter().take(max_results).collect()
    }

    /// Sort results: match-type rank ascending, then relevance descending.
    fn sort_results(results: &mut [ContextResult]) {
        results.sort_by(|a, b| {
            a.match_type
                .rank()
                .cmp(&b.match_type.rank())
                .then(b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal))
        });
    }
}

/// Trigram-based similarity: |intersection| / |union| of trigram sets.
fn trigram_similarity(a: &str, b: &str) -> f64 {
    let ta: std::collections::HashSet<Trigram> = extract_trigrams(a).into_iter().collect();
    let tb: std::collections::HashSet<Trigram> = extract_trigrams(b).into_iter().collect();

    if ta.is_empty() && tb.is_empty() {
        return 0.0;
    }

    let intersection = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

// ── FastContextEngine ───────────────────────────────────────────────────────

/// Top-level engine: manages a `FileIndex`, `FastGrep`, and provides the
/// high-level `ContextFinder` API.
#[derive(Debug)]
pub struct FastContextEngine {
    index: FileIndex,
    grep: FastGrep,
}

impl FastContextEngine {
    pub fn new(cache_capacity: usize) -> Self {
        Self {
            index: FileIndex::new(),
            grep: FastGrep::new(cache_capacity),
        }
    }

    /// Build (or rebuild) an index from a list of entries.
    pub fn build_index(&mut self, entries: Vec<IndexEntry>) -> &FileIndex {
        self.index = FileIndex::new();
        self.grep = FastGrep::new(self.grep.cache.capacity);
        for entry in entries {
            self.grep.index_entry(&entry);
            self.index.add(entry);
        }
        &self.index
    }

    /// Add a single entry to the index.
    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.grep.index_entry(&entry);
        self.index.add(entry);
    }

    /// Invalidate a path for incremental updates.
    pub fn invalidate(&mut self, path: &str) {
        self.index.remove(path);
        self.grep.remove_path(path);
    }

    /// General-purpose context search combining symbol + trigram search.
    pub fn find_relevant(&mut self, query: &str, max_results: usize) -> Vec<ContextResult> {
        let mut results = self.grep.search_symbol(query, &self.index);
        let fuzzy = self.grep.search_fuzzy(query, &self.index, max_results);

        // Merge, dedup by (file_path, snippet).
        let mut seen = std::collections::HashSet::new();
        for r in &results {
            seen.insert((r.file_path.clone(), r.snippet.clone()));
        }
        for r in fuzzy {
            if seen.insert((r.file_path.clone(), r.snippet.clone())) {
                results.push(r);
            }
        }

        FastGrep::sort_results(&mut results);
        results.truncate(max_results);
        results
    }

    /// Find a symbol by exact or partial name.
    pub fn find_symbol(&self, name: &str) -> Vec<ContextResult> {
        self.grep.search_symbol(name, &self.index)
    }

    /// Find references to a symbol.
    pub fn find_references(&self, symbol: &str) -> Vec<ContextResult> {
        self.grep.search_references(symbol, &self.index)
    }

    /// Find implementations of a trait.
    pub fn find_implementations(&self, trait_name: &str) -> Vec<ContextResult> {
        self.grep.search_implementations(trait_name, &self.index)
    }

    /// Access the underlying file index.
    pub fn file_index(&self) -> &FileIndex {
        &self.index
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(path: &str, symbols: &[&str]) -> IndexEntry {
        IndexEntry {
            file_path: path.to_string(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            imports: vec!["use std::collections::HashMap".to_string()],
            exports: vec!["pub fn main".to_string()],
            last_modified: 1000,
            file_hash: "abc123".to_string(),
            line_count: 100,
        }
    }

    // ── FileIndex tests ─────────────────────────────────────────────────

    #[test]
    fn test_file_index_add_and_lookup() {
        let mut idx = FileIndex::new();
        idx.add(sample_entry("src/main.rs", &["main", "run"]));
        assert_eq!(idx.len(), 1);
        assert!(idx.lookup("src/main.rs").is_some());
        assert!(idx.lookup("src/lib.rs").is_none());
    }

    #[test]
    fn test_file_index_remove() {
        let mut idx = FileIndex::new();
        idx.add(sample_entry("a.rs", &["foo"]));
        idx.add(sample_entry("b.rs", &["bar"]));
        assert_eq!(idx.len(), 2);
        let removed = idx.remove("a.rs");
        assert!(removed.is_some());
        assert_eq!(idx.len(), 1);
        assert!(idx.lookup("a.rs").is_none());
    }

    #[test]
    fn test_file_index_overwrite() {
        let mut idx = FileIndex::new();
        idx.add(sample_entry("a.rs", &["v1"]));
        idx.add(sample_entry("a.rs", &["v2"]));
        assert_eq!(idx.len(), 1);
        let entry = idx.lookup("a.rs").expect("should exist");
        assert_eq!(entry.symbols, vec!["v2".to_string()]);
    }

    #[test]
    fn test_file_index_is_empty() {
        let idx = FileIndex::new();
        assert!(idx.is_empty());
    }

    #[test]
    fn test_file_index_paths() {
        let mut idx = FileIndex::new();
        idx.add(sample_entry("a.rs", &[]));
        idx.add(sample_entry("b.rs", &[]));
        let mut paths: Vec<&str> = idx.paths().collect();
        paths.sort();
        assert_eq!(paths, vec!["a.rs", "b.rs"]);
    }

    // ── Trigram tests ───────────────────────────────────────────────────

    #[test]
    fn test_extract_trigrams_basic() {
        let tris = extract_trigrams("hello");
        // h-e-l, e-l-l, l-l-o
        assert_eq!(tris.len(), 3);
    }

    #[test]
    fn test_extract_trigrams_short_string() {
        assert!(extract_trigrams("ab").is_empty());
        assert!(extract_trigrams("").is_empty());
    }

    #[test]
    fn test_extract_trigrams_case_insensitive() {
        let a = extract_trigrams("Hello");
        let b = extract_trigrams("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn test_trigram_index_search() {
        let mut ti = TrigramIndex::new();
        ti.add_entry(&sample_entry("main.rs", &["process_data", "parse_input"]));
        ti.add_entry(&sample_entry("lib.rs", &["process_result", "transform"]));

        let results = ti.search("process");
        assert!(results.contains(&"main.rs".to_string()));
        assert!(results.contains(&"lib.rs".to_string()));
    }

    #[test]
    fn test_trigram_index_remove() {
        let mut ti = TrigramIndex::new();
        ti.add_entry(&sample_entry("a.rs", &["foobar"]));
        assert!(!ti.is_empty());
        ti.remove_path("a.rs");
        assert!(ti.search("foobar").is_empty());
    }

    #[test]
    fn test_trigram_similarity() {
        let score = trigram_similarity("process_data", "process_result");
        assert!(score > 0.0 && score < 1.0);
        let exact = trigram_similarity("hello", "hello");
        assert!((exact - 1.0).abs() < f64::EPSILON);
    }

    // ── SearchCache tests ───────────────────────────────────────────────

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SearchCache::new(2);
        cache.insert("q1".to_string(), vec![]);
        assert!(cache.get("q1").is_some());
        assert!(cache.get("q2").is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = SearchCache::new(2);
        cache.insert("q1".to_string(), vec![]);
        cache.insert("q2".to_string(), vec![]);
        cache.insert("q3".to_string(), vec![]); // evicts q1
        assert!(cache.get("q1").is_none());
        assert!(cache.get("q2").is_some());
        assert!(cache.get("q3").is_some());
    }

    #[test]
    fn test_cache_lru_access_refreshes() {
        let mut cache = SearchCache::new(2);
        cache.insert("q1".to_string(), vec![]);
        cache.insert("q2".to_string(), vec![]);
        // Access q1 so it becomes most-recent.
        let _ = cache.get("q1");
        cache.insert("q3".to_string(), vec![]); // evicts q2 (oldest)
        assert!(cache.get("q1").is_some());
        assert!(cache.get("q2").is_none());
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = SearchCache::new(4);
        cache.insert("q1".to_string(), vec![]);
        cache.invalidate("q1");
        assert!(cache.get("q1").is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = SearchCache::new(4);
        cache.insert("a".to_string(), vec![]);
        cache.insert("b".to_string(), vec![]);
        cache.clear();
        assert!(cache.is_empty());
    }

    // ── FastGrep tests ──────────────────────────────────────────────────

    #[test]
    fn test_fast_grep_symbol_search_exact() {
        let mut fg = FastGrep::new(8);
        let entry = sample_entry("src/lib.rs", &["parse_config", "run_server"]);
        fg.index_entry(&entry);
        let mut idx = FileIndex::new();
        idx.add(entry);

        let results = fg.search_symbol("parse_config", &idx);
        assert!(!results.is_empty());
        assert_eq!(results[0].match_type, MatchType::Exact);
    }

    #[test]
    fn test_fast_grep_symbol_search_partial() {
        let mut fg = FastGrep::new(8);
        let entry = sample_entry("src/lib.rs", &["parse_config"]);
        fg.index_entry(&entry);
        let mut idx = FileIndex::new();
        idx.add(entry);

        let results = fg.search_symbol("parse", &idx);
        assert!(!results.is_empty());
        assert_eq!(results[0].match_type, MatchType::Symbol);
    }

    #[test]
    fn test_fast_grep_implementations() {
        let entry = IndexEntry {
            file_path: "src/provider.rs".to_string(),
            symbols: vec!["impl AIProvider for Ollama".to_string(), "OllamaConfig".to_string()],
            imports: vec![],
            exports: vec![],
            last_modified: 0,
            file_hash: String::new(),
            line_count: 50,
        };
        let fg = FastGrep::new(4);
        let mut idx = FileIndex::new();
        idx.add(entry);

        let results = fg.search_implementations("AIProvider", &idx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_type, MatchType::Structural);
    }

    #[test]
    fn test_fast_grep_references() {
        let entry = IndexEntry {
            file_path: "src/main.rs".to_string(),
            symbols: vec!["run_server".to_string()],
            imports: vec!["use crate::config::parse_config".to_string()],
            exports: vec![],
            last_modified: 0,
            file_hash: String::new(),
            line_count: 30,
        };
        let fg = FastGrep::new(4);
        let mut idx = FileIndex::new();
        idx.add(entry);

        let results = fg.search_references("parse_config", &idx);
        assert!(!results.is_empty());
        assert_eq!(results[0].match_type, MatchType::Structural);
    }

    // ── FastContextEngine tests ─────────────────────────────────────────

    #[test]
    fn test_engine_build_index() {
        let mut engine = FastContextEngine::new(8);
        let entries = vec![
            sample_entry("a.rs", &["foo", "bar"]),
            sample_entry("b.rs", &["baz"]),
        ];
        engine.build_index(entries);
        assert_eq!(engine.file_index().len(), 2);
    }

    #[test]
    fn test_engine_find_relevant() {
        let mut engine = FastContextEngine::new(8);
        engine.add_entry(sample_entry("a.rs", &["parse_config", "run"]));
        engine.add_entry(sample_entry("b.rs", &["parse_input", "transform"]));

        let results = engine.find_relevant("parse", 10);
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_engine_find_symbol() {
        let mut engine = FastContextEngine::new(8);
        engine.add_entry(sample_entry("x.rs", &["compute_hash"]));
        let results = engine.find_symbol("compute_hash");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_type, MatchType::Exact);
    }

    #[test]
    fn test_engine_invalidate() {
        let mut engine = FastContextEngine::new(8);
        engine.add_entry(sample_entry("a.rs", &["hello"]));
        engine.invalidate("a.rs");
        assert!(engine.file_index().lookup("a.rs").is_none());
        assert!(engine.find_symbol("hello").is_empty());
    }

    #[test]
    fn test_engine_incremental_update() {
        let mut engine = FastContextEngine::new(8);
        engine.add_entry(sample_entry("a.rs", &["old_symbol"]));
        assert!(!engine.find_symbol("old_symbol").is_empty());

        engine.invalidate("a.rs");
        engine.add_entry(sample_entry("a.rs", &["new_symbol"]));
        assert!(engine.find_symbol("old_symbol").is_empty());
        assert!(!engine.find_symbol("new_symbol").is_empty());
    }

    #[test]
    fn test_relevance_ranking_order() {
        let mut engine = FastContextEngine::new(8);
        engine.add_entry(IndexEntry {
            file_path: "a.rs".to_string(),
            symbols: vec!["process".to_string(), "process_data".to_string()],
            imports: vec![],
            exports: vec![],
            last_modified: 0,
            file_hash: String::new(),
            line_count: 10,
        });

        let results = engine.find_relevant("process", 10);
        // Exact match should come before partial/symbol match.
        assert!(!results.is_empty());
        assert_eq!(results[0].match_type, MatchType::Exact);
    }

    #[test]
    fn test_find_implementations_via_engine() {
        let mut engine = FastContextEngine::new(4);
        engine.add_entry(IndexEntry {
            file_path: "provider.rs".to_string(),
            symbols: vec!["impl Display for Foo".to_string()],
            imports: vec![],
            exports: vec![],
            last_modified: 0,
            file_hash: String::new(),
            line_count: 20,
        });
        let results = engine.find_implementations("Display");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_references_via_engine() {
        let mut engine = FastContextEngine::new(4);
        engine.add_entry(IndexEntry {
            file_path: "consumer.rs".to_string(),
            symbols: vec!["do_stuff".to_string()],
            imports: vec!["use crate::helpers::compute".to_string()],
            exports: vec![],
            last_modified: 0,
            file_hash: String::new(),
            line_count: 15,
        });
        let results = engine.find_references("compute");
        assert_eq!(results.len(), 1);
    }
}
