#![allow(dead_code)]
//! Prompt cache advisor — analyzes conversations and suggests optimal caching
//! boundaries to minimize cost and latency. Matches Claude Code 1.x's cache
//! control guidance and Anthropic's prompt caching best practices.
//!
//! Cache types:
//! - **Ephemeral** — 5-minute TTL (Claude's default prompt cache)
//! - **Persistent** — up to 1-hour TTL (extended cache)
//!
//! The advisor scans a message list, measures static vs dynamic content,
//! and recommends cache_control breakpoints.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A segment of a prompt with its cache characteristics.
#[derive(Debug, Clone)]
pub struct PromptSegment {
    pub id: String,
    pub content_type: SegmentType,
    pub token_estimate: usize,
    pub change_frequency: ChangeFreq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentType {
    SystemPrompt,
    ToolDefinitions,
    ConversationHistory,
    UserTurn,
    AssistantTurn,
    RagContext,
    FileContent,
}

impl std::fmt::Display for SegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentType::SystemPrompt => write!(f, "system_prompt"),
            SegmentType::ToolDefinitions => write!(f, "tool_definitions"),
            SegmentType::ConversationHistory => write!(f, "conversation_history"),
            SegmentType::UserTurn => write!(f, "user_turn"),
            SegmentType::AssistantTurn => write!(f, "assistant_turn"),
            SegmentType::RagContext => write!(f, "rag_context"),
            SegmentType::FileContent => write!(f, "file_content"),
        }
    }
}

/// How often a segment changes across requests.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeFreq {
    /// Never changes in a session (system prompt, tool defs)
    Static,
    /// Changes occasionally (RAG context, file content)
    Rare,
    /// Changes every few turns (conversation history)
    Moderate,
    /// Changes every turn (new user message)
    PerTurn,
}

/// A cache control recommendation.
#[derive(Debug, Clone)]
pub struct CacheRecommendation {
    pub segment_id: String,
    pub cache_type: CacheType,
    pub reason: String,
    /// Estimated tokens saved per request if cached.
    pub tokens_saved: usize,
    /// Estimated cost savings per 1M requests (USD).
    pub estimated_savings_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheType {
    /// No caching — too dynamic or too small.
    None,
    /// 5-minute ephemeral cache.
    Ephemeral,
    /// Up to 1-hour persistent cache.
    Persistent,
}

impl std::fmt::Display for CacheType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheType::None => write!(f, "none"),
            CacheType::Ephemeral => write!(f, "ephemeral"),
            CacheType::Persistent => write!(f, "persistent"),
        }
    }
}

/// Summary of cache advisory analysis.
#[derive(Debug, Default)]
pub struct CacheAdvisorySummary {
    pub total_tokens: usize,
    pub cacheable_tokens: usize,
    pub recommendations: Vec<CacheRecommendation>,
    pub total_estimated_savings_usd: f64,
}

impl CacheAdvisorySummary {
    pub fn cache_ratio(&self) -> f64 {
        if self.total_tokens == 0 { return 0.0; }
        self.cacheable_tokens as f64 / self.total_tokens as f64
    }

    pub fn cache_efficiency_label(&self) -> &'static str {
        let r = self.cache_ratio();
        if r >= 0.7 { "excellent" }
        else if r >= 0.4 { "good" }
        else if r >= 0.2 { "moderate" }
        else { "poor" }
    }
}

// ---------------------------------------------------------------------------
// Pricing (per Anthropic's published rates, Apr 2026)
// ---------------------------------------------------------------------------

/// Input token prices in USD per 1K tokens (non-cached baseline).
fn input_price_per_1k(model: &str) -> f64 {
    match model {
        "claude-opus-4-6" => 0.015,
        "claude-sonnet-4-6" => 0.003,
        "claude-haiku-4-5" => 0.00025,
        _ => 0.003, // default to Sonnet pricing
    }
}

/// Cache write price multiplier (cache writes cost ~25% more than reads).
const CACHE_WRITE_MULTIPLIER: f64 = 1.25;
/// Cache read price multiplier (~10% of input cost).
const CACHE_READ_MULTIPLIER: f64 = 0.1;

fn cache_savings_per_1k_tokens(model: &str) -> f64 {
    let base = input_price_per_1k(model);
    // Savings per read = base - (base * CACHE_READ_MULTIPLIER)
    base * (1.0 - CACHE_READ_MULTIPLIER)
}

// ---------------------------------------------------------------------------
// Advisor
// ---------------------------------------------------------------------------

/// Analyzes a list of prompt segments and recommends caching strategies.
pub struct CacheAdvisor {
    pub model: String,
    /// Minimum token count to bother caching a segment.
    pub min_cacheable_tokens: usize,
}

impl Default for CacheAdvisor {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-6".to_string(),
            min_cacheable_tokens: 1024,
        }
    }
}

