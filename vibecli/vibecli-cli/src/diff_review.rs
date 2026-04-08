#![allow(dead_code)]
//! Intelligent diff review — risk assessment, regression detection, and test suggestions.

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Safe,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            RiskLevel::Critical
        } else if score >= 0.6 {
            RiskLevel::High
        } else if score >= 0.4 {
            RiskLevel::Medium
        } else if score >= 0.2 {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Low => "LOW",
            RiskLevel::Safe => "SAFE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpactArea {
    Security,
    PublicAPI,
    Database,
    Configuration,
    Tests,
    Documentation,
    Build,
    Dependencies,
}

impl ImpactArea {
    pub fn label(&self) -> &'static str {
        match self {
            ImpactArea::Security => "Security",
            ImpactArea::PublicAPI => "Public API",
            ImpactArea::Database => "Database",
            ImpactArea::Configuration => "Configuration",
            ImpactArea::Tests => "Tests",
            ImpactArea::Documentation => "Documentation",
            ImpactArea::Build => "Build",
            ImpactArea::Dependencies => "Dependencies",
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: String,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_score: f64,
    pub file_risks: Vec<FileRisk>,
    pub impact_areas: Vec<ImpactArea>,
    pub suggested_reviewers: Vec<String>,
    pub test_suggestions: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct FileRisk {
    pub path: String,
    pub risk: RiskLevel,
    pub score: f64,
    pub reasons: Vec<String>,
    pub impacted_areas: Vec<ImpactArea>,
}

#[derive(Debug, Clone)]
pub struct RegressionSignal {
    pub file_path: String,
    pub signal_type: String,
    pub description: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub total_hunks: usize,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub risk_threshold: f64,
    pub max_files: usize,
    pub include_test_suggestions: bool,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            risk_threshold: 0.5,
            max_files: 100,
            include_test_suggestions: true,
        }
    }
}

// ---------------------------------------------------------------------------
// RiskScorer
// ---------------------------------------------------------------------------

pub struct RiskScorer;

impl RiskScorer {
    /// Score a file based on its path and change magnitude.
    /// Returns (score, reasons) where score is in 0.0..=1.0.
    pub fn score_file(path: &str, additions: usize, deletions: usize) -> (f64, Vec<String>) {
        let mut score: f64 = 0.0;
        let mut reasons: Vec<String> = Vec::new();
        let lower = path.to_lowercase();

        // Security-sensitive files
        let security_patterns = [
            "auth", "crypto", "secret", "password", "token", "credential",
            "login", "session", "permission", "acl", "rbac", "oauth",
            "jwt", "cert", "tls", "ssl", "key",
        ];
        for pat in &security_patterns {
            if lower.contains(pat) {
                score += 0.3;
                reasons.push(format!("Security-sensitive file (matches '{pat}')"));
                break;
            }
        }

        // Public API changes
        let api_patterns = [
            "api/", "routes/", "handler", "controller", "endpoint",
            "openapi", "swagger", "graphql", "schema.rs", "schema.ts",
            "proto", "grpc",
        ];
        for pat in &api_patterns {
            if lower.contains(pat) {
                score += 0.2;
                reasons.push("Public API surface change".into());
                break;
            }
        }

        // Database migrations
        let db_patterns = [
            "migration", "migrate", "schema", "sql", "alembic",
            "flyway", "liquibase", "knex",
        ];
        for pat in &db_patterns {
            if lower.contains(pat) {
                score += 0.3;
                reasons.push("Database migration or schema change".into());
                break;
            }
        }

        // Large changes (>500 lines)
        let total_lines = additions + deletions;
        if total_lines > 500 {
            score += 0.2;
            reasons.push(format!("Large change ({total_lines} lines)"));
        } else if total_lines > 200 {
            score += 0.1;
            reasons.push(format!("Medium-sized change ({total_lines} lines)"));
        }

        // Deleting tests
        let is_test_file = lower.contains("test") || lower.contains("spec");
        if is_test_file && deletions > additions {
            score += 0.3;
            reasons.push("Test deletions exceed additions".into());
        }

        // Dependency changes
        let dep_patterns = [
            "cargo.toml", "package.json", "go.mod", "requirements.txt",
            "gemfile", "pom.xml", "build.gradle", "poetry.lock",
            "yarn.lock", "package-lock.json", "cargo.lock",
        ];
        for pat in &dep_patterns {
            if lower.ends_with(pat) || lower.contains(pat) {
                score += 0.15;
                reasons.push("Dependency file change".into());
                break;
            }
        }

        // Configuration files
        let config_patterns = [
            ".env", "config.toml", "config.yaml", "config.json",
            ".yml", "dockerfile", "docker-compose", "nginx.conf",
            "terraform", ".tf",
        ];
        for pat in &config_patterns {
            if lower.contains(pat) {
                score += 0.1;
                reasons.push("Configuration file change".into());
                break;
            }
        }

        // Clamp to 1.0
        score = score.min(1.0);

        if reasons.is_empty() {
            reasons.push("No specific risk factors detected".into());
        }

        (score, reasons)
    }
}

// ---------------------------------------------------------------------------
// TestSuggester
// ---------------------------------------------------------------------------

pub struct TestSuggester;

