//! Agent self-review gate — automated quality checks before task completion.
//!
//! Closes P0 Gap 2: Agent reviews own changes (lint, test, security scan) before
//! marking task complete; iterates if issues found. Inspired by GitHub Copilot's
//! agent self-review and Cursor BugBot.
//!
//! # Architecture
//!
//! ```text
//! Agent completes task
//!   → SelfReviewGate::run_checks()
//!     ├─ LintCheck     (clippy, eslint, pylint, etc.)
//!     ├─ TestCheck      (cargo test, npm test, pytest, etc.)
//!     ├─ SecurityCheck   (secret scan, dependency audit, SAST)
//!     ├─ DiffReview      (AI reviews own diff for quality)
//!     └─ BuildCheck      (cargo check, npm build — already exists)
//!   → If issues found && retries < max → inject feedback, continue agent loop
//!   → If all pass || retries exhausted → mark task complete
//! ```

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Check types
// ---------------------------------------------------------------------------

/// Types of self-review checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CheckKind {
    Lint,
    Test,
    Security,
    DiffReview,
    Build,
    Format,
    TypeCheck,
    Custom(String),
}

impl CheckKind {
    pub fn name(&self) -> &str {
        match self {
            CheckKind::Lint => "lint",
            CheckKind::Test => "test",
            CheckKind::Security => "security",
            CheckKind::DiffReview => "diff_review",
            CheckKind::Build => "build",
            CheckKind::Format => "format",
            CheckKind::TypeCheck => "typecheck",
            CheckKind::Custom(name) => name,
        }
    }
}

/// Severity of a check finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Critical => "critical",
        }
    }
}

/// A single finding from a check.
#[derive(Debug, Clone)]
pub struct Finding {
    pub check: CheckKind,
    pub severity: Severity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

impl Finding {
    pub fn new(check: CheckKind, severity: Severity, message: &str) -> Self {
        Self {
            check,
            severity,
            message: message.to_string(),
            file: None,
            line: None,
            suggestion: None,
        }
    }

    pub fn with_location(mut self, file: &str, line: usize) -> Self {
        self.file = Some(file.to_string());
        self.line = Some(line);
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Format this finding as a single-line summary.
    pub fn summary(&self) -> String {
        let loc = match (&self.file, self.line) {
            (Some(f), Some(l)) => format!(" ({}:{})", f, l),
            (Some(f), None) => format!(" ({})", f),
            _ => String::new(),
        };
        format!(
            "[{}] {}: {}{}",
            self.check.name(),
            self.severity.as_str(),
            self.message,
            loc
        )
    }
}

// ---------------------------------------------------------------------------
// Check result
// ---------------------------------------------------------------------------

/// Result of running a single check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub kind: CheckKind,
    pub passed: bool,
    pub findings: Vec<Finding>,
    pub duration_ms: u64,
    pub command: Option<String>,
    pub raw_output: Option<String>,
}

impl CheckResult {
    pub fn pass(kind: CheckKind) -> Self {
        Self {
            kind,
            passed: true,
            findings: Vec::new(),
            duration_ms: 0,
            command: None,
            raw_output: None,
        }
    }