impl CacheAdvisor {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            min_cacheable_tokens: 1024,
        }
    }

    /// Analyze segments and produce recommendations.
    pub fn analyze(&self, segments: &[PromptSegment]) -> CacheAdvisorySummary {
        let total_tokens: usize = segments.iter().map(|s| s.token_estimate).sum();
        let mut recommendations = Vec::new();
        let mut cacheable_tokens = 0usize;

        for seg in segments {
            let rec = self.recommend_segment(seg);
            if rec.cache_type != CacheType::None {
                cacheable_tokens += seg.token_estimate;
            }
            recommendations.push(rec);
        }

        let total_savings: f64 = recommendations.iter().map(|r| r.estimated_savings_usd).sum();

        CacheAdvisorySummary {
            total_tokens,
            cacheable_tokens,
            recommendations,
            total_estimated_savings_usd: total_savings,
        }
    }

    fn recommend_segment(&self, seg: &PromptSegment) -> CacheRecommendation {
        // Too small to cache
        if seg.token_estimate < self.min_cacheable_tokens {
            return CacheRecommendation {
                segment_id: seg.id.clone(),
                cache_type: CacheType::None,
                reason: format!("Too small ({} tokens < {} min)", seg.token_estimate, self.min_cacheable_tokens),
                tokens_saved: 0,
                estimated_savings_usd: 0.0,
            };
        }

        let (cache_type, reason) = match (&seg.content_type, &seg.change_frequency) {
            // Static content → persistent cache
            (SegmentType::SystemPrompt, ChangeFreq::Static) =>
                (CacheType::Persistent, "System prompts are static — use persistent cache for maximum savings".to_string()),
            (SegmentType::ToolDefinitions, ChangeFreq::Static) =>
                (CacheType::Persistent, "Tool definitions rarely change — persistent cache is optimal".to_string()),
            (SegmentType::FileContent, ChangeFreq::Static) =>
                (CacheType::Persistent, "Static file content — persist across requests".to_string()),

            // Rare changes → ephemeral
            (SegmentType::RagContext, _) =>
                (CacheType::Ephemeral, "RAG context changes per query — ephemeral (5-min) cache avoids redundant retrieval".to_string()),
            (SegmentType::FileContent, ChangeFreq::Rare) =>
                (CacheType::Ephemeral, "File content changes rarely within a session — ephemeral cache saves re-reads".to_string()),

            // Conversation history → ephemeral (grows per turn)
            (SegmentType::ConversationHistory, ChangeFreq::Moderate) =>
                (CacheType::Ephemeral, "Conversation history grows each turn — cache up to the latest assistant turn".to_string()),

            // Per-turn changes → no cache
            (_, ChangeFreq::PerTurn) =>
                (CacheType::None, "Changes every turn — caching would never hit".to_string()),

            // Default for large static-ish content
            (_, ChangeFreq::Static) =>
                (CacheType::Persistent, "Static content — persistent cache recommended".to_string()),
            (_, ChangeFreq::Rare) =>
                (CacheType::Ephemeral, "Rarely-changing content — ephemeral cache recommended".to_string()),
            _ =>
                (CacheType::None, "Dynamic content — not cacheable".to_string()),
        };

        let savings = if cache_type != CacheType::None {
            // Savings per 1M requests
            let savings_per_1k = cache_savings_per_1k_tokens(&self.model);
            (seg.token_estimate as f64 / 1000.0) * savings_per_1k * 1_000_000.0
        } else {
            0.0
        };

        CacheRecommendation {
            segment_id: seg.id.clone(),
            cache_type,
            reason,
            tokens_saved: seg.token_estimate,
            estimated_savings_usd: savings,
        }
    }

    /// Generate a structured report string.
    pub fn report(&self, summary: &CacheAdvisorySummary) -> String {
        let mut out = String::new();
        out.push_str("# Cache Advisory Report\n\n");
        out.push_str(&format!("Model: {}\n", self.model));
        out.push_str(&format!("Total tokens: {}\n", summary.total_tokens));
        out.push_str(&format!("Cacheable tokens: {} ({:.0}%)\n",
            summary.cacheable_tokens,
            summary.cache_ratio() * 100.0
        ));
        out.push_str(&format!("Cache efficiency: {}\n", summary.cache_efficiency_label()));
        out.push_str(&format!("Estimated savings: ${:.4}/1M requests\n\n", summary.total_estimated_savings_usd));
        out.push_str("## Recommendations\n\n");
        for rec in &summary.recommendations {
            out.push_str(&format!(
                "- **{}**: `{}` — {}\n",
                rec.segment_id, rec.cache_type, rec.reason
            ));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Estimate token count from text (rough: 1 token ≈ 4 chars).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Build a segment from text content.
pub fn segment_from_text(id: &str, text: &str, kind: SegmentType, freq: ChangeFreq) -> PromptSegment {
    PromptSegment {
        id: id.to_string(),
        content_type: kind,
        token_estimate: estimate_tokens(text),
        change_frequency: freq,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn large_static_segment(id: &str, kind: SegmentType) -> PromptSegment {
        PromptSegment {
            id: id.to_string(),
            content_type: kind,
            token_estimate: 8192,
            change_frequency: ChangeFreq::Static,
        }
    }

    #[test]
    fn test_system_prompt_gets_persistent_cache() {
        let advisor = CacheAdvisor::default();
        let seg = large_static_segment("sys", SegmentType::SystemPrompt);
        let summary = advisor.analyze(&[seg]);
        let rec = &summary.recommendations[0];
        assert_eq!(rec.cache_type, CacheType::Persistent);
    }

    #[test]
    fn test_small_segment_no_cache() {
        let advisor = CacheAdvisor::default();
        let seg = PromptSegment {
            id: "tiny".into(),
            content_type: SegmentType::SystemPrompt,
            token_estimate: 100,
            change_frequency: ChangeFreq::Static,
        };
        let summary = advisor.analyze(&[seg]);
        assert_eq!(summary.recommendations[0].cache_type, CacheType::None);
    }

    #[test]
    fn test_per_turn_no_cache() {
        let advisor = CacheAdvisor::default();
        let seg = PromptSegment {
            id: "user_msg".into(),
            content_type: SegmentType::UserTurn,
            token_estimate: 5000,
            change_frequency: ChangeFreq::PerTurn,
        };
        let summary = advisor.analyze(&[seg]);
        assert_eq!(summary.recommendations[0].cache_type, CacheType::None);
    }

    #[test]
    fn test_rag_context_ephemeral() {
        let advisor = CacheAdvisor::default();
        let seg = PromptSegment {
            id: "rag".into(),
            content_type: SegmentType::RagContext,
            token_estimate: 4096,
            change_frequency: ChangeFreq::Rare,
        };
        let summary = advisor.analyze(&[seg]);
        assert_eq!(summary.recommendations[0].cache_type, CacheType::Ephemeral);
    }

    #[test]
    fn test_tool_definitions_persistent() {
        let advisor = CacheAdvisor::default();
        let seg = large_static_segment("tools", SegmentType::ToolDefinitions);
        let summary = advisor.analyze(&[seg]);
        assert_eq!(summary.recommendations[0].cache_type, CacheType::Persistent);
    }

    #[test]
    fn test_cache_ratio_calculation() {
        let advisor = CacheAdvisor::default();
        let segments = vec![
            large_static_segment("sys", SegmentType::SystemPrompt),
            PromptSegment {
                id: "user".into(),
                content_type: SegmentType::UserTurn,
                token_estimate: 8192,
                change_frequency: ChangeFreq::PerTurn,
            },
        ];
        let summary = advisor.analyze(&segments);
        assert!((summary.cache_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_efficiency_label() {
        let mut s = CacheAdvisorySummary::default();
        s.total_tokens = 100;
        s.cacheable_tokens = 80;
        assert_eq!(s.cache_efficiency_label(), "excellent");
        s.cacheable_tokens = 45;
        assert_eq!(s.cache_efficiency_label(), "good");
        s.cacheable_tokens = 25;
        assert_eq!(s.cache_efficiency_label(), "moderate");
        s.cacheable_tokens = 10;
        assert_eq!(s.cache_efficiency_label(), "poor");
    }

    #[test]
    fn test_savings_calculation_positive() {
        let advisor = CacheAdvisor::new("claude-sonnet-4-6");
        let seg = large_static_segment("sys", SegmentType::SystemPrompt);
        let summary = advisor.analyze(&[seg]);
        assert!(summary.total_estimated_savings_usd > 0.0);
    }

    #[test]
    fn test_report_output_contains_keys() {
        let advisor = CacheAdvisor::default();
        let seg = large_static_segment("sys", SegmentType::SystemPrompt);
        let summary = advisor.analyze(&[seg]);
        let report = advisor.report(&summary);
        assert!(report.contains("Cache Advisory Report"));
        assert!(report.contains("Recommendations"));
        assert!(report.contains("persistent"));
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "a".repeat(4000);
        assert_eq!(estimate_tokens(&text), 1000);
    }

    #[test]
    fn test_segment_from_text_helper() {
        let seg = segment_from_text("s1", &"x".repeat(8000), SegmentType::SystemPrompt, ChangeFreq::Static);
        assert_eq!(seg.token_estimate, 2000);
        assert_eq!(seg.content_type, SegmentType::SystemPrompt);
    }

    #[test]
    fn test_conversation_history_ephemeral() {
        let advisor = CacheAdvisor::default();
        let seg = PromptSegment {
            id: "history".into(),
            content_type: SegmentType::ConversationHistory,
            token_estimate: 16000,
            change_frequency: ChangeFreq::Moderate,
        };
        let summary = advisor.analyze(&[seg]);
        assert_eq!(summary.recommendations[0].cache_type, CacheType::Ephemeral);
    }

    #[test]
    fn test_empty_segments() {
        let advisor = CacheAdvisor::default();
        let summary = advisor.analyze(&[]);
        assert_eq!(summary.total_tokens, 0);
        assert_eq!(summary.cache_ratio(), 0.0);
    }
}