impl TestSuggester {
    /// Suggest test descriptions based on changed code in a file.
    pub fn suggest(file: &DiffFile) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lower_path = file.path.to_lowercase();

        // Detect new functions/methods in hunks
        for hunk in &file.hunks {
            let lines: Vec<&str> = hunk.content.lines().collect();
            for line in &lines {
                let trimmed = line.trim();

                // New function → unit test
                if let Some(stripped) = trimmed.strip_prefix('+') {
                    let code = &stripped.trim();
                    if code.starts_with("pub fn ")
                        || code.starts_with("fn ")
                        || code.starts_with("pub async fn ")
                        || code.starts_with("async fn ")
                    {
                        if let Some(name) = extract_fn_name(code) {
                            suggestions.push(format!(
                                "Add unit test for new function `{name}`"
                            ));
                        }
                    }

                    // New error path → error test
                    if code.contains("Err(") || code.contains("return Err")
                        || code.contains("bail!") || code.contains("anyhow!")
                        || code.contains("panic!") || code.contains("unwrap()")
                    {
                        suggestions.push(
                            "Add error-path test for new error handling code".into(),
                        );
                    }

                    // JS/TS function detection
                    if code.starts_with("export function ")
                        || code.starts_with("export const ")
                        || code.starts_with("function ")
                    {
                        if let Some(name) = extract_js_fn_name(code) {
                            suggestions.push(format!(
                                "Add unit test for new function `{name}`"
                            ));
                        }
                    }
                }
            }
        }

        // Changed API → integration test
        let api_patterns = ["api/", "routes/", "handler", "controller", "endpoint"];
        for pat in &api_patterns {
            if lower_path.contains(pat) {
                suggestions.push(format!(
                    "Add integration test for API changes in `{}`",
                    file.path
                ));
                break;
            }
        }

        // UI change → snapshot test
        let ui_extensions = [".tsx", ".jsx", ".vue", ".svelte"];
        for ext in &ui_extensions {
            if lower_path.ends_with(ext) {
                suggestions.push(format!(
                    "Add snapshot test for UI component `{}`",
                    file.path
                ));
                break;
            }
        }

        // Config change → validation test
        let config_exts = [".toml", ".yaml", ".yml", ".json"];
        let is_config = config_exts.iter().any(|e| lower_path.ends_with(e))
            && !lower_path.contains("lock");
        if is_config {
            suggestions.push(format!(
                "Add config validation test for `{}`",
                file.path
            ));
        }

        // Database change → migration test
        if lower_path.contains("migration") || lower_path.contains(".sql") {
            suggestions.push("Add migration test (up + down rollback)".into());
        }

        // Deduplicate
        suggestions.sort();
        suggestions.dedup();
        suggestions
    }
}

fn extract_fn_name(code: &str) -> Option<String> {
    // Parse "pub fn foo(" or "fn foo(" etc.
    let code = code
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("pub ")
        .trim_start_matches("fn ");
    let end = code.find('(')?;
    let name = code[..end].trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}

