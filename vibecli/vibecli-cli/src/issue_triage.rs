//! Autonomous issue triage — classify, label, and draft responses for issues.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueType {
    Bug,
    FeatureRequest,
    Question,
    Documentation,
    Duplicate,
    Enhancement,
    Chore,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IssueType::Bug => "Bug",
            IssueType::FeatureRequest => "FeatureRequest",
            IssueType::Question => "Question",
            IssueType::Documentation => "Documentation",
            IssueType::Duplicate => "Duplicate",
            IssueType::Enhancement => "Enhancement",
            IssueType::Chore => "Chore",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IssueSeverity::Critical => "Critical",
            IssueSeverity::High => "High",
            IssueSeverity::Medium => "Medium",
            IssueSeverity::Low => "Low",
            IssueSeverity::Informational => "Informational",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriageStatus {
    Untriaged,
    InProgress,
    Triaged,
    NeedsInfo,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSource {
    GitHub,
    GitLab,
    Linear,
    Jira,
    Manual,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub body: String,
    pub source: IssueSource,
    pub author: String,
    pub created_at: u64,
    pub labels: Vec<String>,
    pub status: TriageStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageResult {
    pub issue_id: String,
    pub classified_type: IssueType,
    pub severity: IssueSeverity,
    pub suggested_labels: Vec<String>,
    pub related_files: Vec<String>,
    pub confidence: f64,
    pub draft_response: String,
    pub auto_assigned: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageRule {
    pub id: String,
    pub pattern: String,
    pub applies_label: String,
    pub sets_severity: Option<IssueSeverity>,
    pub sets_type: Option<IssueType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageLearning {
    pub issue_id: String,
    pub original_type: IssueType,
    pub corrected_type: Option<IssueType>,
    pub original_severity: IssueSeverity,
    pub corrected_severity: Option<IssueSeverity>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageConfig {
    pub auto_label: bool,
    pub auto_assign: bool,
    pub auto_respond: bool,
    pub min_confidence: f64,
    pub stale_days: u64,
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            auto_label: true,
            auto_assign: false,
            auto_respond: false,
            min_confidence: 0.5,
            stale_days: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageMetrics {
    pub total_triaged: u64,
    pub auto_labeled: u64,
    pub auto_responded: u64,
    pub corrections: u64,
    pub avg_confidence: f64,
}

impl Default for TriageMetrics {
    fn default() -> Self {
        Self {
            total_triaged: 0,
            auto_labeled: 0,
            auto_responded: 0,
            corrections: 0,
            avg_confidence: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword tables
// ---------------------------------------------------------------------------

const BUG_KEYWORDS: &[&str] = &[
    "bug", "crash", "error", "broken", "fail", "not working", "regression",
];
const FEATURE_KEYWORDS: &[&str] = &[
    "feature", "add", "request", "implement", "proposal", "wish",
];
const QUESTION_KEYWORDS: &[&str] = &["how", "question", "?", "help", "why"];
const DOC_KEYWORDS: &[&str] = &["docs", "documentation", "readme", "typo", "guide"];
const ENHANCEMENT_KEYWORDS: &[&str] = &[
    "improve", "enhance", "optimize", "refactor", "performance",
];
const CHORE_KEYWORDS: &[&str] = &["chore", "ci", "deps", "bump", "lint", "update dependency"];
const CRITICAL_KEYWORDS: &[&str] = &["critical", "security", "data loss", "vulnerability"];
const HIGH_KEYWORDS: &[&str] = &["broken", "crash", "blocker", "severe"];
const MEDIUM_KEYWORDS: &[&str] = &["bug", "error", "fail", "wrong"];
const LOW_KEYWORDS: &[&str] = &["minor", "cosmetic", "nit", "small"];

// ---------------------------------------------------------------------------
// IssueClassifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueClassifier;

impl IssueClassifier {
    pub fn classify(&self, issue: &Issue) -> IssueType {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();

        let score = |keywords: &[&str]| -> usize {
            keywords.iter().filter(|kw| text.contains(**kw)).count()
        };

        let scores = vec![
            (IssueType::Bug, score(BUG_KEYWORDS)),
            (IssueType::FeatureRequest, score(FEATURE_KEYWORDS)),
            (IssueType::Question, score(QUESTION_KEYWORDS)),
            (IssueType::Documentation, score(DOC_KEYWORDS)),
            (IssueType::Enhancement, score(ENHANCEMENT_KEYWORDS)),
            (IssueType::Chore, score(CHORE_KEYWORDS)),
        ];

        let best = scores
            .into_iter()
            .max_by_key(|(_, s)| *s)
            .unwrap_or((IssueType::Enhancement, 0));

        if best.1 == 0 {
            IssueType::Enhancement
        } else {
            best.0
        }
    }

    pub fn estimate_severity(&self, issue: &Issue) -> IssueSeverity {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();

        let count = |kws: &[&str]| kws.iter().filter(|k| text.contains(**k)).count();

        let c = count(CRITICAL_KEYWORDS);
        let h = count(HIGH_KEYWORDS);
        let m = count(MEDIUM_KEYWORDS);
        let l = count(LOW_KEYWORDS);

        if c >= 2 || text.contains("data loss") || text.contains("vulnerability") {
            IssueSeverity::Critical
        } else if c >= 1 || h >= 2 {
            IssueSeverity::High
        } else if h >= 1 || m >= 2 {
            IssueSeverity::Medium
        } else if m >= 1 {
            IssueSeverity::Low
        } else if l >= 1 {
            IssueSeverity::Informational
        } else {
            IssueSeverity::Low
        }
    }

    pub fn confidence(&self, issue: &Issue) -> f64 {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();

        let all_groups: &[&[&str]] = &[
            BUG_KEYWORDS,
            FEATURE_KEYWORDS,
            QUESTION_KEYWORDS,
            DOC_KEYWORDS,
            ENHANCEMENT_KEYWORDS,
            CHORE_KEYWORDS,
        ];

        let mut best_score: f64 = 0.0;
        for keywords in all_groups {
            let hits = keywords.iter().filter(|kw| text.contains(**kw)).count();
            if !keywords.is_empty() {
                let s = hits as f64 / keywords.len() as f64;
                if s > best_score {
                    best_score = s;
                }
            }
        }

        // Scale so single-hit yields ~0.4 and multiple hits approach 1.0
        (best_score * 4.0).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// AutoLabeler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoLabeler {
    pub rules: Vec<TriageRule>,
}

impl AutoLabeler {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: TriageRule) {
        self.rules.push(rule);
    }

    pub fn suggest_labels(&self, issue: &Issue) -> Vec<String> {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();
        let mut labels = Vec::new();
        for rule in &self.rules {
            if text.contains(&rule.pattern.to_lowercase()) && !labels.contains(&rule.applies_label)
            {
                labels.push(rule.applies_label.clone());
            }
        }
        labels
    }

    pub fn detect_components(&self, issue: &Issue) -> Vec<String> {
        let text = format!("{} {}", issue.title, issue.body);
        let mut components = Vec::new();

        // Match patterns like src/, .rs, .tsx, .ts, .js etc.
        for word in text.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.');
            if (w.contains('/') || w.ends_with(".rs") || w.ends_with(".tsx") || w.ends_with(".ts")
                || w.ends_with(".js") || w.ends_with(".py") || w.ends_with(".go")
                || w.ends_with(".toml") || w.ends_with(".json"))
                && !components.contains(&w.to_string())
            {
                components.push(w.to_string());
            }
        }
        components
    }
}

impl Default for AutoLabeler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CodeLinker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLinker;

impl CodeLinker {
    pub fn find_related_files(&self, issue: &Issue, known_files: &[&str]) -> Vec<String> {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();
        let mut related = Vec::new();
        for f in known_files {
            let name = f.rsplit('/').next().unwrap_or(f).to_lowercase();
            if text.contains(&name) || text.contains(&f.to_lowercase()) {
                related.push(f.to_string());
            }
        }
        related.sort();
        related.dedup();
        related
    }

    pub fn find_related_functions(&self, issue: &Issue) -> Vec<String> {
        let text = format!("{} {}", issue.title, issue.body);
        let mut functions = Vec::new();
        for word in text.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '(' && c != ')' && c != '_');
            if w.ends_with("()") {
                let name = w.trim_end_matches("()");
                if !name.is_empty() && !functions.contains(&name.to_string()) {
                    functions.push(name.to_string());
                }
            }
        }
        functions
    }
}

// ---------------------------------------------------------------------------
// ResponseDrafter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDrafter;

impl ResponseDrafter {
    pub fn draft(&self, issue: &Issue, triage: &TriageResult) -> String {
        let type_label = match &triage.classified_type {
            IssueType::Bug => "bug report",
            IssueType::FeatureRequest => "feature request",
            IssueType::Question => "question",
            IssueType::Documentation => "documentation issue",
            IssueType::Duplicate => "duplicate report",
            IssueType::Enhancement => "enhancement suggestion",
            IssueType::Chore => "maintenance task",
        };

        let severity_note = match &triage.severity {
            IssueSeverity::Critical | IssueSeverity::High => {
                " This has been marked as high priority and will be addressed promptly."
            }
            _ => "",
        };

        let files_note = if triage.related_files.is_empty() {
            String::new()
        } else {
            format!(
                " Related files: {}.",
                triage.related_files.join(", ")
            )
        };

        format!(
            "Thank you @{} for filing this {}! Severity: {}.{}{} We will review it shortly.",
            issue.author,
            type_label,
            triage.severity,
            severity_note,
            files_note,
        )
    }
}

// ---------------------------------------------------------------------------
// TriageEngine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageEngine {
    pub config: TriageConfig,
    pub issues: HashMap<String, Issue>,
    pub results: HashMap<String, TriageResult>,
    pub rules: Vec<TriageRule>,
    pub learning: Vec<TriageLearning>,
    pub metrics: TriageMetrics,
    pub classifier: IssueClassifier,
    pub labeler: AutoLabeler,
    pub linker: CodeLinker,
    pub drafter: ResponseDrafter,
    next_id: u64,
    timestamp_counter: u64,
}

impl TriageEngine {
    pub fn new(config: TriageConfig) -> Self {
        Self {
            config,
            issues: HashMap::new(),
            results: HashMap::new(),
            rules: Vec::new(),
            learning: Vec::new(),
            metrics: TriageMetrics::default(),
            classifier: IssueClassifier,
            labeler: AutoLabeler::new(),
            linker: CodeLinker,
            drafter: ResponseDrafter,
            next_id: 1,
            timestamp_counter: 0,
        }
    }

    /// Store an issue and auto-triage if config.auto_label is true.
    /// Returns the issue id.
    pub fn add_issue(&mut self, issue: Issue) -> String {
        let id = issue.id.clone();
        self.issues.insert(id.clone(), issue);
        if self.config.auto_label {
            let _ = self.triage(&id);
        }
        id
    }

    /// Full triage pipeline for a single issue.
    pub fn triage(&mut self, issue_id: &str) -> Result<TriageResult, String> {
        let issue = self
            .issues
            .get_mut(issue_id)
            .ok_or_else(|| format!("Issue '{}' not found", issue_id))?;
        issue.status = TriageStatus::InProgress;
        let issue = issue.clone();

        let classified_type = self.classifier.classify(&issue);
        let severity = self.classifier.estimate_severity(&issue);
        let confidence = self.classifier.confidence(&issue);

        // Collect labels from rules
        let mut suggested_labels = self.labeler.suggest_labels(&issue);
        // Also apply engine-level rules
        for rule in &self.rules {
            let text = format!("{} {}", issue.title, issue.body).to_lowercase();
            if text.contains(&rule.pattern.to_lowercase())
                && !suggested_labels.contains(&rule.applies_label)
            {
                suggested_labels.push(rule.applies_label.clone());
            }
        }

        // Add type and severity labels
        let type_label = format!("type:{}", classified_type.to_string().to_lowercase());
        if !suggested_labels.contains(&type_label) {
            suggested_labels.push(type_label);
        }
        let sev_label = format!("severity:{}", severity.to_string().to_lowercase());
        if !suggested_labels.contains(&sev_label) {
            suggested_labels.push(sev_label);
        }

        // Detect components as related files
        let components = self.labeler.detect_components(&issue);
        let mut related_files = components;

        // Also find known file refs (empty for now, but we use linker with components)
        let known: Vec<&str> = related_files.iter().map(|s| s.as_str()).collect();
        let linked = self.linker.find_related_files(&issue, &known);
        for f in linked {
            if !related_files.contains(&f) {
                related_files.push(f);
            }
        }

        // Auto-assign
        let auto_assigned = if self.config.auto_assign {
            Some(format!("team-{}", classified_type.to_string().to_lowercase()))
        } else {
            None
        };

        let mut result = TriageResult {
            issue_id: issue_id.to_string(),
            classified_type,
            severity,
            suggested_labels,
            related_files,
            confidence,
            draft_response: String::new(),
            auto_assigned,
        };

        // Generate draft response
        if self.config.auto_respond || confidence >= self.config.min_confidence {
            result.draft_response = self.drafter.draft(&issue, &result);
        }

        // Update issue status
        if let Some(iss) = self.issues.get_mut(issue_id) {
            iss.status = TriageStatus::Triaged;
        }

        // Update metrics
        self.metrics.total_triaged += 1;
        if self.config.auto_label {
            self.metrics.auto_labeled += 1;
        }
        if !result.draft_response.is_empty() {
            self.metrics.auto_responded += 1;
        }
        let n = self.metrics.total_triaged as f64;
        self.metrics.avg_confidence =
            self.metrics.avg_confidence * ((n - 1.0) / n) + result.confidence / n;

        self.results.insert(issue_id.to_string(), result.clone());
        Ok(result)
    }

    /// Triage all untriaged issues.
    pub fn batch_triage(&mut self) -> Vec<TriageResult> {
        let untriaged_ids: Vec<String> = self
            .issues
            .iter()
            .filter(|(_, iss)| iss.status == TriageStatus::Untriaged)
            .map(|(id, _)| id.clone())
            .collect();

        let mut results = Vec::new();
        for id in untriaged_ids {
            if let Ok(r) = self.triage(&id) {
                results.push(r);
            }
        }
        results
    }

    /// Record a learning correction.
    pub fn correct(
        &mut self,
        issue_id: &str,
        corrected_type: Option<IssueType>,
        corrected_severity: Option<IssueSeverity>,
    ) -> Result<(), String> {
        let result = self
            .results
            .get(issue_id)
            .ok_or_else(|| format!("No triage result for '{}'", issue_id))?;

        self.timestamp_counter += 1;
        let learning = TriageLearning {
            issue_id: issue_id.to_string(),
            original_type: result.classified_type.clone(),
            corrected_type,
            original_severity: result.severity.clone(),
            corrected_severity,
            timestamp: self.timestamp_counter,
        };

        self.learning.push(learning);
        self.metrics.corrections += 1;
        Ok(())
    }

    pub fn list_issues(&self) -> Vec<&Issue> {
        self.issues.values().collect()
    }

    pub fn list_by_status(&self, status: &TriageStatus) -> Vec<&Issue> {
        self.issues
            .values()
            .filter(|iss| &iss.status == status)
            .collect()
    }

    pub fn get_result(&self, issue_id: &str) -> Option<&TriageResult> {
        self.results.get(issue_id)
    }

    /// Mark old untriaged issues as Stale. Returns the count of newly stale issues.
    pub fn mark_stale(&mut self, age_days: u64) -> usize {
        let threshold_secs = age_days * 86400;
        let now = self.timestamp_counter;
        let mut count = 0;
        for issue in self.issues.values_mut() {
            if issue.status == TriageStatus::Untriaged {
                // Use created_at as the reference timestamp
                let age = if now > issue.created_at {
                    now - issue.created_at
                } else if issue.created_at > 0 {
                    // Treat large created_at values as epoch timestamps
                    // and check if they're old enough by comparing against threshold
                    threshold_secs + 1 // Default: mark as stale if untriaged
                } else {
                    0
                };
                if age > threshold_secs {
                    issue.status = TriageStatus::Stale;
                    count += 1;
                }
            }
        }
        count
    }

    pub fn get_metrics(&self) -> &TriageMetrics {
        &self.metrics
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(id: &str, title: &str, body: &str) -> Issue {
        Issue {
            id: id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            source: IssueSource::GitHub,
            author: "alice".to_string(),
            created_at: 1_700_000_000,
            labels: Vec::new(),
            status: TriageStatus::Untriaged,
        }
    }

    fn default_engine() -> TriageEngine {
        TriageEngine::new(TriageConfig::default())
    }

    // -- IssueClassifier: classify -------------------------------------------

    #[test]
    fn classify_bug_by_title() {
        let c = IssueClassifier;
        let issue = make_issue("1", "App crashes on startup", "");
        assert_eq!(c.classify(&issue), IssueType::Bug);
    }

    #[test]
    fn classify_bug_by_body() {
        let c = IssueClassifier;
        let issue = make_issue("2", "Problem", "I see an error and it is broken");
        assert_eq!(c.classify(&issue), IssueType::Bug);
    }

    #[test]
    fn classify_feature_request() {
        let c = IssueClassifier;
        let issue = make_issue("3", "Feature request: add dark mode", "Please implement this");
        assert_eq!(c.classify(&issue), IssueType::FeatureRequest);
    }

    #[test]
    fn classify_question() {
        let c = IssueClassifier;
        let issue = make_issue("4", "How do I configure this?", "I need help");
        assert_eq!(c.classify(&issue), IssueType::Question);
    }

    #[test]
    fn classify_documentation() {
        let c = IssueClassifier;
        let issue = make_issue("5", "Docs are outdated", "The readme has a typo in the guide");
        assert_eq!(c.classify(&issue), IssueType::Documentation);
    }

    #[test]
    fn classify_enhancement() {
        let c = IssueClassifier;
        let issue = make_issue("6", "Improve performance of indexer", "Optimize the algorithm");
        assert_eq!(c.classify(&issue), IssueType::Enhancement);
    }

    #[test]
    fn classify_chore() {
        let c = IssueClassifier;
        let issue = make_issue("7", "Chore: bump deps", "Update dependency versions and lint");
        assert_eq!(c.classify(&issue), IssueType::Chore);
    }

    #[test]
    fn classify_unknown_defaults_to_enhancement() {
        let c = IssueClassifier;
        let issue = make_issue("8", "Something vague", "No keywords here at all");
        assert_eq!(c.classify(&issue), IssueType::Enhancement);
    }

    // -- IssueClassifier: estimate_severity ----------------------------------

    #[test]
    fn severity_critical_data_loss() {
        let c = IssueClassifier;
        let issue = make_issue("s1", "Data loss on save", "");
        assert_eq!(c.estimate_severity(&issue), IssueSeverity::Critical);
    }

    #[test]
    fn severity_critical_vulnerability() {
        let c = IssueClassifier;
        let issue = make_issue("s2", "Security vulnerability found", "");
        assert_eq!(c.estimate_severity(&issue), IssueSeverity::Critical);
    }

    #[test]
    fn severity_high() {
        let c = IssueClassifier;
        let issue = make_issue("s3", "App is broken and crash", "severe issue");
        let sev = c.estimate_severity(&issue);
        assert!(sev == IssueSeverity::High || sev == IssueSeverity::Medium);
    }

    #[test]
    fn severity_medium() {
        let c = IssueClassifier;
        let issue = make_issue("s4", "Bug with error dialog", "something fails");
        let sev = c.estimate_severity(&issue);
        assert!(sev == IssueSeverity::Medium || sev == IssueSeverity::Low);
    }

    #[test]
    fn severity_informational() {
        let c = IssueClassifier;
        let issue = make_issue("s5", "Minor cosmetic nit", "");
        assert_eq!(c.estimate_severity(&issue), IssueSeverity::Informational);
    }

    #[test]
    fn severity_default_low() {
        let c = IssueClassifier;
        let issue = make_issue("s6", "Unrelated title", "No severity signals");
        assert_eq!(c.estimate_severity(&issue), IssueSeverity::Low);
    }

    // -- IssueClassifier: confidence -----------------------------------------

    #[test]
    fn confidence_zero_for_no_keywords() {
        let c = IssueClassifier;
        let issue = make_issue("c1", "xyz", "abc");
        assert_eq!(c.confidence(&issue), 0.0);
    }

    #[test]
    fn confidence_increases_with_more_keywords() {
        let c = IssueClassifier;
        let single = make_issue("c2", "crash", "");
        let multi = make_issue("c3", "crash error bug broken fail regression", "");
        assert!(c.confidence(&multi) > c.confidence(&single));
    }

    #[test]
    fn confidence_clamped_to_one() {
        let c = IssueClassifier;
        let issue = make_issue("c4", "bug crash error broken fail not working regression", "");
        assert!(c.confidence(&issue) <= 1.0);
    }

    // -- AutoLabeler ---------------------------------------------------------

    #[test]
    fn labeler_suggest_labels_from_rules() {
        let mut labeler = AutoLabeler::new();
        labeler.add_rule(TriageRule {
            id: "r1".into(),
            pattern: "crash".into(),
            applies_label: "bug".into(),
            sets_severity: None,
            sets_type: None,
        });
        let issue = make_issue("l1", "App crash", "");
        let labels = labeler.suggest_labels(&issue);
        assert_eq!(labels, vec!["bug".to_string()]);
    }

    #[test]
    fn labeler_no_duplicate_labels() {
        let mut labeler = AutoLabeler::new();
        labeler.add_rule(TriageRule {
            id: "r1".into(),
            pattern: "crash".into(),
            applies_label: "bug".into(),
            sets_severity: None,
            sets_type: None,
        });
        labeler.add_rule(TriageRule {
            id: "r2".into(),
            pattern: "error".into(),
            applies_label: "bug".into(),
            sets_severity: None,
            sets_type: None,
        });
        let issue = make_issue("l2", "crash and error", "");
        let labels = labeler.suggest_labels(&issue);
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn labeler_no_match_returns_empty() {
        let labeler = AutoLabeler::new();
        let issue = make_issue("l3", "Hello", "");
        assert!(labeler.suggest_labels(&issue).is_empty());
    }

    #[test]
    fn labeler_detect_components_rs_file() {
        let labeler = AutoLabeler::new();
        let issue = make_issue("d1", "Bug in config.rs", "See also src/main.rs");
        let comps = labeler.detect_components(&issue);
        assert!(comps.contains(&"config.rs".to_string()));
        assert!(comps.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn labeler_detect_components_tsx() {
        let labeler = AutoLabeler::new();
        let issue = make_issue("d2", "Error in App.tsx", "");
        let comps = labeler.detect_components(&issue);
        assert!(comps.contains(&"App.tsx".to_string()));
    }

    #[test]
    fn labeler_detect_no_components() {
        let labeler = AutoLabeler::new();
        let issue = make_issue("d3", "General question", "No file refs");
        assert!(labeler.detect_components(&issue).is_empty());
    }

    // -- CodeLinker ----------------------------------------------------------

    #[test]
    fn code_linker_find_related_files() {
        let linker = CodeLinker;
        let issue = make_issue("f1", "Bug in config.rs", "See main.rs");
        let known = vec!["src/config.rs", "src/main.rs", "src/util.rs"];
        let related = linker.find_related_files(&issue, &known);
        assert!(related.contains(&"src/config.rs".to_string()));
        assert!(related.contains(&"src/main.rs".to_string()));
        assert!(!related.contains(&"src/util.rs".to_string()));
    }

    #[test]
    fn code_linker_no_known_files() {
        let linker = CodeLinker;
        let issue = make_issue("f2", "Anything", "");
        assert!(linker.find_related_files(&issue, &[]).is_empty());
    }

    #[test]
    fn code_linker_find_related_functions() {
        let linker = CodeLinker;
        let issue = make_issue("f3", "Call to process() fails", "Also check init()");
        let fns = linker.find_related_functions(&issue);
        assert!(fns.contains(&"process".to_string()));
        assert!(fns.contains(&"init".to_string()));
    }

    #[test]
    fn code_linker_no_functions() {
        let linker = CodeLinker;
        let issue = make_issue("f4", "No function refs", "Just text");
        assert!(linker.find_related_functions(&issue).is_empty());
    }

    // -- ResponseDrafter -----------------------------------------------------

    #[test]
    fn drafter_includes_type_and_severity() {
        let drafter = ResponseDrafter;
        let issue = make_issue("dr1", "Crash", "");
        let result = TriageResult {
            issue_id: "dr1".into(),
            classified_type: IssueType::Bug,
            severity: IssueSeverity::Critical,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.8,
            draft_response: String::new(),
            auto_assigned: None,
        };
        let resp = drafter.draft(&issue, &result);
        assert!(resp.contains("bug report"));
        assert!(resp.contains("Critical"));
        assert!(resp.contains("@alice"));
        assert!(resp.contains("high priority"));
    }

    #[test]
    fn drafter_includes_related_files() {
        let drafter = ResponseDrafter;
        let issue = make_issue("dr2", "Issue", "");
        let result = TriageResult {
            issue_id: "dr2".into(),
            classified_type: IssueType::Enhancement,
            severity: IssueSeverity::Low,
            suggested_labels: vec![],
            related_files: vec!["src/main.rs".into()],
            confidence: 0.5,
            draft_response: String::new(),
            auto_assigned: None,
        };
        let resp = drafter.draft(&issue, &result);
        assert!(resp.contains("src/main.rs"));
    }

    // -- TriageEngine --------------------------------------------------------

    #[test]
    fn engine_add_and_triage_issue() {
        let mut engine = default_engine();
        let issue = make_issue("e1", "App crashes with error", "broken");
        engine.add_issue(issue);
        let result = engine.get_result("e1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().classified_type, IssueType::Bug);
    }

    #[test]
    fn engine_triage_not_found() {
        let mut engine = default_engine();
        assert!(engine.triage("nonexistent").is_err());
    }

    #[test]
    fn engine_batch_triage() {
        let config = TriageConfig {
            auto_label: false,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        engine.issues.insert(
            "b1".into(),
            make_issue("b1", "Bug crash", "error"),
        );
        engine.issues.insert(
            "b2".into(),
            make_issue("b2", "Feature request", "add dark mode"),
        );
        let results = engine.batch_triage();
        assert_eq!(results.len(), 2);
        assert_eq!(engine.metrics.total_triaged, 2);
    }

    #[test]
    fn engine_batch_triage_skips_triaged() {
        let mut engine = default_engine();
        let mut issue = make_issue("bt1", "Bug crash", "");
        issue.status = TriageStatus::Triaged;
        engine.issues.insert("bt1".into(), issue);
        let results = engine.batch_triage();
        assert!(results.is_empty());
    }

    #[test]
    fn engine_correct_records_learning() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("c1", "Crash error", "broken"));
        let res = engine.correct("c1", Some(IssueType::FeatureRequest), None);
        assert!(res.is_ok());
        assert_eq!(engine.learning.len(), 1);
        assert_eq!(engine.metrics.corrections, 1);
        assert_eq!(
            engine.learning[0].corrected_type,
            Some(IssueType::FeatureRequest)
        );
    }

    #[test]
    fn engine_correct_not_found() {
        let mut engine = default_engine();
        assert!(engine.correct("nope", None, None).is_err());
    }

    #[test]
    fn engine_list_issues() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("li1", "A", ""));
        engine.add_issue(make_issue("li2", "B", ""));
        assert_eq!(engine.list_issues().len(), 2);
    }

    #[test]
    fn engine_list_by_status() {
        let config = TriageConfig {
            auto_label: false,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        engine.add_issue(make_issue("s1", "A", ""));
        engine.add_issue(make_issue("s2", "B", ""));
        let _ = engine.triage("s1");
        let untriaged = engine.list_by_status(&TriageStatus::Untriaged);
        assert_eq!(untriaged.len(), 1);
        let triaged = engine.list_by_status(&TriageStatus::Triaged);
        assert_eq!(triaged.len(), 1);
    }

    #[test]
    fn engine_get_result() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("gr1", "Crash error", ""));
        assert!(engine.get_result("gr1").is_some());
        assert!(engine.get_result("nope").is_none());
    }

    #[test]
    fn engine_mark_stale() {
        let config = TriageConfig {
            auto_label: false,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        let mut issue = make_issue("st1", "Old issue", "");
        issue.created_at = 0;
        engine.issues.insert("st1".into(), issue);
        engine.timestamp_counter = 86400 * 31; // 31 days in seconds
        let count = engine.mark_stale(30);
        assert_eq!(count, 1);
        assert_eq!(
            engine.issues.get("st1").unwrap().status,
            TriageStatus::Stale
        );
    }

    #[test]
    fn engine_mark_stale_skips_triaged() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("st2", "Triaged issue", "crash error"));
        // Issue was auto-triaged, so status is Triaged
        engine.timestamp_counter = 86400 * 100;
        let count = engine.mark_stale(1);
        assert_eq!(count, 0);
    }

    #[test]
    fn engine_metrics_after_triage() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("m1", "Bug crash error", "broken"));
        let m = engine.get_metrics();
        assert_eq!(m.total_triaged, 1);
        assert!(m.avg_confidence > 0.0);
        assert!(m.auto_labeled > 0);
    }

    #[test]
    fn engine_metrics_avg_confidence_multiple() {
        let mut engine = default_engine();
        engine.add_issue(make_issue("m2", "Bug crash error", ""));
        engine.add_issue(make_issue("m3", "Vague thing", "nothing"));
        let m = engine.get_metrics();
        assert_eq!(m.total_triaged, 2);
        assert!(m.avg_confidence > 0.0);
    }

    #[test]
    fn engine_auto_assign() {
        let config = TriageConfig {
            auto_assign: true,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        engine.add_issue(make_issue("aa1", "Bug crash", "error"));
        let result = engine.get_result("aa1").unwrap();
        assert!(result.auto_assigned.is_some());
        assert!(result.auto_assigned.as_ref().unwrap().contains("bug"));
    }

    #[test]
    fn engine_auto_respond() {
        let config = TriageConfig {
            auto_respond: true,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        engine.add_issue(make_issue("ar1", "Bug crash", "error"));
        let result = engine.get_result("ar1").unwrap();
        assert!(!result.draft_response.is_empty());
    }

    #[test]
    fn engine_rules_affect_triage() {
        let mut engine = default_engine();
        engine.rules.push(TriageRule {
            id: "r1".into(),
            pattern: "security".into(),
            applies_label: "security".into(),
            sets_severity: Some(IssueSeverity::Critical),
            sets_type: None,
        });
        engine.add_issue(make_issue("ru1", "Security issue", "vulnerability"));
        let result = engine.get_result("ru1").unwrap();
        assert!(result.suggested_labels.contains(&"security".to_string()));
    }

    #[test]
    fn engine_labeler_rules_affect_triage() {
        let mut engine = default_engine();
        engine.labeler.add_rule(TriageRule {
            id: "lr1".into(),
            pattern: "performance".into(),
            applies_label: "perf".into(),
            sets_severity: None,
            sets_type: None,
        });
        engine.add_issue(make_issue("lr1", "Performance regression", "slow"));
        let result = engine.get_result("lr1").unwrap();
        assert!(result.suggested_labels.contains(&"perf".to_string()));
    }

    #[test]
    fn issue_source_variants_all_work() {
        let sources = vec![
            IssueSource::GitHub,
            IssueSource::GitLab,
            IssueSource::Linear,
            IssueSource::Jira,
            IssueSource::Manual,
        ];
        for source in sources {
            let mut issue = make_issue("sv", "Test", "");
            issue.source = source;
            let c = IssueClassifier;
            let _ = c.classify(&issue);
        }
    }

    #[test]
    fn triage_status_variants() {
        let statuses = vec![
            TriageStatus::Untriaged,
            TriageStatus::InProgress,
            TriageStatus::Triaged,
            TriageStatus::NeedsInfo,
            TriageStatus::Stale,
        ];
        for s in &statuses {
            let _ = format!("{:?}", s);
        }
        assert_ne!(TriageStatus::Untriaged, TriageStatus::Triaged);
    }

    #[test]
    fn issue_type_display() {
        assert_eq!(IssueType::Bug.to_string(), "Bug");
        assert_eq!(IssueType::FeatureRequest.to_string(), "FeatureRequest");
        assert_eq!(IssueType::Chore.to_string(), "Chore");
    }

    #[test]
    fn severity_display() {
        assert_eq!(IssueSeverity::Critical.to_string(), "Critical");
        assert_eq!(IssueSeverity::Informational.to_string(), "Informational");
    }

    #[test]
    fn triage_config_default() {
        let config = TriageConfig::default();
        assert!(config.auto_label);
        assert!(!config.auto_assign);
        assert!(!config.auto_respond);
        assert_eq!(config.min_confidence, 0.5);
        assert_eq!(config.stale_days, 30);
    }

    #[test]
    fn triage_metrics_default() {
        let m = TriageMetrics::default();
        assert_eq!(m.total_triaged, 0);
        assert_eq!(m.auto_labeled, 0);
        assert_eq!(m.auto_responded, 0);
        assert_eq!(m.corrections, 0);
        assert_eq!(m.avg_confidence, 0.0);
    }
}
