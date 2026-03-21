//! Massive context streaming architecture for VibeCody.
//!
//! Handles 10M–100M token context windows with hierarchical summarization
//! and sliding window retrieval.  Extends the `infinite_context` module for
//! next-generation model context sizes (e.g. Magic.dev LTM-2-mini, 100M
//! tokens).
//!
//! # Architecture
//!
//! ```text
//! Raw Content ──→ ContextStreamingEngine
//!                       │
//!                       ├─ ContextSegment (full content + metadata)
//!                       │       │
//!                       │       ├─ Level 0  (full detail)
//!                       │       ├─ Level 1  (paragraph summaries)
//!                       │       ├─ Level 2  (section summaries)
//!                       │       ├─ Level 3  (file summaries)
//!                       │       └─ Level 4  (project summaries)
//!                       │
//!                       ├─ SlidingWindow (active working set)
//!                       │       ├─ overlap region
//!                       │       └─ eviction (LRU / relevance / hybrid)
//!                       │
//!                       └─ ContextCache (LRU segment cache)
//! ```

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn fnv1a_hash(data: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by the context streaming engine.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingError {
    SegmentNotFound,
    WindowFull,
    TokenLimitExceeded,
    CompressionFailed,
    InvalidLevel,
    CacheMiss,
    QueryError(String),
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SegmentNotFound => write!(f, "segment not found"),
            Self::WindowFull => write!(f, "sliding window is full"),
            Self::TokenLimitExceeded => write!(f, "token limit exceeded"),
            Self::CompressionFailed => write!(f, "compression failed"),
            Self::InvalidLevel => write!(f, "invalid hierarchy level"),
            Self::CacheMiss => write!(f, "cache miss"),
            Self::QueryError(msg) => write!(f, "query error: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Eviction strategy
// ---------------------------------------------------------------------------

/// Strategy used to decide which segments to evict from the sliding window.
#[derive(Debug, Clone, PartialEq)]
pub enum EvictionStrategy {
    /// Least-recently-used.
    LRU,
    /// Lowest relevance score first.
    RelevanceScore,
    /// Least frequently accessed.
    AccessFrequency,
    /// Oldest last-accessed timestamp first.
    TimeDecay,
    /// Weighted combination of all signals.
    Hybrid,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the [`ContextStreamingEngine`].
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum tokens the engine will manage (default 10M).
    pub max_tokens: usize,
    /// Sliding window size in tokens (default 1M).
    pub window_size: usize,
    /// Overlap between adjacent windows (default 50K).
    pub overlap_tokens: usize,
    /// Number of summary levels (default 5, levels 0..4).
    pub summary_levels: usize,
    /// How to pick eviction victims.
    pub eviction_strategy: EvictionStrategy,
    /// Target compression ratio for summaries (default 0.1 = 10%).
    pub compression_ratio: f32,
    /// Max segments held in the LRU cache (default 1000).
    pub cache_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10_000_000,
            window_size: 1_000_000,
            overlap_tokens: 50_000,
            summary_levels: 5,
            eviction_strategy: EvictionStrategy::Hybrid,
            compression_ratio: 0.1,
            cache_size: 1000,
        }
    }
}

// ---------------------------------------------------------------------------
// Context segment
// ---------------------------------------------------------------------------

/// A single segment of context (a file, block, or summary).
#[derive(Debug, Clone)]
pub struct ContextSegment {
    pub id: String,
    /// Source path or identifier.
    pub source: String,
    pub content_hash: u64,
    /// Summary text (always present once hierarchy is built).
    pub summary: String,
    /// Full content — `None` when evicted to save memory.
    pub full_content: Option<String>,
    pub tokens: usize,
    pub relevance_score: f32,
    pub access_count: u32,
    pub last_accessed: u64,
    /// Hierarchy level (0 = full detail, 4 = highest summary).
    pub level: u32,
    /// IDs of child segments (next level down).
    pub children_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Hierarchy level
// ---------------------------------------------------------------------------

/// One level of the hierarchical summary.
#[derive(Debug, Clone)]
pub struct ContextHierarchyLevel {
    pub level: u32,
    pub segments: Vec<ContextSegment>,
    pub total_tokens: usize,
}

impl ContextHierarchyLevel {
    pub fn new(level: u32) -> Self {
        Self {
            level,
            segments: Vec::new(),
            total_tokens: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Sliding window
// ---------------------------------------------------------------------------

/// The active working window of segment IDs.
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    /// Segment IDs currently inside the window.
    pub segments: Vec<String>,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub cursor_position: usize,
}

impl SlidingWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            segments: Vec::new(),
            total_tokens: 0,
            max_tokens,
            cursor_position: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Query types
// ---------------------------------------------------------------------------

/// A query against the context hierarchy.
#[derive(Debug, Clone)]
pub struct ContextQuery {
    pub query: String,
    pub max_results: usize,
    pub min_relevance: f32,
    pub include_summaries: bool,
    /// Which hierarchy levels to search (`None` = all).
    pub levels: Option<Vec<u32>>,
}

impl ContextQuery {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            max_results: 10,
            min_relevance: 0.3,
            include_summaries: true,
            levels: None,
        }
    }
}

/// A single match inside a [`ContextQueryResult`].
#[derive(Debug, Clone)]
pub struct ContextMatch {
    pub segment_id: String,
    pub source: String,
    pub content: String,
    pub relevance_score: f32,
    pub level: u32,
    pub tokens: usize,
}

/// Result of running a [`ContextQuery`].
#[derive(Debug, Clone)]
pub struct ContextQueryResult {
    pub query: String,
    pub matches: Vec<ContextMatch>,
    pub total_searched: usize,
    pub tokens_used: usize,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Per-level stats.
#[derive(Debug, Clone)]
pub struct LevelStats {
    pub level: u32,
    pub segment_count: usize,
    pub total_tokens: usize,
    pub avg_relevance: f32,
}

/// Overall engine statistics.
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub total_segments: usize,
    pub total_tokens: usize,
    pub tokens_in_window: usize,
    pub tokens_summarized: usize,
    pub cache_hit_rate: f32,
    pub eviction_count: usize,
    pub compression_savings_percent: f32,
    pub levels: Vec<LevelStats>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Main context streaming engine.
pub struct ContextStreamingEngine {
    config: StreamingConfig,
    /// All segments keyed by ID.
    segments: HashMap<String, ContextSegment>,
    /// Hierarchy levels (index = level number).
    hierarchy: Vec<ContextHierarchyLevel>,
    /// Sliding window over active segments.
    window: SlidingWindow,
    /// Simple LRU cache tracking (segment_id → insertion order).
    cache_order: Vec<String>,
    /// Running counter for generating unique IDs.
    next_id: u64,
    /// Total evictions performed.
    eviction_count: usize,
    /// Cache hit / miss counters.
    cache_hits: usize,
    cache_misses: usize,
}

impl ContextStreamingEngine {
    // -- Construction --------------------------------------------------------

    pub fn new(config: StreamingConfig) -> Self {
        let window_max = config.window_size;
        let levels = config.summary_levels;
        let mut hierarchy = Vec::with_capacity(levels);
        for i in 0..levels {
            hierarchy.push(ContextHierarchyLevel::new(i as u32));
        }
        Self {
            config,
            segments: HashMap::new(),
            hierarchy,
            window: SlidingWindow::new(window_max),
            cache_order: Vec::new(),
            next_id: 1,
            eviction_count: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    // -- Token estimation ----------------------------------------------------

    /// Estimate token count (~4 characters per token).
    pub fn estimate_tokens(text: &str) -> usize {
        text.len().div_ceil(4)
    }

    // -- Segment CRUD --------------------------------------------------------

    /// Add content as a new level-0 segment.  Returns the segment ID.
    pub fn add_segment(&mut self, source: &str, content: &str) -> Result<String, StreamingError> {
        let tokens = Self::estimate_tokens(content);
        let total_after = self.total_tokens() + tokens;
        if total_after > self.config.max_tokens {
            return Err(StreamingError::TokenLimitExceeded);
        }

        let id = format!("seg-{}", self.next_id);
        self.next_id += 1;

        let summary = self.summarize_content(
            content,
            (tokens as f32 * self.config.compression_ratio) as usize,
        );

        let segment = ContextSegment {
            id: id.clone(),
            source: source.to_string(),
            content_hash: fnv1a_hash(content),
            summary,
            full_content: Some(content.to_string()),
            tokens,
            relevance_score: 1.0,
            access_count: 0,
            last_accessed: now_secs(),
            level: 0,
            children_ids: Vec::new(),
        };

        self.segments.insert(id.clone(), segment.clone());
        self.push_cache(&id);

        // Add to hierarchy level 0.
        if let Some(lvl) = self.hierarchy.get_mut(0) {
            lvl.total_tokens += tokens;
            lvl.segments.push(segment);
        }

        Ok(id)
    }

    /// Look up a segment by ID (increments access counters).
    pub fn get_segment(&mut self, id: &str) -> Option<&ContextSegment> {
        // Two-phase to satisfy borrow checker.
        if self.segments.contains_key(id) {
            self.cache_hits += 1;
            let seg = self.segments.get_mut(id).expect("checked above");
            seg.access_count += 1;
            seg.last_accessed = now_secs();
            return self.segments.get(id);
        }
        self.cache_misses += 1;
        None
    }

    /// Remove a segment entirely.
    pub fn remove_segment(&mut self, id: &str) -> Result<(), StreamingError> {
        let seg = self
            .segments
            .remove(id)
            .ok_or(StreamingError::SegmentNotFound)?;
        // Remove from hierarchy.
        if let Some(lvl) = self.hierarchy.get_mut(seg.level as usize) {
            lvl.total_tokens = lvl.total_tokens.saturating_sub(seg.tokens);
            lvl.segments.retain(|s| s.id != id);
        }
        // Remove from window.
        self.window.segments.retain(|s| s != id);
        self.window.total_tokens = self.window.total_tokens.saturating_sub(seg.tokens);
        // Remove from cache order.
        self.cache_order.retain(|s| s != id);
        Ok(())
    }

    // -- Summarisation -------------------------------------------------------

    /// Create a truncation-based summary keeping the first `target_tokens`
    /// worth of characters.
    pub fn summarize_content(&self, content: &str, target_tokens: usize) -> String {
        let target_chars = target_tokens * 4;
        if content.len() <= target_chars || target_chars == 0 {
            return content.to_string();
        }
        // Find a clean break point near the target.
        let end = content[..target_chars]
            .rfind(['.', '\n'])
            .unwrap_or(target_chars);
        let mut s = content[..end].to_string();
        s.push_str("...");
        s
    }

    // -- Hierarchy -----------------------------------------------------------

    /// Build summary hierarchy for a segment across all configured levels.
    pub fn build_hierarchy(&mut self, segment_id: &str) -> Result<(), StreamingError> {
        let base = self
            .segments
            .get(segment_id)
            .ok_or(StreamingError::SegmentNotFound)?
            .clone();

        let content = base.full_content.as_deref().unwrap_or(&base.summary);

        let mut parent_id = segment_id.to_string();
        let mut current_content = content.to_string();
        let mut current_tokens = base.tokens;

        for lvl in 1..self.config.summary_levels {
            let target = (current_tokens as f32 * self.config.compression_ratio) as usize;
            if target == 0 {
                break;
            }
            let summary = self.summarize_content(&current_content, target);
            let sum_tokens = Self::estimate_tokens(&summary);

            let id = format!("{}-L{}", segment_id, lvl);
            let seg = ContextSegment {
                id: id.clone(),
                source: base.source.clone(),
                content_hash: fnv1a_hash(&summary),
                summary: summary.clone(),
                full_content: Some(summary.clone()),
                tokens: sum_tokens,
                relevance_score: base.relevance_score,
                access_count: 0,
                last_accessed: now_secs(),
                level: lvl as u32,
                children_ids: vec![parent_id.clone()],
            };

            self.segments.insert(id.clone(), seg.clone());

            if let Some(h) = self.hierarchy.get_mut(lvl) {
                h.total_tokens += sum_tokens;
                h.segments.push(seg);
            }

            parent_id = id;
            current_content = summary;
            current_tokens = sum_tokens;
        }

        Ok(())
    }

    /// Return all segments at a given hierarchy level.
    pub fn get_level_segments(&self, level: u32) -> Vec<&ContextSegment> {
        self.segments
            .values()
            .filter(|s| s.level == level)
            .collect()
    }

    // -- Query ---------------------------------------------------------------

    /// Search across the hierarchy for segments matching a query.
    pub fn query(&self, q: &ContextQuery) -> ContextQueryResult {
        let start = now_secs();

        let mut matches: Vec<ContextMatch> = Vec::new();
        let mut total_searched: usize = 0;
        let mut tokens_used: usize = 0;

        for seg in self.segments.values() {
            // Level filter.
            if let Some(ref levels) = q.levels {
                if !levels.contains(&seg.level) {
                    continue;
                }
            }
            total_searched += 1;

            let text = if q.include_summaries {
                &seg.summary
            } else {
                seg.full_content.as_deref().unwrap_or(&seg.summary)
            };

            let score = Self::score_relevance(&q.query, text);
            if score >= q.min_relevance {
                tokens_used += seg.tokens;
                matches.push(ContextMatch {
                    segment_id: seg.id.clone(),
                    source: seg.source.clone(),
                    content: text.to_string(),
                    relevance_score: score,
                    level: seg.level,
                    tokens: seg.tokens,
                });
            }
        }

        // Sort descending by relevance then truncate.
        matches.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(q.max_results);

        let duration_ms = now_secs().saturating_sub(start) * 1000;
        ContextQueryResult {
            query: q.query.clone(),
            matches,
            total_searched,
            tokens_used,
            duration_ms,
        }
    }

    /// Keyword-overlap relevance scorer.
    pub fn score_relevance(query: &str, content: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        if keywords.is_empty() {
            return 0.0;
        }
        let hits = keywords
            .iter()
            .filter(|kw| content_lower.contains(*kw))
            .count();
        hits as f32 / keywords.len() as f32
    }

    // -- Sliding window ------------------------------------------------------

    /// Add a segment to the sliding window, evicting if necessary.
    pub fn add_to_window(&mut self, segment_id: &str) -> Result<(), StreamingError> {
        let tokens = self
            .segments
            .get(segment_id)
            .ok_or(StreamingError::SegmentNotFound)?
            .tokens;

        // Evict until there is room.
        while self.window.total_tokens + tokens > self.window.max_tokens {
            if self.window.segments.is_empty() {
                return Err(StreamingError::WindowFull);
            }
            self.evict_from_window();
        }

        self.window.segments.push(segment_id.to_string());
        self.window.total_tokens += tokens;
        Ok(())
    }

    /// Evict the lowest-value segment from the window based on the configured
    /// strategy.
    pub fn evict_from_window(&mut self) {
        if self.window.segments.is_empty() {
            return;
        }

        let victim_idx = self.pick_eviction_victim();
        let victim_id = self.window.segments.remove(victim_idx);

        if let Some(seg) = self.segments.get_mut(&victim_id) {
            self.window.total_tokens = self.window.total_tokens.saturating_sub(seg.tokens);
            // Evict full content to save memory — keep summary.
            seg.full_content = None;
        }
        self.eviction_count += 1;
    }

    /// Return references to segments currently in the window.
    pub fn get_window_contents(&self) -> Vec<&ContextSegment> {
        self.window
            .segments
            .iter()
            .filter_map(|id| self.segments.get(id))
            .collect()
    }

    /// Expand a previously evicted segment (reload full content from cache).
    pub fn expand_segment(&mut self, id: &str) -> Result<String, StreamingError> {
        let seg = self
            .segments
            .get(id)
            .ok_or(StreamingError::SegmentNotFound)?;
        if let Some(ref content) = seg.full_content {
            self.cache_hits += 1;
            return Ok(content.clone());
        }
        self.cache_misses += 1;
        // In a real implementation this would reload from disk.  Here we
        // fall back to the summary.
        Err(StreamingError::CacheMiss)
    }

    // -- Compression ---------------------------------------------------------

    /// Create a compressed copy of a segment (summary only, no full content).
    pub fn compress_segment(&self, segment: &ContextSegment) -> ContextSegment {
        let summary_tokens = Self::estimate_tokens(&segment.summary);
        ContextSegment {
            id: format!("{}-compressed", segment.id),
            source: segment.source.clone(),
            content_hash: segment.content_hash,
            summary: segment.summary.clone(),
            full_content: None,
            tokens: summary_tokens,
            relevance_score: segment.relevance_score,
            access_count: segment.access_count,
            last_accessed: segment.last_accessed,
            level: segment.level,
            children_ids: segment.children_ids.clone(),
        }
    }

    // -- Statistics ----------------------------------------------------------

    /// Compute current engine statistics.
    pub fn get_stats(&self) -> ContextStats {
        let total_segments = self.segments.len();
        let total_tokens: usize = self.segments.values().map(|s| s.tokens).sum();
        let tokens_in_window = self.window.total_tokens;
        let tokens_summarized: usize = self
            .segments
            .values()
            .filter(|s| s.full_content.is_none())
            .map(|s| s.tokens)
            .sum();

        let total_accesses = self.cache_hits + self.cache_misses;
        let cache_hit_rate = if total_accesses > 0 {
            self.cache_hits as f32 / total_accesses as f32
        } else {
            0.0
        };

        let full_tokens: usize = self
            .segments
            .values()
            .filter(|s| s.full_content.is_some())
            .map(|s| Self::estimate_tokens(s.full_content.as_deref().unwrap_or("")))
            .sum();
        let summary_only_tokens: usize = self
            .segments
            .values()
            .map(|s| Self::estimate_tokens(&s.summary))
            .sum();
        let compression_savings_percent = if full_tokens > 0 {
            (1.0 - summary_only_tokens as f32 / full_tokens as f32) * 100.0
        } else {
            0.0
        };

        let mut levels: Vec<LevelStats> = Vec::new();
        for lvl in 0..self.config.summary_levels {
            let segs: Vec<&ContextSegment> = self
                .segments
                .values()
                .filter(|s| s.level == lvl as u32)
                .collect();
            let count = segs.len();
            let tok: usize = segs.iter().map(|s| s.tokens).sum();
            let avg_rel = if count > 0 {
                segs.iter().map(|s| s.relevance_score).sum::<f32>() / count as f32
            } else {
                0.0
            };
            levels.push(LevelStats {
                level: lvl as u32,
                segment_count: count,
                total_tokens: tok,
                avg_relevance: avg_rel,
            });
        }

        ContextStats {
            total_segments,
            total_tokens,
            tokens_in_window,
            tokens_summarized,
            cache_hit_rate,
            eviction_count: self.eviction_count,
            compression_savings_percent,
            levels,
        }
    }

    // -- Internal helpers ----------------------------------------------------

    fn total_tokens(&self) -> usize {
        self.segments.values().map(|s| s.tokens).sum()
    }

    fn push_cache(&mut self, id: &str) {
        self.cache_order.push(id.to_string());
        while self.cache_order.len() > self.config.cache_size {
            let evicted = self.cache_order.remove(0);
            if let Some(seg) = self.segments.get_mut(&evicted) {
                seg.full_content = None;
            }
        }
    }

    /// Pick the index of the segment to evict based on the configured strategy.
    fn pick_eviction_victim(&self) -> usize {
        if self.window.segments.is_empty() {
            return 0;
        }
        match self.config.eviction_strategy {
            EvictionStrategy::LRU => self.pick_lru_victim(),
            EvictionStrategy::RelevanceScore => self.pick_relevance_victim(),
            EvictionStrategy::AccessFrequency => self.pick_frequency_victim(),
            EvictionStrategy::TimeDecay => self.pick_lru_victim(),
            EvictionStrategy::Hybrid => self.pick_hybrid_victim(),
        }
    }

    fn pick_lru_victim(&self) -> usize {
        self.window
            .segments
            .iter()
            .enumerate()
            .min_by_key(|(_, id)| {
                self.segments
                    .get(id.as_str())
                    .map(|s| s.last_accessed)
                    .unwrap_or(0)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn pick_relevance_victim(&self) -> usize {
        self.window
            .segments
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let sa = self
                    .segments
                    .get(a.as_str())
                    .map(|s| s.relevance_score)
                    .unwrap_or(0.0);
                let sb = self
                    .segments
                    .get(b.as_str())
                    .map(|s| s.relevance_score)
                    .unwrap_or(0.0);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn pick_frequency_victim(&self) -> usize {
        self.window
            .segments
            .iter()
            .enumerate()
            .min_by_key(|(_, id)| {
                self.segments
                    .get(id.as_str())
                    .map(|s| s.access_count)
                    .unwrap_or(0)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn pick_hybrid_victim(&self) -> usize {
        // Combine: lower score = more evictable.
        // score = relevance * 0.4 + frequency_norm * 0.3 + recency_norm * 0.3
        let now = now_secs();
        self.window
            .segments
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let score = |id: &str| -> f64 {
                    let seg = match self.segments.get(id) {
                        Some(s) => s,
                        None => return 0.0,
                    };
                    let rel = seg.relevance_score as f64 * 0.4;
                    let freq = (seg.access_count as f64).min(100.0) / 100.0 * 0.3;
                    let age = now.saturating_sub(seg.last_accessed) as f64;
                    let recency = (1.0 / (1.0 + age)) * 0.3;
                    rel + freq + recency
                };
                let sa = score(a);
                let sb = score(b);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers -------------------------------------------------------------

    fn default_engine() -> ContextStreamingEngine {
        ContextStreamingEngine::new(StreamingConfig::default())
    }

    fn small_engine() -> ContextStreamingEngine {
        let config = StreamingConfig {
            max_tokens: 1000,
            window_size: 500,
            overlap_tokens: 50,
            summary_levels: 5,
            eviction_strategy: EvictionStrategy::LRU,
            compression_ratio: 0.1,
            cache_size: 10,
        };
        ContextStreamingEngine::new(config)
    }

    fn sample_content() -> &'static str {
        "The quick brown fox jumps over the lazy dog. \
         This is a sample paragraph for testing the context streaming engine. \
         It contains multiple sentences to exercise summarization and \
         token estimation logic across the hierarchy levels."
    }

    // -- StreamingConfig defaults --------------------------------------------

    #[test]
    fn test_config_defaults() {
        let cfg = StreamingConfig::default();
        assert_eq!(cfg.max_tokens, 10_000_000);
        assert_eq!(cfg.window_size, 1_000_000);
        assert_eq!(cfg.overlap_tokens, 50_000);
        assert_eq!(cfg.summary_levels, 5);
        assert_eq!(cfg.eviction_strategy, EvictionStrategy::Hybrid);
        assert!((cfg.compression_ratio - 0.1).abs() < f32::EPSILON);
        assert_eq!(cfg.cache_size, 1000);
    }

    // -- Token estimation ----------------------------------------------------

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(ContextStreamingEngine::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // "abcd" = 4 chars → 1 token
        assert_eq!(ContextStreamingEngine::estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_estimate_tokens_longer() {
        // 100 chars → 25 tokens
        let text = "a".repeat(100);
        assert_eq!(ContextStreamingEngine::estimate_tokens(&text), 25);
    }

    // -- Segment CRUD --------------------------------------------------------

    #[test]
    fn test_add_segment() {
        let mut engine = default_engine();
        let id = engine.add_segment("file.rs", sample_content()).unwrap();
        assert!(id.starts_with("seg-"));
        assert_eq!(engine.segments.len(), 1);
    }

    #[test]
    fn test_add_segment_token_limit() {
        let mut engine = small_engine();
        let big = "x".repeat(5000); // ~1250 tokens, exceeds 1000 limit
        let result = engine.add_segment("big.rs", &big);
        assert_eq!(result, Err(StreamingError::TokenLimitExceeded));
    }

    #[test]
    fn test_get_segment() {
        let mut engine = default_engine();
        let id = engine.add_segment("a.rs", "hello world").unwrap();
        let seg = engine.get_segment(&id).unwrap();
        assert_eq!(seg.source, "a.rs");
        assert_eq!(seg.access_count, 1);
    }

    #[test]
    fn test_get_segment_not_found() {
        let mut engine = default_engine();
        assert!(engine.get_segment("nonexistent").is_none());
    }

    #[test]
    fn test_remove_segment() {
        let mut engine = default_engine();
        let id = engine.add_segment("a.rs", "content").unwrap();
        engine.remove_segment(&id).unwrap();
        assert!(engine.segments.is_empty());
    }

    #[test]
    fn test_remove_segment_not_found() {
        let mut engine = default_engine();
        let result = engine.remove_segment("nope");
        assert_eq!(result, Err(StreamingError::SegmentNotFound));
    }

    #[test]
    fn test_multiple_segments() {
        let mut engine = default_engine();
        let id1 = engine.add_segment("a.rs", "first").unwrap();
        let id2 = engine.add_segment("b.rs", "second").unwrap();
        assert_ne!(id1, id2);
        assert_eq!(engine.segments.len(), 2);
    }

    // -- Summarisation -------------------------------------------------------

    #[test]
    fn test_summarize_short_content() {
        let engine = default_engine();
        let summary = engine.summarize_content("short", 100);
        assert_eq!(summary, "short");
    }

    #[test]
    fn test_summarize_truncates() {
        let engine = default_engine();
        let long = "word ".repeat(200); // ~1000 chars
        let summary = engine.summarize_content(&long, 5); // target = 20 chars
        assert!(summary.len() < long.len());
        assert!(summary.ends_with("..."));
    }

    // -- Hierarchy -----------------------------------------------------------

    #[test]
    fn test_build_hierarchy() {
        let mut engine = default_engine();
        let id = engine.add_segment("file.rs", &"x ".repeat(500)).unwrap();
        engine.build_hierarchy(&id).unwrap();
        // Should create segments at levels 1..4.
        let l1: Vec<_> = engine.get_level_segments(1);
        assert!(!l1.is_empty());
    }

    #[test]
    fn test_build_hierarchy_not_found() {
        let mut engine = default_engine();
        let result = engine.build_hierarchy("ghost");
        assert_eq!(result, Err(StreamingError::SegmentNotFound));
    }

    #[test]
    fn test_hierarchy_levels_decrease_tokens() {
        let mut engine = default_engine();
        let content = "function foo() { return 42; } ".repeat(100);
        let id = engine.add_segment("big.js", &content).unwrap();
        engine.build_hierarchy(&id).unwrap();

        let l0_tokens: usize = engine.get_level_segments(0).iter().map(|s| s.tokens).sum();
        let l1_tokens: usize = engine.get_level_segments(1).iter().map(|s| s.tokens).sum();
        assert!(
            l1_tokens < l0_tokens,
            "level 1 should have fewer tokens than level 0"
        );
    }

    // -- Query ---------------------------------------------------------------

    #[test]
    fn test_query_finds_match() {
        let mut engine = default_engine();
        engine
            .add_segment("animals.txt", "The quick brown fox jumps over the lazy dog")
            .unwrap();
        let q = ContextQuery {
            query: "fox".to_string(),
            max_results: 10,
            min_relevance: 0.3,
            include_summaries: false, // Search full content, not truncated summary
            levels: None,
        };
        let result = engine.query(&q);
        assert!(!result.matches.is_empty());
        assert!(result.matches[0].relevance_score > 0.0);
    }

    #[test]
    fn test_query_no_match() {
        let mut engine = default_engine();
        engine.add_segment("a.txt", "hello world").unwrap();
        let q = ContextQuery {
            query: "zzzznotfound".to_string(),
            max_results: 10,
            min_relevance: 0.5,
            include_summaries: true,
            levels: None,
        };
        let result = engine.query(&q);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn test_query_level_filter() {
        let mut engine = default_engine();
        let id = engine
            .add_segment("data.rs", &"rust code pattern ".repeat(50))
            .unwrap();
        engine.build_hierarchy(&id).unwrap();
        let q = ContextQuery {
            query: "rust code".to_string(),
            max_results: 10,
            min_relevance: 0.0,
            include_summaries: true,
            levels: Some(vec![0]),
        };
        let result = engine.query(&q);
        for m in &result.matches {
            assert_eq!(m.level, 0);
        }
    }

    #[test]
    fn test_query_max_results() {
        let mut engine = default_engine();
        for i in 0..20 {
            engine
                .add_segment(&format!("f{i}.rs"), "common keyword here")
                .unwrap();
        }
        let q = ContextQuery {
            query: "keyword".to_string(),
            max_results: 5,
            min_relevance: 0.0,
            include_summaries: true,
            levels: None,
        };
        let result = engine.query(&q);
        assert!(result.matches.len() <= 5);
    }

    // -- Relevance scoring ---------------------------------------------------

    #[test]
    fn test_score_relevance_full_match() {
        let score = ContextStreamingEngine::score_relevance("hello world", "hello world foo");
        assert!((score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_relevance_partial() {
        let score = ContextStreamingEngine::score_relevance("hello world", "hello foo");
        assert!((score - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_relevance_no_match() {
        let score = ContextStreamingEngine::score_relevance("abc", "xyz");
        assert!(score < f32::EPSILON);
    }

    #[test]
    fn test_score_relevance_empty_query() {
        let score = ContextStreamingEngine::score_relevance("", "anything");
        assert!(score < f32::EPSILON);
    }

    // -- Sliding window ------------------------------------------------------

    #[test]
    fn test_add_to_window() {
        let mut engine = default_engine();
        let id = engine.add_segment("w.rs", "window content").unwrap();
        engine.add_to_window(&id).unwrap();
        assert_eq!(engine.window.segments.len(), 1);
        assert!(engine.window.total_tokens > 0);
    }

    #[test]
    fn test_add_to_window_not_found() {
        let mut engine = default_engine();
        let result = engine.add_to_window("ghost");
        assert_eq!(result, Err(StreamingError::SegmentNotFound));
    }

    #[test]
    fn test_window_eviction() {
        let config = StreamingConfig {
            max_tokens: 10000,
            window_size: 100,  // Small window to force eviction
            overlap_tokens: 10,
            ..StreamingConfig::default()
        };
        let mut engine = ContextStreamingEngine::new(config);
        // Fill window beyond capacity — each segment is ~50 tokens
        for i in 0..10 {
            let content = "x".repeat(200); // 200 chars ≈ 50 tokens
            let id = engine.add_segment(&format!("f{i}.rs"), &content).unwrap();
            engine.add_to_window(&id).unwrap();
        }
        assert!(engine.eviction_count > 0);
    }

    #[test]
    fn test_get_window_contents() {
        let mut engine = default_engine();
        let id = engine.add_segment("a.rs", "hello").unwrap();
        engine.add_to_window(&id).unwrap();
        let contents = engine.get_window_contents();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].source, "a.rs");
    }

    // -- Expand / Cache ------------------------------------------------------

    #[test]
    fn test_expand_segment_present() {
        let mut engine = default_engine();
        let id = engine.add_segment("e.rs", "expand me").unwrap();
        let content = engine.expand_segment(&id).unwrap();
        assert_eq!(content, "expand me");
    }

    #[test]
    fn test_expand_segment_evicted() {
        let mut engine = small_engine();
        let id = engine.add_segment("e.rs", "data").unwrap();
        // Force eviction of full content.
        engine.segments.get_mut(&id).unwrap().full_content = None;
        let result = engine.expand_segment(&id);
        assert_eq!(result, Err(StreamingError::CacheMiss));
    }

    #[test]
    fn test_expand_segment_not_found() {
        let mut engine = default_engine();
        let result = engine.expand_segment("nope");
        assert_eq!(result, Err(StreamingError::SegmentNotFound));
    }

    // -- Compression ---------------------------------------------------------

    #[test]
    fn test_compress_segment() {
        let mut engine = default_engine();
        let id = engine.add_segment("c.rs", sample_content()).unwrap();
        let seg = engine.segments.get(&id).unwrap().clone();
        let compressed = engine.compress_segment(&seg);
        assert!(compressed.full_content.is_none());
        assert!(compressed.id.ends_with("-compressed"));
        assert!(compressed.tokens <= seg.tokens);
    }

    // -- Statistics ----------------------------------------------------------

    #[test]
    fn test_get_stats_empty() {
        let engine = default_engine();
        let stats = engine.get_stats();
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.tokens_in_window, 0);
        assert_eq!(stats.eviction_count, 0);
    }

    #[test]
    fn test_get_stats_with_data() {
        let mut engine = default_engine();
        engine.add_segment("a.rs", "hello world").unwrap();
        engine.add_segment("b.rs", "goodbye world").unwrap();
        let stats = engine.get_stats();
        assert_eq!(stats.total_segments, 2);
        assert!(stats.total_tokens > 0);
        assert_eq!(stats.levels.len(), 5);
    }

    #[test]
    fn test_stats_level_breakdown() {
        let mut engine = default_engine();
        let id = engine.add_segment("x.rs", &"token ".repeat(200)).unwrap();
        engine.build_hierarchy(&id).unwrap();
        let stats = engine.get_stats();
        assert!(stats.levels[0].segment_count >= 1);
    }

    // -- Eviction strategies -------------------------------------------------

    #[test]
    fn test_eviction_strategy_relevance() {
        let config = StreamingConfig {
            max_tokens: 2000,
            window_size: 60, // Small window: can hold ~2 small segments but not 3
            eviction_strategy: EvictionStrategy::RelevanceScore,
            ..StreamingConfig::default()
        };
        let mut engine = ContextStreamingEngine::new(config);
        // Each segment ~25 tokens (100 chars / 4)
        let id1 = engine.add_segment("low.rs", &"a".repeat(100)).unwrap();
        engine.segments.get_mut(&id1).unwrap().relevance_score = 0.1;
        let id2 = engine.add_segment("high.rs", &"b".repeat(100)).unwrap();
        engine.segments.get_mut(&id2).unwrap().relevance_score = 0.9;
        engine.add_to_window(&id1).unwrap();
        engine.add_to_window(&id2).unwrap();
        // Third segment forces eviction (25+25+25 = 75 > 60)
        let _id3 = engine.add_segment("mid.rs", &"c".repeat(100)).unwrap();
        engine.add_to_window(&_id3).unwrap();
        // Low relevance segment should have been evicted.
        assert!(!engine.window.segments.contains(&id1));
    }

    #[test]
    fn test_eviction_strategy_frequency() {
        let config = StreamingConfig {
            max_tokens: 2000,
            window_size: 60, // Small window: forces eviction on third segment
            eviction_strategy: EvictionStrategy::AccessFrequency,
            ..StreamingConfig::default()
        };
        let mut engine = ContextStreamingEngine::new(config);
        let id1 = engine.add_segment("rare.rs", &"a".repeat(100)).unwrap();
        let id2 = engine.add_segment("freq.rs", &"b".repeat(100)).unwrap();
        // Bump access count on id2.
        engine.segments.get_mut(&id2).unwrap().access_count = 100;
        engine.add_to_window(&id1).unwrap();
        engine.add_to_window(&id2).unwrap();
        let id3 = engine.add_segment("new.rs", &"c".repeat(100)).unwrap();
        engine.add_to_window(&id3).unwrap();
        // Rarely accessed segment should be evicted.
        assert!(!engine.window.segments.contains(&id1));
    }

    // -- Error display -------------------------------------------------------

    #[test]
    fn test_error_display() {
        assert_eq!(
            StreamingError::SegmentNotFound.to_string(),
            "segment not found"
        );
        assert_eq!(
            StreamingError::WindowFull.to_string(),
            "sliding window is full"
        );
        assert_eq!(
            StreamingError::TokenLimitExceeded.to_string(),
            "token limit exceeded"
        );
        assert_eq!(
            StreamingError::CompressionFailed.to_string(),
            "compression failed"
        );
        assert_eq!(
            StreamingError::InvalidLevel.to_string(),
            "invalid hierarchy level"
        );
        assert_eq!(StreamingError::CacheMiss.to_string(), "cache miss");
        assert_eq!(
            StreamingError::QueryError("oops".into()).to_string(),
            "query error: oops"
        );
    }

    // -- SlidingWindow struct ------------------------------------------------

    #[test]
    fn test_sliding_window_new() {
        let w = SlidingWindow::new(1_000_000);
        assert!(w.segments.is_empty());
        assert_eq!(w.total_tokens, 0);
        assert_eq!(w.max_tokens, 1_000_000);
        assert_eq!(w.cursor_position, 0);
    }

    // -- ContextQuery defaults -----------------------------------------------

    #[test]
    fn test_context_query_defaults() {
        let q = ContextQuery::new("search term");
        assert_eq!(q.query, "search term");
        assert_eq!(q.max_results, 10);
        assert!((q.min_relevance - 0.3).abs() < f32::EPSILON);
        assert!(q.include_summaries);
        assert!(q.levels.is_none());
    }

    // -- ContextHierarchyLevel -----------------------------------------------

    #[test]
    fn test_hierarchy_level_new() {
        let lvl = ContextHierarchyLevel::new(3);
        assert_eq!(lvl.level, 3);
        assert!(lvl.segments.is_empty());
        assert_eq!(lvl.total_tokens, 0);
    }

    // -- FNV hash helper -----------------------------------------------------

    #[test]
    fn test_fnv_hash_deterministic() {
        let h1 = fnv1a_hash("hello");
        let h2 = fnv1a_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fnv_hash_different_inputs() {
        let h1 = fnv1a_hash("hello");
        let h2 = fnv1a_hash("world");
        assert_ne!(h1, h2);
    }

    // -- Cache eviction via push_cache ---------------------------------------

    #[test]
    fn test_cache_eviction_on_overflow() {
        let config = StreamingConfig {
            cache_size: 3,
            ..StreamingConfig::default()
        };
        let mut engine = ContextStreamingEngine::new(config);
        for i in 0..5 {
            engine
                .add_segment(&format!("f{i}.rs"), &format!("content {i}"))
                .unwrap();
        }
        // First two segments should have their full_content evicted.
        let seg1 = engine.segments.get("seg-1").unwrap();
        assert!(
            seg1.full_content.is_none(),
            "seg-1 should have been cache-evicted"
        );
        let seg2 = engine.segments.get("seg-2").unwrap();
        assert!(
            seg2.full_content.is_none(),
            "seg-2 should have been cache-evicted"
        );
        // Most recent should still have content.
        let seg5 = engine.segments.get("seg-5").unwrap();
        assert!(seg5.full_content.is_some());
    }

    // -- Remove from window on segment removal -------------------------------

    #[test]
    fn test_remove_segment_cleans_window() {
        let mut engine = default_engine();
        let id = engine.add_segment("w.rs", "windowed").unwrap();
        engine.add_to_window(&id).unwrap();
        assert_eq!(engine.window.segments.len(), 1);
        engine.remove_segment(&id).unwrap();
        assert!(engine.window.segments.is_empty());
        assert_eq!(engine.window.total_tokens, 0);
    }
}
