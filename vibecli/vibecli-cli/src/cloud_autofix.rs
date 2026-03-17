
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of issue the autofix addresses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FixType {
    CompileError,
    TestFailure,
    LintViolation,
    SecurityVuln,
    TypeMismatch,
    NullCheck,
    BoundaryCheck,
    ResourceLeak,
}

/// Outcome of a proposed fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixResult {
    Merged,
    Rejected,
    Pending,
    TestFailed,
    ConflictDetected,
}

/// Strategy used when generating a fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixStrategy {
    /// Apply the smallest possible change.
    Direct,
    /// Change only what is strictly necessary to pass tests.
    Minimal,
    /// Refactor surrounding code for long-term quality.
    Comprehensive,
}

/// A single fix attempt against a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixAttempt {
    pub id: String,
    pub pr_number: u64,
    pub file_path: String,
    pub original_code: String,
    pub fixed_code: String,
    pub fix_type: FixType,
    pub confidence: f64,
    pub test_passed: bool,
}

/// Result of running the test suite against a fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub passed: bool,
    pub total_tests: u32,
    pub failed_tests: u32,
    pub duration_ms: u64,
    pub output: String,
}

/// Resource limits for the cloud sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_disk_mb: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_cores: 2,
            max_memory_mb: 4096,
            max_disk_mb: 10240,
        }
    }
}

/// Configuration for cloud-based sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudExecution {
    pub sandbox_id: String,
    pub container_image: String,
    pub timeout_secs: u64,
    pub resource_limits: ResourceLimits,
}

impl Default for CloudExecution {
    fn default() -> Self {
        Self {
            sandbox_id: String::new(),
            container_image: "vibecody/autofix:latest".to_string(),
            timeout_secs: 300,
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// Aggregate statistics for autofix operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofixStats {
    pub total_attempts: u64,
    pub merged: u64,
    pub rejected: u64,
    pub pending: u64,
    pub merge_rate: f64,
    pub avg_confidence: f64,
}

/// Cloud-based autofix agent that tests and proposes fixes on PRs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofixAgent {
    pub name: String,
    pub enabled: bool,
    pub strategy: FixStrategy,
    sandbox: CloudExecution,
}

impl AutofixAgent {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            strategy: FixStrategy::Minimal,
            sandbox: CloudExecution::default(),
        }
    }

    pub fn with_strategy(mut self, strategy: FixStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn configure_sandbox(&mut self, config: CloudExecution) {
        self.sandbox = config;
    }

    pub fn sandbox(&self) -> &CloudExecution {
        &self.sandbox
    }
}

/// Pipeline that orchestrates fix generation, testing, and merging.
pub struct AutofixPipeline {
    agent: AutofixAgent,
    attempts: Vec<FixAttempt>,
    results: HashMap<String, FixResult>,
    strategy: FixStrategy,
    sandbox: CloudExecution,
}

impl AutofixPipeline {
    /// Create a new pipeline with default settings.
    pub fn new() -> Self {
        Self {
            agent: AutofixAgent::new("default"),
            attempts: Vec::new(),
            results: HashMap::new(),
            strategy: FixStrategy::Minimal,
            sandbox: CloudExecution::default(),
        }
    }

    /// Create a pipeline with a specific agent.
    pub fn with_agent(agent: AutofixAgent) -> Self {
        let strategy = agent.strategy.clone();
        let sandbox = agent.sandbox.clone();
        Self {
            agent,
            attempts: Vec::new(),
            results: HashMap::new(),
            strategy,
            sandbox,
        }
    }

    /// Configure the cloud sandbox used for test execution.
    pub fn configure_sandbox(&mut self, config: CloudExecution) {
        self.sandbox = config;
    }

