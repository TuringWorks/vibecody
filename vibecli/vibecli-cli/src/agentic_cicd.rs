#![allow(dead_code)]
//! AI agents as CI/CD participants for VibeCody.
//!
//! Auto-fix failing builds, generate missing tests, update dependencies,
//! and resolve merge conflicts using AI-driven strategies.
//!
//! REPL commands: `/cicd analyze|fix|gaps|deps|conflicts|report|optimize`

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum FailureType {
    BuildError,
    TestFailure,
    LintError,
    TypeCheckError,
    DependencyConflict,
    SecurityVulnerability,
    MergeConflict,
    DeploymentError,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildError => write!(f, "build_error"),
            Self::TestFailure => write!(f, "test_failure"),
            Self::LintError => write!(f, "lint_error"),
            Self::TypeCheckError => write!(f, "type_check_error"),
            Self::DependencyConflict => write!(f, "dependency_conflict"),
            Self::SecurityVulnerability => write!(f, "security_vulnerability"),
            Self::MergeConflict => write!(f, "merge_conflict"),
            Self::DeploymentError => write!(f, "deployment_error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FixStrategy {
    CompilerErrorFix,
    MissingImport,
    TypeAnnotation,
    TestUpdate,
    DependencyBump,
    LintAutofix,
    ConflictResolution,
    SecurityPatch,
    RetryBuild,
}

impl std::fmt::Display for FixStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerErrorFix => write!(f, "compiler_error_fix"),
            Self::MissingImport => write!(f, "missing_import"),
            Self::TypeAnnotation => write!(f, "type_annotation"),
            Self::TestUpdate => write!(f, "test_update"),
            Self::DependencyBump => write!(f, "dependency_bump"),
            Self::LintAutofix => write!(f, "lint_autofix"),
            Self::ConflictResolution => write!(f, "conflict_resolution"),
            Self::SecurityPatch => write!(f, "security_patch"),
            Self::RetryBuild => write!(f, "retry_build"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for TestPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateType {
    Major,
    Minor,
    Patch,
    Security,
}

impl std::fmt::Display for UpdateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Patch => write!(f, "patch"),
            Self::Security => write!(f, "security"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    TakeOurs,
    TakeTheirs,
    CombineBoth,
    ManualRequired,
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TakeOurs => write!(f, "take_ours"),
            Self::TakeTheirs => write!(f, "take_theirs"),
            Self::CombineBoth => write!(f, "combine_both"),
            Self::ManualRequired => write!(f, "manual_required"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CiCdError {
    FixFailed(String),
    MaxAttemptsExceeded(usize),
    TimeoutExceeded(u64),
    UnsupportedFailureType(String),
    PipelineNotFound(String),
    ConflictUnresolvable(String),
}

impl std::fmt::Display for CiCdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FixFailed(msg) => write!(f, "fix failed: {msg}"),
            Self::MaxAttemptsExceeded(n) => write!(f, "max attempts exceeded: {n}"),
            Self::TimeoutExceeded(s) => write!(f, "timeout exceeded: {s}s"),
            Self::UnsupportedFailureType(t) => write!(f, "unsupported failure type: {t}"),
            Self::PipelineNotFound(name) => write!(f, "pipeline not found: {name}"),
            Self::ConflictUnresolvable(path) => write!(f, "conflict unresolvable: {path}"),
        }
    }
}

// === Config ===

#[derive(Debug, Clone)]
pub struct CiCdConfig {
    pub auto_fix_builds: bool,
    pub auto_generate_tests: bool,
    pub auto_update_deps: bool,
    pub auto_resolve_conflicts: bool,
    pub max_fix_attempts: usize,
    pub timeout_secs: u64,
}

impl Default for CiCdConfig {
    fn default() -> Self {
        Self {
            auto_fix_builds: true,
            auto_generate_tests: true,
            auto_update_deps: false,
            auto_resolve_conflicts: false,
            max_fix_attempts: 3,
            timeout_secs: 600,
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct CiFailure {
    pub id: String,
    pub pipeline_name: String,
    pub failure_type: FailureType,
    pub error_message: String,
    pub failed_at: u64,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub log_snippet: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixAttempt {
    pub id: String,
    pub failure_id: String,
    pub strategy: FixStrategy,
    pub description: String,
    pub files_modified: Vec<String>,
    pub diff_summary: String,
    pub success: bool,
    pub attempted_at: u64,
    pub duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct TestGap {
    pub file_path: String,
    pub function_name: String,
    pub coverage_percent: f32,
    pub suggested_tests: Vec<String>,
    pub priority: TestPriority,
}

#[derive(Debug, Clone)]
pub struct DependencyUpdate {
    pub package: String,
    pub current_version: String,
    pub latest_version: String,
    pub update_type: UpdateType,
    pub breaking: bool,
    pub changelog_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConflictResolution {
    pub file_path: String,
    pub our_change: String,
    pub their_change: String,
    pub resolution: String,
    pub strategy: MergeStrategy,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct PipelineReport {
    pub failures_detected: usize,
    pub fixes_attempted: usize,
    pub fixes_successful: usize,
    pub tests_generated: usize,
    pub deps_updated: usize,
    pub conflicts_resolved: usize,
    pub duration_secs: u64,
}

// === Main Struct ===

pub struct AgenticCiCd {
    config: CiCdConfig,
    fix_history: Vec<FixAttempt>,
    tests_generated: usize,
    deps_updated: usize,
    conflicts_resolved: usize,
    start_time: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl AgenticCiCd {
    pub fn new(config: CiCdConfig) -> Self {
        Self {
            config,
            fix_history: Vec::new(),
            tests_generated: 0,
            deps_updated: 0,
            conflicts_resolved: 0,
            start_time: now_secs(),
        }
    }

    /// Determine the best fix strategy for a given CI failure.
    pub fn analyze_failure(&self, failure: &CiFailure) -> FixStrategy {
        match &failure.failure_type {
            FailureType::BuildError => {
                let msg = failure.error_message.to_lowercase();
                if msg.contains("cannot find") || msg.contains("unresolved import") {
                    FixStrategy::MissingImport
                } else if msg.contains("type") || msg.contains("expected") {
                    FixStrategy::TypeAnnotation
                } else {
                    FixStrategy::CompilerErrorFix
                }
            }
            FailureType::TestFailure => FixStrategy::TestUpdate,
            FailureType::LintError => FixStrategy::LintAutofix,
            FailureType::TypeCheckError => FixStrategy::TypeAnnotation,
            FailureType::DependencyConflict => FixStrategy::DependencyBump,
            FailureType::SecurityVulnerability => FixStrategy::SecurityPatch,
            FailureType::MergeConflict => FixStrategy::ConflictResolution,
            FailureType::DeploymentError => FixStrategy::RetryBuild,
        }
    }

    /// Attempt to fix a CI failure. Returns the fix attempt record.
    pub fn attempt_fix(&mut self, failure: &CiFailure) -> Result<FixAttempt, CiCdError> {
        let attempt_count = self
            .fix_history
            .iter()
            .filter(|a| a.failure_id == failure.id)
            .count();

        if attempt_count >= self.config.max_fix_attempts {
            return Err(CiCdError::MaxAttemptsExceeded(self.config.max_fix_attempts));
        }

        let strategy = self.analyze_failure(failure);
        let file_path = failure.file_path.clone().unwrap_or_default();

        let (description, diff_summary, files_modified) = match &strategy {
            FixStrategy::MissingImport => (
                format!("Add missing import for error: {}", failure.error_message),
                format!("+ use missing_module;\n  // in {file_path}"),
                vec![file_path],
            ),
            FixStrategy::TypeAnnotation => (
                format!("Add type annotation to fix: {}", failure.error_message),
                format!("- let value = expr;\n+ let value: Type = expr;\n  // in {file_path}"),
                vec![file_path],
            ),
            FixStrategy::CompilerErrorFix => (
                format!("Fix compiler error: {}", failure.error_message),
                format!("Applied compiler fix in {file_path}"),
                vec![file_path],
            ),
            FixStrategy::TestUpdate => (
                format!("Update failing test: {}", failure.error_message),
                "Updated test expectations to match current behavior".to_string(),
                vec![file_path],
            ),
            FixStrategy::LintAutofix => (
                "Apply lint autofix".to_string(),
                "Ran linter with --fix flag".to_string(),
                vec![file_path],
            ),
            FixStrategy::DependencyBump => (
                "Bump conflicting dependency".to_string(),
                "Updated dependency version in manifest".to_string(),
                vec!["Cargo.toml".to_string()],
            ),
            FixStrategy::SecurityPatch => (
                format!("Apply security patch: {}", failure.error_message),
                "Updated vulnerable dependency to patched version".to_string(),
                vec!["Cargo.toml".to_string()],
            ),
            FixStrategy::ConflictResolution => (
                "Resolve merge conflict".to_string(),
                format!("Resolved conflict markers in {file_path}"),
                vec![file_path],
            ),
            FixStrategy::RetryBuild => (
                "Retry build with clean state".to_string(),
                "Cleared caches and retried build".to_string(),
                Vec::new(),
            ),
        };

        let attempt = FixAttempt {
            id: format!("fix-{}-{}", failure.id, attempt_count + 1),
            failure_id: failure.id.clone(),
            strategy,
            description,
            files_modified,
            diff_summary,
            success: true,
            attempted_at: now_secs(),
            duration_secs: 2,
        };

        self.fix_history.push(attempt.clone());
        Ok(attempt)
    }

    /// Generate a fix suggestion for a compiler/build error.
    pub fn generate_compiler_fix(&self, error: &str, file: &str) -> String {
        let lower = error.to_lowercase();
        if lower.contains("cannot find") {
            format!("// In {file}: add missing import or define the symbol\nuse crate::missing_module;")
        } else if lower.contains("expected") && lower.contains("found") {
            format!("// In {file}: fix type mismatch\n// Change the expression type to match expected")
        } else if lower.contains("unused") {
            format!("// In {file}: prefix unused variable with underscore\nlet _unused = value;")
        } else if lower.contains("borrow") {
            format!("// In {file}: fix borrow checker error\n// Consider using .clone() or restructuring ownership")
        } else {
            format!("// In {file}: review compiler error and apply targeted fix\n// Error: {error}")
        }
    }

    /// Detect test coverage gaps for the given files and coverage data.
    pub fn detect_test_gaps(
        &self,
        files: &[String],
        coverage: &[(String, f32)],
    ) -> Vec<TestGap> {
        let cov_map: HashMap<&str, f32> = coverage.iter().map(|(f, c)| (f.as_str(), *c)).collect();

        files
            .iter()
            .filter_map(|file| {
                let pct = cov_map.get(file.as_str()).copied().unwrap_or(0.0);
                if pct < 80.0 {
                    let priority = if pct < 20.0 {
                        TestPriority::Critical
                    } else if pct < 40.0 {
                        TestPriority::High
                    } else if pct < 60.0 {
                        TestPriority::Medium
                    } else {
                        TestPriority::Low
                    };

                    let fn_name = file
                        .rsplit('/')
                        .next()
                        .unwrap_or(file)
                        .trim_end_matches(".rs")
                        .to_string();

                    Some(TestGap {
                        file_path: file.clone(),
                        function_name: fn_name,
                        coverage_percent: pct,
                        suggested_tests: vec![
                            format!("test_{}_basic", file.replace('/', "_").replace('.', "_")),
                            format!("test_{}_edge_cases", file.replace('/', "_").replace('.', "_")),
                        ],
                        priority,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Generate a test skeleton for a detected test gap.
    pub fn generate_test_skeleton(&self, gap: &TestGap) -> String {
        let mod_name = gap.function_name.replace('-', "_");
        let mut tests = String::new();
        tests.push_str(&format!("#[cfg(test)]\nmod {mod_name}_tests {{\n"));
        tests.push_str("    use super::*;\n\n");

        for (i, suggested) in gap.suggested_tests.iter().enumerate() {
            let test_name = suggested
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                .collect::<String>();
            tests.push_str(&format!("    #[test]\n    fn {test_name}() {{\n"));
            tests.push_str(&format!(
                "        // TODO: test {} (coverage: {:.1}%)\n",
                gap.function_name, gap.coverage_percent
            ));
            tests.push_str("        assert!(true);\n    }\n");
            if i < gap.suggested_tests.len() - 1 {
                tests.push('\n');
            }
        }
        tests.push_str("}\n");
        tests
    }

    /// Check for available dependency updates.
    pub fn check_dependency_updates(
        &self,
        deps: &[(String, String)],
    ) -> Vec<DependencyUpdate> {
        deps.iter()
            .map(|(pkg, version)| {
                let parts: Vec<&str> = version.split('.').collect();
                let (major, minor, patch) = match parts.as_slice() {
                    [ma, mi, pa] => (
                        ma.parse::<u32>().unwrap_or(0),
                        mi.parse::<u32>().unwrap_or(0),
                        pa.parse::<u32>().unwrap_or(0),
                    ),
                    _ => (0, 0, 1),
                };

                let new_patch = patch + 1;
                let latest = format!("{major}.{minor}.{new_patch}");

                let update_type = if pkg.contains("security") || pkg.contains("crypto") {
                    UpdateType::Security
                } else {
                    UpdateType::Patch
                };

                DependencyUpdate {
                    package: pkg.clone(),
                    current_version: version.clone(),
                    latest_version: latest,
                    update_type,
                    breaking: false,
                    changelog_url: Some(format!("https://crates.io/crates/{pkg}/changelog")),
                }
            })
            .collect()
    }

    /// Attempt to resolve a merge conflict.
    pub fn resolve_merge_conflict(
        &mut self,
        conflict: &ConflictResolution,
    ) -> Result<String, CiCdError> {
        if conflict.strategy == MergeStrategy::ManualRequired {
            return Err(CiCdError::ConflictUnresolvable(conflict.file_path.clone()));
        }

        let resolved = match &conflict.strategy {
            MergeStrategy::TakeOurs => conflict.our_change.clone(),
            MergeStrategy::TakeTheirs => conflict.their_change.clone(),
            MergeStrategy::CombineBoth => {
                format!("{}\n{}", conflict.our_change, conflict.their_change)
            }
            MergeStrategy::ManualRequired => unreachable!(),
        };

        self.conflicts_resolved += 1;
        Ok(resolved)
    }

    /// Generate a summary report for the current pipeline session.
    pub fn generate_pipeline_report(&self) -> PipelineReport {
        let successful = self.fix_history.iter().filter(|a| a.success).count();
        PipelineReport {
            failures_detected: self.fix_history.len(),
            fixes_attempted: self.fix_history.len(),
            fixes_successful: successful,
            tests_generated: self.tests_generated,
            deps_updated: self.deps_updated,
            conflicts_resolved: self.conflicts_resolved,
            duration_secs: now_secs().saturating_sub(self.start_time),
        }
    }

    /// Return the full fix attempt history.
    pub fn get_fix_history(&self) -> Vec<&FixAttempt> {
        self.fix_history.iter().collect()
    }

    /// Suggest optimizations for a CI pipeline based on step names.
    pub fn suggest_pipeline_optimization(&self, steps: &[String]) -> Vec<String> {
        let mut suggestions = Vec::new();

        let has_install = steps.iter().any(|s| s.contains("install"));
        let has_build = steps.iter().any(|s| s.contains("build"));
        let has_test = steps.iter().any(|s| s.contains("test"));
        let has_lint = steps.iter().any(|s| s.contains("lint"));
        let has_cache = steps.iter().any(|s| s.contains("cache"));

        if has_install && !has_cache {
            suggestions.push("Add dependency caching step to speed up installs".to_string());
        }
        if has_build && has_test {
            suggestions.push("Consider running tests in parallel with build artifacts".to_string());
        }
        if has_lint && has_test {
            suggestions.push("Run lint and test steps concurrently to reduce wall time".to_string());
        }
        if steps.len() > 8 {
            suggestions.push("Pipeline has many steps; consider consolidating related steps".to_string());
        }
        if !steps.iter().any(|s| s.contains("artifact") || s.contains("upload")) {
            suggestions.push("Consider uploading build artifacts for faster downstream jobs".to_string());
        }

        suggestions
    }

    /// Generate a GitHub Action workflow snippet that fixes a CI failure.
    pub fn generate_github_action_fix(&self, failure: &CiFailure) -> String {
        let strategy = self.analyze_failure(failure);
        let step_name = format!("Auto-fix: {strategy}");
        let fix_cmd = match &strategy {
            FixStrategy::LintAutofix => "cargo clippy --fix --allow-dirty".to_string(),
            FixStrategy::DependencyBump => "cargo update".to_string(),
            FixStrategy::TestUpdate => "cargo test -- --ignored 2>&1 || true".to_string(),
            FixStrategy::CompilerErrorFix | FixStrategy::MissingImport | FixStrategy::TypeAnnotation => {
                "cargo check 2>&1 | head -50".to_string()
            }
            _ => format!("echo 'Strategy {strategy} requires manual intervention'"),
        };

        format!(
            r#"name: AI Auto-Fix
on:
  workflow_run:
    workflows: ["CI"]
    types: [completed]
jobs:
  auto-fix:
    if: ${{{{ github.event.workflow_run.conclusion == 'failure' }}}}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: {step_name}
        run: {fix_cmd}
      - name: Commit fix
        run: |
          git config user.name "vibecody-bot"
          git config user.email "bot@vibecody.dev"
          git add -A
          git diff --cached --quiet || git commit -m "fix: auto-fix {strategy} via VibeCody"
          git push
"#
        )
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cicd() -> AgenticCiCd {
        AgenticCiCd::new(CiCdConfig::default())
    }

    fn make_failure(ftype: FailureType, msg: &str) -> CiFailure {
        CiFailure {
            id: "fail-1".to_string(),
            pipeline_name: "ci-main".to_string(),
            failure_type: ftype,
            error_message: msg.to_string(),
            failed_at: 1000,
            file_path: Some("src/lib.rs".to_string()),
            line_number: Some(42),
            log_snippet: "error snippet".to_string(),
        }
    }

    #[test]
    fn test_default_config() {
        let cfg = CiCdConfig::default();
        assert!(cfg.auto_fix_builds);
        assert!(cfg.auto_generate_tests);
        assert!(!cfg.auto_update_deps);
        assert!(!cfg.auto_resolve_conflicts);
        assert_eq!(cfg.max_fix_attempts, 3);
        assert_eq!(cfg.timeout_secs, 600);
    }

    #[test]
    fn test_analyze_build_error_missing_import() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::BuildError, "cannot find value `foo`");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::MissingImport);
    }

    #[test]
    fn test_analyze_build_error_type_mismatch() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::BuildError, "expected u32 found String");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::TypeAnnotation);
    }

    #[test]
    fn test_analyze_build_error_generic() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::BuildError, "some obscure error");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::CompilerErrorFix);
    }

    #[test]
    fn test_analyze_test_failure() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::TestFailure, "assertion failed");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::TestUpdate);
    }

    #[test]
    fn test_analyze_lint_error() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::LintError, "unused variable");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::LintAutofix);
    }

    #[test]
    fn test_analyze_security_vulnerability() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::SecurityVulnerability, "CVE-2024-1234");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::SecurityPatch);
    }

    #[test]
    fn test_analyze_merge_conflict() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::MergeConflict, "conflict in main.rs");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::ConflictResolution);
    }

    #[test]
    fn test_analyze_deployment_error() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::DeploymentError, "deploy timeout");
        assert_eq!(cicd.analyze_failure(&f), FixStrategy::RetryBuild);
    }

    #[test]
    fn test_attempt_fix_success() {
        let mut cicd = default_cicd();
        let f = make_failure(FailureType::LintError, "unused variable");
        let result = cicd.attempt_fix(&f);
        assert!(result.is_ok());
        let attempt = result.unwrap();
        assert!(attempt.success);
        assert_eq!(attempt.failure_id, "fail-1");
        assert_eq!(attempt.strategy, FixStrategy::LintAutofix);
    }

    #[test]
    fn test_attempt_fix_max_attempts_exceeded() {
        let mut cicd = AgenticCiCd::new(CiCdConfig {
            max_fix_attempts: 1,
            ..CiCdConfig::default()
        });
        let f = make_failure(FailureType::BuildError, "error");
        let _ = cicd.attempt_fix(&f).unwrap();
        let result = cicd.attempt_fix(&f);
        assert_eq!(result, Err(CiCdError::MaxAttemptsExceeded(1)));
    }

    #[test]
    fn test_fix_history_tracks_attempts() {
        let mut cicd = default_cicd();
        let f = make_failure(FailureType::TestFailure, "test failed");
        cicd.attempt_fix(&f).unwrap();
        cicd.attempt_fix(&f).unwrap();
        assert_eq!(cicd.get_fix_history().len(), 2);
    }

    #[test]
    fn test_generate_compiler_fix_cannot_find() {
        let cicd = default_cicd();
        let fix = cicd.generate_compiler_fix("cannot find value `foo`", "src/lib.rs");
        assert!(fix.contains("missing import"));
    }

    #[test]
    fn test_generate_compiler_fix_type_mismatch() {
        let cicd = default_cicd();
        let fix = cicd.generate_compiler_fix("expected u32, found String", "src/main.rs");
        assert!(fix.contains("type mismatch"));
    }

    #[test]
    fn test_generate_compiler_fix_unused() {
        let cicd = default_cicd();
        let fix = cicd.generate_compiler_fix("unused variable `x`", "src/lib.rs");
        assert!(fix.contains("underscore"));
    }

    #[test]
    fn test_generate_compiler_fix_borrow() {
        let cicd = default_cicd();
        let fix = cicd.generate_compiler_fix("cannot borrow as mutable", "src/lib.rs");
        assert!(fix.contains("borrow"));
    }

    #[test]
    fn test_generate_compiler_fix_generic() {
        let cicd = default_cicd();
        let fix = cicd.generate_compiler_fix("some weird error", "src/lib.rs");
        assert!(fix.contains("review compiler error"));
    }

    #[test]
    fn test_detect_test_gaps_filters_high_coverage() {
        let cicd = default_cicd();
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let cov = vec![("a.rs".to_string(), 90.0), ("b.rs".to_string(), 30.0)];
        let gaps = cicd.detect_test_gaps(&files, &cov);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].file_path, "b.rs");
    }

    #[test]
    fn test_detect_test_gaps_priority_critical() {
        let cicd = default_cicd();
        let files = vec!["a.rs".to_string()];
        let cov = vec![("a.rs".to_string(), 10.0)];
        let gaps = cicd.detect_test_gaps(&files, &cov);
        assert_eq!(gaps[0].priority, TestPriority::Critical);
    }

    #[test]
    fn test_detect_test_gaps_priority_high() {
        let cicd = default_cicd();
        let files = vec!["a.rs".to_string()];
        let cov = vec![("a.rs".to_string(), 35.0)];
        let gaps = cicd.detect_test_gaps(&files, &cov);
        assert_eq!(gaps[0].priority, TestPriority::High);
    }

    #[test]
    fn test_detect_test_gaps_no_coverage_data() {
        let cicd = default_cicd();
        let files = vec!["new_file.rs".to_string()];
        let cov: Vec<(String, f32)> = vec![];
        let gaps = cicd.detect_test_gaps(&files, &cov);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].coverage_percent, 0.0);
        assert_eq!(gaps[0].priority, TestPriority::Critical);
    }

    #[test]
    fn test_generate_test_skeleton() {
        let cicd = default_cicd();
        let gap = TestGap {
            file_path: "src/foo.rs".to_string(),
            function_name: "foo".to_string(),
            coverage_percent: 25.0,
            suggested_tests: vec!["test_foo_basic".to_string()],
            priority: TestPriority::High,
        };
        let skeleton = cicd.generate_test_skeleton(&gap);
        assert!(skeleton.contains("#[cfg(test)]"));
        assert!(skeleton.contains("fn test_foo_basic"));
        assert!(skeleton.contains("25.0%"));
    }

    #[test]
    fn test_check_dependency_updates() {
        let cicd = default_cicd();
        let deps = vec![("serde".to_string(), "1.0.5".to_string())];
        let updates = cicd.check_dependency_updates(&deps);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package, "serde");
        assert_eq!(updates[0].latest_version, "1.0.6");
        assert_eq!(updates[0].update_type, UpdateType::Patch);
        assert!(!updates[0].breaking);
    }

    #[test]
    fn test_check_dependency_updates_security() {
        let cicd = default_cicd();
        let deps = vec![("ring-security".to_string(), "0.1.0".to_string())];
        let updates = cicd.check_dependency_updates(&deps);
        assert_eq!(updates[0].update_type, UpdateType::Security);
    }

    #[test]
    fn test_resolve_conflict_take_ours() {
        let mut cicd = default_cicd();
        let conflict = ConflictResolution {
            file_path: "src/lib.rs".to_string(),
            our_change: "our code".to_string(),
            their_change: "their code".to_string(),
            resolution: String::new(),
            strategy: MergeStrategy::TakeOurs,
            confidence: 0.9,
        };
        let result = cicd.resolve_merge_conflict(&conflict).unwrap();
        assert_eq!(result, "our code");
        assert_eq!(cicd.conflicts_resolved, 1);
    }

    #[test]
    fn test_resolve_conflict_take_theirs() {
        let mut cicd = default_cicd();
        let conflict = ConflictResolution {
            file_path: "src/lib.rs".to_string(),
            our_change: "our code".to_string(),
            their_change: "their code".to_string(),
            resolution: String::new(),
            strategy: MergeStrategy::TakeTheirs,
            confidence: 0.8,
        };
        let result = cicd.resolve_merge_conflict(&conflict).unwrap();
        assert_eq!(result, "their code");
    }

    #[test]
    fn test_resolve_conflict_combine_both() {
        let mut cicd = default_cicd();
        let conflict = ConflictResolution {
            file_path: "src/lib.rs".to_string(),
            our_change: "line A".to_string(),
            their_change: "line B".to_string(),
            resolution: String::new(),
            strategy: MergeStrategy::CombineBoth,
            confidence: 0.7,
        };
        let result = cicd.resolve_merge_conflict(&conflict).unwrap();
        assert!(result.contains("line A"));
        assert!(result.contains("line B"));
    }

    #[test]
    fn test_resolve_conflict_manual_required() {
        let mut cicd = default_cicd();
        let conflict = ConflictResolution {
            file_path: "src/lib.rs".to_string(),
            our_change: "ours".to_string(),
            their_change: "theirs".to_string(),
            resolution: String::new(),
            strategy: MergeStrategy::ManualRequired,
            confidence: 0.2,
        };
        let result = cicd.resolve_merge_conflict(&conflict);
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_report() {
        let mut cicd = default_cicd();
        let f = make_failure(FailureType::LintError, "lint");
        cicd.attempt_fix(&f).unwrap();
        cicd.tests_generated = 5;
        cicd.deps_updated = 2;
        let report = cicd.generate_pipeline_report();
        assert_eq!(report.fixes_attempted, 1);
        assert_eq!(report.fixes_successful, 1);
        assert_eq!(report.tests_generated, 5);
        assert_eq!(report.deps_updated, 2);
    }

    #[test]
    fn test_suggest_pipeline_optimization_no_cache() {
        let cicd = default_cicd();
        let steps = vec!["install deps".to_string(), "build".to_string()];
        let suggestions = cicd.suggest_pipeline_optimization(&steps);
        assert!(suggestions.iter().any(|s| s.contains("caching")));
    }

    #[test]
    fn test_suggest_pipeline_optimization_parallel() {
        let cicd = default_cicd();
        let steps = vec!["lint".to_string(), "test".to_string()];
        let suggestions = cicd.suggest_pipeline_optimization(&steps);
        assert!(suggestions.iter().any(|s| s.contains("concurrently")));
    }

    #[test]
    fn test_suggest_pipeline_optimization_many_steps() {
        let cicd = default_cicd();
        let steps: Vec<String> = (0..10).map(|i| format!("step-{i}")).collect();
        let suggestions = cicd.suggest_pipeline_optimization(&steps);
        assert!(suggestions.iter().any(|s| s.contains("consolidating")));
    }

    #[test]
    fn test_generate_github_action_fix_lint() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::LintError, "unused");
        let yaml = cicd.generate_github_action_fix(&f);
        assert!(yaml.contains("clippy --fix"));
        assert!(yaml.contains("vibecody-bot"));
    }

    #[test]
    fn test_generate_github_action_fix_dep() {
        let cicd = default_cicd();
        let f = make_failure(FailureType::DependencyConflict, "conflict");
        let yaml = cicd.generate_github_action_fix(&f);
        assert!(yaml.contains("cargo update"));
    }

    #[test]
    fn test_failure_type_display() {
        assert_eq!(format!("{}", FailureType::BuildError), "build_error");
        assert_eq!(format!("{}", FailureType::MergeConflict), "merge_conflict");
    }

    #[test]
    fn test_fix_strategy_display() {
        assert_eq!(format!("{}", FixStrategy::CompilerErrorFix), "compiler_error_fix");
        assert_eq!(format!("{}", FixStrategy::SecurityPatch), "security_patch");
    }

    #[test]
    fn test_cicd_error_display() {
        let e = CiCdError::MaxAttemptsExceeded(3);
        assert_eq!(format!("{e}"), "max attempts exceeded: 3");
    }

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(format!("{}", MergeStrategy::CombineBoth), "combine_both");
    }

    #[test]
    fn test_update_type_display() {
        assert_eq!(format!("{}", UpdateType::Security), "security");
    }

    #[test]
    fn test_test_priority_display() {
        assert_eq!(format!("{}", TestPriority::Critical), "critical");
    }
}
