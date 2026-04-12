#![allow(dead_code)]
//! Streaming agent reasoning extraction and categorization.
//!
//! Parses `<thinking>...</thinking>` blocks and labelled lines from LLM
//! streaming output, classifies each fragment into a `ThoughtCategory`, assigns
//! a confidence score, and stores them in a `ThoughtSession` for later
//! filtering, summarisation, and markdown export.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// The semantic category of a reasoning fragment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThoughtCategory {
    Planning,
    Reasoning,
    Uncertainty,
    Decision,
    Observation,
}

impl std::fmt::Display for ThoughtCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Planning => write!(f, "Planning"),
            Self::Reasoning => write!(f, "Reasoning"),
            Self::Uncertainty => write!(f, "Uncertainty"),
            Self::Decision => write!(f, "Decision"),
            Self::Observation => write!(f, "Observation"),
        }
    }
}

/// Derived confidence band from a 0-100 score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl ConfidenceLevel {
    pub fn from_score(score: u8) -> Self {
        if score >= 80 {
            Self::High
        } else if score >= 50 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// A single reasoning fragment extracted from agent output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtUnit {
    pub thought_id: String,
    pub category: ThoughtCategory,
    pub content: String,
    pub confidence: u8,
    pub confidence_level: ConfidenceLevel,
    pub timestamp_ms: u64,
    pub agent_id: String,
}

impl ThoughtUnit {
    fn new(
        thought_id: impl Into<String>,
        category: ThoughtCategory,
        content: impl Into<String>,
        confidence: u8,
        agent_id: impl Into<String>,
    ) -> Self {
        let confidence_level = ConfidenceLevel::from_score(confidence);
        Self {
            thought_id: thought_id.into(),
            category,
            content: content.into(),
            confidence,
            confidence_level,
            timestamp_ms: 0,
            agent_id: agent_id.into(),
        }
    }
}

/// Summary statistics for a `ThoughtSession`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSummary {
    pub total_units: usize,
    pub planning_count: usize,
    pub reasoning_count: usize,
    pub uncertainty_count: usize,
    pub decision_count: usize,
    pub observation_count: usize,
    pub avg_confidence: f32,
    pub dominant_category: ThoughtCategory,
}

/// Await-state notification — integrates with agent_await.rs concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitStateNotification {
    pub agent_id: String,
    pub condition_id: String,
    pub reason: String,
    pub is_waiting: bool,
    pub timestamp_ms: u64,
}

// ─── ThoughtExtractor ────────────────────────────────────────────────────────

/// Extracts and classifies thought fragments from raw streaming text.
pub struct ThoughtExtractor {
    /// Partial content accumulated while inside an open `<thinking>` tag.
    buffer: String,
    /// Whether we are currently inside a `<thinking>` block.
    in_thinking_block: bool,
    next_id: u64,
    agent_id: String,
}

