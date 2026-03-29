#![allow(dead_code)]
//! Explainable agent — structured explanations and audit trails for every AI code change.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeReason {
    UserRequest,
    BugFix,
    Refactoring,
    Performance,
    Security,
    Convention,
    Dependency,
    TestCoverage,
    Documentation,
    AutoDetected,
}

impl ChangeReason {
    pub fn as_str(&self) -> &str {
        match self {
            ChangeReason::UserRequest => "UserRequest",
            ChangeReason::BugFix => "BugFix",
            ChangeReason::Refactoring => "Refactoring",
            ChangeReason::Performance => "Performance",
            ChangeReason::Security => "Security",
            ChangeReason::Convention => "Convention",
            ChangeReason::Dependency => "Dependency",
            ChangeReason::TestCoverage => "TestCoverage",
            ChangeReason::Documentation => "Documentation",
            ChangeReason::AutoDetected => "AutoDetected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfidenceLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplanationFormat {
    Markdown,
    Json,
    Compact,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Alternative {
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub rejected_reason: String,
}

#[derive(Debug, Clone)]
pub struct ContextUsed {
    pub source: String,
    pub description: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone)]
pub struct CodeChange {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub old_code: String,
    pub new_code: String,
    pub change_type: String,
}

#[derive(Debug, Clone)]
pub struct ExplanationChain {
    pub id: String,
    pub change: CodeChange,
    pub intent: String,
    pub reason: ChangeReason,
    pub confidence: ConfidenceLevel,
    pub context_used: Vec<ContextUsed>,
    pub alternatives: Vec<Alternative>,
    pub reasoning_steps: Vec<String>,
    pub trade_offs: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub session_id: String,
    pub chain: ExplanationChain,
    pub user_feedback: Option<String>,
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AuditTrail {
    pub entries: Vec<AuditEntry>,
    pub session_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct ExplainConfig {
    pub track_alternatives: bool,
    pub max_alternatives: usize,
    pub track_context: bool,
    pub auto_explain: bool,
    pub min_confidence_to_explain: ConfidenceLevel,
}

impl Default for ExplainConfig {
    fn default() -> Self {
        Self {
            track_alternatives: true,
            max_alternatives: 3,
            track_context: true,
            auto_explain: true,
            min_confidence_to_explain: ConfidenceLevel::Low,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExplainMetrics {
    pub total_explanations: u64,
    pub total_alternatives: u64,
    pub avg_confidence: f64,
    pub acceptance_rate: f64,
    pub most_common_reason: String,
    pub by_reason: HashMap<String, u64>,
}

impl Default for ExplainMetrics {
    fn default() -> Self {
        Self {
            total_explanations: 0,
            total_alternatives: 0,
            avg_confidence: 0.0,
            acceptance_rate: 0.0,
            most_common_reason: String::new(),
            by_reason: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// ConfidenceEstimator
// ---------------------------------------------------------------------------

pub struct ConfidenceEstimator;

impl ConfidenceEstimator {
    pub fn estimate(change: &CodeChange, context_count: usize, has_tests: bool) -> ConfidenceLevel {
        let lines_changed = if change.line_end >= change.line_start {
            change.line_end - change.line_start + 1
        } else {
            1
        };

        let mut score: f64 = 0.5;

        // More context raises confidence
        score += (context_count as f64) * 0.1;

        // Having tests raises confidence
        if has_tests {
            score += 0.2;
        }

        // Large changes lower confidence
        if lines_changed > 50 {
            score -= 0.3;
        } else if lines_changed > 20 {
            score -= 0.15;
        } else if lines_changed <= 5 {
            score += 0.1;
        }

        // Empty new code (deletions) are slightly less confident
        if change.new_code.is_empty() && !change.old_code.is_empty() {
            score -= 0.05;
        }

        if score >= 0.9 {
            ConfidenceLevel::VeryHigh
        } else if score >= 0.7 {
            ConfidenceLevel::High
        } else if score >= 0.5 {
            ConfidenceLevel::Medium
        } else if score >= 0.3 {
            ConfidenceLevel::Low
        } else {
            ConfidenceLevel::Uncertain
        }
    }

    pub fn to_score(level: &ConfidenceLevel) -> f64 {
        match level {
            ConfidenceLevel::VeryHigh => 0.95,
            ConfidenceLevel::High => 0.8,
            ConfidenceLevel::Medium => 0.6,
            ConfidenceLevel::Low => 0.4,
            ConfidenceLevel::Uncertain => 0.2,
        }
    }
}

// ---------------------------------------------------------------------------
// ExplanationEngine
// ---------------------------------------------------------------------------

pub struct ExplanationEngine {
    pub config: ExplainConfig,
    pub trail: AuditTrail,
    pub metrics: ExplainMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl ExplanationEngine {
    pub fn new(config: ExplainConfig) -> Self {
        let session_id = format!("session-{}", 1);
        Self {
            config,
            trail: AuditTrail {
                entries: Vec::new(),
                session_id: session_id.clone(),
                created_at: 0,
            },
            metrics: ExplainMetrics::default(),
            next_id: 1,
            timestamp_counter: 0,
        }
    }

    pub fn explain(
        &mut self,
        change: CodeChange,
        intent: &str,
        reason: ChangeReason,
    ) -> ExplanationChain {
        let id = format!("explain-{}", self.next_id);
        self.next_id += 1;
        self.timestamp_counter += 1;

        let context_count = 0;
        let has_tests = change.file_path.contains("test")
            || change.change_type.contains("test");
        let confidence = ConfidenceEstimator::estimate(&change, context_count, has_tests);

        ExplanationChain {
            id,
            change,
            intent: intent.to_string(),
            reason,
            confidence,
            context_used: Vec::new(),
            alternatives: Vec::new(),
            reasoning_steps: Vec::new(),
            trade_offs: Vec::new(),
            created_at: self.timestamp_counter,
        }
    }

    pub fn add_context(
        &mut self,
        chain_id: &str,
        source: &str,
        desc: &str,
        relevance: f64,
    ) -> Result<(), String> {
        if !self.config.track_context {
            return Ok(());
        }
        let entry = self
            .trail
            .entries
            .iter_mut()
            .find(|e| e.chain.id == chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        entry.chain.context_used.push(ContextUsed {
            source: source.to_string(),
            description: desc.to_string(),
            relevance_score: relevance,
        });
        Ok(())
    }

    pub fn add_alternative(
        &mut self,
        chain_id: &str,
        alt: Alternative,
    ) -> Result<(), String> {
        let max = self.config.max_alternatives;
        let entry = self
            .trail
            .entries
            .iter_mut()
            .find(|e| e.chain.id == chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        if entry.chain.alternatives.len() >= max {
            return Err(format!(
                "Max alternatives ({}) reached for {}",
                max, chain_id
            ));
        }
        entry.chain.alternatives.push(alt);
        self.metrics.total_alternatives += 1;
        Ok(())
    }

    pub fn add_reasoning_step(
        &mut self,
        chain_id: &str,
        step: &str,
    ) -> Result<(), String> {
        let entry = self
            .trail
            .entries
            .iter_mut()
            .find(|e| e.chain.id == chain_id)
            .ok_or_else(|| format!("Chain {} not found", chain_id))?;
        entry.chain.reasoning_steps.push(step.to_string());
        Ok(())
    }

    pub fn record(&mut self, chain: ExplanationChain) {
        let reason_key = chain.reason.as_str().to_string();
        let confidence_score = ConfidenceEstimator::to_score(&chain.confidence);
        let alt_count = chain.alternatives.len() as u64;

        let entry = AuditEntry {
            session_id: self.trail.session_id.clone(),
            chain,
            user_feedback: None,
            accepted: None,
        };
        self.trail.entries.push(entry);

        // Update metrics
        self.metrics.total_explanations += 1;
        self.metrics.total_alternatives += alt_count;
        *self.metrics.by_reason.entry(reason_key).or_insert(0) += 1;

        // Recompute avg confidence
        let total = self.metrics.total_explanations as f64;
        let prev_avg = self.metrics.avg_confidence;
        self.metrics.avg_confidence =
            prev_avg + (confidence_score - prev_avg) / total;

        // Recompute most common reason
        if let Some((reason, _)) = self
            .metrics
            .by_reason
            .iter()
            .max_by_key(|(_, count)| *count)
        {
            self.metrics.most_common_reason = reason.clone();
        }

        // Recompute acceptance rate
        self.recompute_acceptance_rate();
    }

    fn recompute_acceptance_rate(&mut self) {
        let decided: Vec<_> = self
            .trail
            .entries
            .iter()
            .filter(|e| e.accepted.is_some())
            .collect();
        if decided.is_empty() {
            self.metrics.acceptance_rate = 0.0;
        } else {
            let accepted = decided.iter().filter(|e| e.accepted == Some(true)).count();
            self.metrics.acceptance_rate = accepted as f64 / decided.len() as f64;
        }
    }

    pub fn get_explanation(&self, id: &str) -> Option<&AuditEntry> {
        self.trail.entries.iter().find(|e| e.chain.id == id)
    }

    pub fn query_by_file(&self, path: &str) -> Vec<&AuditEntry> {
        self.trail
            .entries
            .iter()
            .filter(|e| e.chain.change.file_path == path)
            .collect()
    }

    pub fn query_by_reason(&self, reason: &ChangeReason) -> Vec<&AuditEntry> {
        self.trail
            .entries
            .iter()
            .filter(|e| e.chain.reason == *reason)
            .collect()
    }

    pub fn query(&self, text: &str) -> Vec<&AuditEntry> {
        let lower = text.to_lowercase();
        self.trail
            .entries
            .iter()
            .filter(|e| {
                e.chain.intent.to_lowercase().contains(&lower)
                    || e.chain
                        .reasoning_steps
                        .iter()
                        .any(|s| s.to_lowercase().contains(&lower))
                    || e.chain.change.change_type.to_lowercase().contains(&lower)
                    || e.chain.change.file_path.to_lowercase().contains(&lower)
            })
            .collect()
    }

    pub fn accept(&mut self, id: &str) -> Result<(), String> {
        let entry = self
            .trail
            .entries
            .iter_mut()
            .find(|e| e.chain.id == id)
            .ok_or_else(|| format!("Entry {} not found", id))?;
        entry.accepted = Some(true);
        self.recompute_acceptance_rate();
        Ok(())
    }

    pub fn reject(&mut self, id: &str, feedback: &str) -> Result<(), String> {
        let entry = self
            .trail
            .entries
            .iter_mut()
            .find(|e| e.chain.id == id)
            .ok_or_else(|| format!("Entry {} not found", id))?;
        entry.accepted = Some(false);
        entry.user_feedback = Some(feedback.to_string());
        self.recompute_acceptance_rate();
        Ok(())
    }

    pub fn export(&self, format: ExplanationFormat) -> String {
        match format {
            ExplanationFormat::Markdown => self.export_markdown_all(),
            ExplanationFormat::Json => self.export_json_all(),
            ExplanationFormat::Compact => self.export_compact_all(),
        }
    }

    pub fn export_entry(
        &self,
        id: &str,
        format: ExplanationFormat,
    ) -> Result<String, String> {
        let entry = self
            .trail
            .entries
            .iter()
            .find(|e| e.chain.id == id)
            .ok_or_else(|| format!("Entry {} not found", id))?;
        Ok(match format {
            ExplanationFormat::Markdown => Self::format_entry_markdown(entry),
            ExplanationFormat::Json => Self::format_entry_json(entry),
            ExplanationFormat::Compact => Self::format_entry_compact(entry),
        })
    }

    pub fn get_trail(&self) -> &AuditTrail {
        &self.trail
    }

    pub fn get_metrics(&self) -> &ExplainMetrics {
        &self.metrics
    }

    pub fn clear(&mut self) {
        self.trail.entries.clear();
        self.metrics = ExplainMetrics::default();
        self.next_id = 1;
        self.timestamp_counter = 0;
    }

    // -- Private export helpers --

    fn export_markdown_all(&self) -> String {
        let mut out = String::from("# Audit Trail\n\n");
        out.push_str(&format!("Session: {}\n\n", self.trail.session_id));
        for entry in &self.trail.entries {
            out.push_str(&Self::format_entry_markdown(entry));
            out.push_str("\n---\n\n");
        }
        out
    }

    fn format_entry_markdown(entry: &AuditEntry) -> String {
        let c = &entry.chain;
        let mut out = format!("## {}\n\n", c.id);
        out.push_str(&format!("**Intent:** {}\n\n", c.intent));
        out.push_str(&format!("**Reason:** {:?}\n\n", c.reason));
        out.push_str(&format!("**Confidence:** {:?}\n\n", c.confidence));
        out.push_str(&format!(
            "**File:** {} (lines {}-{})\n\n",
            c.change.file_path, c.change.line_start, c.change.line_end
        ));
        if !c.reasoning_steps.is_empty() {
            out.push_str("**Reasoning:**\n");
            for (i, step) in c.reasoning_steps.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, step));
            }
            out.push('\n');
        }
        if !c.alternatives.is_empty() {
            out.push_str("**Alternatives considered:**\n");
            for alt in &c.alternatives {
                out.push_str(&format!("- {} (rejected: {})\n", alt.description, alt.rejected_reason));
            }
            out.push('\n');
        }
        if !c.trade_offs.is_empty() {
            out.push_str("**Trade-offs:**\n");
            for t in &c.trade_offs {
                out.push_str(&format!("- {}\n", t));
            }
            out.push('\n');
        }
        if let Some(accepted) = entry.accepted {
            out.push_str(&format!(
                "**Status:** {}\n",
                if accepted { "Accepted" } else { "Rejected" }
            ));
        }
        if let Some(fb) = &entry.user_feedback {
            out.push_str(&format!("**Feedback:** {}\n", fb));
        }
        out
    }

    fn export_json_all(&self) -> String {
        let mut out = String::from("[\n");
        for (i, entry) in self.trail.entries.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
            }
            out.push_str(&Self::format_entry_json(entry));
        }
        out.push_str("\n]");
        out
    }

    fn format_entry_json(entry: &AuditEntry) -> String {
        let c = &entry.chain;
        let alts: Vec<String> = c
            .alternatives
            .iter()
            .map(|a| {
                format!(
                    "{{\"description\":\"{}\",\"rejected_reason\":\"{}\"}}",
                    a.description, a.rejected_reason
                )
            })
            .collect();
        format!(
            "{{\"id\":\"{}\",\"intent\":\"{}\",\"reason\":\"{:?}\",\"confidence\":\"{:?}\",\"file\":\"{}\",\"lines\":[{},{}],\"alternatives\":[{}],\"accepted\":{}}}",
            c.id,
            c.intent,
            c.reason,
            c.confidence,
            c.change.file_path,
            c.change.line_start,
            c.change.line_end,
            alts.join(","),
            match entry.accepted {
                Some(true) => "true",
                Some(false) => "false",
                None => "null",
            }
        )
    }

    fn export_compact_all(&self) -> String {
        self.trail
            .entries
            .iter()
            .map(Self::format_entry_compact)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_entry_compact(entry: &AuditEntry) -> String {
        let c = &entry.chain;
        let status = match entry.accepted {
            Some(true) => "accepted",
            Some(false) => "rejected",
            None => "pending",
        };
        format!(
            "[{}] {:?} | {} | {} (L{}-L{}) | {:?} | {}",
            c.id,
            c.reason,
            c.intent,
            c.change.file_path,
            c.change.line_start,
            c.change.line_end,
            c.confidence,
            status,
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_change() -> CodeChange {
        CodeChange {
            file_path: "src/main.rs".to_string(),
            line_start: 10,
            line_end: 15,
            old_code: "let x = 1;".to_string(),
            new_code: "let x = 2;".to_string(),
            change_type: "modification".to_string(),
        }
    }

    fn sample_alternative() -> Alternative {
        Alternative {
            description: "Use a constant".to_string(),
            pros: vec!["Immutable".to_string()],
            cons: vec!["Less flexible".to_string()],
            rejected_reason: "Needs runtime value".to_string(),
        }
    }

    fn engine() -> ExplanationEngine {
        ExplanationEngine::new(ExplainConfig::default())
    }

    // -- ChangeReason --

    #[test]
    fn test_change_reason_as_str() {
        assert_eq!(ChangeReason::UserRequest.as_str(), "UserRequest");
        assert_eq!(ChangeReason::BugFix.as_str(), "BugFix");
        assert_eq!(ChangeReason::Refactoring.as_str(), "Refactoring");
        assert_eq!(ChangeReason::Performance.as_str(), "Performance");
        assert_eq!(ChangeReason::Security.as_str(), "Security");
        assert_eq!(ChangeReason::Convention.as_str(), "Convention");
        assert_eq!(ChangeReason::Dependency.as_str(), "Dependency");
        assert_eq!(ChangeReason::TestCoverage.as_str(), "TestCoverage");
        assert_eq!(ChangeReason::Documentation.as_str(), "Documentation");
        assert_eq!(ChangeReason::AutoDetected.as_str(), "AutoDetected");
    }

    // -- ConfidenceEstimator --

    #[test]
    fn test_confidence_to_score() {
        assert!((ConfidenceEstimator::to_score(&ConfidenceLevel::VeryHigh) - 0.95).abs() < f64::EPSILON);
        assert!((ConfidenceEstimator::to_score(&ConfidenceLevel::High) - 0.8).abs() < f64::EPSILON);
        assert!((ConfidenceEstimator::to_score(&ConfidenceLevel::Medium) - 0.6).abs() < f64::EPSILON);
        assert!((ConfidenceEstimator::to_score(&ConfidenceLevel::Low) - 0.4).abs() < f64::EPSILON);
        assert!((ConfidenceEstimator::to_score(&ConfidenceLevel::Uncertain) - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_small_change_with_tests_and_context() {
        let change = CodeChange {
            file_path: "src/test_foo.rs".to_string(),
            line_start: 1,
            line_end: 3,
            old_code: "a".to_string(),
            new_code: "b".to_string(),
            change_type: "test".to_string(),
        };
        let level = ConfidenceEstimator::estimate(&change, 3, true);
        // 0.5 + 0.3 (context) + 0.2 (tests) + 0.1 (small) = 1.1 => VeryHigh
        assert_eq!(level, ConfidenceLevel::VeryHigh);
    }

    #[test]
    fn test_estimate_large_change_no_context() {
        let change = CodeChange {
            file_path: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 100,
            old_code: "big".to_string(),
            new_code: "bigger".to_string(),
            change_type: "refactor".to_string(),
        };
        let level = ConfidenceEstimator::estimate(&change, 0, false);
        // 0.5 + 0 + 0 - 0.3 = 0.2 => Uncertain
        assert_eq!(level, ConfidenceLevel::Uncertain);
    }

    #[test]
    fn test_estimate_medium_change() {
        let change = CodeChange {
            file_path: "src/foo.rs".to_string(),
            line_start: 1,
            line_end: 30,
            old_code: "a".to_string(),
            new_code: "b".to_string(),
            change_type: "modification".to_string(),
        };
        let level = ConfidenceEstimator::estimate(&change, 1, false);
        // 0.5 + 0.1 - 0.15 = 0.45 => Low
        assert_eq!(level, ConfidenceLevel::Low);
    }

    #[test]
    fn test_estimate_deletion_lowers_confidence() {
        let change = CodeChange {
            file_path: "src/foo.rs".to_string(),
            line_start: 1,
            line_end: 3,
            old_code: "code here".to_string(),
            new_code: "".to_string(),
            change_type: "delete".to_string(),
        };
        let level = ConfidenceEstimator::estimate(&change, 1, false);
        // 0.5 + 0.1 + 0.1 (small) - 0.05 (deletion) = 0.65 => Medium
        assert_eq!(level, ConfidenceLevel::Medium);
    }

    // -- ExplainConfig default --

    #[test]
    fn test_default_config() {
        let cfg = ExplainConfig::default();
        assert!(cfg.track_alternatives);
        assert_eq!(cfg.max_alternatives, 3);
        assert!(cfg.track_context);
        assert!(cfg.auto_explain);
        assert_eq!(cfg.min_confidence_to_explain, ConfidenceLevel::Low);
    }

    // -- ExplanationEngine::new --

    #[test]
    fn test_engine_new() {
        let e = engine();
        assert_eq!(e.trail.entries.len(), 0);
        assert_eq!(e.metrics.total_explanations, 0);
        assert!(e.trail.session_id.starts_with("session-"));
    }

    // -- explain --

    #[test]
    fn test_explain_produces_chain() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix off-by-one", ChangeReason::BugFix);
        assert_eq!(chain.id, "explain-1");
        assert_eq!(chain.intent, "fix off-by-one");
        assert_eq!(chain.reason, ChangeReason::BugFix);
        assert!(chain.context_used.is_empty());
        assert!(chain.alternatives.is_empty());
    }

    #[test]
    fn test_explain_increments_id() {
        let mut e = engine();
        let c1 = e.explain(sample_change(), "a", ChangeReason::BugFix);
        let c2 = e.explain(sample_change(), "b", ChangeReason::Refactoring);
        assert_eq!(c1.id, "explain-1");
        assert_eq!(c2.id, "explain-2");
    }

    #[test]
    fn test_explain_timestamps_increment() {
        let mut e = engine();
        let c1 = e.explain(sample_change(), "a", ChangeReason::BugFix);
        let c2 = e.explain(sample_change(), "b", ChangeReason::BugFix);
        assert!(c2.created_at > c1.created_at);
    }

    // -- record --

    #[test]
    fn test_record_adds_entry() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        e.record(chain);
        assert_eq!(e.trail.entries.len(), 1);
        assert_eq!(e.metrics.total_explanations, 1);
    }

    #[test]
    fn test_record_updates_by_reason() {
        let mut e = engine();
        let c1 = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let c2 = e.explain(sample_change(), "fix2", ChangeReason::BugFix);
        e.record(c1);
        e.record(c2);
        assert_eq!(*e.metrics.by_reason.get("BugFix").unwrap(), 2);
        assert_eq!(e.metrics.most_common_reason, "BugFix");
    }

    #[test]
    fn test_record_updates_avg_confidence() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        e.record(chain);
        assert!(e.metrics.avg_confidence > 0.0);
    }

    // -- get_explanation --

    #[test]
    fn test_get_explanation_found() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        assert!(e.get_explanation(&id).is_some());
    }

    #[test]
    fn test_get_explanation_not_found() {
        let e = engine();
        assert!(e.get_explanation("nonexistent").is_none());
    }

    // -- query_by_file --

    #[test]
    fn test_query_by_file() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        e.record(chain);
        let results = e.query_by_file("src/main.rs");
        assert_eq!(results.len(), 1);
        assert!(e.query_by_file("src/other.rs").is_empty());
    }

    // -- query_by_reason --

    #[test]
    fn test_query_by_reason() {
        let mut e = engine();
        let c1 = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let c2 = e.explain(sample_change(), "perf", ChangeReason::Performance);
        e.record(c1);
        e.record(c2);
        assert_eq!(e.query_by_reason(&ChangeReason::BugFix).len(), 1);
        assert_eq!(e.query_by_reason(&ChangeReason::Performance).len(), 1);
        assert_eq!(e.query_by_reason(&ChangeReason::Security).len(), 0);
    }

    // -- query (fuzzy) --

    #[test]
    fn test_query_fuzzy_intent() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix off-by-one error", ChangeReason::BugFix);
        e.record(chain);
        assert_eq!(e.query("off-by-one").len(), 1);
        assert_eq!(e.query("OFF-BY-ONE").len(), 1);
        assert!(e.query("unrelated").is_empty());
    }

    #[test]
    fn test_query_matches_file_path() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        e.record(chain);
        assert_eq!(e.query("main.rs").len(), 1);
    }

    #[test]
    fn test_query_matches_reasoning_steps() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.add_reasoning_step(&id, "Analyzed the loop bounds").unwrap();
        assert_eq!(e.query("loop bounds").len(), 1);
    }

    // -- accept / reject --

    #[test]
    fn test_accept() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.accept(&id).unwrap();
        assert_eq!(e.get_explanation(&id).unwrap().accepted, Some(true));
        assert!((e.metrics.acceptance_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reject_with_feedback() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.reject(&id, "wrong approach").unwrap();
        let entry = e.get_explanation(&id).unwrap();
        assert_eq!(entry.accepted, Some(false));
        assert_eq!(entry.user_feedback.as_deref(), Some("wrong approach"));
        assert!((e.metrics.acceptance_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_accept_not_found() {
        let mut e = engine();
        assert!(e.accept("nonexistent").is_err());
    }

    #[test]
    fn test_reject_not_found() {
        let mut e = engine();
        assert!(e.reject("nonexistent", "nope").is_err());
    }

    #[test]
    fn test_acceptance_rate_mixed() {
        let mut e = engine();
        let c1 = e.explain(sample_change(), "a", ChangeReason::BugFix);
        let c2 = e.explain(sample_change(), "b", ChangeReason::BugFix);
        let id1 = c1.id.clone();
        let id2 = c2.id.clone();
        e.record(c1);
        e.record(c2);
        e.accept(&id1).unwrap();
        e.reject(&id2, "nah").unwrap();
        assert!((e.metrics.acceptance_rate - 0.5).abs() < f64::EPSILON);
    }

    // -- add_context --

    #[test]
    fn test_add_context() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.add_context(&id, "LSP", "type info", 0.9).unwrap();
        let entry = e.get_explanation(&id).unwrap();
        assert_eq!(entry.chain.context_used.len(), 1);
        assert_eq!(entry.chain.context_used[0].source, "LSP");
    }

    #[test]
    fn test_add_context_not_found() {
        let mut e = engine();
        assert!(e.add_context("nope", "a", "b", 0.5).is_err());
    }

    #[test]
    fn test_add_context_disabled() {
        let mut e = ExplanationEngine::new(ExplainConfig {
            track_context: false,
            ..Default::default()
        });
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        // Should silently succeed but not add
        e.add_context(&id, "LSP", "info", 0.8).unwrap();
        assert!(e.get_explanation(&id).unwrap().chain.context_used.is_empty());
    }

    // -- add_alternative --

    #[test]
    fn test_add_alternative() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.add_alternative(&id, sample_alternative()).unwrap();
        assert_eq!(
            e.get_explanation(&id).unwrap().chain.alternatives.len(),
            1
        );
    }

    #[test]
    fn test_add_alternative_max_reached() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        for _ in 0..3 {
            e.add_alternative(&id, sample_alternative()).unwrap();
        }
        assert!(e.add_alternative(&id, sample_alternative()).is_err());
    }

    #[test]
    fn test_add_alternative_not_found() {
        let mut e = engine();
        assert!(e.add_alternative("nope", sample_alternative()).is_err());
    }

    // -- add_reasoning_step --

    #[test]
    fn test_add_reasoning_step() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.add_reasoning_step(&id, "Step 1").unwrap();
        e.add_reasoning_step(&id, "Step 2").unwrap();
        assert_eq!(
            e.get_explanation(&id)
                .unwrap()
                .chain
                .reasoning_steps
                .len(),
            2
        );
    }

    #[test]
    fn test_add_reasoning_step_not_found() {
        let mut e = engine();
        assert!(e.add_reasoning_step("nope", "step").is_err());
    }

    // -- export --

    #[test]
    fn test_export_markdown() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix bug", ChangeReason::BugFix);
        e.record(chain);
        let md = e.export(ExplanationFormat::Markdown);
        assert!(md.contains("# Audit Trail"));
        assert!(md.contains("fix bug"));
        assert!(md.contains("BugFix"));
    }

    #[test]
    fn test_export_json() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix bug", ChangeReason::BugFix);
        e.record(chain);
        let json = e.export(ExplanationFormat::Json);
        assert!(json.starts_with('['));
        assert!(json.contains("\"intent\":\"fix bug\""));
    }

    #[test]
    fn test_export_compact() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix bug", ChangeReason::BugFix);
        e.record(chain);
        let compact = e.export(ExplanationFormat::Compact);
        assert!(compact.contains("BugFix"));
        assert!(compact.contains("pending"));
    }

    #[test]
    fn test_export_entry_found() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        assert!(e.export_entry(&id, ExplanationFormat::Markdown).is_ok());
        assert!(e.export_entry(&id, ExplanationFormat::Json).is_ok());
        assert!(e.export_entry(&id, ExplanationFormat::Compact).is_ok());
    }

    #[test]
    fn test_export_entry_not_found() {
        let e = engine();
        assert!(e.export_entry("nope", ExplanationFormat::Json).is_err());
    }

    #[test]
    fn test_export_empty_trail() {
        let e = engine();
        let md = e.export(ExplanationFormat::Markdown);
        assert!(md.contains("# Audit Trail"));
        let json = e.export(ExplanationFormat::Json);
        assert!(json.starts_with('[') && json.ends_with(']'));
    }

    // -- get_trail / get_metrics --

    #[test]
    fn test_get_trail() {
        let e = engine();
        let trail = e.get_trail();
        assert!(trail.entries.is_empty());
        assert!(trail.session_id.starts_with("session-"));
    }

    #[test]
    fn test_get_metrics_initial() {
        let e = engine();
        let m = e.get_metrics();
        assert_eq!(m.total_explanations, 0);
        assert_eq!(m.total_alternatives, 0);
        assert!((m.avg_confidence - 0.0).abs() < f64::EPSILON);
    }

    // -- clear --

    #[test]
    fn test_clear() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        e.record(chain);
        assert_eq!(e.trail.entries.len(), 1);
        e.clear();
        assert_eq!(e.trail.entries.len(), 0);
        assert_eq!(e.metrics.total_explanations, 0);
    }

    // -- Multiple records and metrics --

    #[test]
    fn test_multiple_reasons_most_common() {
        let mut e = engine();
        for _ in 0..3 {
            let c = e.explain(sample_change(), "sec", ChangeReason::Security);
            e.record(c);
        }
        let c = e.explain(sample_change(), "bug", ChangeReason::BugFix);
        e.record(c);
        assert_eq!(e.metrics.most_common_reason, "Security");
    }

    #[test]
    fn test_total_alternatives_across_records() {
        let mut e = engine();
        let mut chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        chain.alternatives.push(sample_alternative());
        chain.alternatives.push(sample_alternative());
        e.record(chain);
        assert_eq!(e.metrics.total_alternatives, 2);
    }

    // -- Trade-offs in export --

    #[test]
    fn test_trade_offs_in_markdown_export() {
        let mut e = engine();
        let mut chain = e.explain(sample_change(), "perf", ChangeReason::Performance);
        chain.trade_offs.push("Higher memory usage".to_string());
        chain.trade_offs.push("Faster execution".to_string());
        e.record(chain);
        let md = e.export(ExplanationFormat::Markdown);
        assert!(md.contains("Higher memory usage"));
        assert!(md.contains("Faster execution"));
    }

    // -- Accepted status in compact export --

    #[test]
    fn test_accepted_in_compact_export() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.accept(&id).unwrap();
        let compact = e.export(ExplanationFormat::Compact);
        assert!(compact.contains("accepted"));
    }

