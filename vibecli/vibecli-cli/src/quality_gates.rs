//! Quality gates for gating agent task completion.
//!
//! Claw-code parity Wave 3: enforces configurable pass/fail criteria (test
//! coverage, lint score, security findings) before marking a task as complete.

use serde::{Deserialize, Serialize};

// ─── Gate Criterion ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GateCriterion {
    /// All tests must pass (0 failures).
    TestsPass,
    /// Test coverage above threshold (0–100).
    CoverageAbove { min_pct: f64 },
    /// Zero clippy errors (warnings allowed).
    ClippyClean,
    /// No high/critical security findings.
    NoSecurityFindings,
    /// Compilation succeeds with no errors.
    Compiles,
    /// Number of lint warnings below limit.
    LintWarningsBelow { max: u32 },
    /// Custom named gate (evaluated externally).
    Custom { name: String },
}

impl std::fmt::Display for GateCriterion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TestsPass => write!(f, "tests_pass"),
            Self::CoverageAbove { min_pct } => write!(f, "coverage>={min_pct}%"),
            Self::ClippyClean => write!(f, "clippy_clean"),
            Self::NoSecurityFindings => write!(f, "no_security_findings"),
            Self::Compiles => write!(f, "compiles"),
            Self::LintWarningsBelow { max } => write!(f, "lint_warnings<={max}"),
            Self::Custom { name } => write!(f, "{name}"),
        }
    }
}

// ─── Gate Evaluation ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GateOutcome { Pass, Fail { reason: String }, Skipped }

impl GateOutcome {
    pub fn passed(&self) -> bool { matches!(self, Self::Pass) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub criterion: GateCriterion,
    pub outcome: GateOutcome,
}

// ─── Task Evidence ────────────────────────────────────────────────────────────

/// Observed metrics from running the task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskEvidence {
    pub tests_total: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub coverage_pct: f64,
    pub clippy_errors: u32,
    pub clippy_warnings: u32,
    pub security_high: u32,
    pub security_critical: u32,
    pub compilation_ok: bool,
    /// Custom key → bool (external gate results).
    pub custom_gates: std::collections::HashMap<String, bool>,
}

impl TaskEvidence {
    pub fn all_tests_pass(&self) -> bool { self.tests_failed == 0 && self.tests_total > 0 }
}

// ─── Quality Gate Evaluator ───────────────────────────────────────────────────

pub struct QualityGate {
    pub name: String,
    pub criteria: Vec<GateCriterion>,
    /// If true, any failure blocks the task; if false, failures are advisory.
    pub blocking: bool,
}

impl QualityGate {
    pub fn new(name: impl Into<String>, blocking: bool) -> Self {
        Self { name: name.into(), criteria: Vec::new(), blocking }
    }

    pub fn require(mut self, c: GateCriterion) -> Self { self.criteria.push(c); self }

    pub fn evaluate(&self, evidence: &TaskEvidence) -> Vec<GateResult> {
        self.criteria.iter().map(|criterion| {
            let outcome = match criterion {
                GateCriterion::TestsPass => {
                    if evidence.tests_total == 0 {
                        GateOutcome::Skipped
                    } else if evidence.all_tests_pass() {
                        GateOutcome::Pass
                    } else {
                        GateOutcome::Fail { reason: format!("{} tests failed", evidence.tests_failed) }
                    }
                }
                GateCriterion::CoverageAbove { min_pct } => {
                    if evidence.coverage_pct >= *min_pct {
                        GateOutcome::Pass
                    } else {
                        GateOutcome::Fail { reason: format!("coverage {:.1}% < {min_pct}%", evidence.coverage_pct) }
                    }
                }
                GateCriterion::ClippyClean => {
                    if evidence.clippy_errors == 0 {
                        GateOutcome::Pass
                    } else {
                        GateOutcome::Fail { reason: format!("{} clippy errors", evidence.clippy_errors) }
                    }
                }
                GateCriterion::NoSecurityFindings => {
                    if evidence.security_high + evidence.security_critical == 0 {
                        GateOutcome::Pass
                    } else {
                        GateOutcome::Fail { reason: format!("{} high/{} critical findings", evidence.security_high, evidence.security_critical) }
                    }
                }
                GateCriterion::Compiles => {
                    if evidence.compilation_ok { GateOutcome::Pass }
                    else { GateOutcome::Fail { reason: "compilation failed".into() } }
                }
                GateCriterion::LintWarningsBelow { max } => {
                    if evidence.clippy_warnings <= *max {
                        GateOutcome::Pass
                    } else {
                        GateOutcome::Fail { reason: format!("{} warnings > max {max}", evidence.clippy_warnings) }
                    }
                }
                GateCriterion::Custom { name } => {
                    match evidence.custom_gates.get(name) {
                        None => GateOutcome::Skipped,
                        Some(true) => GateOutcome::Pass,
                        Some(false) => GateOutcome::Fail { reason: format!("custom gate '{name}' failed") },
                    }
                }
            };
            GateResult { criterion: criterion.clone(), outcome }
        }).collect()
    }