fn extract_js_fn_name(code: &str) -> Option<String> {
    let code = code
        .trim_start_matches("export ")
        .trim_start_matches("default ");
    if let Some(rest) = code.strip_prefix("function ") {
        let end = rest.find('(')?;
        let name = rest[..end].trim();
        if name.is_empty() { None } else { Some(name.to_string()) }
    } else if let Some(rest) = code.strip_prefix("const ") {
        let end = rest.find(' ').or_else(|| rest.find('='))?;
        let name = rest[..end].trim();
        if name.is_empty() { None } else { Some(name.to_string()) }
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// DiffAnalyzer
// ---------------------------------------------------------------------------

pub struct DiffAnalyzer {
    config: ReviewConfig,
}

impl DiffAnalyzer {
    pub fn new(config: ReviewConfig) -> Self {
        Self { config }
    }

    /// Parse unified diff text into a list of DiffFile structs.
    pub fn parse_diff(diff_text: &str) -> Vec<DiffFile> {
        let mut files: Vec<DiffFile> = Vec::new();
        let mut current_path: Option<String> = None;
        let mut current_change_type = ChangeType::Modified;
        let mut current_hunks: Vec<DiffHunk> = Vec::new();
        let mut additions: usize = 0;
        let mut deletions: usize = 0;

        // Hunk accumulation state
        let mut in_hunk = false;
        let mut hunk_start: usize = 0;
        let mut hunk_end: usize = 0;
        let mut hunk_content = String::new();
        let mut hunk_context = String::new();

        let lines: Vec<&str> = diff_text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Detect file header: "diff --git a/... b/..."
            if line.starts_with("diff --git ") {
                // Flush previous file
                if let Some(path) = current_path.take() {
                    if in_hunk && !hunk_content.is_empty() {
                        current_hunks.push(DiffHunk {
                            start_line: hunk_start,
                            end_line: hunk_end,
                            content: hunk_content.clone(),
                            context: hunk_context.clone(),
                        });
                    }
                    files.push(DiffFile {
                        path,
                        change_type: current_change_type,
                        additions,
                        deletions,
                        hunks: current_hunks.clone(),
                    });
                }

                // Reset
                current_hunks.clear();
                additions = 0;
                deletions = 0;
                in_hunk = false;
                hunk_content.clear();
                hunk_context.clear();
                current_change_type = ChangeType::Modified;

                // Extract path from "diff --git a/foo b/foo"
                if let Some(b_part) = line.split(" b/").last() {
                    current_path = Some(b_part.to_string());
                }

                i += 1;
                continue;
            }

            // Detect new/deleted file markers
            if line.starts_with("new file") {
                current_change_type = ChangeType::Added;
                i += 1;
                continue;
            }
            if line.starts_with("deleted file") {
                current_change_type = ChangeType::Deleted;
                i += 1;
                continue;
            }
            if line.starts_with("rename from") || line.starts_with("similarity index") {
                current_change_type = ChangeType::Renamed;
                i += 1;
                continue;
            }
            if line.starts_with("rename to") {
                if let Some(path) = line.strip_prefix("rename to ") {
                    current_path = Some(path.to_string());
                }
                i += 1;
                continue;
            }

            // Hunk header: @@ -a,b +c,d @@
            if line.starts_with("@@") {
                // Flush previous hunk
                if in_hunk && !hunk_content.is_empty() {
                    current_hunks.push(DiffHunk {
                        start_line: hunk_start,
                        end_line: hunk_end,
                        content: hunk_content.clone(),
                        context: hunk_context.clone(),
                    });
                }
                hunk_content.clear();
                hunk_context.clear();
                in_hunk = true;

                // Parse line numbers from "@@ -old,len +new,len @@"
                let (start, end) = parse_hunk_header(line);
                hunk_start = start;
                hunk_end = end;

                // Context text after @@
                if let Some(ctx) = line.split("@@").nth(2) {
                    hunk_context = ctx.trim().to_string();
                }

                i += 1;
                continue;
            }

            // Count additions/deletions inside hunks
            if in_hunk {
                if line.starts_with('+') && !line.starts_with("+++") {
                    additions += 1;
                    hunk_content.push_str(line);
                    hunk_content.push('\n');
                    if hunk_end < hunk_start + additions + deletions {
                        hunk_end = hunk_start + additions + deletions;
                    }
                } else if line.starts_with('-') && !line.starts_with("---") {
                    deletions += 1;
                    hunk_content.push_str(line);
                    hunk_content.push('\n');
                    if hunk_end < hunk_start + additions + deletions {
                        hunk_end = hunk_start + additions + deletions;
                    }
                } else if line.starts_with(' ') {
                    // Context line
                    hunk_content.push_str(line);
                    hunk_content.push('\n');
                }
            }

            i += 1;
        }

        // Flush last file
        if let Some(path) = current_path.take() {
            if in_hunk && !hunk_content.is_empty() {
                current_hunks.push(DiffHunk {
                    start_line: hunk_start,
                    end_line: hunk_end,
                    content: hunk_content,
                    context: hunk_context,
                });
            }
            files.push(DiffFile {
                path,
                change_type: current_change_type,
                additions,
                deletions,
                hunks: current_hunks,
            });
        }

        files
    }

    /// Full risk assessment of a set of diff files.
    pub fn analyze(&self, files: &[DiffFile]) -> RiskAssessment {
        let limited = if files.len() > self.config.max_files {
            &files[..self.config.max_files]
        } else {
            files
        };

        let file_risks: Vec<FileRisk> = limited.iter().map(|f| self.file_risk(f)).collect();

        // Overall score = weighted average biased toward max
        let max_score = file_risks
            .iter()
            .map(|fr| fr.score)
            .fold(0.0_f64, f64::max);
        let avg_score = if file_risks.is_empty() {
            0.0
        } else {
            file_risks.iter().map(|fr| fr.score).sum::<f64>() / file_risks.len() as f64
        };
        // 70% max, 30% avg — heavily penalise any single risky file
        let overall_score = (0.7 * max_score + 0.3 * avg_score).min(1.0);

        let mut impact_set: HashSet<ImpactArea> = HashSet::new();
        for fr in &file_risks {
            for area in &fr.impacted_areas {
                impact_set.insert(*area);
            }
        }
        let impact_areas: Vec<ImpactArea> = impact_set.into_iter().collect();

        let suggested_reviewers = Self::suggest_reviewers(&file_risks, &impact_areas);

        let test_suggestions = if self.config.include_test_suggestions {
            limited
                .iter()
                .flat_map(TestSuggester::suggest)
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        let overall_risk = RiskLevel::from_score(overall_score);
        let summary = self.build_summary(&file_risks, overall_risk, overall_score, limited.len());

        RiskAssessment {
            overall_risk,
            risk_score: overall_score,
            file_risks,
            impact_areas,
            suggested_reviewers,
            test_suggestions,
            summary,
        }
    }

    /// Compute risk for a single file.
    pub fn file_risk(&self, file: &DiffFile) -> FileRisk {
        let (mut score, mut reasons) =
            RiskScorer::score_file(&file.path, file.additions, file.deletions);

        // Bonus risk for deleted files (removing functionality)
        if file.change_type == ChangeType::Deleted {
            score = (score + 0.15).min(1.0);
            reasons.push("File deleted — possible functionality removal".into());
        }

        // Bonus risk for many hunks (scattered changes)
        if file.hunks.len() > 5 {
            score = (score + 0.1).min(1.0);
            reasons.push(format!("Scattered changes across {} hunks", file.hunks.len()));
        }

        let impacted_areas = Self::detect_impact_areas(&file.path, file);

        FileRisk {
            path: file.path.clone(),
            risk: RiskLevel::from_score(score),
            score,
            reasons,
            impacted_areas,
        }
    }

    /// Detect regression signals across the diff.
    pub fn detect_regressions(&self, files: &[DiffFile]) -> Vec<RegressionSignal> {
        let mut signals = Vec::new();

        for file in files {
            let lower = file.path.to_lowercase();

            // Deleted test files
            if file.change_type == ChangeType::Deleted
                && (lower.contains("test") || lower.contains("spec"))
            {
                signals.push(RegressionSignal {
                    file_path: file.path.clone(),
                    signal_type: "test_removed".into(),
                    description: "Test file was deleted — may reduce coverage".into(),
                    confidence: 0.9,
                });
            }

            // Removed assertions / test lines
            for hunk in &file.hunks {
                let removed_assertions = hunk
                    .content
                    .lines()
                    .filter(|l| l.starts_with('-'))
                    .filter(|l| {
                        l.contains("assert") || l.contains("expect(") || l.contains("#[test]")
                            || l.contains("it(\"") || l.contains("test(\"")
                    })
                    .count();
                if removed_assertions > 0 {
                    signals.push(RegressionSignal {
                        file_path: file.path.clone(),
                        signal_type: "assertion_removed".into(),
                        description: format!(
                            "{removed_assertions} assertion(s) removed in hunk at line {}",
                            hunk.start_line
                        ),
                        confidence: 0.85,
                    });
                }

                // Removed error handling
                let removed_error_handling = hunk
                    .content
                    .lines()
                    .filter(|l| l.starts_with('-'))
                    .filter(|l| {
                        l.contains("catch") || l.contains("Err(") || l.contains("try {")
                            || l.contains("rescue") || l.contains("except")
                    })
                    .count();
                if removed_error_handling > 0 {
                    signals.push(RegressionSignal {
                        file_path: file.path.clone(),
                        signal_type: "error_handling_removed".into(),
                        description: format!(
                            "{removed_error_handling} error-handling statement(s) removed"
                        ),
                        confidence: 0.75,
                    });
                }

                // Unsafe unwrap added
                let added_unwraps = hunk
                    .content
                    .lines()
                    .filter(|l| l.starts_with('+'))
                    .filter(|l| l.contains(".unwrap()"))
                    .count();
                if added_unwraps > 0 {
                    signals.push(RegressionSignal {
                        file_path: file.path.clone(),
                        signal_type: "unsafe_unwrap_added".into(),
                        description: format!(
                            "{added_unwraps} .unwrap() call(s) added — potential panic"
                        ),
                        confidence: 0.7,
                    });
                }

                // TODO/FIXME/HACK added
                let added_todos = hunk
                    .content
                    .lines()
                    .filter(|l| l.starts_with('+'))
                    .filter(|l| {
                        let upper = l.to_uppercase();
                        upper.contains("TODO") || upper.contains("FIXME") || upper.contains("HACK")
                    })
                    .count();
                if added_todos > 0 {
                    signals.push(RegressionSignal {
                        file_path: file.path.clone(),
                        signal_type: "tech_debt_added".into(),
                        description: format!(
                            "{added_todos} TODO/FIXME/HACK comment(s) added"
                        ),
                        confidence: 0.5,
                    });
                }
            }

            // Large net deletion in non-test file
            if !lower.contains("test") && file.deletions > file.additions + 50 {
                signals.push(RegressionSignal {
                    file_path: file.path.clone(),
                    signal_type: "large_deletion".into(),
                    description: format!(
                        "Net deletion of {} lines — possible functionality loss",
                        file.deletions - file.additions
                    ),
                    confidence: 0.6,
                });
            }
        }

        signals
    }

    /// Suggest tests based on all files in the diff.
    pub fn suggest_tests(&self, files: &[DiffFile]) -> Vec<String> {
        let mut all: Vec<String> = files
            .iter()
            .flat_map(TestSuggester::suggest)
            .collect();
        all.sort();
        all.dedup();
        all
    }

    /// Compute aggregate diff statistics.
    pub fn stats(&self, files: &[DiffFile]) -> DiffStats {
        let mut lang_set: HashSet<String> = HashSet::new();
        let mut total_additions = 0;
        let mut total_deletions = 0;
        let mut total_hunks = 0;

        for f in files {
            total_additions += f.additions;
            total_deletions += f.deletions;
            total_hunks += f.hunks.len();
            if let Some(lang) = detect_language(&f.path) {
                lang_set.insert(lang);
            }
        }

        let mut languages: Vec<String> = lang_set.into_iter().collect();
        languages.sort();

        DiffStats {
            files_changed: files.len(),
            total_additions,
            total_deletions,
            total_hunks,
            languages,
        }
    }

    /// Generate a human-readable summary of a risk assessment.
    pub fn summarize(&self, assessment: &RiskAssessment) -> String {
        self.build_summary(
            &assessment.file_risks,
            assessment.overall_risk,
            assessment.risk_score,
            assessment.file_risks.len(),
        )
    }

    // -- internal helpers --

    fn build_summary(
        &self,
        file_risks: &[FileRisk],
        overall_risk: RiskLevel,
        score: f64,
        file_count: usize,
    ) -> String {
        let high_risk_count = file_risks
            .iter()
            .filter(|fr| matches!(fr.risk, RiskLevel::Critical | RiskLevel::High))
            .count();

        let mut s = format!(
            "Overall risk: {} (score {:.2}) across {file_count} file(s).",
            overall_risk.label(),
            score,
        );

        if high_risk_count > 0 {
            s.push_str(&format!(
                " {high_risk_count} file(s) flagged as high/critical risk."
            ));
        }

        for fr in file_risks.iter().filter(|fr| fr.score >= self.config.risk_threshold) {
            s.push_str(&format!("\n  - {} ({:.2}): {}", fr.path, fr.score, fr.reasons.join("; ")));
        }

        s
    }

    fn suggest_reviewers(
        file_risks: &[FileRisk],
        impact_areas: &[ImpactArea],
    ) -> Vec<String> {
        let mut reviewers: Vec<String> = Vec::new();

        for area in impact_areas {
            match area {
                ImpactArea::Security => reviewers.push("security-team".into()),
                ImpactArea::Database => reviewers.push("dba-team".into()),
                ImpactArea::PublicAPI => reviewers.push("api-team".into()),
                ImpactArea::Dependencies => reviewers.push("platform-team".into()),
                ImpactArea::Build => reviewers.push("devops-team".into()),
                _ => {}
            }
        }

        let has_critical = file_risks
            .iter()
            .any(|fr| matches!(fr.risk, RiskLevel::Critical));
        if has_critical {
            reviewers.push("tech-lead".into());
        }

        reviewers.sort();
        reviewers.dedup();
        reviewers
    }

    fn detect_impact_areas(path: &str, file: &DiffFile) -> Vec<ImpactArea> {
        let mut areas = Vec::new();
        let lower = path.to_lowercase();

        let security_kw = ["auth", "crypto", "secret", "password", "token", "credential",
            "login", "session", "permission", "key"];
        if security_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Security);
        }

        let api_kw = ["api/", "routes/", "handler", "controller", "endpoint",
            "openapi", "swagger", "graphql", "proto"];
        if api_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::PublicAPI);
        }

        let db_kw = ["migration", "schema", ".sql", "alembic"];
        if db_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Database);
        }

        let config_kw = [".env", "config.", "dockerfile", "docker-compose", ".tf", "nginx"];
        if config_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Configuration);
        }

        if lower.contains("test") || lower.contains("spec") {
            areas.push(ImpactArea::Tests);
        }

        let doc_kw = [".md", "readme", "changelog", "docs/", "doc/"];
        if doc_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Documentation);
        }

        let build_kw = ["makefile", "cmake", "build.rs", "build.gradle",
            ".github/workflows", "ci/", "jenkinsfile"];
        if build_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Build);
        }

        let dep_kw = ["cargo.toml", "package.json", "go.mod", "requirements.txt",
            "gemfile", "pom.xml", "cargo.lock", "yarn.lock", "package-lock"];
        if dep_kw.iter().any(|k| lower.contains(k)) {
            areas.push(ImpactArea::Dependencies);
        }

        // Also scan hunk content for inline signals
        let _ = file; // file already used via path
        areas.sort_by_key(|a| a.label());
        areas.dedup();
        areas
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_hunk_header(line: &str) -> (usize, usize) {
    // "@@ -old_start,old_len +new_start,new_len @@"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in &parts {
        if part.starts_with('+') && part.contains(',') {
            let nums: Vec<&str> = part.trim_start_matches('+').split(',').collect();
            if nums.len() == 2 {
                let start = nums[0].parse::<usize>().unwrap_or(1);
                let len = nums[1].parse::<usize>().unwrap_or(1);
                return (start, start + len.saturating_sub(1));
            }
        } else if part.starts_with('+') {
            let start = part.trim_start_matches('+').parse::<usize>().unwrap_or(1);
            return (start, start);
        }
    }
    (1, 1)
}

