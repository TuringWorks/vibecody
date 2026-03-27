//! Autonomous issue triage engine.
//!
//! Provides keyword-based classification, severity estimation, rule-driven
//! actions, related-file detection, draft response generation, batch triage,
//! and learning-from-corrections with accuracy tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The category an issue falls into.
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

/// How severe the issue is considered to be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Trivial,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IssueSeverity::Critical => "Critical",
            IssueSeverity::High => "High",
            IssueSeverity::Medium => "Medium",
            IssueSeverity::Low => "Low",
            IssueSeverity::Trivial => "Trivial",
        };
        write!(f, "{}", s)
    }
}

/// Where the issue originated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSource {
    GitHub,
    GitLab,
    Linear,
    Jira,
    Manual,
}

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// A single issue to triage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub body: String,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: u64,
    pub source: IssueSource,
    pub url: Option<String>,
}

/// The output of triaging a single issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageResult {
    pub issue_id: String,
    pub classified_type: IssueType,
    pub severity: IssueSeverity,
    pub suggested_labels: Vec<String>,
    pub related_files: Vec<String>,
    pub confidence: f64,
    pub draft_response: Option<String>,
    pub assigned_to: Option<String>,
    pub duplicate_of: Option<String>,
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// A condition that determines whether a rule fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleCondition {
    TitleContains(String),
    BodyContains(String),
    LabelPresent(String),
    AuthorIs(String),
    Any,
}

/// An action to apply when a rule fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleAction {
    SetType(IssueType),
    SetSeverity(IssueSeverity),
    AddLabel(String),
    AssignTo(String),
    MarkDuplicate(String),
}

/// A triage rule consisting of a condition and an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageRule {
    pub id: String,
    pub name: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: u32,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Learning
// ---------------------------------------------------------------------------

/// A recorded correction of a previous triage result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageCorrection {
    pub issue_id: String,
    pub original_type: IssueType,
    pub corrected_type: IssueType,
    pub original_severity: IssueSeverity,
    pub corrected_severity: IssueSeverity,
    pub timestamp: u64,
}

/// Tracks corrections so the engine can learn over time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageLearning {
    pub corrections: Vec<TriageCorrection>,
    /// Maps IssueType display name → (correct_count, total_count).
    pub type_accuracy: HashMap<String, (u32, u32)>,
}

impl Default for TriageLearning {
    fn default() -> Self {
        Self::new()
    }
}

impl TriageLearning {
    pub fn new() -> Self {
        Self {
            corrections: Vec::new(),
            type_accuracy: HashMap::new(),
        }
    }

    /// Record a human correction of an automated triage result.
    pub fn record_correction(
        &mut self,
        issue_id: &str,
        original: &TriageResult,
        corrected_type: IssueType,
        corrected_severity: IssueSeverity,
    ) {
        let key = original.classified_type.to_string();
        let entry = self.type_accuracy.entry(key).or_insert((0, 0));
        entry.1 += 1;
        if original.classified_type == corrected_type {
            entry.0 += 1;
        }

        self.corrections.push(TriageCorrection {
            issue_id: issue_id.to_string(),
            original_type: original.classified_type.clone(),
            corrected_type,
            original_severity: original.severity.clone(),
            corrected_severity,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
    }

    /// Return accuracy (0.0–1.0) for a given issue type, or 1.0 when no data.
    pub fn get_accuracy(&self, issue_type: &IssueType) -> f64 {
        let key = issue_type.to_string();
        match self.type_accuracy.get(&key) {
            Some(&(correct, total)) if total > 0 => correct as f64 / total as f64,
            _ => 1.0,
        }
    }

    pub fn get_corrections(&self) -> &[TriageCorrection] {
        &self.corrections
    }
}

// ---------------------------------------------------------------------------
// Config & Metrics
// ---------------------------------------------------------------------------

/// Configuration for the triage engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageConfig {
    pub auto_label: bool,
    pub auto_assign: bool,
    pub auto_respond: bool,
    pub min_confidence: f64,
    /// Maps category name → default assignee.
    pub default_assignees: HashMap<String, String>,
    pub known_files: Vec<String>,
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            auto_label: true,
            auto_assign: false,
            auto_respond: false,
            min_confidence: 0.5,
            default_assignees: HashMap::new(),
            known_files: Vec::new(),
        }
    }
}

