#![allow(dead_code)]
//! Intent-preserving refactoring — semantic intent understanding with equivalence verification.
//!
//! Understands the semantic INTENT behind refactoring requests, plans multi-step
//! transformations, and verifies behavioral equivalence before and after each step.
//!
//! # Architecture
//!
//! ```text
//! User Request -> IntentParser -> PlanGenerator -> RefactorEngine -> EquivalenceChecker
//!   - Keyword-based intent detection (12 intent types)
//!   - Multi-step plan generation per intent + target files
//!   - Step-by-step execution with before/after snapshots
//!   - Public API signature extraction for equivalence checks
//!   - Rollback support on failure or verification mismatch
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RefactorIntent {
    MakeTestable,
    ReduceCoupling,
    ImprovePerformance,
    AddErrorHandling,
    ExtractService,
    ConsolidateDuplicates,
    ModernizeSyntax,
    AddTyping,
    SplitModule,
    MergeModules,
    AddCaching,
    AddLogging,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Planned,
    InProgress,
    Completed,
    Skipped,
    Failed(String),
    Rolled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    Equivalent,
    NotEquivalent(String),
    Unknown,
    Skipped,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RefactorPlan {
    pub id: String,
    pub intent: RefactorIntent,
    pub description: String,
    pub target_files: Vec<String>,
    pub steps: Vec<RefactorStep>,
    pub created_at: u64,
    pub estimated_impact: f64,
}

#[derive(Debug, Clone)]
pub struct RefactorStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub status: StepStatus,
    pub before_snapshot: String,
    pub after_snapshot: Option<String>,
    pub verification: VerificationResult,
}

