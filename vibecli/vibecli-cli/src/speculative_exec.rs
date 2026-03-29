#![allow(dead_code)]
//! Speculative execution — fork ambiguous decisions into parallel branches, test, pick best.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionType {
    ArchitectureChoice,
    LibrarySelection,
    AlgorithmChoice,
    PatternChoice,
    APIDesign,
    ErrorStrategy,
    TestStrategy,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BranchOutcome {
    Pending,
    Running,
    Passed(f64),
    Failed(String),
    Timeout,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionStrategy {
    TestScore,
    SmallestDiff,
    LowestCost,
    Manual,
    Composite,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DecisionPoint {
    pub id: String,
    pub description: String,
    pub decision_type: DecisionType,
    pub options: Vec<String>,
    pub context: String,
    pub confidence: f64,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct SpeculativeBranch {
    pub id: String,
    pub decision_id: String,
    pub option_chosen: String,
    pub worktree_path: Option<String>,
    pub diff_size: usize,
    pub test_results: Option<TestResult>,
    pub outcome: BranchOutcome,
    pub cost_tokens: usize,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
}

impl TestResult {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }
}

#[derive(Debug, Clone)]
pub struct SpeculativeSession {
    pub id: String,
    pub decision: DecisionPoint,
    pub branches: Vec<SpeculativeBranch>,
    pub selected_branch: Option<String>,
    pub strategy: SelectionStrategy,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct SpecConfig {
    pub max_branches: usize,
    pub confidence_threshold: f64,
    pub timeout_secs: u64,
    pub auto_select: bool,
    pub run_tests: bool,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            max_branches: 4,
            confidence_threshold: 0.7,
            timeout_secs: 300,
            auto_select: true,
            run_tests: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecMetrics {
    pub total_sessions: u64,
    pub total_branches: u64,
    pub auto_selected: u64,
    pub manual_selected: u64,
    pub avg_branches_per_session: f64,
    pub time_saved_estimate_secs: f64,
    pub total_cost_tokens: u64,
}

impl Default for SpecMetrics {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            total_branches: 0,
            auto_selected: 0,
            manual_selected: 0,
            avg_branches_per_session: 0.0,
            time_saved_estimate_secs: 0.0,
            total_cost_tokens: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BranchComparison {
    pub branch_id: String,
    pub option: String,
    pub test_score: f64,
    pub diff_size: usize,
    pub cost: usize,
    pub duration_secs: f64,
    pub recommendation: bool,
}

// ---------------------------------------------------------------------------
// BranchScorer
// ---------------------------------------------------------------------------

pub struct BranchScorer;

impl BranchScorer {
    /// Composite score: test_pass_rate(0.5) + inv_diff_size(0.2) + inv_cost(0.15) + speed(0.15)
    pub fn score(branch: &SpeculativeBranch) -> f64 {
        let test_score = match &branch.test_results {
            Some(tr) => tr.pass_rate(),
            None => 0.5, // neutral when no tests
        };

        // Inverse diff size — smaller is better. Clamp to avoid div-by-zero.
        let diff_score = 1.0 / (1.0 + branch.diff_size as f64 / 100.0);

        // Inverse cost — fewer tokens is better.
        let cost_score = 1.0 / (1.0 + branch.cost_tokens as f64 / 1000.0);

        // Speed — shorter duration is better.
        let duration = match (branch.started_at, branch.completed_at) {
            (s, Some(c)) if c >= s => (c - s) as f64,
            _ => 300.0, // default penalty if not completed
        };
        let speed_score = 1.0 / (1.0 + duration / 60.0);

        test_score * 0.5 + diff_score * 0.2 + cost_score * 0.15 + speed_score * 0.15
    }

    /// Rank branches by score descending. Returns vec of (branch_id, score).
    pub fn rank(branches: &[SpeculativeBranch]) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = branches
            .iter()
            .map(|b| (b.id.clone(), Self::score(b)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

// ---------------------------------------------------------------------------
// SpeculativeEngine
// ---------------------------------------------------------------------------

pub struct SpeculativeEngine {
    pub config: SpecConfig,
    sessions: HashMap<String, SpeculativeSession>,
    metrics: SpecMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl SpeculativeEngine {
    pub fn new(config: SpecConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            metrics: SpecMetrics::default(),
            next_id: 1,
            timestamp_counter: 1,
        }
    }

    fn gen_id(&mut self, prefix: &str) -> String {
        let id = format!("{}-{}", prefix, self.next_id);
        self.next_id += 1;
        id
    }

    fn now(&mut self) -> u64 {
        let t = self.timestamp_counter;
        self.timestamp_counter += 1;
        t
    }

    /// Heuristic detection of decision points in prompts.
    /// Looks for "or", "alternatively", "option A/B", question marks in planning text.
    pub fn detect_decision_point(&mut self, prompt: &str) -> Option<DecisionPoint> {
        let lower = prompt.to_lowercase();

        // Check for decision indicators
        let has_or = lower.contains(" or ");
        let has_alternatively = lower.contains("alternatively");
        let has_option = lower.contains("option a") || lower.contains("option b")
            || lower.contains("option 1") || lower.contains("option 2");
        let has_question = lower.contains('?');
        let has_choice = lower.contains("choose between")
            || lower.contains("should we")
            || lower.contains("which approach")
            || lower.contains("which pattern")
            || lower.contains("either");

        if !(has_or || has_alternatively || has_option || has_choice) {
            return None;
        }

        // Determine decision type from content
        let decision_type = if lower.contains("architecture") || lower.contains("monolith")
            || lower.contains("microservice")
        {
            DecisionType::ArchitectureChoice
        } else if lower.contains("library") || lower.contains("crate")
            || lower.contains("package") || lower.contains("dependency")
        {
            DecisionType::LibrarySelection
        } else if lower.contains("algorithm") || lower.contains("sort")
            || lower.contains("search") || lower.contains("hash")
        {
            DecisionType::AlgorithmChoice
        } else if lower.contains("pattern") || lower.contains("design") {
            DecisionType::PatternChoice
        } else if lower.contains("api") || lower.contains("endpoint")
            || lower.contains("rest") || lower.contains("grpc")
        {
            DecisionType::APIDesign
        } else if lower.contains("error") || lower.contains("exception")
            || lower.contains("result") || lower.contains("panic")
        {
            DecisionType::ErrorStrategy
        } else if lower.contains("test") || lower.contains("spec")
            || lower.contains("assert")
        {
            DecisionType::TestStrategy
        } else {
            DecisionType::Custom("general".to_string())
        };

        // Extract options from the prompt (simple heuristic)
        let mut options = Vec::new();
        if lower.contains("option a") {
            options.push("Option A".to_string());
        }
        if lower.contains("option b") {
            options.push("Option B".to_string());
        }
        if lower.contains("option c") {
            options.push("Option C".to_string());
        }
        if lower.contains("option 1") {
            options.push("Option 1".to_string());
        }
        if lower.contains("option 2") {
            options.push("Option 2".to_string());
        }
        if lower.contains("option 3") {
            options.push("Option 3".to_string());
        }
        if options.is_empty() {
            // Fallback: split on " or "
            if has_or {
                let parts: Vec<&str> = lower.splitn(3, " or ").collect();
                if parts.len() >= 2 {
                    // Use last word(s) before " or " and first word(s) after
                    options.push("Approach A".to_string());
                    options.push("Approach B".to_string());
                }
            }
        }

        // Confidence: lower when more uncertainty signals are present
        let signals = [has_or, has_alternatively, has_option, has_question, has_choice];
        let signal_count = signals.iter().filter(|&&s| s).count();
        let confidence = (1.0 - signal_count as f64 * 0.15).max(0.1);

        let id = self.gen_id("dp");
        let ts = self.now();

        Some(DecisionPoint {
            id,
            description: prompt.chars().take(200).collect(),
            decision_type,
            options,
            context: prompt.to_string(),
            confidence,
            created_at: ts,
        })
    }

    pub fn create_session(
        &mut self,
        decision: DecisionPoint,
        strategy: SelectionStrategy,
    ) -> String {
        let id = self.gen_id("sess");
        let ts = self.now();
        let session = SpeculativeSession {
            id: id.clone(),
            decision,
            branches: Vec::new(),
            selected_branch: None,
            strategy,
            created_at: ts,
        };
        self.sessions.insert(id.clone(), session);
        self.metrics.total_sessions += 1;
        self.update_avg_branches();
        id
    }

    pub fn add_branch(
        &mut self,
        session_id: &str,
        option: &str,
    ) -> Result<String, String> {
        // Validate first with immutable borrow, then mutate
        {
            let session = self
                .sessions
                .get(session_id)
                .ok_or_else(|| format!("Session not found: {}", session_id))?;
            if session.branches.len() >= self.config.max_branches {
                return Err(format!(
                    "Maximum branches ({}) reached for session {}",
                    self.config.max_branches, session_id
                ));
            }
            if session.selected_branch.is_some() {
                return Err("Session already has a selected branch".to_string());
            }
        }

        let decision_id = self.sessions.get(session_id).expect("session verified above").decision.id.clone();
        let branch_id = self.gen_id("br");
        let ts = self.now();

        let branch = SpeculativeBranch {
            id: branch_id.clone(),
            decision_id,
            option_chosen: option.to_string(),
            worktree_path: None,
            diff_size: 0,
            test_results: None,
            outcome: BranchOutcome::Pending,
            cost_tokens: 0,
            started_at: ts,
            completed_at: None,
        };

        let session = self.sessions.get_mut(session_id).unwrap();
        session.branches.push(branch);
        self.metrics.total_branches += 1;
        self.update_avg_branches();
        Ok(branch_id)
    }

    pub fn start_branch(
        &mut self,
        session_id: &str,
        branch_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let branch = session
            .branches
            .iter_mut()
            .find(|b| b.id == branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        match &branch.outcome {
            BranchOutcome::Pending => {
                branch.outcome = BranchOutcome::Running;
                Ok(())
            }
            other => Err(format!(
                "Branch {} is not Pending (current: {:?})",
                branch_id, other
            )),
        }
    }

    pub fn complete_branch(
        &mut self,
        session_id: &str,
        branch_id: &str,
        diff_size: usize,
        test_result: Option<TestResult>,
        cost: usize,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let branch = session
            .branches
            .iter_mut()
            .find(|b| b.id == branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        let score = match &test_result {
            Some(tr) => tr.pass_rate(),
            None => 0.5,
        };

        branch.diff_size = diff_size;
        branch.test_results = test_result;
        branch.cost_tokens = cost;
        branch.completed_at = Some(self.timestamp_counter);
        self.timestamp_counter += 1;
        branch.outcome = BranchOutcome::Passed(score);

        self.metrics.total_cost_tokens += cost as u64;

        Ok(())
    }

    pub fn fail_branch(
        &mut self,
        session_id: &str,
        branch_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let branch = session
            .branches
            .iter_mut()
            .find(|b| b.id == branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        branch.outcome = BranchOutcome::Failed(reason.to_string());
        branch.completed_at = Some(self.timestamp_counter);
        self.timestamp_counter += 1;

        Ok(())
    }

    pub fn auto_select(&mut self, session_id: &str) -> Result<String, String> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        if session.selected_branch.is_some() {
            return Err("Session already has a selected branch".to_string());
        }

        let completed: Vec<&SpeculativeBranch> = session
            .branches
            .iter()
            .filter(|b| matches!(b.outcome, BranchOutcome::Passed(_)))
            .collect();

        if completed.is_empty() {
            return Err("No completed branches to select from".to_string());
        }

        let best_id = match &session.strategy {
            SelectionStrategy::TestScore => {
                let mut best: Option<(&str, f64)> = None;
                for b in &completed {
                    let score = b.test_results.as_ref().map(|t| t.pass_rate()).unwrap_or(0.0);
                    if best.is_none() || score > best.unwrap().1 {
                        best = Some((&b.id, score));
                    }
                }
                best.unwrap().0.to_string()
            }
            SelectionStrategy::SmallestDiff => {
                completed
                    .iter()
                    .min_by_key(|b| b.diff_size)
                    .unwrap()
                    .id
                    .clone()
            }
            SelectionStrategy::LowestCost => {
                completed
                    .iter()
                    .min_by_key(|b| b.cost_tokens)
                    .unwrap()
                    .id
                    .clone()
            }
            SelectionStrategy::Composite | SelectionStrategy::Manual => {
                // Composite uses BranchScorer
                let completed_owned: Vec<SpeculativeBranch> =
                    completed.into_iter().cloned().collect();
                let ranked = BranchScorer::rank(&completed_owned);
                ranked
                    .first()
                    .ok_or("No branches to rank")?
                    .0
                    .clone()
            }
        };

        let session = self.sessions.get_mut(session_id).unwrap();
        session.selected_branch = Some(best_id.clone());
        self.metrics.auto_selected += 1;

        // Estimate time saved: sum of all branch durations minus best branch duration
        let _best_duration = session
            .branches
            .iter()
            .find(|b| b.id == best_id)
            .and_then(|b| b.completed_at.map(|c| c.saturating_sub(b.started_at)))
            .unwrap_or(0) as f64;
        let total_serial: f64 = session
            .branches
            .iter()
            .filter_map(|b| b.completed_at.map(|c| c.saturating_sub(b.started_at) as f64))
            .sum();
        // Parallel execution saves (total_serial - max_single_branch) time
        let max_duration = session
            .branches
            .iter()
            .filter_map(|b| b.completed_at.map(|c| c.saturating_sub(b.started_at)))
            .max()
            .unwrap_or(0) as f64;
        let saved = (total_serial - max_duration).max(0.0);
        self.metrics.time_saved_estimate_secs += saved;

        Ok(best_id)
    }

    pub fn manual_select(
        &mut self,
        session_id: &str,
        branch_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        if session.selected_branch.is_some() {
            return Err("Session already has a selected branch".to_string());
        }

        let exists = session.branches.iter().any(|b| b.id == branch_id);
        if !exists {
            return Err(format!("Branch not found: {}", branch_id));
        }

        session.selected_branch = Some(branch_id.to_string());
        self.metrics.manual_selected += 1;
        Ok(())
    }

    pub fn compare_branches(
        &self,
        session_id: &str,
    ) -> Result<Vec<BranchComparison>, String> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let completed: Vec<&SpeculativeBranch> = session
            .branches
            .iter()
            .filter(|b| matches!(b.outcome, BranchOutcome::Passed(_)))
            .collect();

        if completed.is_empty() {
            return Err("No completed branches to compare".to_string());
        }

        // Rank to determine recommendation
        let completed_owned: Vec<SpeculativeBranch> =
            completed.iter().cloned().cloned().collect();
        let ranked = BranchScorer::rank(&completed_owned);
        let best_id = ranked.first().map(|(id, _)| id.clone());

        let comparisons = completed
            .iter()
            .map(|b| {
                let duration = match b.completed_at {
                    Some(c) => c.saturating_sub(b.started_at) as f64,
                    None => 0.0,
                };
                let test_score = b
                    .test_results
                    .as_ref()
                    .map(|t| t.pass_rate())
                    .unwrap_or(0.0);
                BranchComparison {
                    branch_id: b.id.clone(),
                    option: b.option_chosen.clone(),
                    test_score,
                    diff_size: b.diff_size,
                    cost: b.cost_tokens,
                    duration_secs: duration,
                    recommendation: best_id.as_deref() == Some(&b.id),
                }
            })
            .collect();

        Ok(comparisons)
    }

    pub fn get_session(&self, session_id: &str) -> Option<&SpeculativeSession> {
        self.sessions.get(session_id)
    }

    pub fn list_sessions(&self) -> Vec<&SpeculativeSession> {
        self.sessions.values().collect()
    }

    pub fn get_metrics(&self) -> &SpecMetrics {
        &self.metrics
    }

    fn update_avg_branches(&mut self) {
        if self.metrics.total_sessions == 0 {
            self.metrics.avg_branches_per_session = 0.0;
        } else {
            self.metrics.avg_branches_per_session =
                self.metrics.total_branches as f64 / self.metrics.total_sessions as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_engine() -> SpeculativeEngine {
        SpeculativeEngine::new(SpecConfig::default())
    }

    fn make_decision(desc: &str) -> DecisionPoint {
        DecisionPoint {
            id: "dp-test".to_string(),
            description: desc.to_string(),
            decision_type: DecisionType::ArchitectureChoice,
            options: vec!["A".to_string(), "B".to_string()],
            context: desc.to_string(),
            confidence: 0.5,
            created_at: 0,
        }
    }

    fn make_test_result(total: usize, passed: usize) -> TestResult {
        TestResult {
            total,
            passed,
            failed: total - passed,
            skipped: 0,
            duration_ms: 100,
        }
    }

    // -- SpecConfig defaults --

    #[test]
    fn test_config_defaults() {
        let c = SpecConfig::default();
        assert_eq!(c.max_branches, 4);
        assert!((c.confidence_threshold - 0.7).abs() < f64::EPSILON);
        assert_eq!(c.timeout_secs, 300);
        assert!(c.auto_select);
        assert!(c.run_tests);
    }

    // -- SpecMetrics defaults --

    #[test]
    fn test_metrics_defaults() {
        let m = SpecMetrics::default();
        assert_eq!(m.total_sessions, 0);
        assert_eq!(m.total_branches, 0);
        assert_eq!(m.auto_selected, 0);
        assert_eq!(m.manual_selected, 0);
        assert!((m.avg_branches_per_session - 0.0).abs() < f64::EPSILON);
        assert!((m.time_saved_estimate_secs - 0.0).abs() < f64::EPSILON);
        assert_eq!(m.total_cost_tokens, 0);
    }

    // -- TestResult --

    #[test]
    fn test_pass_rate_all_pass() {
        let tr = make_test_result(10, 10);
        assert!((tr.pass_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pass_rate_none_pass() {
        let tr = make_test_result(10, 0);
        assert!((tr.pass_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pass_rate_partial() {
        let tr = make_test_result(10, 7);
        assert!((tr.pass_rate() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pass_rate_zero_total() {
        let tr = TestResult {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_ms: 0,
        };
        assert!((tr.pass_rate() - 0.0).abs() < f64::EPSILON);
    }

    // -- DecisionType variants --

    #[test]
    fn test_decision_type_custom() {
        let dt = DecisionType::Custom("my-type".to_string());
        assert_eq!(dt, DecisionType::Custom("my-type".to_string()));
    }

    #[test]
    fn test_decision_type_clone() {
        let dt = DecisionType::ArchitectureChoice;
        let dt2 = dt.clone();
        assert_eq!(dt, dt2);
    }

    // -- BranchOutcome --

    #[test]
    fn test_branch_outcome_variants() {
        assert_eq!(BranchOutcome::Pending, BranchOutcome::Pending);
        assert_eq!(BranchOutcome::Running, BranchOutcome::Running);
        assert_eq!(BranchOutcome::Timeout, BranchOutcome::Timeout);
        assert_eq!(
            BranchOutcome::Failed("oops".into()),
            BranchOutcome::Failed("oops".into())
        );
        assert_eq!(BranchOutcome::Passed(0.9), BranchOutcome::Passed(0.9));
    }

    // -- Engine creation --

    #[test]
    fn test_engine_new() {
        let e = default_engine();
        assert_eq!(e.sessions.len(), 0);
        assert_eq!(e.metrics.total_sessions, 0);
    }

    // -- detect_decision_point --

    #[test]
    fn test_detect_or_keyword() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("Should we use REST or gRPC?");
        assert!(dp.is_some());
        let dp = dp.unwrap();
        assert_eq!(dp.decision_type, DecisionType::APIDesign);
    }

    #[test]
    fn test_detect_alternatively() {
        let mut e = default_engine();
        let dp = e.detect_decision_point(
            "We could use a HashMap. Alternatively we could use a BTreeMap.",
        );
        assert!(dp.is_some());
    }

    #[test]
    fn test_detect_option_ab() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("Option A: monolith. Option B: microservices.");
        assert!(dp.is_some());
        let dp = dp.unwrap();
        assert!(dp.options.contains(&"Option A".to_string()));
        assert!(dp.options.contains(&"Option B".to_string()));
    }

    #[test]
    fn test_detect_no_decision() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("Implement the login page with React.");
        assert!(dp.is_none());
    }

    #[test]
    fn test_detect_choose_between() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("Choose between quicksort or mergesort algorithm.");
        assert!(dp.is_some());
        assert_eq!(dp.unwrap().decision_type, DecisionType::AlgorithmChoice);
    }

    #[test]
    fn test_detect_library_selection() {
        let mut e = default_engine();
        let dp = e.detect_decision_point(
            "Should we use the reqwest crate or hyper crate as a dependency?",
        );
        assert!(dp.is_some());
        assert_eq!(dp.unwrap().decision_type, DecisionType::LibrarySelection);
    }

    #[test]
    fn test_detect_error_strategy() {
        let mut e = default_engine();
        let dp = e.detect_decision_point(
            "Should we use Result types or panic on errors?",
        );
        assert!(dp.is_some());
        assert_eq!(dp.unwrap().decision_type, DecisionType::ErrorStrategy);
    }

    #[test]
    fn test_detect_test_strategy() {
        let mut e = default_engine();
        let dp = e.detect_decision_point(
            "Should we use integration tests or unit test specs?",
        );
        assert!(dp.is_some());
        assert_eq!(dp.unwrap().decision_type, DecisionType::TestStrategy);
    }

    #[test]
    fn test_detect_pattern_choice() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("Which design pattern should we use or alternatively a builder?");
        assert!(dp.is_some());
        assert_eq!(dp.unwrap().decision_type, DecisionType::PatternChoice);
    }

    #[test]
    fn test_detect_confidence_lower_with_more_signals() {
        let mut e = default_engine();
        // Many signals: "or", "alternatively", "?"
        let dp1 = e
            .detect_decision_point("Use A or B? Alternatively C.")
            .unwrap();
        // Fewer signals: just "or"
        let dp2 = e
            .detect_decision_point("Use monolith or microservice architecture")
            .unwrap();
        assert!(dp1.confidence < dp2.confidence);
    }

    // -- create_session --

    #[test]
    fn test_create_session() {
        let mut e = default_engine();
        let d = make_decision("test");
        let id = e.create_session(d, SelectionStrategy::Composite);
        assert!(e.get_session(&id).is_some());
        assert_eq!(e.metrics.total_sessions, 1);
    }

    // -- add_branch --

    #[test]
    fn test_add_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "Option A");
        assert!(bid.is_ok());
        assert_eq!(e.metrics.total_branches, 1);
    }

    #[test]
    fn test_add_branch_max_exceeded() {
        let mut e = SpeculativeEngine::new(SpecConfig {
            max_branches: 2,
            ..SpecConfig::default()
        });
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        assert!(e.add_branch(&sid, "A").is_ok());
        assert!(e.add_branch(&sid, "B").is_ok());
        assert!(e.add_branch(&sid, "C").is_err());
    }

    #[test]
    fn test_add_branch_invalid_session() {
        let mut e = default_engine();
        assert!(e.add_branch("nonexistent", "A").is_err());
    }

    // -- start_branch --

    #[test]
    fn test_start_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        assert!(e.start_branch(&sid, &bid).is_ok());
        let branch = &e.get_session(&sid).unwrap().branches[0];
        assert_eq!(branch.outcome, BranchOutcome::Running);
    }

    #[test]
    fn test_start_branch_already_running() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.start_branch(&sid, &bid).unwrap();
        assert!(e.start_branch(&sid, &bid).is_err());
    }

    #[test]
    fn test_start_branch_invalid_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        assert!(e.start_branch(&sid, "nonexistent").is_err());
    }

    // -- complete_branch --

    #[test]
    fn test_complete_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.start_branch(&sid, &bid).unwrap();
        let tr = make_test_result(10, 9);
        assert!(e.complete_branch(&sid, &bid, 50, Some(tr), 1000).is_ok());
        let branch = &e.get_session(&sid).unwrap().branches[0];
        assert_eq!(branch.diff_size, 50);
        assert_eq!(branch.cost_tokens, 1000);
        assert!(matches!(branch.outcome, BranchOutcome::Passed(_)));
    }

    #[test]
    fn test_complete_branch_no_tests() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        assert!(e.complete_branch(&sid, &bid, 30, None, 500).is_ok());
        let branch = &e.get_session(&sid).unwrap().branches[0];
        assert!(matches!(branch.outcome, BranchOutcome::Passed(_)));
    }

    #[test]
    fn test_complete_branch_updates_cost_metric() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.complete_branch(&sid, &bid, 10, None, 2000).unwrap();
        assert_eq!(e.metrics.total_cost_tokens, 2000);
    }

    // -- fail_branch --

    #[test]
    fn test_fail_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.start_branch(&sid, &bid).unwrap();
        assert!(e.fail_branch(&sid, &bid, "compile error").is_ok());
        let branch = &e.get_session(&sid).unwrap().branches[0];
        assert_eq!(
            branch.outcome,
            BranchOutcome::Failed("compile error".to_string())
        );
        assert!(branch.completed_at.is_some());
    }

    // -- auto_select --

    #[test]
    fn test_auto_select_composite() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.complete_branch(&sid, &b1, 100, Some(make_test_result(10, 6)), 2000)
            .unwrap();
        e.complete_branch(&sid, &b2, 30, Some(make_test_result(10, 10)), 500)
            .unwrap();
        let best = e.auto_select(&sid).unwrap();
        // B should win: 100% tests, smaller diff, lower cost
        assert_eq!(best, b2);
        assert_eq!(e.metrics.auto_selected, 1);
    }

    #[test]
    fn test_auto_select_test_score() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::TestScore);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.complete_branch(&sid, &b1, 200, Some(make_test_result(10, 10)), 5000)
            .unwrap();
        e.complete_branch(&sid, &b2, 10, Some(make_test_result(10, 5)), 100)
            .unwrap();
        let best = e.auto_select(&sid).unwrap();
        assert_eq!(best, b1); // 100% tests wins even with bigger diff
    }

    #[test]
    fn test_auto_select_smallest_diff() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::SmallestDiff);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.complete_branch(&sid, &b1, 500, Some(make_test_result(10, 10)), 100)
            .unwrap();
        e.complete_branch(&sid, &b2, 20, Some(make_test_result(10, 5)), 100)
            .unwrap();
        let best = e.auto_select(&sid).unwrap();
        assert_eq!(best, b2);
    }

    #[test]
    fn test_auto_select_lowest_cost() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::LowestCost);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.complete_branch(&sid, &b1, 50, None, 5000).unwrap();
        e.complete_branch(&sid, &b2, 50, None, 100).unwrap();
        let best = e.auto_select(&sid).unwrap();
        assert_eq!(best, b2);
    }

    #[test]
    fn test_auto_select_no_completed() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.start_branch(&sid, &bid).unwrap();
        assert!(e.auto_select(&sid).is_err());
    }

    #[test]
    fn test_auto_select_already_selected() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.complete_branch(&sid, &bid, 10, None, 100).unwrap();
        e.auto_select(&sid).unwrap();
        assert!(e.auto_select(&sid).is_err());
    }

    // -- manual_select --

    #[test]
    fn test_manual_select() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Manual);
        let bid = e.add_branch(&sid, "A").unwrap();
        assert!(e.manual_select(&sid, &bid).is_ok());
        assert_eq!(e.get_session(&sid).unwrap().selected_branch, Some(bid));
        assert_eq!(e.metrics.manual_selected, 1);
    }

    #[test]
    fn test_manual_select_already_selected() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Manual);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.manual_select(&sid, &b1).unwrap();
        assert!(e.manual_select(&sid, &b2).is_err());
    }

    #[test]
    fn test_manual_select_invalid_branch() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Manual);
        assert!(e.manual_select(&sid, "nonexistent").is_err());
    }

    // -- compare_branches --

    #[test]
    fn test_compare_branches() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        let b1 = e.add_branch(&sid, "A").unwrap();
        let b2 = e.add_branch(&sid, "B").unwrap();
        e.complete_branch(&sid, &b1, 100, Some(make_test_result(10, 8)), 1000)
            .unwrap();
        e.complete_branch(&sid, &b2, 50, Some(make_test_result(10, 10)), 500)
            .unwrap();
        let comps = e.compare_branches(&sid).unwrap();
        assert_eq!(comps.len(), 2);
        // Exactly one should be recommended
        let recs: Vec<_> = comps.iter().filter(|c| c.recommendation).collect();
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn test_compare_no_completed() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Composite);
        e.add_branch(&sid, "A").unwrap();
        assert!(e.compare_branches(&sid).is_err());
    }

    // -- list_sessions --

    #[test]
    fn test_list_sessions() {
        let mut e = default_engine();
        assert_eq!(e.list_sessions().len(), 0);
        let d = make_decision("test");
        e.create_session(d, SelectionStrategy::Composite);
        assert_eq!(e.list_sessions().len(), 1);
    }

    // -- BranchScorer --

    #[test]
    fn test_scorer_perfect_branch() {
        let branch = SpeculativeBranch {
            id: "br-1".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "A".to_string(),
            worktree_path: None,
            diff_size: 0,
            test_results: Some(make_test_result(10, 10)),
            outcome: BranchOutcome::Passed(1.0),
            cost_tokens: 0,
            started_at: 0,
            completed_at: Some(1),
        };
        let score = BranchScorer::score(&branch);
        // High score expected: all tests pass, zero diff, zero cost, fast
        assert!(score > 0.8);
    }

    #[test]
    fn test_scorer_bad_branch() {
        let branch = SpeculativeBranch {
            id: "br-1".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "A".to_string(),
            worktree_path: None,
            diff_size: 10000,
            test_results: Some(make_test_result(10, 0)),
            outcome: BranchOutcome::Passed(0.0),
            cost_tokens: 100000,
            started_at: 0,
            completed_at: Some(600),
        };
        let score = BranchScorer::score(&branch);
        assert!(score < 0.2);
    }

    #[test]
    fn test_scorer_no_tests() {
        let branch = SpeculativeBranch {
            id: "br-1".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "A".to_string(),
            worktree_path: None,
            diff_size: 50,
            test_results: None,
            outcome: BranchOutcome::Passed(0.5),
            cost_tokens: 500,
            started_at: 0,
            completed_at: Some(10),
        };
        let score = BranchScorer::score(&branch);
        // neutral test score (0.5), should be moderate
        assert!(score > 0.3 && score < 0.7);
    }

    #[test]
    fn test_rank_ordering() {
        let good = SpeculativeBranch {
            id: "good".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "A".to_string(),
            worktree_path: None,
            diff_size: 10,
            test_results: Some(make_test_result(10, 10)),
            outcome: BranchOutcome::Passed(1.0),
            cost_tokens: 100,
            started_at: 0,
            completed_at: Some(5),
        };
        let bad = SpeculativeBranch {
            id: "bad".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "B".to_string(),
            worktree_path: None,
            diff_size: 5000,
            test_results: Some(make_test_result(10, 2)),
            outcome: BranchOutcome::Passed(0.2),
            cost_tokens: 50000,
            started_at: 0,
            completed_at: Some(300),
        };
        let ranked = BranchScorer::rank(&[good, bad]);
        assert_eq!(ranked[0].0, "good");
        assert_eq!(ranked[1].0, "bad");
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn test_rank_empty() {
        let ranked = BranchScorer::rank(&[]);
        assert!(ranked.is_empty());
    }

    // -- avg_branches metric --

    #[test]
    fn test_avg_branches_metric() {
        let mut e = default_engine();
        let d1 = make_decision("test1");
        let sid1 = e.create_session(d1, SelectionStrategy::Composite);
        e.add_branch(&sid1, "A").unwrap();
        e.add_branch(&sid1, "B").unwrap();
        // 2 branches / 1 session = 2.0
        assert!((e.metrics.avg_branches_per_session - 2.0).abs() < f64::EPSILON);

        let d2 = make_decision("test2");
        let sid2 = e.create_session(d2, SelectionStrategy::Composite);
        e.add_branch(&sid2, "X").unwrap();
        // 3 branches / 2 sessions = 1.5
        assert!((e.metrics.avg_branches_per_session - 1.5).abs() < f64::EPSILON);
    }

    // -- add_branch after selection rejected --

    #[test]
    fn test_add_branch_after_selection() {
        let mut e = default_engine();
        let d = make_decision("test");
        let sid = e.create_session(d, SelectionStrategy::Manual);
        let bid = e.add_branch(&sid, "A").unwrap();
        e.manual_select(&sid, &bid).unwrap();
        // Should not allow adding more branches after selection
        assert!(e.add_branch(&sid, "B").is_err());
    }

    // -- end-to-end workflow --

    #[test]
    fn test_full_workflow() {
        let mut e = default_engine();

        // Detect a decision
        let dp = e
            .detect_decision_point("Should we use Option A microservices or Option B monolith architecture?")
            .unwrap();
        assert_eq!(dp.decision_type, DecisionType::ArchitectureChoice);

        // Create session
        let sid = e.create_session(dp, SelectionStrategy::Composite);

        // Add branches
        let b1 = e.add_branch(&sid, "microservices").unwrap();
        let b2 = e.add_branch(&sid, "monolith").unwrap();

        // Start and complete branches
        e.start_branch(&sid, &b1).unwrap();
        e.start_branch(&sid, &b2).unwrap();

        e.complete_branch(&sid, &b1, 200, Some(make_test_result(20, 18)), 3000)
            .unwrap();
        e.complete_branch(&sid, &b2, 80, Some(make_test_result(15, 15)), 1500)
            .unwrap();

        // Compare
        let comps = e.compare_branches(&sid).unwrap();
        assert_eq!(comps.len(), 2);

        // Auto select
        let best = e.auto_select(&sid).unwrap();
        assert!(!best.is_empty());
        assert!(e.get_session(&sid).unwrap().selected_branch.is_some());

        // Check metrics
        let m = e.get_metrics();
        assert_eq!(m.total_sessions, 1);
        assert_eq!(m.total_branches, 2);
        assert_eq!(m.auto_selected, 1);
        assert_eq!(m.total_cost_tokens, 4500);
    }

    #[test]
    fn test_detect_numeric_options() {
        let mut e = default_engine();
        let dp = e
            .detect_decision_point("Option 1: use threads. Option 2: use async. Option 3: use processes.")
            .unwrap();
        assert!(dp.options.contains(&"Option 1".to_string()));
        assert!(dp.options.contains(&"Option 2".to_string()));
        assert!(dp.options.contains(&"Option 3".to_string()));
    }

    #[test]
    fn test_detect_either_keyword() {
        let mut e = default_engine();
        let dp = e.detect_decision_point("We can either use a HashMap or a BTreeMap.");
        assert!(dp.is_some());
    }

    #[test]
    fn test_scorer_incomplete_branch() {
        // Branch with no completed_at gets a speed penalty
        let branch = SpeculativeBranch {
            id: "br-1".to_string(),
            decision_id: "dp-1".to_string(),
            option_chosen: "A".to_string(),
            worktree_path: None,
            diff_size: 50,
            test_results: Some(make_test_result(10, 10)),
            outcome: BranchOutcome::Running,
            cost_tokens: 500,
            started_at: 0,
            completed_at: None,
        };
        let score = BranchScorer::score(&branch);
        // Should still have decent score from tests but speed penalty
        assert!(score > 0.3);
        assert!(score < 0.9);
    }

    #[test]
    fn test_multiple_sessions_isolation() {
        let mut e = default_engine();
        let d1 = make_decision("session 1");
        let d2 = make_decision("session 2");
        let s1 = e.create_session(d1, SelectionStrategy::Composite);
        let s2 = e.create_session(d2, SelectionStrategy::TestScore);

        let b1 = e.add_branch(&s1, "A").unwrap();
        let b2 = e.add_branch(&s2, "X").unwrap();

        // Branch from s1 should not be in s2
        assert_eq!(e.get_session(&s1).unwrap().branches.len(), 1);
        assert_eq!(e.get_session(&s2).unwrap().branches.len(), 1);
        assert_eq!(e.get_session(&s1).unwrap().branches[0].id, b1);
        assert_eq!(e.get_session(&s2).unwrap().branches[0].id, b2);
    }

    #[test]
    fn test_fail_branch_invalid_session() {
        let mut e = default_engine();
        assert!(e.fail_branch("nope", "br-1", "reason").is_err());
    }

    #[test]
    fn test_complete_branch_invalid_session() {
        let mut e = default_engine();
        assert!(e.complete_branch("nope", "br-1", 0, None, 0).is_err());
    }

    #[test]
    fn test_compare_branches_invalid_session() {
        let e = default_engine();
        assert!(e.compare_branches("nope").is_err());
    }

    #[test]
    fn test_selection_strategy_clone() {
        let s = SelectionStrategy::Composite;
        let s2 = s.clone();
        assert_eq!(s, s2);
    }

    #[test]
    fn test_branch_comparison_fields() {
        let bc = BranchComparison {
            branch_id: "br-1".to_string(),
            option: "A".to_string(),
            test_score: 0.95,
            diff_size: 42,
            cost: 1000,
            duration_secs: 5.0,
            recommendation: true,
        };
        assert_eq!(bc.branch_id, "br-1");
        assert!(bc.recommendation);
        assert_eq!(bc.diff_size, 42);
    }
}