fn detect_language(path: &str) -> Option<String> {
    let ext_map: HashMap<&str, &str> = [
        ("rs", "Rust"), ("ts", "TypeScript"), ("tsx", "TypeScript"),
        ("js", "JavaScript"), ("jsx", "JavaScript"), ("py", "Python"),
        ("go", "Go"), ("java", "Java"), ("rb", "Ruby"), ("cpp", "C++"),
        ("c", "C"), ("cs", "C#"), ("swift", "Swift"), ("kt", "Kotlin"),
        ("sql", "SQL"), ("sh", "Shell"), ("toml", "TOML"), ("yaml", "YAML"),
        ("yml", "YAML"), ("json", "JSON"), ("html", "HTML"), ("css", "CSS"),
        ("scss", "SCSS"), ("vue", "Vue"), ("svelte", "Svelte"),
        ("proto", "Protobuf"), ("tf", "Terraform"), ("md", "Markdown"),
    ]
    .into_iter()
    .collect();

    let ext = path.rsplit('.').next()?;
    ext_map.get(ext).map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_diff() -> &'static str {
        "\
diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -10,6 +10,8 @@ fn verify_token
 fn verify_token(token: &str) -> bool {
     let decoded = decode(token);
+    if decoded.is_none() {
+        return false;
+    }
     decoded.unwrap()
 }
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,6 @@
 fn main() {
+    println!(\"hello\");
+    println!(\"world\");
 }
diff --git a/tests/auth_test.rs b/tests/auth_test.rs
deleted file mode 100644
--- a/tests/auth_test.rs
+++ /dev/null
@@ -1,5 +0,0 @@
-#[test]
-fn test_verify() {
-    assert!(verify_token(\"valid\"));
-}
"
    }

    fn large_diff() -> &'static str {
        "\
diff --git a/api/routes/users.rs b/api/routes/users.rs
new file mode 100644
--- /dev/null
+++ b/api/routes/users.rs
@@ -0,0 +1,10 @@
+pub fn create_user() {}
+pub fn delete_user() {}
+pub fn update_user() {}
+pub fn get_user() {}
+pub fn list_users() {}
+pub fn ban_user() {}
+pub fn unban_user() {}
+pub fn reset_password() {}
+pub fn change_email() {}
+pub fn verify_email() {}
"
    }

    // -- RiskLevel --

    #[test]
    fn test_risk_level_from_score_critical() {
        assert_eq!(RiskLevel::from_score(0.95), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_score(0.8), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_from_score_high() {
        assert_eq!(RiskLevel::from_score(0.7), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(0.6), RiskLevel::High);
    }

    #[test]
    fn test_risk_level_from_score_medium() {
        assert_eq!(RiskLevel::from_score(0.5), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(0.4), RiskLevel::Medium);
    }

    #[test]
    fn test_risk_level_from_score_low() {
        assert_eq!(RiskLevel::from_score(0.3), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(0.2), RiskLevel::Low);
    }

    #[test]
    fn test_risk_level_from_score_safe() {
        assert_eq!(RiskLevel::from_score(0.1), RiskLevel::Safe);
        assert_eq!(RiskLevel::from_score(0.0), RiskLevel::Safe);
    }

    #[test]
    fn test_risk_level_label() {
        assert_eq!(RiskLevel::Critical.label(), "CRITICAL");
        assert_eq!(RiskLevel::Safe.label(), "SAFE");
    }

    // -- RiskScorer --

    #[test]
    fn test_risk_scorer_security_file() {
        let (score, reasons) = RiskScorer::score_file("src/auth.rs", 10, 5);
        assert!(score >= 0.3, "Security file should score >= 0.3");
        assert!(reasons.iter().any(|r| r.contains("Security")));
    }

    #[test]
    fn test_risk_scorer_api_file() {
        let (score, reasons) = RiskScorer::score_file("api/routes/users.rs", 10, 5);
        assert!(score >= 0.2);
        assert!(reasons.iter().any(|r| r.contains("API")));
    }

    #[test]
    fn test_risk_scorer_database_migration() {
        let (score, reasons) = RiskScorer::score_file("migrations/001_create_users.sql", 20, 0);
        assert!(score >= 0.3);
        assert!(reasons.iter().any(|r| r.contains("Database")));
    }

    #[test]
    fn test_risk_scorer_large_change() {
        let (score, reasons) = RiskScorer::score_file("src/lib.rs", 400, 200);
        assert!(score >= 0.2);
        assert!(reasons.iter().any(|r| r.contains("Large change")));
    }

    #[test]
    fn test_risk_scorer_test_deletion() {
        let (score, reasons) = RiskScorer::score_file("tests/unit_test.rs", 5, 100);
        assert!(score >= 0.3);
        assert!(reasons.iter().any(|r| r.contains("Test deletions")));
    }

    #[test]
    fn test_risk_scorer_dependency_change() {
        let (score, reasons) = RiskScorer::score_file("Cargo.toml", 3, 1);
        assert!(score >= 0.15);
        assert!(reasons.iter().any(|r| r.contains("Dependency")));
    }

    #[test]
    fn test_risk_scorer_safe_file() {
        let (score, reasons) = RiskScorer::score_file("src/utils.rs", 5, 2);
        assert!(score < 0.2);
        assert!(reasons.iter().any(|r| r.contains("No specific risk")));
    }

    #[test]
    fn test_risk_scorer_score_clamped_to_one() {
        // auth + migration + large + dependency-ish path
        let (score, _) =
            RiskScorer::score_file("auth_migration_schema.toml", 600, 600);
        assert!(score <= 1.0);
    }

    // -- DiffAnalyzer::parse_diff --

    #[test]
    fn test_parse_diff_file_count() {
        let files = DiffAnalyzer::parse_diff(sample_diff());
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_parse_diff_paths() {
        let files = DiffAnalyzer::parse_diff(sample_diff());
        assert_eq!(files[0].path, "src/auth.rs");
        assert_eq!(files[1].path, "src/main.rs");
        assert_eq!(files[2].path, "tests/auth_test.rs");
    }

    #[test]
    fn test_parse_diff_change_types() {
        let files = DiffAnalyzer::parse_diff(sample_diff());
        assert_eq!(files[0].change_type, ChangeType::Modified);
        assert_eq!(files[2].change_type, ChangeType::Deleted);
    }

    #[test]
    fn test_parse_diff_additions_deletions() {
        let files = DiffAnalyzer::parse_diff(sample_diff());
        // auth.rs: +3 additions
        assert_eq!(files[0].additions, 3);
        // main.rs: +2 additions
        assert_eq!(files[1].additions, 2);
    }

    #[test]
    fn test_parse_diff_new_file() {
        let files = DiffAnalyzer::parse_diff(large_diff());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].change_type, ChangeType::Added);
        assert_eq!(files[0].additions, 10);
    }

    #[test]
    fn test_parse_diff_hunks() {
        let files = DiffAnalyzer::parse_diff(sample_diff());
        assert!(!files[0].hunks.is_empty());
    }

    #[test]
    fn test_parse_diff_empty() {
        let files = DiffAnalyzer::parse_diff("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_diff_rename() {
        let diff = "\
diff --git a/old.rs b/new.rs
similarity index 95%
rename from old.rs
rename to new.rs
@@ -1,3 +1,3 @@
 fn foo() {
-    old();
+    new();
 }
";
        let files = DiffAnalyzer::parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].change_type, ChangeType::Renamed);
        assert_eq!(files[0].path, "new.rs");
    }

    // -- DiffAnalyzer::analyze --

    #[test]
    fn test_analyze_overall_risk() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let assessment = analyzer.analyze(&files);
        // auth.rs is security-sensitive → should be at least Medium
        assert!(assessment.risk_score >= 0.2);
    }

    #[test]
    fn test_analyze_impact_areas() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let assessment = analyzer.analyze(&files);
        assert!(assessment.impact_areas.contains(&ImpactArea::Security));
        assert!(assessment.impact_areas.contains(&ImpactArea::Tests));
    }

    #[test]
    fn test_analyze_suggested_reviewers() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let assessment = analyzer.analyze(&files);
        assert!(assessment.suggested_reviewers.contains(&"security-team".to_string()));
    }

    #[test]
    fn test_analyze_max_files_limit() {
        let analyzer = DiffAnalyzer::new(ReviewConfig {
            max_files: 1,
            ..Default::default()
        });
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let assessment = analyzer.analyze(&files);
        assert_eq!(assessment.file_risks.len(), 1);
    }

    // -- detect_regressions --

    #[test]
    fn test_detect_regressions_deleted_test() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let regressions = analyzer.detect_regressions(&files);
        assert!(
            regressions.iter().any(|r| r.signal_type == "test_removed"),
            "Should detect deleted test file"
        );
    }

    #[test]
    fn test_detect_regressions_removed_assertion() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let regressions = analyzer.detect_regressions(&files);
        assert!(
            regressions.iter().any(|r| r.signal_type == "assertion_removed"),
            "Should detect removed assertions"
        );
    }

    #[test]
    fn test_detect_regressions_unwrap_added() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