    /// True if all blocking criteria pass (or skipped).
    pub fn allows_completion(&self, evidence: &TaskEvidence) -> bool {
        if !self.blocking { return true; }
        self.evaluate(evidence).iter().all(|r| r.outcome != GateOutcome::Fail { reason: String::new() } || !matches!(r.outcome, GateOutcome::Fail { .. }))
    }

    pub fn is_passing(&self, evidence: &TaskEvidence) -> bool {
        self.evaluate(evidence).iter().all(|r| r.outcome.passed() || matches!(r.outcome, GateOutcome::Skipped))
    }
}

/// Standard gate preset for Rust projects.
pub fn rust_project_gate() -> QualityGate {
    QualityGate::new("rust-project", true)
        .require(GateCriterion::Compiles)
        .require(GateCriterion::TestsPass)
        .require(GateCriterion::ClippyClean)
        .require(GateCriterion::CoverageAbove { min_pct: 70.0 })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_evidence() -> TaskEvidence {
        TaskEvidence {
            tests_total: 100, tests_passed: 100, tests_failed: 0,
            coverage_pct: 85.0, clippy_errors: 0, clippy_warnings: 2,
            security_high: 0, security_critical: 0, compilation_ok: true,
            custom_gates: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_tests_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::TestsPass);
        let results = gate.evaluate(&passing_evidence());
        assert_eq!(results[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_tests_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::TestsPass);
        let mut ev = passing_evidence();
        ev.tests_failed = 3;
        let results = gate.evaluate(&ev);
        assert!(matches!(results[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_tests_skipped_when_no_tests() {
        let gate = QualityGate::new("g", true).require(GateCriterion::TestsPass);
        let ev = TaskEvidence { tests_total: 0, ..Default::default() };
        let results = gate.evaluate(&ev);
        assert_eq!(results[0].outcome, GateOutcome::Skipped);
    }

    #[test]
    fn test_coverage_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::CoverageAbove { min_pct: 80.0 });
        let results = gate.evaluate(&passing_evidence());
        assert_eq!(results[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_coverage_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::CoverageAbove { min_pct: 90.0 });
        let results = gate.evaluate(&passing_evidence());
        assert!(matches!(results[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_clippy_clean_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::ClippyClean);
        let results = gate.evaluate(&passing_evidence());
        assert_eq!(results[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_clippy_clean_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::ClippyClean);
        let mut ev = passing_evidence();
        ev.clippy_errors = 2;
        let results = gate.evaluate(&ev);
        assert!(matches!(results[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_security_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::NoSecurityFindings);
        let results = gate.evaluate(&passing_evidence());
        assert_eq!(results[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_security_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::NoSecurityFindings);
        let mut ev = passing_evidence();
        ev.security_critical = 1;
        let results = gate.evaluate(&ev);
        assert!(matches!(results[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_compiles_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::Compiles);
        assert_eq!(gate.evaluate(&passing_evidence())[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_compiles_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::Compiles);
        let mut ev = passing_evidence();
        ev.compilation_ok = false;
        assert!(matches!(gate.evaluate(&ev)[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_lint_warnings_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::LintWarningsBelow { max: 5 });
        assert_eq!(gate.evaluate(&passing_evidence())[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_lint_warnings_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::LintWarningsBelow { max: 1 });
        assert!(matches!(gate.evaluate(&passing_evidence())[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_custom_gate_pass() {
        let gate = QualityGate::new("g", true).require(GateCriterion::Custom { name: "integration".into() });
        let mut ev = passing_evidence();
        ev.custom_gates.insert("integration".into(), true);
        assert_eq!(gate.evaluate(&ev)[0].outcome, GateOutcome::Pass);
    }

    #[test]
    fn test_custom_gate_fail() {
        let gate = QualityGate::new("g", true).require(GateCriterion::Custom { name: "integration".into() });
        let mut ev = passing_evidence();
        ev.custom_gates.insert("integration".into(), false);
        assert!(matches!(gate.evaluate(&ev)[0].outcome, GateOutcome::Fail { .. }));
    }

    #[test]
    fn test_custom_gate_skipped_when_absent() {
        let gate = QualityGate::new("g", true).require(GateCriterion::Custom { name: "missing".into() });
        assert_eq!(gate.evaluate(&passing_evidence())[0].outcome, GateOutcome::Skipped);
    }

    #[test]
    fn test_is_passing_all_pass() {
        let gate = rust_project_gate();
        assert!(gate.is_passing(&passing_evidence()));
    }

    #[test]
    fn test_is_passing_fails_on_test_failure() {
        let gate = rust_project_gate();
        let mut ev = passing_evidence();
        ev.tests_failed = 1;
        assert!(!gate.is_passing(&ev));
    }

    #[test]
    fn test_criterion_display() {
        assert_eq!(GateCriterion::TestsPass.to_string(), "tests_pass");
        assert_eq!(GateCriterion::CoverageAbove { min_pct: 80.0 }.to_string(), "coverage>=80%");
    }

    #[test]
    fn test_non_blocking_gate_always_allows() {
        let gate = QualityGate::new("advisory", false)
            .require(GateCriterion::CoverageAbove { min_pct: 99.0 });
        assert!(gate.allows_completion(&passing_evidence()));
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Green Contract — hierarchical merge preconditions
// ═════════════════════════════════════════════════════════════════════════════
//
// Quality levels form a strict order:
// `TargetedTests < Package < Workspace < MergeReady`
//
// Higher levels cumulatively satisfy all lower levels. A contract is
// evaluated against a set of `CheckResults` and returns a `GreenOutcome`.
//
// Integration: wire `GreenContract::evaluate()` into `agentic_cicd.rs` merge gates.

// ── QualityLevel ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityLevel {
    TargetedTests,
    Package,
    Workspace,
    MergeReady,
}

impl std::fmt::Display for QualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetedTests => write!(f, "targeted_tests"),
            Self::Package => write!(f, "package"),
            Self::Workspace => write!(f, "workspace"),
            Self::MergeReady => write!(f, "merge_ready"),
        }
    }
}

impl QualityLevel {
    /// Returns true if this level satisfies `lower` (i.e., this >= lower).
    pub fn satisfies(&self, lower: &QualityLevel) -> bool {
        self >= lower
    }
}

// ── GreenOutcome ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GreenOutcome {
    Pass,
    Fail(String),
}

impl GreenOutcome {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass) }
    pub fn reason(&self) -> Option<&str> {
        match self { Self::Fail(r) => Some(r), _ => None }
    }
}

impl std::fmt::Display for GreenOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Fail(r) => write!(f, "fail: {r}"),
        }
    }
}

// ── CheckResults ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CheckResults {
    pub tests_passed: bool,
    pub package_build: bool,
    pub workspace_build: bool,
    pub lint_clean: bool,
    pub merge_checks: bool,
}

impl CheckResults {
    pub fn all_passing() -> Self {
        Self {
            tests_passed: true,
            package_build: true,
            workspace_build: true,
            lint_clean: true,
            merge_checks: true,
        }
    }
}

// ── GreenContract ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GreenContract {
    pub level: QualityLevel,
    pub description: String,
}

impl GreenContract {
    pub fn new(level: QualityLevel) -> Self {
        let description = format!("Green contract at level: {level}");
        Self { level, description }
    }

    /// Evaluate the contract against actual check results.
    /// Higher levels cumulatively require all lower-level checks.
    pub fn evaluate(&self, results: &CheckResults) -> GreenOutcome {
        // TargetedTests: requires tests_passed
        if !results.tests_passed {
            return GreenOutcome::Fail("tests did not pass".to_string());
        }
        if self.level == QualityLevel::TargetedTests {
            return GreenOutcome::Pass;
        }

        // Package: additionally requires package_build
        if !results.package_build {
            return GreenOutcome::Fail("package build failed".to_string());
        }
        if self.level == QualityLevel::Package {
            return GreenOutcome::Pass;
        }

        // Workspace: additionally requires workspace_build + lint_clean
        if !results.workspace_build {
            return GreenOutcome::Fail("workspace build failed".to_string());
        }
        if !results.lint_clean {
            return GreenOutcome::Fail("lint checks failed".to_string());
        }
        if self.level == QualityLevel::Workspace {
            return GreenOutcome::Pass;
        }

        // MergeReady: additionally requires merge_checks
        if !results.merge_checks {
            return GreenOutcome::Fail("merge checks failed".to_string());
        }
        GreenOutcome::Pass
    }
}

// ── Green Contract Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod green_contract_tests {
    use super::*;

    #[test]
    fn quality_level_ordering() {
        assert!(QualityLevel::TargetedTests < QualityLevel::Package);
        assert!(QualityLevel::Package < QualityLevel::Workspace);
        assert!(QualityLevel::Workspace < QualityLevel::MergeReady);
    }

    #[test]
    fn higher_level_satisfies_lower() {
        assert!(QualityLevel::MergeReady.satisfies(&QualityLevel::TargetedTests));
        assert!(QualityLevel::MergeReady.satisfies(&QualityLevel::Package));
        assert!(QualityLevel::MergeReady.satisfies(&QualityLevel::Workspace));
        assert!(QualityLevel::Workspace.satisfies(&QualityLevel::Package));
    }

    #[test]
    fn same_level_satisfies_self() {
        assert!(QualityLevel::Package.satisfies(&QualityLevel::Package));
        assert!(QualityLevel::MergeReady.satisfies(&QualityLevel::MergeReady));
    }

    #[test]
    fn lower_does_not_satisfy_higher() {
        assert!(!QualityLevel::TargetedTests.satisfies(&QualityLevel::MergeReady));
        assert!(!QualityLevel::Package.satisfies(&QualityLevel::Workspace));
    }

    #[test]
    fn evaluate_pass_when_all_checks_met() {
        let contract = GreenContract::new(QualityLevel::MergeReady);
        let results = CheckResults::all_passing();
        assert_eq!(contract.evaluate(&results), GreenOutcome::Pass);
    }

    #[test]
    fn evaluate_fail_with_reason_when_tests_fail() {
        let contract = GreenContract::new(QualityLevel::TargetedTests);
        let results = CheckResults { tests_passed: false, ..Default::default() };
        let outcome = contract.evaluate(&results);
        assert!(matches!(outcome, GreenOutcome::Fail(_)));
        assert!(outcome.reason().unwrap().contains("tests"));
    }

    #[test]
    fn evaluate_merge_ready_requires_all_checks() {
        let contract = GreenContract::new(QualityLevel::MergeReady);
        // All passing except merge_checks
        let results = CheckResults {
            tests_passed: true, package_build: true, workspace_build: true,
            lint_clean: true, merge_checks: false,
        };
        let outcome = contract.evaluate(&results);
        assert!(matches!(outcome, GreenOutcome::Fail(_)));
        assert!(outcome.reason().unwrap().contains("merge"));
    }
}