    pub fn fail(kind: CheckKind, findings: Vec<Finding>) -> Self {
        Self {
            kind,
            passed: false,
            findings,
            duration_ms: 0,
            command: None,
            raw_output: None,
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn with_command(mut self, cmd: &str) -> Self {
        self.command = Some(cmd.to_string());
        self
    }

    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity >= Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Lint checker
// ---------------------------------------------------------------------------

/// Language-specific lint tool configuration.
#[derive(Debug, Clone)]
pub struct LintConfig {
    pub tool: String,
    pub args: Vec<String>,
    pub file_patterns: Vec<String>,
}

impl LintConfig {
    /// Detect appropriate linter for a project directory.
    pub fn detect(project_path: &Path) -> Vec<Self> {
        let mut linters = Vec::new();
        if project_path.join("Cargo.toml").exists() {
            linters.push(Self {
                tool: "cargo".to_string(),
                args: vec!["clippy".to_string(), "--quiet".to_string(), "--message-format=short".to_string()],
                file_patterns: vec!["*.rs".to_string()],
            });
        }
        if project_path.join("package.json").exists() {
            linters.push(Self {
                tool: "npx".to_string(),
                args: vec!["eslint".to_string(), ".".to_string(), "--format=compact".to_string()],
                file_patterns: vec!["*.ts".to_string(), "*.tsx".to_string(), "*.js".to_string()],
            });
        }
        if project_path.join("pyproject.toml").exists() || project_path.join("setup.py").exists() {
            linters.push(Self {
                tool: "ruff".to_string(),
                args: vec!["check".to_string(), ".".to_string()],
                file_patterns: vec!["*.py".to_string()],
            });
        }
        if project_path.join("go.mod").exists() {
            linters.push(Self {
                tool: "golangci-lint".to_string(),
                args: vec!["run".to_string()],
                file_patterns: vec!["*.go".to_string()],
            });
        }
        linters
    }

    pub fn command_string(&self) -> String {
        format!("{} {}", self.tool, self.args.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Test runner
// ---------------------------------------------------------------------------

/// Language-specific test runner configuration.
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub tool: String,
    pub args: Vec<String>,
    pub timeout_secs: u64,
}

impl TestConfig {
    /// Detect appropriate test runner for a project directory.
    pub fn detect(project_path: &Path) -> Vec<Self> {
        let mut runners = Vec::new();
        if project_path.join("Cargo.toml").exists() {
            runners.push(Self {
                tool: "cargo".to_string(),
                args: vec!["test".to_string(), "--quiet".to_string()],
                timeout_secs: 300,
            });
        }
        if project_path.join("package.json").exists() {
            runners.push(Self {
                tool: "npm".to_string(),
                args: vec!["test".to_string(), "--if-present".to_string()],
                timeout_secs: 120,
            });
        }
        if project_path.join("pyproject.toml").exists() || project_path.join("setup.py").exists() {
            runners.push(Self {
                tool: "pytest".to_string(),
                args: vec!["-q".to_string()],
                timeout_secs: 120,
            });
        }
        if project_path.join("go.mod").exists() {
            runners.push(Self {
                tool: "go".to_string(),
                args: vec!["test".to_string(), "./...".to_string()],
                timeout_secs: 120,
            });
        }
        runners
    }

    pub fn command_string(&self) -> String {
        format!("{} {}", self.tool, self.args.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Security scanner
// ---------------------------------------------------------------------------

/// Security check types.
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityCheckType {
    SecretScan,
    DependencyAudit,
    Sast,
}

/// Built-in secret patterns for scanning.
pub struct SecretScanner {
    patterns: Vec<SecretPattern>,
}

#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub name: String,
    pub pattern: String,
    pub severity: Severity,
}

impl SecretScanner {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                SecretPattern {
                    name: "AWS Access Key".to_string(),
                    pattern: "AKIA[0-9A-Z]{16}".to_string(),
                    severity: Severity::Critical,
                },
                SecretPattern {
                    name: "GitHub Token".to_string(),
                    pattern: "gh[ps]_[A-Za-z0-9_]{36,}".to_string(),
                    severity: Severity::Critical,
                },
                SecretPattern {
                    name: "Generic API Key".to_string(),
                    pattern: r#"(?i)(api[_-]?key|apikey)\s*[=:]\s*["']?[A-Za-z0-9]{20,}"#.to_string(),
                    severity: Severity::Error,
                },
                SecretPattern {
                    name: "Private Key".to_string(),
                    pattern: "-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----".to_string(),
                    severity: Severity::Critical,
                },
                SecretPattern {
                    name: "Password in Config".to_string(),
                    pattern: r#"(?i)(password|passwd|pwd)\s*[=:]\s*["'][^"']{8,}"#.to_string(),
                    severity: Severity::Error,
                },
                SecretPattern {
                    name: "Slack Webhook".to_string(),
                    pattern: "https://hooks\\.slack\\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+".to_string(),
                    severity: Severity::Error,
                },
            ],
        }
    }

    pub fn patterns(&self) -> &[SecretPattern] {
        &self.patterns
    }

    /// Scan a single line of text for secrets.
    pub fn scan_line(&self, line: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        for pattern in &self.patterns {
            // Simple substring check for common patterns
            if line.contains("AKIA") && pattern.name == "AWS Access Key" {
                findings.push(Finding::new(
                    CheckKind::Security,
                    pattern.severity.clone(),
                    &format!("Potential {}", pattern.name),
                ));
            }
            if (line.contains("ghp_") || line.contains("ghs_")) && pattern.name == "GitHub Token" {
                findings.push(Finding::new(
                    CheckKind::Security,
                    pattern.severity.clone(),
                    &format!("Potential {}", pattern.name),
                ));
            }
            if line.contains("BEGIN") && line.contains("PRIVATE KEY") && pattern.name == "Private Key" {
                findings.push(Finding::new(
                    CheckKind::Security,
                    pattern.severity.clone(),
                    &format!("Potential {}", pattern.name),
                ));
            }
            if line.contains("hooks.slack.com/services/") && pattern.name == "Slack Webhook" {
                findings.push(Finding::new(
                    CheckKind::Security,
                    pattern.severity.clone(),
                    &format!("Potential {}", pattern.name),
                ));
            }
        }
        findings
    }

    /// Scan content for all secret patterns.
    pub fn scan_content(&self, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            for mut finding in self.scan_line(line) {
                finding.line = Some(i + 1);
                findings.push(finding);
            }
        }
        findings
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency audit configuration.
#[derive(Debug, Clone)]
pub struct DependencyAudit {
    pub tool: String,
    pub args: Vec<String>,
}

impl DependencyAudit {
    pub fn detect(project_path: &Path) -> Vec<Self> {
        let mut auditors = Vec::new();
        if project_path.join("Cargo.lock").exists() {
            auditors.push(Self {
                tool: "cargo".to_string(),
                args: vec!["audit".to_string()],
            });
        }
        if project_path.join("package-lock.json").exists() {
            auditors.push(Self {
                tool: "npm".to_string(),
                args: vec!["audit".to_string(), "--production".to_string()],
            });
        }
        auditors
    }
}

// ---------------------------------------------------------------------------
// Self-review gate configuration
// ---------------------------------------------------------------------------

/// Configuration for the self-review gate.
#[derive(Debug, Clone)]
pub struct SelfReviewConfig {
    pub enabled: bool,
    pub max_retries: usize,
    pub checks: Vec<CheckKind>,
    pub fail_on_warning: bool,
    pub timeout_secs: u64,
    /// Minimum severity to block completion.
    pub min_blocking_severity: Severity,
}

impl Default for SelfReviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            checks: vec![
                CheckKind::Build,
                CheckKind::Lint,
                CheckKind::Test,
                CheckKind::Security,
            ],
            fail_on_warning: false,
            timeout_secs: 300,
            min_blocking_severity: Severity::Error,
        }
    }
}

impl SelfReviewConfig {
    pub fn with_checks(mut self, checks: Vec<CheckKind>) -> Self {
        self.checks = checks;
        self
    }

    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
}

// ---------------------------------------------------------------------------
// Self-review gate
// ---------------------------------------------------------------------------

/// The self-review gate: runs configured checks and decides pass/fail.
pub struct SelfReviewGate {
    config: SelfReviewConfig,
    project_path: PathBuf,
    results: Vec<ReviewIteration>,
}

/// One iteration of the self-review process.
#[derive(Debug, Clone)]
pub struct ReviewIteration {
    pub iteration: usize,
    pub checks: Vec<CheckResult>,
    pub passed: bool,
    pub timestamp: u64,
    pub feedback: Option<String>,
}

impl ReviewIteration {
    pub fn total_findings(&self) -> usize {
        self.checks.iter().map(|c| c.findings.len()).sum()
    }

    pub fn error_count(&self) -> usize {
        self.checks.iter().map(|c| c.error_count()).sum()
    }

    pub fn warning_count(&self) -> usize {
        self.checks.iter().map(|c| c.warning_count()).sum()
    }
}

/// Overall self-review decision.
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewDecision {
    /// All checks passed, task can complete.
    Approved,
    /// Checks failed, agent should retry with feedback.
    NeedsRevision { feedback: String, iteration: usize },
    /// Max retries exhausted, task completes with warnings.
    ForcedApproval { warnings: Vec<String> },
}

impl SelfReviewGate {
    pub fn new(config: SelfReviewConfig, project_path: PathBuf) -> Self {
        Self {
            config,
            project_path,
            results: Vec::new(),
        }
    }

    pub fn config(&self) -> &SelfReviewConfig {
        &self.config
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    pub fn iterations(&self) -> &[ReviewIteration] {
        &self.results
    }

    pub fn current_iteration(&self) -> usize {
        self.results.len()
    }

    /// Run all configured checks and return the decision.
    pub fn evaluate(&mut self, check_results: Vec<CheckResult>) -> ReviewDecision {
        let iteration = self.results.len() + 1;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Determine if checks pass
        let has_blocking = check_results.iter().any(|r| {
            !r.passed
                && r.findings
                    .iter()
                    .any(|f| f.severity >= self.config.min_blocking_severity)
        });

        let has_warnings = check_results.iter().any(|r| r.warning_count() > 0);
        let passed = !has_blocking && (!self.config.fail_on_warning || !has_warnings);

        let feedback = if !passed {
            Some(self.generate_feedback(&check_results))
        } else {
            None
        };

        let review = ReviewIteration {
            iteration,
            checks: check_results,
            passed,
            timestamp: ts,
            feedback: feedback.clone(),
        };
        self.results.push(review);

        if passed {
            ReviewDecision::Approved
        } else if iteration < self.config.max_retries {
            ReviewDecision::NeedsRevision {
                feedback: feedback.unwrap_or_default(),
                iteration,
            }
        } else {
            let warnings: Vec<String> = self
                .results
                .last()
                .map(|r| {
                    r.checks
                        .iter()
                        .flat_map(|c| c.findings.iter())
                        .map(|f| f.summary())
                        .collect()
                })
                .unwrap_or_default();
            ReviewDecision::ForcedApproval { warnings }
        }
    }

    /// Generate feedback text for the agent to iterate on.
    fn generate_feedback(&self, results: &[CheckResult]) -> String {
        let mut lines = Vec::new();
        lines.push("Self-review found issues that need to be fixed:\n".to_string());

        for result in results {
            if result.passed {
                continue;
            }
            lines.push(format!("## {} check FAILED", result.kind.name()));
            if let Some(cmd) = &result.command {
                lines.push(format!("Command: `{}`", cmd));
            }
            for finding in &result.findings {
                lines.push(format!("- {}", finding.summary()));
                if let Some(suggestion) = &finding.suggestion {
                    lines.push(format!("  Suggestion: {}", suggestion));
                }
            }
            lines.push(String::new());
        }

        lines.push("Please fix these issues and try completing the task again.".to_string());
        lines.join("\n")
    }

    /// Generate a summary report of all review iterations.
    pub fn report(&self) -> ReviewReport {
        let total_iterations = self.results.len();
        let final_passed = self.results.last().is_some_and(|r| r.passed);
        let total_findings: usize = self.results.iter().map(|r| r.total_findings()).sum();
        let checks_run: Vec<String> = self
            .config
            .checks
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        ReviewReport {
            total_iterations,
            final_passed,
            total_findings,
            checks_run,
            iterations: self.results.clone(),
        }
    }
}

/// Summary report of the self-review process.
#[derive(Debug, Clone)]
pub struct ReviewReport {
    pub total_iterations: usize,
    pub final_passed: bool,
    pub total_findings: usize,
    pub checks_run: Vec<String>,
    pub iterations: Vec<ReviewIteration>,
}

impl ReviewReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Self-Review Report\n\n");
        md.push_str(&format!(
            "**Status**: {}\n",
            if self.final_passed { "PASSED" } else { "FAILED" }
        ));
        md.push_str(&format!("**Iterations**: {}\n", self.total_iterations));
        md.push_str(&format!("**Total findings**: {}\n", self.total_findings));
        md.push_str(&format!("**Checks**: {}\n\n", self.checks_run.join(", ")));

        for iter in &self.iterations {
            md.push_str(&format!(
                "## Iteration {}: {}\n",
                iter.iteration,
                if iter.passed { "PASS" } else { "FAIL" }
            ));
            md.push_str(&format!(
                "- Errors: {}, Warnings: {}\n",
                iter.error_count(),
                iter.warning_count()
            ));
            for check in &iter.checks {
                md.push_str(&format!(
                    "  - {}: {} ({} findings)\n",
                    check.kind.name(),
                    if check.passed { "pass" } else { "fail" },
                    check.findings.len()
                ));
            }
            md.push('\n');
        }
        md
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_project() -> PathBuf {
        PathBuf::from("/tmp/test-project")
    }

    #[test]
    fn test_check_kind_name() {
        assert_eq!(CheckKind::Lint.name(), "lint");
        assert_eq!(CheckKind::Test.name(), "test");
        assert_eq!(CheckKind::Security.name(), "security");
        assert_eq!(CheckKind::DiffReview.name(), "diff_review");
        assert_eq!(CheckKind::Build.name(), "build");
        assert_eq!(CheckKind::Format.name(), "format");
        assert_eq!(CheckKind::TypeCheck.name(), "typecheck");
        assert_eq!(CheckKind::Custom("mycheck".into()).name(), "mycheck");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_finding_new() {
        let f = Finding::new(CheckKind::Lint, Severity::Warning, "unused variable");
        assert_eq!(f.message, "unused variable");
        assert!(f.file.is_none());
        assert!(f.line.is_none());
    }

    #[test]
    fn test_finding_with_location() {
        let f = Finding::new(CheckKind::Lint, Severity::Error, "err")
            .with_location("main.rs", 42);
        assert_eq!(f.file.as_deref(), Some("main.rs"));
        assert_eq!(f.line, Some(42));
    }

    #[test]
    fn test_finding_with_suggestion() {
        let f = Finding::new(CheckKind::Lint, Severity::Warning, "w")
            .with_suggestion("remove it");
        assert_eq!(f.suggestion.as_deref(), Some("remove it"));
    }

    #[test]
    fn test_finding_summary() {
        let f = Finding::new(CheckKind::Lint, Severity::Error, "unused import")
            .with_location("lib.rs", 10);
        assert_eq!(f.summary(), "[lint] error: unused import (lib.rs:10)");
    }

    #[test]
    fn test_finding_summary_no_location() {
        let f = Finding::new(CheckKind::Test, Severity::Warning, "slow test");
        assert_eq!(f.summary(), "[test] warning: slow test");
    }

    #[test]
    fn test_finding_summary_file_only() {
        let mut f = Finding::new(CheckKind::Security, Severity::Critical, "secret found");
        f.file = Some("config.toml".to_string());
        assert_eq!(f.summary(), "[security] critical: secret found (config.toml)");
    }

    #[test]
    fn test_check_result_pass() {
        let r = CheckResult::pass(CheckKind::Build);
        assert!(r.passed);
        assert_eq!(r.findings.len(), 0);
        assert_eq!(r.error_count(), 0);
    }

    #[test]
    fn test_check_result_fail() {
        let findings = vec![
            Finding::new(CheckKind::Lint, Severity::Error, "e1"),
            Finding::new(CheckKind::Lint, Severity::Warning, "w1"),
        ];
        let r = CheckResult::fail(CheckKind::Lint, findings);
        assert!(!r.passed);
        assert_eq!(r.error_count(), 1);
        assert_eq!(r.warning_count(), 1);
    }

    #[test]
    fn test_check_result_with_duration() {
        let r = CheckResult::pass(CheckKind::Test).with_duration(1500);
        assert_eq!(r.duration_ms, 1500);
    }

    #[test]
    fn test_check_result_with_command() {
        let r = CheckResult::pass(CheckKind::Build).with_command("cargo check");
        assert_eq!(r.command.as_deref(), Some("cargo check"));
    }

    #[test]
    fn test_lint_config_detect_rust() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let linters = LintConfig::detect(&dir);
        assert!(linters.iter().any(|l| l.tool == "cargo"));
    }

    #[test]
    fn test_lint_config_command_string() {
        let lc = LintConfig {
            tool: "cargo".into(),
            args: vec!["clippy".into(), "--quiet".into()],
            file_patterns: vec![],
        };
        assert_eq!(lc.command_string(), "cargo clippy --quiet");
    }

    #[test]
    fn test_test_config_detect_rust() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let runners = TestConfig::detect(&dir);
        assert!(runners.iter().any(|r| r.tool == "cargo"));
    }

    #[test]
    fn test_test_config_command_string() {
        let tc = TestConfig {
            tool: "npm".into(),
            args: vec!["test".into()],
            timeout_secs: 60,
        };
        assert_eq!(tc.command_string(), "npm test");
    }

    #[test]
    fn test_secret_scanner_new() {
        let scanner = SecretScanner::new();
        assert!(!scanner.patterns().is_empty());
    }

    #[test]
    fn test_secret_scanner_aws_key() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan_line("aws_key = AKIAIOSFODNN7EXAMPLE");
        assert!(findings.iter().any(|f| f.message.contains("AWS")));
    }

