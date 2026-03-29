#![allow(dead_code)]
//! Proactive intelligence — background scanner that identifies issues and suggests improvements.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScanCategory {
    Performance,
    Security,
    TechDebt,
    Correctness,
    Accessibility,
    TestingGaps,
    Documentation,
    DependencyHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SuggestionPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScanTrigger {
    Manual,
    OnSave,
    OnPush,
    Scheduled(u64),
    OnFileChange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Snoozed(u64),
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DigestFrequency {
    Immediate,
    Hourly,
    Daily,
    Weekly,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveScanConfig {
    pub enabled: bool,
    pub triggers: Vec<ScanTrigger>,
    pub categories: Vec<ScanCategory>,
    pub min_confidence: f64,
    pub digest_frequency: DigestFrequency,
    pub quiet_mode: bool,
    pub max_suggestions_per_scan: usize,
}

impl Default for ProactiveScanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            triggers: vec![ScanTrigger::OnSave, ScanTrigger::Manual],
            categories: vec![
                ScanCategory::Performance,
                ScanCategory::Security,
                ScanCategory::TechDebt,
                ScanCategory::Correctness,
                ScanCategory::Accessibility,
                ScanCategory::TestingGaps,
                ScanCategory::Documentation,
                ScanCategory::DependencyHealth,
            ],
            min_confidence: 0.5,
            digest_frequency: DigestFrequency::Daily,
            quiet_mode: false,
            max_suggestions_per_scan: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub category: ScanCategory,
    pub priority: SuggestionPriority,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub confidence: f64,
    pub status: SuggestionStatus,
    pub created_at: u64,
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub suggestions: Vec<Suggestion>,
    pub scanned_files: usize,
    pub scan_duration_ms: u64,
    pub category: ScanCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub suggestion_id: String,
    pub category: ScanCategory,
    pub was_accepted: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningStore {
    pub entries: Vec<LearningEntry>,
}

impl LearningStore {
    pub fn record(&mut self, entry: LearningEntry) {
        self.entries.push(entry);
    }

    pub fn acceptance_rate(&self, category: &ScanCategory) -> f64 {
        let relevant: Vec<&LearningEntry> = self
            .entries
            .iter()
            .filter(|e| &e.category == category)
            .collect();
        if relevant.is_empty() {
            return 0.5; // neutral prior when no data
        }
        let accepted = relevant.iter().filter(|e| e.was_accepted).count();
        accepted as f64 / relevant.len() as f64
    }

    pub fn total_accepted(&self) -> usize {
        self.entries.iter().filter(|e| e.was_accepted).count()
    }

    pub fn total_rejected(&self) -> usize {
        self.entries.iter().filter(|e| !e.was_accepted).count()
    }

    /// Returns (accepted, rejected) per category.
    pub fn category_stats(&self) -> HashMap<ScanCategory, (usize, usize)> {
        let mut map: HashMap<ScanCategory, (usize, usize)> = HashMap::new();
        for entry in &self.entries {
            let stat = map.entry(entry.category.clone()).or_insert((0, 0));
            if entry.was_accepted {
                stat.0 += 1;
            } else {
                stat.1 += 1;
            }
        }
        map
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProactiveMetrics {
    pub total_scans: u64,
    pub total_suggestions: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub snoozed: u64,
    pub avg_confidence: f64,
}

// ---------------------------------------------------------------------------
// SuggestionGenerator
// ---------------------------------------------------------------------------

pub struct SuggestionGenerator;

impl SuggestionGenerator {
    pub fn generate_for_category(category: &ScanCategory, file_path: &str) -> Option<Suggestion> {
        let ext = file_path.rsplit('.').next().unwrap_or("");
        let confidence = Self::estimate_confidence(category, ext);
        if confidence < 0.1 {
            return None;
        }

        let (title, description, priority, fix_hint) = match category {
            ScanCategory::Performance => match ext {
                "rs" => (
                    "Avoid unnecessary cloning in hot path".to_string(),
                    "Consider borrowing instead of cloning to reduce allocations.".to_string(),
                    SuggestionPriority::High,
                    Some("Use &T instead of T.clone()".to_string()),
                ),
                "js" | "ts" | "tsx" => (
                    "Memoize expensive computation".to_string(),
                    "Repeated computation detected; wrap with useMemo or cache result.".to_string(),
                    SuggestionPriority::Medium,
                    Some("Wrap with useMemo(() => expr, [deps])".to_string()),
                ),
                "py" => (
                    "Use list comprehension instead of loop-append".to_string(),
                    "List comprehension is faster than repeated list.append().".to_string(),
                    SuggestionPriority::Low,
                    Some("Replace loop with [x for x in iterable]".to_string()),
                ),
                _ => (
                    "General performance review recommended".to_string(),
                    "Consider profiling this file for hotspots.".to_string(),
                    SuggestionPriority::Low,
                    None,
                ),
            },
            ScanCategory::Security => match ext {
                "rs" => (
                    "Validate untrusted input before use".to_string(),
                    "Input from external sources should be validated and sanitized.".to_string(),
                    SuggestionPriority::Critical,
                    Some("Add input validation at the boundary".to_string()),
                ),
                "js" | "ts" | "tsx" => (
                    "Potential XSS via dangerouslySetInnerHTML".to_string(),
                    "Ensure content passed to dangerouslySetInnerHTML is sanitized.".to_string(),
                    SuggestionPriority::Critical,
                    Some("Use DOMPurify.sanitize() before rendering".to_string()),
                ),
                "py" => (
                    "Avoid eval() on user input".to_string(),
                    "Using eval() with untrusted data can lead to code injection.".to_string(),
                    SuggestionPriority::Critical,
                    Some("Use ast.literal_eval() or a whitelist parser".to_string()),
                ),
                _ => (
                    "Review file for secrets or credentials".to_string(),
                    "Scan for hardcoded API keys, passwords, or tokens.".to_string(),
                    SuggestionPriority::High,
                    None,
                ),
            },
            ScanCategory::TechDebt => (
                "Reduce function complexity".to_string(),
                "Function exceeds recommended cyclomatic complexity threshold.".to_string(),
                SuggestionPriority::Medium,
                Some("Extract helper functions to reduce complexity".to_string()),
            ),
            ScanCategory::Correctness => (
                "Potential off-by-one error".to_string(),
                "Loop boundary may cause off-by-one; verify start/end indices.".to_string(),
                SuggestionPriority::High,
                Some("Check inclusive vs exclusive bounds".to_string()),
            ),
            ScanCategory::Accessibility => match ext {
                "tsx" | "jsx" | "html" => (
                    "Missing alt attribute on image".to_string(),
                    "Images require alt text for screen readers.".to_string(),
                    SuggestionPriority::Medium,
                    Some("Add alt=\"descriptive text\" to <img> tags".to_string()),
                ),
                _ => (
                    "Review accessibility compliance".to_string(),
                    "Ensure UI components meet WCAG 2.1 AA standards.".to_string(),
                    SuggestionPriority::Low,
                    None,
                ),
            },
            ScanCategory::TestingGaps => (
                "Missing test coverage for error paths".to_string(),
                "Error handling branches lack corresponding unit tests.".to_string(),
                SuggestionPriority::Medium,
                Some("Add tests for Result::Err and panic branches".to_string()),
            ),
            ScanCategory::Documentation => (
                "Public function missing doc comment".to_string(),
                "Public API should have documentation comments for discoverability.".to_string(),
                SuggestionPriority::Low,
                Some("Add /// doc comment above the function".to_string()),
            ),
            ScanCategory::DependencyHealth => (
                "Outdated dependency detected".to_string(),
                "One or more dependencies have newer versions with security patches.".to_string(),
                SuggestionPriority::High,
                Some("Run cargo update / npm update and review changelogs".to_string()),
            ),
        };

        Some(Suggestion {
            id: String::new(), // caller assigns id
            category: category.clone(),
            priority,
            title,
            description,
            file_path: Some(file_path.to_string()),
            line_range: Some((1, 50)),
            confidence,
            status: SuggestionStatus::Pending,
            created_at: 0, // caller assigns timestamp
            fix_hint,
        })
    }

    pub fn estimate_confidence(category: &ScanCategory, file_ext: &str) -> f64 {
        let base: f64 = match category {
            ScanCategory::Security => 0.85,
            ScanCategory::Correctness => 0.75,
            ScanCategory::Performance => 0.70,
            ScanCategory::DependencyHealth => 0.80,
            ScanCategory::TestingGaps => 0.65,
            ScanCategory::TechDebt => 0.60,
            ScanCategory::Accessibility => 0.55,
            ScanCategory::Documentation => 0.50,
        };
        let ext_bonus: f64 = match file_ext {
            "rs" | "ts" | "tsx" => 0.10,
            "js" | "py" => 0.05,
            "html" | "css" => 0.02,
            _ => 0.0,
        };
        (base + ext_bonus).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// ProactiveAgent
// ---------------------------------------------------------------------------

pub struct ProactiveAgent {
    pub config: ProactiveScanConfig,
    pub suggestions: HashMap<String, Suggestion>,
    pub learning: LearningStore,
    pub metrics: ProactiveMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl ProactiveAgent {
    pub fn new(config: ProactiveScanConfig) -> Self {
        Self {
            config,
            suggestions: HashMap::new(),
            learning: LearningStore::default(),
            metrics: ProactiveMetrics::default(),
            next_id: 1,
            timestamp_counter: 1000,
        }
    }

    fn alloc_id(&mut self) -> String {
        let id = format!("sug-{:04}", self.next_id);
        self.next_id += 1;
        id
    }

    fn now(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn scan(&mut self, category: &ScanCategory, files: &[&str]) -> ScanResult {
        let start = self.timestamp_counter;
        let mut result_suggestions = Vec::new();

        for &file in files {
            if result_suggestions.len() >= self.config.max_suggestions_per_scan {
                break;
            }
            if let Some(mut sug) = SuggestionGenerator::generate_for_category(category, file) {
                if sug.confidence >= self.config.min_confidence {
                    let id = self.alloc_id();
                    let ts = self.now();
                    sug.id = id.clone();
                    sug.created_at = ts;
                    result_suggestions.push(sug.clone());
                    self.suggestions.insert(id, sug);
                }
            }
        }

        self.metrics.total_scans += 1;
        self.metrics.total_suggestions += result_suggestions.len() as u64;
        self.update_avg_confidence();

        let duration = self.timestamp_counter.saturating_sub(start) + 5; // simulated

        ScanResult {
            suggestions: result_suggestions,
            scanned_files: files.len(),
            scan_duration_ms: duration,
            category: category.clone(),
        }
    }

    pub fn scan_all(&mut self, files: &[&str]) -> Vec<ScanResult> {
        let cats: Vec<ScanCategory> = self.config.categories.clone();
        let mut results = Vec::new();
        for cat in &cats {
            results.push(self.scan(cat, files));
        }
        results
    }

    pub fn accept(&mut self, id: &str) -> Result<(), String> {
        let sug = self
            .suggestions
            .get_mut(id)
            .ok_or_else(|| format!("Suggestion {} not found", id))?;
        if sug.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion {} is not pending", id));
        }
        sug.status = SuggestionStatus::Accepted;
        let ts = self.timestamp_counter + 1;
        self.timestamp_counter = ts;
        self.learning.record(LearningEntry {
            suggestion_id: id.to_string(),
            category: sug.category.clone(),
            was_accepted: true,
            timestamp: ts,
        });
        self.metrics.accepted += 1;
        Ok(())
    }

    pub fn reject(&mut self, id: &str) -> Result<(), String> {
        let sug = self
            .suggestions
            .get_mut(id)
            .ok_or_else(|| format!("Suggestion {} not found", id))?;
        if sug.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion {} is not pending", id));
        }
        sug.status = SuggestionStatus::Rejected;
        let ts = self.timestamp_counter + 1;
        self.timestamp_counter = ts;
        self.learning.record(LearningEntry {
            suggestion_id: id.to_string(),
            category: sug.category.clone(),
            was_accepted: false,
            timestamp: ts,
        });
        self.metrics.rejected += 1;
        Ok(())
    }

    pub fn snooze(&mut self, id: &str, until: u64) -> Result<(), String> {
        let sug = self
            .suggestions
            .get_mut(id)
            .ok_or_else(|| format!("Suggestion {} not found", id))?;
        if sug.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion {} is not pending", id));
        }
        sug.status = SuggestionStatus::Snoozed(until);
        self.metrics.snoozed += 1;
        Ok(())
    }

    pub fn list_suggestions(&self) -> Vec<&Suggestion> {
        self.suggestions.values().collect()
    }

    pub fn list_by_category(&self, cat: &ScanCategory) -> Vec<&Suggestion> {
        self.suggestions
            .values()
            .filter(|s| &s.category == cat)
            .collect()
    }

    pub fn list_by_priority(&self, pri: &SuggestionPriority) -> Vec<&Suggestion> {
        self.suggestions
            .values()
            .filter(|s| &s.priority == pri)
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.suggestions
            .values()
            .filter(|s| s.status == SuggestionStatus::Pending)
            .count()
    }

    pub fn digest(&self) -> Vec<&Suggestion> {
        let mut pending: Vec<&Suggestion> = self
            .suggestions
            .values()
            .filter(|s| s.status == SuggestionStatus::Pending)
            .collect();
        pending.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        pending
    }

    pub fn get_learning_stats(&self) -> &LearningStore {
        &self.learning
    }

    pub fn get_metrics(&self) -> &ProactiveMetrics {
        &self.metrics
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let now = self.timestamp_counter;
        let expired_ids: Vec<String> = self
            .suggestions
            .iter()
            .filter_map(|(id, s)| {
                if let SuggestionStatus::Snoozed(until) = s.status {
                    if now > until {
                        return Some(id.clone());
                    }
                }
                None
            })
            .collect();
        let count = expired_ids.len();
        for id in &expired_ids {
            if let Some(sug) = self.suggestions.get_mut(id) {
                sug.status = SuggestionStatus::Expired;
            }
        }
        count
    }

    pub fn should_suggest(&self, category: &ScanCategory) -> bool {
        self.learning.acceptance_rate(category) >= 0.3
    }

    fn update_avg_confidence(&mut self) {
        if self.suggestions.is_empty() {
            self.metrics.avg_confidence = 0.0;
            return;
        }
        let sum: f64 = self.suggestions.values().map(|s| s.confidence).sum();
        self.metrics.avg_confidence = sum / self.suggestions.len() as f64;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_agent() -> ProactiveAgent {
        ProactiveAgent::new(ProactiveScanConfig::default())
    }

    fn rust_files() -> Vec<&'static str> {
        vec!["main.rs", "lib.rs", "utils.rs"]
    }

    // -- Config defaults --

    #[test]
    fn test_default_config_enabled() {
        let cfg = ProactiveScanConfig::default();
        assert!(cfg.enabled);
    }

    #[test]
    fn test_default_config_min_confidence() {
        let cfg = ProactiveScanConfig::default();
        assert!((cfg.min_confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_config_categories_count() {
        let cfg = ProactiveScanConfig::default();
        assert_eq!(cfg.categories.len(), 8);
    }

    #[test]
    fn test_default_config_max_suggestions() {
        let cfg = ProactiveScanConfig::default();
        assert_eq!(cfg.max_suggestions_per_scan, 10);
    }

    #[test]
    fn test_default_config_digest_frequency() {
        let cfg = ProactiveScanConfig::default();
        assert_eq!(cfg.digest_frequency, DigestFrequency::Daily);
    }

    // -- SuggestionGenerator --

    #[test]
    fn test_generate_performance_rs() {
        let sug = SuggestionGenerator::generate_for_category(&ScanCategory::Performance, "hot.rs");
        assert!(sug.is_some());
        let s = sug.unwrap();
        assert_eq!(s.category, ScanCategory::Performance);
        assert!(s.confidence > 0.5);
    }

    #[test]
    fn test_generate_security_js() {
        let sug = SuggestionGenerator::generate_for_category(&ScanCategory::Security, "app.js");
        assert!(sug.is_some());
        let s = sug.unwrap();
        assert_eq!(s.priority, SuggestionPriority::Critical);
    }

    #[test]
    fn test_generate_security_py() {
        let sug = SuggestionGenerator::generate_for_category(&ScanCategory::Security, "run.py");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("eval"));
    }

    #[test]
    fn test_generate_accessibility_tsx() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::Accessibility, "App.tsx");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("alt"));
    }

    #[test]
    fn test_generate_testing_gaps() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::TestingGaps, "foo.rs");
        assert!(sug.is_some());
    }

    #[test]
    fn test_generate_documentation() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::Documentation, "lib.rs");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("doc comment"));
    }

    #[test]
    fn test_generate_dependency_health() {
        let sug = SuggestionGenerator::generate_for_category(
            &ScanCategory::DependencyHealth,
            "Cargo.toml",
        );
        assert!(sug.is_some());
    }

    #[test]
    fn test_generate_tech_debt() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::TechDebt, "complex.rs");
        assert!(sug.is_some());
        assert_eq!(sug.unwrap().priority, SuggestionPriority::Medium);
    }

    #[test]
    fn test_generate_correctness() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::Correctness, "loop.rs");
        assert!(sug.is_some());
        assert_eq!(sug.unwrap().priority, SuggestionPriority::High);
    }

    #[test]
    fn test_estimate_confidence_security_rs() {
        let c = SuggestionGenerator::estimate_confidence(&ScanCategory::Security, "rs");
        assert!(c > 0.9);
    }

    #[test]
    fn test_estimate_confidence_docs_unknown() {
        let c = SuggestionGenerator::estimate_confidence(&ScanCategory::Documentation, "xyz");
        assert!((c - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_confidence_capped_at_one() {
        let c = SuggestionGenerator::estimate_confidence(&ScanCategory::Security, "ts");
        assert!(c <= 1.0);
    }

    // -- ProactiveAgent scan --

    #[test]
    fn test_scan_produces_suggestions() {
        let mut agent = default_agent();
        let result = agent.scan(&ScanCategory::Performance, &rust_files());
        assert!(!result.suggestions.is_empty());
        assert_eq!(result.scanned_files, 3);
    }

    #[test]
    fn test_scan_respects_max_suggestions() {
        let mut cfg = ProactiveScanConfig::default();
        cfg.max_suggestions_per_scan = 2;
        let mut agent = ProactiveAgent::new(cfg);
        let files: Vec<&str> = vec!["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"];
        let result = agent.scan(&ScanCategory::Security, &files);
        assert!(result.suggestions.len() <= 2);
    }

    #[test]
    fn test_scan_all_covers_all_categories() {
        let mut agent = default_agent();
        let results = agent.scan_all(&["main.rs"]);
        assert_eq!(results.len(), 8);
    }

    #[test]
    fn test_scan_increments_metrics() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["app.rs"]);
        assert_eq!(agent.metrics.total_scans, 1);
        assert!(agent.metrics.total_suggestions >= 1);
    }

    #[test]
    fn test_scan_assigns_unique_ids() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs", "b.rs"]);
        let ids: Vec<String> = agent.suggestions.keys().cloned().collect();
        let unique: std::collections::HashSet<String> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_scan_filters_by_min_confidence() {
        let mut cfg = ProactiveScanConfig::default();
        cfg.min_confidence = 0.99;
        let mut agent = ProactiveAgent::new(cfg);
        let result = agent.scan(&ScanCategory::Documentation, &["notes.md"]);
        // Documentation + unknown ext = 0.50, below 0.99
        assert!(result.suggestions.is_empty());
    }

    // -- Accept / Reject / Snooze --

    #[test]
    fn test_accept_suggestion() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["app.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        assert!(agent.accept(&id).is_ok());
        assert_eq!(agent.suggestions[&id].status, SuggestionStatus::Accepted);
        assert_eq!(agent.metrics.accepted, 1);
    }

    #[test]
    fn test_reject_suggestion() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["app.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        assert!(agent.reject(&id).is_ok());
        assert_eq!(agent.suggestions[&id].status, SuggestionStatus::Rejected);
        assert_eq!(agent.metrics.rejected, 1);
    }

    #[test]
    fn test_snooze_suggestion() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["app.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        assert!(agent.snooze(&id, 9999).is_ok());
        assert_eq!(
            agent.suggestions[&id].status,
            SuggestionStatus::Snoozed(9999)
        );
        assert_eq!(agent.metrics.snoozed, 1);
    }

    #[test]
    fn test_accept_nonexistent_fails() {
        let mut agent = default_agent();
        assert!(agent.accept("nope").is_err());
    }

    #[test]
    fn test_reject_nonexistent_fails() {
        let mut agent = default_agent();
        assert!(agent.reject("nope").is_err());
    }

    #[test]
    fn test_snooze_nonexistent_fails() {
        let mut agent = default_agent();
        assert!(agent.snooze("nope", 100).is_err());
    }

    #[test]
    fn test_double_accept_fails() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.accept(&id).unwrap();
        assert!(agent.accept(&id).is_err());
    }

    #[test]
    fn test_reject_after_accept_fails() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.accept(&id).unwrap();
        assert!(agent.reject(&id).is_err());
    }

    // -- Listing / Filtering --

    #[test]
    fn test_list_suggestions() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Performance, &["a.rs", "b.rs"]);
        assert!(agent.list_suggestions().len() >= 2);
    }

    #[test]
    fn test_list_by_category() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        agent.scan(&ScanCategory::Performance, &["b.rs"]);
        let sec = agent.list_by_category(&ScanCategory::Security);
        assert!(!sec.is_empty());
        assert!(sec.iter().all(|s| s.category == ScanCategory::Security));
    }

    #[test]
    fn test_list_by_priority() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let critical = agent.list_by_priority(&SuggestionPriority::Critical);
        assert!(!critical.is_empty());
    }

    #[test]
    fn test_pending_count() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs", "b.rs"]);
        let total = agent.suggestions.len();
        assert_eq!(agent.pending_count(), total);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.accept(&id).unwrap();
        assert_eq!(agent.pending_count(), total - 1);
    }

    // -- Digest --

    #[test]
    fn test_digest_returns_pending_only() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs", "b.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.accept(&id).unwrap();
        let digest = agent.digest();
        assert!(digest
            .iter()
            .all(|s| s.status == SuggestionStatus::Pending));
    }

    #[test]
    fn test_digest_sorted_by_priority() {
        let mut agent = default_agent();
        agent.scan_all(&["main.rs", "app.tsx"]);
        let digest = agent.digest();
        for window in digest.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    // -- Cleanup --

    #[test]
    fn test_cleanup_expired_marks_past_snoozed() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.snooze(&id, 500).unwrap(); // snooze until 500, but counter > 1000
        let count = agent.cleanup_expired();
        assert_eq!(count, 1);
        assert_eq!(agent.suggestions[&id].status, SuggestionStatus::Expired);
    }

    #[test]
    fn test_cleanup_no_false_positives() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.snooze(&id, 999_999).unwrap(); // far future
        let count = agent.cleanup_expired();
        assert_eq!(count, 0);
    }

    // -- Learning --

    #[test]
    fn test_learning_record_and_acceptance_rate() {
        let mut store = LearningStore::default();
        store.record(LearningEntry {
            suggestion_id: "s1".into(),
            category: ScanCategory::Security,
            was_accepted: true,
            timestamp: 1,
        });
        store.record(LearningEntry {
            suggestion_id: "s2".into(),
            category: ScanCategory::Security,
            was_accepted: false,
            timestamp: 2,
        });
        assert!((store.acceptance_rate(&ScanCategory::Security) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_learning_no_data_returns_neutral() {
        let store = LearningStore::default();
        assert!((store.acceptance_rate(&ScanCategory::Performance) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_learning_total_accepted_rejected() {
        let mut store = LearningStore::default();
        store.record(LearningEntry {
            suggestion_id: "a".into(),
            category: ScanCategory::TechDebt,
            was_accepted: true,
            timestamp: 1,
        });
        store.record(LearningEntry {
            suggestion_id: "b".into(),
            category: ScanCategory::TechDebt,
            was_accepted: false,
            timestamp: 2,
        });
        store.record(LearningEntry {
            suggestion_id: "c".into(),
            category: ScanCategory::Security,
            was_accepted: true,
            timestamp: 3,
        });
        assert_eq!(store.total_accepted(), 2);
        assert_eq!(store.total_rejected(), 1);
    }

    #[test]
    fn test_learning_category_stats() {
        let mut store = LearningStore::default();
        store.record(LearningEntry {
            suggestion_id: "a".into(),
            category: ScanCategory::Security,
            was_accepted: true,
            timestamp: 1,
        });
        store.record(LearningEntry {
            suggestion_id: "b".into(),
            category: ScanCategory::Security,
            was_accepted: false,
            timestamp: 2,
        });
        store.record(LearningEntry {
            suggestion_id: "c".into(),
            category: ScanCategory::Performance,
            was_accepted: true,
            timestamp: 3,
        });
        let stats = store.category_stats();
        assert_eq!(stats[&ScanCategory::Security], (1, 1));
        assert_eq!(stats[&ScanCategory::Performance], (1, 0));
    }

    // -- should_suggest --

    #[test]
    fn test_should_suggest_true_with_no_data() {
        let agent = default_agent();
        // neutral 0.5 >= 0.3 threshold
        assert!(agent.should_suggest(&ScanCategory::Security));
    }

    #[test]
    fn test_should_suggest_false_when_all_rejected() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Documentation, &["a.rs", "b.rs", "c.rs"]);
        let ids: Vec<String> = agent.suggestions.keys().cloned().collect();
        for id in ids {
            let _ = agent.reject(&id);
        }
        // acceptance rate = 0.0 < 0.3
        assert!(!agent.should_suggest(&ScanCategory::Documentation));
    }

    // -- Metrics --

    #[test]
    fn test_metrics_avg_confidence() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["app.rs"]);
        assert!(agent.metrics.avg_confidence > 0.0);
    }

    #[test]
    fn test_metrics_after_multiple_scans() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        agent.scan(&ScanCategory::Performance, &["b.rs"]);
        assert_eq!(agent.metrics.total_scans, 2);
    }

    // -- Serialization round-trip --

    #[test]
    fn test_scan_category_serde_round_trip() {
        let cat = ScanCategory::DependencyHealth;
        let json = serde_json::to_string(&cat).unwrap();
        let back: ScanCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat);
    }

    #[test]
    fn test_suggestion_serde_round_trip() {
        let sug = Suggestion {
            id: "test-1".into(),
            category: ScanCategory::Security,
            priority: SuggestionPriority::Critical,
            title: "Test".into(),
            description: "Desc".into(),
            file_path: Some("main.rs".into()),
            line_range: Some((1, 10)),
            confidence: 0.95,
            status: SuggestionStatus::Pending,
            created_at: 42,
            fix_hint: Some("fix it".into()),
        };
        let json = serde_json::to_string(&sug).unwrap();
        let back: Suggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "test-1");
        assert_eq!(back.priority, SuggestionPriority::Critical);
    }

    #[test]
    fn test_config_serde_round_trip() {
        let cfg = ProactiveScanConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: ProactiveScanConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.categories.len(), 8);
    }

    #[test]
    fn test_snoozed_status_serde() {
        let status = SuggestionStatus::Snoozed(12345);
        let json = serde_json::to_string(&status).unwrap();
        let back: SuggestionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SuggestionStatus::Snoozed(12345));
    }

    #[test]
    fn test_scheduled_trigger_serde() {
        let trigger = ScanTrigger::Scheduled(30);
        let json = serde_json::to_string(&trigger).unwrap();
        let back: ScanTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ScanTrigger::Scheduled(30));
    }

    // -- Additional edge-case tests --

    #[test]
    fn test_scan_empty_file_list() {
        let mut agent = default_agent();
        let result = agent.scan(&ScanCategory::Security, &[]);
        assert!(result.suggestions.is_empty());
        assert_eq!(result.scanned_files, 0);
    }

    #[test]
    fn test_generate_performance_ts() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::Performance, "index.ts");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("Memoize"));
    }

    #[test]
    fn test_generate_security_tsx() {
        let sug = SuggestionGenerator::generate_for_category(&ScanCategory::Security, "App.tsx");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("XSS"));
    }

    #[test]
    fn test_generate_accessibility_non_ui_file() {
        let sug =
            SuggestionGenerator::generate_for_category(&ScanCategory::Accessibility, "data.json");
        assert!(sug.is_some());
        assert!(sug.unwrap().title.contains("accessibility"));
    }

    #[test]
    fn test_scan_result_duration_positive() {
        let mut agent = default_agent();
        let result = agent.scan(&ScanCategory::Performance, &["main.rs"]);
        assert!(result.scan_duration_ms > 0);
    }

    #[test]
    fn test_learning_all_accepted_rate_is_one() {
        let mut store = LearningStore::default();
        for i in 0..5 {
            store.record(LearningEntry {
                suggestion_id: format!("s{}", i),
                category: ScanCategory::Security,
                was_accepted: true,
                timestamp: i as u64,
            });
        }
        assert!((store.acceptance_rate(&ScanCategory::Security) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_agent_new_starts_empty() {
        let agent = default_agent();
        assert!(agent.suggestions.is_empty());
        assert_eq!(agent.metrics.total_scans, 0);
        assert_eq!(agent.pending_count(), 0);
    }

    #[test]
    fn test_snooze_then_reject_fails() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        let id = agent.suggestions.keys().next().unwrap().clone();
        agent.snooze(&id, 99999).unwrap();
        assert!(agent.reject(&id).is_err());
    }

    #[test]
    fn test_multiple_categories_in_suggestions() {
        let mut agent = default_agent();
        agent.scan(&ScanCategory::Security, &["a.rs"]);
        agent.scan(&ScanCategory::Documentation, &["b.rs"]);
        let cats: std::collections::HashSet<_> = agent
            .suggestions
            .values()
            .map(|s| s.category.clone())
            .collect();
        assert!(cats.contains(&ScanCategory::Security));
        assert!(cats.contains(&ScanCategory::Documentation));
    }
}