    #[test]
    fn test_rejected_in_compact_export() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.reject(&id, "bad idea").unwrap();
        let compact = e.export(ExplanationFormat::Compact);
        assert!(compact.contains("rejected"));
    }

    // -- ExplanationChain fields --

    #[test]
    fn test_chain_change_fields() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "intent", ChangeReason::UserRequest);
        assert_eq!(chain.change.file_path, "src/main.rs");
        assert_eq!(chain.change.line_start, 10);
        assert_eq!(chain.change.line_end, 15);
        assert_eq!(chain.change.old_code, "let x = 1;");
        assert_eq!(chain.change.new_code, "let x = 2;");
    }

    // -- ContextUsed relevance --

    #[test]
    fn test_context_relevance_score() {
        let mut e = engine();
        let chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        let id = chain.id.clone();
        e.record(chain);
        e.add_context(&id, "LSP", "type data", 0.85).unwrap();
        let ctx = &e.get_explanation(&id).unwrap().chain.context_used[0];
        assert!((ctx.relevance_score - 0.85).abs() < f64::EPSILON);
    }

    // -- Edge: record chain with pre-populated alternatives --

    #[test]
    fn test_record_chain_with_alternatives_updates_metrics() {
        let mut e = engine();
        let mut chain = e.explain(sample_change(), "fix", ChangeReason::BugFix);
        chain.alternatives.push(sample_alternative());
        e.record(chain);
        // alternatives from the chain itself are counted
        assert_eq!(e.metrics.total_alternatives, 1);
    }
}
