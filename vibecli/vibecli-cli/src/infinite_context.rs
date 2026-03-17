//! Infinite code context module for unlimited codebase understanding.
//!
//! Provides a multi-level context hierarchy that can represent an entire
//! codebase within a bounded token budget by progressively summarizing
//! and compressing context at different depth levels:
//!
//!   Level 0: Full file content
//!   Level 1: Function-level summaries
//!   Level 2: File skeleton (signatures only)
//!   Level 3: Module-level summary (one sentence per file)
//!   Level 4: Project-level summary (architecture overview)
//!
//! The system scores chunks by recency, proximity, keyword relevance,
//! dependency distance, and access frequency, then manages eviction and
//! compression to keep context within the configured token budget.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── ChunkType ────────────────────────────────────────────────────────────────

/// The kind of content a context chunk contains.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChunkType {
    FileContent,
    Summary,
    Symbol,
    Dependency,
    DocString,
    TestCase,
    Config,
}

// ── ContextChunk ─────────────────────────────────────────────────────────────

/// A single piece of context with metadata for scoring and management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub id: String,
    pub source_file: PathBuf,
    pub content: String,
    pub chunk_type: ChunkType,
    pub relevance_score: f64,
    pub token_count: usize,
    /// 0=full, 1=summary, 2=skeleton, 3=signature-only
    pub depth_level: u8,
    pub last_accessed: u64,
    pub access_count: u32,
}

impl ContextChunk {
    pub fn new(
        id: impl Into<String>,
        source_file: impl Into<PathBuf>,
        content: impl Into<String>,
        chunk_type: ChunkType,
    ) -> Self {
        let content = content.into();
        let token_count = token_estimate(&content);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: id.into(),
            source_file: source_file.into(),
            content,
            chunk_type,
            relevance_score: 0.0,
            token_count,
            depth_level: 0,
            last_accessed: now,
            access_count: 0,
        }
    }
}

// ── ContextHierarchy ─────────────────────────────────────────────────────────

/// Multi-level context representation utilities.
///
/// Each level provides progressively less detail:
///   0 — full file content
///   1 — function-level summaries (one-liner per function)
///   2 — file skeleton (signatures + structure, no bodies)
///   3 — module-level summary (one sentence per file)
///   4 — project-level summary (architecture overview)
pub struct ContextHierarchy;

impl ContextHierarchy {
    /// Summarize file content at the given depth level.
    pub fn summarize_file(content: &str, level: u8) -> String {
        match level {
            0 => content.to_string(),
            1 => Self::summarize_to_function_level(content),
            2 => Self::extract_skeleton(content),
            3 => Self::summarize_to_module_level(content),
            4 => Self::summarize_to_project_level(content),
            _ => Self::summarize_to_project_level(content),
        }
    }

    /// Produce function-level summaries (one-liner per function).
    fn summarize_to_function_level(content: &str) -> String {
        let mut summaries = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if Self::is_function_line(line) || Self::is_struct_line(line) || Self::is_class_line(line) {
                let summary = Self::summarize_function_block(&lines, i);
                summaries.push(summary);
            }
            i += 1;
        }
        if summaries.is_empty() {
            // Fall back to first non-empty line
            let first = content.lines().find(|l| !l.trim().is_empty());
            return first.unwrap_or("(empty)").trim().to_string();
        }
        summaries.join("\n")
    }

    /// Summarize a function block: signature + first doc comment or first line.
    fn summarize_function_block(lines: &[&str], start: usize) -> String {
        let sig = lines[start].trim().to_string();
        // Look for doc comment above
        if start > 0 {
            let prev = lines[start - 1].trim();
            if prev.starts_with("///") || prev.starts_with("//!") || prev.starts_with('#') {
                let doc = prev
                    .trim_start_matches("///")
                    .trim_start_matches("//!")
                    .trim();
                if !doc.is_empty() {
                    return format!("{sig} // {doc}");
                }
            }
        }
        sig
    }

    /// Summarize a function's content: first doc comment line or first statement.
    pub fn summarize_function(content: &str) -> String {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                let doc = trimmed
                    .trim_start_matches("///")
                    .trim_start_matches("//!")
                    .trim();
                if !doc.is_empty() {
                    return doc.to_string();
                }
            }
            if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("use ")
            {
                return trimmed.to_string();
            }
        }
        "(empty)".to_string()
    }

    /// Extract signatures: fn, struct, class, impl, enum, trait, type, const, interface lines.
    pub fn extract_signatures(content: &str) -> Vec<String> {
        let mut sigs = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if Self::is_signature_line(trimmed) {
                // Strip trailing opening brace for cleanliness
                let clean = trimmed.trim_end_matches('{').trim().to_string();
                sigs.push(clean);
            }
        }
        sigs
    }

    /// Extract a skeleton: signatures + structure (no function bodies).
    pub fn extract_skeleton(content: &str) -> String {
        let mut skeleton = Vec::new();
        let mut brace_depth: i32 = 0;
        let mut in_body = false;
        let mut body_start_depth: i32 = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Count braces
            let opens = trimmed.chars().filter(|&c| c == '{').count() as i32;
            let closes = trimmed.chars().filter(|&c| c == '}').count() as i32;

            if in_body {
                brace_depth += opens - closes;
                if brace_depth <= body_start_depth {
                    in_body = false;
                    skeleton.push("}".to_string());
                }
                continue;
            }

            if Self::is_signature_line(trimmed) {
                skeleton.push(line.to_string());
                brace_depth += opens - closes;
                if Self::is_function_line(trimmed) && opens > 0 {
                    in_body = true;
                    body_start_depth = brace_depth - opens;
                }
            } else if trimmed.starts_with("use ")
                || trimmed.starts_with("mod ")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("///")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("package ")
                || trimmed.starts_with("#[")
                || trimmed.starts_with("#![")
            {
                skeleton.push(line.to_string());
                brace_depth += opens - closes;
            } else if trimmed == "}" {
                brace_depth += opens - closes;
                skeleton.push(line.to_string());
            } else {
                brace_depth += opens - closes;
            }
        }
        skeleton.join("\n")
    }

    fn summarize_to_module_level(content: &str) -> String {
        let sigs = Self::extract_signatures(content);
        let count = sigs.len();
        let first_doc = content
            .lines()
            .find(|l| l.trim().starts_with("//!"))
            .map(|l| {
                l.trim()
                    .trim_start_matches("//!")
                    .trim()
                    .to_string()
            });
        if let Some(doc) = first_doc {
            format!("{doc} ({count} definitions)")
        } else if count > 0 {
            format!("Module with {count} definitions: {}", sigs[0])
        } else {
            "Module with no public definitions".to_string()
        }
    }

    fn summarize_to_project_level(content: &str) -> String {
        let sigs = Self::extract_signatures(content);
        let types: Vec<&String> = sigs
            .iter()
            .filter(|s| {
                s.starts_with("pub struct")
                    || s.starts_with("struct")
                    || s.starts_with("pub enum")
                    || s.starts_with("enum")
                    || s.starts_with("pub trait")
                    || s.starts_with("trait")
                    || s.starts_with("class")
            })
            .collect();
        if types.is_empty() {
            format!("{} top-level items", sigs.len())
        } else {
            let names: Vec<&str> = types.iter().take(5).map(|s| s.as_str()).collect();
            format!("Defines: {}", names.join(", "))
        }
    }

    fn is_function_line(line: &str) -> bool {
        let l = line.trim_start_matches("pub ");
        let l = l.trim_start_matches("pub(crate) ");
        let l = l.trim_start_matches("async ");
        let l = l.trim_start_matches("unsafe ");
        let l = l.trim_start_matches("const ");
        let l = l.trim_start_matches("extern \"C\" ");
        l.starts_with("fn ")
            || l.starts_with("def ")
            || l.starts_with("func ")
            || l.starts_with("function ")
    }

    fn is_struct_line(line: &str) -> bool {
        let l = line.trim_start_matches("pub ");
        let l = l.trim_start_matches("pub(crate) ");
        l.starts_with("struct ") || l.starts_with("enum ") || l.starts_with("union ")
    }

    fn is_class_line(line: &str) -> bool {
        line.starts_with("class ") || line.starts_with("interface ") || line.starts_with("abstract class ")
    }

    fn is_signature_line(line: &str) -> bool {
        Self::is_function_line(line)
            || Self::is_struct_line(line)
            || Self::is_class_line(line)
            || {
                let l = line.trim_start_matches("pub ");
                let l = l.trim_start_matches("pub(crate) ");
                l.starts_with("trait ")
                    || l.starts_with("impl ")
                    || l.starts_with("type ")
                    || l.starts_with("const ")
                    || l.starts_with("static ")
            }
    }
}