/// Aggregate metrics for triage activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriageMetrics {
    pub total_triaged: u32,
    pub by_type: HashMap<String, u32>,
    pub by_severity: HashMap<String, u32>,
    pub avg_confidence: f64,
    pub corrections_count: u32,
}

impl Default for TriageMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl TriageMetrics {
    pub fn new() -> Self {
        Self {
            total_triaged: 0,
            by_type: HashMap::new(),
            by_severity: HashMap::new(),
            avg_confidence: 0.0,
            corrections_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword tables (used by classify_type)
// ---------------------------------------------------------------------------

const BUG_KEYWORDS: &[&str] = &[
    "crash", "error", "bug", "broken", "fix", "fail", "not working", "regression",
];

const FEATURE_KEYWORDS: &[&str] = &[
    "feature", "add", "support", "implement", "request", "new", "wish", "proposal",
];

const QUESTION_KEYWORDS: &[&str] = &[
    "how", "why", "?", "question", "help", "documentation", "example",
];

const DOC_KEYWORDS: &[&str] = &[
    "docs", "readme", "typo", "documentation", "guide", "tutorial",
];

const ENHANCEMENT_KEYWORDS: &[&str] = &[
    "improve", "enhance", "optimize", "refactor", "performance", "speed",
];

const CHORE_KEYWORDS: &[&str] = &[
    "chore", "ci", "deps", "dependency", "bump", "update dependency", "lint",
];

// ---------------------------------------------------------------------------
// TriageEngine
// ---------------------------------------------------------------------------

/// The main triage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageEngine {
    pub rules: Vec<TriageRule>,
    pub history: Vec<TriageResult>,
    pub learning: TriageLearning,
    pub config: TriageConfig,
    pub metrics: TriageMetrics,
}

impl TriageEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: TriageConfig) -> Self {
        Self {
            rules: Vec::new(),
            history: Vec::new(),
            learning: TriageLearning::new(),
            config,
            metrics: TriageMetrics::new(),
        }
    }

    // -- Rules ---------------------------------------------------------------

    /// Add a triage rule.
    pub fn add_rule(&mut self, rule: TriageRule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a rule by id.
    pub fn remove_rule(&mut self, id: &str) -> Result<(), String> {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        if self.rules.len() == before {
            Err(format!("Rule '{}' not found", id))
        } else {
            Ok(())
        }
    }

    /// Evaluate all enabled rules against an issue and return matching actions,
    /// ordered by rule priority (highest first).
    pub fn apply_rules(&self, issue: &Issue) -> Vec<RuleAction> {
        let mut actions = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            let matched = match &rule.condition {
                RuleCondition::TitleContains(pat) => {
                    issue.title.to_lowercase().contains(&pat.to_lowercase())
                }
                RuleCondition::BodyContains(pat) => {
                    issue.body.to_lowercase().contains(&pat.to_lowercase())
                }
                RuleCondition::LabelPresent(label) => {
                    issue.labels.iter().any(|l| l.to_lowercase() == label.to_lowercase())
                }
                RuleCondition::AuthorIs(author) => {
                    issue.author.to_lowercase() == author.to_lowercase()
                }
                RuleCondition::Any => true,
            };
            if matched {
                actions.push(rule.action.clone());
            }
        }
        actions
    }

    // -- Classification ------------------------------------------------------

    /// Classify the issue type using keyword heuristics. Returns the type and a
    /// confidence score (0.0–1.0).
    pub fn classify_type(&self, issue: &Issue) -> (IssueType, f64) {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();

        let score = |keywords: &[&str]| -> f64 {
            let hits = keywords.iter().filter(|kw| text.contains(**kw)).count();
            if keywords.is_empty() {
                return 0.0;
            }
            hits as f64 / keywords.len() as f64
        };

        let scores: Vec<(IssueType, f64)> = vec![
            (IssueType::Bug, score(BUG_KEYWORDS)),
            (IssueType::FeatureRequest, score(FEATURE_KEYWORDS)),
            (IssueType::Question, score(QUESTION_KEYWORDS)),
            (IssueType::Documentation, score(DOC_KEYWORDS)),
            (IssueType::Enhancement, score(ENHANCEMENT_KEYWORDS)),
            (IssueType::Chore, score(CHORE_KEYWORDS)),
        ];

        let best = scores
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((IssueType::Enhancement, 0.0));

        if best.1 == 0.0 {
            (IssueType::Enhancement, 0.1)
        } else {
            // Scale confidence so a single-keyword hit yields ~0.5 and multiple
            // hits approach 1.0.
            let confidence = (best.1 * 5.0).clamp(0.1, 1.0);
            (best.0, confidence)
        }
    }