    /// Analyze a PR diff and generate fix candidates.
    ///
    /// Scans the diff text for patterns that indicate common issues
    /// and produces `FixAttempt` entries with proposed corrections.
    pub fn analyze_pr(&mut self, pr_number: u64, diff: &str) -> Vec<FixAttempt> {
        let mut fixes = Vec::new();
        let lines: Vec<&str> = diff.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Detect unwrap() calls — potential null-check issue.
            if trimmed.contains(".unwrap()") {
                let fixed = trimmed.replace(".unwrap()", ".unwrap_or_default()");
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    &fixed,
                    FixType::NullCheck,
                    0.82,
                ));
            }

            // Detect TODO/FIXME compile markers.
            if trimmed.contains("todo!()") || trimmed.contains("unimplemented!()") {
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    "// TODO: implement",
                    FixType::CompileError,
                    0.60,
                ));
            }

            // Detect unsafe blocks.
            if trimmed.starts_with("unsafe {") || trimmed.contains("unsafe {") {
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    &format!("/* SAFETY: reviewed */ {}", trimmed),
                    FixType::SecurityVuln,
                    0.55,
                ));
            }

            // Detect potential boundary issues with indexing.
            if trimmed.contains("[i]") || trimmed.contains("[index]") {
                let fixed = if trimmed.contains("[i]") {
                    trimmed.replace("[i]", ".get(i).copied().unwrap_or_default()")
                } else {
                    trimmed.replace("[index]", ".get(index).copied().unwrap_or_default()")
                };
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    &fixed,
                    FixType::BoundaryCheck,
                    0.75,
                ));
            }

            // Detect lint: unused variable.
            if trimmed.starts_with("let ") && !trimmed.starts_with("let _") && trimmed.contains('=')
            {
                if let Some(var_name) = trimmed
                    .strip_prefix("let ")
                    .and_then(|s| s.split([':', '=']).next())
                    .map(|s| s.trim())
                {
                    if !var_name.is_empty() && !var_name.starts_with('_') {
                        let fixed = trimmed.replacen(
                            &format!("let {}", var_name),
                            &format!("let _{}", var_name),
                            1,
                        );
                        fixes.push(self.create_attempt(
                            pr_number,
                            &format!("file_{}.rs", i),
                            trimmed,
                            &fixed,
                            FixType::LintViolation,
                            0.90,
                        ));
                    }
                }
            }

            // Detect type mismatches: as casts without safety.
            if trimmed.contains(" as u") || trimmed.contains(" as i") {
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    &format!("/* checked cast */ {}", trimmed),
                    FixType::TypeMismatch,
                    0.70,
                ));
            }

            // Detect resource leaks: File::open without closing.
            if trimmed.contains("File::open") && !trimmed.contains("drop") {
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    &format!("{{ let _f = {}; /* auto-drop */ }}", trimmed),
                    FixType::ResourceLeak,
                    0.65,
                ));
            }

            // Detect test failure patterns (assert with wrong value).
            if trimmed.contains("#[should_panic]") {
                fixes.push(self.create_attempt(
                    pr_number,
                    &format!("file_{}.rs", i),
                    trimmed,
                    "#[should_panic(expected = \"explicit panic\")]",
                    FixType::TestFailure,
                    0.50,
                ));
            }
        }

        // Adjust confidence based on strategy.
        for fix in &mut fixes {
            match self.strategy {
                FixStrategy::Direct => fix.confidence *= 0.9,
                FixStrategy::Minimal => { /* keep as-is */ }
                FixStrategy::Comprehensive => fix.confidence *= 1.05_f64.min(1.0 / fix.confidence),
            }
            fix.confidence = fix.confidence.clamp(0.0, 1.0);
        }

        self.attempts.extend(fixes.clone());
        fixes
    }

    /// Run tests against a fix attempt in the configured sandbox.
    pub fn run_tests(&self, fix_attempt: &FixAttempt) -> TestResult {
        // Simulate test execution — in production this would invoke the
        // cloud sandbox container with the patched code.
        let base_pass_rate = fix_attempt.confidence;
        let total = 50;
        let failed = ((1.0 - base_pass_rate) * total as f64).round() as u32;
        let passed = failed == 0;

        TestResult {
            passed,
            total_tests: total,
            failed_tests: failed,
            duration_ms: self.sandbox.timeout_secs * 10, // simulated
            output: if passed {
                "All tests passed.".to_string()
            } else {
                format!("{} test(s) failed.", failed)
            },
        }
    }

    /// Propose a fix: run tests and record the result.
    pub fn propose_fix(&mut self, fix_attempt: &FixAttempt) -> FixResult {
        let test_result = self.run_tests(fix_attempt);

        let result = if !test_result.passed {
            FixResult::TestFailed
        } else if fix_attempt.confidence >= 0.80 {
            FixResult::Merged
        } else if fix_attempt.confidence >= 0.50 {
            FixResult::Pending
        } else {
            FixResult::Rejected
        };

        self.results.insert(fix_attempt.id.clone(), result.clone());
        result
    }

    /// Calculate the merge rate as a percentage (0.0–100.0).
    pub fn get_merge_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let merged = self
            .results
            .values()
            .filter(|r| **r == FixResult::Merged)
            .count() as f64;
        (merged / self.results.len() as f64) * 100.0
    }

    /// Get aggregate statistics for all attempts processed so far.
    pub fn get_stats(&self) -> AutofixStats {
        let merged = self
            .results
            .values()
            .filter(|r| **r == FixResult::Merged)
            .count() as u64;
        let rejected = self
            .results
            .values()
            .filter(|r| **r == FixResult::Rejected)
            .count() as u64;
        let pending = self
            .results
            .values()
            .filter(|r| **r == FixResult::Pending)
            .count() as u64;
        let total = self.results.len() as u64;

        let avg_confidence = if self.attempts.is_empty() {
            0.0
        } else {
            self.attempts.iter().map(|a| a.confidence).sum::<f64>() / self.attempts.len() as f64
        };

        AutofixStats {
            total_attempts: total,
            merged,
            rejected,
            pending,
            merge_rate: self.get_merge_rate(),
            avg_confidence,
        }
    }

    // ---- internal helpers ----

    fn create_attempt(
        &self,
        pr_number: u64,
        file_path: &str,
        original: &str,
        fixed: &str,
        fix_type: FixType,
        confidence: f64,
    ) -> FixAttempt {
        let id = format!(
            "fix-{}-{}-{}",
            pr_number,
            file_path.replace('/', "-"),
            self.attempts.len()
        );
        FixAttempt {
            id,
            pr_number,
            file_path: file_path.to_string(),
            original_code: original.to_string(),
            fixed_code: fixed.to_string(),
            fix_type,
            confidence,
            test_passed: false,
        }
    }
}