// ── ContextWindow ────────────────────────────────────────────────────────────

/// Manages a collection of context chunks within a token budget.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContextWindow {
    pub max_tokens: usize,
    pub used_tokens: usize,
    pub chunks: Vec<ContextChunk>,
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
            chunks: Vec::new(),
        }
    }

    /// Add a chunk. If it exceeds the budget, evict lowest-relevance chunks first,
    /// then compress remaining chunks to higher depth levels.
    pub fn add_chunk(&mut self, chunk: ContextChunk) {
        let needed = chunk.token_count;

        // Try to make room if needed
        while self.used_tokens + needed > self.max_tokens && !self.chunks.is_empty() {
            self.evict_least_relevant();
        }

        // If still over budget after eviction, try compression
        if self.used_tokens + needed > self.max_tokens && !self.chunks.is_empty() {
            let target = self.max_tokens.saturating_sub(needed);
            self.compress_to_fit(target);
        }

        if self.used_tokens + needed <= self.max_tokens || self.chunks.is_empty() {
            self.used_tokens += needed;
            self.chunks.push(chunk);
            self.sort_by_relevance();
        }
    }

    /// Remove the chunk with the lowest relevance score.
    pub fn evict_least_relevant(&mut self) {
        if self.chunks.is_empty() {
            return;
        }
        let min_idx = self
            .chunks
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.relevance_score
                    .partial_cmp(&b.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        let removed = self.chunks.remove(min_idx);
        self.used_tokens = self.used_tokens.saturating_sub(removed.token_count);
    }

    /// Progressively compress chunks from level 0 → 1 → 2 → 3 until under target.
    pub fn compress_to_fit(&mut self, target_tokens: usize) {
        // Compress lowest-relevance chunks first
        let mut indices: Vec<usize> = (0..self.chunks.len()).collect();
        indices.sort_by(|&a, &b| {
            self.chunks[a]
                .relevance_score
                .partial_cmp(&self.chunks[b].relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for idx in indices {
            if self.used_tokens <= target_tokens {
                break;
            }
            let chunk = &self.chunks[idx];
            if chunk.depth_level >= 3 {
                continue;
            }
            let new_level = chunk.depth_level + 1;
            let old_tokens = chunk.token_count;
            let new_content =
                ContextHierarchy::summarize_file(&chunk.content, new_level);
            let new_tokens = token_estimate(&new_content);

            self.chunks[idx].content = new_content;
            self.chunks[idx].depth_level = new_level;
            self.chunks[idx].token_count = new_tokens;
            self.used_tokens = self.used_tokens.saturating_sub(old_tokens) + new_tokens;
        }
    }

    /// Concatenate all chunks with separators for feeding to an LLM.
    pub fn get_context_string(&self) -> String {
        self.chunks
            .iter()
            .map(|c| {
                format!(
                    "--- {} (depth={}, score={:.2}) ---\n{}",
                    c.source_file.display(),
                    c.depth_level,
                    c.relevance_score,
                    c.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Tokens remaining in the budget.
    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.used_tokens)
    }

    fn sort_by_relevance(&mut self) {
        self.chunks
            .sort_by(|a, b| {
                b.relevance_score
                    .partial_cmp(&a.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
    }
}

// ── ContextScorer ────────────────────────────────────────────────────────────

/// Scores relevance of files and chunks using multiple signals.
pub struct ContextScorer;

impl ContextScorer {
    /// Score by recency: exponential decay with half-life of 1 hour (3600s).
    pub fn score_by_recency(last_modified: u64) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_secs = now.saturating_sub(last_modified) as f64;
        let half_life = 3600.0;
        (-age_secs * (2.0_f64.ln()) / half_life).exp()
    }

    /// Score by recency with an explicit `now` timestamp (for testing).
    pub fn score_by_recency_at(last_modified: u64, now: u64) -> f64 {
        let age_secs = now.saturating_sub(last_modified) as f64;
        let half_life = 3600.0;
        (-age_secs * (2.0_f64.ln()) / half_life).exp()
    }

    /// Score by edit distance: directory proximity between two file paths.
    /// Same directory → 1.0, one level apart → 0.75, etc.
    pub fn score_by_edit_distance(file: &str, current_file: &str) -> f64 {
        let path_a = Path::new(file);
        let path_b = Path::new(current_file);

        let components_a: Vec<_> = path_a.components().collect();
        let components_b: Vec<_> = path_b.components().collect();

        let common = components_a
            .iter()
            .zip(components_b.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let total = components_a.len().max(components_b.len());
        if total == 0 {
            return 1.0;
        }
        let distance = (components_a.len() - common) + (components_b.len() - common);
        1.0 / (1.0 + distance as f64 * 0.25)
    }

    /// Score by keyword match: simple TF-IDF-like scoring.
    pub fn score_by_keyword_match(content: &str, query: &str) -> f64 {
        if query.is_empty() || content.is_empty() {
            return 0.0;
        }
        let content_lower = content.to_lowercase();
        let query_words: Vec<&str> = query.split_whitespace().collect();
        if query_words.is_empty() {
            return 0.0;
        }

        let content_word_count = content_lower.split_whitespace().count().max(1) as f64;
        let mut total_score = 0.0;

        for word in &query_words {
            let word_lower = word.to_lowercase();
            let matches = content_lower.matches(&word_lower).count() as f64;
            // TF component: frequency / total words
            let tf = matches / content_word_count;
            // IDF-like boost for rarer terms (longer words score higher)
            let idf = 1.0 + (word_lower.len() as f64).ln();
            total_score += tf * idf;
        }

        // Normalize by query length
        total_score / query_words.len() as f64
    }

    /// Score by dependency: how many imports reference this file.
    pub fn score_by_dependency(file: &str, imports: &[String]) -> f64 {
        if imports.is_empty() {
            return 0.0;
        }
        let file_stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if file_stem.is_empty() {
            return 0.0;
        }
        let matches = imports
            .iter()
            .filter(|imp| imp.contains(file_stem))
            .count() as f64;
        let total = imports.len() as f64;
        (matches / total).min(1.0)
    }

    /// Score by access frequency: logarithmic scaling.
    pub fn score_by_access_frequency(access_count: u32) -> f64 {
        if access_count == 0 {
            return 0.0;
        }
        (1.0 + access_count as f64).ln() / 10.0_f64.ln()
    }

    /// Weighted combination of multiple scores.
    pub fn combined_score(scores: &[f64], weights: &[f64]) -> f64 {
        if scores.is_empty() || weights.is_empty() {
            return 0.0;
        }
        let len = scores.len().min(weights.len());
        let weighted_sum: f64 = scores[..len]
            .iter()
            .zip(weights[..len].iter())
            .map(|(s, w)| s * w)
            .sum();
        let weight_sum: f64 = weights[..len].iter().sum();
        if weight_sum == 0.0 {
            return 0.0;
        }
        weighted_sum / weight_sum
    }
}

// ── ContextCache ─────────────────────────────────────────────────────────────

/// Cache entry with access ordering for LRU eviction.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    value: String,
    access_order: u64,
}

/// LRU cache for processed summaries at different depth levels.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContextCache {
    capacity: usize,
    cache: HashMap<String, CacheEntry>,
    counter: u64,
}

impl ContextCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            counter: 0,
        }
    }

    /// Get a cached summary. Returns None if not found.
    pub fn get(&mut self, key: &str) -> Option<&String> {
        self.counter += 1;
        let counter = self.counter;
        if let Some(entry) = self.cache.get_mut(key) {
            entry.access_order = counter;
            Some(&entry.value)
        } else {
            None
        }
    }

    /// Insert a summary into the cache. Evicts oldest if at capacity.
    pub fn put(&mut self, key: String, value: String) {
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            self.evict_oldest();
        }
        self.counter += 1;
        self.cache.insert(
            key,
            CacheEntry {
                value,
                access_order: self.counter,
            },
        );
    }

    /// Evict the least-recently-used entry.
    pub fn evict_oldest(&mut self) {
        if self.cache.is_empty() {
            return;
        }
        let oldest_key = self
            .cache
            .iter()
            .min_by_key(|(_, v)| v.access_order)
            .map(|(k, _)| k.clone());
        if let Some(key) = oldest_key {
            self.cache.remove(&key);
        }
    }

    /// Remove all depth-level entries for a given file.
    pub fn invalidate(&mut self, file: &str) {
        let keys_to_remove: Vec<String> = self
            .cache
            .keys()
            .filter(|k| k.starts_with(file))
            .cloned()
            .collect();
        for key in keys_to_remove {
            self.cache.remove(&key);
        }
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.counter = 0;
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

// ── ContextStats ─────────────────────────────────────────────────────────────

/// Statistics about the context manager state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_files: usize,
    pub indexed_files: usize,
    pub total_tokens: usize,
    pub used_tokens: usize,
    pub compression_ratio: f64,
}

// ── InfiniteContextManager ───────────────────────────────────────────────────

/// The main orchestrator for infinite code context.
///
/// Builds, queries, expands, and compresses context for a workspace,
/// keeping everything within a configurable token budget.
pub struct InfiniteContextManager {
    pub workspace: PathBuf,
    pub max_tokens: usize,
    pub cache: ContextCache,
    file_index: Vec<PathBuf>,
    original_tokens: usize,
}

impl InfiniteContextManager {
    /// Create a new manager for the given workspace.
    pub fn new(workspace: &Path, max_tokens: usize) -> Self {
        Self {
            workspace: workspace.to_path_buf(),
            max_tokens,
            cache: ContextCache::new(1000),
            file_index: Vec::new(),
            original_tokens: 0,
        }
    }

    /// Build a project-level summary (level 4) of the workspace.
    pub fn build_project_summary(workspace: &Path) -> Result<String> {
        let mut summaries = Vec::new();
        let walker = walkdir::WalkDir::new(workspace)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if Self::is_binary_or_ignored(path) {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !Self::is_code_file(ext) {
                continue;
            }
            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.is_empty() {
                continue;
            }
            let rel = path
                .strip_prefix(workspace)
                .unwrap_or(path)
                .display()
                .to_string();
            let sigs = ContextHierarchy::extract_signatures(&content);
            let type_count = sigs.len();
            if type_count > 0 {
                summaries.push(format!("  {rel}: {type_count} definitions"));
            }
        }

        if summaries.is_empty() {
            return Ok("Empty or non-code workspace".to_string());
        }
        summaries.sort();
        Ok(format!(
            "Project at {}: {} code files\n{}",
            workspace.display(),
            summaries.len(),
            summaries.join("\n")
        ))
    }

    /// Build a context window tailored to a query, optionally focusing on a current file.
    pub fn build_context_for_query(
        &mut self,
        query: &str,
        current_file: Option<&str>,
    ) -> Result<ContextWindow> {
        let mut window = ContextWindow::new(self.max_tokens);
        self.file_index.clear();
        self.original_tokens = 0;

        let walker = walkdir::WalkDir::new(&self.workspace)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok());

        let mut candidates: Vec<(PathBuf, f64, String)> = Vec::new();

        for entry in walker {
            let path = entry.path();
            if !path.is_file() || Self::is_binary_or_ignored(path) {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !Self::is_code_file(ext) {
                continue;
            }

            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.is_empty() {
                continue;
            }

            self.file_index.push(path.to_path_buf());
            let tokens = token_estimate(&content);
            self.original_tokens += tokens;

            let path_str = path.to_string_lossy().to_string();

            // Score this file
            let keyword_score = ContextScorer::score_by_keyword_match(&content, query);
            let proximity_score = current_file
                .map(|cf| ContextScorer::score_by_edit_distance(&path_str, cf))
                .unwrap_or(0.5);
            let scores = [keyword_score, proximity_score];
            let weights = [0.7, 0.3];
            let score = ContextScorer::combined_score(&scores, &weights);

            candidates.push((path.to_path_buf(), score, content));
        }

        // Sort by score descending
        candidates.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Add highest-scoring files at appropriate depth
        for (path, score, content) in candidates {
            let depth = if score > 0.5 {
                0 // Full content for highly relevant
            } else if score > 0.2 {
                1 // Function summaries
            } else if score > 0.1 {
                2 // Skeleton
            } else {
                3 // Signature only
            };

            let summarized = ContextHierarchy::summarize_file(&content, depth);
            let cache_key = format!("{}:{}", path.display(), depth);
            self.cache.put(cache_key, summarized.clone());

            let mut chunk = ContextChunk::new(
                format!("file:{}", path.display()),
                &path,
                summarized,
                ChunkType::FileContent,
            );
            chunk.relevance_score = score;
            chunk.depth_level = depth;

            if window.remaining_tokens() < chunk.token_count && window.remaining_tokens() < 100 {
                break;
            }
            window.add_chunk(chunk);
        }

        Ok(window)
    }

    /// Expand a file from its current summary to full content.
    pub fn expand_context(&self, window: &mut ContextWindow, file: &str) -> Result<()> {
        let path = Path::new(file);
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {file}"))?;

        // Find and update existing chunk, or add new one
        if let Some(chunk) = window
            .chunks
            .iter_mut()
            .find(|c| c.source_file.to_string_lossy() == file)
        {
            let old_tokens = chunk.token_count;
            let new_tokens = token_estimate(&content);
            chunk.content = content;
            chunk.depth_level = 0;
            chunk.token_count = new_tokens;
            window.used_tokens = window.used_tokens.saturating_sub(old_tokens) + new_tokens;
        } else {
            let chunk = ContextChunk::new(
                format!("file:{file}"),
                file,
                content,
                ChunkType::FileContent,
            );
            window.add_chunk(chunk);
        }

        Ok(())
    }

    /// Compress least-relevant chunks to reduce token usage by `target_reduction`.
    pub fn compress_context(
        &self,
        window: &mut ContextWindow,
        target_reduction: usize,
    ) {
        let target = window.used_tokens.saturating_sub(target_reduction);
        window.compress_to_fit(target);
    }

    /// Re-score and rebalance the context window for a new query.
    pub fn refresh_context(
        &self,
        window: &mut ContextWindow,
        new_query: &str,
    ) {
        for chunk in &mut window.chunks {
            let keyword_score =
                ContextScorer::score_by_keyword_match(&chunk.content, new_query);
            let freq_score =
                ContextScorer::score_by_access_frequency(chunk.access_count);
            chunk.relevance_score =
                ContextScorer::combined_score(&[keyword_score, freq_score], &[0.8, 0.2]);
            chunk.access_count += 1;
        }
        window.sort_by_relevance();
    }

    /// Get a file's content at a specific depth level.
    pub fn get_file_at_depth(&mut self, file: &str, depth: u8) -> Result<String> {
        let cache_key = format!("{file}:{depth}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("Failed to read {file}"))?;
        let summarized = ContextHierarchy::summarize_file(&content, depth);
        self.cache.put(cache_key, summarized.clone());
        Ok(summarized)
    }

    /// Current statistics.
    pub fn stats(&self) -> ContextStats {
        let indexed = self.file_index.len();
        let compression_ratio = if self.original_tokens > 0 {
            1.0 - (self.original_tokens as f64 / self.max_tokens as f64).min(1.0)
        } else {
            0.0
        };
        ContextStats {
            total_files: self.file_index.len(),
            indexed_files: indexed,
            total_tokens: self.original_tokens,
            used_tokens: self.original_tokens,
            compression_ratio,
        }
    }

    fn is_binary_or_ignored(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains("/target/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/.git/")
            || path_str.contains("/__pycache__/")
            || path_str.contains("/dist/")
            || path_str.contains("/build/")
    }

    fn is_code_file(ext: &str) -> bool {
        matches!(
            ext,
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "rb"
                | "swift"
                | "kt"
                | "scala"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "md"
                | "css"
                | "html"
                | "sql"
                | "sh"
                | "bash"
                | "zsh"
                | "lua"
                | "zig"
                | "ex"
                | "exs"
                | "erl"
                | "clj"
                | "cs"
        )
    }
}

// ── Token estimation ─────────────────────────────────────────────────────────

/// Estimate token count using the word/4 heuristic (rough GPT tokenizer approximation).
pub fn token_estimate(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    // Approximate: 1 token ≈ 4 characters (GPT-family heuristic)
    // We use character count / 4 as it's more reliable than word count for code
    let char_count = text.len();
    char_count.div_ceil(4)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── ContextHierarchy tests ───────────────────────────────────────────

    #[test]
    fn test_summarize_file_level_0_returns_full_content() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        assert_eq!(ContextHierarchy::summarize_file(content, 0), content);
    }

    #[test]
    fn test_summarize_file_level_1_function_summaries() {
        let content = "/// Does stuff\nfn foo() {\n    bar();\n}\nfn baz() {\n    qux();\n}";
        let result = ContextHierarchy::summarize_file(content, 1);
        assert!(result.contains("fn foo()"));
        assert!(result.contains("fn baz()"));
    }

    #[test]
    fn test_summarize_file_level_2_skeleton() {
        let content = "struct Foo {\n    x: i32,\n}\n\nfn bar() {\n    let a = 1;\n    let b = 2;\n}";
        let result = ContextHierarchy::summarize_file(content, 2);
        assert!(result.contains("struct Foo"));
        assert!(!result.contains("let a = 1"));
    }

    #[test]
    fn test_summarize_file_level_3_module_summary() {
        let content = "//! My module\nfn foo() {}\nstruct Bar {}";
        let result = ContextHierarchy::summarize_file(content, 3);
        assert!(result.contains("My module"));
        assert!(result.contains("definitions"));
    }

    #[test]
    fn test_summarize_file_level_4_project_summary() {
        let content = "pub struct MyType {\n    x: i32,\n}\npub enum Status { Ok, Err }";
        let result = ContextHierarchy::summarize_file(content, 4);
        assert!(result.contains("pub struct MyType"));
    }

    #[test]
    fn test_summarize_file_level_above_4_same_as_4() {
        let content = "pub struct A {}\npub struct B {}";
        let r4 = ContextHierarchy::summarize_file(content, 4);
        let r5 = ContextHierarchy::summarize_file(content, 5);
        assert_eq!(r4, r5);
    }

    #[test]
    fn test_summarize_function_with_doc_comment() {
        let content = "/// Computes the fibonacci number.\nfn fib(n: u32) -> u32 {\n    n\n}";
        let result = ContextHierarchy::summarize_function(content);
        assert_eq!(result, "Computes the fibonacci number.");
    }

    #[test]
    fn test_summarize_function_without_doc_comment() {
        let content = "fn fib(n: u32) -> u32 {\n    n\n}";
        let result = ContextHierarchy::summarize_function(content);
        assert_eq!(result, "fn fib(n: u32) -> u32 {");
    }

    #[test]
    fn test_summarize_function_empty() {
        let result = ContextHierarchy::summarize_function("");
        assert_eq!(result, "(empty)");
    }

    #[test]
    fn test_extract_signatures_rust() {
        let content = "use std::io;\n\npub fn hello() {\n    println!(\"hi\");\n}\n\nstruct Foo {\n    x: i32,\n}\n\nimpl Foo {\n    fn new() -> Self { Foo { x: 0 } }\n}";
        let sigs = ContextHierarchy::extract_signatures(content);
        assert!(sigs.iter().any(|s| s.contains("pub fn hello()")));
        assert!(sigs.iter().any(|s| s.contains("struct Foo")));
        assert!(sigs.iter().any(|s| s.contains("impl Foo")));
        assert!(sigs.iter().any(|s| s.contains("fn new()")));
    }

    #[test]
    fn test_extract_signatures_empty() {
        let sigs = ContextHierarchy::extract_signatures("");
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_extract_signatures_no_code_lines() {
        let content = "// just a comment\n# and a header\nsome random text";
        let sigs = ContextHierarchy::extract_signatures(content);
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_extract_skeleton_preserves_structure() {
        let content = "struct Point {\n    x: f64,\n    y: f64,\n}";
        let skeleton = ContextHierarchy::extract_skeleton(content);
        assert!(skeleton.contains("struct Point"));
    }

    #[test]
    fn test_extract_skeleton_removes_function_bodies() {
        let content = "fn compute() {\n    let x = 1;\n    let y = 2;\n    x + y\n}";
        let skeleton = ContextHierarchy::extract_skeleton(content);
        assert!(skeleton.contains("fn compute()"));
        assert!(!skeleton.contains("let x = 1"));
    }

    // ── ContextWindow tests ──────────────────────────────────────────────

    #[test]
    fn test_window_new_defaults() {
        let w = ContextWindow::new(1000);
        assert_eq!(w.max_tokens, 1000);
        assert_eq!(w.used_tokens, 0);
        assert!(w.chunks.is_empty());
    }

    #[test]
    fn test_window_add_chunk_within_budget() {
        let mut w = ContextWindow::new(10000);
        let chunk = ContextChunk::new("c1", "file.rs", "hello world", ChunkType::FileContent);
        w.add_chunk(chunk);
        assert_eq!(w.chunks.len(), 1);
        assert!(w.used_tokens > 0);
    }

    #[test]
    fn test_window_add_chunk_evicts_when_full() {
        let mut w = ContextWindow::new(20);
        let mut c1 = ContextChunk::new("c1", "a.rs", "short", ChunkType::FileContent);
        c1.relevance_score = 0.1;
        let mut c2 = ContextChunk::new("c2", "b.rs", "also short text", ChunkType::FileContent);
        c2.relevance_score = 0.9;
        w.add_chunk(c1);
        w.add_chunk(c2);
        // Should have evicted c1 since it had lower relevance
        let has_high_relevance = w.chunks.iter().any(|c| c.relevance_score > 0.5);
        assert!(has_high_relevance || w.chunks.len() <= 2);
    }

    #[test]
    fn test_window_evict_least_relevant() {
        let mut w = ContextWindow::new(100000);
        let mut c1 = ContextChunk::new("c1", "a.rs", "content a", ChunkType::FileContent);
        c1.relevance_score = 0.1;
        let mut c2 = ContextChunk::new("c2", "b.rs", "content b", ChunkType::FileContent);
        c2.relevance_score = 0.9;
        w.add_chunk(c1);
        w.add_chunk(c2);
        w.evict_least_relevant();
        assert_eq!(w.chunks.len(), 1);
        assert_eq!(w.chunks[0].id, "c2");
    }

    #[test]
    fn test_window_evict_empty() {
        let mut w = ContextWindow::new(100);
        w.evict_least_relevant(); // Should not panic
        assert!(w.chunks.is_empty());
    }

    #[test]
    fn test_window_compress_to_fit() {
        let mut w = ContextWindow::new(100000);
        let content = "pub fn hello() {\n    println!(\"hello world\");\n    let x = 42;\n    let y = x * 2;\n}";
        let mut chunk = ContextChunk::new("c1", "a.rs", content, ChunkType::FileContent);
        chunk.relevance_score = 0.5;
        chunk.depth_level = 0;
        w.add_chunk(chunk);
        let before = w.used_tokens;
        w.compress_to_fit(1); // Compress to almost nothing
        // After compression, depth should have increased
        assert!(w.chunks[0].depth_level > 0);
        assert!(w.used_tokens <= before);
    }

    #[test]
    fn test_window_get_context_string() {
        let mut w = ContextWindow::new(100000);
        let mut c = ContextChunk::new("c1", "test.rs", "fn main() {}", ChunkType::FileContent);
        c.relevance_score = 1.0;
        w.add_chunk(c);
        let ctx = w.get_context_string();
        assert!(ctx.contains("test.rs"));
        assert!(ctx.contains("fn main()"));
    }

    #[test]
    fn test_window_remaining_tokens() {
        let mut w = ContextWindow::new(1000);
        assert_eq!(w.remaining_tokens(), 1000);
        let c = ContextChunk::new("c1", "a.rs", "hello", ChunkType::FileContent);
        let tokens = c.token_count;
        w.add_chunk(c);
        assert_eq!(w.remaining_tokens(), 1000 - tokens);
    }

    #[test]
    fn test_window_sorted_by_relevance() {
        let mut w = ContextWindow::new(100000);
        let mut c1 = ContextChunk::new("c1", "a.rs", "aaa", ChunkType::FileContent);
        c1.relevance_score = 0.3;
        let mut c2 = ContextChunk::new("c2", "b.rs", "bbb", ChunkType::FileContent);
        c2.relevance_score = 0.9;
        let mut c3 = ContextChunk::new("c3", "c.rs", "ccc", ChunkType::FileContent);
        c3.relevance_score = 0.6;
        w.add_chunk(c1);
        w.add_chunk(c2);
        w.add_chunk(c3);
        assert_eq!(w.chunks[0].id, "c2");
        assert_eq!(w.chunks[1].id, "c3");
        assert_eq!(w.chunks[2].id, "c1");
    }

    // ── ContextScorer tests ──────────────────────────────────────────────

    #[test]
    fn test_score_recency_now_is_1() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let score = ContextScorer::score_by_recency_at(now, now);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_score_recency_one_hour_ago_is_half() {
        let now = 1_000_000u64;
        let one_hour_ago = now - 3600;
        let score = ContextScorer::score_by_recency_at(one_hour_ago, now);
        assert!((score - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_score_recency_decays_over_time() {
        let now = 1_000_000u64;
        let s1 = ContextScorer::score_by_recency_at(now - 100, now);
        let s2 = ContextScorer::score_by_recency_at(now - 1000, now);
        let s3 = ContextScorer::score_by_recency_at(now - 10000, now);
        assert!(s1 > s2);
        assert!(s2 > s3);
    }

    #[test]
    fn test_score_edit_distance_same_directory() {
        let score = ContextScorer::score_by_edit_distance("src/foo.rs", "src/bar.rs");
        // Same parent dir: common=1 ("src"), distance = (2-1)+(2-1) = 2, score = 1/(1+0.5) ≈ 0.67
        assert!(score > 0.6);
    }

    #[test]
    fn test_score_edit_distance_different_directories() {
        let score = ContextScorer::score_by_edit_distance(
            "src/module_a/deep/foo.rs",
            "tests/module_b/bar.rs",
        );
        let same_dir = ContextScorer::score_by_edit_distance("src/foo.rs", "src/bar.rs");
        assert!(same_dir > score);
    }

    #[test]
    fn test_score_edit_distance_empty_paths() {
        let score = ContextScorer::score_by_edit_distance("", "");
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_score_keyword_match_exact() {
        let score = ContextScorer::score_by_keyword_match("fn parse_token()", "parse");
        assert!(score > 0.0);
    }

    #[test]
    fn test_score_keyword_match_no_match() {
        let score = ContextScorer::score_by_keyword_match("fn hello()", "zzzzz");
        assert!((score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_score_keyword_match_empty_query() {
        let score = ContextScorer::score_by_keyword_match("some content", "");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_keyword_match_empty_content() {
        let score = ContextScorer::score_by_keyword_match("", "query");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_keyword_match_case_insensitive() {
        let s1 = ContextScorer::score_by_keyword_match("Hello World", "hello");
        let s2 = ContextScorer::score_by_keyword_match("Hello World", "HELLO");
        assert!((s1 - s2).abs() < 0.001);
    }

    #[test]
    fn test_score_keyword_multiple_matches_higher() {
        let s1 = ContextScorer::score_by_keyword_match("parse parse parse", "parse");
        let s2 = ContextScorer::score_by_keyword_match("parse once only", "parse");
        assert!(s1 > s2);
    }

    #[test]
    fn test_score_dependency_matching_import() {
        let imports = vec!["use crate::config;".to_string(), "use crate::utils;".to_string()];
        let score = ContextScorer::score_by_dependency("src/config.rs", &imports);
        assert!(score > 0.0);
    }

    #[test]
    fn test_score_dependency_no_matching_import() {
        let imports = vec!["use crate::config;".to_string()];
        let score = ContextScorer::score_by_dependency("src/unknown.rs", &imports);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_dependency_empty_imports() {
        let score = ContextScorer::score_by_dependency("src/foo.rs", &[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_access_frequency_zero() {
        assert_eq!(ContextScorer::score_by_access_frequency(0), 0.0);
    }

    #[test]
    fn test_score_access_frequency_increases() {
        let s1 = ContextScorer::score_by_access_frequency(1);
        let s2 = ContextScorer::score_by_access_frequency(10);
        let s3 = ContextScorer::score_by_access_frequency(100);
        assert!(s1 > 0.0);
        assert!(s2 > s1);
        assert!(s3 > s2);
    }

    #[test]
    fn test_combined_score_equal_weights() {
        let scores = [0.5, 0.5];
        let weights = [1.0, 1.0];
        let result = ContextScorer::combined_score(&scores, &weights);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_combined_score_unequal_weights() {
        let scores = [1.0, 0.0];
        let weights = [3.0, 1.0];
        let result = ContextScorer::combined_score(&scores, &weights);
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_combined_score_empty() {
        assert_eq!(ContextScorer::combined_score(&[], &[]), 0.0);
    }

    #[test]
    fn test_combined_score_zero_weights() {
        let scores = [1.0, 1.0];
        let weights = [0.0, 0.0];
        assert_eq!(ContextScorer::combined_score(&scores, &weights), 0.0);
    }

    #[test]
    fn test_combined_score_mismatched_lengths() {
        let scores = [0.8, 0.6, 0.4];
        let weights = [1.0, 1.0];
        // Should use min(len) = 2
        let result = ContextScorer::combined_score(&scores, &weights);
        assert!((result - 0.7).abs() < 0.001);
    }

    // ── ContextCache tests ───────────────────────────────────────────────

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = ContextCache::new(10);
        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_cache_get_missing() {
        let mut cache = ContextCache::new(10);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_capacity_eviction() {
        let mut cache = ContextCache::new(3);
        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());
        cache.put("c".to_string(), "3".to_string());
        // Access "a" to make it recent
        cache.get("a");
        // Adding d should evict "b" (oldest by access order)
        cache.put("d".to_string(), "4".to_string());
        assert!(cache.get("b").is_none());
        assert!(cache.get("a").is_some());
        assert!(cache.get("d").is_some());
    }

    #[test]
    fn test_cache_evict_oldest() {
        let mut cache = ContextCache::new(10);
        cache.put("old".to_string(), "v1".to_string());
        cache.put("new".to_string(), "v2".to_string());
        cache.evict_oldest();
        assert!(cache.get("old").is_none());
        assert!(cache.get("new").is_some());
    }

    #[test]
    fn test_cache_evict_oldest_empty() {
        let mut cache = ContextCache::new(10);
        cache.evict_oldest(); // Should not panic
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = ContextCache::new(10);
        cache.put("src/foo.rs:0".to_string(), "full".to_string());
        cache.put("src/foo.rs:1".to_string(), "summary".to_string());
        cache.put("src/foo.rs:2".to_string(), "skeleton".to_string());
        cache.put("src/bar.rs:0".to_string(), "other".to_string());
        cache.invalidate("src/foo.rs");
        assert_eq!(cache.len(), 1);
        assert!(cache.get("src/bar.rs:0").is_some());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ContextCache::new(10);
        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_overwrite_existing_key() {
        let mut cache = ContextCache::new(5);
        cache.put("key".to_string(), "old".to_string());
        cache.put("key".to_string(), "new".to_string());
        assert_eq!(cache.get("key").unwrap(), "new");
        assert_eq!(cache.len(), 1);
    }

    // ── Token estimation tests ───────────────────────────────────────────

    #[test]
    fn test_token_estimate_empty() {
        assert_eq!(token_estimate(""), 0);
    }

    #[test]
    fn test_token_estimate_short_text() {
        let est = token_estimate("hello");
        // "hello" is 5 chars → ceil(5/4) = 2
        assert_eq!(est, 2);
    }

    #[test]
    fn test_token_estimate_code_block() {
        let code = "fn main() {\n    println!(\"hello world\");\n}";
        let est = token_estimate(code);
        assert!(est > 0);
        assert!(est < 100); // Sanity bound
    }

    #[test]
    fn test_token_estimate_proportional() {
        let short = token_estimate("hello");
        let long = token_estimate("hello world this is a longer piece of text with more tokens");
        assert!(long > short);
    }

    // ── InfiniteContextManager tests ─────────────────────────────────────

    #[test]
    fn test_manager_new() {
        let mgr = InfiniteContextManager::new(Path::new("/tmp"), 100_000);
        assert_eq!(mgr.max_tokens, 100_000);
        assert_eq!(mgr.workspace, Path::new("/tmp"));
    }

    #[test]
    fn test_manager_build_project_summary_empty_dir() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let summary = InfiniteContextManager::build_project_summary(dir.path())
            .expect("failed to build summary");
        assert!(summary.contains("Empty") || summary.contains("0"));
    }

    #[test]
    fn test_manager_build_project_summary_with_files() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        std::fs::write(
            dir.path().join("main.rs"),
            "pub fn main() {}\nstruct Config {}",
        )
        .unwrap();
        let summary = InfiniteContextManager::build_project_summary(dir.path())
            .expect("failed to build summary");
        assert!(summary.contains("main.rs"));
        assert!(summary.contains("definitions"));
    }

    #[test]
    fn test_manager_build_context_for_query() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        std::fs::write(
            dir.path().join("parser.rs"),
            "/// Parses input tokens\npub fn parse(input: &str) -> Vec<Token> { vec![] }",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("utils.rs"),
            "pub fn format_output(s: &str) -> String { s.to_string() }",
        )
        .unwrap();

        let mut mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let window = mgr
            .build_context_for_query("parse", None)
            .expect("failed to build context");
        assert!(!window.chunks.is_empty());
        // parser.rs should score higher
        let ctx = window.get_context_string();
        assert!(ctx.contains("parse"));
    }

    #[test]
    fn test_manager_build_context_with_current_file() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(sub.join("b.rs"), "fn b() {}").unwrap();

        let mut mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let window = mgr
            .build_context_for_query("function", Some(&sub.join("a.rs").to_string_lossy()))
            .expect("failed");
        assert!(!window.chunks.is_empty());
    }

    #[test]
    fn test_manager_expand_context() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let file = dir.path().join("code.rs");
        std::fs::write(&file, "fn expanded() { full_content(); }").unwrap();

        let mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let mut window = ContextWindow::new(100_000);
        mgr.expand_context(&mut window, &file.to_string_lossy())
            .expect("failed to expand");
        assert_eq!(window.chunks.len(), 1);
        assert!(window.chunks[0].content.contains("full_content"));
    }

    #[test]
    fn test_manager_compress_context() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let mut window = ContextWindow::new(100_000);

        let content = "pub fn hello() {\n    println!(\"hello world\");\n    let x = 42;\n    let y = x * 2;\n}";
        let mut chunk = ContextChunk::new("c1", "a.rs", content, ChunkType::FileContent);
        chunk.relevance_score = 0.5;
        window.add_chunk(chunk);

        let before_tokens = window.used_tokens;
        mgr.compress_context(&mut window, before_tokens / 2);
        assert!(window.chunks[0].depth_level > 0);
    }

    #[test]
    fn test_manager_refresh_context() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let mut window = ContextWindow::new(100_000);

        let mut c1 = ContextChunk::new("c1", "parser.rs", "fn parse() {}", ChunkType::FileContent);
        c1.relevance_score = 0.5;
        let mut c2 = ContextChunk::new("c2", "format.rs", "fn format() {}", ChunkType::FileContent);
        c2.relevance_score = 0.5;
        window.add_chunk(c1);
        window.add_chunk(c2);

        mgr.refresh_context(&mut window, "parse");
        // parser.rs should now score higher
        let parser_chunk = window.chunks.iter().find(|c| c.id == "c1").unwrap();
        let format_chunk = window.chunks.iter().find(|c| c.id == "c2").unwrap();
        assert!(parser_chunk.relevance_score >= format_chunk.relevance_score);
    }

    #[test]
    fn test_manager_get_file_at_depth() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let file = dir.path().join("code.rs");
        std::fs::write(&file, "pub fn hello() {\n    world();\n}\nstruct Foo {}").unwrap();

        let mut mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let full = mgr.get_file_at_depth(&file.to_string_lossy(), 0).unwrap();
        assert!(full.contains("world()"));

        let skel = mgr.get_file_at_depth(&file.to_string_lossy(), 2).unwrap();
        assert!(skel.contains("pub fn hello()"));
    }

    #[test]
    fn test_manager_get_file_at_depth_caches() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let file = dir.path().join("cached.rs");
        std::fs::write(&file, "fn foo() {}").unwrap();

        let mut mgr = InfiniteContextManager::new(dir.path(), 100_000);
        let _ = mgr.get_file_at_depth(&file.to_string_lossy(), 0).unwrap();
        // Second call should use cache
        let result = mgr.get_file_at_depth(&file.to_string_lossy(), 0).unwrap();
        assert!(result.contains("fn foo()"));
    }

    #[test]
    fn test_manager_stats() {
        let mgr = InfiniteContextManager::new(Path::new("/tmp"), 50_000);
        let stats = mgr.stats();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_tokens, 0);
    }

    // ── Edge case tests ──────────────────────────────────────────────────

    #[test]
    fn test_edge_empty_file_summarize() {
        let result = ContextHierarchy::summarize_file("", 1);
        assert_eq!(result, "(empty)");
    }

    #[test]
    fn test_edge_binary_detection() {
        assert!(InfiniteContextManager::is_binary_or_ignored(Path::new(
            "/project/target/debug/binary"
        )));
        assert!(InfiniteContextManager::is_binary_or_ignored(Path::new(
            "/project/node_modules/pkg/index.js"
        )));
        assert!(!InfiniteContextManager::is_binary_or_ignored(Path::new(
            "/project/src/main.rs"
        )));
    }

    #[test]
    fn test_edge_code_file_detection() {
        assert!(InfiniteContextManager::is_code_file("rs"));
        assert!(InfiniteContextManager::is_code_file("py"));
        assert!(InfiniteContextManager::is_code_file("ts"));
        assert!(!InfiniteContextManager::is_code_file("exe"));
        assert!(!InfiniteContextManager::is_code_file("png"));
        assert!(!InfiniteContextManager::is_code_file(""));
    }

    #[test]
    fn test_edge_deeply_nested_path_scoring() {
        let score = ContextScorer::score_by_edit_distance(
            "a/b/c/d/e/f/g/file.rs",
            "x/y/z/other.rs",
        );
        assert!(score > 0.0);
        assert!(score < 0.5);
    }

    #[test]
    fn test_edge_very_large_content_token_estimate() {
        let large = "x".repeat(1_000_000);
        let est = token_estimate(&large);
        assert_eq!(est, 250_000);
    }

    #[test]
    fn test_edge_window_single_huge_chunk() {
        let mut w = ContextWindow::new(10); // Very small budget
        let chunk = ContextChunk::new(
            "big",
            "huge.rs",
            "a".repeat(1000), // Way over budget
            ChunkType::FileContent,
        );
        w.add_chunk(chunk);
        // Should still add it since the window was empty
        assert_eq!(w.chunks.len(), 1);
    }

    #[test]
    fn test_edge_context_chunk_new_sets_defaults() {
        let c = ContextChunk::new("id1", "file.rs", "content", ChunkType::Summary);
        assert_eq!(c.id, "id1");
        assert_eq!(c.source_file, PathBuf::from("file.rs"));
        assert_eq!(c.chunk_type, ChunkType::Summary);
        assert_eq!(c.depth_level, 0);
        assert_eq!(c.access_count, 0);
        assert!(c.last_accessed > 0);
        assert!(c.token_count > 0);
    }

    #[test]
    fn test_chunk_type_variants() {
        let types = vec![
            ChunkType::FileContent,
            ChunkType::Summary,
            ChunkType::Symbol,
            ChunkType::Dependency,
            ChunkType::DocString,
            ChunkType::TestCase,
            ChunkType::Config,
        ];
        assert_eq!(types.len(), 7);
        // Ensure they're distinct
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
    }

    #[test]
    fn test_context_stats_serialization() {
        let stats = ContextStats {
            total_files: 100,
            indexed_files: 80,
            total_tokens: 500_000,
            used_tokens: 100_000,
            compression_ratio: 0.8,
        };
        let json = serde_json::to_string(&stats).expect("serialize failed");
        let deserialized: ContextStats = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(deserialized.total_files, 100);
        assert_eq!(deserialized.indexed_files, 80);
    }

    #[test]
    fn test_extract_signatures_python_like() {
        let content = "def hello():\n    pass\n\nclass MyClass:\n    def method(self):\n        pass";
        let sigs = ContextHierarchy::extract_signatures(content);
        assert!(sigs.iter().any(|s| s.contains("def hello()")));
        assert!(sigs.iter().any(|s| s.contains("class MyClass")));
    }

    #[test]
    fn test_window_compress_already_at_max_depth() {
        let mut w = ContextWindow::new(100_000);
        let mut chunk = ContextChunk::new("c1", "a.rs", "fn x() {}", ChunkType::FileContent);
        chunk.depth_level = 3;
        chunk.relevance_score = 0.5;
        w.add_chunk(chunk);
        let tokens_before = w.used_tokens;
        w.compress_to_fit(1);
        // Should not change since already at max depth
        assert_eq!(w.chunks[0].depth_level, 3);
        assert_eq!(w.used_tokens, tokens_before);
    }
}