    // -- Severity ------------------------------------------------------------

    /// Estimate severity from keyword signals in the issue text.
    pub fn estimate_severity(&self, issue: &Issue) -> IssueSeverity {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();

        let critical_kw = ["crash", "data loss", "security", "vulnerability", "exploit", "urgent"];
        let high_kw = ["regression", "broken", "blocker", "severe", "critical"];
        let medium_kw = ["bug", "error", "fail", "not working", "wrong"];
        let low_kw = ["minor", "cosmetic", "typo", "style", "nit"];

        let count = |kws: &[&str]| kws.iter().filter(|k| text.contains(**k)).count();

        let c = count(&critical_kw);
        let h = count(&high_kw);
        let m = count(&medium_kw);
        let l = count(&low_kw);

        if c >= 2 || text.contains("data loss") || text.contains("vulnerability") {
            IssueSeverity::Critical
        } else if c >= 1 || h >= 2 {
            IssueSeverity::High
        } else if h >= 1 || m >= 2 {
            IssueSeverity::Medium
        } else if m >= 1 {
            IssueSeverity::Low
        } else if l >= 1 {
            IssueSeverity::Trivial
        } else {
            IssueSeverity::Low
        }
    }

    // -- Related files -------------------------------------------------------

    /// Find known files that are mentioned (by name or partial path) in the
    /// issue title or body.
    pub fn find_related_files(&self, issue: &Issue, known_files: &[String]) -> Vec<String> {
        let text = format!("{} {}", issue.title, issue.body).to_lowercase();
        let mut related = Vec::new();
        for f in known_files {
            let name = f.rsplit('/').next().unwrap_or(f).to_lowercase();
            if text.contains(&name) || text.contains(&f.to_lowercase()) {
                related.push(f.clone());
            }
        }
        related.sort();
        related.dedup();
        related
    }

    // -- Response generation -------------------------------------------------

    /// Generate a draft response acknowledging the issue.
    pub fn generate_response(&self, issue: &Issue, result: &TriageResult) -> String {
        let type_label = match &result.classified_type {
            IssueType::Bug => "bug report",
            IssueType::FeatureRequest => "feature request",
            IssueType::Question => "question",
            IssueType::Documentation => "documentation issue",
            IssueType::Duplicate => "duplicate report",
            IssueType::Enhancement => "enhancement suggestion",
            IssueType::Chore => "maintenance task",
        };

        let severity_note = match &result.severity {
            IssueSeverity::Critical | IssueSeverity::High => {
                " This has been marked as high priority and will be addressed promptly."
            }
            _ => "",
        };

        let assignee_note = match &result.assigned_to {
            Some(user) => format!(" It has been assigned to @{}.", user),
            None => String::new(),
        };

        let duplicate_note = match &result.duplicate_of {
            Some(dup) => format!(" Note: this may be a duplicate of #{}.", dup),
            None => String::new(),
        };

        format!(
            "Thank you @{} for filing this {}!{}{}{} We will review it shortly.",
            issue.author, type_label, severity_note, assignee_note, duplicate_note,
        )
    }

    // -- Triage --------------------------------------------------------------

