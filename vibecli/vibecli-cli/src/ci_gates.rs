//! CI-enforceable AI checks for VibeCody.
//!
//! Implements CI pipeline gates that enforce code quality, security, and style
//! rules on diffs. Inspired by Continue's CI-enforceable AI checks.
//!
//! REPL commands: `/cigate run|add|remove|list|report|prebuilt`

use std::time::{SystemTime, UNIX_EPOCH};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Json,
    Text,
    Markdown,
    JUnit,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Text => write!(f, "text"),
            Self::Markdown => write!(f, "markdown"),
            Self::JUnit => write!(f, "junit"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleCategory {
    Security,
    CodeQuality,
    Performance,
    Style,
    TestCoverage,
    ApiBreaking,
    DependencyAudit,
    Documentation,
}

impl std::fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Security => write!(f, "security"),
            Self::CodeQuality => write!(f, "code-quality"),
            Self::Performance => write!(f, "performance"),
            Self::Style => write!(f, "style"),
            Self::TestCoverage => write!(f, "test-coverage"),
            Self::ApiBreaking => write!(f, "api-breaking"),
            Self::DependencyAudit => write!(f, "dependency-audit"),
            Self::Documentation => write!(f, "documentation"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    RegexMatch(String),
    FileExists(String),
    FileMustNotExist(String),
    MaxFileSize(usize),
    RequiredHeader(String),
    NoTodoComments,
    NoConsoleLog,
    NoHardcodedSecrets,
    TestCoverageMin(f32),
    CustomScript(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrebuiltRule {
    OwaspTop10,
    PerformanceRegression,
    ApiBreakingChanges,
    NoSecrets,
    RequireTests,
    MaxComplexity,
    NoTodos,
}

impl PrebuiltRule {
    pub fn to_rules(&self) -> Vec<CiRule> {
        match self {
            Self::OwaspTop10 => vec![
                CiRule {
                    id: "owasp-sql-injection".into(),
                    name: "SQL Injection Detection".into(),
                    description: "Detect potential SQL injection vulnerabilities".into(),
                    category: RuleCategory::Security,
                    severity: Severity::Error,
                    enabled: true,
                    pattern: Some("*.rs".into()),
                    check_type: CheckType::RegexMatch(r#"format!\(.*SELECT.*\{.*\}"#.into()),
                },
                CiRule {
                    id: "owasp-xss".into(),
                    name: "XSS Detection".into(),
                    description: "Detect potential cross-site scripting".into(),
                    category: RuleCategory::Security,
                    severity: Severity::Error,
                    enabled: true,
                    pattern: Some("*.{js,ts,tsx,jsx}".into()),
                    check_type: CheckType::RegexMatch(r"dangerouslySetInnerHTML".into()),
                },
                CiRule {
                    id: "owasp-hardcoded-secrets".into(),
                    name: "Hardcoded Secrets".into(),
                    description: "Detect hardcoded secrets and credentials".into(),
                    category: RuleCategory::Security,
                    severity: Severity::Error,
                    enabled: true,
                    pattern: None,
                    check_type: CheckType::NoHardcodedSecrets,
                },
            ],
            Self::PerformanceRegression => vec![
                CiRule {
                    id: "perf-no-sync-io".into(),
                    name: "No Synchronous I/O in Async".into(),
                    description: "Detect blocking I/O in async contexts".into(),
                    category: RuleCategory::Performance,
                    severity: Severity::Warning,
                    enabled: true,
                    pattern: Some("*.rs".into()),
                    check_type: CheckType::RegexMatch(r"std::fs::(read|write)".into()),
                },
            ],
            Self::ApiBreakingChanges => vec![
                CiRule {
                    id: "api-no-pub-removal".into(),
                    name: "No Public API Removal".into(),
                    description: "Detect removal of public API items".into(),
                    category: RuleCategory::ApiBreaking,
                    severity: Severity::Error,
                    enabled: true,
                    pattern: Some("*.rs".into()),
                    check_type: CheckType::RegexMatch(r"^-\s*pub\s+(fn|struct|enum|trait)".into()),
                },
            ],
            Self::NoSecrets => vec![
                CiRule {
                    id: "no-hardcoded-secrets".into(),
                    name: "No Hardcoded Secrets".into(),
                    description: "Prevent committing API keys, tokens, and passwords".into(),
                    category: RuleCategory::Security,
                    severity: Severity::Error,
                    enabled: true,
                    pattern: None,
                    check_type: CheckType::NoHardcodedSecrets,
                },
            ],
            Self::RequireTests => vec![
                CiRule {
                    id: "require-test-file".into(),
                    name: "Require Tests".into(),
                    description: "Ensure test files exist for new modules".into(),
                    category: RuleCategory::TestCoverage,
                    severity: Severity::Warning,
                    enabled: true,
                    pattern: Some("*.rs".into()),
                    check_type: CheckType::RegexMatch(r"#\[cfg\(test\)\]".into()),
                },
            ],
            Self::MaxComplexity => vec![
                CiRule {
                    id: "max-file-size".into(),
                    name: "Maximum File Size".into(),
                    description: "Files should not exceed 50KB".into(),
                    category: RuleCategory::CodeQuality,
                    severity: Severity::Warning,
                    enabled: true,
                    pattern: None,
                    check_type: CheckType::MaxFileSize(50_000),
                },
            ],
            Self::NoTodos => vec![
                CiRule {
                    id: "no-todo-comments".into(),
                    name: "No TODO Comments".into(),
                    description: "Prevent TODO/FIXME/HACK comments from being committed".into(),
                    category: RuleCategory::CodeQuality,
                    severity: Severity::Warning,
                    enabled: true,
                    pattern: None,
                    check_type: CheckType::NoTodoComments,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateError {
    RuleNotFound,
    RuleParseError,
    TimeoutExceeded,
    CheckFailed,
    InvalidConfig,
    DuplicateRule,
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuleNotFound => write!(f, "rule not found"),
            Self::RuleParseError => write!(f, "rule parse error"),
            Self::TimeoutExceeded => write!(f, "timeout exceeded"),
            Self::CheckFailed => write!(f, "check failed"),
            Self::InvalidConfig => write!(f, "invalid config"),
            Self::DuplicateRule => write!(f, "duplicate rule"),
        }
    }
}

// === Structs ===

#[derive(Debug, Clone)]
pub struct GateConfig {
    pub rules_dir: String,
    pub fail_on_error: bool,
    pub timeout_secs: u64,
    pub parallel_checks: bool,
    pub output_format: OutputFormat,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            rules_dir: ".viberules/ci/".into(),
            fail_on_error: true,
            timeout_secs: 300,
            parallel_checks: true,
            output_format: OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CiRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: RuleCategory,
    pub severity: Severity,
    pub enabled: bool,
    pub pattern: Option<String>,
    pub check_type: CheckType,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub rule_id: String,
    pub rule_name: String,
    pub passed: bool,
    pub severity: Severity,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub suggestion: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct GateReport {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub infos: usize,
    pub results: Vec<CheckResult>,
    pub overall_passed: bool,
    pub duration_ms: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: String,
    pub additions: Vec<String>,
    pub deletions: Vec<String>,
    pub is_new: bool,
    pub is_deleted: bool,
}

// === CiGateRunner ===

pub struct CiGateRunner {
    pub config: GateConfig,
    rules: Vec<CiRule>,
    execution_history: Vec<GateReport>,
}

impl CiGateRunner {
    pub fn new(config: GateConfig) -> Self {
        Self {
            config,
            rules: Vec::new(),
            execution_history: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: CiRule) -> Result<(), GateError> {
        if self.rules.iter().any(|r| r.id == rule.id) {
            return Err(GateError::DuplicateRule);
        }
        self.rules.push(rule);
        Ok(())
    }

    pub fn remove_rule(&mut self, id: &str) -> Result<(), GateError> {
        let idx = self.rules.iter().position(|r| r.id == id)
            .ok_or(GateError::RuleNotFound)?;
        self.rules.remove(idx);
        Ok(())
    }

    pub fn get_rule(&self, id: &str) -> Option<&CiRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    pub fn list_rules(&self) -> Vec<&CiRule> {
        self.rules.iter().collect()
    }

    pub fn load_prebuilt(&mut self, prebuilt: PrebuiltRule) {
        for rule in prebuilt.to_rules() {
            // Skip duplicates silently when loading prebuilt sets
            if !self.rules.iter().any(|r| r.id == rule.id) {
                self.rules.push(rule);
            }
        }
    }

    pub fn run_checks(&mut self, files: &[DiffFile]) -> GateReport {
        let start = SystemTime::now();
        let mut all_results = Vec::new();

        let enabled_rules: Vec<_> = self.rules.iter().filter(|r| r.enabled).cloned().collect();
        for rule in &enabled_rules {
            let results = self.run_single_check(rule, files);
            all_results.extend(results);
        }

        let failed = all_results.iter().filter(|r| !r.passed && r.severity == Severity::Error).count();
        let warnings = all_results.iter().filter(|r| !r.passed && r.severity == Severity::Warning).count();
        let infos = all_results.iter().filter(|r| !r.passed && r.severity == Severity::Info).count();
        let passed = all_results.iter().filter(|r| r.passed).count();

        let overall_passed = if self.config.fail_on_error {
            failed == 0
        } else {
            true
        };

        let duration_ms = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let report = GateReport {
            total_checks: all_results.len(),
            passed,
            failed,
            warnings,
            infos,
            results: all_results,
            overall_passed,
            duration_ms,
            timestamp,
        };

        self.execution_history.push(report.clone());
        report
    }

    pub fn run_single_check(&self, rule: &CiRule, files: &[DiffFile]) -> Vec<CheckResult> {
        let start = SystemTime::now();
        let filtered = self.filter_files(rule, files);

        let mut results = match &rule.check_type {
            CheckType::RegexMatch(pattern) => self.check_regex_match(pattern, &filtered, rule),
            CheckType::NoHardcodedSecrets => self.check_no_secrets(&filtered, rule),
            CheckType::NoTodoComments => self.check_no_todos(&filtered, rule),
            CheckType::NoConsoleLog => self.check_no_console_log(&filtered, rule),
            CheckType::FileExists(path) => self.check_file_exists(path, &filtered, rule),
            CheckType::FileMustNotExist(path) => self.check_file_must_not_exist(path, &filtered, rule),
            CheckType::MaxFileSize(max) => self.check_max_file_size(*max, &filtered, rule),
            CheckType::RequiredHeader(header) => self.check_required_header(header, &filtered, rule),
            CheckType::TestCoverageMin(min) => self.check_test_coverage_min(*min, &filtered, rule),
            CheckType::CustomScript(_cmd) => {
                vec![CheckResult {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    passed: true,
                    severity: rule.severity.clone(),
                    message: "Custom script checks not executed in this context".into(),
                    file_path: None,
                    line_number: None,
                    suggestion: None,
                    duration_ms: 0,
                }]
            }
        };

        let duration = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);
        for r in &mut results {
            r.duration_ms = duration;
        }

        // If no results generated (e.g. no matching files), emit a pass
        if results.is_empty() {
            results.push(CheckResult {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: "No matching files to check".into(),
                file_path: None,
                line_number: None,
                suggestion: None,
                duration_ms: 0,
            });
        }

        results
    }

    fn filter_files<'a>(&self, rule: &CiRule, files: &'a [DiffFile]) -> Vec<&'a DiffFile> {
        match &rule.pattern {
            None => files.iter().collect(),
            Some(pat) => {
                files.iter().filter(|f| Self::glob_matches(pat, &f.path)).collect()
            }
        }
    }

    fn glob_matches(pattern: &str, path: &str) -> bool {
        // Support simple glob patterns: *.ext and *.{ext1,ext2}
        if let Some(rest) = pattern.strip_prefix("*.") {
            if rest.starts_with('{') && rest.ends_with('}') {
                let exts = &rest[1..rest.len() - 1];
                for ext in exts.split(',') {
                    if path.ends_with(&format!(".{}", ext.trim())) {
                        return true;
                    }
                }
                return false;
            }
            return path.ends_with(&format!(".{}", rest));
        }
        // Exact match fallback
        path == pattern
    }

    pub fn check_regex_match(&self, pattern: &str, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let mut results = Vec::new();
        let re = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => {
                results.push(CheckResult {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    passed: false,
                    severity: Severity::Error,
                    message: format!("Invalid regex pattern: {}", pattern),
                    file_path: None,
                    line_number: None,
                    suggestion: Some("Fix the regex pattern".into()),
                    duration_ms: 0,
                });
                return results;
            }
        };

        for file in files {
            for (i, line) in file.additions.iter().enumerate() {
                if re.is_match(line) {
                    results.push(CheckResult {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        passed: false,
                        severity: rule.severity.clone(),
                        message: format!("Pattern '{}' matched in {}", pattern, file.path),
                        file_path: Some(file.path.clone()),
                        line_number: Some((i + 1) as u32),
                        suggestion: Some(format!("Review line for: {}", rule.description)),
                        duration_ms: 0,
                    });
                }
            }
        }

        if results.is_empty() {
            results.push(CheckResult {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: format!("No matches for pattern '{}'", pattern),
                file_path: None,
                line_number: None,
                suggestion: None,
                duration_ms: 0,
            });
        }

        results
    }

    pub fn check_no_secrets(&self, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let secret_patterns = [
            (r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"][A-Za-z0-9_\-]{16,}['"]"#, "API key"),
            (r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}['"]"#, "password/secret"),
            (r#"(?i)bearer\s+[A-Za-z0-9_\-\.]{20,}"#, "bearer token"),
            (r#"AKIA[0-9A-Z]{16}"#, "AWS access key"),
            (r#"(?i)(sk-[A-Za-z0-9]{32,})"#, "OpenAI API key"),
            (r#"ghp_[A-Za-z0-9]{36}"#, "GitHub personal access token"),
            (r#"(?i)private[_-]?key\s*[:=]\s*['"]"#, "private key"),
        ];

        let mut results = Vec::new();
        let compiled: Vec<_> = secret_patterns.iter()
            .filter_map(|(p, label)| regex::Regex::new(p).ok().map(|re| (re, *label)))
            .collect();

        for file in files {
            for (i, line) in file.additions.iter().enumerate() {
                for (re, label) in &compiled {
                    if re.is_match(line) {
                        results.push(CheckResult {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            passed: false,
                            severity: Severity::Error,
                            message: format!("Potential {} detected in {}", label, file.path),
                            file_path: Some(file.path.clone()),
                            line_number: Some((i + 1) as u32),
                            suggestion: Some(format!("Remove hardcoded {} and use environment variables", label)),
                            duration_ms: 0,
                        });
                    }
                }
            }
        }

        if results.is_empty() {
            results.push(CheckResult {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: "No hardcoded secrets detected".into(),
                file_path: None,
                line_number: None,
                suggestion: None,
                duration_ms: 0,
            });
        }

        results
    }

    pub fn check_no_todos(&self, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let re = regex::Regex::new(r"(?i)\b(TODO|FIXME|HACK|XXX)\b").expect("valid regex");
        let mut results = Vec::new();

        for file in files {
            for (i, line) in file.additions.iter().enumerate() {
                if re.is_match(line) {
                    results.push(CheckResult {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        passed: false,
                        severity: rule.severity.clone(),
                        message: format!("TODO/FIXME comment found in {}", file.path),
                        file_path: Some(file.path.clone()),
                        line_number: Some((i + 1) as u32),
                        suggestion: Some("Resolve the TODO before committing".into()),
                        duration_ms: 0,
                    });
                }
            }
        }

        if results.is_empty() {
            results.push(CheckResult {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: "No TODO/FIXME comments found".into(),
                file_path: None,
                line_number: None,
                suggestion: None,
                duration_ms: 0,
            });
        }

        results
    }

    pub fn check_no_console_log(&self, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let re = regex::Regex::new(r"console\.(log|warn|error|debug|info)\s*\(").expect("valid regex");
        let mut results = Vec::new();

        for file in files {
            for (i, line) in file.additions.iter().enumerate() {
                if re.is_match(line) {
                    results.push(CheckResult {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        passed: false,
                        severity: rule.severity.clone(),
                        message: format!("console.log found in {}", file.path),
                        file_path: Some(file.path.clone()),
                        line_number: Some((i + 1) as u32),
                        suggestion: Some("Remove console.log or use a proper logger".into()),
                        duration_ms: 0,
                    });
                }
            }
        }

        if results.is_empty() {
            results.push(CheckResult {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                passed: true,
                severity: rule.severity.clone(),
                message: "No console.log statements found".into(),
                file_path: None,
                line_number: None,
                suggestion: None,
                duration_ms: 0,
            });
        }

        results
    }

    fn check_file_exists(&self, path: &str, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let found = files.iter().any(|f| f.path == path);
        vec![CheckResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            passed: found,
            severity: rule.severity.clone(),
            message: if found {
                format!("Required file '{}' exists", path)
            } else {
                format!("Required file '{}' not found", path)
            },
            file_path: Some(path.into()),
            line_number: None,
            suggestion: if found { None } else { Some(format!("Create file '{}'", path)) },
            duration_ms: 0,
        }]
    }

    fn check_file_must_not_exist(&self, path: &str, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let found = files.iter().any(|f| f.path == path && f.is_new);
        vec![CheckResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            passed: !found,
            severity: rule.severity.clone(),
            message: if found {
                format!("Forbidden file '{}' was added", path)
            } else {
                format!("Forbidden file '{}' not present", path)
            },
            file_path: Some(path.into()),
            line_number: None,
            suggestion: if found { Some(format!("Remove file '{}'", path)) } else { None },
            duration_ms: 0,
        }]
    }

    fn check_max_file_size(&self, max_bytes: usize, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let mut results = Vec::new();
        for file in files {
            let size: usize = file.additions.iter().map(|l| l.len() + 1).sum();
            if size > max_bytes {
                results.push(CheckResult {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    passed: false,
                    severity: rule.severity.clone(),
                    message: format!("File '{}' exceeds max size ({} > {} bytes)", file.path, size, max_bytes),
                    file_path: Some(file.path.clone()),
                    line_number: None,
                    suggestion: Some("Split the file into smaller modules".into()),
                    duration_ms: 0,
                });
            }
        }
        results
    }

    fn check_required_header(&self, header: &str, files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        let mut results = Vec::new();
        for file in files {
            let has_header = file.additions.first().is_some_and(|l| l.contains(header));
            if !has_header {
                results.push(CheckResult {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    passed: false,
                    severity: rule.severity.clone(),
                    message: format!("File '{}' missing required header", file.path),
                    file_path: Some(file.path.clone()),
                    line_number: Some(1),
                    suggestion: Some(format!("Add header: {}", header)),
                    duration_ms: 0,
                });
            }
        }
        results
    }

    fn check_test_coverage_min(&self, _min: f32, _files: &[&DiffFile], rule: &CiRule) -> Vec<CheckResult> {
        // Coverage checks require external tooling; emit informational result
        vec![CheckResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            passed: true,
            severity: Severity::Info,
            message: "Test coverage check requires external tooling".into(),
            file_path: None,
            line_number: None,
            suggestion: None,
            duration_ms: 0,
        }]
    }

    pub fn format_report(&self, report: &GateReport) -> String {
        match self.config.output_format {
            OutputFormat::Json => {
                let results_json: Vec<String> = report.results.iter().map(|r| {
                    format!(
                        r#"    {{"rule_id":"{}","passed":{},"severity":"{}","message":"{}","file":"{}","line":{}}}"#,
                        r.rule_id, r.passed, r.severity,
                        r.message.replace('"', r#"\""#),
                        r.file_path.as_deref().unwrap_or(""),
                        r.line_number.map_or("null".into(), |n| n.to_string()),
                    )
                }).collect();
                format!(
                    r#"{{"total":{},"passed":{},"failed":{},"warnings":{},"overall_passed":{},"results":[
{}
]}}"#,
                    report.total_checks, report.passed, report.failed,
                    report.warnings, report.overall_passed,
                    results_json.join(",\n"),
                )
            }
            OutputFormat::Text => Self::format_as_text(report),
            OutputFormat::Markdown => Self::format_as_markdown(report),
            OutputFormat::JUnit => Self::format_as_junit(report),
        }
    }

    pub fn format_as_text(report: &GateReport) -> String {
        let mut out = String::new();
        out.push_str(&format!("CI Gate Report: {}/{} passed\n", report.passed, report.total_checks));
        out.push_str(&format!("Status: {}\n", if report.overall_passed { "PASSED" } else { "FAILED" }));
        out.push_str("---\n");
        for r in &report.results {
            let icon = if r.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!("[{}] [{}] {}: {}\n", icon, r.severity, r.rule_name, r.message));
            if let Some(ref suggestion) = r.suggestion {
                out.push_str(&format!("  Suggestion: {}\n", suggestion));
            }
        }
        out
    }

    pub fn format_as_markdown(report: &GateReport) -> String {
        let mut out = String::new();
        let status = if report.overall_passed { "PASSED" } else { "FAILED" };
        out.push_str(&format!("# CI Gate Report: {}\n\n", status));
        out.push_str("| Metric | Count |\n|--------|-------|\n");
        out.push_str(&format!("| Total  | {} |\n", report.total_checks));
        out.push_str(&format!("| Passed | {} |\n", report.passed));
        out.push_str(&format!("| Failed | {} |\n", report.failed));
        out.push_str(&format!("| Warnings | {} |\n\n", report.warnings));
        out.push_str("## Results\n\n");
        for r in &report.results {
            let icon = if r.passed { "+" } else { "-" };
            out.push_str(&format!("- [{}] **{}**: {}\n", icon, r.rule_name, r.message));
        }
        out
    }

    pub fn format_as_junit(report: &GateReport) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(&format!(
            "<testsuite name=\"ci-gates\" tests=\"{}\" failures=\"{}\" warnings=\"{}\">\n",
            report.total_checks, report.failed, report.warnings,
        ));
        for r in &report.results {
            out.push_str(&format!("  <testcase name=\"{}\" classname=\"{}\"", r.rule_name, r.rule_id));
            if r.passed {
                out.push_str(" />\n");
            } else {
                out.push_str(">\n");
                let tag = match r.severity {
                    Severity::Error => "failure",
                    Severity::Warning => "warning",
                    Severity::Info => "system-out",
                };
                out.push_str(&format!("    <{} message=\"{}\" />\n", tag, r.message.replace('"', "&quot;")));
                out.push_str("  </testcase>\n");
            }
        }
        out.push_str("</testsuite>\n");
        out
    }

    pub fn generate_exit_code(report: &GateReport) -> i32 {
        if report.overall_passed && report.warnings == 0 {
            0
        } else if report.overall_passed && report.warnings > 0 {
            2
        } else {
            1
        }
    }

    pub fn generate_github_action_yaml() -> String {
        r#"name: VibeCody CI Gates
on:
  pull_request:
    branches: [main]

jobs:
  ci-gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install VibeCody
        run: curl -fsSL https://vibecody.dev/install.sh | bash

      - name: Run CI Gates
        run: vibecli ci-gate --format junit --output ci-gates-report.xml
        env:
          VIBECODY_RULES_DIR: .viberules/ci/

      - name: Upload Report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: ci-gates-report
          path: ci-gates-report.xml

      - name: Publish Test Results
        if: always()
        uses: EnricoMi/publish-unit-test-result-action@v2
        with:
          files: ci-gates-report.xml
"#.to_string()
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> GateConfig {
        GateConfig::default()
    }

    fn make_diff_file(path: &str, additions: Vec<&str>) -> DiffFile {
        DiffFile {
            path: path.into(),
            additions: additions.into_iter().map(String::from).collect(),
            deletions: Vec::new(),
            is_new: false,
            is_deleted: false,
        }
    }

    fn make_rule(id: &str, check_type: CheckType) -> CiRule {
        CiRule {
            id: id.into(),
            name: format!("Rule {}", id),
            description: format!("Description for {}", id),
            category: RuleCategory::CodeQuality,
            severity: Severity::Error,
            enabled: true,
            pattern: None,
            check_type,
        }
    }

    // -- Config defaults --

    #[test]
    fn test_config_defaults() {
        let cfg = GateConfig::default();
        assert_eq!(cfg.rules_dir, ".viberules/ci/");
        assert!(cfg.fail_on_error);
        assert_eq!(cfg.timeout_secs, 300);
        assert!(cfg.parallel_checks);
        assert_eq!(cfg.output_format, OutputFormat::Text);
    }

    #[test]
    fn test_config_custom() {
        let cfg = GateConfig {
            rules_dir: "/custom/rules".into(),
            fail_on_error: false,
            timeout_secs: 60,
            parallel_checks: false,
            output_format: OutputFormat::Json,
        };
        assert_eq!(cfg.rules_dir, "/custom/rules");
        assert!(!cfg.fail_on_error);
        assert_eq!(cfg.timeout_secs, 60);
    }

    // -- Rule CRUD --

    #[test]
    fn test_add_rule() {
        let mut runner = CiGateRunner::new(make_config());
        let rule = make_rule("r1", CheckType::NoTodoComments);
        assert!(runner.add_rule(rule).is_ok());
        assert_eq!(runner.list_rules().len(), 1);
    }

    #[test]
    fn test_add_duplicate_rule() {
        let mut runner = CiGateRunner::new(make_config());
        let rule = make_rule("r1", CheckType::NoTodoComments);
        runner.add_rule(rule.clone()).unwrap();
        assert_eq!(runner.add_rule(rule), Err(GateError::DuplicateRule));
    }

    #[test]
    fn test_remove_rule() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(make_rule("r1", CheckType::NoTodoComments)).unwrap();
        assert!(runner.remove_rule("r1").is_ok());
        assert_eq!(runner.list_rules().len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_rule() {
        let mut runner = CiGateRunner::new(make_config());
        assert_eq!(runner.remove_rule("nope"), Err(GateError::RuleNotFound));
    }

    #[test]
    fn test_get_rule() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(make_rule("r1", CheckType::NoTodoComments)).unwrap();
        assert!(runner.get_rule("r1").is_some());
        assert!(runner.get_rule("r2").is_none());
    }

    #[test]
    fn test_list_rules_empty() {
        let runner = CiGateRunner::new(make_config());
        assert!(runner.list_rules().is_empty());
    }

    // -- Prebuilt rules --

    #[test]
    fn test_prebuilt_owasp() {
        let rules = PrebuiltRule::OwaspTop10.to_rules();
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().any(|r| r.id.contains("sql-injection")));
        assert!(rules.iter().any(|r| r.id.contains("xss")));
    }

    #[test]
    fn test_prebuilt_no_secrets() {
        let rules = PrebuiltRule::NoSecrets.to_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].check_type, CheckType::NoHardcodedSecrets);
    }

    #[test]
    fn test_load_prebuilt() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        assert_eq!(runner.list_rules().len(), 1);
    }

    #[test]
    fn test_load_prebuilt_no_duplicate() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        assert_eq!(runner.list_rules().len(), 1);
    }

    // -- Check execution --

    #[test]
    fn test_check_pass_no_todos() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        let files = vec![make_diff_file("src/lib.rs", vec!["fn main() {}", "let x = 1;"])];
        let report = runner.run_checks(&files);
        assert!(report.overall_passed);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn test_check_fail_todos() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-todos", CheckType::NoTodoComments)
        }).unwrap();
        let files = vec![make_diff_file("src/lib.rs", vec!["// TODO: fix this"])];
        let report = runner.run_checks(&files);
        assert!(!report.overall_passed);
        assert!(report.failed > 0);
    }

    // -- Regex matching --

    #[test]
    fn test_regex_match_found() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(make_rule("regex-test", CheckType::RegexMatch(r"unsafe\s+\{".into()))).unwrap();
        let files = vec![make_diff_file("src/lib.rs", vec!["unsafe {", "safe code"])];
        let report = runner.run_checks(&files);
        assert!(report.results.iter().any(|r| !r.passed));
    }

    #[test]
    fn test_regex_match_not_found() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(make_rule("regex-test", CheckType::RegexMatch(r"unsafe\s+\{".into()))).unwrap();
        let files = vec![make_diff_file("src/lib.rs", vec!["fn safe() {}"])];
        let report = runner.run_checks(&files);
        assert!(report.overall_passed);
    }

    // -- Secret detection --

    #[test]
    fn test_detect_api_key() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoSecrets);
        let files = vec![make_diff_file("config.rs", vec![
            r#"let api_key = "sk-1234567890abcdef1234567890abcdef1234";"#
        ])];
        let report = runner.run_checks(&files);
        assert!(report.results.iter().any(|r| !r.passed && r.message.contains("API key")));
    }

    #[test]
    fn test_detect_bearer_token() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoSecrets);
        let files = vec![make_diff_file("auth.rs", vec![
            "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.abcdef"
        ])];
        let report = runner.run_checks(&files);
        assert!(report.results.iter().any(|r| !r.passed && r.message.contains("bearer token")));
    }

    #[test]
    fn test_detect_password() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoSecrets);
        let files = vec![make_diff_file("db.rs", vec![
            r#"let password = "supersecretpassword123";"#
        ])];
        let report = runner.run_checks(&files);
        assert!(report.results.iter().any(|r| !r.passed && r.message.contains("password")));
    }

    #[test]
    fn test_detect_aws_key() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoSecrets);
        let files = vec![make_diff_file("aws.rs", vec![
            "let key = \"AKIAIOSFODNN7EXAMPLE\";"
        ])];
        let report = runner.run_checks(&files);
        assert!(report.results.iter().any(|r| !r.passed && r.message.contains("AWS")));
    }

    #[test]
    fn test_no_secrets_clean() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoSecrets);
        let files = vec![make_diff_file("clean.rs", vec!["fn main() {}", "let x = 42;"])];
        let report = runner.run_checks(&files);
        assert!(report.overall_passed);
    }

    // -- TODO detection --

    #[test]
    fn test_todo_detection() {
        let runner = CiGateRunner::new(make_config());
        let rule = make_rule("todos", CheckType::NoTodoComments);
        let file = make_diff_file("main.rs", vec!["// TODO: implement", "let x = 1;"]);
        let results = runner.run_single_check(&rule, &[file]);
        assert!(results.iter().any(|r| !r.passed));
    }

    #[test]
    fn test_fixme_detection() {
        let runner = CiGateRunner::new(make_config());
        let rule = make_rule("todos", CheckType::NoTodoComments);
        let file = make_diff_file("main.rs", vec!["// FIXME: broken"]);
        let results = runner.run_single_check(&rule, &[file]);
        assert!(results.iter().any(|r| !r.passed));
    }

    // -- console.log detection --

    #[test]
    fn test_console_log_detection() {
        let runner = CiGateRunner::new(make_config());
        let rule = make_rule("no-console", CheckType::NoConsoleLog);
        let file = make_diff_file("app.js", vec!["console.log('debug');"]);
        let results = runner.run_single_check(&rule, &[file]);
        assert!(results.iter().any(|r| !r.passed));
    }

    #[test]
    fn test_console_log_clean() {
        let runner = CiGateRunner::new(make_config());
        let rule = make_rule("no-console", CheckType::NoConsoleLog);
        let file = make_diff_file("app.js", vec!["logger.info('message');"]);
        let results = runner.run_single_check(&rule, &[file]);
        assert!(results.iter().all(|r| r.passed));
    }

    // -- Report generation --

    #[test]
    fn test_report_all_pass() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        let files = vec![make_diff_file("clean.rs", vec!["fn main() {}"])];
        let report = runner.run_checks(&files);
        assert!(report.overall_passed);
        assert_eq!(report.failed, 0);
        assert!(report.passed > 0);
    }

    #[test]
    fn test_report_some_fail() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-todos", CheckType::NoTodoComments)
        }).unwrap();
        runner.add_rule(make_rule("no-console", CheckType::NoConsoleLog)).unwrap();
        let files = vec![make_diff_file("app.js", vec!["// TODO: fix", "clean code"])];
        let report = runner.run_checks(&files);
        assert!(!report.overall_passed);
    }

    #[test]
    fn test_report_all_fail() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-todos", CheckType::NoTodoComments)
        }).unwrap();
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-console", CheckType::NoConsoleLog)
        }).unwrap();
        let files = vec![make_diff_file("app.js", vec!["// TODO: fix", "console.log('x');"])];
        let report = runner.run_checks(&files);
        assert!(!report.overall_passed);
        assert!(report.failed >= 2);
    }

    // -- Report formatting --

    #[test]
    fn test_format_text() {
        let report = GateReport {
            total_checks: 2, passed: 1, failed: 1, warnings: 0, infos: 0,
            results: vec![
                CheckResult {
                    rule_id: "r1".into(), rule_name: "Rule 1".into(),
                    passed: true, severity: Severity::Info, message: "OK".into(),
                    file_path: None, line_number: None, suggestion: None, duration_ms: 0,
                },
                CheckResult {
                    rule_id: "r2".into(), rule_name: "Rule 2".into(),
                    passed: false, severity: Severity::Error, message: "Bad".into(),
                    file_path: Some("f.rs".into()), line_number: Some(10),
                    suggestion: Some("Fix it".into()), duration_ms: 0,
                },
            ],
            overall_passed: false, duration_ms: 42, timestamp: 0,
        };
        let text = CiGateRunner::format_as_text(&report);
        assert!(text.contains("FAILED"));
        assert!(text.contains("PASS"));
        assert!(text.contains("FAIL"));
        assert!(text.contains("Fix it"));
    }

    #[test]
    fn test_format_markdown() {
        let report = GateReport {
            total_checks: 1, passed: 1, failed: 0, warnings: 0, infos: 0,
            results: vec![CheckResult {
                rule_id: "r1".into(), rule_name: "Rule 1".into(),
                passed: true, severity: Severity::Info, message: "OK".into(),
                file_path: None, line_number: None, suggestion: None, duration_ms: 0,
            }],
            overall_passed: true, duration_ms: 10, timestamp: 0,
        };
        let md = CiGateRunner::format_as_markdown(&report);
        assert!(md.contains("# CI Gate Report: PASSED"));
        assert!(md.contains("| Passed | 1 |"));
    }

    #[test]
    fn test_format_junit() {
        let report = GateReport {
            total_checks: 1, passed: 0, failed: 1, warnings: 0, infos: 0,
            results: vec![CheckResult {
                rule_id: "r1".into(), rule_name: "Rule 1".into(),
                passed: false, severity: Severity::Error, message: "Error found".into(),
                file_path: None, line_number: None, suggestion: None, duration_ms: 0,
            }],
            overall_passed: false, duration_ms: 5, timestamp: 0,
        };
        let junit = CiGateRunner::format_as_junit(&report);
        assert!(junit.contains("<?xml"));
        assert!(junit.contains("<testsuite"));
        assert!(junit.contains("<failure"));
    }

    // -- Exit codes --

    #[test]
    fn test_exit_code_pass() {
        let report = GateReport {
            total_checks: 1, passed: 1, failed: 0, warnings: 0, infos: 0,
            results: vec![], overall_passed: true, duration_ms: 0, timestamp: 0,
        };
        assert_eq!(CiGateRunner::generate_exit_code(&report), 0);
    }

    #[test]
    fn test_exit_code_fail() {
        let report = GateReport {
            total_checks: 1, passed: 0, failed: 1, warnings: 0, infos: 0,
            results: vec![], overall_passed: false, duration_ms: 0, timestamp: 0,
        };
        assert_eq!(CiGateRunner::generate_exit_code(&report), 1);
    }

    #[test]
    fn test_exit_code_warn() {
        let report = GateReport {
            total_checks: 2, passed: 1, failed: 0, warnings: 1, infos: 0,
            results: vec![], overall_passed: true, duration_ms: 0, timestamp: 0,
        };
        assert_eq!(CiGateRunner::generate_exit_code(&report), 2);
    }

    // -- File pattern matching --

    #[test]
    fn test_glob_matches_extension() {
        assert!(CiGateRunner::glob_matches("*.rs", "src/lib.rs"));
        assert!(!CiGateRunner::glob_matches("*.rs", "src/lib.js"));
    }

    #[test]
    fn test_glob_matches_multi_extension() {
        assert!(CiGateRunner::glob_matches("*.{js,ts,tsx}", "app.tsx"));
        assert!(!CiGateRunner::glob_matches("*.{js,ts,tsx}", "app.rs"));
    }

    #[test]
    fn test_file_pattern_filters() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            pattern: Some("*.js".into()),
            ..make_rule("js-only", CheckType::NoConsoleLog)
        }).unwrap();
        let files = vec![
            make_diff_file("app.js", vec!["console.log('x');"]),
            make_diff_file("app.rs", vec!["console.log('x');"]),
        ];
        let report = runner.run_checks(&files);
        // Only the .js file should be checked
        let failed_files: Vec<_> = report.results.iter()
            .filter(|r| !r.passed)
            .filter_map(|r| r.file_path.as_deref())
            .collect();
        assert!(failed_files.contains(&"app.js"));
        assert!(!failed_files.contains(&"app.rs"));
    }

    // -- Severity handling --

    #[test]
    fn test_warning_severity_does_not_fail() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            severity: Severity::Warning,
            ..make_rule("warn-todos", CheckType::NoTodoComments)
        }).unwrap();
        let files = vec![make_diff_file("lib.rs", vec!["// TODO: later"])];
        let report = runner.run_checks(&files);
        // Warnings don't cause overall failure when fail_on_error=true
        assert!(report.overall_passed);
        assert!(report.warnings > 0);
    }

    // -- Multiple rules on same file --

    #[test]
    fn test_multiple_rules_same_file() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-todos", CheckType::NoTodoComments)
        }).unwrap();
        runner.add_rule(make_rule("no-console", CheckType::NoConsoleLog)).unwrap();
        let files = vec![make_diff_file("app.js", vec![
            "// TODO: fix this",
            "console.log('debug');",
        ])];
        let report = runner.run_checks(&files);
        let failed: Vec<_> = report.results.iter().filter(|r| !r.passed).collect();
        assert!(failed.len() >= 2);
    }

    // -- Empty diff --

    #[test]
    fn test_empty_diff() {
        let mut runner = CiGateRunner::new(make_config());
        runner.load_prebuilt(PrebuiltRule::NoTodos);
        let report = runner.run_checks(&[]);
        assert!(report.overall_passed);
    }

    // -- Error cases --

    #[test]
    fn test_gate_error_display() {
        assert_eq!(GateError::RuleNotFound.to_string(), "rule not found");
        assert_eq!(GateError::DuplicateRule.to_string(), "duplicate rule");
        assert_eq!(GateError::TimeoutExceeded.to_string(), "timeout exceeded");
        assert_eq!(GateError::InvalidConfig.to_string(), "invalid config");
    }

    #[test]
    fn test_github_action_yaml() {
        let yaml = CiGateRunner::generate_github_action_yaml();
        assert!(yaml.contains("VibeCody CI Gates"));
        assert!(yaml.contains("vibecli ci-gate"));
        assert!(yaml.contains("pull_request"));
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Text.to_string(), "text");
        assert_eq!(OutputFormat::Markdown.to_string(), "markdown");
        assert_eq!(OutputFormat::JUnit.to_string(), "junit");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }

    #[test]
    fn test_rule_category_display() {
        assert_eq!(RuleCategory::Security.to_string(), "security");
        assert_eq!(RuleCategory::CodeQuality.to_string(), "code-quality");
    }

    #[test]
    fn test_disabled_rule_skipped() {
        let mut runner = CiGateRunner::new(make_config());
        runner.add_rule(CiRule {
            enabled: false,
            ..make_rule("disabled", CheckType::NoTodoComments)
        }).unwrap();
        let files = vec![make_diff_file("lib.rs", vec!["// TODO: this should not be caught"])];
        let report = runner.run_checks(&files);
        // Disabled rule produces no results
        assert_eq!(report.total_checks, 0);
    }

    #[test]
    fn test_fail_on_error_false() {
        let cfg = GateConfig { fail_on_error: false, ..GateConfig::default() };
        let mut runner = CiGateRunner::new(cfg);
        runner.add_rule(CiRule {
            severity: Severity::Error,
            ..make_rule("no-todos", CheckType::NoTodoComments)
        }).unwrap();
        let files = vec![make_diff_file("lib.rs", vec!["// TODO: fix"])];
        let report = runner.run_checks(&files);
        // Even with errors, overall_passed is true when fail_on_error=false
        assert!(report.overall_passed);
    }
}