#[derive(Debug, Clone)]
pub struct RefactorSession {
    pub plan: RefactorPlan,
    pub current_step: usize,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    pub files_affected: Vec<String>,
    pub functions_affected: Vec<String>,
    pub test_files: Vec<String>,
    pub risk_score: f64,
    pub breaking_changes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RefactorConfig {
    pub auto_verify: bool,
    pub rollback_on_failure: bool,
    pub max_steps: usize,
    pub require_tests: bool,
}

impl Default for RefactorConfig {
    fn default() -> Self {
        Self {
            auto_verify: true,
            rollback_on_failure: true,
            max_steps: 20,
            require_tests: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RefactorMetrics {
    pub total_refactors: u64,
    pub completed: u64,
    pub rolled_back: u64,
    pub steps_executed: u64,
    pub equivalence_verified: u64,
    pub avg_steps_per_refactor: f64,
}

// ---------------------------------------------------------------------------
// IntentParser — keyword-based intent detection
// ---------------------------------------------------------------------------

pub struct IntentParser;

impl IntentParser {
    /// Parse a natural-language refactoring request into a `RefactorIntent`.
    pub fn parse(input: &str) -> Option<RefactorIntent> {
        let lower = input.to_lowercase();

        // Order matters: check more specific phrases first.
        if lower.contains("testable") || lower.contains("test coverage") || lower.contains("make testable") {
            return Some(RefactorIntent::MakeTestable);
        }
        if lower.contains("coupling") || lower.contains("decouple") || lower.contains("reduce coupling") {
            return Some(RefactorIntent::ReduceCoupling);
        }
        if lower.contains("performance") || lower.contains("optimize") || lower.contains("speed up") || lower.contains("faster") {
            return Some(RefactorIntent::ImprovePerformance);
        }
        if lower.contains("error handling") || lower.contains("error recovery") || lower.contains("add error") || lower.contains("handle error") {
            return Some(RefactorIntent::AddErrorHandling);
        }
        if lower.contains("extract service") || lower.contains("extract module") || lower.contains("pull out") {
            return Some(RefactorIntent::ExtractService);
        }
        if lower.contains("duplicate") || lower.contains("consolidate") || lower.contains("dry") || lower.contains("deduplicate") {
            return Some(RefactorIntent::ConsolidateDuplicates);
        }
        if lower.contains("modernize") || lower.contains("modern syntax") || lower.contains("update syntax") || lower.contains("upgrade syntax") {
            return Some(RefactorIntent::ModernizeSyntax);
        }
        if lower.contains("typing") || lower.contains("type annotation") || lower.contains("add types") || lower.contains("type safety") {
            return Some(RefactorIntent::AddTyping);
        }
        if lower.contains("split module") || lower.contains("split file") || lower.contains("break apart") {
            return Some(RefactorIntent::SplitModule);
        }
        if lower.contains("merge module") || lower.contains("merge file") || lower.contains("combine module") {
            return Some(RefactorIntent::MergeModules);
        }
        if lower.contains("caching") || lower.contains("cache") || lower.contains("memoize") || lower.contains("memoization") {
            return Some(RefactorIntent::AddCaching);
        }
        if lower.contains("logging") || lower.contains("log") || lower.contains("trace") || lower.contains("instrument") {
            return Some(RefactorIntent::AddLogging);
        }
        None
    }

    /// Return a human-readable description for the given intent.
    pub fn describe(intent: &RefactorIntent) -> &str {
        match intent {
            RefactorIntent::MakeTestable => "Extract dependencies and add seams to make code unit-testable",
            RefactorIntent::ReduceCoupling => "Introduce interfaces and reduce direct dependencies between modules",
            RefactorIntent::ImprovePerformance => "Optimize hot paths, reduce allocations, and improve algorithmic efficiency",
            RefactorIntent::AddErrorHandling => "Replace panics/unwraps with proper error types and recovery logic",
            RefactorIntent::ExtractService => "Extract a cohesive set of functions into a standalone service or module",
            RefactorIntent::ConsolidateDuplicates => "Identify and merge duplicated logic into shared abstractions",
            RefactorIntent::ModernizeSyntax => "Update legacy patterns to idiomatic modern language features",
            RefactorIntent::AddTyping => "Add type annotations and strengthen type safety across the codebase",
            RefactorIntent::SplitModule => "Break a large module into smaller, focused sub-modules",
            RefactorIntent::MergeModules => "Combine related small modules into a single cohesive module",
            RefactorIntent::AddCaching => "Add caching layers to reduce redundant computation or I/O",
            RefactorIntent::AddLogging => "Add structured logging and tracing instrumentation",
        }
    }

    /// Suggest applicable refactoring intents for the given code with confidence scores.
    pub fn suggest_intents(code: &str) -> Vec<(RefactorIntent, f64)> {
        let mut suggestions = Vec::new();

        // Heuristic: unwrap/expect -> AddErrorHandling
        let unwrap_count = code.matches(".unwrap()").count() + code.matches(".expect(").count();
        if unwrap_count > 0 {
            let confidence = (unwrap_count as f64 * 0.15).min(0.95);
            suggestions.push((RefactorIntent::AddErrorHandling, confidence));
        }

        // Heuristic: long functions -> MakeTestable / SplitModule
        let line_count = code.lines().count();
        if line_count > 200 {
            suggestions.push((RefactorIntent::SplitModule, 0.7));
            suggestions.push((RefactorIntent::MakeTestable, 0.5));
        }

        // Heuristic: repeated blocks -> ConsolidateDuplicates
        let lines: Vec<&str> = code.lines().map(|l| l.trim()).filter(|l| l.len() > 20).collect();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for line in &lines {
            *seen.entry(line).or_insert(0) += 1;
        }
        let dup_count = seen.values().filter(|&&c| c > 2).count();
        if dup_count > 0 {
            let confidence = (dup_count as f64 * 0.2).min(0.9);
            suggestions.push((RefactorIntent::ConsolidateDuplicates, confidence));
        }

        // Heuristic: no log/trace calls -> AddLogging
        if !code.contains("log::") && !code.contains("tracing::") && !code.contains("println!") && line_count > 50 {
            suggestions.push((RefactorIntent::AddLogging, 0.4));
        }

        // Heuristic: many direct struct field accesses -> ReduceCoupling
        let dot_accesses = code.matches(".field").count() + code.matches(".inner").count();
        if dot_accesses > 10 {
            suggestions.push((RefactorIntent::ReduceCoupling, 0.45));
        }

        // Heuristic: var/any usage (TypeScript) -> AddTyping
        if code.contains(": any") || code.contains("as any") || code.contains("var ") {
            suggestions.push((RefactorIntent::AddTyping, 0.6));
        }

        // Heuristic: old-style callbacks / no async-await -> ModernizeSyntax
        if code.contains(".then(") && !code.contains("async ") {
            suggestions.push((RefactorIntent::ModernizeSyntax, 0.55));
        }

        // Sort by confidence descending
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        suggestions
    }
}

// ---------------------------------------------------------------------------
// PlanGenerator — multi-step plan creation
// ---------------------------------------------------------------------------

pub struct PlanGenerator;

impl PlanGenerator {
    /// Generate a multi-step refactoring plan for the given intent and target files.
    pub fn generate(intent: &RefactorIntent, files: &[&str]) -> RefactorPlan {
        let steps = Self::generate_steps(intent, files);
        let estimated_impact = Self::estimate_impact_score(intent, files.len());

        RefactorPlan {
            id: String::new(), // Assigned by the engine
            intent: intent.clone(),
            description: IntentParser::describe(intent).to_string(),
            target_files: files.iter().map(|f| f.to_string()).collect(),
            steps,
            created_at: 0, // Assigned by the engine
            estimated_impact,
        }
    }

    fn generate_steps(intent: &RefactorIntent, files: &[&str]) -> Vec<RefactorStep> {
        let templates: Vec<(&str, &str)> = match intent {
            RefactorIntent::MakeTestable => vec![
                ("Identify tight couplings", "Scan for direct dependencies that hinder testing"),
                ("Extract interfaces", "Create trait/interface for each dependency"),
                ("Inject dependencies", "Replace direct construction with dependency injection"),
                ("Add test doubles", "Create mock/stub implementations for testing"),
                ("Write unit tests", "Add tests using the new seams"),
            ],
            RefactorIntent::ReduceCoupling => vec![
                ("Analyze dependency graph", "Map module-to-module dependencies"),
                ("Define abstractions", "Create traits/interfaces at module boundaries"),
                ("Invert dependencies", "Point dependencies toward abstractions"),
                ("Validate isolation", "Ensure modules compile independently"),
            ],
            RefactorIntent::ImprovePerformance => vec![
                ("Profile hot paths", "Identify the most expensive code paths"),
                ("Reduce allocations", "Replace heap allocations with stack or pooled alternatives"),
                ("Optimize algorithms", "Switch to more efficient data structures or algorithms"),
                ("Add benchmarks", "Create benchmarks to verify improvements"),
            ],
            RefactorIntent::AddErrorHandling => vec![
                ("Audit panics", "Find all unwrap(), expect(), panic! call sites"),
                ("Define error types", "Create structured error enum for the module"),
                ("Replace panics", "Convert each panic site to Result/Option propagation"),
                ("Add recovery logic", "Implement graceful degradation at boundaries"),
                ("Update tests", "Ensure tests cover error paths"),
            ],
            RefactorIntent::ExtractService => vec![
                ("Identify cohesive set", "Group functions that form a logical service"),
                ("Create module", "Set up the new module/crate structure"),
                ("Move functions", "Relocate functions and their dependencies"),
                ("Update imports", "Fix all call sites to reference the new module"),
                ("Add public API", "Define a clean public interface"),
            ],
            RefactorIntent::ConsolidateDuplicates => vec![
                ("Detect duplicates", "Scan for repeated logic patterns"),
                ("Design abstraction", "Create a shared function or trait"),
                ("Replace occurrences", "Swap each duplicate with the shared abstraction"),
                ("Verify behavior", "Run tests to confirm no regressions"),
            ],
            RefactorIntent::ModernizeSyntax => vec![
                ("Catalog legacy patterns", "List outdated syntax and idioms"),
                ("Apply modern equivalents", "Rewrite using current language features"),
                ("Update dependencies", "Bump any library versions needed for new syntax"),
                ("Run linter", "Verify no new warnings introduced"),
            ],
            RefactorIntent::AddTyping => vec![
                ("Identify untyped code", "Find variables and parameters lacking type annotations"),
                ("Infer types", "Determine correct types from usage context"),
                ("Annotate signatures", "Add explicit type annotations to function signatures"),
                ("Enable strict mode", "Turn on strict type-checking flags"),
            ],
            RefactorIntent::SplitModule => vec![
                ("Analyze responsibilities", "Identify distinct concerns within the module"),
                ("Plan sub-modules", "Design the target module structure"),
                ("Extract sub-modules", "Move code into new sub-module files"),
                ("Wire re-exports", "Set up public re-exports from the parent module"),
                ("Update dependents", "Fix all external imports"),
            ],
            RefactorIntent::MergeModules => vec![
                ("Analyze overlap", "Identify shared types and functions across modules"),
                ("Resolve conflicts", "Handle naming collisions and visibility differences"),
                ("Combine code", "Merge source files into the target module"),
                ("Update imports", "Redirect all external references"),
            ],
            RefactorIntent::AddCaching => vec![
                ("Identify cache candidates", "Find expensive pure computations or I/O"),
                ("Design cache strategy", "Choose TTL, eviction, and invalidation policies"),
                ("Implement cache layer", "Add caching wrappers around identified functions"),
                ("Add cache metrics", "Instrument hit/miss rates for observability"),
            ],
            RefactorIntent::AddLogging => vec![
                ("Identify log points", "Determine where log statements should be added"),
                ("Choose log levels", "Assign appropriate severity to each log point"),
                ("Add structured logging", "Insert log calls with context fields"),
                ("Verify output", "Check log output format and verbosity"),
            ],
        };

        let mut steps = Vec::new();
        for (i, (title, desc)) in templates.iter().enumerate() {
            // Distribute steps across target files in round-robin fashion.
            let file_path = if files.is_empty() {
                String::new()
            } else {
                files[i % files.len()].to_string()
            };

            steps.push(RefactorStep {
                id: format!("step-{}", i + 1),
                title: title.to_string(),
                description: desc.to_string(),
                file_path,
                status: StepStatus::Planned,
                before_snapshot: String::new(),
                after_snapshot: None,
                verification: VerificationResult::Skipped,
            });
        }
        steps
    }

    fn estimate_impact_score(intent: &RefactorIntent, file_count: usize) -> f64 {
        let base = match intent {
            RefactorIntent::MakeTestable => 0.6,
            RefactorIntent::ReduceCoupling => 0.7,
            RefactorIntent::ImprovePerformance => 0.5,
            RefactorIntent::AddErrorHandling => 0.65,
            RefactorIntent::ExtractService => 0.75,
            RefactorIntent::ConsolidateDuplicates => 0.55,
            RefactorIntent::ModernizeSyntax => 0.3,
            RefactorIntent::AddTyping => 0.4,
            RefactorIntent::SplitModule => 0.6,
            RefactorIntent::MergeModules => 0.5,
            RefactorIntent::AddCaching => 0.45,
            RefactorIntent::AddLogging => 0.25,
        };
        // More files -> higher impact, capped at 1.0
        (base + file_count as f64 * 0.05).min(1.0)
    }

    /// Estimate the impact of an existing plan.
    pub fn estimate_impact(plan: &RefactorPlan) -> ImpactAnalysis {
        let functions_affected: Vec<String> = plan.steps.iter().map(|s| s.title.clone()).collect();
        let test_files: Vec<String> = plan
            .target_files
            .iter()
            .map(|f| {
                if f.ends_with(".rs") {
                    f.replace(".rs", "_test.rs")
                } else if f.ends_with(".ts") {
                    f.replace(".ts", ".test.ts")
                } else {
                    format!("{}.test", f)
                }
            })
            .collect();

        let risk_score = match plan.intent {
            RefactorIntent::ExtractService | RefactorIntent::MergeModules => 0.8,
            RefactorIntent::SplitModule | RefactorIntent::ReduceCoupling => 0.6,
            RefactorIntent::AddErrorHandling | RefactorIntent::MakeTestable => 0.5,
            RefactorIntent::ImprovePerformance | RefactorIntent::AddCaching => 0.4,
            RefactorIntent::ConsolidateDuplicates => 0.35,
            RefactorIntent::ModernizeSyntax | RefactorIntent::AddTyping => 0.25,
            RefactorIntent::AddLogging => 0.15,
        };

        let breaking_changes = if risk_score > 0.5 {
            vec!["Public API signatures may change".to_string()]
        } else {
            vec![]
        };

        ImpactAnalysis {
            files_affected: plan.target_files.clone(),
            functions_affected,
            test_files,
            risk_score,
            breaking_changes,
        }
    }
}

// ---------------------------------------------------------------------------
// EquivalenceChecker — behavioral equivalence verification
// ---------------------------------------------------------------------------

pub struct EquivalenceChecker;

impl EquivalenceChecker {
    /// Compare before/after code for behavioral equivalence by checking public API
    /// signatures and export parity.
    pub fn check(before: &str, after: &str) -> VerificationResult {
        let before_api = Self::snapshot_api(before);
        let after_api = Self::snapshot_api(after);

        if before_api.is_empty() && after_api.is_empty() {
            return VerificationResult::Unknown;
        }

        // Check that all before-signatures are present in after
        let mut missing = Vec::new();
        for sig in &before_api {
            if !after_api.contains(sig) {
                missing.push(sig.clone());
            }
        }

        if missing.is_empty() {
            VerificationResult::Equivalent
        } else {
            VerificationResult::NotEquivalent(format!(
                "Missing signatures after refactor: {}",
                missing.join(", ")
            ))
        }
    }

    /// Extract public function/method signatures from code.
    pub fn snapshot_api(code: &str) -> Vec<String> {
        let mut signatures = Vec::new();
        for line in code.lines() {
            let trimmed = line.trim();

            // Rust: pub fn / pub async fn
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ") {
                if let Some(sig) = Self::extract_rust_sig(trimmed) {
                    signatures.push(sig);
                }
            }

            // TypeScript/JavaScript: export function / export const / export default
            if trimmed.starts_with("export function ")
                || trimmed.starts_with("export const ")
                || trimmed.starts_with("export default ")
            {
                if let Some(sig) = Self::extract_ts_sig(trimmed) {
                    signatures.push(sig);
                }
            }

            // Python: def (at top indent level)
            if trimmed.starts_with("def ") && line.starts_with("def ") {
                if let Some(sig) = Self::extract_python_sig(trimmed) {
                    signatures.push(sig);
                }
            }
        }
        signatures
    }

    fn extract_rust_sig(line: &str) -> Option<String> {
        // Extract up to the opening brace or semicolon
        let sig = line
            .split('{')
            .next()
            .unwrap_or(line)
            .split(';')
            .next()
            .unwrap_or(line)
            .trim();
        if sig.contains("fn ") {
            Some(sig.to_string())
        } else {
            None
        }
    }

    fn extract_ts_sig(line: &str) -> Option<String> {
        let sig = line.split('{').next().unwrap_or(line).trim();
        if !sig.is_empty() {
            Some(sig.to_string())
        } else {
            None
        }
    }

    fn extract_python_sig(line: &str) -> Option<String> {
        let sig = line.split(':').next().unwrap_or(line).trim();
        if sig.starts_with("def ") {
            Some(sig.to_string())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// RefactorEngine — orchestrates sessions, steps, rollback, verification
// ---------------------------------------------------------------------------

pub struct RefactorEngine {
    pub config: RefactorConfig,
    sessions: HashMap<String, RefactorSession>,
    metrics: RefactorMetrics,
    next_id: u64,
    ts: u64,
}

impl RefactorEngine {
    pub fn new(config: RefactorConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            metrics: RefactorMetrics::default(),
            next_id: 1,
            ts: 1,
        }
    }

    fn next_session_id(&mut self) -> String {
        let id = format!("refactor-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn now(&mut self) -> u64 {
        let t = self.ts;
        self.ts += 1;
        t
    }

    /// Create a new refactoring session and return its id.
    pub fn plan(&mut self, intent: RefactorIntent, target_files: Vec<String>) -> String {
        let file_refs: Vec<&str> = target_files.iter().map(|s| s.as_str()).collect();
        let mut plan = PlanGenerator::generate(&intent, &file_refs);

        let id = self.next_session_id();
        plan.id = id.clone();
        plan.created_at = self.now();

        // Enforce max_steps
        if plan.steps.len() > self.config.max_steps {
            plan.steps.truncate(self.config.max_steps);
        }

        let session = RefactorSession {
            plan,
            current_step: 0,
            started_at: self.now(),
            completed_at: None,
        };

        self.sessions.insert(id.clone(), session);
        self.metrics.total_refactors += 1;
        id
    }

    /// Execute the next planned step in the session.
    pub fn execute_step(&mut self, session_id: &str) -> Result<StepStatus, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        if session.current_step >= session.plan.steps.len() {
            return Err("All steps already executed".to_string());
        }

        let idx = session.current_step;
        let step = &mut session.plan.steps[idx];

        // Skip steps that are already completed or skipped
        if matches!(step.status, StepStatus::Completed | StepStatus::Skipped) {
            session.current_step += 1;
            return Ok(step.status.clone());
        }

        step.status = StepStatus::InProgress;

        // Simulate execution: mark as completed with a synthetic after_snapshot
        step.after_snapshot = Some(format!("[refactored] {}", step.before_snapshot));
        step.status = StepStatus::Completed;

        // Auto-verify if configured
        if self.config.auto_verify {
            let before = step.before_snapshot.clone();
            let after = step.after_snapshot.clone().unwrap_or_default();
            let verification = EquivalenceChecker::check(&before, &after);
            step.verification = verification.clone();

            if let VerificationResult::NotEquivalent(_) = &step.verification {
                if self.config.rollback_on_failure {
                    step.status = StepStatus::Rolled;
                    step.after_snapshot = None;
                    self.metrics.rolled_back += 1;
                    return Ok(StepStatus::Rolled);
                }
            }

            if matches!(step.verification, VerificationResult::Equivalent) {
                self.metrics.equivalence_verified += 1;
            }
        }

        session.current_step += 1;
        self.metrics.steps_executed += 1;

        // Check if all steps are done
        let all_done = session.plan.steps.iter().all(|s| {
            matches!(
                s.status,
                StepStatus::Completed | StepStatus::Skipped | StepStatus::Rolled
            )
        });
        if all_done || session.current_step >= session.plan.steps.len() {
            let t = self.ts; self.ts += 1;
            session.completed_at = Some(t);
            self.metrics.completed += 1;
            self.update_avg_steps();
        }

        Ok(StepStatus::Completed)
    }

    /// Skip the current step.
    pub fn skip_step(&mut self, session_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        if session.current_step >= session.plan.steps.len() {
            return Err("No more steps to skip".to_string());
        }

        let idx = session.current_step;
        session.plan.steps[idx].status = StepStatus::Skipped;
        session.plan.steps[idx].verification = VerificationResult::Skipped;
        session.current_step += 1;

        // Check completion
        if session.current_step >= session.plan.steps.len() {
            let t = self.ts; self.ts += 1;
            session.completed_at = Some(t);
            self.metrics.completed += 1;
            self.update_avg_steps();
        }

        Ok(())
    }

    /// Rollback all completed steps in the session. Returns number of steps rolled back.
    pub fn rollback(&mut self, session_id: &str) -> Result<usize, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut count = 0;
        for step in session.plan.steps.iter_mut().rev() {
            if matches!(step.status, StepStatus::Completed) {
                step.status = StepStatus::Rolled;
                step.after_snapshot = None;
                step.verification = VerificationResult::Skipped;
                count += 1;
            }
        }

        if count > 0 {
            session.current_step = 0;
            session.completed_at = None;
            self.metrics.rolled_back += count as u64;
        }

        Ok(count)
    }

    /// Verify a specific step by index.
    pub fn verify_step(
        &mut self,
        session_id: &str,
        step_idx: usize,
    ) -> Result<VerificationResult, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        if step_idx >= session.plan.steps.len() {
            return Err(format!(
                "Step index {} out of range (max {})",
                step_idx,
                session.plan.steps.len() - 1
            ));
        }

        let step = &mut session.plan.steps[step_idx];
        let before = step.before_snapshot.clone();
        let after = match &step.after_snapshot {
            Some(a) => a.clone(),
            None => return Ok(VerificationResult::Skipped),
        };

        let result = EquivalenceChecker::check(&before, &after);
        step.verification = result.clone();

        if matches!(result, VerificationResult::Equivalent) {
            self.metrics.equivalence_verified += 1;
        }

        Ok(result)
    }

    /// Get a session by id.
    pub fn get_session(&self, id: &str) -> Option<&RefactorSession> {
        self.sessions.get(id)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<&RefactorSession> {
        self.sessions.values().collect()
    }

    /// Get engine metrics.
    pub fn get_metrics(&self) -> &RefactorMetrics {
        &self.metrics
    }

    fn update_avg_steps(&mut self) {
        if self.metrics.completed > 0 {
            self.metrics.avg_steps_per_refactor =
                self.metrics.steps_executed as f64 / self.metrics.completed as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // IntentParser tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_make_testable() {
        assert_eq!(
            IntentParser::parse("make this code testable"),
            Some(RefactorIntent::MakeTestable)
        );
    }

    #[test]
    fn parse_test_coverage() {
        assert_eq!(
            IntentParser::parse("improve test coverage"),
            Some(RefactorIntent::MakeTestable)
        );
    }

    #[test]
    fn parse_reduce_coupling() {
        assert_eq!(
            IntentParser::parse("reduce coupling between modules"),
            Some(RefactorIntent::ReduceCoupling)
        );
    }

    #[test]
    fn parse_decouple() {
        assert_eq!(
            IntentParser::parse("decouple the service layer"),
            Some(RefactorIntent::ReduceCoupling)
        );
    }

    #[test]
    fn parse_improve_performance() {
        assert_eq!(
            IntentParser::parse("optimize performance"),
            Some(RefactorIntent::ImprovePerformance)
        );
    }

    #[test]
    fn parse_speed_up() {
        assert_eq!(
            IntentParser::parse("speed up the parser"),
            Some(RefactorIntent::ImprovePerformance)
        );
    }

    #[test]
    fn parse_faster() {
        assert_eq!(
            IntentParser::parse("make it faster"),
            Some(RefactorIntent::ImprovePerformance)
        );
    }

    #[test]
    fn parse_add_error_handling() {
        assert_eq!(
            IntentParser::parse("add error handling"),
            Some(RefactorIntent::AddErrorHandling)
        );
    }

    #[test]
    fn parse_handle_errors() {
        assert_eq!(
            IntentParser::parse("handle error cases properly"),
            Some(RefactorIntent::AddErrorHandling)
        );
    }

    #[test]
    fn parse_extract_service() {
        assert_eq!(
            IntentParser::parse("extract service for auth"),
            Some(RefactorIntent::ExtractService)
        );
    }

    #[test]
    fn parse_extract_module() {
        assert_eq!(
            IntentParser::parse("extract module for payments"),
            Some(RefactorIntent::ExtractService)
        );
    }

    #[test]
    fn parse_consolidate_duplicates() {
        assert_eq!(
            IntentParser::parse("consolidate duplicate code"),
            Some(RefactorIntent::ConsolidateDuplicates)
        );
    }

    #[test]
    fn parse_dry() {
        assert_eq!(
            IntentParser::parse("apply DRY principle"),
            Some(RefactorIntent::ConsolidateDuplicates)
        );
    }

    #[test]
    fn parse_modernize_syntax() {
        assert_eq!(
            IntentParser::parse("modernize the syntax"),
            Some(RefactorIntent::ModernizeSyntax)
        );
    }

    #[test]
    fn parse_add_typing() {
        assert_eq!(
            IntentParser::parse("add type annotations"),
            Some(RefactorIntent::AddTyping)
        );
    }

    #[test]
    fn parse_type_safety() {
        assert_eq!(
            IntentParser::parse("improve type safety"),
            Some(RefactorIntent::AddTyping)
        );
    }

    #[test]
    fn parse_split_module() {
        assert_eq!(
            IntentParser::parse("split module into parts"),
            Some(RefactorIntent::SplitModule)
        );
    }

    #[test]
    fn parse_split_file() {
        assert_eq!(
            IntentParser::parse("split file into smaller files"),
            Some(RefactorIntent::SplitModule)
        );
    }

    #[test]
    fn parse_merge_modules() {
        assert_eq!(
            IntentParser::parse("merge module A and B"),
            Some(RefactorIntent::MergeModules)
        );
    }

    #[test]
    fn parse_add_caching() {
        assert_eq!(
            IntentParser::parse("add caching layer"),
            Some(RefactorIntent::AddCaching)
        );
    }

    #[test]
    fn parse_memoize() {
        assert_eq!(
            IntentParser::parse("memoize expensive calls"),
            Some(RefactorIntent::AddCaching)
        );
    }

    #[test]
    fn parse_add_logging() {
        assert_eq!(
            IntentParser::parse("add logging to the service"),
            Some(RefactorIntent::AddLogging)
        );
    }

    #[test]
    fn parse_instrument() {
        assert_eq!(
            IntentParser::parse("instrument the code"),
            Some(RefactorIntent::AddLogging)
        );
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(IntentParser::parse("do something random"), None);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(
            IntentParser::parse("OPTIMIZE PERFORMANCE NOW"),
            Some(RefactorIntent::ImprovePerformance)
        );
    }

    #[test]
    fn describe_all_intents() {
        let intents = vec![
            RefactorIntent::MakeTestable,
            RefactorIntent::ReduceCoupling,
            RefactorIntent::ImprovePerformance,
            RefactorIntent::AddErrorHandling,
            RefactorIntent::ExtractService,
            RefactorIntent::ConsolidateDuplicates,
            RefactorIntent::ModernizeSyntax,
            RefactorIntent::AddTyping,
            RefactorIntent::SplitModule,
            RefactorIntent::MergeModules,
            RefactorIntent::AddCaching,
            RefactorIntent::AddLogging,
        ];
        for intent in intents {
            let desc = IntentParser::describe(&intent);
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn suggest_error_handling_for_unwraps() {
        let code = "fn foo() { x.unwrap(); y.unwrap(); z.expect(\"oops\"); }";
        let suggestions = IntentParser::suggest_intents(code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::AddErrorHandling));
    }

    #[test]
    fn suggest_split_for_long_code() {
        let code = (0..250)
            .map(|i| format!("let x{} = {};", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let suggestions = IntentParser::suggest_intents(&code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::SplitModule));
    }

    #[test]
    fn suggest_typing_for_any() {
        let code = "export const foo: any = 42;\nconst bar = x as any;";
        let suggestions = IntentParser::suggest_intents(code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::AddTyping));
    }

    #[test]
    fn suggest_modernize_for_callbacks() {
        let code = "fetch(url).then(res => res.json()).then(data => console.log(data));";
        let suggestions = IntentParser::suggest_intents(code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::ModernizeSyntax));
    }

    #[test]
    fn suggest_sorted_by_confidence() {
        let code =
            "fn a() { x.unwrap(); y.unwrap(); z.unwrap(); w.unwrap(); v.unwrap(); u.unwrap(); t.unwrap(); }";
        let suggestions = IntentParser::suggest_intents(code);
        for window in suggestions.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[test]
    fn suggest_empty_code() {
        let suggestions = IntentParser::suggest_intents("");
        assert!(suggestions.is_empty());
    }

    // -----------------------------------------------------------------------
    // PlanGenerator tests
    // -----------------------------------------------------------------------

    #[test]
    fn generate_plan_has_steps() {
        let plan = PlanGenerator::generate(&RefactorIntent::MakeTestable, &["src/main.rs"]);
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.intent, RefactorIntent::MakeTestable);
    }

    #[test]
    fn generate_plan_with_multiple_files() {
        let plan =
            PlanGenerator::generate(&RefactorIntent::SplitModule, &["a.rs", "b.rs", "c.rs"]);
        assert_eq!(plan.target_files.len(), 3);
        // Steps should be distributed across files
        let paths: Vec<&str> = plan.steps.iter().map(|s| s.file_path.as_str()).collect();
        assert!(paths.contains(&"a.rs"));
    }

    #[test]
    fn generate_plan_empty_files() {
        let plan = PlanGenerator::generate(&RefactorIntent::AddLogging, &[]);
        assert!(plan.target_files.is_empty());
        // Steps still generated, with empty file_path
        for step in &plan.steps {
            assert!(step.file_path.is_empty());
        }
    }

    #[test]
    fn estimate_impact_returns_analysis() {
        let plan = PlanGenerator::generate(&RefactorIntent::ExtractService, &["lib.rs"]);
        let impact = PlanGenerator::estimate_impact(&plan);
        assert_eq!(impact.files_affected.len(), 1);
        assert!(impact.risk_score > 0.0);
    }

    #[test]
    fn estimate_impact_high_risk_has_breaking_changes() {
        let plan = PlanGenerator::generate(&RefactorIntent::ExtractService, &["lib.rs"]);
        let impact = PlanGenerator::estimate_impact(&plan);
        assert!(!impact.breaking_changes.is_empty());
    }

    #[test]
    fn estimate_impact_low_risk_no_breaking() {
        let plan = PlanGenerator::generate(&RefactorIntent::AddLogging, &["lib.rs"]);
        let impact = PlanGenerator::estimate_impact(&plan);
        assert!(impact.breaking_changes.is_empty());
    }

    #[test]
    fn estimate_impact_test_file_names() {
        let plan = PlanGenerator::generate(&RefactorIntent::AddCaching, &["cache.rs", "app.ts"]);
        let impact = PlanGenerator::estimate_impact(&plan);
        assert!(impact.test_files.contains(&"cache_test.rs".to_string()));
        assert!(impact.test_files.contains(&"app.test.ts".to_string()));
    }

    #[test]
    fn plan_all_intents_produce_steps() {
        let intents = vec![
            RefactorIntent::MakeTestable,
            RefactorIntent::ReduceCoupling,
            RefactorIntent::ImprovePerformance,
            RefactorIntent::AddErrorHandling,
            RefactorIntent::ExtractService,
            RefactorIntent::ConsolidateDuplicates,
            RefactorIntent::ModernizeSyntax,
            RefactorIntent::AddTyping,
            RefactorIntent::SplitModule,
            RefactorIntent::MergeModules,
            RefactorIntent::AddCaching,
            RefactorIntent::AddLogging,
        ];
        for intent in intents {
            let plan = PlanGenerator::generate(&intent, &["test.rs"]);
            assert!(!plan.steps.is_empty(), "No steps for {:?}", intent);
            assert!(plan.estimated_impact > 0.0);
        }
    }

    #[test]
    fn estimated_impact_capped_at_one() {
        let files: Vec<&str> = (0..50).map(|_| "f.rs").collect();
        let plan = PlanGenerator::generate(&RefactorIntent::ExtractService, &files);
        assert!(plan.estimated_impact <= 1.0);
    }

    // -----------------------------------------------------------------------
    // EquivalenceChecker tests
    // -----------------------------------------------------------------------

    #[test]
    fn check_equivalent_rust() {
        let before = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}";
        let after = "pub fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}";
        assert_eq!(
            EquivalenceChecker::check(before, after),
            VerificationResult::Equivalent
        );
    }

    #[test]
    fn check_not_equivalent_missing_fn() {
        let before =
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }";
        let after = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        match EquivalenceChecker::check(before, after) {
            VerificationResult::NotEquivalent(msg) => assert!(msg.contains("sub")),
            other => panic!("Expected NotEquivalent, got {:?}", other),
        }
    }

    #[test]
    fn check_unknown_for_empty() {
        assert_eq!(
            EquivalenceChecker::check("", ""),
            VerificationResult::Unknown
        );
    }

    #[test]
    fn snapshot_api_rust() {
        let code = "pub fn foo() -> bool { true }\nfn private() {}\npub async fn bar(x: u32) -> String { x.to_string() }";
        let api = EquivalenceChecker::snapshot_api(code);
        assert_eq!(api.len(), 2);
        assert!(api[0].contains("foo"));
        assert!(api[1].contains("bar"));
    }

    #[test]
    fn snapshot_api_typescript() {
        let code = "export function greet(name: string): string {\n  return `Hello ${name}`;\n}\nfunction internal() {}";
        let api = EquivalenceChecker::snapshot_api(code);
        assert_eq!(api.len(), 1);
        assert!(api[0].contains("greet"));
    }

    #[test]
    fn snapshot_api_python() {
        let code =
            "def top_level(x, y):\n    return x + y\n\nclass Foo:\n    def method(self):\n        pass";
        let api = EquivalenceChecker::snapshot_api(code);
        assert_eq!(api.len(), 1);
        assert!(api[0].contains("top_level"));
    }

    #[test]
    fn snapshot_api_export_const() {
        let code = "export const MAX_SIZE = 100;";
        let api = EquivalenceChecker::snapshot_api(code);
        assert_eq!(api.len(), 1);
        assert!(api[0].contains("MAX_SIZE"));
    }

    // -----------------------------------------------------------------------
    // RefactorConfig tests
    // -----------------------------------------------------------------------

    #[test]
    fn default_config() {
        let cfg = RefactorConfig::default();
        assert!(cfg.auto_verify);
        assert!(cfg.rollback_on_failure);
        assert_eq!(cfg.max_steps, 20);
        assert!(cfg.require_tests);
    }

    // -----------------------------------------------------------------------
    // RefactorEngine tests
    // -----------------------------------------------------------------------

    #[test]
    fn engine_plan_creates_session() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        assert!(engine.get_session(&id).is_some());
    }

    #[test]
    fn engine_plan_returns_unique_ids() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id1 = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        let id2 = engine.plan(RefactorIntent::AddCaching, vec!["b.rs".into()]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn engine_execute_step_advances() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        let status = engine.execute_step(&id).expect("execute failed");
        assert_eq!(status, StepStatus::Completed);
        assert_eq!(engine.get_session(&id).expect("missing").current_step, 1);
    }

    #[test]
    fn engine_execute_all_steps() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        let step_count = engine.get_session(&id).expect("missing").plan.steps.len();
        for _ in 0..step_count {
            engine.execute_step(&id).expect("execute failed");
        }
        assert!(engine
            .get_session(&id)
            .expect("missing")
            .completed_at
            .is_some());
    }