    /// Triage a single issue, applying rules, classification, severity, related
    /// files, and optional response generation.
    pub fn triage_issue(&mut self, issue: &Issue) -> TriageResult {
        let (mut classified_type, confidence) = self.classify_type(issue);
        let mut severity = self.estimate_severity(issue);
        let mut suggested_labels: Vec<String> = Vec::new();
        let mut assigned_to: Option<String> = None;
        let mut duplicate_of: Option<String> = None;

        // Apply rules (higher-priority rules first).
        let actions = self.apply_rules(issue);
        for action in &actions {
            match action {
                RuleAction::SetType(t) => classified_type = t.clone(),
                RuleAction::SetSeverity(s) => severity = s.clone(),
                RuleAction::AddLabel(l) => {
                    if !suggested_labels.contains(l) {
                        suggested_labels.push(l.clone());
                    }
                }
                RuleAction::AssignTo(u) => assigned_to = Some(u.clone()),
                RuleAction::MarkDuplicate(d) => duplicate_of = Some(d.clone()),
            }
        }

        // Auto-label from type
        if self.config.auto_label {
            let type_label = classified_type.to_string().to_lowercase();
            if !suggested_labels.iter().any(|l| l.to_lowercase() == type_label) {
                suggested_labels.push(type_label);
            }
            let sev_label = format!("severity:{}", severity.to_string().to_lowercase());
            if !suggested_labels.contains(&sev_label) {
                suggested_labels.push(sev_label);
            }
        }

        // Auto-assign from default_assignees if no rule already assigned
        if self.config.auto_assign && assigned_to.is_none() {
            let key = classified_type.to_string();
            if let Some(user) = self.config.default_assignees.get(&key) {
                assigned_to = Some(user.clone());
            }
        }

        let related_files = self.find_related_files(issue, &self.config.known_files.clone());

        let mut result = TriageResult {
            issue_id: issue.id.clone(),
            classified_type,
            severity,
            suggested_labels,
            related_files,
            confidence,
            draft_response: None,
            assigned_to,
            duplicate_of,
        };

        // Auto-respond
        if self.config.auto_respond {
            let response = self.generate_response(issue, &result);
            result.draft_response = Some(response);
        }

        // Update metrics
        self.metrics.total_triaged += 1;
        *self
            .metrics
            .by_type
            .entry(result.classified_type.to_string())
            .or_insert(0) += 1;
        *self
            .metrics
            .by_severity
            .entry(result.severity.to_string())
            .or_insert(0) += 1;

        // Rolling average confidence
        let n = self.metrics.total_triaged as f64;
        self.metrics.avg_confidence =
            self.metrics.avg_confidence * ((n - 1.0) / n) + result.confidence / n;

        self.history.push(result.clone());
        result
    }

    /// Triage a batch of issues.
    pub fn triage_batch(&mut self, issues: &[Issue]) -> Vec<TriageResult> {
        issues.iter().map(|i| self.triage_issue(i)).collect()
    }

    /// Return the full triage history.
    pub fn get_history(&self) -> &[TriageResult] {
        &self.history
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers -------------------------------------------------------------

    fn make_issue(id: &str, title: &str, body: &str) -> Issue {
        Issue {
            id: id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            author: "alice".to_string(),
            labels: Vec::new(),
            created_at: 1700000000,
            source: IssueSource::GitHub,
            url: None,
        }
    }

    fn default_engine() -> TriageEngine {
        TriageEngine::new(TriageConfig::default())
    }

    // -- Issue classification ------------------------------------------------

    #[test]
    fn classify_bug_by_title() {
        let engine = default_engine();
        let issue = make_issue("1", "App crashes on startup", "");
        let (t, c) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Bug);
        assert!(c > 0.0);
    }

