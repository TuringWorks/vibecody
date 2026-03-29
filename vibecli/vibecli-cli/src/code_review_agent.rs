#![allow(dead_code)]
//! Autonomous code review agent — intent-aware, convention-following code analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Critical,
    Warning,
    Suggestion,
    Praise,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewCategory {
    Security,
    Performance,
    Correctness,
    Style,
    Complexity,
    Testing,
    Documentation,
    Naming,
    ErrorHandling,
    Concurrency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewScope {
    File,
    Diff,
    PR,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Complete,
    Dismissed,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub id: String,
    pub severity: ReviewSeverity,
    pub category: ReviewCategory,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub suggestion: Option<String>,
    pub confidence: f64,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    pub min_confidence: f64,
    pub categories: Vec<ReviewCategory>,
    pub max_findings: usize,
    pub include_praise: bool,
    pub auto_dismiss_below: f64,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            categories: vec![
                ReviewCategory::Security,
                ReviewCategory::Performance,
                ReviewCategory::Correctness,
                ReviewCategory::Style,
                ReviewCategory::Complexity,
                ReviewCategory::Testing,
                ReviewCategory::Documentation,
                ReviewCategory::Naming,
                ReviewCategory::ErrorHandling,
                ReviewCategory::Concurrency,
            ],
            max_findings: 50,
            include_praise: true,
            auto_dismiss_below: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSession {
    pub id: String,
    pub scope: ReviewScope,
    pub findings: Vec<ReviewFinding>,
    pub files_reviewed: Vec<String>,
    pub status: ReviewStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub total_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRule {
    pub id: String,
    pub pattern: String,
    pub category: ReviewCategory,
    pub severity: ReviewSeverity,
    pub message: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMetrics {
    pub total_reviews: u64,
    pub total_findings: u64,
    pub by_severity: HashMap<String, u64>,
    pub by_category: HashMap<String, u64>,
    pub avg_findings_per_review: f64,
    pub auto_fixed: u64,
}

impl Default for ReviewMetrics {
    fn default() -> Self {
        Self {
            total_reviews: 0,
            total_findings: 0,
            by_severity: HashMap::new(),
            by_category: HashMap::new(),
            avg_findings_per_review: 0.0,
            auto_fixed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConventionRule {
    pub name: String,
    pub pattern: String,
    pub description: String,
    pub examples_good: Vec<String>,
    pub examples_bad: Vec<String>,
}

// ---------------------------------------------------------------------------
// PatternAnalyzer
// ---------------------------------------------------------------------------

pub struct PatternAnalyzer;

impl PatternAnalyzer {
    /// Detect common issues in source code based on file extension.
    pub fn analyze(content: &str, file_ext: &str) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Determine if this is test code (Rust or TS/JS)
        let is_test_file = content.contains("#[cfg(test)]")
            || content.contains("#[test]")
            || file_ext == "test.ts"
            || file_ext == "test.js"
            || file_ext == "spec.ts"
            || file_ext == "spec.js";

        for (idx, line) in lines.iter().enumerate() {
            let lineno = idx + 1;
            let trimmed = line.trim();

            // --- unwrap() in non-test code ---
            if !is_test_file
                && (file_ext == "rs" || file_ext == "rust")
                && trimmed.contains(".unwrap()")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("///")
            {
                findings.push(ReviewFinding {
                    id: format!("PA-unwrap-{lineno}"),
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::ErrorHandling,
                    title: "Bare unwrap() in production code".into(),
                    description: format!(
                        "Line {lineno}: `unwrap()` can panic at runtime. \
                         Prefer `expect()`, `?`, or explicit match."
                    ),
                    file_path: String::new(),
                    line_start: lineno,
                    line_end: lineno,
                    suggestion: Some("Replace with `.expect(\"reason\")` or propagate the error with `?`".into()),
                    confidence: 0.85,
                    auto_fixable: false,
                });
            }

            // --- TODO / FIXME ---
            if trimmed.contains("TODO") || trimmed.contains("FIXME") {
                findings.push(ReviewFinding {
                    id: format!("PA-todo-{lineno}"),
                    severity: ReviewSeverity::Suggestion,
                    category: ReviewCategory::Documentation,
                    title: "TODO/FIXME comment".into(),
                    description: format!("Line {lineno}: Unresolved TODO/FIXME marker."),
                    file_path: String::new(),
                    line_start: lineno,
                    line_end: lineno,
                    suggestion: None,
                    confidence: 0.95,
                    auto_fixable: false,
                });
            }

            // --- magic numbers (integers >= 2 digits, not 0/1/10/100, outside const) ---
            if !trimmed.starts_with("const ")
                && !trimmed.starts_with("let ")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("///")
                && !trimmed.starts_with("#")
            {
                for word in trimmed.split(|c: char| !c.is_ascii_digit()) {
                    if word.len() >= 2 {
                        if let Ok(n) = word.parse::<i64>() {
                            if n.abs() > 1 && n != 10 && n != 100 && n != 1000 {
                                findings.push(ReviewFinding {
                                    id: format!("PA-magic-{lineno}-{n}"),
                                    severity: ReviewSeverity::Suggestion,
                                    category: ReviewCategory::Style,
                                    title: "Magic number".into(),
                                    description: format!(
                                        "Line {lineno}: Magic number `{n}` — consider extracting to a named constant."
                                    ),
                                    file_path: String::new(),
                                    line_start: lineno,
                                    line_end: lineno,
                                    suggestion: Some(format!("Extract `{n}` into a named constant.")),
                                    confidence: 0.55,
                                    auto_fixable: false,
                                });
                            }
                        }
                    }
                }
            }

            // --- hardcoded secrets patterns ---
            let lower = trimmed.to_lowercase();
            if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key") || lower.contains("apikey"))
                && (trimmed.contains('=') || trimmed.contains(':'))
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("#")
                && !trimmed.starts_with("///")
            {
                // Heuristic: looks like assignment of a literal
                if trimmed.contains('"') || trimmed.contains('\'') {
                    findings.push(ReviewFinding {
                        id: format!("PA-secret-{lineno}"),
                        severity: ReviewSeverity::Critical,
                        category: ReviewCategory::Security,
                        title: "Possible hardcoded secret".into(),
                        description: format!(
                            "Line {lineno}: Potential hardcoded credential. Use environment variables or a secret manager."
                        ),
                        file_path: String::new(),
                        line_start: lineno,
                        line_end: lineno,
                        suggestion: Some("Move secret to an environment variable or vault.".into()),
                        confidence: 0.75,
                        auto_fixable: false,
                    });
                }
            }

            // --- empty catch blocks (JS/TS) ---
            if (file_ext == "ts" || file_ext == "js" || file_ext == "tsx" || file_ext == "jsx")
                && trimmed.contains("catch")
                && trimmed.contains("{}")
            {
                findings.push(ReviewFinding {
                    id: format!("PA-emptycatch-{lineno}"),
                    severity: ReviewSeverity::Warning,
                    category: ReviewCategory::ErrorHandling,
                    title: "Empty catch block".into(),
                    description: format!("Line {lineno}: Empty catch block swallows errors silently."),
                    file_path: String::new(),
                    line_start: lineno,
                    line_end: lineno,
                    suggestion: Some("Log the error or handle it explicitly.".into()),
                    confidence: 0.90,
                    auto_fixable: false,
                });
            }
        }

        // --- long functions (> 50 lines) ---
        Self::detect_long_functions(content, file_ext, &mut findings);

        // --- deep nesting (> 4 levels) ---
        Self::detect_deep_nesting(content, &mut findings);

        // --- unused imports (Rust) ---
        if file_ext == "rs" || file_ext == "rust" {
            Self::detect_unused_imports_rust(content, &mut findings);
        }

        findings
    }

    fn detect_long_functions(content: &str, file_ext: &str, findings: &mut Vec<ReviewFinding>) {
        let lines: Vec<&str> = content.lines().collect();
        let fn_keyword = if file_ext == "rs" || file_ext == "rust" {
            "fn "
        } else {
            "function "
        };

        let mut fn_start: Option<(usize, String)> = None;
        let mut brace_depth: i32 = 0;
        let mut fn_brace_depth: i32 = 0;

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if fn_start.is_none() && trimmed.contains(fn_keyword) && !trimmed.starts_with("//") {
                // Extract function name heuristically
                let name = trimmed
                    .split(fn_keyword)
                    .nth(1)
                    .unwrap_or("")
                    .split(|c: char| c == '(' || c == '<' || c == ' ')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                fn_start = Some((idx + 1, name));
                fn_brace_depth = brace_depth;
            }

            brace_depth += line.chars().filter(|&c| c == '{').count() as i32;
            brace_depth -= line.chars().filter(|&c| c == '}').count() as i32;

            if let Some((start_line, ref name)) = fn_start {
                if brace_depth <= fn_brace_depth && idx + 1 > start_line {
                    let length = idx + 1 - start_line;
                    if length > 50 {
                        findings.push(ReviewFinding {
                            id: format!("PA-longfn-{start_line}"),
                            severity: ReviewSeverity::Warning,
                            category: ReviewCategory::Complexity,
                            title: format!("Long function `{name}` ({length} lines)"),
                            description: format!(
                                "Function `{name}` starting at line {start_line} is {length} lines. \
                                 Consider splitting into smaller functions."
                            ),
                            file_path: String::new(),
                            line_start: start_line,
                            line_end: idx + 1,
                            suggestion: Some("Extract logical sub-steps into helper functions.".into()),
                            confidence: 0.80,
                            auto_fixable: false,
                        });
                    }
                    fn_start = None;
                }
            }
        }
    }

    fn detect_deep_nesting(content: &str, findings: &mut Vec<ReviewFinding>) {
        let mut depth: i32 = 0;
        for (idx, line) in content.lines().enumerate() {
            depth += line.chars().filter(|&c| c == '{').count() as i32;
            if depth > 4 {
                findings.push(ReviewFinding {
                    id: format!("PA-nesting-{}", idx + 1),
                    severity: ReviewSeverity::Suggestion,
                    category: ReviewCategory::Complexity,
                    title: format!("Deep nesting (depth {depth})"),
                    description: format!(
                        "Line {}: Nesting depth is {depth} (>4). Consider early returns or extracting helpers.",
                        idx + 1
                    ),
                    file_path: String::new(),
                    line_start: idx + 1,
                    line_end: idx + 1,
                    suggestion: Some("Use early returns, guard clauses, or helper functions.".into()),
                    confidence: 0.70,
                    auto_fixable: false,
                });
            }
            depth -= line.chars().filter(|&c| c == '}').count() as i32;
        }
    }

    fn detect_unused_imports_rust(content: &str, findings: &mut Vec<ReviewFinding>) {
        let lines: Vec<&str> = content.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("use ") {
                continue;
            }
            // Extract the last segment of the import (the actual name used in code)
            let import_path = trimmed
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim();
            // Handle `use foo::bar::Baz;` — look for `Baz`
            if let Some(name) = import_path.split("::").last() {
                // Skip glob imports and braced groups
                if name.contains('*') || name.contains('{') {
                    continue;
                }
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                // Count occurrences of the name in non-import lines
                let usage_count = lines.iter().enumerate().filter(|(i, l)| {
                    *i != idx && !l.trim().starts_with("use ") && l.contains(name)
                }).count();
                if usage_count == 0 {
                    findings.push(ReviewFinding {
                        id: format!("PA-unused-import-{}", idx + 1),
                        severity: ReviewSeverity::Suggestion,
                        category: ReviewCategory::Style,
                        title: format!("Possibly unused import `{name}`"),
                        description: format!(
                            "Line {}: `{name}` does not appear elsewhere in the file.",
                            idx + 1
                        ),
                        file_path: String::new(),
                        line_start: idx + 1,
                        line_end: idx + 1,
                        suggestion: Some(format!("Remove the unused import `{name}`.")),
                        confidence: 0.60,
                        auto_fixable: true,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NamingChecker
// ---------------------------------------------------------------------------

pub struct NamingChecker;

impl NamingChecker {
    /// Check Rust naming conventions: snake_case for functions, CamelCase for types.
    pub fn check_rust(content: &str) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let lineno = idx + 1;

            // Function names — `fn some_name(`
            if let Some(rest) = trimmed.strip_prefix("fn ").or_else(|| trimmed.strip_prefix("pub fn ").or_else(|| trimmed.strip_prefix("pub(crate) fn "))) {
                let name = rest.split(|c: char| c == '(' || c == '<' || c == ' ')
                    .next()
                    .unwrap_or("");
                if !name.is_empty() && !Self::is_snake_case(name) {
                    findings.push(ReviewFinding {
                        id: format!("NC-rust-fn-{lineno}"),
                        severity: ReviewSeverity::Warning,
                        category: ReviewCategory::Naming,
                        title: format!("Function `{name}` is not snake_case"),
                        description: format!(
                            "Line {lineno}: Rust convention requires snake_case for function names."
                        ),
                        file_path: String::new(),
                        line_start: lineno,
                        line_end: lineno,
                        suggestion: Some(format!("Rename to `{}`", Self::to_snake_case(name))),
                        confidence: 0.90,
                        auto_fixable: false,
                    });
                }
            }

            // Type names — `struct Foo`, `enum Bar`, `trait Baz`
            for kw in &["struct ", "enum ", "trait "] {
                let prefix_variants = [
                    format!("pub {kw}"),
                    format!("pub(crate) {kw}"),
                    kw.to_string(),
                ];
                for prefix in &prefix_variants {
                    if let Some(rest) = trimmed.strip_prefix(prefix.as_str()) {
                        let name = rest.split(|c: char| c == '{' || c == '(' || c == '<' || c == ' ' || c == ';')
                            .next()
                            .unwrap_or("");
                        if !name.is_empty() && !Self::is_camel_case(name) {
                            findings.push(ReviewFinding {
                                id: format!("NC-rust-type-{lineno}"),
                                severity: ReviewSeverity::Warning,
                                category: ReviewCategory::Naming,
                                title: format!("Type `{name}` is not CamelCase"),
                                description: format!(
                                    "Line {lineno}: Rust convention requires CamelCase for type names."
                                ),
                                file_path: String::new(),
                                line_start: lineno,
                                line_end: lineno,
                                suggestion: None,
                                confidence: 0.90,
                                auto_fixable: false,
                            });
                        }
                        break;
                    }
                }
            }
        }
        findings
    }

    /// Check TypeScript naming conventions: camelCase functions, PascalCase components.
    pub fn check_typescript(content: &str) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let lineno = idx + 1;

            // function declarations
            if let Some(rest) = trimmed.strip_prefix("function ") {
                let name = rest.split(|c: char| c == '(' || c == '<' || c == ' ')
                    .next()
                    .unwrap_or("");
                if !name.is_empty() && !Self::is_camel_case_lower(name) {
                    findings.push(ReviewFinding {
                        id: format!("NC-ts-fn-{lineno}"),
                        severity: ReviewSeverity::Warning,
                        category: ReviewCategory::Naming,
                        title: format!("Function `{name}` is not camelCase"),
                        description: format!(
                            "Line {lineno}: TypeScript convention uses camelCase for function names."
                        ),
                        file_path: String::new(),
                        line_start: lineno,
                        line_end: lineno,
                        suggestion: None,
                        confidence: 0.85,
                        auto_fixable: false,
                    });
                }
            }

            // React components — `const FooBar = (` or `export const FooBar`
            if (trimmed.starts_with("const ") || trimmed.starts_with("export const "))
                && (trimmed.contains("React.FC") || trimmed.contains(": FC") || trimmed.contains("=> {") || trimmed.contains("=> ("))
            {
                let after_const = if let Some(r) = trimmed.strip_prefix("export const ") {
                    r
                } else {
                    trimmed.strip_prefix("const ").unwrap_or("")
                };
                let name = after_const.split(|c: char| c == ' ' || c == ':' || c == '=')
                    .next()
                    .unwrap_or("");
                if !name.is_empty() && !Self::is_pascal_case(name) {
                    findings.push(ReviewFinding {
                        id: format!("NC-ts-comp-{lineno}"),
                        severity: ReviewSeverity::Warning,
                        category: ReviewCategory::Naming,
                        title: format!("Component `{name}` is not PascalCase"),
                        description: format!(
                            "Line {lineno}: React components should use PascalCase."
                        ),
                        file_path: String::new(),
                        line_start: lineno,
                        line_end: lineno,
                        suggestion: None,
                        confidence: 0.85,
                        auto_fixable: false,
                    });
                }
            }
        }
        findings
    }

    // --- helpers ---

    fn is_snake_case(s: &str) -> bool {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !s.starts_with('_')
    }

    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_ascii_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
        result
    }

    fn is_camel_case(s: &str) -> bool {
        !s.is_empty()
            && s.chars().next().map_or(false, |c| c.is_ascii_uppercase())
            && !s.contains('_')
    }

    fn is_camel_case_lower(s: &str) -> bool {
        !s.is_empty()
            && s.chars().next().map_or(false, |c| c.is_ascii_lowercase())
            && !s.contains('_')
    }

    fn is_pascal_case(s: &str) -> bool {
        Self::is_camel_case(s)
    }
}

// ---------------------------------------------------------------------------
// CodeReviewAgent
// ---------------------------------------------------------------------------

pub struct CodeReviewAgent {
    pub config: ReviewConfig,
    sessions: HashMap<String, ReviewSession>,
    rules: Vec<ReviewRule>,
    conventions: Vec<ConventionRule>,
    metrics: ReviewMetrics,
    next_id: u64,
}

impl CodeReviewAgent {
    pub fn new(config: ReviewConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            rules: Vec::new(),
            conventions: Vec::new(),
            metrics: ReviewMetrics::default(),
            next_id: 1,
        }
    }

    /// Review a single file.
    pub fn review_file(&mut self, path: &str, content: &str) -> ReviewSession {
        let session_id = self.next_session_id();
        let file_ext = Self::file_ext(path);
        let total_lines = content.lines().count();

        let mut findings = PatternAnalyzer::analyze(content, file_ext);

        // Naming checks
        match file_ext {
            "rs" | "rust" => findings.extend(NamingChecker::check_rust(content)),
            "ts" | "tsx" => findings.extend(NamingChecker::check_typescript(content)),
            "js" | "jsx" => findings.extend(NamingChecker::check_typescript(content)),
            _ => {}
        }

        // Apply custom rules
        for rule in &self.rules {
            if rule.languages.is_empty() || rule.languages.iter().any(|l| l == file_ext) {
                for (idx, line) in content.lines().enumerate() {
                    if line.contains(&rule.pattern) {
                        findings.push(ReviewFinding {
                            id: format!("CR-{}-{}", rule.id, idx + 1),
                            severity: rule.severity.clone(),
                            category: rule.category.clone(),
                            title: rule.message.clone(),
                            description: format!("Line {}: matched rule `{}`.", idx + 1, rule.id),
                            file_path: path.to_string(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            suggestion: None,
                            confidence: 0.80,
                            auto_fixable: false,
                        });
                    }
                }
            }
        }

        // Set file_path on all findings
        for f in &mut findings {
            if f.file_path.is_empty() {
                f.file_path = path.to_string();
            }
        }

        // Filter by config
        self.apply_config_filters(&mut findings);

        // Add praise if enabled
        if self.config.include_praise && findings.iter().all(|f| f.severity != ReviewSeverity::Critical) {
            findings.push(ReviewFinding {
                id: format!("PRAISE-{session_id}"),
                severity: ReviewSeverity::Praise,
                category: ReviewCategory::Style,
                title: "Clean code".into(),
                description: "No critical issues found — nice work!".into(),
                file_path: path.to_string(),
                line_start: 0,
                line_end: 0,
                suggestion: None,
                confidence: 1.0,
                auto_fixable: false,
            });
        }

        let session = ReviewSession {
            id: session_id.clone(),
            scope: ReviewScope::File,
            findings: findings.clone(),
            files_reviewed: vec![path.to_string()],
            status: ReviewStatus::Complete,
            started_at: 1_700_000_000,
            completed_at: Some(1_700_000_001),
            total_lines,
        };

        self.update_metrics(&session);
        self.sessions.insert(session_id, session.clone());
        session
    }

    /// Review a diff for a given file.
    pub fn review_diff(&mut self, diff: &str, file_path: &str) -> ReviewSession {
        let session_id = self.next_session_id();
        let file_ext = Self::file_ext(file_path);
        let mut findings = Vec::new();
        let total_lines = diff.lines().count();

        // Extract only added lines from the diff
        let added_content: String = diff
            .lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
            .map(|l| &l[1..])
            .collect::<Vec<_>>()
            .join("\n");

        if !added_content.is_empty() {
            findings.extend(PatternAnalyzer::analyze(&added_content, file_ext));
        }

        for f in &mut findings {
            f.file_path = file_path.to_string();
        }

        self.apply_config_filters(&mut findings);

        let session = ReviewSession {
            id: session_id.clone(),
            scope: ReviewScope::Diff,
            findings: findings.clone(),
            files_reviewed: vec![file_path.to_string()],
            status: ReviewStatus::Complete,
            started_at: 1_700_000_000,
            completed_at: Some(1_700_000_001),
            total_lines,
        };

        self.update_metrics(&session);
        self.sessions.insert(session_id, session.clone());
        session
    }

    /// Batch-review multiple files.
    pub fn review_files(&mut self, files: &[(&str, &str)]) -> ReviewSession {
        let session_id = self.next_session_id();
        let mut all_findings = Vec::new();
        let mut files_reviewed = Vec::new();
        let mut total_lines = 0;

        for (path, content) in files {
            let file_ext = Self::file_ext(path);
            total_lines += content.lines().count();
            files_reviewed.push(path.to_string());

            let mut file_findings = PatternAnalyzer::analyze(content, file_ext);
            match file_ext {
                "rs" | "rust" => file_findings.extend(NamingChecker::check_rust(content)),
                "ts" | "tsx" | "js" | "jsx" => {
                    file_findings.extend(NamingChecker::check_typescript(content))
                }
                _ => {}
            }
            for f in &mut file_findings {
                f.file_path = path.to_string();
            }
            all_findings.extend(file_findings);
        }

        self.apply_config_filters(&mut all_findings);

        let session = ReviewSession {
            id: session_id.clone(),
            scope: ReviewScope::File,
            findings: all_findings,
            files_reviewed,
            status: ReviewStatus::Complete,
            started_at: 1_700_000_000,
            completed_at: Some(1_700_000_001),
            total_lines,
        };

        self.update_metrics(&session);
        self.sessions.insert(session_id, session.clone());
        session
    }

    pub fn add_rule(&mut self, rule: ReviewRule) {
        self.rules.push(rule);
    }

    pub fn add_convention(&mut self, conv: ConventionRule) {
        self.conventions.push(conv);
    }

    pub fn get_session(&self, id: &str) -> Option<&ReviewSession> {
        self.sessions.get(id)
    }

    pub fn list_sessions(&self) -> Vec<&ReviewSession> {
        self.sessions.values().collect()
    }

    pub fn dismiss(&mut self, session_id: &str, finding_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session `{session_id}` not found"))?;
        let finding = session
            .findings
            .iter_mut()
            .find(|f| f.id == finding_id)
            .ok_or_else(|| format!("Finding `{finding_id}` not found in session `{session_id}`"))?;
        finding.confidence = 0.0; // mark dismissed
        Ok(())
    }

    /// Return (file_path, suggested_fix_content) pairs for auto-fixable findings.
    pub fn auto_fix_suggestions(&self, session_id: &str) -> Vec<(String, String)> {
        let Some(session) = self.sessions.get(session_id) else {
            return Vec::new();
        };
        session
            .findings
            .iter()
            .filter(|f| f.auto_fixable && f.suggestion.is_some())
            .map(|f| (f.file_path.clone(), f.suggestion.clone().unwrap_or_default()))
            .collect()
    }

    pub fn get_metrics(&self) -> &ReviewMetrics {
        &self.metrics
    }

    // --- private helpers ---

    fn next_session_id(&mut self) -> String {
        let id = format!("review-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn file_ext(path: &str) -> &str {
        path.rsplit('.').next().unwrap_or("")
    }

    fn apply_config_filters(&self, findings: &mut Vec<ReviewFinding>) {
        findings.retain(|f| {
            f.confidence >= self.config.min_confidence
                && self.config.categories.contains(&f.category)
                && (f.severity != ReviewSeverity::Praise || self.config.include_praise)
        });
        // Auto-dismiss low-confidence
        for f in findings.iter_mut() {
            if f.confidence < self.config.auto_dismiss_below {
                f.confidence = 0.0;
            }
        }
        findings.truncate(self.config.max_findings);
    }

    fn update_metrics(&mut self, session: &ReviewSession) {
        self.metrics.total_reviews += 1;
        self.metrics.total_findings += session.findings.len() as u64;
        for f in &session.findings {
            *self
                .metrics
                .by_severity
                .entry(format!("{:?}", f.severity))
                .or_insert(0) += 1;
            *self
                .metrics
                .by_category
                .entry(format!("{:?}", f.category))
                .or_insert(0) += 1;
        }
        if self.metrics.total_reviews > 0 {
            self.metrics.avg_findings_per_review =
                self.metrics.total_findings as f64 / self.metrics.total_reviews as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_agent() -> CodeReviewAgent {
        CodeReviewAgent::new(ReviewConfig::default())
    }

    // -- Enum / struct sanity -----------------------------------------------

    #[test]
    fn test_severity_variants() {
        let s = ReviewSeverity::Critical;
        assert_eq!(s, ReviewSeverity::Critical);
        assert_ne!(s, ReviewSeverity::Praise);
    }

    #[test]
    fn test_category_variants() {
        let c = ReviewCategory::Security;
        assert_ne!(c, ReviewCategory::Performance);
    }

    #[test]
    fn test_scope_variants() {
        assert_ne!(ReviewScope::File, ReviewScope::Diff);
        assert_ne!(ReviewScope::PR, ReviewScope::Commit);
    }

    #[test]
    fn test_status_variants() {
        assert_ne!(ReviewStatus::Pending, ReviewStatus::Complete);
    }

    #[test]
    fn test_default_config() {
        let cfg = ReviewConfig::default();
        assert!((cfg.min_confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(cfg.max_findings, 50);
        assert!(cfg.include_praise);
        assert!((cfg.auto_dismiss_below - 0.3).abs() < f64::EPSILON);
        assert_eq!(cfg.categories.len(), 10);
    }

    #[test]
    fn test_default_metrics() {
        let m = ReviewMetrics::default();
        assert_eq!(m.total_reviews, 0);
        assert_eq!(m.total_findings, 0);
        assert!(m.by_severity.is_empty());
    }

    // -- PatternAnalyzer ---------------------------------------------------

    #[test]
    fn test_detect_unwrap_in_rust() {
        let code = "let x = foo().unwrap();\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("unwrap")));
    }

    #[test]
    fn test_no_unwrap_warning_in_test_code() {
        let code = "#[cfg(test)]\nmod tests {\n  let x = foo().unwrap();\n}\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(!findings.iter().any(|f| f.title.contains("unwrap")));
    }

    #[test]
    fn test_detect_todo() {
        let code = "// TODO: fix this later\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("TODO")));
    }

    #[test]
    fn test_detect_fixme() {
        let code = "// FIXME: broken\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("TODO")));
    }

    #[test]
    fn test_detect_hardcoded_secret() {
        let code = "let password = \"hunter2\";\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.category == ReviewCategory::Security));
    }

    #[test]
    fn test_no_secret_in_comment() {
        let code = "// password = \"hunter2\"\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(!findings.iter().any(|f| f.category == ReviewCategory::Security));
    }

    #[test]
    fn test_detect_empty_catch() {
        let code = "try { foo(); } catch (e) {}\n";
        let findings = PatternAnalyzer::analyze(code, "ts");
        assert!(findings.iter().any(|f| f.title.contains("catch")));
    }

    #[test]
    fn test_no_empty_catch_in_rust() {
        let code = "try { foo(); } catch (e) {}\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(!findings.iter().any(|f| f.title.contains("catch")));
    }

    #[test]
    fn test_detect_deep_nesting() {
        let code = "fn a() {\n  if true {\n    if true {\n      if true {\n        if true {\n          if true {\n          }\n        }\n      }\n    }\n  }\n}\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("nesting")));
    }

    #[test]
    fn test_detect_long_function() {
        let mut code = String::from("fn very_long() {\n");
        for i in 0..55 {
            code.push_str(&format!("    let x{i} = {i};\n"));
        }
        code.push_str("}\n");
        let findings = PatternAnalyzer::analyze(&code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("Long function")));
    }

    #[test]
    fn test_short_function_no_warning() {
        let code = "fn short() {\n    let x = 1;\n}\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(!findings.iter().any(|f| f.title.contains("Long function")));
    }

    #[test]
    fn test_detect_unused_import() {
        let code = "use std::collections::BTreeSet;\nfn main() {}\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(findings.iter().any(|f| f.title.contains("unused import")));
    }

    #[test]
    fn test_used_import_no_warning() {
        let code = "use std::collections::HashMap;\nfn main() { let _m: HashMap<(), ()> = HashMap::new(); }\n";
        let findings = PatternAnalyzer::analyze(code, "rs");
        assert!(!findings.iter().any(|f| f.title.contains("unused import") && f.title.contains("HashMap")));
    }

    // -- NamingChecker -----------------------------------------------------

    #[test]
    fn test_rust_snake_case_valid() {
        let code = "fn my_function() {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_rust_snake_case_violation() {
        let code = "fn myFunction() {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.iter().any(|f| f.title.contains("not snake_case")));
    }

    #[test]
    fn test_rust_camel_case_type_valid() {
        let code = "struct MyStruct {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(!findings.iter().any(|f| f.title.contains("not CamelCase")));
    }

    #[test]
    fn test_rust_camel_case_type_violation() {
        let code = "struct my_struct {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.iter().any(|f| f.title.contains("not CamelCase")));
    }

    #[test]
    fn test_rust_enum_camel_case() {
        let code = "enum good_enum {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.iter().any(|f| f.title.contains("not CamelCase")));
    }

    #[test]
    fn test_typescript_function_camel_case() {
        let code = "function myFunc() {}\n";
        let findings = NamingChecker::check_typescript(code);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_typescript_function_violation() {
        let code = "function MyFunc() {}\n";
        let findings = NamingChecker::check_typescript(code);
        assert!(findings.iter().any(|f| f.title.contains("not camelCase")));
    }

    #[test]
    fn test_typescript_component_pascal_case() {
        let code = "export const MyComponent: React.FC = () => {\n";
        let findings = NamingChecker::check_typescript(code);
        assert!(!findings.iter().any(|f| f.title.contains("not PascalCase")));
    }

    #[test]
    fn test_typescript_component_violation() {
        let code = "export const myComponent: React.FC = () => {\n";
        let findings = NamingChecker::check_typescript(code);
        assert!(findings.iter().any(|f| f.title.contains("not PascalCase")));
    }

    // -- CodeReviewAgent ---------------------------------------------------

    #[test]
    fn test_review_file_creates_session() {
        let mut agent = default_agent();
        let session = agent.review_file("test.rs", "fn main() {}\n");
        assert_eq!(session.status, ReviewStatus::Complete);
        assert_eq!(session.files_reviewed, vec!["test.rs"]);
        assert_eq!(session.scope, ReviewScope::File);
    }

    #[test]
    fn test_review_file_with_findings() {
        let mut agent = default_agent();
        let code = "let api_key = \"sk-12345\";\nlet x = foo().unwrap();\n";
        let session = agent.review_file("app.rs", code);
        assert!(!session.findings.is_empty());
    }

    #[test]
    fn test_review_diff() {
        let mut agent = default_agent();
        let diff = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,1 +1,2 @@\n+let x = bar().unwrap();\n";
        let session = agent.review_diff(diff, "foo.rs");
        assert_eq!(session.scope, ReviewScope::Diff);
        assert!(session.findings.iter().any(|f| f.title.contains("unwrap")));
    }

    #[test]
    fn test_review_files_batch() {
        let mut agent = default_agent();
        let files: Vec<(&str, &str)> = vec![
            ("a.rs", "fn main() {}\n"),
            ("b.rs", "let password = \"abc\";\n"),
        ];
        let session = agent.review_files(&files);
        assert_eq!(session.files_reviewed.len(), 2);
    }

    #[test]
    fn test_add_custom_rule() {
        let mut agent = default_agent();
        agent.add_rule(ReviewRule {
            id: "no-println".into(),
            pattern: "println!".into(),
            category: ReviewCategory::Style,
            severity: ReviewSeverity::Suggestion,
            message: "Avoid println! in library code".into(),
            languages: vec!["rs".into()],
        });
        let session = agent.review_file("lib.rs", "println!(\"hello\");\n");
        assert!(session.findings.iter().any(|f| f.title.contains("println")));
    }

    #[test]
    fn test_add_convention() {
        let mut agent = default_agent();
        agent.add_convention(ConventionRule {
            name: "error-prefix".into(),
            pattern: "Error".into(),
            description: "Error types should end with Error".into(),
            examples_good: vec!["ParseError".into()],
            examples_bad: vec!["ParseFail".into()],
        });
        assert_eq!(agent.conventions.len(), 1);
    }

    #[test]
    fn test_get_session() {
        let mut agent = default_agent();
        let session = agent.review_file("f.rs", "fn a() {}\n");
        let retrieved = agent.get_session(&session.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }

    #[test]
    fn test_list_sessions() {
        let mut agent = default_agent();
        agent.review_file("a.rs", "");
        agent.review_file("b.rs", "");
        assert_eq!(agent.list_sessions().len(), 2);
    }

    #[test]
    fn test_dismiss_finding() {
        let mut agent = default_agent();
        let code = "let x = foo().unwrap();\n";
        let session = agent.review_file("f.rs", code);
        let finding_id = session.findings.iter().find(|f| f.title.contains("unwrap")).map(|f| f.id.clone());
        if let Some(fid) = finding_id {
            assert!(agent.dismiss(&session.id, &fid).is_ok());
        }
    }

    #[test]
    fn test_dismiss_nonexistent_session() {
        let mut agent = default_agent();
        assert!(agent.dismiss("no-such", "no-such").is_err());
    }

    #[test]
    fn test_dismiss_nonexistent_finding() {
        let mut agent = default_agent();
        let session = agent.review_file("f.rs", "let x = foo().unwrap();\n");
        assert!(agent.dismiss(&session.id, "nope").is_err());
    }

    #[test]
    fn test_auto_fix_suggestions() {
        let mut agent = default_agent();
        let code = "use std::collections::BTreeSet;\nfn main() {}\n";
        let session = agent.review_file("f.rs", code);
        let fixes = agent.auto_fix_suggestions(&session.id);
        // Unused import is auto_fixable
        assert!(!fixes.is_empty() || session.findings.iter().any(|f| f.auto_fixable));
    }

    #[test]
    fn test_auto_fix_no_session() {
        let agent = default_agent();
        assert!(agent.auto_fix_suggestions("nope").is_empty());
    }

    #[test]
    fn test_metrics_updated() {
        let mut agent = default_agent();
        agent.review_file("f.rs", "let x = foo().unwrap();\n");
        let m = agent.get_metrics();
        assert_eq!(m.total_reviews, 1);
        assert!(m.total_findings > 0);
    }

    #[test]
    fn test_metrics_avg_findings() {
        let mut agent = default_agent();
        agent.review_file("a.rs", "fn a() {}\n");
        agent.review_file("b.rs", "fn b() {}\n");
        let m = agent.get_metrics();
        assert_eq!(m.total_reviews, 2);
        assert!(m.avg_findings_per_review >= 0.0);
    }

    #[test]
    fn test_praise_included_when_clean() {
        let mut agent = default_agent();
        let session = agent.review_file("clean.rs", "fn main() {}\n");
        assert!(session.findings.iter().any(|f| f.severity == ReviewSeverity::Praise));
    }

    #[test]
    fn test_praise_excluded_when_disabled() {
        let mut cfg = ReviewConfig::default();
        cfg.include_praise = false;
        let mut agent = CodeReviewAgent::new(cfg);
        let session = agent.review_file("clean.rs", "fn main() {}\n");
        assert!(!session.findings.iter().any(|f| f.severity == ReviewSeverity::Praise));
    }

    #[test]
    fn test_min_confidence_filter() {
        let mut cfg = ReviewConfig::default();
        cfg.min_confidence = 0.99;
        let mut agent = CodeReviewAgent::new(cfg);
        let session = agent.review_file("f.rs", "let x = foo().unwrap();\n");
        // unwrap confidence is 0.85, should be filtered out
        assert!(!session.findings.iter().any(|f| f.title.contains("unwrap")));
    }

    #[test]
    fn test_max_findings_cap() {
        let mut cfg = ReviewConfig::default();
        cfg.max_findings = 1;
        let mut agent = CodeReviewAgent::new(cfg);
        let code = "let api_key = \"x\";\nlet password = \"y\";\nlet x = foo().unwrap();\n// TODO fix\n";
        let session = agent.review_file("f.rs", code);
        assert!(session.findings.len() <= 1);
    }

    #[test]
    fn test_category_filter() {
        let mut cfg = ReviewConfig::default();
        cfg.categories = vec![ReviewCategory::Security]; // only security
        cfg.include_praise = false;
        let mut agent = CodeReviewAgent::new(cfg);
        let code = "let x = foo().unwrap();\nlet password = \"abc\";\n";
        let session = agent.review_file("f.rs", code);
        for f in &session.findings {
            assert_eq!(f.category, ReviewCategory::Security);
        }
    }

    #[test]
    fn test_session_id_increments() {
        let mut agent = default_agent();
        let s1 = agent.review_file("a.rs", "");
        let s2 = agent.review_file("b.rs", "");
        assert_ne!(s1.id, s2.id);
        assert_eq!(s1.id, "review-1");
        assert_eq!(s2.id, "review-2");
    }

    #[test]
    fn test_file_ext_extraction() {
        assert_eq!(CodeReviewAgent::file_ext("src/main.rs"), "rs");
        assert_eq!(CodeReviewAgent::file_ext("app.tsx"), "tsx");
        assert_eq!(CodeReviewAgent::file_ext("noext"), "noext");
    }

    #[test]
    fn test_review_session_total_lines() {
        let mut agent = default_agent();
        let code = "line1\nline2\nline3\n";
        let session = agent.review_file("f.rs", code);
        assert_eq!(session.total_lines, 3);
    }

    #[test]
    fn test_serde_roundtrip_finding() {
        let finding = ReviewFinding {
            id: "test-1".into(),
            severity: ReviewSeverity::Warning,
            category: ReviewCategory::Performance,
            title: "Test".into(),
            description: "Desc".into(),
            file_path: "f.rs".into(),
            line_start: 1,
            line_end: 1,
            suggestion: Some("Fix it".into()),
            confidence: 0.9,
            auto_fixable: true,
        };
        let json = serde_json::to_string(&finding).expect("serialize");
        let back: ReviewFinding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "test-1");
        assert_eq!(back.severity, ReviewSeverity::Warning);
    }

    #[test]
    fn test_serde_roundtrip_config() {
        let cfg = ReviewConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: ReviewConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.max_findings, 50);
    }

    #[test]
    fn test_naming_helpers_snake_case() {
        assert!(NamingChecker::is_snake_case("my_func"));
        assert!(!NamingChecker::is_snake_case("myFunc"));
        assert!(!NamingChecker::is_snake_case("MyFunc"));
        assert!(!NamingChecker::is_snake_case("_leading"));
    }

    #[test]
    fn test_naming_helpers_camel_case() {
        assert!(NamingChecker::is_camel_case("MyStruct"));
        assert!(!NamingChecker::is_camel_case("my_struct"));
        assert!(!NamingChecker::is_camel_case("myStruct"));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(NamingChecker::to_snake_case("myFunction"), "my_function");
        assert_eq!(NamingChecker::to_snake_case("HTMLParser"), "h_t_m_l_parser");
        assert_eq!(NamingChecker::to_snake_case("getData"), "get_data");
    }

    #[test]
    fn test_custom_rule_language_filter() {
        let mut agent = default_agent();
        agent.add_rule(ReviewRule {
            id: "ts-only".into(),
            pattern: "console.log".into(),
            category: ReviewCategory::Style,
            severity: ReviewSeverity::Suggestion,
            message: "Remove console.log".into(),
            languages: vec!["ts".into()],
        });
        // Should NOT fire for Rust
        let session = agent.review_file("f.rs", "console.log(\"hi\");\n");
        assert!(!session.findings.iter().any(|f| f.title.contains("console.log")));
    }

    #[test]
    fn test_convention_rule_fields() {
        let conv = ConventionRule {
            name: "error-suffix".into(),
            pattern: "Error$".into(),
            description: "Error types end with Error".into(),
            examples_good: vec!["ParseError".into()],
            examples_bad: vec!["BadParse".into()],
        };
        assert_eq!(conv.examples_good.len(), 1);
        assert_eq!(conv.examples_bad.len(), 1);
    }

    #[test]
    fn test_review_empty_file() {
        let mut agent = default_agent();
        let session = agent.review_file("empty.rs", "");
        assert_eq!(session.total_lines, 0);
        assert_eq!(session.status, ReviewStatus::Complete);
    }

    #[test]
    fn test_pub_fn_naming_check() {
        let code = "pub fn badName() {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.iter().any(|f| f.title.contains("not snake_case")));
    }

    #[test]
    fn test_pub_struct_naming_check() {
        let code = "pub struct good_name {}\n";
        let findings = NamingChecker::check_rust(code);
        assert!(findings.iter().any(|f| f.title.contains("not CamelCase")));
    }
}
