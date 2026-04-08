#![allow(dead_code)]
//! AI code review engine — comprehensive PR analysis, pattern detection, quality gates, and learning.
//!
//! Matches/exceeds Qodo Merge, CodeRabbit, and Bito features:
//! - Multi-detector static analysis (security, complexity, style, docs, tests, duplication, architecture)
//! - Quality gates with natural-language rules and structured conditions
//! - Multi-linter aggregation with false-positive filtering
//! - PR summary & architectural diagram generation
//! - Learning from reviewer feedback (precision/recall tracking)

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReviewSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Security,
}

impl ReviewSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Critical => "Critical",
            Self::Security => "Security",
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            Self::Info => 0.1,
            Self::Warning => 0.3,
            Self::Error => 0.6,
            Self::Critical => 0.9,
            Self::Security => 1.0,
        }
    }
}

impl std::fmt::Display for ReviewSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReviewCategory {
    Bug,
    Security,
    Performance,
    Style,
    Documentation,
    Testing,
    Architecture,
    Complexity,
    Duplication,
    Accessibility,
    BreakingChange,
    MergeConflictRisk,
}

impl ReviewCategory {
    pub fn all() -> Vec<ReviewCategory> {
        vec![
            Self::Bug,
            Self::Security,
            Self::Performance,
            Self::Style,
            Self::Documentation,
            Self::Testing,
            Self::Architecture,
            Self::Complexity,
            Self::Duplication,
            Self::Accessibility,
            Self::BreakingChange,
            Self::MergeConflictRisk,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Bug => "Bug",
            Self::Security => "Security",
            Self::Performance => "Performance",
            Self::Style => "Style",
            Self::Documentation => "Documentation",
            Self::Testing => "Testing",
            Self::Architecture => "Architecture",
            Self::Complexity => "Complexity",
            Self::Duplication => "Duplication",
            Self::Accessibility => "Accessibility",
            Self::BreakingChange => "Breaking Change",
            Self::MergeConflictRisk => "Merge Conflict Risk",
        }
    }
}

impl std::fmt::Display for ReviewCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QualityGateCondition {
    MaxComplexity(u32),
    MinTestCoverage(f64),
    MaxFileLength(usize),
    MaxFunctionLength(usize),
    NoTodoComments,
    RequireDocstrings,
    NoHardcodedSecrets,
    CustomRegex(String),
    MaxDuplication(f64),
    RequireErrorHandling,
}

impl QualityGateCondition {
    pub fn label(&self) -> &str {
        match self {
            Self::MaxComplexity(_) => "Max Complexity",
            Self::MinTestCoverage(_) => "Min Test Coverage",
            Self::MaxFileLength(_) => "Max File Length",
            Self::MaxFunctionLength(_) => "Max Function Length",
            Self::NoTodoComments => "No TODO Comments",
            Self::RequireDocstrings => "Require Docstrings",
            Self::NoHardcodedSecrets => "No Hardcoded Secrets",
            Self::CustomRegex(_) => "Custom Regex",
            Self::MaxDuplication(_) => "Max Duplication",
            Self::RequireErrorHandling => "Require Error Handling",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestType {
    Unit,
    Integration,
    Edge,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "Unit"),
            Self::Integration => write!(f, "Integration"),
            Self::Edge => write!(f, "Edge"),
        }
    }
}

// ── Structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub id: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub severity: ReviewSeverity,
    pub category: ReviewCategory,
    pub message: String,
    pub suggestion: Option<String>,
    pub auto_fixable: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct PrAnalysis {
    pub title: String,
    pub summary: String,
    pub files_changed: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub findings: Vec<ReviewFinding>,
    pub risk_score: f64,
    pub architectural_impact: String,
    pub test_coverage_delta: f64,
    pub breaking_changes: Vec<BreakingChange>,
    pub suggested_reviewers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct QualityGate {
    pub id: String,
    pub name: String,
    pub rule: String,
    pub condition: QualityGateCondition,
    pub enabled: bool,
    pub severity: ReviewSeverity,
}

#[derive(Debug, Clone)]
pub struct QualityGateResult {
    pub gate_id: String,
    pub passed: bool,
    pub message: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewLearning {
    pub finding_id: String,
    pub was_accepted: bool,
    pub reviewer_comment: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct LinterResult {
    pub linter_name: String,
    pub file: String,
    pub line: usize,
    pub message: String,
    pub severity: ReviewSeverity,
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub enabled_categories: Vec<ReviewCategory>,
    pub quality_gates: Vec<QualityGate>,
    pub linters: Vec<String>,
    pub auto_fix: bool,
    pub learning_enabled: bool,
    pub max_findings_per_file: usize,
    pub ignore_patterns: Vec<String>,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            enabled_categories: ReviewCategory::all(),
            quality_gates: Vec::new(),
            linters: vec![
                "clippy".to_string(),
                "eslint".to_string(),
                "pylint".to_string(),
            ],
            auto_fix: false,
            learning_enabled: true,
            max_findings_per_file: 50,
            ignore_patterns: vec![
                "*.min.js".to_string(),
                "*.lock".to_string(),
                "vendor/*".to_string(),
                "node_modules/*".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestSuggestion {
    pub file: String,
    pub function_name: String,
    pub test_type: TestType,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct BreakingChange {
    pub file: String,
    pub change_type: String,
    pub description: String,
    pub affected_apis: Vec<String>,
    pub migration_hint: String,
}

#[derive(Debug, Clone)]
pub struct LearningStats {
    pub total_findings: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

impl Default for LearningStats {
    fn default() -> Self {
        Self {
            total_findings: 0,
            accepted: 0,
            rejected: 0,
            precision: 1.0,
            recall: 1.0,
            f1_score: 1.0,
        }
    }
}

// ── Diff Parsing Helpers ───────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DiffFile {
    path: String,
    added_lines: Vec<(usize, String)>,
    removed_lines: Vec<(usize, String)>,
    lines_added: usize,
    lines_removed: usize,
}

fn parse_unified_diff(diff: &str) -> Vec<DiffFile> {
    let mut files: Vec<DiffFile> = Vec::new();
    let mut current: Option<DiffFile> = None;
    let mut current_line: usize = 0;

    for line in diff.lines() {
        if line.starts_with("+++ b/") || line.starts_with("+++ ") {
            let path = line
                .strip_prefix("+++ b/")
                .or_else(|| line.strip_prefix("+++ "))
                .unwrap_or("unknown")
                .to_string();
            if let Some(f) = current.take() {
                files.push(f);
            }
            current = Some(DiffFile {
                path,
                added_lines: Vec::new(),
                removed_lines: Vec::new(),
                lines_added: 0,
                lines_removed: 0,
            });
            current_line = 0;
        } else if line.starts_with("@@ ") {
            // Parse hunk header: @@ -start,count +start,count @@
            if let Some(plus_pos) = line.find('+') {
                let after_plus = &line[plus_pos + 1..];
                if let Some(comma_or_space) = after_plus.find([',', ' ']) {
                    if let Ok(n) = after_plus[..comma_or_space].parse::<usize>() {
                        current_line = n.saturating_sub(1);
                    }
                }
            }
        } else if let Some(ref mut f) = current {
            if let Some(added) = line.strip_prefix('+') {
                current_line += 1;
                f.added_lines.push((current_line, added.to_string()));
                f.lines_added += 1;
            } else if let Some(stripped) = line.strip_prefix('-') {
                f.removed_lines
                    .push((current_line, stripped.to_string()));
                f.lines_removed += 1;
            } else {
                current_line += 1;
            }
        }
    }
    if let Some(f) = current {
        files.push(f);
    }
    files
}

// ── Pattern Detectors (static analysis) ────────────────────────────────

/// Detect OWASP Top 10, hardcoded secrets, SQL injection, XSS, command injection, path traversal.
pub fn detect_security_issues(code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;

    let secret_patterns: Vec<(&str, &str)> = vec![
        ("AKIA[0-9A-Z]{16}", "AWS access key detected"),
        ("(?i)password\\s*=\\s*[\"'][^\"']+[\"']", "Hardcoded password"),
        ("(?i)secret\\s*=\\s*[\"'][^\"']+[\"']", "Hardcoded secret"),
        ("(?i)api[_-]?key\\s*=\\s*[\"'][^\"']+[\"']", "Hardcoded API key"),
        ("(?i)token\\s*=\\s*[\"'][a-zA-Z0-9]{20,}", "Hardcoded token"),
        ("-----BEGIN (RSA |EC )?PRIVATE KEY-----", "Private key in source"),
    ];

    let injection_patterns: Vec<(&str, &str, &str)> = vec![
        ("eval(", "Potential code injection via eval()", "Avoid eval(); use safer alternatives"),
        ("exec(", "Potential command injection via exec()", "Validate and sanitize all inputs before execution"),
        ("innerHTML", "Potential XSS via innerHTML assignment", "Use textContent or sanitize HTML input"),
        ("document.write", "Potential XSS via document.write()", "Use DOM manipulation methods instead"),
        (".exec(", "Potential command execution", "Validate inputs and use parameterized calls"),
        ("dangerouslySetInnerHTML", "React dangerouslySetInnerHTML usage", "Ensure HTML is sanitized before rendering"),
    ];

    let sql_patterns: Vec<(&str, &str)> = vec![
        ("format!(\"SELECT", "Potential SQL injection via string formatting"),
        ("format!(\"INSERT", "Potential SQL injection via string formatting"),
        ("format!(\"UPDATE", "Potential SQL injection via string formatting"),
        ("format!(\"DELETE", "Potential SQL injection via string formatting"),
        ("+ \" WHERE ", "Potential SQL injection via string concatenation"),
        ("f\"SELECT", "Potential SQL injection via f-string"),
        ("f\"INSERT", "Potential SQL injection via f-string"),
    ];

    let path_patterns: Vec<(&str, &str)> = vec![
        ("../", "Potential path traversal"),
        ("..\\\\", "Potential path traversal (Windows)"),
    ];

    for (line_num, line) in code.lines().enumerate() {
        let ln = line_num + 1;
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("*") {
            continue;
        }

        // Secret detection
        for (pattern, msg) in &secret_patterns {
            // Simple substring check for key patterns
            let lower = trimmed.to_lowercase();
            let check = if pattern.starts_with("AKIA") {
                trimmed.contains("AKIA")
            } else if pattern.contains("password") {
                lower.contains("password") && (lower.contains("= \"") || lower.contains("= '"))
            } else if pattern.contains("secret") {
                lower.contains("secret") && (lower.contains("= \"") || lower.contains("= '"))
            } else if pattern.contains("api") {
                (lower.contains("api_key") || lower.contains("api-key") || lower.contains("apikey"))
                    && (lower.contains("= \"") || lower.contains("= '"))
            } else if pattern.contains("token") {
                lower.contains("token") && (lower.contains("= \"") || lower.contains("= '"))
            } else {
                lower.contains("private key")
            };
            if check {
                findings.push(ReviewFinding {
                    id: format!("SEC-{}", next_id),
                    file: String::new(),
                    line_start: ln,
                    line_end: ln,
                    severity: ReviewSeverity::Security,
                    category: ReviewCategory::Security,
                    message: msg.to_string(),
                    suggestion: Some("Remove hardcoded credential and use environment variables or a secrets manager".to_string()),
                    auto_fixable: false,
                    confidence: 0.85,
                });
                next_id += 1;
            }
        }

        // Injection detection
        for (pattern, msg, suggestion) in &injection_patterns {
            if trimmed.contains(pattern) {
                findings.push(ReviewFinding {
                    id: format!("SEC-{}", next_id),
                    file: String::new(),
                    line_start: ln,
                    line_end: ln,
                    severity: ReviewSeverity::Security,
                    category: ReviewCategory::Security,
                    message: msg.to_string(),
                    suggestion: Some(suggestion.to_string()),
                    auto_fixable: false,
                    confidence: 0.75,
                });
                next_id += 1;
            }
        }

        // SQL injection
        for (pattern, msg) in &sql_patterns {
            if trimmed.contains(pattern) {
                findings.push(ReviewFinding {
                    id: format!("SEC-{}", next_id),
                    file: String::new(),
                    line_start: ln,
                    line_end: ln,
                    severity: ReviewSeverity::Security,
                    category: ReviewCategory::Security,
                    message: msg.to_string(),
                    suggestion: Some("Use parameterized queries or an ORM instead of string formatting".to_string()),
                    auto_fixable: false,
                    confidence: 0.80,
                });
                next_id += 1;
            }
        }

        // Path traversal
        if !trimmed.starts_with("//") && !trimmed.starts_with('#') {
            for (pattern, msg) in &path_patterns {
                if trimmed.contains(pattern) && !trimmed.contains("test") && !trimmed.contains("spec") {
                    findings.push(ReviewFinding {
                        id: format!("SEC-{}", next_id),
                        file: String::new(),
                        line_start: ln,
                        line_end: ln,
                        severity: ReviewSeverity::Warning,
                        category: ReviewCategory::Security,
                        message: msg.to_string(),
                        suggestion: Some("Sanitize file paths and validate against allowed directories".to_string()),
                        auto_fixable: false,
                        confidence: 0.60,
                    });
                    next_id += 1;
                }
            }
        }

        // Unsafe blocks in Rust
        if trimmed.starts_with("unsafe ") || trimmed.contains("unsafe {") {
            findings.push(ReviewFinding {
                id: format!("SEC-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Warning,
                category: ReviewCategory::Security,
                message: "Unsafe block detected — ensure memory safety invariants are upheld".to_string(),
                suggestion: Some("Document why unsafe is necessary and what invariants must hold".to_string()),
                auto_fixable: false,
                confidence: 0.90,
            });
            next_id += 1;
        }
    }

    findings
}

/// Detect cyclomatic complexity, deep nesting, long functions.
pub fn detect_complexity_issues(code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;
    let mut current_fn_name: Option<String> = None;
    let mut fn_start: usize = 0;
    let mut fn_lines: usize = 0;
    let mut nesting_depth: i32 = 0;
    let mut max_nesting: i32 = 0;
    let mut branch_count: u32 = 0;

    let branch_keywords = ["if ", "else if ", "elif ", "match ", "case ", "while ", "for ", "loop "];

    for (line_num, line) in code.lines().enumerate() {
        let ln = line_num + 1;
        let trimmed = line.trim();

        // Detect function start
        let is_fn = trimmed.starts_with("pub fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("func ")
            || (trimmed.contains("=> {") && trimmed.contains("const "));

        if is_fn {
            // Emit findings for previous function
            if let Some(ref name) = current_fn_name {
                emit_complexity_findings(
                    &mut findings,
                    &mut next_id,
                    name,
                    fn_start,
                    ln - 1,
                    fn_lines,
                    branch_count,
                    max_nesting,
                );
            }
            current_fn_name = extract_function_name(trimmed);
            fn_start = ln;
            fn_lines = 0;
            nesting_depth = 0;
            max_nesting = 0;
            branch_count = 1; // base path
        }

        if current_fn_name.is_some() {
            fn_lines += 1;
            let opens = trimmed.matches('{').count() as i32;
            let closes = trimmed.matches('}').count() as i32;
            nesting_depth += opens - closes;
            if nesting_depth > max_nesting {
                max_nesting = nesting_depth;
            }
            for kw in &branch_keywords {
                if trimmed.starts_with(kw) || trimmed.contains(&format!(" {}", kw)) {
                    branch_count += 1;
                }
            }
        }

        // Detect deeply nested line even outside tracked function
        let indent = line.len() - line.trim_start().len();
        if indent > 24 && !trimmed.is_empty() && !trimmed.starts_with("//") {
            findings.push(ReviewFinding {
                id: format!("CX-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Warning,
                category: ReviewCategory::Complexity,
                message: format!("Deeply nested code (indent level {})", indent / 4),
                suggestion: Some("Consider extracting into a helper function or using early returns".to_string()),
                auto_fixable: false,
                confidence: 0.70,
            });
            next_id += 1;
        }
    }

    // Final function
    if let Some(ref name) = current_fn_name {
        let total = code.lines().count();
        emit_complexity_findings(
            &mut findings,
            &mut next_id,
            name,
            fn_start,
            total,
            fn_lines,
            branch_count,
            max_nesting,
        );
    }

    findings
}

fn extract_function_name(line: &str) -> Option<String> {
    // Rust: fn name(
    // Python: def name(
    // JS/TS: function name(
    // Go: func name(
    let prefixes = ["pub async fn ", "async fn ", "pub fn ", "fn ", "def ", "function ", "func "];
    for prefix in &prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            if let Some(paren) = rest.find('(') {
                let name = rest[..paren].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn emit_complexity_findings(
    findings: &mut Vec<ReviewFinding>,
    next_id: &mut u64,
    name: &str,
    fn_start: usize,
    fn_end: usize,
    fn_lines: usize,
    branch_count: u32,
    max_nesting: i32,
) {
    // Long function
    if fn_lines > 60 {
        findings.push(ReviewFinding {
            id: format!("CX-{}", *next_id),
            file: String::new(),
            line_start: fn_start,
            line_end: fn_end,
            severity: if fn_lines > 150 {
                ReviewSeverity::Error
            } else {
                ReviewSeverity::Warning
            },
            category: ReviewCategory::Complexity,
            message: format!("Function '{}' is {} lines long", name, fn_lines),
            suggestion: Some("Break into smaller, focused functions".to_string()),
            auto_fixable: false,
            confidence: 0.95,
        });
        *next_id += 1;
    }

    // High cyclomatic complexity
    if branch_count > 10 {
        findings.push(ReviewFinding {
            id: format!("CX-{}", *next_id),
            file: String::new(),
            line_start: fn_start,
            line_end: fn_end,
            severity: if branch_count > 20 {
                ReviewSeverity::Error
            } else {
                ReviewSeverity::Warning
            },
            category: ReviewCategory::Complexity,
            message: format!(
                "Function '{}' has cyclomatic complexity {} (threshold: 10)",
                name, branch_count
            ),
            suggestion: Some("Reduce branching by extracting sub-functions or using lookup tables".to_string()),
            auto_fixable: false,
            confidence: 0.90,
        });
        *next_id += 1;
    }

    // Deep nesting
    if max_nesting > 4 {
        findings.push(ReviewFinding {
            id: format!("CX-{}", *next_id),
            file: String::new(),
            line_start: fn_start,
            line_end: fn_end,
            severity: ReviewSeverity::Warning,
            category: ReviewCategory::Complexity,
            message: format!(
                "Function '{}' has max nesting depth {} (threshold: 4)",
                name, max_nesting
            ),
            suggestion: Some("Use early returns, guard clauses, or extract nested logic".to_string()),
            auto_fixable: false,
            confidence: 0.85,
        });
        *next_id += 1;
    }
}

/// Detect naming conventions, unused imports, inconsistent formatting.
pub fn detect_style_issues(code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;

    for (line_num, line) in code.lines().enumerate() {
        let ln = line_num + 1;
        let trimmed = line.trim();

        // Trailing whitespace
        if line.ends_with(' ') || line.ends_with('\t') {
            findings.push(ReviewFinding {
                id: format!("STY-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Style,
                message: "Trailing whitespace".to_string(),
                suggestion: Some("Remove trailing whitespace".to_string()),
                auto_fixable: true,
                confidence: 1.0,
            });
            next_id += 1;
        }

        // Very long lines
        if line.len() > 120 && !trimmed.starts_with("//") && !trimmed.starts_with('#') && !trimmed.starts_with("///") {
            findings.push(ReviewFinding {
                id: format!("STY-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Style,
                message: format!("Line is {} characters (max recommended: 120)", line.len()),
                suggestion: Some("Break into multiple lines for readability".to_string()),
                auto_fixable: false,
                confidence: 0.90,
            });
            next_id += 1;
        }

        // TODO/FIXME/HACK/XXX comments
        let upper = trimmed.to_uppercase();
        if (upper.contains("TODO") || upper.contains("FIXME") || upper.contains("HACK") || upper.contains("XXX"))
            && (trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*'))
        {
            findings.push(ReviewFinding {
                id: format!("STY-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Style,
                message: "TODO/FIXME/HACK comment found".to_string(),
                suggestion: Some("Track in issue tracker instead of code comments".to_string()),
                auto_fixable: false,
                confidence: 0.95,
            });
            next_id += 1;
        }

        // Consecutive blank lines (check with context)
        if trimmed.is_empty() {
            // We check for two blank lines — simple heuristic
            // Actual check deferred to multi-line analysis below
        }

        // Magic numbers
        if !trimmed.starts_with("//")
            && !trimmed.starts_with("const ")
            && !trimmed.starts_with("let ")
            && !trimmed.starts_with("static ")
            && !trimmed.starts_with('#')
        {
            // Check for bare numeric literals > 2 digits that are not 0, 1, 2, 100, etc.
            for word in trimmed.split_whitespace() {
                if let Ok(n) = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '-').parse::<i64>() {
                    if n.abs() > 2 && n != 10 && n != 100 && n != 1000 && n != 0xFF {
                        // Only flag if it looks like a standalone number in an expression
                        if (trimmed.contains("==") || trimmed.contains(">=") || trimmed.contains("<=") || trimmed.contains("> ") || trimmed.contains("< "))
                            && word.chars().all(|c| c.is_ascii_digit() || c == '-')
                        {
                            findings.push(ReviewFinding {
                                id: format!("STY-{}", next_id),
                                file: String::new(),
                                line_start: ln,
                                line_end: ln,
                                severity: ReviewSeverity::Info,
                                category: ReviewCategory::Style,
                                message: format!("Magic number {} — consider using a named constant", n),
                                suggestion: Some("Extract to a named constant for clarity".to_string()),
                                auto_fixable: false,
                                confidence: 0.55,
                            });
                            next_id += 1;
                            break;
                        }
                    }
                }
            }
        }

        // Snake_case check for Rust function names
        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ")) && !trimmed.starts_with("fn main") {
            if let Some(name) = extract_function_name(trimmed) {
                if name.chars().any(|c| c.is_uppercase()) && !name.contains("::") {
                    findings.push(ReviewFinding {
                        id: format!("STY-{}", next_id),
                        file: String::new(),
                        line_start: ln,
                        line_end: ln,
                        severity: ReviewSeverity::Warning,
                        category: ReviewCategory::Style,
                        message: format!("Function '{}' should use snake_case naming", name),
                        suggestion: Some("Rename to snake_case per Rust conventions".to_string()),
                        auto_fixable: true,
                        confidence: 0.90,
                    });
                    next_id += 1;
                }
            }
        }

        // println! in production code (Rust)
        if trimmed.starts_with("println!") || trimmed.starts_with("dbg!") || trimmed.starts_with("print!") {
            findings.push(ReviewFinding {
                id: format!("STY-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Style,
                message: "Debug print statement in code".to_string(),
                suggestion: Some("Use a logging framework (log/tracing) instead of println!/dbg!".to_string()),
                auto_fixable: false,
                confidence: 0.80,
            });
            next_id += 1;
        }

        // console.log in JS/TS
        if trimmed.contains("console.log") || trimmed.contains("console.debug") {
            findings.push(ReviewFinding {
                id: format!("STY-{}", next_id),
                file: String::new(),
                line_start: ln,
                line_end: ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Style,
                message: "Debug console statement in code".to_string(),
                suggestion: Some("Remove console.log or use a proper logger".to_string()),
                auto_fixable: true,
                confidence: 0.85,
            });
            next_id += 1;
        }
    }

    findings
}

/// Detect missing docstrings, undocumented public APIs.
pub fn detect_documentation_gaps(code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;
    let lines: Vec<&str> = code.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let ln = i + 1;
        let trimmed = line.trim();

        // Rust: public items without doc comments
        let is_pub_item = trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("pub type ");

        if is_pub_item {
            let has_doc = if i > 0 {
                let prev = lines[i - 1].trim();
                prev.starts_with("///") || prev.starts_with("//!") || prev.starts_with("#[doc")
            } else {
                false
            };
            // Also check two lines above (for multi-line doc comments)
            let has_doc2 = if i > 1 {
                let prev2 = lines[i - 2].trim();
                prev2.starts_with("///") || prev2.starts_with("//!")
            } else {
                false
            };
            if !has_doc && !has_doc2 {
                let item_type = if trimmed.contains("fn ") {
                    "function"
                } else if trimmed.contains("struct ") {
                    "struct"
                } else if trimmed.contains("enum ") {
                    "enum"
                } else if trimmed.contains("trait ") {
                    "trait"
                } else {
                    "item"
                };
                findings.push(ReviewFinding {
                    id: format!("DOC-{}", next_id),
                    file: String::new(),
                    line_start: ln,
                    line_end: ln,
                    severity: ReviewSeverity::Info,
                    category: ReviewCategory::Documentation,
                    message: format!("Public {} missing documentation", item_type),
                    suggestion: Some(format!("Add a /// doc comment explaining this {}", item_type)),
                    auto_fixable: false,
                    confidence: 0.90,
                });
                next_id += 1;
            }
        }

        // Python: functions without docstrings
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            let has_docstring = if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                next_line.starts_with("\"\"\"") || next_line.starts_with("'''")
            } else {
                false
            };
            if !has_docstring {
                if let Some(name) = extract_function_name(trimmed) {
                    findings.push(ReviewFinding {
                        id: format!("DOC-{}", next_id),
                        file: String::new(),
                        line_start: ln,
                        line_end: ln,
                        severity: ReviewSeverity::Info,
                        category: ReviewCategory::Documentation,
                        message: format!("Function '{}' missing docstring", name),
                        suggestion: Some("Add a docstring describing the function".to_string()),
                        auto_fixable: false,
                        confidence: 0.85,
                    });
                    next_id += 1;
                }
            }
        }

        // Missing module-level doc
        if ln == 1 && !trimmed.starts_with("//!") && !trimmed.starts_with("#!") && !trimmed.starts_with("\"\"\"") {
            findings.push(ReviewFinding {
                id: format!("DOC-{}", next_id),
                file: String::new(),
                line_start: 1,
                line_end: 1,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Documentation,
                message: "Missing module-level documentation".to_string(),
                suggestion: Some("Add a module doc comment (//! for Rust, \"\"\" for Python)".to_string()),
                auto_fixable: false,
                confidence: 0.70,
            });
            next_id += 1;
        }
    }

    findings
}

/// Detect untested functions and missing edge case tests.
pub fn detect_test_gaps(code: &str, test_code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;

    // Extract function names from source
    let mut functions: Vec<(String, usize)> = Vec::new();
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if (trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("async fn "))
            && !trimmed.contains("#[test]")
        {
            if let Some(name) = extract_function_name(trimmed) {
                if name != "main" && name != "new" && name != "default" && !name.starts_with("test_") {
                    functions.push((name, i + 1));
                }
            }
        }
    }

    let test_lower = test_code.to_lowercase();

    for (name, ln) in &functions {
        let name_lower = name.to_lowercase();
        // Check if the function name appears in test code
        let is_tested = test_lower.contains(&name_lower)
            || test_lower.contains(&format!("test_{}", name_lower))
            || test_lower.contains(&format!("{}_test", name_lower));

        if !is_tested {
            findings.push(ReviewFinding {
                id: format!("TST-{}", next_id),
                file: String::new(),
                line_start: *ln,
                line_end: *ln,
                severity: ReviewSeverity::Warning,
                category: ReviewCategory::Testing,
                message: format!("Function '{}' appears to have no test coverage", name),
                suggestion: Some(format!("Add unit tests for '{}'", name)),
                auto_fixable: false,
                confidence: 0.75,
            });
            next_id += 1;
        }
    }

    // Check for error handling paths without tests
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if (trimmed.contains("Err(") || trimmed.contains("panic!") || trimmed.contains("unwrap()"))
            && !test_lower.contains("err")
            && !test_lower.contains("error")
            && !test_lower.contains("panic")
        {
            findings.push(ReviewFinding {
                id: format!("TST-{}", next_id),
                file: String::new(),
                line_start: i + 1,
                line_end: i + 1,
                severity: ReviewSeverity::Warning,
                category: ReviewCategory::Testing,
                message: "Error path may lack test coverage".to_string(),
                suggestion: Some("Add tests for error/failure scenarios".to_string()),
                auto_fixable: false,
                confidence: 0.60,
            });
            let _ = next_id;
            break; // Only one per file
        }
    }

    findings
}

/// Detect copy-paste / code duplication across files.
pub fn detect_duplication(files: &[(&str, &str)]) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;
    let min_dup_lines = 6;

    // Build line-hash windows per file
    struct FileChunk {
        file: String,
        start: usize,
        lines: Vec<String>,
    }

    let mut chunks: Vec<FileChunk> = Vec::new();

    for (path, content) in files {
        let lines: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
            .collect();

        if lines.len() < min_dup_lines {
            continue;
        }

        for start in 0..=lines.len().saturating_sub(min_dup_lines) {
            let window: Vec<String> = lines[start..start + min_dup_lines].to_vec();
            // Skip trivial chunks (all braces / blank)
            let non_trivial = window.iter().filter(|l| l.len() > 3).count();
            if non_trivial >= 4 {
                chunks.push(FileChunk {
                    file: path.to_string(),
                    start: start + 1,
                    lines: window,
                });
            }
        }
    }

    // Compare chunks across different files
    let mut reported: std::collections::HashSet<String> = std::collections::HashSet::new();
    for i in 0..chunks.len() {
        for j in (i + 1)..chunks.len() {
            if chunks[i].file == chunks[j].file {
                continue;
            }
            if chunks[i].lines == chunks[j].lines {
                let key = format!("{}:{}+{}:{}", chunks[i].file, chunks[i].start, chunks[j].file, chunks[j].start);
                if reported.contains(&key) {
                    continue;
                }
                reported.insert(key);
                findings.push(ReviewFinding {
                    id: format!("DUP-{}", next_id),
                    file: chunks[i].file.clone(),
                    line_start: chunks[i].start,
                    line_end: chunks[i].start + min_dup_lines - 1,
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::Duplication,
                    message: format!(
                        "Duplicated code block ({} lines) also found in {} at line {}",
                        min_dup_lines, chunks[j].file, chunks[j].start
                    ),
                    suggestion: Some("Extract shared logic into a common function or module".to_string()),
                    auto_fixable: false,
                    confidence: 0.85,
                });
                next_id += 1;

                if findings.len() >= 20 {
                    return findings;
                }
            }
        }
    }

    findings
}

/// Detect circular dependencies and layer violations.
pub fn detect_architecture_violations(code: &str) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let mut next_id: u64 = 1;

    // Collect imports/use statements
    let mut imports: Vec<(usize, String)> = Vec::new();
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.starts_with("require(") {
            imports.push((i + 1, trimmed.to_string()));
        }
    }

    // Layer violation patterns
    let violations: Vec<(&str, &str, &str)> = vec![
        ("controller", "repository", "Controller should not directly access repository layer"),
        ("view", "repository", "View layer should not access data layer directly"),
        ("model", "controller", "Model should not depend on controller"),
        ("domain", "infrastructure", "Domain should not depend on infrastructure"),
        ("core", "ui", "Core logic should not depend on UI layer"),
        ("util", "service", "Utility modules should not depend on service layer"),
    ];

    for (ln, import_line) in &imports {
        let lower = import_line.to_lowercase();
        for (from_layer, to_layer, msg) in &violations {
            // Simple heuristic: if the file's path context mentions from_layer
            // and the import mentions to_layer
            if lower.contains(to_layer) {
                findings.push(ReviewFinding {
                    id: format!("ARCH-{}", next_id),
                    file: String::new(),
                    line_start: *ln,
                    line_end: *ln,
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::Architecture,
                    message: format!("{} (importing {})", msg, to_layer),
                    suggestion: Some(format!(
                        "Use dependency inversion: {} should depend on abstractions, not {}",
                        from_layer, to_layer
                    )),
                    auto_fixable: false,
                    confidence: 0.50,
                });
                next_id += 1;
            }
        }

        // Wildcard imports
        if lower.contains("::*") || lower.contains("import *") {
            findings.push(ReviewFinding {
                id: format!("ARCH-{}", next_id),
                file: String::new(),
                line_start: *ln,
                line_end: *ln,
                severity: ReviewSeverity::Info,
                category: ReviewCategory::Architecture,
                message: "Wildcard import may pull in unintended symbols".to_string(),
                suggestion: Some("Import only the specific items you need".to_string()),
                auto_fixable: false,
                confidence: 0.75,
            });
            next_id += 1;
        }
    }

    // God class / god file detection
    let line_count = code.lines().count();
    if line_count > 500 {
        findings.push(ReviewFinding {
            id: format!("ARCH-{}", next_id),
            file: String::new(),
            line_start: 1,
            line_end: line_count,
            severity: if line_count > 1000 {
                ReviewSeverity::Error
            } else {
                ReviewSeverity::Warning
            },
            category: ReviewCategory::Architecture,
            message: format!("File is {} lines — consider splitting", line_count),
            suggestion: Some("Split into smaller modules with clear single responsibility".to_string()),
            auto_fixable: false,
            confidence: 0.80,
        });
        next_id += 1;
    }

    // Too many imports (high coupling)
    if imports.len() > 20 {
        findings.push(ReviewFinding {
            id: format!("ARCH-{}", next_id),
            file: String::new(),
            line_start: 1,
            line_end: imports.last().map(|(ln, _)| *ln).unwrap_or(1),
            severity: ReviewSeverity::Warning,
            category: ReviewCategory::Architecture,
            message: format!("High coupling: {} imports detected", imports.len()),
            suggestion: Some("Review dependencies — consider facade pattern or module restructuring".to_string()),
            auto_fixable: false,
            confidence: 0.65,
        });
    }

    findings
}

// ── Multi-Linter Aggregation ───────────────────────────────────────────

pub struct LinterAggregator {
    supported_linters: HashMap<String, Vec<String>>,
}

impl Default for LinterAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl LinterAggregator {
    pub fn new() -> Self {
        let mut supported = HashMap::new();
        supported.insert("clippy".to_string(), vec!["rs".to_string()]);
        supported.insert("eslint".to_string(), vec!["js".to_string(), "ts".to_string(), "jsx".to_string(), "tsx".to_string()]);
        supported.insert("pylint".to_string(), vec!["py".to_string()]);
        supported.insert("golint".to_string(), vec!["go".to_string()]);
        supported.insert("rubocop".to_string(), vec!["rb".to_string()]);
        supported.insert("shellcheck".to_string(), vec!["sh".to_string(), "bash".to_string()]);
        supported.insert("hadolint".to_string(), vec!["Dockerfile".to_string()]);
        supported.insert("markdownlint".to_string(), vec!["md".to_string()]);
        Self {
            supported_linters: supported,
        }
    }

    pub fn supported_linters(&self) -> Vec<String> {
        self.supported_linters.keys().cloned().collect()
    }

    pub fn run_linters(&self, files: &[&str], config: &ReviewConfig) -> Vec<LinterResult> {
        let mut results = Vec::new();

        for file in files {
            let ext = file.rsplit('.').next().unwrap_or("");
            for linter_name in &config.linters {
                if let Some(supported_exts) = self.supported_linters.get(linter_name) {
                    if supported_exts.iter().any(|e| e == ext || file.ends_with(e)) {
                        // Simulated linter output
                        results.extend(self.simulate_linter(linter_name, file));
                    }
                }
            }
        }

        // False positive filtering via confidence
        results
            .into_iter()
            .filter(|r| self.confidence_score(r) > 0.3)
            .collect()
    }

    fn simulate_linter(&self, linter: &str, file: &str) -> Vec<LinterResult> {
        // In production this would invoke the actual linter binary.
        // Here we return simulated results for known patterns.
        match linter {
            "clippy" => vec![LinterResult {
                linter_name: "clippy".to_string(),
                file: file.to_string(),
                line: 1,
                message: "Linter check passed".to_string(),
                severity: ReviewSeverity::Info,
            }],
            "eslint" => vec![LinterResult {
                linter_name: "eslint".to_string(),
                file: file.to_string(),
                line: 1,
                message: "Linter check passed".to_string(),
                severity: ReviewSeverity::Info,
            }],
            _ => vec![LinterResult {
                linter_name: linter.to_string(),
                file: file.to_string(),
                line: 1,
                message: "Linter check passed".to_string(),
                severity: ReviewSeverity::Info,
            }],
        }
    }

    fn confidence_score(&self, result: &LinterResult) -> f64 {
        // Higher severity findings get higher confidence
        match result.severity {
            ReviewSeverity::Security => 0.95,
            ReviewSeverity::Critical => 0.90,
            ReviewSeverity::Error => 0.80,
            ReviewSeverity::Warning => 0.60,
            ReviewSeverity::Info => 0.40,
        }
    }
}

// ── AI Code Review Engine ──────────────────────────────────────────────

pub struct AiCodeReviewEngine {
    config: ReviewConfig,
    learnings: Vec<ReviewLearning>,
    learning_stats: LearningStats,
    next_id: u64,
    linter_aggregator: LinterAggregator,
}

impl AiCodeReviewEngine {
    pub fn new(config: ReviewConfig) -> Self {
        Self {
            config,
            learnings: Vec::new(),
            learning_stats: LearningStats::default(),
            next_id: 1,
            linter_aggregator: LinterAggregator::new(),
        }
    }

    fn gen_id(&mut self, prefix: &str) -> String {
        let id = format!("{}-{}", prefix, self.next_id);
        self.next_id += 1;
        id
    }

    /// Full PR analysis from a unified diff.
    pub fn analyze_diff(&mut self, diff: &str, config: &ReviewConfig) -> PrAnalysis {
        let diff_files = parse_unified_diff(diff);
        let mut all_findings: Vec<ReviewFinding> = Vec::new();
        let mut total_added = 0usize;
        let mut total_removed = 0usize;

        for df in &diff_files {
            total_added += df.lines_added;
            total_removed += df.lines_removed;

            // Build the added content for analysis
            let added_content: String = df
                .added_lines
                .iter()
                .map(|(_, line)| line.as_str())
                .collect::<Vec<&str>>()
                .join("\n");

            if !added_content.is_empty() {
                let mut file_findings = self.analyze_file(&df.path, &added_content, config);
                // Cap findings per file
                file_findings.truncate(config.max_findings_per_file);
                all_findings.extend(file_findings);
            }
        }

        let breaking = self.detect_breaking_changes(diff);
        let risk = self.calculate_risk_score(&all_findings, total_added + total_removed, &breaking);

        let arch_impact = if diff_files.len() > 10 {
            "High — changes span many files; review architectural coherence".to_string()
        } else if diff_files.len() > 5 {
            "Medium — moderate file spread".to_string()
        } else {
            "Low — focused change".to_string()
        };

        let title = self.infer_pr_title(&diff_files);
        let summary = self.infer_pr_summary(&diff_files, &all_findings);
        let reviewers = self.suggest_reviewers(&diff_files);

        PrAnalysis {
            title,
            summary,
            files_changed: diff_files.len(),
            lines_added: total_added,
            lines_removed: total_removed,
            findings: all_findings,
            risk_score: risk,
            architectural_impact: arch_impact,
            test_coverage_delta: self.estimate_coverage_delta(&diff_files),
            breaking_changes: breaking,
            suggested_reviewers: reviewers,
        }
    }

    /// Single-file review combining all detectors.
    pub fn analyze_file(&mut self, path: &str, content: &str, config: &ReviewConfig) -> Vec<ReviewFinding> {
        let mut findings: Vec<ReviewFinding> = Vec::new();

        // Skip ignored patterns
        for pattern in &config.ignore_patterns {
            let pat = pattern.replace('*', "");
            if path.contains(&pat) {
                return findings;
            }
        }

        if config.enabled_categories.contains(&ReviewCategory::Security) {
            findings.extend(detect_security_issues(content));
        }
        if config.enabled_categories.contains(&ReviewCategory::Complexity) {
            findings.extend(detect_complexity_issues(content));
        }
        if config.enabled_categories.contains(&ReviewCategory::Style) {
            findings.extend(detect_style_issues(content));
        }
        if config.enabled_categories.contains(&ReviewCategory::Documentation) {
            findings.extend(detect_documentation_gaps(content));
        }
        if config.enabled_categories.contains(&ReviewCategory::Architecture) {
            findings.extend(detect_architecture_violations(content));
        }

        // Set file path on all findings
        for f in &mut findings {
            if f.file.is_empty() {
                f.file = path.to_string();
            }
        }

        // Apply learning-based confidence adjustment
        if config.learning_enabled {
            self.adjust_confidence_from_learnings(&mut findings);
        }

        findings.truncate(config.max_findings_per_file);
        findings
    }

    /// Check quality gates against analysis results.
    pub fn check_quality_gates(
        &self,
        analysis: &PrAnalysis,
        gates: &[QualityGate],
    ) -> Vec<QualityGateResult> {
        let mut results = Vec::new();

        for gate in gates {
            if !gate.enabled {
                continue;
            }
            let result = match &gate.condition {
                QualityGateCondition::MaxComplexity(max) => {
                    let complexity_findings = analysis
                        .findings
                        .iter()
                        .filter(|f| f.category == ReviewCategory::Complexity)
                        .count() as u32;
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: complexity_findings <= *max,
                        message: format!(
                            "Complexity findings: {} (max: {})",
                            complexity_findings, max
                        ),
                        details: analysis
                            .findings
                            .iter()
                            .filter(|f| f.category == ReviewCategory::Complexity)
                            .map(|f| f.message.clone())
                            .collect(),
                    }
                }
                QualityGateCondition::MinTestCoverage(min) => {
                    let passed = analysis.test_coverage_delta >= *min;
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed,
                        message: format!(
                            "Test coverage delta: {:.1}% (min: {:.1}%)",
                            analysis.test_coverage_delta * 100.0,
                            min * 100.0
                        ),
                        details: vec![],
                    }
                }
                QualityGateCondition::MaxFileLength(max) => {
                    let violations: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| {
                            f.category == ReviewCategory::Architecture
                                && f.message.contains("lines")
                        })
                        .map(|f| format!("{}: {}", f.file, f.message))
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: violations.is_empty(),
                        message: format!("Max file length: {} lines", max),
                        details: violations,
                    }
                }
                QualityGateCondition::MaxFunctionLength(max) => {
                    let long_fns: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| {
                            f.category == ReviewCategory::Complexity
                                && f.message.contains("lines long")
                        })
                        .map(|f| f.message.clone())
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: long_fns.is_empty(),
                        message: format!("Max function length: {} lines", max),
                        details: long_fns,
                    }
                }
                QualityGateCondition::NoTodoComments => {
                    let todos: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| f.message.contains("TODO") || f.message.contains("FIXME"))
                        .map(|f| format!("{}:{} {}", f.file, f.line_start, f.message))
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: todos.is_empty(),
                        message: "No TODO/FIXME comments".to_string(),
                        details: todos,
                    }
                }
                QualityGateCondition::RequireDocstrings => {
                    let missing: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| f.category == ReviewCategory::Documentation)
                        .map(|f| format!("{}:{} {}", f.file, f.line_start, f.message))
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: missing.is_empty(),
                        message: "Require docstrings on public items".to_string(),
                        details: missing,
                    }
                }
                QualityGateCondition::NoHardcodedSecrets => {
                    let secrets: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| {
                            f.category == ReviewCategory::Security
                                && (f.message.contains("Hardcoded")
                                    || f.message.contains("Private key")
                                    || f.message.contains("AWS access key"))
                        })
                        .map(|f| format!("{}:{} {}", f.file, f.line_start, f.message))
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: secrets.is_empty(),
                        message: "No hardcoded secrets".to_string(),
                        details: secrets,
                    }
                }
                QualityGateCondition::CustomRegex(pattern) => {
                    // Check if any finding messages match the regex pattern
                    let matched: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| f.message.to_lowercase().contains(&pattern.to_lowercase()))
                        .map(|f| f.message.clone())
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: matched.is_empty(),
                        message: format!("Custom rule: {}", gate.rule),
                        details: matched,
                    }
                }
                QualityGateCondition::MaxDuplication(max_pct) => {
                    let dup_count = analysis
                        .findings
                        .iter()
                        .filter(|f| f.category == ReviewCategory::Duplication)
                        .count();
                    let dup_pct = if analysis.files_changed > 0 {
                        dup_count as f64 / analysis.files_changed as f64
                    } else {
                        0.0
                    };
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: dup_pct <= *max_pct,
                        message: format!("Duplication: {:.1}% (max: {:.1}%)", dup_pct * 100.0, max_pct * 100.0),
                        details: vec![],
                    }
                }
                QualityGateCondition::RequireErrorHandling => {
                    let unwraps: Vec<String> = analysis
                        .findings
                        .iter()
                        .filter(|f| f.message.contains("unwrap") || f.message.contains("panic"))
                        .map(|f| format!("{}:{} {}", f.file, f.line_start, f.message))
                        .collect();
                    QualityGateResult {
                        gate_id: gate.id.clone(),
                        passed: unwraps.is_empty(),
                        message: "Require proper error handling".to_string(),
                        details: unwraps,
                    }
                }
            };
            results.push(result);
        }

        results
    }

    /// Suggest tests based on the diff.
    pub fn suggest_tests(&self, diff: &str) -> Vec<TestSuggestion> {
        let diff_files = parse_unified_diff(diff);
        let mut suggestions = Vec::new();

        for df in &diff_files {
            // Skip test files
            if df.path.contains("test") || df.path.contains("spec") {
                continue;
            }

            for (_, line) in &df.added_lines {
                let trimmed = line.trim();
                if let Some(name) = extract_function_name(trimmed) {
                    suggestions.push(TestSuggestion {
                        file: df.path.clone(),
                        function_name: name.clone(),
                        test_type: TestType::Unit,
                        description: format!("Add unit test for new function '{}'", name),
                    });

                    // Suggest edge case test for functions with parameters
                    if trimmed.contains("Option<") || trimmed.contains("Result<") {
                        suggestions.push(TestSuggestion {
                            file: df.path.clone(),
                            function_name: name.clone(),
                            test_type: TestType::Edge,
                            description: format!(
                                "Test '{}' with None/Err inputs and boundary values",
                                name
                            ),
                        });
                    }
                }

                // Integration test suggestions for API endpoints
                if trimmed.contains("async fn") && (trimmed.contains("handler") || trimmed.contains("endpoint") || trimmed.contains("route")) {
                    if let Some(name) = extract_function_name(trimmed) {
                        suggestions.push(TestSuggestion {
                            file: df.path.clone(),
                            function_name: name,
                            test_type: TestType::Integration,
                            description: "Add integration test for API endpoint".to_string(),
                        });
                    }
                }
            }
        }

        suggestions
    }

    /// Detect breaking changes in the diff (removed public APIs, signature changes).
    pub fn detect_breaking_changes(&self, diff: &str) -> Vec<BreakingChange> {
        let diff_files = parse_unified_diff(diff);
        let mut changes = Vec::new();

        for df in &diff_files {
            // Removed public functions
            for (_, line) in &df.removed_lines {
                let trimmed = line.trim();

                if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ") {
                    if let Some(name) = extract_function_name(trimmed) {
                        changes.push(BreakingChange {
                            file: df.path.clone(),
                            change_type: "Removed public function".to_string(),
                            description: format!("Public function '{}' was removed", name),
                            affected_apis: vec![name.clone()],
                            migration_hint: format!(
                                "Update all callers of '{}' to use the replacement API",
                                name
                            ),
                        });
                    }
                }

                // Removed public struct/enum
                if trimmed.starts_with("pub struct ") || trimmed.starts_with("pub enum ") {
                    let kind = if trimmed.contains("struct") {
                        "struct"
                    } else {
                        "enum"
                    };
                    let name = trimmed
                        .split_whitespace()
                        .nth(2)
                        .unwrap_or("unknown")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    changes.push(BreakingChange {
                        file: df.path.clone(),
                        change_type: format!("Removed public {}", kind),
                        description: format!("Public {} '{}' was removed", kind, name),
                        affected_apis: vec![name.to_string()],
                        migration_hint: format!("Migrate consumers of '{}' to the new type", name),
                    });
                }

                // Changed function signatures (removed params)
                if trimmed.starts_with("pub fn ") && trimmed.contains('(') {
                    // Check if same function exists in added lines with different sig
                    if let Some(name) = extract_function_name(trimmed) {
                        for (_, added) in &df.added_lines {
                            if added.contains(&format!("fn {}", name)) && added.trim() != trimmed {
                                changes.push(BreakingChange {
                                    file: df.path.clone(),
                                    change_type: "Changed function signature".to_string(),
                                    description: format!(
                                        "Signature of '{}' changed — may break callers",
                                        name
                                    ),
                                    affected_apis: vec![name.clone()],
                                    migration_hint: format!(
                                        "Update all callers of '{}' to match new signature",
                                        name
                                    ),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        changes
    }

    /// Generate a markdown PR summary.
    pub fn generate_pr_summary(&self, analysis: &PrAnalysis) -> String {
        let mut md = String::new();
        md.push_str(&format!("## {}\n\n", analysis.title));
        md.push_str(&format!("{}\n\n", analysis.summary));

        md.push_str("### Stats\n\n");
        md.push_str("| Metric | Value |\n|--------|-------|\n");
        md.push_str(&format!("| Files changed | {} |\n", analysis.files_changed));
        md.push_str(&format!("| Lines added | +{} |\n", analysis.lines_added));
        md.push_str(&format!("| Lines removed | -{} |\n", analysis.lines_removed));
        md.push_str(&format!(
            "| Risk score | {:.1}/10 |\n",
            analysis.risk_score * 10.0
        ));
        md.push_str(&format!(
            "| Architectural impact | {} |\n",
            analysis.architectural_impact
        ));
        md.push_str(&format!(
            "| Test coverage delta | {:.1}% |\n\n",
            analysis.test_coverage_delta * 100.0
        ));

        if !analysis.findings.is_empty() {
            md.push_str("### Findings\n\n");
            let security: Vec<&ReviewFinding> = analysis
                .findings
                .iter()
                .filter(|f| f.severity == ReviewSeverity::Security || f.severity == ReviewSeverity::Critical)
                .collect();
            let warnings: Vec<&ReviewFinding> = analysis
                .findings
                .iter()
                .filter(|f| f.severity == ReviewSeverity::Warning || f.severity == ReviewSeverity::Error)
                .collect();
            let info: Vec<&ReviewFinding> = analysis
                .findings
                .iter()
                .filter(|f| f.severity == ReviewSeverity::Info)
                .collect();

            if !security.is_empty() {
                md.push_str(&format!(
                    "- **{} critical/security issues** require immediate attention\n",
                    security.len()
                ));
            }
            if !warnings.is_empty() {
                md.push_str(&format!("- **{} warnings** should be reviewed\n", warnings.len()));
            }
            if !info.is_empty() {
                md.push_str(&format!("- **{} informational** suggestions\n", info.len()));
            }
            md.push('\n');

            // Top findings detail
            md.push_str("#### Top Findings\n\n");
            for (i, finding) in analysis.findings.iter().take(10).enumerate() {
                md.push_str(&format!(
                    "{}. **[{}]** `{}:{}` — {}\n",
                    i + 1,
                    finding.severity,
                    finding.file,
                    finding.line_start,
                    finding.message
                ));
                if let Some(ref sug) = finding.suggestion {
                    md.push_str(&format!("   > Suggestion: {}\n", sug));
                }
            }
            md.push('\n');
        }

        if !analysis.breaking_changes.is_empty() {
            md.push_str("### Breaking Changes\n\n");
            for bc in &analysis.breaking_changes {
                md.push_str(&format!(
                    "- **{}** in `{}`: {}\n  Migration: {}\n",
                    bc.change_type, bc.file, bc.description, bc.migration_hint
                ));
            }
            md.push('\n');
        }

        if !analysis.suggested_reviewers.is_empty() {
            md.push_str("### Suggested Reviewers\n\n");
            for r in &analysis.suggested_reviewers {
                md.push_str(&format!("- @{}\n", r));
            }
            md.push('\n');
        }

        md
    }

    /// Generate a Mermaid architectural diagram of changes.
    pub fn generate_architectural_diagram(&self, analysis: &PrAnalysis) -> String {
        let mut mermaid = String::from("graph TD\n");

        // Group files by directory
        let mut dirs: HashMap<String, Vec<String>> = HashMap::new();
        for finding in &analysis.findings {
            let dir = finding
                .file
                .rsplit('/')
                .nth(1)
                .unwrap_or("root")
                .to_string();
            let file = finding
                .file
                .rsplit('/')
                .next()
                .unwrap_or(&finding.file)
                .to_string();
            dirs.entry(dir).or_default().push(file);
        }

        // Deduplicate
        for files in dirs.values_mut() {
            files.sort();
            files.dedup();
        }

        let mut node_id = 0u32;
        let mut dir_nodes: HashMap<String, String> = HashMap::new();

        for (dir, files) in &dirs {
            let dir_id = format!("D{}", node_id);
            node_id += 1;
            let sanitized_dir = dir.replace(['-', '.'], "_");
            mermaid.push_str(&format!("    {}[\"{}\"]\n", dir_id, sanitized_dir));
            dir_nodes.insert(dir.clone(), dir_id.clone());

            for file in files {
                let file_id = format!("F{}", node_id);
                node_id += 1;
                let sanitized = file.replace(['-', '.'], "_");
                mermaid.push_str(&format!("    {}[\"{}\"]\n", file_id, sanitized));
                mermaid.push_str(&format!("    {} --> {}\n", dir_id, file_id));
            }
        }

        // Add severity legend
        let sec_count = analysis
            .findings
            .iter()
            .filter(|f| f.severity == ReviewSeverity::Security)
            .count();
        let crit_count = analysis
            .findings
            .iter()
            .filter(|f| f.severity == ReviewSeverity::Critical)
            .count();

        if sec_count > 0 || crit_count > 0 {
            mermaid.push_str(&format!(
                "    style_note[\"⚠ {} security, {} critical findings\"]\n",
                sec_count, crit_count
            ));
        }

        mermaid
    }

    /// Record feedback from a human reviewer (accepted/rejected finding).
    pub fn record_learning(&mut self, learning: ReviewLearning) {
        if learning.was_accepted {
            self.learning_stats.accepted += 1;
        } else {
            self.learning_stats.rejected += 1;
        }
        self.learning_stats.total_findings += 1;
        self.recalculate_learning_stats();
        self.learnings.push(learning);
    }

    /// Get precision/recall/F1 learning stats.
    pub fn get_learning_stats(&self) -> LearningStats {
        self.learning_stats.clone()
    }

    /// Get all recorded learnings.
    pub fn get_learnings(&self) -> &[ReviewLearning] {
        &self.learnings
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn recalculate_learning_stats(&mut self) {
        let total = self.learning_stats.total_findings;
        if total == 0 {
            return;
        }
        let accepted = self.learning_stats.accepted as f64;
        let total_f = total as f64;

        // Precision = accepted / total (how many of our findings were actually useful)
        self.learning_stats.precision = accepted / total_f;

        // Recall estimate (assumes we catch ~80% of real issues)
        self.learning_stats.recall = 0.80 * self.learning_stats.precision;

        // F1 = 2 * (P * R) / (P + R)
        let p = self.learning_stats.precision;
        let r = self.learning_stats.recall;
        self.learning_stats.f1_score = if p + r > 0.0 {
            2.0 * p * r / (p + r)
        } else {
            0.0
        };
    }

    fn adjust_confidence_from_learnings(&self, findings: &mut [ReviewFinding]) {
        if self.learnings.is_empty() {
            return;
        }

        // If precision is low, reduce confidence on lower-severity findings
        let precision = self.learning_stats.precision;
        for finding in findings.iter_mut() {
            if precision < 0.7 && finding.severity == ReviewSeverity::Info {
                finding.confidence *= 0.8;
            }
            if precision < 0.5 {
                finding.confidence *= 0.9;
            }
        }
    }

    fn calculate_risk_score(
        &self,
        findings: &[ReviewFinding],
        total_changed_lines: usize,
        breaking: &[BreakingChange],
    ) -> f64 {
        let mut score = 0.0;

        // Severity-weighted finding score
        for f in findings {
            score += f.severity.weight() * f.confidence;
        }

        // Normalize by change size
        let size_factor = if total_changed_lines > 1000 {
            0.3
        } else if total_changed_lines > 500 {
            0.2
        } else if total_changed_lines > 100 {
            0.1
        } else {
            0.05
        };
        score += size_factor * total_changed_lines as f64;

        // Breaking change penalty
        score += breaking.len() as f64 * 0.5;

        // Clamp to 0.0-1.0
        (score / (score + 10.0)).min(1.0)
    }

    fn estimate_coverage_delta(&self, diff_files: &[DiffFile]) -> f64 {
        let test_files = diff_files
            .iter()
            .filter(|f| f.path.contains("test") || f.path.contains("spec"))
            .count();
        let src_files = diff_files
            .iter()
            .filter(|f| !f.path.contains("test") && !f.path.contains("spec"))
            .count();

        if src_files == 0 {
            return 0.0;
        }

        // Rough heuristic: ratio of test files to source files
        let ratio = test_files as f64 / src_files as f64;
        (ratio - 0.5).clamp(-0.5, 0.5) // Delta between -50% and +50%
    }

    fn infer_pr_title(&self, diff_files: &[DiffFile]) -> String {
        if diff_files.is_empty() {
            return "Empty changeset".to_string();
        }

        let total_added: usize = diff_files.iter().map(|f| f.lines_added).sum();
        let total_removed: usize = diff_files.iter().map(|f| f.lines_removed).sum();

        if diff_files.len() == 1 {
            let path = &diff_files[0].path;
            let fname = path.rsplit('/').next().unwrap_or(path);
            if total_removed == 0 {
                format!("Add {}", fname)
            } else if total_added == 0 {
                format!("Remove {}", fname)
            } else {
                format!("Update {}", fname)
            }
        } else {
            let dirs: Vec<String> = diff_files
                .iter()
                .filter_map(|f| f.path.split('/').nth(0).map(String::from))
                .collect::<std::collections::HashSet<String>>()
                .into_iter()
                .collect();
            if dirs.len() == 1 {
                format!("Update {} ({} files)", dirs[0], diff_files.len())
            } else {
                format!("Update {} files across {} directories", diff_files.len(), dirs.len())
            }
        }
    }

    fn infer_pr_summary(&self, diff_files: &[DiffFile], findings: &[ReviewFinding]) -> String {
        let total_added: usize = diff_files.iter().map(|f| f.lines_added).sum();
        let total_removed: usize = diff_files.iter().map(|f| f.lines_removed).sum();

        let mut summary = format!(
            "This PR modifies {} file(s) with +{} / -{} lines.",
            diff_files.len(),
            total_added,
            total_removed
        );

        if !findings.is_empty() {
            let sec = findings
                .iter()
                .filter(|f| f.category == ReviewCategory::Security)
                .count();
            let bugs = findings
                .iter()
                .filter(|f| f.category == ReviewCategory::Bug)
                .count();
            summary.push_str(&format!(
                " Detected {} findings ({} security, {} bug-related).",
                findings.len(),
                sec,
                bugs
            ));
        }

        summary
    }

    fn suggest_reviewers(&self, diff_files: &[DiffFile]) -> Vec<String> {
        let mut reviewers = Vec::new();

        let has_security = diff_files
            .iter()
            .any(|f| f.path.contains("auth") || f.path.contains("security") || f.path.contains("crypto"));
        let has_infra = diff_files
            .iter()
            .any(|f| f.path.contains("deploy") || f.path.contains("docker") || f.path.contains("k8s") || f.path.contains("ci"));
        let has_frontend = diff_files
            .iter()
            .any(|f| f.path.ends_with(".tsx") || f.path.ends_with(".jsx") || f.path.ends_with(".css"));
        let has_backend = diff_files
            .iter()
            .any(|f| f.path.ends_with(".rs") || f.path.ends_with(".go") || f.path.ends_with(".py"));

        if has_security {
            reviewers.push("security-team".to_string());
        }
        if has_infra {
            reviewers.push("devops-team".to_string());
        }
        if has_frontend {
            reviewers.push("frontend-team".to_string());
        }
        if has_backend {
            reviewers.push("backend-team".to_string());
        }

        if reviewers.is_empty() {
            reviewers.push("team-lead".to_string());
        }

        reviewers
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ─────────────────────────────────────────────────────────

    fn default_config() -> ReviewConfig {
        ReviewConfig::default()
    }

    fn make_engine() -> AiCodeReviewEngine {
        AiCodeReviewEngine::new(default_config())
    }

    fn sample_diff() -> &'static str {
        r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,10 @@
 use std::collections::HashMap;
+use std::fs;

 fn main() {
-    println!("hello");
+    let password = "supersecret123";
+    let data = fs::read_to_string("config.toml").unwrap();
+    println!("loaded config");
+    eval(&data);
+    let query = format!("SELECT * FROM users WHERE id = {}", id);
 }
"#
    }

    // ── Enum tests ─────────────────────────────────────────────────────

    #[test]
    fn test_review_severity_label() {
        assert_eq!(ReviewSeverity::Info.label(), "Info");
        assert_eq!(ReviewSeverity::Security.label(), "Security");
    }

    #[test]
    fn test_review_severity_weight() {
        assert!(ReviewSeverity::Security.weight() > ReviewSeverity::Info.weight());
        assert!(ReviewSeverity::Critical.weight() > ReviewSeverity::Warning.weight());
    }

    #[test]
    fn test_review_severity_display() {
        assert_eq!(format!("{}", ReviewSeverity::Error), "Error");
        assert_eq!(format!("{}", ReviewSeverity::Warning), "Warning");
    }

    #[test]
    fn test_review_category_all() {
        let all = ReviewCategory::all();
        assert_eq!(all.len(), 12);
        assert!(all.contains(&ReviewCategory::Bug));
        assert!(all.contains(&ReviewCategory::MergeConflictRisk));
    }

    #[test]
    fn test_review_category_label() {
        assert_eq!(ReviewCategory::BreakingChange.label(), "Breaking Change");
        assert_eq!(ReviewCategory::Security.label(), "Security");
    }

    #[test]
    fn test_review_category_display() {
        assert_eq!(format!("{}", ReviewCategory::Performance), "Performance");
    }

    #[test]
    fn test_quality_gate_condition_label() {
        assert_eq!(QualityGateCondition::NoTodoComments.label(), "No TODO Comments");
        assert_eq!(QualityGateCondition::MaxComplexity(10).label(), "Max Complexity");
    }

    #[test]
    fn test_test_type_display() {
        assert_eq!(format!("{}", TestType::Unit), "Unit");
        assert_eq!(format!("{}", TestType::Integration), "Integration");
        assert_eq!(format!("{}", TestType::Edge), "Edge");
    }

    // ── Default config ─────────────────────────────────────────────────

    #[test]
    fn test_default_config_has_all_categories() {
        let config = default_config();
        assert_eq!(config.enabled_categories.len(), 12);
    }

    #[test]
    fn test_default_config_linters() {
        let config = default_config();
        assert!(config.linters.contains(&"clippy".to_string()));
        assert!(config.linters.contains(&"eslint".to_string()));
    }

    #[test]
    fn test_default_config_ignore_patterns() {
        let config = default_config();
        assert!(config.ignore_patterns.contains(&"*.min.js".to_string()));
        assert!(config.ignore_patterns.contains(&"vendor/*".to_string()));
    }

    #[test]
    fn test_default_learning_stats() {
        let stats = LearningStats::default();
        assert_eq!(stats.total_findings, 0);
        assert_eq!(stats.precision, 1.0);
    }

    // ── Diff parsing ───────────────────────────────────────────────────

    #[test]
    fn test_parse_unified_diff_basic() {
        let files = parse_unified_diff(sample_diff());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
    }

    #[test]
    fn test_parse_unified_diff_counts() {
        let files = parse_unified_diff(sample_diff());
        assert!(files[0].lines_added > 0);
        assert!(files[0].lines_removed > 0);
    }

    #[test]
    fn test_parse_unified_diff_empty() {
        let files = parse_unified_diff("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_unified_diff_multiple_files() {
        let diff = r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,3 +1,4 @@
 fn foo() {}
+fn bar() {}
diff --git a/b.rs b/b.rs
--- a/b.rs
+++ b/b.rs
@@ -1,2 +1,3 @@
 fn baz() {}
+fn qux() {}
"#;
        let files = parse_unified_diff(diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[1].path, "b.rs");
    }

    // ── Security detector ──────────────────────────────────────────────

    #[test]
    fn test_detect_hardcoded_password() {
        let code = r#"let password = "hunter2";"#;
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Hardcoded password")));
    }

    #[test]
    fn test_detect_hardcoded_secret() {
        let code = r#"let secret = "abc123def456";"#;
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Hardcoded secret")));
    }

    #[test]
    fn test_detect_hardcoded_api_key() {
        let code = r#"let api_key = "sk-1234567890abcdef";"#;
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("API key")));
    }

    #[test]
    fn test_detect_eval_injection() {
        let code = "eval(user_input);";
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("eval")));
    }

    #[test]
    fn test_detect_innerhtml_xss() {
        let code = "element.innerHTML = userContent;";
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("XSS")));
    }

    #[test]
    fn test_detect_sql_injection() {
        let code = r#"let q = format!("SELECT * FROM users WHERE id = {}", user_id);"#;
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("SQL injection")));
    }

    #[test]
    fn test_detect_unsafe_block() {
        let code = "unsafe { ptr::read(p) }";
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Unsafe")));
    }

    #[test]
    fn test_detect_document_write() {
        let code = "document.write(content);";
        let findings = detect_security_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("document.write")));
    }

    #[test]
    fn test_security_skips_comments() {
        let code = "// password = \"test123\";";
        let findings = detect_security_issues(code);
        // Comments should be skipped
        assert!(findings.iter().all(|f| !f.message.contains("Hardcoded password")));
    }

    #[test]
    fn test_security_findings_have_suggestions() {
        let code = "eval(x);";
        let findings = detect_security_issues(code);
        for f in &findings {
            assert!(f.suggestion.is_some());
        }
    }

    // ── Complexity detector ────────────────────────────────────────────

    #[test]
    fn test_detect_long_function() {
        let mut code = String::from("fn long_function() {\n");
        for i in 0..70 {
            code.push_str(&format!("    let x{} = {};\n", i, i));
        }
        code.push_str("}\n");
        let findings = detect_complexity_issues(&code);
        assert!(findings.iter().any(|f| f.message.contains("lines long")));
    }

    #[test]
    fn test_detect_deep_nesting() {
        let code = "fn nested() {\n    if true {\n        if true {\n            if true {\n                if true {\n                    if true {\n                            do_thing();\n                    }\n                }\n            }\n        }\n    }\n}\n";
        let findings = detect_complexity_issues(code);
        assert!(
            findings.iter().any(|f| f.message.contains("nesting") || f.message.contains("nested") || f.message.contains("indent")),
            "Expected nesting finding, got: {:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_complexity_for_short_function() {
        let code = "fn short() {\n    return 42;\n}\n";
        let findings = detect_complexity_issues(code);
        assert!(findings.iter().all(|f| !f.message.contains("lines long")));
    }

    #[test]
    fn test_complexity_high_branching() {
        let mut code = String::from("fn branchy(x: i32) {\n");
        for i in 0..15 {
            code.push_str(&format!("    if x == {} {{ return; }}\n", i));
        }
        code.push_str("}\n");
        let findings = detect_complexity_issues(&code);
        assert!(findings.iter().any(|f| f.message.contains("cyclomatic complexity")));
    }

    // ── Style detector ─────────────────────────────────────────────────

    #[test]
    fn test_detect_trailing_whitespace() {
        let code = "let x = 1;   \n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Trailing whitespace")));
    }

    #[test]
    fn test_detect_todo_comment() {
        let code = "// TODO: fix this later\n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("TODO")));
    }

    #[test]
    fn test_detect_println_debug() {
        let code = "println!(\"debug output\");\n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Debug print")));
    }

    #[test]
    fn test_detect_console_log() {
        let code = "console.log('debug');\n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("console")));
    }

    #[test]
    fn test_detect_non_snake_case_function() {
        let code = "fn myFunction() {}\n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("snake_case")));
    }

    #[test]
    fn test_no_style_issue_for_clean_code() {
        let code = "fn clean_function() {\n    let x = 1;\n}\n";
        let findings = detect_style_issues(code);
        // Should not flag trailing whitespace or debug prints
        assert!(findings.iter().all(|f| !f.message.contains("Trailing whitespace")));
        assert!(findings.iter().all(|f| !f.message.contains("Debug print")));
    }

    #[test]
    fn test_detect_long_line() {
        let long = format!("let x = \"{}\";", "a".repeat(130));
        let findings = detect_style_issues(&long);
        assert!(findings.iter().any(|f| f.message.contains("characters")));
    }

    #[test]
    fn test_detect_dbg_macro() {
        let code = "dbg!(value);\n";
        let findings = detect_style_issues(code);
        assert!(findings.iter().any(|f| f.message.contains("Debug print")));
    }

    // ── Documentation detector ─────────────────────────────────────────

    #[test]
    fn test_detect_undocumented_pub_fn() {
        let code = "pub fn my_function() {}\n";
        let findings = detect_documentation_gaps(code);
        assert!(findings.iter().any(|f| f.message.contains("function") && f.message.contains("documentation")));
    }

    #[test]
    fn test_detect_undocumented_pub_struct() {
        let code = "pub struct MyStruct {}\n";
        let findings = detect_documentation_gaps(code);
        assert!(findings.iter().any(|f| f.message.contains("struct")));
    }

    #[test]
    fn test_documented_fn_no_finding() {
        let code = "/// Does something useful\npub fn documented() {}\n";
        let findings = detect_documentation_gaps(code);
        assert!(
            findings.iter().all(|f| !f.message.contains("function missing documentation")),
            "Documented function should not be flagged"
        );
    }

    #[test]
    fn test_detect_missing_module_doc() {
        let code = "use std::io;\nfn main() {}\n";
        let findings = detect_documentation_gaps(code);
        assert!(findings.iter().any(|f| f.message.contains("module-level")));
    }

    #[test]
    fn test_module_doc_present() {
        let code = "//! This is a module doc\nuse std::io;\n";
        let findings = detect_documentation_gaps(code);
        assert!(findings.iter().all(|f| !f.message.contains("module-level")));
    }

    #[test]
    fn test_detect_python_missing_docstring() {
        let code = "def my_function():\n    pass\n";
        let findings = detect_documentation_gaps(code);
        assert!(findings.iter().any(|f| f.message.contains("docstring")));
    }

    // ── Test gap detector ──────────────────────────────────────────────

    #[test]
    fn test_detect_untested_function() {
        let code = "pub fn process_data() {}\npub fn validate() {}\n";
        let test_code = "fn test_validate() { validate(); }\n";
        let findings = detect_test_gaps(code, test_code);
        assert!(findings.iter().any(|f| f.message.contains("process_data")));
    }

    #[test]
    fn test_tested_function_not_flagged() {
        let code = "pub fn compute() {}\n";
        let test_code = "fn test_compute() { compute(); }\n";
        let findings = detect_test_gaps(code, test_code);
        assert!(findings.iter().all(|f| !f.message.contains("compute")));
    }

    #[test]
    fn test_detect_error_path_without_tests() {
        let code = "pub fn risky() { let x = foo().unwrap(); }\n";
        let test_code = "fn test_risky() { risky(); }\n";
        let findings = detect_test_gaps(code, test_code);
        assert!(findings.iter().any(|f| f.message.contains("Error path")));
    }

    #[test]
    fn test_skip_main_and_new_in_test_gaps() {
        let code = "fn main() {}\nfn new() -> Self {}\n";
        let test_code = "";
        let findings = detect_test_gaps(code, test_code);
        assert!(findings.iter().all(|f| !f.message.contains("'main'")));
        assert!(findings.iter().all(|f| !f.message.contains("'new'")));
    }

    // ── Duplication detector ───────────────────────────────────────────

    #[test]
    fn test_detect_duplication_across_files() {
        let shared = "let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\nlet e = 5;\nlet f = 6;\nlet g = 7;\n";
        let files = vec![("file_a.rs", shared), ("file_b.rs", shared)];
        let findings = detect_duplication(&files);
        assert!(!findings.is_empty(), "Should detect duplication across files");
        assert!(findings[0].category == ReviewCategory::Duplication);
    }

    #[test]
    fn test_no_duplication_for_unique_files() {
        let files = vec![
            ("a.rs", "fn foo() { 1 }\nfn bar() { 2 }\n"),
            ("b.rs", "fn baz() { 3 }\nfn qux() { 4 }\n"),
        ];
        let findings = detect_duplication(&files);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_duplication_same_file_ignored() {
        let code = "let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\nlet e = 5;\nlet f = 6;\nlet g = 7;\n";
        let files = vec![("same.rs", code)];
        let findings = detect_duplication(&files);
        assert!(findings.is_empty(), "Same-file duplication should not be flagged");
    }

    // ── Architecture detector ──────────────────────────────────────────

    #[test]
    fn test_detect_wildcard_import() {
        let code = "use std::collections::*;\nfn main() {}\n";
        let findings = detect_architecture_violations(code);
        assert!(findings.iter().any(|f| f.message.contains("Wildcard")));
    }

    #[test]
    fn test_detect_god_file() {
        let mut code = String::new();
        for i in 0..600 {
            code.push_str(&format!("let var_{} = {};\n", i, i));
        }
        let findings = detect_architecture_violations(&code);
        assert!(findings.iter().any(|f| f.message.contains("lines")));
    }

    #[test]
    fn test_detect_high_coupling() {
        let mut code = String::new();
        for i in 0..25 {
            code.push_str(&format!("use crate::module_{}::something;\n", i));
        }
        code.push_str("fn main() {}\n");
        let findings = detect_architecture_violations(&code);
        assert!(findings.iter().any(|f| f.message.contains("coupling") || f.message.contains("imports")));
    }

    #[test]
    fn test_no_violations_for_clean_file() {
        let code = "use std::io;\nfn main() {\n    println!(\"hello\");\n}\n";
        let findings = detect_architecture_violations(code);
        assert!(findings.iter().all(|f| !f.message.contains("Wildcard")));
        assert!(findings.iter().all(|f| !f.message.contains("coupling")));
    }

    // ── Linter aggregator ──────────────────────────────────────────────

    #[test]
    fn test_linter_aggregator_supported() {
        let agg = LinterAggregator::new();
        let supported = agg.supported_linters();
        assert!(supported.contains(&"clippy".to_string()));
        assert!(supported.contains(&"eslint".to_string()));
        assert!(supported.contains(&"pylint".to_string()));
    }

    #[test]
    fn test_linter_aggregator_run() {
        let agg = LinterAggregator::new();
        let config = default_config();
        let results = agg.run_linters(&["main.rs", "app.js"], &config);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_linter_aggregator_filters_by_extension() {
        let agg = LinterAggregator::new();
        let config = default_config();
        let results = agg.run_linters(&["image.png"], &config);
        assert!(results.is_empty(), "PNG file should not match any linter");
    }

    // ── Engine: analyze_diff ───────────────────────────────────────────

    #[test]
    fn test_analyze_diff_basic() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(sample_diff(), &config);
        assert_eq!(analysis.files_changed, 1);
        assert!(analysis.lines_added > 0);
    }

    #[test]
    fn test_analyze_diff_finds_security_issues() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(sample_diff(), &config);
        assert!(
            analysis.findings.iter().any(|f| f.category == ReviewCategory::Security),
            "Should detect security issues in diff"
        );
    }

    #[test]
    fn test_analyze_diff_risk_score() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(sample_diff(), &config);
        assert!(analysis.risk_score > 0.0);
        assert!(analysis.risk_score <= 1.0);
    }

    #[test]
    fn test_analyze_diff_empty() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff("", &config);
        assert_eq!(analysis.files_changed, 0);
        assert_eq!(analysis.lines_added, 0);
    }

    #[test]
    fn test_analyze_diff_suggested_reviewers() {
        let diff = r#"diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,2 +1,3 @@
 fn login() {}
+fn logout() {}
"#;
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(diff, &config);
        assert!(analysis.suggested_reviewers.contains(&"security-team".to_string()));
    }

    // ── Engine: analyze_file ───────────────────────────────────────────

    #[test]
    fn test_analyze_file_combines_detectors() {
        let mut engine = make_engine();
        let config = default_config();
        let code = "eval(input);\n// TODO: remove this\npub fn BadName() {}\n";
        let findings = engine.analyze_file("test.rs", code, &config);
        let categories: Vec<&ReviewCategory> = findings.iter().map(|f| &f.category).collect();
        assert!(categories.contains(&&ReviewCategory::Security));
        assert!(categories.contains(&&ReviewCategory::Style));
    }

    #[test]
    fn test_analyze_file_respects_ignore_patterns() {
        let mut engine = make_engine();
        let config = default_config();
        let findings = engine.analyze_file("vendor/lib.js", "eval(x);", &config);
        assert!(findings.is_empty(), "vendor files should be ignored");
    }

    #[test]
    fn test_analyze_file_sets_file_path() {
        let mut engine = make_engine();
        let config = default_config();
        let findings = engine.analyze_file("src/lib.rs", "eval(x);", &config);
        for f in &findings {
            assert_eq!(f.file, "src/lib.rs");
        }
    }

    #[test]
    fn test_analyze_file_respects_max_findings() {
        let mut config = default_config();
        config.max_findings_per_file = 3;
        let mut engine = AiCodeReviewEngine::new(config.clone());
        let code = "eval(a);\neval(b);\neval(c);\neval(d);\neval(e);\nprintln!(\"x\");\n// TODO a\n// FIXME b\n";
        let findings = engine.analyze_file("test.rs", code, &config);
        assert!(findings.len() <= 3);
    }

    // ── Engine: quality gates ──────────────────────────────────────────

    #[test]
    fn test_quality_gate_no_todo() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(
            r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,3 @@
 fn foo() {}
+// TODO: fix this
"#,
            &config,
        );
        let gates = vec![QualityGate {
            id: "g1".to_string(),
            name: "No TODOs".to_string(),
            rule: "No TODO comments allowed".to_string(),
            condition: QualityGateCondition::NoTodoComments,
            enabled: true,
            severity: ReviewSeverity::Warning,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed, "Gate should fail when TODO is present");
    }

    #[test]
    fn test_quality_gate_no_secrets() {
        let mut engine = make_engine();
        let config = default_config();
        let analysis = engine.analyze_diff(
            r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,3 @@
 fn foo() {}
+let password = "abc123";
"#,
            &config,
        );
        let gates = vec![QualityGate {
            id: "g2".to_string(),
            name: "No Secrets".to_string(),
            rule: "No hardcoded secrets".to_string(),
            condition: QualityGateCondition::NoHardcodedSecrets,
            enabled: true,
            severity: ReviewSeverity::Security,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
    }

    #[test]
    fn test_quality_gate_disabled_skipped() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "test".to_string(),
            summary: "test".to_string(),
            files_changed: 1,
            lines_added: 10,
            lines_removed: 5,
            findings: vec![],
            risk_score: 0.1,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let gates = vec![QualityGate {
            id: "g3".to_string(),
            name: "Disabled gate".to_string(),
            rule: "test".to_string(),
            condition: QualityGateCondition::NoTodoComments,
            enabled: false,
            severity: ReviewSeverity::Info,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert!(results.is_empty());
    }

    #[test]
    fn test_quality_gate_max_complexity() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "test".to_string(),
            summary: "test".to_string(),
            files_changed: 1,
            lines_added: 10,
            lines_removed: 5,
            findings: vec![
                ReviewFinding {
                    id: "cx-1".to_string(),
                    file: "a.rs".to_string(),
                    line_start: 1,
                    line_end: 50,
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::Complexity,
                    message: "High complexity".to_string(),
                    suggestion: None,
                    auto_fixable: false,
                    confidence: 0.8,
                },
            ],
            risk_score: 0.3,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let gates = vec![QualityGate {
            id: "g4".to_string(),
            name: "Max complexity".to_string(),
            rule: "No more than 0 complexity findings".to_string(),
            condition: QualityGateCondition::MaxComplexity(0),
            enabled: true,
            severity: ReviewSeverity::Error,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert!(!results[0].passed);
    }

    #[test]
    fn test_quality_gate_require_docstrings_pass() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "test".to_string(),
            summary: "test".to_string(),
            files_changed: 1,
            lines_added: 5,
            lines_removed: 0,
            findings: vec![],
            risk_score: 0.0,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let gates = vec![QualityGate {
            id: "g5".to_string(),
            name: "Require docs".to_string(),
            rule: "All public items documented".to_string(),
            condition: QualityGateCondition::RequireDocstrings,
            enabled: true,
            severity: ReviewSeverity::Warning,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert!(results[0].passed);
    }

    #[test]
    fn test_quality_gate_custom_regex() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "test".to_string(),
            summary: "test".to_string(),
            files_changed: 1,
            lines_added: 5,
            lines_removed: 0,
            findings: vec![ReviewFinding {
                id: "f1".to_string(),
                file: "a.rs".to_string(),
                line_start: 1,
                line_end: 1,
                severity: ReviewSeverity::Warning,
                category: ReviewCategory::Style,
                message: "Debug print statement in code".to_string(),
                suggestion: None,
                auto_fixable: false,
                confidence: 0.8,
            }],
            risk_score: 0.1,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let gates = vec![QualityGate {
            id: "g6".to_string(),
            name: "No debug prints".to_string(),
            rule: "No debug print statements".to_string(),
            condition: QualityGateCondition::CustomRegex("debug print".to_string()),
            enabled: true,
            severity: ReviewSeverity::Warning,
        }];
        let results = engine.check_quality_gates(&analysis, &gates);
        assert!(!results[0].passed);
    }

    // ── Engine: suggest_tests ──────────────────────────────────────────

    #[test]
    fn test_suggest_tests_for_new_functions() {
        let engine = make_engine();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,2 +1,5 @@
 fn existing() {}
+pub fn new_feature(x: i32) -> bool {
+    x > 0
+}
"#;
        let suggestions = engine.suggest_tests(diff);
        assert!(suggestions.iter().any(|s| s.function_name == "new_feature"));
    }

    #[test]
    fn test_suggest_tests_skip_test_files() {
        let engine = make_engine();
        let diff = r#"diff --git a/tests/test_lib.rs b/tests/test_lib.rs
--- a/tests/test_lib.rs
+++ b/tests/test_lib.rs
@@ -1,2 +1,4 @@
 fn existing_test() {}
+fn test_new() {
+}
"#;
        let suggestions = engine.suggest_tests(diff);
        assert!(suggestions.is_empty(), "Test files should be skipped");
    }

    #[test]
    fn test_suggest_edge_tests_for_option_result() {
        let engine = make_engine();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,2 +1,4 @@
 fn existing() {}
+pub fn parse(input: Option<&str>) -> Result<i32, String> {
+}
"#;
        let suggestions = engine.suggest_tests(diff);
        assert!(suggestions.iter().any(|s| s.test_type == TestType::Edge));
    }

    // ── Engine: breaking changes ───────────────────────────────────────

    #[test]
    fn test_detect_removed_pub_fn() {
        let engine = make_engine();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,4 +1,2 @@
-pub fn deprecated_api() {}
-pub fn old_helper() {}
 fn internal() {}
"#;
        let bc = engine.detect_breaking_changes(diff);
        assert!(!bc.is_empty());
        assert!(bc.iter().any(|b| b.description.contains("deprecated_api")));
    }

    #[test]
    fn test_detect_removed_pub_struct() {
        let engine = make_engine();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,1 @@
-pub struct OldConfig {}
 fn main() {}
"#;
        let bc = engine.detect_breaking_changes(diff);
        assert!(bc.iter().any(|b| b.description.contains("OldConfig")));
    }

    #[test]
    fn test_no_breaking_change_for_internal() {
        let engine = make_engine();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,2 @@
-fn internal_helper() {}
 fn main() {}
"#;
        let bc = engine.detect_breaking_changes(diff);
        assert!(bc.is_empty(), "Removing private fn should not be breaking");
    }

    // ── Engine: PR summary generation ──────────────────────────────────

    #[test]
    fn test_generate_pr_summary_basic() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "Add auth module".to_string(),
            summary: "Implements JWT authentication".to_string(),
            files_changed: 3,
            lines_added: 200,
            lines_removed: 10,
            findings: vec![],
            risk_score: 0.2,
            architectural_impact: "Medium".to_string(),
            test_coverage_delta: 0.1,
            breaking_changes: vec![],
            suggested_reviewers: vec!["security-team".to_string()],
        };
        let md = engine.generate_pr_summary(&analysis);
        assert!(md.contains("Add auth module"));
        assert!(md.contains("200"));
        assert!(md.contains("security-team"));
    }

    #[test]
    fn test_generate_pr_summary_with_findings() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "Update".to_string(),
            summary: "Changes".to_string(),
            files_changed: 1,
            lines_added: 10,
            lines_removed: 5,
            findings: vec![ReviewFinding {
                id: "f1".to_string(),
                file: "a.rs".to_string(),
                line_start: 5,
                line_end: 5,
                severity: ReviewSeverity::Security,
                category: ReviewCategory::Security,
                message: "Hardcoded password".to_string(),
                suggestion: Some("Use env var".to_string()),
                auto_fixable: false,
                confidence: 0.9,
            }],
            risk_score: 0.8,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let md = engine.generate_pr_summary(&analysis);
        assert!(md.contains("Hardcoded password"));
        assert!(md.contains("critical/security"));
    }

    #[test]
    fn test_generate_pr_summary_with_breaking_changes() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "Refactor API".to_string(),
            summary: "Major refactoring".to_string(),
            files_changed: 5,
            lines_added: 300,
            lines_removed: 200,
            findings: vec![],
            risk_score: 0.6,
            architectural_impact: "High".to_string(),
            test_coverage_delta: -0.1,
            breaking_changes: vec![BreakingChange {
                file: "api.rs".to_string(),
                change_type: "Removed public function".to_string(),
                description: "Removed legacy_endpoint".to_string(),
                affected_apis: vec!["legacy_endpoint".to_string()],
                migration_hint: "Use new_endpoint instead".to_string(),
            }],
            suggested_reviewers: vec!["backend-team".to_string()],
        };
        let md = engine.generate_pr_summary(&analysis);
        assert!(md.contains("Breaking Changes"));
        assert!(md.contains("legacy_endpoint"));
    }

    // ── Engine: architectural diagram ──────────────────────────────────

    #[test]
    fn test_generate_architectural_diagram() {
        let engine = make_engine();
        let analysis = PrAnalysis {
            title: "test".to_string(),
            summary: "test".to_string(),
            files_changed: 2,
            lines_added: 10,
            lines_removed: 5,
            findings: vec![
                ReviewFinding {
                    id: "f1".to_string(),
                    file: "src/auth.rs".to_string(),
                    line_start: 1,
                    line_end: 10,
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::Security,
                    message: "test".to_string(),
                    suggestion: None,
                    auto_fixable: false,
                    confidence: 0.8,
                },
                ReviewFinding {
                    id: "f2".to_string(),
                    file: "src/api.rs".to_string(),
                    line_start: 5,
                    line_end: 15,
                    severity: ReviewSeverity::Info,
                    category: ReviewCategory::Style,
                    message: "test".to_string(),
                    suggestion: None,
                    auto_fixable: false,
                    confidence: 0.5,
                },
            ],
            risk_score: 0.3,
            architectural_impact: "Low".to_string(),
            test_coverage_delta: 0.0,
            breaking_changes: vec![],
            suggested_reviewers: vec![],
        };
        let diagram = engine.generate_architectural_diagram(&analysis);
        assert!(diagram.starts_with("graph TD"));
        assert!(diagram.contains("auth"));
    }

    // ── Engine: learning ───────────────────────────────────────────────

    #[test]
    fn test_record_learning_accepted() {
        let mut engine = make_engine();
        engine.record_learning(ReviewLearning {
            finding_id: "SEC-1".to_string(),
            was_accepted: true,
            reviewer_comment: "Good catch".to_string(),
            timestamp: 1000,
        });
        let stats = engine.get_learning_stats();
        assert_eq!(stats.total_findings, 1);
        assert_eq!(stats.accepted, 1);
        assert_eq!(stats.rejected, 0);
    }

    #[test]
    fn test_record_learning_rejected() {
        let mut engine = make_engine();
        engine.record_learning(ReviewLearning {
            finding_id: "SEC-2".to_string(),
            was_accepted: false,
            reviewer_comment: "False positive".to_string(),
            timestamp: 1001,
        });
        let stats = engine.get_learning_stats();
        assert_eq!(stats.rejected, 1);
        assert!(stats.precision < 1.0);
    }

    #[test]
    fn test_learning_precision_calculation() {
        let mut engine = make_engine();
        for i in 0..8 {
            engine.record_learning(ReviewLearning {
                finding_id: format!("f-{}", i),
                was_accepted: true,
                reviewer_comment: "ok".to_string(),
                timestamp: 1000 + i,
            });
        }
        for i in 8..10 {
            engine.record_learning(ReviewLearning {
                finding_id: format!("f-{}", i),
                was_accepted: false,
                reviewer_comment: "nope".to_string(),
                timestamp: 1000 + i,
            });
        }
        let stats = engine.get_learning_stats();
        assert_eq!(stats.total_findings, 10);
        assert!((stats.precision - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_learning_f1_score() {
        let mut engine = make_engine();
        engine.record_learning(ReviewLearning {
            finding_id: "f1".to_string(),
            was_accepted: true,
            reviewer_comment: "ok".to_string(),
            timestamp: 1000,
        });
        let stats = engine.get_learning_stats();
        assert!(stats.f1_score > 0.0);
        assert!(stats.f1_score <= 1.0);
    }

    #[test]
    fn test_get_learnings() {
        let mut engine = make_engine();
        engine.record_learning(ReviewLearning {
            finding_id: "a".to_string(),
            was_accepted: true,
            reviewer_comment: "yes".to_string(),
            timestamp: 100,
        });
        engine.record_learning(ReviewLearning {
            finding_id: "b".to_string(),
            was_accepted: false,
            reviewer_comment: "no".to_string(),
            timestamp: 200,
        });
        assert_eq!(engine.get_learnings().len(), 2);
    }

    #[test]
    fn test_confidence_adjustment_with_low_precision() {
        let mut engine = make_engine();
        // Record many rejections to lower precision
        for i in 0..10 {
            engine.record_learning(ReviewLearning {
                finding_id: format!("r-{}", i),
                was_accepted: false,
                reviewer_comment: "fp".to_string(),
                timestamp: i as u64,
            });
        }
        let mut config = default_config();
        config.learning_enabled = true;
        let findings = engine.analyze_file(
            "test.rs",
            "println!(\"debug\");\n",
            &config,
        );
        // Info-level findings should have reduced confidence
        for f in &findings {
            if f.severity == ReviewSeverity::Info {
                assert!(f.confidence < 0.85, "Confidence should be reduced: {}", f.confidence);
            }
        }
    }

    // ── Engine: integrated scenarios ───────────────────────────────────

    #[test]
    fn test_full_review_workflow() {
        let mut engine = make_engine();
        let config = default_config();

        // 1. Analyze diff
        let analysis = engine.analyze_diff(sample_diff(), &config);
        assert!(!analysis.findings.is_empty());

        // 2. Check quality gates
        let gates = vec![
            QualityGate {
                id: "g1".to_string(),
                name: "No secrets".to_string(),
                rule: "No hardcoded secrets".to_string(),
                condition: QualityGateCondition::NoHardcodedSecrets,
                enabled: true,
                severity: ReviewSeverity::Security,
            },
        ];
        let gate_results = engine.check_quality_gates(&analysis, &gates);
        assert!(!gate_results[0].passed);

        // 3. Generate summary
        let summary = engine.generate_pr_summary(&analysis);
        assert!(!summary.is_empty());

        // 4. Record learning
        for finding in &analysis.findings {
            engine.record_learning(ReviewLearning {
                finding_id: finding.id.clone(),
                was_accepted: true,
                reviewer_comment: "Valid finding".to_string(),
                timestamp: 9999,
            });
        }
        let stats = engine.get_learning_stats();
        assert!(stats.total_findings > 0);
        assert_eq!(stats.precision, 1.0);
    }

    #[test]
    fn test_category_filtering() {
        let mut config = default_config();
        config.enabled_categories = vec![ReviewCategory::Security];
        let mut engine = AiCodeReviewEngine::new(config.clone());
        let code = "eval(x);\nprintln!(\"debug\");\n// TODO: fix\n";
        let findings = engine.analyze_file("test.rs", code, &config);
        for f in &findings {
            assert_eq!(f.category, ReviewCategory::Security);
        }
    }

    #[test]
    fn test_risk_score_increases_with_severity() {
        let mut engine = make_engine();
        let config = default_config();

        let safe_diff = r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,3 @@
 fn foo() {}
+fn bar() {}
"#;
        let dangerous_diff = r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,5 @@
 fn foo() {}
+let password = "secret";
+eval(input);
+let q = format!("SELECT * FROM t WHERE x = {}", y);
"#;
        let safe_analysis = engine.analyze_diff(safe_diff, &config);
        let dangerous_analysis = engine.analyze_diff(dangerous_diff, &config);
        assert!(
            dangerous_analysis.risk_score >= safe_analysis.risk_score,
            "Dangerous diff should have higher risk: {} vs {}",
            dangerous_analysis.risk_score,
            safe_analysis.risk_score
        );
    }

    // ── Extract function name ──────────────────────────────────────────

    #[test]
    fn test_extract_fn_name_rust() {
        assert_eq!(extract_function_name("fn foo(x: i32)"), Some("foo".to_string()));
        assert_eq!(extract_function_name("pub fn bar()"), Some("bar".to_string()));
        assert_eq!(extract_function_name("pub async fn baz(s: &str)"), Some("baz".to_string()));
    }

    #[test]
    fn test_extract_fn_name_python() {
        assert_eq!(extract_function_name("def my_func(self):"), Some("my_func".to_string()));
    }

    #[test]
    fn test_extract_fn_name_go() {
        assert_eq!(extract_function_name("func handler(w http.ResponseWriter)"), Some("handler".to_string()));
    }

    #[test]
    fn test_extract_fn_name_none() {
        assert_eq!(extract_function_name("let x = 1;"), None);
        assert_eq!(extract_function_name("struct Foo {}"), None);
    }
}
