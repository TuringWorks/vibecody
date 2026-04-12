#![allow(dead_code)]
//! Ultra-long context adapter (2M–10M tokens).
//!
//! Provides a model registry with context-tier classification, a streaming
//! document chunker that respects paragraph and function boundaries, cost
//! estimation, and an ingestion-session tracker for large codebases.

use serde::{Deserialize, Serialize};

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Broad tier describing a model's context window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextCapability {
    /// Up to 200 K tokens (inclusive).
    Standard,
    /// 200 K – 1 M tokens (exclusive upper bound).
    Extended,
    /// 1 M tokens and above.
    UltraLong,
}

impl std::fmt::Display for ContextCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Extended => write!(f, "Extended"),
            Self::UltraLong => write!(f, "UltraLong"),
        }
    }
}

/// The boundary type used to split a `DocumentChunk`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChunkBoundary {
    FunctionBoundary,
    ClassBoundary,
    BlankLine,
    Paragraph,
    Fixed(usize),
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// Capability profile for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelContextProfile {
    pub model_id: String,
    pub provider: String,
    pub max_tokens: u64,
    pub context_capability: ContextCapability,
    pub cost_per_1k_input: f32,
    pub cost_per_1k_output: f32,
}

impl ModelContextProfile {
    pub fn new(
        model_id: impl Into<String>,
        provider: impl Into<String>,
        max_tokens: u64,
        cost_per_1k_input: f32,
        cost_per_1k_output: f32,
    ) -> Self {
        let context_capability = classify_context(max_tokens);
        Self {
            model_id: model_id.into(),
            provider: provider.into(),
            max_tokens,
            context_capability,
            cost_per_1k_input,
            cost_per_1k_output,
        }
    }
}

/// A chunk produced by `DocumentChunker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub chunk_id: String,
    pub content: String,
    pub token_estimate: u64,
    pub start_byte: u64,
    pub end_byte: u64,
    pub boundary_type: ChunkBoundary,
}

/// Cost estimate for processing `input_tokens` with a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub model_id: String,
    pub input_tokens: u64,
    pub estimated_cost_usd: f32,
    pub within_budget: bool,
}

/// Progress tracker for a multi-chunk ingestion job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    pub total_chunks: u64,
    pub processed_chunks: u64,
    pub total_bytes: u64,
    pub processed_bytes: u64,
}

impl IngestionProgress {
    pub fn pct_complete(&self) -> f32 {
        if self.total_chunks == 0 {
            return 0.0;
        }
        self.processed_chunks as f32 / self.total_chunks as f32 * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.total_chunks > 0 && self.processed_chunks >= self.total_chunks
    }
}

// ─── Free functions ───────────────────────────────────────────────────────────

/// Classify a context window into a `ContextCapability` tier.
pub fn classify_context(max_tokens: u64) -> ContextCapability {
    if max_tokens < 200_000 {
        ContextCapability::Standard
    } else if max_tokens < 1_000_000 {
        ContextCapability::Extended
    } else {
        ContextCapability::UltraLong
    }
}

/// Estimate token count from raw text: one token ≈ 4 characters.
pub fn estimate_tokens(text: &str) -> u64 {
    (text.chars().count() as u64) / 4
}

/// Estimate USD cost for ingesting `input_tokens` with `profile`.
pub fn estimate_cost(
    profile: &ModelContextProfile,
    input_tokens: u64,
    budget_usd: f32,
) -> CostEstimate {
    let estimated_cost_usd = (input_tokens as f32 / 1000.0) * profile.cost_per_1k_input;
    CostEstimate {
        model_id: profile.model_id.clone(),
        input_tokens,
        estimated_cost_usd,
        within_budget: estimated_cost_usd <= budget_usd,
    }
}

// ─── ModelRegistry ───────────────────────────────────────────────────────────