@@ -1,3 +1,4 @@
 fn process() {
+    let val = data.unwrap();
     println!(\"done\");
 }
";
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(diff);
        let regressions = analyzer.detect_regressions(&files);
        assert!(regressions.iter().any(|r| r.signal_type == "unsafe_unwrap_added"));
    }

    #[test]
    fn test_detect_regressions_tech_debt() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
@@ -1,3 +1,4 @@
 fn process() {
+    // TODO: fix this hack
     println!(\"done\");
 }
";
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(diff);
        let regressions = analyzer.detect_regressions(&files);
        assert!(regressions.iter().any(|r| r.signal_type == "tech_debt_added"));
    }

    #[test]
    fn test_detect_regressions_error_handling_removed() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
@@ -1,5 +1,3 @@
 fn process() -> Result<()> {
-    let val = try { fetch() };
-    match val { Err(e) => return Err(e), Ok(v) => v }
+    fetch()
 }
";
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(diff);
        let regressions = analyzer.detect_regressions(&files);
        assert!(regressions.iter().any(|r| r.signal_type == "error_handling_removed"));
    }

    #[test]
    fn test_detect_regressions_large_deletion() {
        let file = DiffFile {
            path: "src/core.rs".into(),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 100,
            hunks: vec![],
        };
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let regressions = analyzer.detect_regressions(&[file]);
        assert!(regressions.iter().any(|r| r.signal_type == "large_deletion"));
    }

    // -- suggest_tests --

    #[test]
    fn test_suggest_tests_new_api() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(large_diff());
        let suggestions = analyzer.suggest_tests(&files);
        assert!(
            suggestions.iter().any(|s| s.contains("integration test")),
            "API file should suggest integration test"
        );
    }

    #[test]
    fn test_suggest_tests_new_function() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(large_diff());
        let suggestions = analyzer.suggest_tests(&files);
        assert!(
            suggestions.iter().any(|s| s.contains("unit test")),
            "New function should suggest unit test"
        );
    }

    #[test]
    fn test_suggest_tests_ui_component() {
        let file = DiffFile {
            path: "src/components/Button.tsx".into(),
            change_type: ChangeType::Modified,
            additions: 5,
            deletions: 2,
            hunks: vec![],
        };
        let suggestions = TestSuggester::suggest(&file);
        assert!(suggestions.iter().any(|s| s.contains("snapshot test")));
    }

    #[test]
    fn test_suggest_tests_config_file() {
        let file = DiffFile {
            path: "config/app.yaml".into(),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 1,
            hunks: vec![],
        };
        let suggestions = TestSuggester::suggest(&file);
        assert!(suggestions.iter().any(|s| s.contains("config validation")));
    }

    #[test]
    fn test_suggest_tests_migration() {
        let file = DiffFile {
            path: "db/migration/001.sql".into(),
            change_type: ChangeType::Added,
            additions: 10,
            deletions: 0,
            hunks: vec![],
        };
        let suggestions = TestSuggester::suggest(&file);
        assert!(suggestions.iter().any(|s| s.contains("migration test")));
    }

    #[test]
    fn test_suggest_tests_disabled() {
        let analyzer = DiffAnalyzer::new(ReviewConfig {
            include_test_suggestions: false,
            ..Default::default()
        });
        let files = DiffAnalyzer::parse_diff(large_diff());
        let assessment = analyzer.analyze(&files);
        assert!(assessment.test_suggestions.is_empty());
    }

    // -- stats --

    #[test]
    fn test_stats_basic() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let stats = analyzer.stats(&files);
        assert_eq!(stats.files_changed, 3);
        assert!(stats.total_additions > 0);
        assert!(stats.languages.contains(&"Rust".to_string()));
    }

    #[test]
    fn test_stats_empty() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let stats = analyzer.stats(&[]);
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.total_additions, 0);
        assert!(stats.languages.is_empty());
    }

    // -- summarize --

    #[test]
    fn test_summarize_contains_risk_label() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let files = DiffAnalyzer::parse_diff(sample_diff());
        let assessment = analyzer.analyze(&files);
        let summary = analyzer.summarize(&assessment);
        assert!(summary.contains("Overall risk:"));
        assert!(summary.contains("file(s)"));
    }

    // -- detect_impact_areas --

    #[test]
    fn test_impact_areas_security() {
        let file = DiffFile {
            path: "src/auth/login.rs".into(),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
        };
        let areas = DiffAnalyzer::detect_impact_areas(&file.path, &file);
        assert!(areas.contains(&ImpactArea::Security));
    }

    #[test]
    fn test_impact_areas_build() {
        let file = DiffFile {
            path: ".github/workflows/ci.yml".into(),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 1,
            hunks: vec![],
        };
        let areas = DiffAnalyzer::detect_impact_areas(&file.path, &file);
        assert!(areas.contains(&ImpactArea::Build));
    }

    #[test]
    fn test_impact_areas_docs() {
        let file = DiffFile {
            path: "docs/README.md".into(),
            change_type: ChangeType::Modified,
            additions: 5,
            deletions: 0,
            hunks: vec![],
        };
        let areas = DiffAnalyzer::detect_impact_areas(&file.path, &file);
        assert!(areas.contains(&ImpactArea::Documentation));
    }

    // -- detect_language --

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(detect_language("src/main.rs"), Some("Rust".into()));
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language("app/index.tsx"), Some("TypeScript".into()));
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(detect_language("Makefile"), None);
    }

    // -- parse_hunk_header --

    #[test]
    fn test_parse_hunk_header_standard() {
        let (start, end) = parse_hunk_header("@@ -10,6 +10,8 @@ fn verify_token");
        assert_eq!(start, 10);
        assert_eq!(end, 17);
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let (start, _end) = parse_hunk_header("@@ -1 +1 @@");
        assert_eq!(start, 1);
    }

    // -- ReviewConfig default --

    #[test]
    fn test_review_config_defaults() {
        let cfg = ReviewConfig::default();
        assert!((cfg.risk_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(cfg.max_files, 100);
        assert!(cfg.include_test_suggestions);
    }

    // -- file_risk scattered hunks bonus --

    #[test]
    fn test_file_risk_scattered_hunks_bonus() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let file = DiffFile {
            path: "src/utils.rs".into(),
            change_type: ChangeType::Modified,
            additions: 5,
            deletions: 2,
            hunks: (0..7)
                .map(|i| DiffHunk {
                    start_line: i * 10,
                    end_line: i * 10 + 5,
                    content: "+line\n".into(),
                    context: String::new(),
                })
                .collect(),
        };
        let risk = analyzer.file_risk(&file);
        assert!(risk.score >= 0.1, "Scattered hunks should add risk");
        assert!(risk.reasons.iter().any(|r| r.contains("Scattered")));
    }

    // -- file_risk deleted file bonus --

    #[test]
    fn test_file_risk_deleted_file_bonus() {
        let analyzer = DiffAnalyzer::new(ReviewConfig::default());
        let file = DiffFile {
            path: "src/utils.rs".into(),
            change_type: ChangeType::Deleted,
            additions: 0,
            deletions: 10,
            hunks: vec![],
        };
        let risk = analyzer.file_risk(&file);
        assert!(risk.reasons.iter().any(|r| r.contains("deleted")));
    }

    // -- extract_fn_name --

    #[test]
    fn test_extract_fn_name_basic() {
        assert_eq!(extract_fn_name("pub fn hello(x: i32)"), Some("hello".into()));
        assert_eq!(extract_fn_name("fn bar()"), Some("bar".into()));
        assert_eq!(extract_fn_name("pub async fn fetch_data()"), Some("fetch_data".into()));
    }

    // -- extract_js_fn_name --

    #[test]
    fn test_extract_js_fn_name_function() {
        assert_eq!(extract_js_fn_name("export function greet()"), Some("greet".into()));
        assert_eq!(extract_js_fn_name("function inner()"), Some("inner".into()));
    }

    #[test]
    fn test_extract_js_fn_name_const() {
        assert_eq!(extract_js_fn_name("export const handler = () =>"), Some("handler".into()));
    }
}