    #[test]
    fn classify_bug_by_body() {
        let engine = default_engine();
        let issue = make_issue("2", "Problem", "I see an error and it is broken");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Bug);
    }

    #[test]
    fn classify_feature_request() {
        let engine = default_engine();
        let issue = make_issue("3", "Feature request: add dark mode", "Please implement this");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::FeatureRequest);
    }

    #[test]
    fn classify_question() {
        let engine = default_engine();
        let issue = make_issue("4", "How do I configure this?", "I need help with an example");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Question);
    }

    #[test]
    fn classify_documentation() {
        let engine = default_engine();
        let issue = make_issue("5", "Docs are outdated", "The readme has a typo in the guide");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Documentation);
    }

    #[test]
    fn classify_enhancement() {
        let engine = default_engine();
        let issue = make_issue("6", "Improve performance of indexer", "Optimize the algorithm");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Enhancement);
    }

    #[test]
    fn classify_chore() {
        let engine = default_engine();
        let issue = make_issue("7", "Chore: bump deps", "Update dependency versions and lint");
        let (t, _) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Chore);
    }

    #[test]
    fn classify_unknown_defaults_to_enhancement() {
        let engine = default_engine();
        let issue = make_issue("8", "Something vague", "No keywords here at all");
        let (t, c) = engine.classify_type(&issue);
        assert_eq!(t, IssueType::Enhancement);
        assert!(c <= 0.2);
    }

    #[test]
    fn classify_confidence_increases_with_more_keywords() {
        let engine = default_engine();
        let single = make_issue("a", "crash", "");
        let multi = make_issue("b", "crash error bug broken fail regression", "");
        let (_, c1) = engine.classify_type(&single);
        let (_, c2) = engine.classify_type(&multi);
        assert!(c2 > c1);
    }

    // -- Severity estimation -------------------------------------------------

    #[test]
    fn severity_critical_data_loss() {
        let engine = default_engine();
        let issue = make_issue("s1", "Data loss on save", "");
        let sev = engine.estimate_severity(&issue);
        assert_eq!(sev, IssueSeverity::Critical);
    }

    #[test]
    fn severity_critical_vulnerability() {
        let engine = default_engine();
        let issue = make_issue("s2", "Security vulnerability found", "");
        let sev = engine.estimate_severity(&issue);
        assert_eq!(sev, IssueSeverity::Critical);
    }

    #[test]
    fn severity_high_regression() {
        let engine = default_engine();
        let issue = make_issue("s3", "Regression in latest release", "App is crash");
        let sev = engine.estimate_severity(&issue);
        assert!(sev == IssueSeverity::High || sev == IssueSeverity::Medium);
    }

    #[test]
    fn severity_medium() {
        let engine = default_engine();
        let issue = make_issue("s4", "Bug with error dialog", "Something fails");
        let sev = engine.estimate_severity(&issue);
        assert!(sev == IssueSeverity::Medium || sev == IssueSeverity::Low);
    }

    #[test]
    fn severity_trivial_typo() {
        let engine = default_engine();
        let issue = make_issue("s5", "Minor cosmetic typo", "");
        let sev = engine.estimate_severity(&issue);
        assert_eq!(sev, IssueSeverity::Trivial);
    }

    #[test]
    fn severity_default_low() {
        let engine = default_engine();
        let issue = make_issue("s6", "Unrelated title", "No severity signals");
        let sev = engine.estimate_severity(&issue);
        assert_eq!(sev, IssueSeverity::Low);
    }

    // -- Rule application ----------------------------------------------------

    #[test]
    fn rule_title_contains() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r1".into(),
            name: "Security bugs".into(),
            condition: RuleCondition::TitleContains("security".into()),
            action: RuleAction::SetSeverity(IssueSeverity::Critical),
            priority: 10,
            enabled: true,
        });
        let issue = make_issue("r1", "Security issue found", "");
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], RuleAction::SetSeverity(IssueSeverity::Critical));
    }

    #[test]
    fn rule_body_contains() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r2".into(),
            name: "Panic in body".into(),
            condition: RuleCondition::BodyContains("panic".into()),
            action: RuleAction::AddLabel("panic".into()),
            priority: 5,
            enabled: true,
        });
        let issue = make_issue("r2", "Issue", "Thread panic detected");
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions, vec![RuleAction::AddLabel("panic".into())]);
    }

    #[test]
    fn rule_label_present() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r3".into(),
            name: "P0 label".into(),
            condition: RuleCondition::LabelPresent("P0".into()),
            action: RuleAction::SetSeverity(IssueSeverity::Critical),
            priority: 20,
            enabled: true,
        });
        let mut issue = make_issue("r3", "Something", "");
        issue.labels = vec!["P0".into()];
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn rule_author_is() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r4".into(),
            name: "VIP author".into(),
            condition: RuleCondition::AuthorIs("ceo".into()),
            action: RuleAction::SetSeverity(IssueSeverity::High),
            priority: 15,
            enabled: true,
        });
        let mut issue = make_issue("r4", "Some issue", "");
        issue.author = "ceo".into();
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn rule_any_matches_everything() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r5".into(),
            name: "Catch-all label".into(),
            condition: RuleCondition::Any,
            action: RuleAction::AddLabel("needs-review".into()),
            priority: 1,
            enabled: true,
        });
        let issue = make_issue("r5", "Anything", "");
        assert_eq!(engine.apply_rules(&issue).len(), 1);
    }

    #[test]
    fn disabled_rule_not_applied() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "r6".into(),
            name: "Disabled".into(),
            condition: RuleCondition::Any,
            action: RuleAction::AddLabel("ghost".into()),
            priority: 100,
            enabled: false,
        });
        let issue = make_issue("r6", "Anything", "");
        assert!(engine.apply_rules(&issue).is_empty());
    }

    #[test]
    fn rules_ordered_by_priority() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "lo".into(),
            name: "Low".into(),
            condition: RuleCondition::Any,
            action: RuleAction::AddLabel("lo".into()),
            priority: 1,
            enabled: true,
        });
        engine.add_rule(TriageRule {
            id: "hi".into(),
            name: "High".into(),
            condition: RuleCondition::Any,
            action: RuleAction::AddLabel("hi".into()),
            priority: 99,
            enabled: true,
        });
        let issue = make_issue("p", "X", "");
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions[0], RuleAction::AddLabel("hi".into()));
        assert_eq!(actions[1], RuleAction::AddLabel("lo".into()));
    }

    #[test]
    fn remove_rule_success() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "del".into(),
            name: "To remove".into(),
            condition: RuleCondition::Any,
            action: RuleAction::AddLabel("x".into()),
            priority: 1,
            enabled: true,
        });
        assert!(engine.remove_rule("del").is_ok());
        assert!(engine.rules.is_empty());
    }

    #[test]
    fn remove_rule_not_found() {
        let mut engine = default_engine();
        assert!(engine.remove_rule("nope").is_err());
    }

    #[test]
    fn rule_mark_duplicate() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "dup".into(),
            name: "Dup detector".into(),
            condition: RuleCondition::TitleContains("duplicate".into()),
            action: RuleAction::MarkDuplicate("42".into()),
            priority: 10,
            enabled: true,
        });
        let issue = make_issue("d1", "This is a duplicate", "");
        let result = engine.triage_issue(&issue);
        assert_eq!(result.duplicate_of, Some("42".into()));
    }

    #[test]
    fn rule_assign_to() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "assign".into(),
            name: "Assign frontend".into(),
            condition: RuleCondition::TitleContains("ui".into()),
            action: RuleAction::AssignTo("bob".into()),
            priority: 10,
            enabled: true,
        });
        let issue = make_issue("a1", "UI glitch", "");
        let result = engine.triage_issue(&issue);
        assert_eq!(result.assigned_to, Some("bob".into()));
    }

    // -- Related file detection ----------------------------------------------

    #[test]
    fn find_related_files_by_filename() {
        let engine = default_engine();
        let issue = make_issue("f1", "Bug in config.rs", "See also main.rs");
        let known = vec![
            "src/config.rs".to_string(),
            "src/main.rs".to_string(),
            "src/util.rs".to_string(),
        ];
        let related = engine.find_related_files(&issue, &known);
        assert!(related.contains(&"src/config.rs".to_string()));
        assert!(related.contains(&"src/main.rs".to_string()));
        assert!(!related.contains(&"src/util.rs".to_string()));
    }

    #[test]
    fn find_related_files_empty_known() {
        let engine = default_engine();
        let issue = make_issue("f2", "Anything", "");
        let related = engine.find_related_files(&issue, &[]);
        assert!(related.is_empty());
    }

    #[test]
    fn find_related_files_no_match() {
        let engine = default_engine();
        let issue = make_issue("f3", "No file refs", "Nothing");
        let known = vec!["src/obscure_module.rs".to_string()];
        let related = engine.find_related_files(&issue, &known);
        assert!(related.is_empty());
    }

    // -- Response generation -------------------------------------------------

    #[test]
    fn generate_response_bug() {
        let engine = default_engine();
        let issue = make_issue("g1", "Crash", "");
        let result = TriageResult {
            issue_id: "g1".into(),
            classified_type: IssueType::Bug,
            severity: IssueSeverity::Critical,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.8,
            draft_response: None,
            assigned_to: None,
            duplicate_of: None,
        };
        let resp = engine.generate_response(&issue, &result);
        assert!(resp.contains("bug report"));
        assert!(resp.contains("high priority"));
        assert!(resp.contains("@alice"));
    }

    #[test]
    fn generate_response_feature() {
        let engine = default_engine();
        let issue = make_issue("g2", "New feature", "");
        let result = TriageResult {
            issue_id: "g2".into(),
            classified_type: IssueType::FeatureRequest,
            severity: IssueSeverity::Low,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.6,
            draft_response: None,
            assigned_to: Some("carol".into()),
            duplicate_of: None,
        };
        let resp = engine.generate_response(&issue, &result);
        assert!(resp.contains("feature request"));
        assert!(resp.contains("@carol"));
    }

    #[test]
    fn generate_response_duplicate_note() {
        let engine = default_engine();
        let issue = make_issue("g3", "Dup", "");
        let result = TriageResult {
            issue_id: "g3".into(),
            classified_type: IssueType::Duplicate,
            severity: IssueSeverity::Low,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.9,
            draft_response: None,
            assigned_to: None,
            duplicate_of: Some("99".into()),
        };
        let resp = engine.generate_response(&issue, &result);
        assert!(resp.contains("duplicate"));
        assert!(resp.contains("#99"));
    }

    // -- Batch triage --------------------------------------------------------

    #[test]
    fn triage_batch_returns_correct_count() {
        let mut engine = default_engine();
        let issues = vec![
            make_issue("b1", "Crash", "error"),
            make_issue("b2", "Feature request", "add dark mode"),
            make_issue("b3", "How to use?", "question"),
        ];
        let results = engine.triage_batch(&issues);
        assert_eq!(results.len(), 3);
        assert_eq!(engine.metrics.total_triaged, 3);
    }

    #[test]
    fn triage_batch_empty() {
        let mut engine = default_engine();
        let results = engine.triage_batch(&[]);
        assert!(results.is_empty());
        assert_eq!(engine.metrics.total_triaged, 0);
    }

    // -- Learning from corrections -------------------------------------------

    #[test]
    fn learning_record_correction() {
        let mut learning = TriageLearning::new();
        let result = TriageResult {
            issue_id: "l1".into(),
            classified_type: IssueType::Bug,
            severity: IssueSeverity::Medium,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.7,
            draft_response: None,
            assigned_to: None,
            duplicate_of: None,
        };
        learning.record_correction("l1", &result, IssueType::FeatureRequest, IssueSeverity::Low);
        assert_eq!(learning.corrections.len(), 1);
        assert_eq!(learning.corrections[0].corrected_type, IssueType::FeatureRequest);
    }

    #[test]
    fn learning_correct_prediction_increases_accuracy() {
        let mut learning = TriageLearning::new();
        let result = TriageResult {
            issue_id: "l2".into(),
            classified_type: IssueType::Bug,
            severity: IssueSeverity::High,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.9,
            draft_response: None,
            assigned_to: None,
            duplicate_of: None,
        };
        // Correct prediction
        learning.record_correction("l2", &result, IssueType::Bug, IssueSeverity::High);
        assert_eq!(learning.get_accuracy(&IssueType::Bug), 1.0);
    }

    #[test]
    fn learning_wrong_prediction_lowers_accuracy() {
        let mut learning = TriageLearning::new();
        let result = TriageResult {
            issue_id: "l3".into(),
            classified_type: IssueType::Bug,
            severity: IssueSeverity::Medium,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.5,
            draft_response: None,
            assigned_to: None,
            duplicate_of: None,
        };
        learning.record_correction("l3", &result, IssueType::Question, IssueSeverity::Low);
        assert_eq!(learning.get_accuracy(&IssueType::Bug), 0.0);
    }

    #[test]
    fn learning_accuracy_no_data_returns_one() {
        let learning = TriageLearning::new();
        assert_eq!(learning.get_accuracy(&IssueType::Chore), 1.0);
    }

    #[test]
    fn learning_get_corrections() {
        let mut learning = TriageLearning::new();
        assert!(learning.get_corrections().is_empty());
        let result = TriageResult {
            issue_id: "c".into(),
            classified_type: IssueType::Enhancement,
            severity: IssueSeverity::Low,
            suggested_labels: vec![],
            related_files: vec![],
            confidence: 0.3,
            draft_response: None,
            assigned_to: None,
            duplicate_of: None,
        };
        learning.record_correction("c", &result, IssueType::Chore, IssueSeverity::Trivial);
        assert_eq!(learning.get_corrections().len(), 1);
    }

    // -- Metrics -------------------------------------------------------------

    #[test]
    fn metrics_updated_after_triage() {
        let mut engine = default_engine();
        engine.triage_issue(&make_issue("m1", "Bug: crash", "error"));
        assert_eq!(engine.metrics.total_triaged, 1);
        assert!(engine.metrics.avg_confidence > 0.0);
        assert!(!engine.metrics.by_type.is_empty());
        assert!(!engine.metrics.by_severity.is_empty());
    }

    #[test]
    fn metrics_avg_confidence_across_multiple() {
        let mut engine = default_engine();
        engine.triage_issue(&make_issue("m2", "Bug crash error", ""));
        engine.triage_issue(&make_issue("m3", "Vague thing", "nothing"));
        assert_eq!(engine.metrics.total_triaged, 2);
        // Avg should be between the two individual confidences.
        assert!(engine.metrics.avg_confidence > 0.0);
        assert!(engine.metrics.avg_confidence < 1.0);
    }

    // -- Edge cases ----------------------------------------------------------

    #[test]
    fn empty_issue_triaged() {
        let mut engine = default_engine();
        let issue = make_issue("e1", "", "");
        let result = engine.triage_issue(&issue);
        assert_eq!(result.issue_id, "e1");
        // Confidence should be very low for empty issues.
        assert!(result.confidence <= 0.2);
    }

    #[test]
    fn no_matching_rules() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "specific".into(),
            name: "Very specific".into(),
            condition: RuleCondition::TitleContains("xyzzy42plugh".into()),
            action: RuleAction::AddLabel("matched".into()),
            priority: 1,
            enabled: true,
        });
        let issue = make_issue("e2", "Normal issue", "body");
        let actions = engine.apply_rules(&issue);
        assert!(actions.is_empty());
    }

    #[test]
    fn low_confidence_triage() {
        let config = TriageConfig {
            min_confidence: 0.9,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        let issue = make_issue("e3", "ambiguous", "");
        let result = engine.triage_issue(&issue);
        // The result still exists but confidence is below threshold.
        assert!(result.confidence < 0.9);
    }

    #[test]
    fn triage_with_auto_assign() {
        let mut assignees = HashMap::new();
        assignees.insert("Bug".to_string(), "bugfixer".to_string());
        let config = TriageConfig {
            auto_assign: true,
            default_assignees: assignees,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        let issue = make_issue("aa1", "Bug crash error", "Broken");
        let result = engine.triage_issue(&issue);
        assert_eq!(result.assigned_to, Some("bugfixer".to_string()));
    }

    #[test]
    fn triage_with_auto_respond() {
        let config = TriageConfig {
            auto_respond: true,
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        let issue = make_issue("ar1", "Bug crash", "Error");
        let result = engine.triage_issue(&issue);
        assert!(result.draft_response.is_some());
    }

    #[test]
    fn history_accumulates() {
        let mut engine = default_engine();
        assert!(engine.get_history().is_empty());
        engine.triage_issue(&make_issue("h1", "A", ""));
        engine.triage_issue(&make_issue("h2", "B", ""));
        assert_eq!(engine.get_history().len(), 2);
    }

    #[test]
    fn triage_with_known_files() {
        let config = TriageConfig {
            known_files: vec!["src/server.rs".to_string(), "src/client.rs".to_string()],
            ..Default::default()
        };
        let mut engine = TriageEngine::new(config);
        let issue = make_issue("kf1", "Bug in server.rs", "");
        let result = engine.triage_issue(&issue);
        assert!(result.related_files.contains(&"src/server.rs".to_string()));
        assert!(!result.related_files.contains(&"src/client.rs".to_string()));
    }

    #[test]
    fn issue_source_variants() {
        for source in [
            IssueSource::GitHub,
            IssueSource::GitLab,
            IssueSource::Linear,
            IssueSource::Jira,
            IssueSource::Manual,
        ] {
            let mut issue = make_issue("src", "Title", "Body");
            issue.source = source.clone();
            let engine = default_engine();
            let (_, _) = engine.classify_type(&issue);
        }
    }

    #[test]
    fn rule_set_type_overrides_classification() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "override".into(),
            name: "Force chore".into(),
            condition: RuleCondition::TitleContains("ci".into()),
            action: RuleAction::SetType(IssueType::Chore),
            priority: 50,
            enabled: true,
        });
        let issue = make_issue("ov1", "CI pipeline broken", "error crash fail");
        let result = engine.triage_issue(&issue);
        // Rule should override the Bug classification.
        assert_eq!(result.classified_type, IssueType::Chore);
    }

    #[test]
    fn case_insensitive_rule_matching() {
        let mut engine = default_engine();
        engine.add_rule(TriageRule {
            id: "ci".into(),
            name: "Case test".into(),
            condition: RuleCondition::TitleContains("URGENT".into()),
            action: RuleAction::SetSeverity(IssueSeverity::Critical),
            priority: 10,
            enabled: true,
        });
        let issue = make_issue("ci1", "urgent issue", "");
        let actions = engine.apply_rules(&issue);
        assert_eq!(actions.len(), 1);
    }
}