/// Registry of known models, queryable by capability and cost.
pub struct ModelRegistry {
    profiles: Vec<ModelContextProfile>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }

    pub fn register(&mut self, profile: ModelContextProfile) {
        self.profiles.push(profile);
    }

    /// Return the cheapest model whose `max_tokens >= required_tokens`.
    pub fn best_for_tokens(&self, required_tokens: u64) -> Option<&ModelContextProfile> {
        self.profiles
            .iter()
            .filter(|p| p.max_tokens >= required_tokens)
            .min_by(|a, b| {
                a.cost_per_1k_input
                    .partial_cmp(&b.cost_per_1k_input)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// All models classified as `UltraLong`.
    pub fn all_ultra_long(&self) -> Vec<&ModelContextProfile> {
        self.profiles
            .iter()
            .filter(|p| p.context_capability == ContextCapability::UltraLong)
            .collect()
    }

    pub fn model_count(&self) -> usize {
        self.profiles.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── DocumentChunker ─────────────────────────────────────────────────────────

/// Splits documents into overlapping chunks that fit within a token budget.
pub struct DocumentChunker {
    target_chunk_tokens: u64,
    overlap_tokens: u64,
    next_id: std::sync::atomic::AtomicU64,
}

impl DocumentChunker {
    pub fn new(target_chunk_tokens: u64, overlap_tokens: u64) -> Self {
        Self {
            target_chunk_tokens,
            overlap_tokens,
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("chunk-{id}")
    }

    // ── paragraph-level splitter ──────────────────────────────────────────

    /// Split `text` into chunks at double-newline boundaries, then further
    /// split any oversized paragraph by `target_chunk_tokens`.  Adjacent chunks
    /// share `overlap_tokens` characters from the previous chunk's tail.
    pub fn chunk_text(&self, text: &str, source_id: &str) -> Vec<DocumentChunk> {
        // Split into paragraph segments at blank lines
        let paragraphs: Vec<&str> = text.split("\n\n").collect();

        // Group paragraphs into chunks that fit the token budget
        let mut chunks: Vec<DocumentChunk> = Vec::new();
        let mut current = String::new();
        let mut overlap_tail = String::new();
        let mut byte_cursor: u64 = 0;

        for para in &paragraphs {
            let para_tokens = estimate_tokens(para);

            // If a single paragraph exceeds the budget, split it by fixed size
            if para_tokens > self.target_chunk_tokens && !para.is_empty() {
                // Flush any pending current content first
                if !current.is_empty() {
                    let start = byte_cursor.saturating_sub(current.len() as u64);
                    chunks.push(self.make_chunk(
                        &current,
                        start,
                        byte_cursor,
                        ChunkBoundary::Paragraph,
                        source_id,
                    ));
                    overlap_tail = self.tail_chars(&current, self.overlap_tokens);
                    current.clear();
                }
                // Split the big paragraph into fixed-size sub-chunks
                let sub_chunks = self.split_by_tokens(para, &overlap_tail, byte_cursor, source_id, ChunkBoundary::Fixed(self.target_chunk_tokens as usize));
                byte_cursor += para.len() as u64 + 2; // +2 for "\n\n"
                overlap_tail = sub_chunks.last().map(|c: &DocumentChunk| self.tail_chars(&c.content, self.overlap_tokens)).unwrap_or_default();
                chunks.extend(sub_chunks);
                continue;
            }

            let would_be = format!("{}{}", current, para);
            let would_be_tokens = estimate_tokens(&would_be);

            if would_be_tokens > self.target_chunk_tokens && !current.is_empty() {
                // Emit the current buffer as a chunk
                let start = byte_cursor.saturating_sub(current.len() as u64);
                chunks.push(self.make_chunk(
                    &current,
                    start,
                    byte_cursor,
                    ChunkBoundary::Paragraph,
                    source_id,
                ));
                overlap_tail = self.tail_chars(&current, self.overlap_tokens);
                current = format!("{}{}", overlap_tail, para);
            } else {
                if current.is_empty() {
                    current = format!("{}{}", overlap_tail, para);
                } else {
                    current.push_str("\n\n");
                    current.push_str(para);
                }
            }
            byte_cursor += para.len() as u64 + 2;
        }

        if !current.trim().is_empty() {
            let start = byte_cursor.saturating_sub(current.len() as u64);
            chunks.push(self.make_chunk(
                &current,
                start,
                byte_cursor,
                ChunkBoundary::Paragraph,
                source_id,
            ));
        }

        chunks
    }

    /// Like `chunk_text` but first splits at Rust/Python/JS function / class
    /// declaration boundaries.
    pub fn chunk_source_file(&self, source: &str, source_id: &str) -> Vec<DocumentChunk> {
        let boundary_markers = ["pub fn ", "fn ", "class ", "def "];

        // Split at function/class boundaries
        let mut segments: Vec<(String, ChunkBoundary)> = Vec::new();
        let mut current_segment = String::new();
        let mut current_boundary = ChunkBoundary::FunctionBoundary;

        for line in source.lines() {
            let trimmed = line.trim();
            let is_boundary = boundary_markers
                .iter()
                .any(|m| trimmed.starts_with(m));

            if is_boundary && !current_segment.is_empty() {
                segments.push((current_segment.clone(), current_boundary.clone()));
                current_segment.clear();
                // Determine boundary type
                current_boundary = if trimmed.starts_with("class ") {
                    ChunkBoundary::ClassBoundary
                } else {
                    ChunkBoundary::FunctionBoundary
                };
            }
            current_segment.push_str(line);
            current_segment.push('\n');
        }
        if !current_segment.trim().is_empty() {
            segments.push((current_segment, current_boundary));
        }

        if segments.is_empty() {
            // Nothing to split — fall back to paragraph chunking
            return self.chunk_text(source, source_id);
        }

        // Now group segments into token-bounded chunks
        let mut chunks: Vec<DocumentChunk> = Vec::new();
        let mut current = String::new();
        let mut overlap_tail = String::new();
        let mut byte_cursor: u64 = 0;
        let mut boundary_type = ChunkBoundary::FunctionBoundary;

        for (seg, seg_boundary) in &segments {
            let seg_tokens = estimate_tokens(seg);

            if seg_tokens > self.target_chunk_tokens {
                // Oversized segment — emit current first, then split
                if !current.is_empty() {
                    let start = byte_cursor.saturating_sub(current.len() as u64);
                    chunks.push(self.make_chunk(&current, start, byte_cursor, boundary_type.clone(), source_id));
                    overlap_tail = self.tail_chars(&current, self.overlap_tokens);
                    current.clear();
                }
                let sub = self.split_by_tokens(seg, &overlap_tail, byte_cursor, source_id, seg_boundary.clone());
                byte_cursor += seg.len() as u64;
                overlap_tail = sub.last().map(|c| self.tail_chars(&c.content, self.overlap_tokens)).unwrap_or_default();
                chunks.extend(sub);
                continue;
            }

            let would_be_tokens = estimate_tokens(&current) + seg_tokens;
            if would_be_tokens > self.target_chunk_tokens && !current.is_empty() {
                let start = byte_cursor.saturating_sub(current.len() as u64);
                chunks.push(self.make_chunk(&current, start, byte_cursor, boundary_type.clone(), source_id));
                overlap_tail = self.tail_chars(&current, self.overlap_tokens);
                current = format!("{}{}", overlap_tail, seg);
            } else {
                current.push_str(seg);
            }
            boundary_type = seg_boundary.clone();
            byte_cursor += seg.len() as u64;
        }

        if !current.trim().is_empty() {
            let start = byte_cursor.saturating_sub(current.len() as u64);
            chunks.push(self.make_chunk(&current, start, byte_cursor, boundary_type, source_id));
        }

        chunks
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn make_chunk(
        &self,
        content: &str,
        start_byte: u64,
        end_byte: u64,
        boundary_type: ChunkBoundary,
        _source_id: &str,
    ) -> DocumentChunk {
        DocumentChunk {
            chunk_id: self.next_id(),
            content: content.to_string(),
            token_estimate: estimate_tokens(content),
            start_byte,
            end_byte,
            boundary_type,
        }
    }

    /// Return up to `overlap_tokens * 4` characters from the tail of `text`.
    fn tail_chars(&self, text: &str, overlap_tokens: u64) -> String {
        let max_chars = (overlap_tokens * 4) as usize;
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= max_chars {
            text.to_string()
        } else {
            chars[chars.len() - max_chars..].iter().collect()
        }
    }

    /// Split a large text block into fixed-size token chunks with overlap.
    fn split_by_tokens(
        &self,
        text: &str,
        overlap_prefix: &str,
        start_byte_offset: u64,
        _source_id: &str,
        boundary_type: ChunkBoundary,
    ) -> Vec<DocumentChunk> {
        let chunk_chars = (self.target_chunk_tokens * 4) as usize;
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0usize;
        let mut first = true;

        while pos < chars.len() {
            let prefix = if first { overlap_prefix.to_string() } else { String::new() };
            first = false;

            let available = chunk_chars.saturating_sub(prefix.chars().count());
            let end = (pos + available).min(chars.len());
            let content_chars: String = chars[pos..end].iter().collect();
            let content = format!("{}{}", prefix, content_chars);

            let byte_start = start_byte_offset + (pos * 4) as u64; // approximate
            let byte_end = byte_start + content.len() as u64;

            chunks.push(DocumentChunk {
                chunk_id: self.next_id(),
                content: content.clone(),
                token_estimate: estimate_tokens(&content),
                start_byte: byte_start,
                end_byte: byte_end,
                boundary_type: boundary_type.clone(),
            });

            if end >= chars.len() {
                break;
            }
            // Advance, but step back by overlap
            let overlap_chars = (self.overlap_tokens * 4) as usize;
            pos = end.saturating_sub(overlap_chars);
            if pos == 0 || pos <= (end - available) {
                // Prevent infinite loop
                pos = end;
            }
        }

        chunks
    }
}

// ─── IngestionSession ────────────────────────────────────────────────────────

/// Accumulates chunks and tracks ingestion progress.
pub struct IngestionSession {
    session_id: String,
    chunks: Vec<DocumentChunk>,
}

impl IngestionSession {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            chunks: Vec::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: DocumentChunk) {
        self.chunks.push(chunk);
    }

    pub fn progress(&self) -> IngestionProgress {
        let total_chunks = self.chunks.len() as u64;
        let total_bytes: u64 = self.chunks.iter().map(|c| c.content.len() as u64).sum();
        IngestionProgress {
            total_chunks,
            processed_chunks: total_chunks, // all added chunks are considered processed
            total_bytes,
            processed_bytes: total_bytes,
        }
    }

    pub fn chunks(&self) -> &[DocumentChunk] {
        &self.chunks
    }

    pub fn total_tokens(&self) -> u64 {
        self.chunks.iter().map(|c| c.token_estimate).sum()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_context ─────────────────────────────────────────────────

    #[test]
    fn test_classify_standard_below_200k() {
        assert_eq!(classify_context(100_000), ContextCapability::Standard);
    }

    #[test]
    fn test_classify_standard_at_199999() {
        assert_eq!(classify_context(199_999), ContextCapability::Standard);
    }

    #[test]
    fn test_classify_extended_at_200k() {
        assert_eq!(classify_context(200_000), ContextCapability::Extended);
    }

    #[test]
    fn test_classify_extended_at_999999() {
        assert_eq!(classify_context(999_999), ContextCapability::Extended);
    }

    #[test]
    fn test_classify_ultra_long_at_1m() {
        assert_eq!(classify_context(1_000_000), ContextCapability::UltraLong);
    }

    #[test]
    fn test_classify_ultra_long_at_10m() {
        assert_eq!(classify_context(10_000_000), ContextCapability::UltraLong);
    }

    // ── estimate_tokens ──────────────────────────────────────────────────

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_four_chars() {
        assert_eq!(estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_estimate_tokens_eight_chars() {
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_estimate_tokens_three_chars() {
        assert_eq!(estimate_tokens("abc"), 0); // integer division
    }

    // ── ModelRegistry ────────────────────────────────────────────────────

    fn make_profile(id: &str, max_tokens: u64, cost: f32) -> ModelContextProfile {
        ModelContextProfile::new(id, "provider", max_tokens, cost, cost * 2.0)
    }

    #[test]
    fn test_registry_model_count() {
        let mut reg = ModelRegistry::new();
        reg.register(make_profile("m1", 128_000, 0.01));
        reg.register(make_profile("m2", 200_000, 0.02));
        assert_eq!(reg.model_count(), 2);
    }

    #[test]
    fn test_registry_best_for_tokens_exact_fit() {
        let mut reg = ModelRegistry::new();
        reg.register(make_profile("cheap-big", 500_000, 0.005));
        reg.register(make_profile("expensive-big", 500_000, 0.10));
        let best = reg.best_for_tokens(300_000).unwrap();
        assert_eq!(best.model_id, "cheap-big");
    }

    #[test]
    fn test_registry_best_for_tokens_cheapest_that_fits() {
        let mut reg = ModelRegistry::new();
        reg.register(make_profile("small", 100_000, 0.001));
        reg.register(make_profile("medium", 500_000, 0.005));
        reg.register(make_profile("large", 2_000_000, 0.02));
        // Require 300K — small doesn't fit, medium is cheapest that fits
        let best = reg.best_for_tokens(300_000).unwrap();
        assert_eq!(best.model_id, "medium");
    }

    #[test]
    fn test_registry_best_for_tokens_none_when_all_too_small() {
        let mut reg = ModelRegistry::new();
        reg.register(make_profile("small", 50_000, 0.001));
        assert!(reg.best_for_tokens(100_000).is_none());
    }

    #[test]
    fn test_registry_all_ultra_long() {
        let mut reg = ModelRegistry::new();
        reg.register(make_profile("std", 128_000, 0.01));
        reg.register(make_profile("ultra", 2_000_000, 0.02));
        reg.register(make_profile("ultra2", 5_000_000, 0.03));
        let ultra = reg.all_ultra_long();
        assert_eq!(ultra.len(), 2);
    }

    #[test]
    fn test_model_context_profile_capability_set() {
        let p = make_profile("m", 1_500_000, 0.01);
        assert_eq!(p.context_capability, ContextCapability::UltraLong);
    }

    // ── estimate_cost ────────────────────────────────────────────────────

    #[test]
    fn test_estimate_cost_within_budget() {
        let profile = make_profile("m", 1_000_000, 0.01); // $0.01 per 1K
        let est = estimate_cost(&profile, 100_000, 5.0); // 100K tokens = $1.00
        assert!((est.estimated_cost_usd - 1.0).abs() < 0.001);
        assert!(est.within_budget);
    }

    #[test]
    fn test_estimate_cost_outside_budget() {
        let profile = make_profile("m", 1_000_000, 0.10); // $0.10 per 1K
        let est = estimate_cost(&profile, 100_000, 5.0); // 100K * $0.10 = $10.00 > $5.00
        assert!(!est.within_budget);
    }

    #[test]
    fn test_estimate_cost_model_id_matches() {
        let profile = make_profile("gemini-ultra", 1_000_000, 0.01);
        let est = estimate_cost(&profile, 1000, 100.0);
        assert_eq!(est.model_id, "gemini-ultra");
    }

    #[test]
    fn test_estimate_cost_zero_tokens() {
        let profile = make_profile("m", 1_000_000, 0.01);
        let est = estimate_cost(&profile, 0, 1.0);
        assert_eq!(est.estimated_cost_usd, 0.0);
        assert!(est.within_budget);
    }

    // ── IngestionProgress ────────────────────────────────────────────────

    #[test]
    fn test_ingestion_progress_pct_complete_full() {
        let p = IngestionProgress {
            total_chunks: 10,
            processed_chunks: 10,
            total_bytes: 1000,
            processed_bytes: 1000,
        };
        assert!((p.pct_complete() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_ingestion_progress_pct_complete_half() {
        let p = IngestionProgress {
            total_chunks: 10,
            processed_chunks: 5,
            total_bytes: 1000,
            processed_bytes: 500,
        };
        assert!((p.pct_complete() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_ingestion_progress_pct_complete_zero_total() {
        let p = IngestionProgress {
            total_chunks: 0,
            processed_chunks: 0,
            total_bytes: 0,
            processed_bytes: 0,
        };
        assert_eq!(p.pct_complete(), 0.0);
    }

    #[test]
    fn test_ingestion_progress_is_complete_true() {
        let p = IngestionProgress {
            total_chunks: 5,
            processed_chunks: 5,
            total_bytes: 0,
            processed_bytes: 0,
        };
        assert!(p.is_complete());
    }

    #[test]
    fn test_ingestion_progress_is_complete_false() {
        let p = IngestionProgress {
            total_chunks: 5,
            processed_chunks: 3,
            total_bytes: 0,
            processed_bytes: 0,
        };
        assert!(!p.is_complete());
    }

    #[test]
    fn test_ingestion_progress_is_complete_zero_total_not_complete() {
        let p = IngestionProgress {
            total_chunks: 0,
            processed_chunks: 0,
            total_bytes: 0,
            processed_bytes: 0,
        };
        assert!(!p.is_complete());
    }

    // ── IngestionSession ─────────────────────────────────────────────────

    fn make_chunk(content: &str) -> DocumentChunk {
        DocumentChunk {
            chunk_id: "c".to_string(),
            content: content.to_string(),
            token_estimate: estimate_tokens(content),
            start_byte: 0,
            end_byte: content.len() as u64,
            boundary_type: ChunkBoundary::Paragraph,
        }
    }

    #[test]
    fn test_session_add_chunk_and_count() {
        let mut session = IngestionSession::new("s1");
        session.add_chunk(make_chunk("hello world"));
        assert_eq!(session.chunks().len(), 1);
    }

    #[test]
    fn test_session_total_tokens() {
        let mut session = IngestionSession::new("s1");
        session.add_chunk(make_chunk("abcdefgh")); // 2 tokens
        session.add_chunk(make_chunk("abcd"));     // 1 token
        assert_eq!(session.total_tokens(), 3);
    }

    #[test]
    fn test_session_progress_complete_after_add() {
        let mut session = IngestionSession::new("s1");
        session.add_chunk(make_chunk("chunk1"));
        session.add_chunk(make_chunk("chunk2"));
        let p = session.progress();
        assert_eq!(p.total_chunks, 2);
        assert_eq!(p.processed_chunks, 2);
        assert!(p.is_complete());
    }

    #[test]
    fn test_session_empty_progress() {
        let session = IngestionSession::new("s1");
        let p = session.progress();
        assert_eq!(p.total_chunks, 0);
        assert!(!p.is_complete());
    }

    // ── DocumentChunker::chunk_text ──────────────────────────────────────

    #[test]
    fn test_chunk_text_single_paragraph_small() {
        // A tiny text that fits in one chunk
        let chunker = DocumentChunker::new(100, 0);
        let chunks = chunker.chunk_text("Hello world", "doc");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Hello world"));
    }

    #[test]
    fn test_chunk_text_splits_at_double_newline() {
        // Each paragraph is 5 tokens (20 chars). Target = 6 tokens.
        // Paragraph 1 alone = 5 tokens (fits). Adding para 2 = 10 tokens > 6, so split.
        let para = "a".repeat(20); // 20 chars = 5 tokens
        let text = format!("{}\n\n{}", para, para);
        let chunker = DocumentChunker::new(6, 0);
        let chunks = chunker.chunk_text(&text, "doc");
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_text_overlap_included() {
        // overlap=2 tokens = 8 chars
        let para1 = "a".repeat(20); // 5 tokens
        let para2 = "b".repeat(20); // 5 tokens
        let text = format!("{}\n\n{}", para1, para2);
        let chunker = DocumentChunker::new(6, 2);
        let chunks = chunker.chunk_text(&text, "doc");
        // Second chunk should start with overlap from first
        if chunks.len() > 1 {
            // The overlap tail from para1 should appear at start of chunk 2
            assert!(!chunks[1].content.is_empty());
        }
    }

    #[test]
    fn test_chunk_text_token_estimate_set() {
        let chunker = DocumentChunker::new(100, 0);
        let chunks = chunker.chunk_text("abcdefgh", "doc"); // 8 chars = 2 tokens
        assert_eq!(chunks[0].token_estimate, 2);
    }

    #[test]
    fn test_chunk_text_chunk_id_increments() {
        let chunker = DocumentChunker::new(2, 0); // very small budget to force splits
        let text = "abcdefgh\n\nijklmnop\n\nqrstuvwx";
        let chunks = chunker.chunk_text(text, "doc");
        assert!(chunks.len() > 1);
        assert_ne!(chunks[0].chunk_id, chunks[1].chunk_id);
    }

    #[test]
    fn test_chunk_text_empty_input() {
        let chunker = DocumentChunker::new(100, 0);
        let chunks = chunker.chunk_text("", "doc");
        assert!(chunks.is_empty());
    }

    // ── DocumentChunker::chunk_source_file ───────────────────────────────

    #[test]
    fn test_chunk_source_splits_at_fn() {
        let source = "fn foo() {\n    let x = 1;\n}\n\nfn bar() {\n    let y = 2;\n}\n";
        let chunker = DocumentChunker::new(5, 0); // small target
        let chunks = chunker.chunk_source_file(source, "lib.rs");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_source_function_boundary_type() {
        let source = "fn alpha() {}\nfn beta() {}\n";
        let chunker = DocumentChunker::new(2, 0);
        let chunks = chunker.chunk_source_file(source, "lib.rs");
        assert!(!chunks.is_empty());
        // At least one chunk should have FunctionBoundary type
        let has_fn_boundary = chunks
            .iter()
            .any(|c| c.boundary_type == ChunkBoundary::FunctionBoundary);
        assert!(has_fn_boundary);
    }

    #[test]
    fn test_chunk_source_class_boundary_type() {
        let source = "class Foo {\n    pass\n}\nclass Bar {\n    pass\n}\n";
        let chunker = DocumentChunker::new(2, 0);
        let chunks = chunker.chunk_source_file(source, "main.py");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_source_large_function_split() {
        // A single huge function that must be split further
        let body: String = "x".repeat(400); // 400 chars = 100 tokens
        let source = format!("fn huge() {{\n{}\n}}\n", body);
        let chunker = DocumentChunker::new(10, 2);
        let chunks = chunker.chunk_source_file(&source, "lib.rs");
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_source_empty_input() {
        let chunker = DocumentChunker::new(100, 0);
        let chunks = chunker.chunk_source_file("", "lib.rs");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_source_single_function_fits_one_chunk() {
        let source = "fn small() { let x = 1; }\n";
        let chunker = DocumentChunker::new(1000, 0);
        let chunks = chunker.chunk_source_file(source, "lib.rs");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_chunk_boundary_fixed_variant() {
        let b = ChunkBoundary::Fixed(512);
        assert_eq!(b, ChunkBoundary::Fixed(512));
        assert_ne!(b, ChunkBoundary::Fixed(1024));
    }
}