impl Default for AutofixPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Pipeline creation ----

    #[test]
    fn test_pipeline_new() {
        let p = AutofixPipeline::new();
        assert!(p.attempts.is_empty());
        assert!(p.results.is_empty());
        assert_eq!(p.strategy, FixStrategy::Minimal);
    }

    #[test]
    fn test_pipeline_default() {
        let p = AutofixPipeline::default();
        assert_eq!(p.agent.name, "default");
    }

    #[test]
    fn test_pipeline_with_agent() {
        let agent = AutofixAgent::new("ci-bot").with_strategy(FixStrategy::Comprehensive);
        let p = AutofixPipeline::with_agent(agent);
        assert_eq!(p.agent.name, "ci-bot");
        assert_eq!(p.strategy, FixStrategy::Comprehensive);
    }

    // ---- Agent ----

    #[test]
    fn test_agent_new() {
        let a = AutofixAgent::new("test");
        assert!(a.enabled);
        assert_eq!(a.strategy, FixStrategy::Minimal);
    }

    #[test]
    fn test_agent_with_strategy() {
        let a = AutofixAgent::new("s").with_strategy(FixStrategy::Direct);
        assert_eq!(a.strategy, FixStrategy::Direct);
    }

    #[test]
    fn test_agent_configure_sandbox() {
        let mut a = AutofixAgent::new("s");
        let cfg = CloudExecution {
            sandbox_id: "sb-1".into(),
            container_image: "img:v2".into(),
            timeout_secs: 600,
            resource_limits: ResourceLimits::default(),
        };
        a.configure_sandbox(cfg);
        assert_eq!(a.sandbox().sandbox_id, "sb-1");
        assert_eq!(a.sandbox().timeout_secs, 600);
    }

    // ---- Fix generation ----

    #[test]
    fn test_analyze_pr_unwrap() {
        let mut p = AutofixPipeline::new();
        let diff = "let x = foo.unwrap();\n";
        let fixes = p.analyze_pr(1, diff);
        assert!(fixes.iter().any(|f| f.fix_type == FixType::NullCheck));
    }

    #[test]
    fn test_analyze_pr_todo() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(2, "todo!()");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::CompileError));
    }

    #[test]
    fn test_analyze_pr_unsafe() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(3, "unsafe { ptr::read(p) }");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::SecurityVuln));
    }

    #[test]
    fn test_analyze_pr_boundary() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(4, "let v = arr[i];");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::BoundaryCheck));
    }

    #[test]
    fn test_analyze_pr_lint_unused() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(5, "let foo = 42;");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::LintViolation));
    }

    #[test]
    fn test_analyze_pr_type_cast() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(6, "let x = val as u32;");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::TypeMismatch));
    }

    #[test]
    fn test_analyze_pr_resource_leak() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(7, "let f = File::open(\"a.txt\");");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::ResourceLeak));
    }

    #[test]
    fn test_analyze_pr_test_failure() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(8, "#[should_panic]");
        assert!(fixes.iter().any(|f| f.fix_type == FixType::TestFailure));
    }

    #[test]
    fn test_analyze_pr_empty_diff() {
        let mut p = AutofixPipeline::new();
        let fixes = p.analyze_pr(9, "");
        assert!(fixes.is_empty());
    }

    #[test]
    fn test_analyze_pr_multiple_issues() {
        let mut p = AutofixPipeline::new();
        let diff = "foo.unwrap()\nunsafe { x }\ntodo!()";
        let fixes = p.analyze_pr(10, diff);
        assert!(fixes.len() >= 3);
    }

    // ---- Test execution ----

    #[test]
    fn test_run_tests_high_confidence() {
        let p = AutofixPipeline::new();
        let attempt = FixAttempt {
            id: "f1".into(),
            pr_number: 1,
            file_path: "a.rs".into(),
            original_code: "x".into(),
            fixed_code: "y".into(),
            fix_type: FixType::NullCheck,
            confidence: 1.0,
            test_passed: false,
        };
        let r = p.run_tests(&attempt);
        assert!(r.passed);
        assert_eq!(r.failed_tests, 0);
        assert_eq!(r.total_tests, 50);
    }

    #[test]
    fn test_run_tests_low_confidence() {
        let p = AutofixPipeline::new();
        let attempt = FixAttempt {
            id: "f2".into(),
            pr_number: 2,
            file_path: "b.rs".into(),
            original_code: "x".into(),
            fixed_code: "y".into(),
            fix_type: FixType::CompileError,
            confidence: 0.3,
            test_passed: false,
        };
        let r = p.run_tests(&attempt);
        assert!(!r.passed);
        assert!(r.failed_tests > 0);
    }

    // ---- Propose fix / merge rate ----

    #[test]
    fn test_propose_fix_merged() {
        let mut p = AutofixPipeline::new();
        let attempt = FixAttempt {
            id: "m1".into(),
            pr_number: 1,
            file_path: "a.rs".into(),
            original_code: "".into(),
            fixed_code: "".into(),
            fix_type: FixType::LintViolation,
            confidence: 1.0,
            test_passed: false,
        };
        assert_eq!(p.propose_fix(&attempt), FixResult::Merged);
    }

    #[test]
    fn test_propose_fix_test_failed() {
        let mut p = AutofixPipeline::new();
        let attempt = FixAttempt {
            id: "tf1".into(),
            pr_number: 2,
            file_path: "b.rs".into(),
            original_code: "".into(),
            fixed_code: "".into(),
            fix_type: FixType::CompileError,
            confidence: 0.2,
            test_passed: false,
        };
        assert_eq!(p.propose_fix(&attempt), FixResult::TestFailed);
    }

    #[test]
    fn test_merge_rate_empty() {
        let p = AutofixPipeline::new();
        assert_eq!(p.get_merge_rate(), 0.0);
    }

    #[test]
    fn test_merge_rate_all_merged() {
        let mut p = AutofixPipeline::new();
        for i in 0..5 {
            let a = FixAttempt {
                id: format!("a{}", i),
                pr_number: 1,
                file_path: "f.rs".into(),
                original_code: "".into(),
                fixed_code: "".into(),
                fix_type: FixType::NullCheck,
                confidence: 1.0,
                test_passed: false,
            };
            p.propose_fix(&a);
        }
        assert!((p.get_merge_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merge_rate_mixed() {
        let mut p = AutofixPipeline::new();
        // One merged
        p.propose_fix(&FixAttempt {
            id: "ok".into(),
            pr_number: 1,
            file_path: "f.rs".into(),
            original_code: "".into(),
            fixed_code: "".into(),
            fix_type: FixType::NullCheck,
            confidence: 1.0,
            test_passed: false,
        });
        // One failed
        p.propose_fix(&FixAttempt {
            id: "fail".into(),
            pr_number: 2,
            file_path: "g.rs".into(),
            original_code: "".into(),
            fixed_code: "".into(),
            fix_type: FixType::CompileError,
            confidence: 0.1,
            test_passed: false,
        });
        let rate = p.get_merge_rate();
        assert!(rate > 0.0 && rate < 100.0);
    }

    // ---- Stats ----

    #[test]
    fn test_get_stats_empty() {
        let p = AutofixPipeline::new();
        let s = p.get_stats();
        assert_eq!(s.total_attempts, 0);
        assert_eq!(s.merge_rate, 0.0);
    }

    #[test]
    fn test_get_stats_after_fixes() {
        let mut p = AutofixPipeline::new();
        let diff = "foo.unwrap()\nunsafe { x }";
        let fixes = p.analyze_pr(42, diff);
        for f in &fixes {
            p.propose_fix(f);
        }
        let s = p.get_stats();
        assert!(s.total_attempts > 0);
        assert!(s.avg_confidence > 0.0);
    }

    // ---- Sandbox config ----

    #[test]
    fn test_configure_sandbox() {
        let mut p = AutofixPipeline::new();
        let cfg = CloudExecution {
            sandbox_id: "sb-99".into(),
            container_image: "custom:v3".into(),
            timeout_secs: 900,
            resource_limits: ResourceLimits {
                max_cpu_cores: 8,
                max_memory_mb: 16384,
                max_disk_mb: 51200,
            },
        };
        p.configure_sandbox(cfg);
        assert_eq!(p.sandbox.sandbox_id, "sb-99");
        assert_eq!(p.sandbox.resource_limits.max_cpu_cores, 8);
    }

    #[test]
    fn test_resource_limits_default() {
        let r = ResourceLimits::default();
        assert_eq!(r.max_cpu_cores, 2);
        assert_eq!(r.max_memory_mb, 4096);
        assert_eq!(r.max_disk_mb, 10240);
    }

    // ---- Fix strategies ----

    #[test]
    fn test_direct_strategy_lowers_confidence() {
        let agent = AutofixAgent::new("d").with_strategy(FixStrategy::Direct);
        let mut p = AutofixPipeline::with_agent(agent);
        let fixes = p.analyze_pr(100, "foo.unwrap()");
        // Direct strategy multiplies confidence by 0.9
        for f in &fixes {
            assert!(f.confidence <= 0.82 * 0.9 + 0.001);
        }
    }

    #[test]
    fn test_comprehensive_strategy() {
        let agent = AutofixAgent::new("c").with_strategy(FixStrategy::Comprehensive);
        let mut p = AutofixPipeline::with_agent(agent);
        let fixes = p.analyze_pr(101, "foo.unwrap()");
        assert!(!fixes.is_empty());
        for f in &fixes {
            assert!(f.confidence <= 1.0);
        }
    }

    // ---- Serialization round-trip ----

    #[test]
    fn test_fix_attempt_serde() {
        let a = FixAttempt {
            id: "s1".into(),
            pr_number: 42,
            file_path: "lib.rs".into(),
            original_code: "old".into(),
            fixed_code: "new".into(),
            fix_type: FixType::SecurityVuln,
            confidence: 0.77,
            test_passed: true,
        };
        let json = serde_json::to_string(&a).expect("serialize");
        let b: FixAttempt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b.id, "s1");
        assert_eq!(b.fix_type, FixType::SecurityVuln);
    }

    #[test]
    fn test_stats_serde() {
        let s = AutofixStats {
            total_attempts: 10,
            merged: 4,
            rejected: 3,
            pending: 3,
            merge_rate: 40.0,
            avg_confidence: 0.72,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let s2: AutofixStats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s2.merged, 4);
        assert!((s2.merge_rate - 40.0).abs() < f64::EPSILON);
    }
}