    #[test]
    fn engine_execute_past_end_errors() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        let step_count = engine.get_session(&id).expect("missing").plan.steps.len();
        for _ in 0..step_count {
            engine.execute_step(&id).expect("execute failed");
        }
        assert!(engine.execute_step(&id).is_err());
    }

    #[test]
    fn engine_execute_unknown_session() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.execute_step("nonexistent").is_err());
    }

    #[test]
    fn engine_skip_step() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::MakeTestable, vec!["lib.rs".into()]);
        engine.skip_step(&id).expect("skip failed");
        let session = engine.get_session(&id).expect("missing");
        assert_eq!(session.plan.steps[0].status, StepStatus::Skipped);
        assert_eq!(session.current_step, 1);
    }

    #[test]
    fn engine_skip_past_end_errors() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        let step_count = engine.get_session(&id).expect("missing").plan.steps.len();
        for _ in 0..step_count {
            engine.skip_step(&id).expect("skip failed");
        }
        assert!(engine.skip_step(&id).is_err());
    }

    #[test]
    fn engine_skip_unknown_session() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.skip_step("bad-id").is_err());
    }

    #[test]
    fn engine_rollback_completed_steps() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        engine.execute_step(&id).expect("exec");
        engine.execute_step(&id).expect("exec");
        let rolled = engine.rollback(&id).expect("rollback");
        assert_eq!(rolled, 2);
        let session = engine.get_session(&id).expect("missing");
        assert_eq!(session.current_step, 0);
        assert!(session.completed_at.is_none());
    }

    #[test]
    fn engine_rollback_nothing_to_roll() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        let rolled = engine.rollback(&id).expect("rollback");
        assert_eq!(rolled, 0);
    }

    #[test]
    fn engine_rollback_unknown_session() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.rollback("nope").is_err());
    }

    #[test]
    fn engine_verify_step() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        engine.execute_step(&id).expect("exec");
        let result = engine.verify_step(&id, 0).expect("verify");
        // With empty before_snapshot and synthetic after, both have no API -> Unknown
        assert_eq!(result, VerificationResult::Unknown);
    }

    #[test]
    fn engine_verify_step_out_of_range() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        assert!(engine.verify_step(&id, 999).is_err());
    }

    #[test]
    fn engine_verify_step_no_after_snapshot() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["main.rs".into()]);
        // Step 0 is Planned, no after_snapshot
        let result = engine.verify_step(&id, 0).expect("verify");
        assert_eq!(result, VerificationResult::Skipped);
    }

    #[test]
    fn engine_verify_unknown_session() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.verify_step("nope", 0).is_err());
    }

    #[test]
    fn engine_list_sessions() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.list_sessions().is_empty());
        engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        engine.plan(RefactorIntent::AddCaching, vec!["b.rs".into()]);
        assert_eq!(engine.list_sessions().len(), 2);
    }

    #[test]
    fn engine_get_session_none() {
        let engine = RefactorEngine::new(RefactorConfig::default());
        assert!(engine.get_session("missing").is_none());
    }

    #[test]
    fn engine_metrics_initial() {
        let engine = RefactorEngine::new(RefactorConfig::default());
        let m = engine.get_metrics();
        assert_eq!(m.total_refactors, 0);
        assert_eq!(m.completed, 0);
    }

    #[test]
    fn engine_metrics_after_plan() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        assert_eq!(engine.get_metrics().total_refactors, 1);
    }

    #[test]
    fn engine_metrics_after_execution() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        let steps = engine.get_session(&id).expect("s").plan.steps.len();
        for _ in 0..steps {
            engine.execute_step(&id).expect("exec");
        }
        let m = engine.get_metrics();
        assert_eq!(m.completed, 1);
        assert_eq!(m.steps_executed, steps as u64);
        assert!(m.avg_steps_per_refactor > 0.0);
    }

    #[test]
    fn engine_metrics_after_rollback() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        engine.execute_step(&id).expect("exec");
        engine.rollback(&id).expect("rollback");
        assert!(engine.get_metrics().rolled_back > 0);
    }

    #[test]
    fn engine_max_steps_truncation() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            max_steps: 2,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::MakeTestable, vec!["lib.rs".into()]);
        let session = engine.get_session(&id).expect("s");
        assert!(session.plan.steps.len() <= 2);
    }

    #[test]
    fn engine_skip_completes_session() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            max_steps: 2,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        let steps = engine.get_session(&id).expect("s").plan.steps.len();
        for _ in 0..steps {
            engine.skip_step(&id).expect("skip");
        }
        assert!(engine
            .get_session(&id)
            .expect("s")
            .completed_at
            .is_some());
    }

    #[test]
    fn engine_auto_verify_with_api_match() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: true,
            rollback_on_failure: false,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        // Steps have empty before_snapshot so verification -> Unknown (not failure)
        let status = engine.execute_step(&id).expect("exec");
        assert_eq!(status, StepStatus::Completed);
    }

    #[test]
    fn engine_rollback_on_failure_config() {
        // With empty snapshots, verification is Unknown, not NotEquivalent,
        // so rollback_on_failure should not trigger.
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: true,
            rollback_on_failure: true,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        let status = engine.execute_step(&id).expect("exec");
        assert_eq!(status, StepStatus::Completed);
    }

    #[test]
    fn session_started_at_set() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        assert!(engine.get_session(&id).expect("s").started_at > 0);
    }

    #[test]
    fn plan_created_at_set() {
        let mut engine = RefactorEngine::new(RefactorConfig::default());
        let id = engine.plan(RefactorIntent::AddLogging, vec!["a.rs".into()]);
        assert!(engine.get_session(&id).expect("s").plan.created_at > 0);
    }

    #[test]
    fn step_ids_are_sequential() {
        let plan = PlanGenerator::generate(&RefactorIntent::MakeTestable, &["a.rs"]);
        for (i, step) in plan.steps.iter().enumerate() {
            assert_eq!(step.id, format!("step-{}", i + 1));
        }
    }

    #[test]
    fn step_initial_status_is_planned() {
        let plan = PlanGenerator::generate(&RefactorIntent::AddCaching, &["cache.rs"]);
        for step in &plan.steps {
            assert_eq!(step.status, StepStatus::Planned);
        }
    }

    #[test]
    fn step_initial_verification_is_skipped() {
        let plan = PlanGenerator::generate(&RefactorIntent::AddCaching, &["cache.rs"]);
        for step in &plan.steps {
            assert_eq!(step.verification, VerificationResult::Skipped);
        }
    }

    #[test]
    fn equivalence_checker_multiple_rust_fns() {
        let before = "pub fn a() {}\npub fn b() {}\npub fn c() {}";
        let after = "pub fn b() {}\npub fn a() {}\npub fn c() {}";
        assert_eq!(
            EquivalenceChecker::check(before, after),
            VerificationResult::Equivalent
        );
    }

    #[test]
    fn equivalence_checker_added_fn_is_equivalent() {
        // Adding a new function should not break equivalence (all old sigs still present)
        let before = "pub fn a() {}";
        let after = "pub fn a() {}\npub fn b() {}";
        assert_eq!(
            EquivalenceChecker::check(before, after),
            VerificationResult::Equivalent
        );
    }

    #[test]
    fn suggest_logging_for_code_without_logs() {
        let mut lines = Vec::new();
        for i in 0..60 {
            lines.push(format!("let x{} = {};", i, i));
        }
        let code = lines.join("\n");
        let suggestions = IntentParser::suggest_intents(&code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::AddLogging));
    }

    #[test]
    fn suggest_duplicates_for_repeated_lines() {
        let line = "let result = some_function_call(arg1, arg2, arg3);";
        let code = std::iter::repeat(line)
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        let suggestions = IntentParser::suggest_intents(&code);
        assert!(suggestions
            .iter()
            .any(|(i, _)| *i == RefactorIntent::ConsolidateDuplicates));
    }

    #[test]
    fn engine_mixed_execute_and_skip() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            max_steps: 4,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddErrorHandling, vec!["lib.rs".into()]);
        let steps = engine.get_session(&id).expect("s").plan.steps.len();
        for i in 0..steps {
            if i % 2 == 0 {
                engine.execute_step(&id).expect("exec");
            } else {
                engine.skip_step(&id).expect("skip");
            }
        }
        assert!(engine
            .get_session(&id)
            .expect("s")
            .completed_at
            .is_some());
    }

    #[test]
    fn engine_rollback_after_mixed_steps() {
        let mut engine = RefactorEngine::new(RefactorConfig {
            auto_verify: false,
            max_steps: 4,
            ..Default::default()
        });
        let id = engine.plan(RefactorIntent::AddErrorHandling, vec!["lib.rs".into()]);
        engine.execute_step(&id).expect("exec");
        engine.skip_step(&id).expect("skip");
        engine.execute_step(&id).expect("exec");
        let rolled = engine.rollback(&id).expect("rollback");
        // Only completed steps get rolled back, not skipped
        assert_eq!(rolled, 2);
    }

    #[test]
    fn parse_pull_out_as_extract_service() {
        assert_eq!(
            IntentParser::parse("pull out the auth logic"),
            Some(RefactorIntent::ExtractService)
        );
    }

    #[test]
    fn parse_break_apart_as_split_module() {
        assert_eq!(
            IntentParser::parse("break apart this file"),
            Some(RefactorIntent::SplitModule)
        );
    }

    #[test]
    fn parse_deduplicate() {
        assert_eq!(
            IntentParser::parse("deduplicate the helpers"),
            Some(RefactorIntent::ConsolidateDuplicates)
        );
    }
}