impl ThoughtExtractor {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            in_thinking_block: false,
            next_id: 1,
            agent_id: String::new(),
        }
    }

    /// Set the agent_id to embed in produced `ThoughtUnit`s.
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = agent_id.into();
        self
    }

    // ── classification helpers ────────────────────────────────────────────

    fn classify(content: &str) -> (ThoughtCategory, u8) {
        let lower = content.to_lowercase();
        let trimmed = content.trim();

        // Uncertainty check first (keyword-driven, not position-driven)
        if lower.contains("unsure")
            || lower.contains("might")
            || lower.contains("probably")
            || lower.contains("not sure")
        {
            return (ThoughtCategory::Uncertainty, 30);
        }

        // Decision
        if trimmed.starts_with("I decide")
            || trimmed.starts_with("Therefore")
            || trimmed.starts_with("So ")
            || trimmed.starts_with("Thus ")
        {
            return (ThoughtCategory::Decision, 90);
        }

        // Planning
        if trimmed.starts_with("Plan")
            || trimmed.starts_with("Step")
            || trimmed.starts_with("First")
            || trimmed.starts_with("Next")
        {
            return (ThoughtCategory::Planning, 70);
        }

        // Observation
        if trimmed.starts_with("I observe")
            || trimmed.starts_with("I see")
            || trimmed.starts_with("I notice")
        {
            return (ThoughtCategory::Observation, 75);
        }

        (ThoughtCategory::Reasoning, 60)
    }

    fn make_id(&mut self) -> String {
        let id = format!("thought-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn make_unit(&mut self, content: &str) -> ThoughtUnit {
        let (cat, conf) = Self::classify(content);
        let id = self.make_id();
        ThoughtUnit::new(id, cat, content.trim(), conf, &self.agent_id)
    }

    fn make_unit_with_category(
        &mut self,
        content: &str,
        cat: ThoughtCategory,
        conf: u8,
    ) -> ThoughtUnit {
        let id = self.make_id();
        ThoughtUnit::new(id, cat, content.trim(), conf, &self.agent_id)
    }

    // ── public API ───────────────────────────────────────────────────────

    /// Extract `ThoughtUnit`s from a streaming chunk.
    ///
    /// Handles complete `<thinking>…</thinking>` blocks within the chunk,
    /// partial open tags (delegated to the internal buffer), and labelled
    /// lines (`Planning:`, `Decision:`, `Observation:`).
    pub fn extract_from_chunk(&mut self, chunk: &str) -> Vec<ThoughtUnit> {
        let mut units: Vec<ThoughtUnit> = Vec::new();

        // We process the chunk character-by-character in a simple state machine
        // so that tags spanning previous chunks are handled correctly.
        let combined = format!("{}{}", self.buffer, chunk);
        self.buffer.clear();
        // in_thinking_block carries over: if buffer was mid-tag, we resume inside the block

        let mut rest: &str = combined.as_str();

        loop {
            if self.in_thinking_block {
                // Look for closing tag
                if let Some(end_pos) = rest.find("</thinking>") {
                    let content = &rest[..end_pos];
                    if !content.trim().is_empty() {
                        units.push(self.make_unit(content));
                    }
                    rest = &rest[end_pos + "</thinking>".len()..];
                    self.in_thinking_block = false;
                } else {
                    // No closing tag yet — buffer the remainder
                    self.buffer.push_str(rest);
                    self.in_thinking_block = true;
                    break;
                }
            } else {
                // Look for opening tag
                if let Some(open_pos) = rest.find("<thinking>") {
                    // Scan the text before the tag for labelled lines
                    let before = &rest[..open_pos];
                    for unit in self.extract_labelled_lines(before) {
                        units.push(unit);
                    }
                    rest = &rest[open_pos + "<thinking>".len()..];
                    self.in_thinking_block = true;
                } else {
                    // No more tags — scan remaining text for labelled lines
                    for unit in self.extract_labelled_lines(rest) {
                        units.push(unit);
                    }
                    break;
                }
            }
        }

        units
    }

    fn extract_labelled_lines(&mut self, text: &str) -> Vec<ThoughtUnit> {
        let mut units = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let result = if let Some(rest) = trimmed.strip_prefix("Planning:") {
                Some((ThoughtCategory::Planning, 70u8, rest.trim()))
            } else if let Some(rest) = trimmed.strip_prefix("Decision:") {
                Some((ThoughtCategory::Decision, 90u8, rest.trim()))
            } else if let Some(rest) = trimmed.strip_prefix("Observation:") {
                Some((ThoughtCategory::Observation, 75u8, rest.trim()))
            } else {
                None
            };
            if let Some((cat, conf, content)) = result {
                if !content.is_empty() {
                    units.push(self.make_unit_with_category(content, cat, conf));
                }
            }
        }
        units
    }

    /// Accumulate a raw chunk into the internal partial-tag buffer.
    pub fn buffer_chunk(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    /// Flush any buffered content as a `Reasoning` unit.
    pub fn flush_buffer(&mut self) -> Vec<ThoughtUnit> {
        if self.buffer.trim().is_empty() {
            self.buffer.clear();
            self.in_thinking_block = false;
            return Vec::new();
        }
        // Strip any dangling open tag
        let content = self
            .buffer
            .trim_start_matches("<thinking>")
            .trim()
            .to_string();
        self.buffer.clear();
        self.in_thinking_block = false;
        if content.is_empty() {
            return Vec::new();
        }
        let id = self.make_id();
        vec![ThoughtUnit::new(
            id,
            ThoughtCategory::Reasoning,
            content,
            60,
            &self.agent_id,
        )]
    }
}

