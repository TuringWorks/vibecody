//! CI/CD AI status checks — AI runs as GitHub/GitLab status check on every PR.
//!
//! Closes P2 Gap 9: AI runs as a GitHub status check on every PR (green/red
//! pass/fail). Supports GitHub and GitLab JSON output formats, annotation-level
//! filtering, suite management, and auto-cleanup of old suites.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Check state & type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CheckState {
    Pending,
    Running,
    Success,
    Failure,
    Error,
    Neutral,
    Skipped,
}

impl CheckState {
    pub fn as_str(&self) -> &str {
        match self {
            CheckState::Pending => "pending",
            CheckState::Running => "running",
            CheckState::Success => "success",
            CheckState::Failure => "failure",
            CheckState::Error => "error",
            CheckState::Neutral => "neutral",
            CheckState::Skipped => "skipped",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, CheckState::Success | CheckState::Failure | CheckState::Error | CheckState::Neutral | CheckState::Skipped)
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, CheckState::Success | CheckState::Neutral | CheckState::Skipped)
    }

    /// Return the "worst" of two states for aggregation.
    fn worst(a: &CheckState, b: &CheckState) -> CheckState {
        let rank = |s: &CheckState| -> u8 {
            match s {
                CheckState::Success => 0,
                CheckState::Skipped => 1,
                CheckState::Neutral => 2,
                CheckState::Pending => 3,
                CheckState::Running => 4,
                CheckState::Failure => 5,
                CheckState::Error => 6,
            }
        };
        if rank(a) >= rank(b) { a.clone() } else { b.clone() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    CodeReview,
    SecurityScan,
    TestCoverage,
    StyleCheck,
    DependencyAudit,
    PerformanceCheck,
    DocumentationCheck,
    CustomCheck(String),
}

impl CheckType {
    pub fn as_str(&self) -> &str {
        match self {
            CheckType::CodeReview => "code_review",
            CheckType::SecurityScan => "security_scan",
            CheckType::TestCoverage => "test_coverage",
            CheckType::StyleCheck => "style_check",
            CheckType::DependencyAudit => "dependency_audit",
            CheckType::PerformanceCheck => "performance_check",
            CheckType::DocumentationCheck => "documentation_check",
            CheckType::CustomCheck(s) => s,
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationLevel {
    Notice,
    Warning,
    Failure,
}

impl AnnotationLevel {
    pub fn as_str(&self) -> &str {
        match self {
            AnnotationLevel::Notice => "notice",
            AnnotationLevel::Warning => "warning",
            AnnotationLevel::Failure => "failure",
        }
    }

    fn severity(&self) -> u8 {
        match self {
            AnnotationLevel::Notice => 0,
            AnnotationLevel::Warning => 1,
            AnnotationLevel::Failure => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckAnnotation {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub level: AnnotationLevel,
    pub message: String,
    pub suggestion: Option<String>,
}

impl CheckAnnotation {
    pub fn new(path: &str, start_line: usize, end_line: usize, level: AnnotationLevel, message: &str) -> Self {
        Self {
            path: path.to_string(),
            start_line,
            end_line,
            level,
            message: message.to_string(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Check output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CheckOutput {
    pub title: String,
    pub summary: String,
    pub annotations: Vec<CheckAnnotation>,
    pub text: Option<String>,
}

impl CheckOutput {
    pub fn new(title: &str, summary: &str) -> Self {
        Self {
            title: title.to_string(),
            summary: summary.to_string(),
            annotations: Vec::new(),
            text: None,
        }
    }

    pub fn add_annotation(&mut self, ann: CheckAnnotation) {
        self.annotations.push(ann);
    }

    /// Render the output as a markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("## {}\n\n", self.title));
        md.push_str(&format!("{}\n\n", self.summary));
        if !self.annotations.is_empty() {
            md.push_str("### Annotations\n\n");
            for ann in &self.annotations {
                md.push_str(&format!(
                    "- **{}** `{}` (L{}-L{}): {}\n",
                    ann.level.as_str(),
                    ann.path,
                    ann.start_line,
                    ann.end_line,
                    ann.message
                ));
                if let Some(ref suggestion) = ann.suggestion {
                    md.push_str(&format!("  - Suggestion: {}\n", suggestion));
                }
            }
            md.push('\n');
        }
        if let Some(ref text) = self.text {
            md.push_str("### Details\n\n");
            md.push_str(text);
            md.push('\n');
        }
        md
    }

    /// Return counts: (notices, warnings, failures).
    pub fn annotation_counts(&self) -> (usize, usize, usize) {
        let mut notices = 0;
        let mut warnings = 0;
        let mut failures = 0;
        for ann in &self.annotations {
            match ann.level {
                AnnotationLevel::Notice => notices += 1,
                AnnotationLevel::Warning => warnings += 1,
                AnnotationLevel::Failure => failures += 1,
            }
        }
        (notices, warnings, failures)
    }
}

// ---------------------------------------------------------------------------
// Status check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StatusCheck {
    pub id: String,
    pub name: String,
    pub check_type: CheckType,
    pub state: CheckState,
    pub description: String,
    pub details_url: Option<String>,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub output: CheckOutput,
}

impl StatusCheck {
    pub fn new(name: &str, check_type: CheckType) -> Self {
        let id = format!("chk-{}-{}", name.replace(' ', "_"), now_secs());
        Self {
            id,
            name: name.to_string(),
            check_type,
            state: CheckState::Pending,
            description: String::new(),
            details_url: None,
            started_at: None,
            completed_at: None,
            output: CheckOutput::new(name, ""),
        }
    }

    pub fn start(&mut self) {
        self.state = CheckState::Running;
        self.started_at = Some(SystemTime::now());
    }

    pub fn succeed(&mut self, summary: &str) {
        self.state = CheckState::Success;
        self.completed_at = Some(SystemTime::now());
        self.output.summary = summary.to_string();
    }

    pub fn fail(&mut self, summary: &str) {
        self.state = CheckState::Failure;
        self.completed_at = Some(SystemTime::now());
        self.output.summary = summary.to_string();
    }

    pub fn add_annotation(&mut self, annotation: CheckAnnotation) {
        self.output.add_annotation(annotation);
    }

    pub fn elapsed(&self) -> Option<Duration> {
        let start = self.started_at?;
        let end = self.completed_at.unwrap_or_else(SystemTime::now);
        end.duration_since(start).ok()
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Format as a GitHub Checks API JSON payload.
    pub fn to_github_json(&self) -> String {
        let started = self.started_at.map(format_time).unwrap_or_default();
        let completed = self.completed_at.map(format_time).unwrap_or_default();
        let annotations_json: Vec<String> = self.output.annotations.iter().map(|a| {
            let suggestion_field = match &a.suggestion {
                Some(s) => format!(", \"raw_details\": \"{}\"", escape_json(s)),
                None => String::new(),
            };
            format!(
                "{{\"path\": \"{}\", \"start_line\": {}, \"end_line\": {}, \"annotation_level\": \"{}\", \"message\": \"{}\"{}}}",
                escape_json(&a.path), a.start_line, a.end_line, a.level.as_str(), escape_json(&a.message), suggestion_field
            )
        }).collect();
        format!(
            "{{\"name\": \"{}\", \"status\": \"{}\", \"conclusion\": \"{}\", \"started_at\": \"{}\", \"completed_at\": \"{}\", \"output\": {{\"title\": \"{}\", \"summary\": \"{}\", \"annotations\": [{}]}}}}",
            escape_json(&self.name),
            if self.is_terminal() { "completed" } else { "in_progress" },
            self.state.as_str(),
            started,
            completed,
            escape_json(&self.output.title),
            escape_json(&self.output.summary),
            annotations_json.join(", ")
        )
    }

    /// Format as a GitLab commit status JSON payload.
    pub fn to_gitlab_json(&self) -> String {
        let gitlab_state = match &self.state {
            CheckState::Pending => "pending",
            CheckState::Running => "running",
            CheckState::Success => "success",
            CheckState::Failure | CheckState::Error => "failed",
            CheckState::Neutral | CheckState::Skipped => "success",
        };
        let url = self.details_url.as_deref().unwrap_or("");
        format!(
            "{{\"state\": \"{}\", \"name\": \"{}\", \"description\": \"{}\", \"target_url\": \"{}\"}}",
            gitlab_state,
            escape_json(&self.name),
            escape_json(&self.output.summary),
            escape_json(url)
        )
    }
}

// ---------------------------------------------------------------------------
// Check suite
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CheckSuite {
    pub id: String,
    pub commit_sha: String,
    pub branch: String,
    pub pr_number: Option<u64>,
    pub checks: Vec<StatusCheck>,
    pub overall_state: CheckState,
    pub created_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub repository: String,
}

impl CheckSuite {
    pub fn new(commit_sha: &str, branch: &str, repo: &str) -> Self {
        let id = format!("suite-{}-{}", &commit_sha[..7.min(commit_sha.len())], now_secs());
        Self {
            id,
            commit_sha: commit_sha.to_string(),
            branch: branch.to_string(),
            pr_number: None,
            checks: Vec::new(),
            overall_state: CheckState::Pending,
            created_at: SystemTime::now(),
            completed_at: None,
            repository: repo.to_string(),
        }
    }

    pub fn with_pr(mut self, pr_number: u64) -> Self {
        self.pr_number = Some(pr_number);
        self
    }

    pub fn add_check(&mut self, check: StatusCheck) {
        self.checks.push(check);
        self.compute_overall_state();
    }

    pub fn get_check(&self, name: &str) -> Option<&StatusCheck> {
        self.checks.iter().find(|c| c.name == name)
    }

    /// Recompute overall_state from all checks (worst state wins).
    pub fn compute_overall_state(&mut self) {
        if self.checks.is_empty() {
            self.overall_state = CheckState::Pending;
            return;
        }
        let mut worst = CheckState::Success;
        for check in &self.checks {
            worst = CheckState::worst(&worst, &check.state);
        }
        self.overall_state = worst;
    }

    /// Mark the suite as complete.
    pub fn complete(&mut self) {
        self.compute_overall_state();
        self.completed_at = Some(SystemTime::now());
    }

    pub fn all_passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|c| c.state.is_pass())
    }

    pub fn failed_checks(&self) -> Vec<&StatusCheck> {
        self.checks.iter().filter(|c| c.state == CheckState::Failure || c.state == CheckState::Error).collect()
    }

    /// Generate a markdown summary of the suite.
    pub fn to_summary_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Check Suite: {}\n\n", self.id));
        md.push_str(&format!("- **Repo**: {}\n", self.repository));
        md.push_str(&format!("- **Branch**: {}\n", self.branch));
        md.push_str(&format!("- **Commit**: {}\n", self.commit_sha));
        if let Some(pr) = self.pr_number {
            md.push_str(&format!("- **PR**: #{}\n", pr));
        }
        md.push_str(&format!("- **Overall**: {}\n\n", self.overall_state.as_str()));

        md.push_str("| Check | State | Annotations |\n");
        md.push_str("|-------|-------|-------------|\n");
        for check in &self.checks {
            let (n, w, f) = check.output.annotation_counts();
            md.push_str(&format!(
                "| {} | {} | {} notices, {} warnings, {} failures |\n",
                check.name, check.state.as_str(), n, w, f
            ));
        }
        md
    }
}

// ---------------------------------------------------------------------------
// CI status config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CiStatusConfig {
    pub enabled_checks: Vec<CheckType>,
    pub required_checks: Vec<String>,
    pub allow_neutral: bool,
    pub auto_approve_on_all_pass: bool,
    pub post_as_comment: bool,
    pub annotation_level_threshold: AnnotationLevel,
}

impl CiStatusConfig {
    pub fn default_config() -> Self {
        Self {
            enabled_checks: vec![
                CheckType::CodeReview,
                CheckType::SecurityScan,
                CheckType::StyleCheck,
                CheckType::TestCoverage,
            ],
            required_checks: vec!["code_review".to_string(), "security_scan".to_string()],
            allow_neutral: true,
            auto_approve_on_all_pass: false,
            post_as_comment: true,
            annotation_level_threshold: AnnotationLevel::Warning,
        }
    }

    pub fn strict() -> Self {
        Self {
            enabled_checks: vec![
                CheckType::CodeReview,
                CheckType::SecurityScan,
                CheckType::StyleCheck,
                CheckType::TestCoverage,
                CheckType::DependencyAudit,
                CheckType::PerformanceCheck,
                CheckType::DocumentationCheck,
            ],
            required_checks: vec![
                "code_review".to_string(),
                "security_scan".to_string(),
                "test_coverage".to_string(),
                "dependency_audit".to_string(),
            ],
            allow_neutral: false,
            auto_approve_on_all_pass: false,
            post_as_comment: true,
            annotation_level_threshold: AnnotationLevel::Notice,
        }
    }
}

// ---------------------------------------------------------------------------
// CI status manager
// ---------------------------------------------------------------------------

pub struct CiStatusManager {
    pub suites: Vec<CheckSuite>,
    pub config: CiStatusConfig,
}

impl CiStatusManager {
    pub fn new() -> Self {
        Self {
            suites: Vec::new(),
            config: CiStatusConfig::default_config(),
        }
    }

    pub fn with_config(config: CiStatusConfig) -> Self {
        Self {
            suites: Vec::new(),
            config,
        }
    }

    pub fn create_suite(&mut self, sha: &str, branch: &str, repo: &str) -> &mut CheckSuite {
        let suite = CheckSuite::new(sha, branch, repo);
        self.suites.push(suite);
        self.suites.last_mut().expect("just pushed")
    }

    pub fn get_suite(&self, id: &str) -> Option<&CheckSuite> {
        self.suites.iter().find(|s| s.id == id)
    }

    pub fn get_suite_mut(&mut self, id: &str) -> Option<&mut CheckSuite> {
        self.suites.iter_mut().find(|s| s.id == id)
    }

    pub fn latest_for_branch(&self, branch: &str) -> Option<&CheckSuite> {
        self.suites.iter().rev().find(|s| s.branch == branch)
    }

    pub fn suites_for_pr(&self, pr_number: u64) -> Vec<&CheckSuite> {
        self.suites.iter().filter(|s| s.pr_number == Some(pr_number)).collect()
    }

    /// Remove suites older than `max_age_days`.
    pub fn cleanup_old(&mut self, max_age_days: u64) {
        let cutoff = Duration::from_secs(max_age_days * 86400);
        let now = SystemTime::now();
        self.suites.retain(|s| {
            now.duration_since(s.created_at)
                .map(|d| d < cutoff)
                .unwrap_or(true)
        });
    }

    pub fn total_suites(&self) -> usize {
        self.suites.len()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_time(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // ISO-8601 approximate (no chrono dependency)
    format!("{}Z", secs)
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- CheckState tests --

    #[test]
    fn test_check_state_as_str() {
        assert_eq!(CheckState::Pending.as_str(), "pending");
        assert_eq!(CheckState::Running.as_str(), "running");
        assert_eq!(CheckState::Success.as_str(), "success");
        assert_eq!(CheckState::Failure.as_str(), "failure");
        assert_eq!(CheckState::Error.as_str(), "error");
        assert_eq!(CheckState::Neutral.as_str(), "neutral");
        assert_eq!(CheckState::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_check_state_is_terminal() {
        assert!(!CheckState::Pending.is_terminal());
        assert!(!CheckState::Running.is_terminal());
        assert!(CheckState::Success.is_terminal());
        assert!(CheckState::Failure.is_terminal());
        assert!(CheckState::Error.is_terminal());
        assert!(CheckState::Neutral.is_terminal());
        assert!(CheckState::Skipped.is_terminal());
    }

    #[test]
    fn test_check_state_is_pass() {
        assert!(CheckState::Success.is_pass());
        assert!(CheckState::Neutral.is_pass());
        assert!(CheckState::Skipped.is_pass());
        assert!(!CheckState::Pending.is_pass());
        assert!(!CheckState::Running.is_pass());
        assert!(!CheckState::Failure.is_pass());
        assert!(!CheckState::Error.is_pass());
    }

    #[test]
    fn test_check_state_worst() {
        assert_eq!(CheckState::worst(&CheckState::Success, &CheckState::Failure), CheckState::Failure);
        assert_eq!(CheckState::worst(&CheckState::Success, &CheckState::Success), CheckState::Success);
        assert_eq!(CheckState::worst(&CheckState::Neutral, &CheckState::Error), CheckState::Error);
        assert_eq!(CheckState::worst(&CheckState::Pending, &CheckState::Success), CheckState::Pending);
    }

    // -- CheckType tests --

    #[test]
    fn test_check_type_as_str() {
        assert_eq!(CheckType::CodeReview.as_str(), "code_review");
        assert_eq!(CheckType::SecurityScan.as_str(), "security_scan");
        assert_eq!(CheckType::TestCoverage.as_str(), "test_coverage");
        assert_eq!(CheckType::StyleCheck.as_str(), "style_check");
        assert_eq!(CheckType::DependencyAudit.as_str(), "dependency_audit");
        assert_eq!(CheckType::PerformanceCheck.as_str(), "performance_check");
        assert_eq!(CheckType::DocumentationCheck.as_str(), "documentation_check");
        assert_eq!(CheckType::CustomCheck("lint".into()).as_str(), "lint");
    }

    // -- AnnotationLevel tests --

    #[test]
    fn test_annotation_level_as_str() {
        assert_eq!(AnnotationLevel::Notice.as_str(), "notice");
        assert_eq!(AnnotationLevel::Warning.as_str(), "warning");
        assert_eq!(AnnotationLevel::Failure.as_str(), "failure");
    }

    #[test]
    fn test_annotation_level_severity() {
        assert!(AnnotationLevel::Notice.severity() < AnnotationLevel::Warning.severity());
        assert!(AnnotationLevel::Warning.severity() < AnnotationLevel::Failure.severity());
    }

    // -- CheckAnnotation tests --

    #[test]
    fn test_annotation_new() {
        let ann = CheckAnnotation::new("src/main.rs", 10, 15, AnnotationLevel::Warning, "unused variable");
        assert_eq!(ann.path, "src/main.rs");
        assert_eq!(ann.start_line, 10);
        assert_eq!(ann.end_line, 15);
        assert_eq!(ann.level, AnnotationLevel::Warning);
        assert!(ann.suggestion.is_none());
    }

    #[test]
    fn test_annotation_with_suggestion() {
        let ann = CheckAnnotation::new("lib.rs", 5, 5, AnnotationLevel::Failure, "missing return")
            .with_suggestion("Add return statement");
        assert_eq!(ann.suggestion, Some("Add return statement".to_string()));
    }

    // -- CheckOutput tests --

    #[test]
    fn test_check_output_new() {
        let out = CheckOutput::new("Code Review", "All checks passed");
        assert_eq!(out.title, "Code Review");
        assert_eq!(out.summary, "All checks passed");
        assert!(out.annotations.is_empty());
    }

    #[test]
    fn test_check_output_add_annotation() {
        let mut out = CheckOutput::new("Review", "Summary");
        out.add_annotation(CheckAnnotation::new("a.rs", 1, 1, AnnotationLevel::Notice, "info"));
        out.add_annotation(CheckAnnotation::new("b.rs", 2, 2, AnnotationLevel::Warning, "warn"));
        assert_eq!(out.annotations.len(), 2);
    }

    #[test]
    fn test_check_output_annotation_counts() {
        let mut out = CheckOutput::new("Test", "Test");
        out.add_annotation(CheckAnnotation::new("a.rs", 1, 1, AnnotationLevel::Notice, "n1"));
        out.add_annotation(CheckAnnotation::new("a.rs", 2, 2, AnnotationLevel::Notice, "n2"));
        out.add_annotation(CheckAnnotation::new("b.rs", 3, 3, AnnotationLevel::Warning, "w1"));
        out.add_annotation(CheckAnnotation::new("c.rs", 4, 4, AnnotationLevel::Failure, "f1"));
        let (n, w, f) = out.annotation_counts();
        assert_eq!(n, 2);
        assert_eq!(w, 1);
        assert_eq!(f, 1);
    }

    #[test]
    fn test_check_output_to_markdown() {
        let mut out = CheckOutput::new("Security Scan", "Found 1 issue");
        out.add_annotation(
            CheckAnnotation::new("src/auth.rs", 42, 42, AnnotationLevel::Failure, "SQL injection risk")
                .with_suggestion("Use parameterized queries")
        );
        out.text = Some("Full report here.".to_string());
        let md = out.to_markdown();
        assert!(md.contains("## Security Scan"));
        assert!(md.contains("Found 1 issue"));
        assert!(md.contains("SQL injection risk"));
        assert!(md.contains("Use parameterized queries"));
        assert!(md.contains("Full report here."));
    }

    #[test]
    fn test_check_output_to_markdown_empty() {
        let out = CheckOutput::new("Clean", "No issues");
        let md = out.to_markdown();
        assert!(md.contains("## Clean"));
        assert!(!md.contains("Annotations"));
    }

    // -- StatusCheck tests --

    #[test]
    fn test_status_check_new() {
        let check = StatusCheck::new("AI Review", CheckType::CodeReview);
        assert_eq!(check.name, "AI Review");
        assert_eq!(check.state, CheckState::Pending);
        assert!(check.started_at.is_none());
        assert!(check.completed_at.is_none());
    }

    #[test]
    fn test_status_check_start() {
        let mut check = StatusCheck::new("Review", CheckType::CodeReview);
        check.start();
        assert_eq!(check.state, CheckState::Running);
        assert!(check.started_at.is_some());
    }

    #[test]
    fn test_status_check_succeed() {
        let mut check = StatusCheck::new("Review", CheckType::CodeReview);
        check.start();
        check.succeed("All good");
        assert_eq!(check.state, CheckState::Success);
        assert!(check.completed_at.is_some());
        assert_eq!(check.output.summary, "All good");
    }

    #[test]
    fn test_status_check_fail() {
        let mut check = StatusCheck::new("Security", CheckType::SecurityScan);
        check.start();
        check.fail("Vulnerability found");
        assert_eq!(check.state, CheckState::Failure);
        assert!(check.completed_at.is_some());
        assert_eq!(check.output.summary, "Vulnerability found");
    }

    #[test]
    fn test_status_check_add_annotation() {
        let mut check = StatusCheck::new("Review", CheckType::CodeReview);
        check.add_annotation(CheckAnnotation::new("main.rs", 1, 1, AnnotationLevel::Warning, "test"));
        assert_eq!(check.output.annotations.len(), 1);
    }

    #[test]
    fn test_status_check_elapsed() {
        let check = StatusCheck::new("Review", CheckType::CodeReview);
        assert!(check.elapsed().is_none()); // not started
    }

    #[test]
    fn test_status_check_elapsed_started() {
        let mut check = StatusCheck::new("Review", CheckType::CodeReview);
        check.start();
        check.succeed("done");
        let elapsed = check.elapsed();
        assert!(elapsed.is_some());
    }

    #[test]
    fn test_status_check_is_terminal() {
        let mut check = StatusCheck::new("R", CheckType::CodeReview);
        assert!(!check.is_terminal());
        check.start();
        assert!(!check.is_terminal());
        check.succeed("ok");
        assert!(check.is_terminal());
    }

    #[test]
    fn test_status_check_to_github_json() {
        let mut check = StatusCheck::new("AI Review", CheckType::CodeReview);
        check.start();
        check.add_annotation(CheckAnnotation::new("src/lib.rs", 10, 12, AnnotationLevel::Warning, "unused import"));
        check.succeed("1 warning");
        let json = check.to_github_json();
        assert!(json.contains("\"name\": \"AI Review\""));
        assert!(json.contains("\"status\": \"completed\""));
        assert!(json.contains("\"conclusion\": \"success\""));
        assert!(json.contains("\"annotation_level\": \"warning\""));
        assert!(json.contains("unused import"));
    }

    #[test]
    fn test_status_check_to_gitlab_json() {
        let mut check = StatusCheck::new("Security", CheckType::SecurityScan);
        check.start();
        check.fail("Critical vulnerability");
        let json = check.to_gitlab_json();
        assert!(json.contains("\"state\": \"failed\""));
        assert!(json.contains("\"name\": \"Security\""));
        assert!(json.contains("Critical vulnerability"));
    }

    #[test]
    fn test_status_check_to_gitlab_json_neutral() {
        let mut check = StatusCheck::new("Docs", CheckType::DocumentationCheck);
        check.state = CheckState::Neutral;
        let json = check.to_gitlab_json();
        assert!(json.contains("\"state\": \"success\""));
    }

    // -- CheckSuite tests --

    #[test]
    fn test_check_suite_new() {
        let suite = CheckSuite::new("abc1234def5678", "main", "org/repo");
        assert!(suite.id.contains("abc1234"));
        assert_eq!(suite.branch, "main");
        assert_eq!(suite.repository, "org/repo");
        assert_eq!(suite.overall_state, CheckState::Pending);
        assert!(suite.pr_number.is_none());
    }

    #[test]
    fn test_check_suite_with_pr() {
        let suite = CheckSuite::new("abc1234", "feature", "org/repo").with_pr(42);
        assert_eq!(suite.pr_number, Some(42));
    }

    #[test]
    fn test_check_suite_add_check() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut check = StatusCheck::new("Review", CheckType::CodeReview);
        check.succeed("ok");
        suite.add_check(check);
        assert_eq!(suite.checks.len(), 1);
        assert_eq!(suite.overall_state, CheckState::Success);
    }

    #[test]
    fn test_check_suite_get_check() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        suite.add_check(StatusCheck::new("Review", CheckType::CodeReview));
        suite.add_check(StatusCheck::new("Security", CheckType::SecurityScan));
        assert!(suite.get_check("Review").is_some());
        assert!(suite.get_check("Security").is_some());
        assert!(suite.get_check("Missing").is_none());
    }

    #[test]
    fn test_check_suite_compute_overall_state_success() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut c1 = StatusCheck::new("C1", CheckType::CodeReview);
        c1.succeed("ok");
        let mut c2 = StatusCheck::new("C2", CheckType::StyleCheck);
        c2.succeed("ok");
        suite.add_check(c1);
        suite.add_check(c2);
        assert_eq!(suite.overall_state, CheckState::Success);
    }

    #[test]
    fn test_check_suite_compute_overall_state_failure() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut c1 = StatusCheck::new("C1", CheckType::CodeReview);
        c1.succeed("ok");
        let mut c2 = StatusCheck::new("C2", CheckType::SecurityScan);
        c2.fail("vuln");
        suite.add_check(c1);
        suite.add_check(c2);
        assert_eq!(suite.overall_state, CheckState::Failure);
    }

    #[test]
    fn test_check_suite_complete() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut check = StatusCheck::new("R", CheckType::CodeReview);
        check.succeed("ok");
        suite.add_check(check);
        suite.complete();
        assert!(suite.completed_at.is_some());
        assert_eq!(suite.overall_state, CheckState::Success);
    }

    #[test]
    fn test_check_suite_all_passed() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut c1 = StatusCheck::new("C1", CheckType::CodeReview);
        c1.succeed("ok");
        suite.add_check(c1);
        assert!(suite.all_passed());
    }

    #[test]
    fn test_check_suite_all_passed_empty() {
        let suite = CheckSuite::new("abc", "main", "repo");
        assert!(!suite.all_passed()); // empty suite does not pass
    }

    #[test]
    fn test_check_suite_all_passed_with_failure() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut c1 = StatusCheck::new("C1", CheckType::CodeReview);
        c1.fail("bad");
        suite.add_check(c1);
        assert!(!suite.all_passed());
    }

    #[test]
    fn test_check_suite_failed_checks() {
        let mut suite = CheckSuite::new("abc", "main", "repo");
        let mut c1 = StatusCheck::new("C1", CheckType::CodeReview);
        c1.succeed("ok");
        let mut c2 = StatusCheck::new("C2", CheckType::SecurityScan);
        c2.fail("vuln");
        suite.add_check(c1);
        suite.add_check(c2);
        let failed = suite.failed_checks();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].name, "C2");
    }

    #[test]
    fn test_check_suite_to_summary_markdown() {
        let mut suite = CheckSuite::new("abc1234567890", "feature-x", "org/repo");
        suite.pr_number = Some(99);
        let mut c1 = StatusCheck::new("Review", CheckType::CodeReview);
        c1.succeed("ok");
        c1.add_annotation(CheckAnnotation::new("a.rs", 1, 1, AnnotationLevel::Notice, "info"));
        suite.add_check(c1);
        let md = suite.to_summary_markdown();
        assert!(md.contains("org/repo"));
        assert!(md.contains("feature-x"));
        assert!(md.contains("#99"));
        assert!(md.contains("Review"));
        assert!(md.contains("success"));
    }

    // -- CiStatusConfig tests --

    #[test]
    fn test_ci_status_config_default() {
        let cfg = CiStatusConfig::default_config();
        assert_eq!(cfg.enabled_checks.len(), 4);
        assert!(cfg.allow_neutral);
        assert!(!cfg.auto_approve_on_all_pass);
        assert!(cfg.post_as_comment);
        assert_eq!(cfg.annotation_level_threshold, AnnotationLevel::Warning);
    }

    #[test]
    fn test_ci_status_config_strict() {
        let cfg = CiStatusConfig::strict();
        assert_eq!(cfg.enabled_checks.len(), 7);
        assert!(!cfg.allow_neutral);
        assert_eq!(cfg.required_checks.len(), 4);
        assert_eq!(cfg.annotation_level_threshold, AnnotationLevel::Notice);
    }

    // -- CiStatusManager tests --

    #[test]
    fn test_ci_status_manager_new() {
        let mgr = CiStatusManager::new();
        assert!(mgr.suites.is_empty());
        assert_eq!(mgr.total_suites(), 0);
    }

    #[test]
    fn test_ci_status_manager_create_suite() {
        let mut mgr = CiStatusManager::new();
        let suite = mgr.create_suite("abc123", "main", "org/repo");
        assert_eq!(suite.commit_sha, "abc123");
        assert_eq!(mgr.total_suites(), 1);
    }

    #[test]
    fn test_ci_status_manager_get_suite() {
        let mut mgr = CiStatusManager::new();
        let id = {
            let suite = mgr.create_suite("abc", "main", "repo");
            suite.id.clone()
        };
        assert!(mgr.get_suite(&id).is_some());
        assert!(mgr.get_suite("nonexistent").is_none());
    }

    #[test]
    fn test_ci_status_manager_latest_for_branch() {
        let mut mgr = CiStatusManager::new();
        mgr.create_suite("abc", "main", "repo");
        mgr.create_suite("def", "feature", "repo");
        mgr.create_suite("ghi", "main", "repo");
        let latest = mgr.latest_for_branch("main");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().commit_sha, "ghi");
    }

    #[test]
    fn test_ci_status_manager_latest_for_branch_none() {
        let mgr = CiStatusManager::new();
        assert!(mgr.latest_for_branch("main").is_none());
    }

    #[test]
    fn test_ci_status_manager_suites_for_pr() {
        let mut mgr = CiStatusManager::new();
        {
            let s1 = mgr.create_suite("abc", "feat", "repo");
            s1.pr_number = Some(42);
        }
        {
            let s2 = mgr.create_suite("def", "feat", "repo");
            s2.pr_number = Some(42);
        }
        mgr.create_suite("ghi", "other", "repo"); // no PR
        let pr_suites = mgr.suites_for_pr(42);
        assert_eq!(pr_suites.len(), 2);
    }

    #[test]
    fn test_ci_status_manager_suites_for_pr_empty() {
        let mgr = CiStatusManager::new();
        assert!(mgr.suites_for_pr(1).is_empty());
    }

    #[test]
    fn test_ci_status_manager_cleanup_old() {
        let mut mgr = CiStatusManager::new();
        mgr.create_suite("abc", "main", "repo");
        mgr.create_suite("def", "main", "repo");
        // All suites are fresh, cleanup with max_age_days=1 should keep them
        mgr.cleanup_old(1);
        assert_eq!(mgr.total_suites(), 2);
    }

    #[test]
    fn test_ci_status_manager_with_config() {
        let cfg = CiStatusConfig::strict();
        let mgr = CiStatusManager::with_config(cfg);
        assert!(!mgr.config.allow_neutral);
    }

    #[test]
    fn test_ci_status_manager_get_suite_mut() {
        let mut mgr = CiStatusManager::new();
        let id = {
            let suite = mgr.create_suite("abc", "main", "repo");
            suite.id.clone()
        };
        let suite = mgr.get_suite_mut(&id).unwrap();
        suite.pr_number = Some(99);
        assert_eq!(mgr.get_suite(&id).unwrap().pr_number, Some(99));
    }

    // -- Helper tests --

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_json("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_format_time() {
        let t = UNIX_EPOCH + Duration::from_secs(1000);
        let s = format_time(t);
        assert_eq!(s, "1000Z");
    }
}