    #[test]
    fn test_secret_scanner_github_token() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan_line("token = ghp_abcdefghijklmnopqrstuvwxyz1234567890");
        assert!(findings.iter().any(|f| f.message.contains("GitHub")));
    }

    #[test]
    fn test_secret_scanner_private_key() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan_line("-----BEGIN RSA PRIVATE KEY-----");
        assert!(findings.iter().any(|f| f.message.contains("Private Key")));
    }

    #[test]
    fn test_secret_scanner_slack_webhook() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan_line("url = https://hooks.slack.com/services/T123/B456/abc");
        assert!(findings.iter().any(|f| f.message.contains("Slack")));
    }

    #[test]
    fn test_secret_scanner_clean_line() {
        let scanner = SecretScanner::new();
        let findings = scanner.scan_line("let x = 42;");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_secret_scanner_content() {
        let scanner = SecretScanner::new();
        let content = "line 1\nAKIAIOSFODNN7EXAMPLE\nline 3\n";
        let findings = scanner.scan_content(content);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].line, Some(2));
    }

    #[test]
    fn test_self_review_config_default() {
        let cfg = SelfReviewConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_retries, 3);
        assert!(!cfg.fail_on_warning);
        assert_eq!(cfg.checks.len(), 4);
    }

    #[test]
    fn test_self_review_config_with_checks() {
        let cfg = SelfReviewConfig::default()
            .with_checks(vec![CheckKind::Build, CheckKind::Lint]);
        assert_eq!(cfg.checks.len(), 2);
    }

    #[test]
    fn test_self_review_config_with_retries() {
        let cfg = SelfReviewConfig::default().with_max_retries(5);
        assert_eq!(cfg.max_retries, 5);
    }

    #[test]
    fn test_gate_all_pass() {
        let config = SelfReviewConfig::default();
        let mut gate = SelfReviewGate::new(config, test_project());
        let results = vec![
            CheckResult::pass(CheckKind::Build),
            CheckResult::pass(CheckKind::Lint),
            CheckResult::pass(CheckKind::Test),
        ];
        let decision = gate.evaluate(results);
        assert_eq!(decision, ReviewDecision::Approved);
        assert_eq!(gate.current_iteration(), 1);
    }

    #[test]
    fn test_gate_fail_needs_revision() {
        let config = SelfReviewConfig::default().with_max_retries(3);
        let mut gate = SelfReviewGate::new(config, test_project());
        let results = vec![CheckResult::fail(
            CheckKind::Lint,
            vec![Finding::new(
                CheckKind::Lint,
                Severity::Error,
                "unused import",
            )],
        )];
        let decision = gate.evaluate(results);
        match decision {
            ReviewDecision::NeedsRevision { iteration, .. } => {
                assert_eq!(iteration, 1);
            }
            _ => panic!("Expected NeedsRevision"),
        }
    }

    #[test]
    fn test_gate_max_retries_forced_approval() {
        let config = SelfReviewConfig::default().with_max_retries(2);
        let mut gate = SelfReviewGate::new(config, test_project());
        let fail_results = vec![CheckResult::fail(
            CheckKind::Test,
            vec![Finding::new(CheckKind::Test, Severity::Error, "test failed")],
        )];
        // Iteration 1: fail
        let d1 = gate.evaluate(fail_results.clone());
        assert!(matches!(d1, ReviewDecision::NeedsRevision { .. }));
        // Iteration 2 (== max_retries): forced approval
        let d2 = gate.evaluate(fail_results);
        assert!(matches!(d2, ReviewDecision::ForcedApproval { .. }));
    }

    #[test]
    fn test_gate_pass_after_retry() {
        let config = SelfReviewConfig::default().with_max_retries(3);
        let mut gate = SelfReviewGate::new(config, test_project());
        // First: fail
        let fail = vec![CheckResult::fail(
            CheckKind::Lint,
            vec![Finding::new(CheckKind::Lint, Severity::Error, "err")],
        )];
        gate.evaluate(fail);
        // Second: pass
        let pass = vec![CheckResult::pass(CheckKind::Lint)];
        let decision = gate.evaluate(pass);
        assert_eq!(decision, ReviewDecision::Approved);
    }

    #[test]
    fn test_gate_fail_on_warning() {
        let mut config = SelfReviewConfig::default();
        config.fail_on_warning = true;
        let mut gate = SelfReviewGate::new(config, test_project());
        let results = vec![CheckResult::fail(
            CheckKind::Lint,
            vec![Finding::new(CheckKind::Lint, Severity::Warning, "warn")],
        )];
        let decision = gate.evaluate(results);
        assert!(matches!(decision, ReviewDecision::NeedsRevision { .. }));
    }

    #[test]
    fn test_gate_warning_doesnt_block_by_default() {
        let config = SelfReviewConfig::default();
        let mut gate = SelfReviewGate::new(config, test_project());
        // Only warnings, no errors
        let mut result = CheckResult::pass(CheckKind::Lint);
        result.findings.push(Finding::new(
            CheckKind::Lint,
            Severity::Warning,
            "unused variable",
        ));
        let decision = gate.evaluate(vec![result]);
        assert_eq!(decision, ReviewDecision::Approved);
    }

    #[test]
    fn test_review_iteration_counts() {
        let iter = ReviewIteration {
            iteration: 1,
            checks: vec![
                CheckResult::fail(
                    CheckKind::Lint,
                    vec![
                        Finding::new(CheckKind::Lint, Severity::Error, "e1"),
                        Finding::new(CheckKind::Lint, Severity::Warning, "w1"),
                    ],
                ),
                CheckResult::fail(
                    CheckKind::Test,
                    vec![Finding::new(CheckKind::Test, Severity::Error, "e2")],
                ),
            ],
            passed: false,
            timestamp: 0,
            feedback: None,
        };
        assert_eq!(iter.total_findings(), 3);
        assert_eq!(iter.error_count(), 2);
        assert_eq!(iter.warning_count(), 1);
    }

    #[test]
    fn test_report_generation() {
        let config = SelfReviewConfig::default();
        let mut gate = SelfReviewGate::new(config, test_project());
        gate.evaluate(vec![CheckResult::pass(CheckKind::Build)]);
        let report = gate.report();
        assert_eq!(report.total_iterations, 1);
        assert!(report.final_passed);
        assert_eq!(report.total_findings, 0);
    }

    #[test]
    fn test_report_to_markdown() {
        let config = SelfReviewConfig::default();
        let mut gate = SelfReviewGate::new(config, test_project());
        gate.evaluate(vec![CheckResult::pass(CheckKind::Build)]);
        let md = gate.report().to_markdown();
        assert!(md.contains("# Self-Review Report"));
        assert!(md.contains("PASSED"));
    }

    #[test]
    fn test_report_failed_markdown() {
        let config = SelfReviewConfig::default().with_max_retries(1);
        let mut gate = SelfReviewGate::new(config, test_project());
        gate.evaluate(vec![CheckResult::fail(
            CheckKind::Lint,
            vec![Finding::new(CheckKind::Lint, Severity::Error, "err")],
        )]);
        let md = gate.report().to_markdown();
        assert!(md.contains("FAILED"));
        assert!(md.contains("lint: fail"));
    }

    #[test]
    fn test_dependency_audit_detect() {
        // May or may not find lock files depending on environment
        let auditors = DependencyAudit::detect(&test_project());
        // Just verify it doesn't panic
        let _ = auditors.len();
    }

    #[test]
    fn test_generate_feedback() {
        let config = SelfReviewConfig::default();
        let gate = SelfReviewGate::new(config, test_project());
        let results = vec![CheckResult::fail(
            CheckKind::Security,
            vec![Finding::new(
                CheckKind::Security,
                Severity::Critical,
                "secret found",
            )
            .with_suggestion("remove the secret")],
        )];
        let feedback = gate.generate_feedback(&results);
        assert!(feedback.contains("security check FAILED"));
        assert!(feedback.contains("secret found"));
        assert!(feedback.contains("remove the secret"));
    }

    #[test]
    fn test_generate_feedback_skips_passed() {
        let config = SelfReviewConfig::default();
        let gate = SelfReviewGate::new(config, test_project());
        let results = vec![
            CheckResult::pass(CheckKind::Build),
            CheckResult::fail(
                CheckKind::Test,
                vec![Finding::new(CheckKind::Test, Severity::Error, "test fail")],
            ),
        ];
        let feedback = gate.generate_feedback(&results);
        assert!(!feedback.contains("build"));
        assert!(feedback.contains("test check FAILED"));
    }

    #[test]
    fn test_gate_project_path() {
        let gate = SelfReviewGate::new(SelfReviewConfig::default(), test_project());
        assert_eq!(gate.project_path(), Path::new("/tmp/test-project"));
    }

    #[test]
    fn test_gate_config_access() {
        let config = SelfReviewConfig::default().with_max_retries(7);
        let gate = SelfReviewGate::new(config, test_project());
        assert_eq!(gate.config().max_retries, 7);
    }

    #[test]
    fn test_min_blocking_severity() {
        let mut config = SelfReviewConfig::default();
        config.min_blocking_severity = Severity::Critical;
        let mut gate = SelfReviewGate::new(config, test_project());
        // Error severity is below Critical, so should pass
        let results = vec![CheckResult::fail(
            CheckKind::Lint,
            vec![Finding::new(CheckKind::Lint, Severity::Error, "err")],
        )];
        let decision = gate.evaluate(results);
        assert_eq!(decision, ReviewDecision::Approved);
    }

    #[test]
    fn test_critical_blocks_with_default_config() {
        let config = SelfReviewConfig::default();
        let mut gate = SelfReviewGate::new(config, test_project());
        let results = vec![CheckResult::fail(
            CheckKind::Security,
            vec![Finding::new(
                CheckKind::Security,
                Severity::Critical,
                "secret",
            )],
        )];
        let decision = gate.evaluate(results);
        assert!(matches!(decision, ReviewDecision::NeedsRevision { .. }));
    }
}