impl Default for ThoughtExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── ThoughtSession ──────────────────────────────────────────────────────────

/// Accumulates `ThoughtUnit`s for a single agent session.
pub struct ThoughtSession {
    agent_id: String,
    units: Vec<ThoughtUnit>,
}

impl ThoughtSession {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            units: Vec::new(),
        }
    }

    pub fn add(&mut self, unit: ThoughtUnit) {
        self.units.push(unit);
    }

    pub fn units(&self) -> &[ThoughtUnit] {
        &self.units
    }

    pub fn filter_by_category(&self, cat: &ThoughtCategory) -> Vec<&ThoughtUnit> {
        self.units
            .iter()
            .filter(|u| &u.category == cat)
            .collect()
    }

    /// Returns Decision units that have High confidence.
    pub fn high_confidence_decisions(&self) -> Vec<&ThoughtUnit> {
        self.units
            .iter()
            .filter(|u| {
                u.category == ThoughtCategory::Decision
                    && u.confidence_level == ConfidenceLevel::High
            })
            .collect()
    }

    pub fn uncertainty_count(&self) -> usize {
        self.units
            .iter()
            .filter(|u| u.category == ThoughtCategory::Uncertainty)
            .count()
    }

    /// Render all units as Markdown.
    pub fn export_markdown(&self) -> String {
        let mut out = String::new();
        for unit in &self.units {
            out.push_str(&format!(
                "## {} ({}%)\n{}\n",
                unit.category, unit.confidence, unit.content
            ));
        }
        out
    }

    pub fn summary(&self) -> ThoughtSummary {
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut total_conf: u32 = 0;

        for u in &self.units {
            *counts.entry(u.category.to_string()).or_insert(0) += 1;
            total_conf += u.confidence as u32;
        }

        let n = self.units.len();
        let avg_confidence = if n == 0 {
            0.0
        } else {
            total_conf as f32 / n as f32
        };

        let dominant_category = counts
            .iter()
            .max_by_key(|(_, v)| *v)
            .map(|(k, _)| match k.as_str() {
                "Planning" => ThoughtCategory::Planning,
                "Uncertainty" => ThoughtCategory::Uncertainty,
                "Decision" => ThoughtCategory::Decision,
                "Observation" => ThoughtCategory::Observation,
                _ => ThoughtCategory::Reasoning,
            })
            .unwrap_or(ThoughtCategory::Reasoning);

        ThoughtSummary {
            total_units: n,
            planning_count: *counts.get("Planning").unwrap_or(&0),
            reasoning_count: *counts.get("Reasoning").unwrap_or(&0),
            uncertainty_count: *counts.get("Uncertainty").unwrap_or(&0),
            decision_count: *counts.get("Decision").unwrap_or(&0),
            observation_count: *counts.get("Observation").unwrap_or(&0),
            avg_confidence,
            dominant_category,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConfidenceLevel::from_score ──────────────────────────────────────

    #[test]
    fn test_confidence_high_boundary() {
        assert_eq!(ConfidenceLevel::from_score(80), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(100), ConfidenceLevel::High);
    }

    #[test]
    fn test_confidence_medium_boundary() {
        assert_eq!(ConfidenceLevel::from_score(50), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(79), ConfidenceLevel::Medium);
    }

    #[test]
    fn test_confidence_low_boundary() {
        assert_eq!(ConfidenceLevel::from_score(0), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(49), ConfidenceLevel::Low);
    }

    // ── category classification ──────────────────────────────────────────

    #[test]
    fn test_classify_uncertainty_unsure() {
        let (cat, conf) = ThoughtExtractor::classify("I am unsure about this approach");
        assert_eq!(cat, ThoughtCategory::Uncertainty);
        assert_eq!(conf, 30);
    }

    #[test]
    fn test_classify_uncertainty_might() {
        let (cat, _) = ThoughtExtractor::classify("This might work");
        assert_eq!(cat, ThoughtCategory::Uncertainty);
    }

    #[test]
    fn test_classify_uncertainty_probably() {
        let (cat, _) = ThoughtExtractor::classify("It is probably fine");
        assert_eq!(cat, ThoughtCategory::Uncertainty);
    }

    #[test]
    fn test_classify_uncertainty_not_sure() {
        let (cat, _) = ThoughtExtractor::classify("I am not sure what to do");
        assert_eq!(cat, ThoughtCategory::Uncertainty);
    }

    #[test]
    fn test_classify_decision_therefore() {
        let (cat, conf) = ThoughtExtractor::classify("Therefore we should use option A");
        assert_eq!(cat, ThoughtCategory::Decision);
        assert_eq!(conf, 90);
    }

    #[test]
    fn test_classify_decision_i_decide() {
        let (cat, _) = ThoughtExtractor::classify("I decide to refactor this function");
        assert_eq!(cat, ThoughtCategory::Decision);
    }

    #[test]
    fn test_classify_decision_so() {
        let (cat, _) = ThoughtExtractor::classify("So we proceed with plan B");
        assert_eq!(cat, ThoughtCategory::Decision);
    }

    #[test]
    fn test_classify_decision_thus() {
        let (cat, _) = ThoughtExtractor::classify("Thus the answer is 42");
        assert_eq!(cat, ThoughtCategory::Decision);
    }

    #[test]
    fn test_classify_planning_plan() {
        let (cat, conf) = ThoughtExtractor::classify("Plan: first fetch data, then process");
        assert_eq!(cat, ThoughtCategory::Planning);
        assert_eq!(conf, 70);
    }

    #[test]
    fn test_classify_planning_step() {
        let (cat, _) = ThoughtExtractor::classify("Step 1: initialise the connection");
        assert_eq!(cat, ThoughtCategory::Planning);
    }

    #[test]
    fn test_classify_planning_first() {
        let (cat, _) = ThoughtExtractor::classify("First, load the config file");
        assert_eq!(cat, ThoughtCategory::Planning);
    }

    #[test]
    fn test_classify_planning_next() {
        let (cat, _) = ThoughtExtractor::classify("Next, validate the schema");
        assert_eq!(cat, ThoughtCategory::Planning);
    }

    #[test]
    fn test_classify_observation_i_see() {
        let (cat, conf) = ThoughtExtractor::classify("I see a pattern here");
        assert_eq!(cat, ThoughtCategory::Observation);
        assert_eq!(conf, 75);
    }

    #[test]
    fn test_classify_observation_i_observe() {
        let (cat, _) = ThoughtExtractor::classify("I observe that the test fails");
        assert_eq!(cat, ThoughtCategory::Observation);
    }

    #[test]
    fn test_classify_observation_i_notice() {
        let (cat, _) = ThoughtExtractor::classify("I notice an off-by-one error");
        assert_eq!(cat, ThoughtCategory::Observation);
    }

    #[test]
    fn test_classify_reasoning_fallback() {
        let (cat, conf) = ThoughtExtractor::classify("The algorithm runs in O(n log n)");
        assert_eq!(cat, ThoughtCategory::Reasoning);
        assert_eq!(conf, 60);
    }

    // ── extract_from_chunk ───────────────────────────────────────────────

    #[test]
    fn test_extract_complete_thinking_block() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("<thinking>Therefore we proceed</thinking>");
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].category, ThoughtCategory::Decision);
        assert_eq!(units[0].confidence, 90);
    }

    #[test]
    fn test_extract_multiple_thinking_blocks() {
        let mut ex = ThoughtExtractor::new();
        let chunk =
            "<thinking>First, let me plan</thinking> text <thinking>I notice a bug</thinking>";
        let units = ex.extract_from_chunk(chunk);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].category, ThoughtCategory::Planning);
        assert_eq!(units[1].category, ThoughtCategory::Observation);
    }

    #[test]
    fn test_extract_labelled_planning_line() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("Planning: fetch config then parse it");
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].category, ThoughtCategory::Planning);
    }

    #[test]
    fn test_extract_labelled_decision_line() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("Decision: use async/await");
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].category, ThoughtCategory::Decision);
    }

    #[test]
    fn test_extract_labelled_observation_line() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("Observation: the loop terminates early");
        assert_eq!(units.len(), 1);
        // The content after "Observation:" is "the loop terminates early"
        // which classifies as Reasoning (no special prefix or keywords)
        assert!(!units[0].content.is_empty());
    }

    #[test]
    fn test_extract_empty_chunk_returns_empty() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("");
        assert!(units.is_empty());
    }

    #[test]
    fn test_extract_no_tags_no_labels_returns_empty() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("some plain text without any markers");
        assert!(units.is_empty());
    }

    #[test]
    fn test_extract_thought_id_increments() {
        let mut ex = ThoughtExtractor::new();
        let u1 = ex.extract_from_chunk("<thinking>First thing</thinking>");
        let u2 = ex.extract_from_chunk("<thinking>Next thing</thinking>");
        assert_eq!(u1[0].thought_id, "thought-1");
        assert_eq!(u2[0].thought_id, "thought-2");
    }

    #[test]
    fn test_extract_confidence_level_set_correctly() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("<thinking>Therefore we go with A</thinking>");
        assert_eq!(units[0].confidence_level, ConfidenceLevel::High);
    }

    #[test]
    fn test_extract_uncertainty_confidence_low() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.extract_from_chunk("<thinking>I am unsure about this</thinking>");
        assert_eq!(units[0].confidence_level, ConfidenceLevel::Low);
    }

    // ── buffer / flush ───────────────────────────────────────────────────

    #[test]
    fn test_buffer_chunk_accumulates() {
        let mut ex = ThoughtExtractor::new();
        ex.buffer_chunk("partial content");
        ex.buffer_chunk(" more");
        let units = ex.flush_buffer();
        assert_eq!(units.len(), 1);
        assert!(units[0].content.contains("partial content more"));
        assert_eq!(units[0].category, ThoughtCategory::Reasoning);
    }

    #[test]
    fn test_flush_empty_buffer_returns_empty() {
        let mut ex = ThoughtExtractor::new();
        let units = ex.flush_buffer();
        assert!(units.is_empty());
    }

    #[test]
    fn test_flush_clears_buffer() {
        let mut ex = ThoughtExtractor::new();
        ex.buffer_chunk("something");
        ex.flush_buffer();
        let units = ex.flush_buffer();
        assert!(units.is_empty());
    }

    #[test]
    fn test_flush_strips_dangling_open_tag() {
        let mut ex = ThoughtExtractor::new();
        ex.buffer_chunk("<thinking>incomplete thought");
        let units = ex.flush_buffer();
        assert_eq!(units.len(), 1);
        assert!(!units[0].content.contains("<thinking>"));
    }

    #[test]
    fn test_partial_tag_across_chunks() {
        let mut ex = ThoughtExtractor::new();
        // First chunk opens but doesn't close the tag
        let u1 = ex.extract_from_chunk("<thinking>partial start");
        assert!(u1.is_empty()); // buffered
        // Second chunk closes the tag
        let u2 = ex.extract_from_chunk(" and end</thinking>");
        assert_eq!(u2.len(), 1);
        assert!(u2[0].content.contains("partial start"));
    }

    #[test]
    fn test_agent_id_embedded() {
        let mut ex = ThoughtExtractor::new().with_agent_id("agent-42");
        let units = ex.extract_from_chunk("<thinking>I notice something</thinking>");
        assert_eq!(units[0].agent_id, "agent-42");
    }

    // ── ThoughtSession ───────────────────────────────────────────────────

    fn make_unit_with(cat: ThoughtCategory, conf: u8) -> ThoughtUnit {
        ThoughtUnit::new(
            format!("t-{}", conf),
            cat,
            "some content",
            conf,
            "agent-1",
        )
    }

    #[test]
    fn test_session_add_and_units() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Planning, 70));
        assert_eq!(session.units().len(), 1);
    }

    #[test]
    fn test_session_filter_by_category() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Planning, 70));
        session.add(make_unit_with(ThoughtCategory::Decision, 90));
        session.add(make_unit_with(ThoughtCategory::Planning, 65));
        let plans = session.filter_by_category(&ThoughtCategory::Planning);
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_session_high_confidence_decisions() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Decision, 90)); // High
        session.add(make_unit_with(ThoughtCategory::Decision, 40)); // Low
        session.add(make_unit_with(ThoughtCategory::Planning, 90)); // High but not Decision
        let hcd = session.high_confidence_decisions();
        assert_eq!(hcd.len(), 1);
        assert_eq!(hcd[0].confidence, 90);
    }

    #[test]
    fn test_session_uncertainty_count() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Uncertainty, 30));
        session.add(make_unit_with(ThoughtCategory::Uncertainty, 25));
        session.add(make_unit_with(ThoughtCategory::Reasoning, 60));
        assert_eq!(session.uncertainty_count(), 2);
    }

    #[test]
    fn test_export_markdown_format() {
        let mut session = ThoughtSession::new("agent-1");
        let mut unit = make_unit_with(ThoughtCategory::Planning, 70);
        unit.content = "my plan".to_string();
        session.add(unit);
        let md = session.export_markdown();
        assert!(md.contains("## Planning (70%)"));
        assert!(md.contains("my plan"));
    }

    #[test]
    fn test_export_markdown_multiple_units() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Reasoning, 60));
        session.add(make_unit_with(ThoughtCategory::Decision, 90));
        let md = session.export_markdown();
        assert!(md.contains("## Reasoning"));
        assert!(md.contains("## Decision"));
    }

    #[test]
    fn test_summary_counts() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Planning, 70));
        session.add(make_unit_with(ThoughtCategory::Planning, 65));
        session.add(make_unit_with(ThoughtCategory::Uncertainty, 30));
        session.add(make_unit_with(ThoughtCategory::Decision, 90));
        let s = session.summary();
        assert_eq!(s.total_units, 4);
        assert_eq!(s.planning_count, 2);
        assert_eq!(s.uncertainty_count, 1);
        assert_eq!(s.decision_count, 1);
    }

    #[test]
    fn test_summary_avg_confidence() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Planning, 80));
        session.add(make_unit_with(ThoughtCategory::Reasoning, 60));
        let s = session.summary();
        assert!((s.avg_confidence - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_summary_dominant_category() {
        let mut session = ThoughtSession::new("agent-1");
        session.add(make_unit_with(ThoughtCategory::Planning, 70));
        session.add(make_unit_with(ThoughtCategory::Planning, 65));
        session.add(make_unit_with(ThoughtCategory::Decision, 90));
        let s = session.summary();
        assert_eq!(s.dominant_category, ThoughtCategory::Planning);
    }

    #[test]
    fn test_summary_empty_session() {
        let session = ThoughtSession::new("agent-1");
        let s = session.summary();
        assert_eq!(s.total_units, 0);
        assert_eq!(s.avg_confidence, 0.0);
    }

    #[test]
    fn test_await_state_notification_fields() {
        let n = AwaitStateNotification {
            agent_id: "a1".to_string(),
            condition_id: "cond-1".to_string(),
            reason: "waiting for lock".to_string(),
            is_waiting: true,
            timestamp_ms: 12345,
        };
        assert!(n.is_waiting);
        assert_eq!(n.agent_id, "a1");
    }

    #[test]
    fn test_filter_by_category_empty_returns_empty() {
        let session = ThoughtSession::new("a");
        let results = session.filter_by_category(&ThoughtCategory::Decision);
        assert!(results.is_empty());
    }

    #[test]
    fn test_high_confidence_decision_threshold() {
        let mut session = ThoughtSession::new("a");
        session.add(make_unit_with(ThoughtCategory::Decision, 80)); // exactly High boundary
        let hcd = session.high_confidence_decisions();
        assert_eq!(hcd.len(), 1);
    }

    #[test]
    fn test_medium_confidence_decision_not_in_high() {
        let mut session = ThoughtSession::new("a");
        session.add(make_unit_with(ThoughtCategory::Decision, 79)); // Medium
        let hcd = session.high_confidence_decisions();
        assert!(hcd.is_empty());
    }

    #[test]
    fn test_export_markdown_empty_session() {
        let session = ThoughtSession::new("a");
        assert_eq!(session.export_markdown(), "");
    }

    #[test]
    fn test_thought_unit_confidence_level_derived() {
        let unit = ThoughtUnit::new("t1", ThoughtCategory::Decision, "content", 85, "a");
        assert_eq!(unit.confidence_level, ConfidenceLevel::High);
    }
}
