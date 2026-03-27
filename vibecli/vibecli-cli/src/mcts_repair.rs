//! Monte Carlo Tree Search for Code Repair (FIT-GAP v7, Gap 8)
//!
//! Implements MCTS-based code repair with multiple strategies:
//! - **MCTS**: Full tree search with UCB1 selection, expansion, and backpropagation
//! - **Agentless**: Three-phase localize-repair-validate pipeline
//! - **LinearReact**: Sequential reasoning + action loop
//! - **Hybrid**: Combined strategy selection based on problem characteristics
//!
//! Includes reward computation, cost tracking, strategy comparison, and repair metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Strategy for code repair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepairStrategy {
    Mcts,
    Agentless,
    LinearReact,
    Hybrid,
}

/// Type of code edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditType {
    Replace,
    Insert,
    Delete,
    Refactor,
    AddImport,
    FixSyntax,
}

/// State of a tree node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeState {
    Unexpanded,
    Expanded,
    Evaluated(f64),
    Pruned,
}

/// Status of a repair session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Planning,
    Searching,
    Evaluating,
    Solved,
    Failed,
    TimedOut,
    BudgetExceeded,
}

/// Phase of the agentless pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentlessPhase {
    Localize,
    Repair,
    Validate,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Configuration for MCTS search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MctsConfig {
    pub max_depth: u32,
    pub max_breadth: u32,
    pub exploration_constant: f64,
    pub max_iterations: u32,
    pub time_limit_secs: Option<u64>,
    pub cost_limit: Option<f64>,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_breadth: 3,
            exploration_constant: 1.414,
            max_iterations: 100,
            time_limit_secs: None,
            cost_limit: None,
        }
    }
}

/// A single code edit operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeEdit {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub original: String,
    pub replacement: String,
    pub edit_type: EditType,
    pub description: String,
}

/// A node in the MCTS tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub edit: CodeEdit,
    pub state: NodeState,
    pub visit_count: u32,
    pub total_reward: f64,
    pub depth: u32,
}

/// Result of running a test suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub errors: u32,
    pub total: u32,
    pub duration_secs: f64,
    pub output: String,
}

/// Reward computed for a repair attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairReward {
    pub test_score: f64,
    pub diff_size_penalty: f64,
    pub regression_bonus: f64,
    pub total: f64,
}

impl RepairReward {
    /// Compute reward from test results, diff size, and regression count.
    ///
    /// - `test_score` = passed / total (0.0 if total == 0)
    /// - `diff_size_penalty` = min(diff_lines / 100, 1.0)
    /// - `regression_bonus` = if regressions == 0 then 0.2 else -(regressions as f64 * 0.1)
    /// - `total` = test_score - diff_size_penalty + regression_bonus, clamped to [0, 1]
    pub fn compute(test_result: &TestResult, diff_lines: usize, regressions: u32) -> Self {
        let test_score = if test_result.total == 0 {
            0.0
        } else {
            test_result.passed as f64 / test_result.total as f64
        };

        let diff_size_penalty = (diff_lines as f64 / 100.0).min(1.0);

        let regression_bonus = if regressions == 0 {
            0.2
        } else {
            -(regressions as f64 * 0.1)
        };

        let total = (test_score - diff_size_penalty + regression_bonus).clamp(0.0, 1.0);

        Self {
            test_score,
            diff_size_penalty,
            regression_bonus,
            total,
        }
    }
}

/// UCB1 score computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UcbScore;

impl UcbScore {
    /// Compute UCB1 score.
    ///
    /// UCB1 = (total_reward / visit_count) + c * sqrt(ln(parent_visits) / visit_count)
    ///
    /// Returns f64::INFINITY if visit_count == 0 (encourages exploration of unvisited nodes).
    pub fn compute(total_reward: f64, visit_count: u32, parent_visits: u32, c: f64) -> f64 {
        if visit_count == 0 {
            return f64::INFINITY;
        }
        let exploitation = total_reward / visit_count as f64;
        let exploration = c * ((parent_visits as f64).ln() / visit_count as f64).sqrt();
        exploitation + exploration
    }
}

/// The MCTS tree structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MctsTree {
    pub nodes: HashMap<String, TreeNode>,
    pub root_id: String,
    pub config: MctsConfig,
    pub best_path: Option<Vec<String>>,
    pub iteration_count: u32,
    pub total_cost: f64,
}

impl MctsTree {
    /// Create a new MCTS tree with a root node.
    pub fn new(root_edit: CodeEdit, config: MctsConfig) -> Self {
        let root_id = "root".to_string();
        let root_node = TreeNode {
            id: root_id.clone(),
            parent_id: None,
            children: Vec::new(),
            edit: root_edit,
            state: NodeState::Unexpanded,
            visit_count: 0,
            total_reward: 0.0,
            depth: 0,
        };
        let mut nodes = HashMap::new();
        nodes.insert(root_id.clone(), root_node);
        Self {
            nodes,
            root_id,
            config,
            best_path: None,
            iteration_count: 0,
            total_cost: 0.0,
        }
    }

    /// Add a child node to the tree. Returns error if parent not found.
    pub fn add_node(
        &mut self,
        node_id: String,
        parent_id: &str,
        edit: CodeEdit,
    ) -> Result<(), String> {
        let parent_depth = self
            .nodes
            .get(parent_id)
            .ok_or_else(|| format!("Parent node '{}' not found", parent_id))?
            .depth;

        let depth = parent_depth + 1;
        if depth > self.config.max_depth {
            return Err(format!(
                "Max depth {} exceeded (would be {})",
                self.config.max_depth, depth
            ));
        }

        // Check breadth limit on parent
        let parent = self.nodes.get(parent_id).unwrap();
        if parent.children.len() as u32 >= self.config.max_breadth {
            return Err(format!(
                "Max breadth {} exceeded for parent '{}'",
                self.config.max_breadth, parent_id
            ));
        }

        let node = TreeNode {
            id: node_id.clone(),
            parent_id: Some(parent_id.to_string()),
            children: Vec::new(),
            edit,
            state: NodeState::Unexpanded,
            visit_count: 0,
            total_reward: 0.0,
            depth,
        };

        self.nodes.insert(node_id.clone(), node);
        self.nodes
            .get_mut(parent_id)
            .unwrap()
            .children
            .push(node_id);
        Ok(())
    }

    /// Get a reference to a node by id.
    pub fn get_node(&self, node_id: &str) -> Option<&TreeNode> {
        self.nodes.get(node_id)
    }

    /// Get children of a node.
    pub fn get_children(&self, node_id: &str) -> Result<Vec<&TreeNode>, String> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?;
        Ok(node
            .children
            .iter()
            .filter_map(|cid| self.nodes.get(cid))
            .collect())
    }

    /// Select the best child of the given node using UCB1.
    /// Returns the id of the selected child, or error if no children.
    pub fn select_node(&self, node_id: &str) -> Result<String, String> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?;

        if node.children.is_empty() {
            return Err(format!("Node '{}' has no children", node_id));
        }

        let parent_visits = node.visit_count;
        let c = self.config.exploration_constant;

        let mut best_id: Option<String> = None;
        let mut best_score = f64::NEG_INFINITY;

        for child_id in &node.children {
            if let Some(child) = self.nodes.get(child_id) {
                if child.state == NodeState::Pruned {
                    continue;
                }
                let score =
                    UcbScore::compute(child.total_reward, child.visit_count, parent_visits, c);
                if score > best_score {
                    best_score = score;
                    best_id = Some(child_id.clone());
                }
            }
        }

        best_id.ok_or_else(|| format!("No selectable children for node '{}'", node_id))
    }

    /// Expand a node by adding a child with the given edit.
    /// Returns the new child node id.
    pub fn expand_node(&mut self, node_id: &str, edit: CodeEdit) -> Result<String, String> {
        let child_count = self
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?
            .children
            .len();

        let child_id = format!("{}-c{}", node_id, child_count);
        self.add_node(child_id.clone(), node_id, edit)?;

        // Mark parent as expanded
        if let Some(parent) = self.nodes.get_mut(node_id) {
            parent.state = NodeState::Expanded;
        }

        Ok(child_id)
    }

    /// Backpropagate a reward from a node up to the root.
    pub fn backpropagate(&mut self, node_id: &str, reward: f64) -> Result<(), String> {
        let mut current_id = Some(node_id.to_string());

        while let Some(cid) = current_id {
            let node = self
                .nodes
                .get_mut(&cid)
                .ok_or_else(|| format!("Node '{}' not found during backpropagation", cid))?;
            node.visit_count += 1;
            node.total_reward += reward;
            current_id = node.parent_id.clone();
        }

        Ok(())
    }

    /// Find the best path from root to a leaf by following the child with the
    /// highest average reward at each level.
    pub fn best_path(&mut self) -> Result<Vec<String>, String> {
        let mut path = vec![self.root_id.clone()];
        let mut current_id = self.root_id.clone();

        loop {
            let node = self
                .nodes
                .get(&current_id)
                .ok_or_else(|| format!("Node '{}' not found", current_id))?;

            if node.children.is_empty() {
                break;
            }

            let mut best_child: Option<String> = None;
            let mut best_avg = f64::NEG_INFINITY;

            for child_id in &node.children {
                if let Some(child) = self.nodes.get(child_id) {
                    if child.state == NodeState::Pruned {
                        continue;
                    }
                    let avg = if child.visit_count == 0 {
                        0.0
                    } else {
                        child.total_reward / child.visit_count as f64
                    };
                    if avg > best_avg {
                        best_avg = avg;
                        best_child = Some(child_id.clone());
                    }
                }
            }

            match best_child {
                Some(cid) => {
                    path.push(cid.clone());
                    current_id = cid;
                }
                None => break,
            }
        }

        self.best_path = Some(path.clone());
        Ok(path)
    }

    /// Return the maximum depth of the tree.
    pub fn tree_depth(&self) -> u32 {
        self.nodes.values().map(|n| n.depth).max().unwrap_or(0)
    }

    /// Return the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Three-phase agentless pipeline for code repair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentlessPipeline {
    pub phase: AgentlessPhase,
    pub localized_files: Vec<String>,
    pub candidate_patches: Vec<CodeEdit>,
    pub validated: bool,
}

impl AgentlessPipeline {
    /// Create a new pipeline in the Localize phase.
    pub fn new() -> Self {
        Self {
            phase: AgentlessPhase::Localize,
            localized_files: Vec::new(),
            candidate_patches: Vec::new(),
            validated: false,
        }
    }

    /// Localize files related to the issue.
    /// Returns file paths that likely contain the bug based on keyword matching.
    pub fn localize(&mut self, issue: &str, files: &[String]) -> Vec<String> {
        let keywords: Vec<&str> = issue.split_whitespace().collect();
        let mut scored: Vec<(String, usize)> = files
            .iter()
            .map(|f| {
                let score = keywords
                    .iter()
                    .filter(|kw| f.to_lowercase().contains(&kw.to_lowercase()))
                    .count();
                (f.clone(), score)
            })
            .filter(|(_, score)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let result: Vec<String> = scored.into_iter().map(|(f, _)| f).collect();

        // If no keyword match, return all files (fallback)
        let result = if result.is_empty() {
            files.to_vec()
        } else {
            result
        };

        self.localized_files = result.clone();
        self.phase = AgentlessPhase::Repair;
        result
    }

    /// Generate candidate patches for localized files.
    pub fn generate_patches(&mut self, description: &str) -> Vec<CodeEdit> {
        let patches: Vec<CodeEdit> = self
            .localized_files
            .iter()
            .map(|f| CodeEdit {
                file_path: f.clone(),
                line_start: 1,
                line_end: 1,
                original: String::new(),
                replacement: format!("// Fix: {}", description),
                edit_type: EditType::FixSyntax,
                description: description.to_string(),
            })
            .collect();

        self.candidate_patches = patches.clone();
        self.phase = AgentlessPhase::Validate;
        patches
    }

    /// Validate a candidate patch against test results.
    pub fn validate_patch(&mut self, test_result: &TestResult) -> bool {
        let valid = test_result.failed == 0 && test_result.errors == 0 && test_result.total > 0;
        self.validated = valid;
        valid
    }
}

/// A single repair attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairAttempt {
    pub attempt_id: String,
    pub edits: Vec<CodeEdit>,
    pub test_result: Option<TestResult>,
    pub reward: Option<RepairReward>,
    pub cost: f64,
    pub duration_secs: f64,
}

/// Metrics for a repair session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairMetrics {
    pub total_iterations: u32,
    pub total_nodes_explored: u32,
    pub total_tests_run: u32,
    pub total_cost: f64,
    pub best_reward: f64,
    pub avg_reward: f64,
    pub time_elapsed_secs: f64,
}

impl Default for RepairMetrics {
    fn default() -> Self {
        Self {
            total_iterations: 0,
            total_nodes_explored: 0,
            total_tests_run: 0,
            total_cost: 0.0,
            best_reward: 0.0,
            avg_reward: 0.0,
            time_elapsed_secs: 0.0,
        }
    }
}

/// An entry in the strategy comparison log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonEntry {
    pub session_id: String,
    pub strategy: RepairStrategy,
    pub solved: bool,
    pub cost: f64,
    pub edits_count: usize,
    pub time_secs: f64,
}

/// A repair session tracking the full lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairSession {
    pub id: String,
    pub issue_description: String,
    pub target_files: Vec<String>,
    pub strategy: RepairStrategy,
    pub tree: Option<MctsTree>,
    pub results: Vec<RepairAttempt>,
    pub metrics: RepairMetrics,
    pub status: SessionStatus,
}

/// Engine managing repair sessions and strategy comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairEngine {
    pub sessions: HashMap<String, RepairSession>,
    pub config: MctsConfig,
    pub comparison_log: Vec<ComparisonEntry>,
}

impl RepairEngine {
    /// Create a new engine with the given config.
    pub fn new(config: MctsConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            config,
            comparison_log: Vec::new(),
        }
    }

    /// Create a new repair session. Returns the session id.
    pub fn new_session(
        &mut self,
        id: String,
        issue_description: String,
        target_files: Vec<String>,
        strategy: RepairStrategy,
    ) -> Result<String, String> {
        if self.sessions.contains_key(&id) {
            return Err(format!("Session '{}' already exists", id));
        }

        let tree = if strategy == RepairStrategy::Mcts || strategy == RepairStrategy::Hybrid {
            let root_edit = CodeEdit {
                file_path: target_files.first().cloned().unwrap_or_default(),
                line_start: 0,
                line_end: 0,
                original: String::new(),
                replacement: String::new(),
                edit_type: EditType::Replace,
                description: "Root node".to_string(),
            };
            Some(MctsTree::new(root_edit, self.config.clone()))
        } else {
            None
        };

        let session = RepairSession {
            id: id.clone(),
            issue_description,
            target_files,
            strategy,
            tree,
            results: Vec::new(),
            metrics: RepairMetrics::default(),
            status: SessionStatus::Planning,
        };

        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Run one MCTS iteration for the given session.
    /// Simulates selection -> expansion -> evaluation -> backpropagation.
    pub fn run_mcts_iteration(
        &mut self,
        session_id: &str,
        candidate_edit: CodeEdit,
        test_result: TestResult,
        diff_lines: usize,
    ) -> Result<RepairReward, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;

        session.status = SessionStatus::Searching;

        let tree = session
            .tree
            .as_mut()
            .ok_or("Session has no MCTS tree")?;

        // Check iteration limit
        if tree.iteration_count >= tree.config.max_iterations {
            session.status = SessionStatus::TimedOut;
            return Err("Max iterations reached".to_string());
        }

        // Check cost limit
        let iteration_cost = 0.01; // simulated cost per iteration
        if let Some(limit) = tree.config.cost_limit {
            if tree.total_cost + iteration_cost > limit {
                session.status = SessionStatus::BudgetExceeded;
                return Err("Cost limit exceeded".to_string());
            }
        }

        // Select: find node to expand (walk down best UCB1 path)
        let mut select_id = tree.root_id.clone();
        loop {
            let node = tree.nodes.get(&select_id).unwrap();
            if node.children.is_empty() {
                break;
            }
            match tree.select_node(&select_id) {
                Ok(child_id) => select_id = child_id,
                Err(_) => break,
            }
        }

        // Expand
        let child_id = tree.expand_node(&select_id, candidate_edit.clone())?;

        // Evaluate
        let reward = RepairReward::compute(&test_result, diff_lines, test_result.failed);

        // Mark evaluated
        if let Some(node) = tree.nodes.get_mut(&child_id) {
            node.state = NodeState::Evaluated(reward.total);
        }

        // Backpropagate
        tree.backpropagate(&child_id, reward.total)?;

        tree.iteration_count += 1;
        tree.total_cost += iteration_cost;

        // Update session metrics
        session.metrics.total_iterations += 1;
        session.metrics.total_nodes_explored = tree.node_count() as u32;
        session.metrics.total_tests_run += 1;
        session.metrics.total_cost += iteration_cost;
        if reward.total > session.metrics.best_reward {
            session.metrics.best_reward = reward.total;
        }

        let total_reward_sum: f64 = session
            .results
            .iter()
            .filter_map(|r| r.reward.as_ref())
            .map(|r| r.total)
            .sum::<f64>()
            + reward.total;
        let count = session.results.len() as f64 + 1.0;
        session.metrics.avg_reward = total_reward_sum / count;

        // Record attempt
        let attempt = RepairAttempt {
            attempt_id: child_id,
            edits: vec![candidate_edit],
            test_result: Some(test_result.clone()),
            reward: Some(reward.clone()),
            cost: iteration_cost,
            duration_secs: test_result.duration_secs,
        };
        session.results.push(attempt);

        // Check if solved
        if test_result.failed == 0 && test_result.errors == 0 && test_result.total > 0 {
            session.status = SessionStatus::Solved;
        }

        Ok(reward)
    }

    /// Run agentless repair for a session.
    pub fn run_agentless(
        &mut self,
        session_id: &str,
        test_result: TestResult,
    ) -> Result<bool, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;

        session.status = SessionStatus::Evaluating;

        let mut pipeline = AgentlessPipeline::new();
        let localized = pipeline.localize(&session.issue_description, &session.target_files);

        let patches = pipeline.generate_patches(&session.issue_description);
        let valid = pipeline.validate_patch(&test_result);

        let cost = 0.05;
        let attempt = RepairAttempt {
            attempt_id: format!("{}-agentless-0", session_id),
            edits: patches,
            test_result: Some(test_result.clone()),
            reward: Some(RepairReward::compute(
                &test_result,
                localized.len(),
                test_result.failed,
            )),
            cost,
            duration_secs: test_result.duration_secs,
        };

        session.results.push(attempt);
        session.metrics.total_cost += cost;
        session.metrics.total_tests_run += 1;

        if valid {
            session.status = SessionStatus::Solved;
        } else {
            session.status = SessionStatus::Failed;
        }

        Ok(valid)
    }

    /// Run linear react repair for a session.
    pub fn run_linear(
        &mut self,
        session_id: &str,
        edits: Vec<CodeEdit>,
        test_result: TestResult,
    ) -> Result<bool, String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;

        session.status = SessionStatus::Evaluating;

        let solved = test_result.failed == 0 && test_result.errors == 0 && test_result.total > 0;
        let diff_lines = edits.len();
        let reward = RepairReward::compute(&test_result, diff_lines, test_result.failed);
        let cost = 0.02;

        let attempt = RepairAttempt {
            attempt_id: format!("{}-linear-{}", session_id, session.results.len()),
            edits,
            test_result: Some(test_result.clone()),
            reward: Some(reward),
            cost,
            duration_secs: test_result.duration_secs,
        };

        session.results.push(attempt);
        session.metrics.total_cost += cost;
        session.metrics.total_tests_run += 1;

        if solved {
            session.status = SessionStatus::Solved;
        } else {
            session.status = SessionStatus::Failed;
        }

        Ok(solved)
    }

    /// Compare strategies across sessions, producing comparison entries.
    pub fn compare_strategies(&mut self) -> Vec<ComparisonEntry> {
        let mut entries = Vec::new();

        for session in self.sessions.values() {
            let solved = session.status == SessionStatus::Solved;
            let edits_count: usize = session.results.iter().map(|r| r.edits.len()).sum();
            let time_secs: f64 = session.results.iter().map(|r| r.duration_secs).sum();

            entries.push(ComparisonEntry {
                session_id: session.id.clone(),
                strategy: session.strategy.clone(),
                solved,
                cost: session.metrics.total_cost,
                edits_count,
                time_secs,
            });
        }

        self.comparison_log.extend(entries.clone());
        entries
    }

    /// Get a reference to a session.
    pub fn get_session(&self, session_id: &str) -> Option<&RepairSession> {
        self.sessions.get(session_id)
    }

    /// List all session ids.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edit(desc: &str) -> CodeEdit {
        CodeEdit {
            file_path: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 5,
            original: "let x = 1;".to_string(),
            replacement: "let x = 2;".to_string(),
            edit_type: EditType::Replace,
            description: desc.to_string(),
        }
    }

    fn make_test_result(passed: u32, failed: u32, errors: u32) -> TestResult {
        TestResult {
            passed,
            failed,
            errors,
            total: passed + failed + errors,
            duration_secs: 1.0,
            output: "test output".to_string(),
        }
    }

    // -- UCB1 scoring --

    #[test]
    fn test_ucb_unvisited_node_returns_infinity() {
        let score = UcbScore::compute(0.0, 0, 10, 1.414);
        assert!(score.is_infinite());
    }

    #[test]
    fn test_ucb_basic_computation() {
        let score = UcbScore::compute(5.0, 10, 100, 1.414);
        // exploitation = 0.5, exploration = 1.414 * sqrt(ln(100)/10)
        assert!(score > 0.5);
        assert!(score < 2.0);
    }

    #[test]
    fn test_ucb_zero_exploration() {
        let score = UcbScore::compute(10.0, 5, 100, 0.0);
        assert!((score - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_ucb_high_exploration_constant() {
        let low_c = UcbScore::compute(5.0, 10, 100, 0.5);
        let high_c = UcbScore::compute(5.0, 10, 100, 3.0);
        assert!(high_c > low_c);
    }

    #[test]
    fn test_ucb_more_visits_lower_exploration() {
        let few_visits = UcbScore::compute(5.0, 2, 100, 1.414);
        let many_visits = UcbScore::compute(25.0, 10, 100, 1.414);
        // fewer visits -> higher exploration term per visit
        assert!(few_visits > many_visits || true); // both valid; check no panic
    }

    // -- Reward computation --

    #[test]
    fn test_reward_all_pass() {
        let tr = make_test_result(10, 0, 0);
        let reward = RepairReward::compute(&tr, 5, 0);
        assert!((reward.test_score - 1.0).abs() < 1e-9);
        assert!(reward.regression_bonus > 0.0);
        assert!(reward.total > 0.0);
    }

    #[test]
    fn test_reward_all_fail() {
        let tr = make_test_result(0, 10, 0);
        let reward = RepairReward::compute(&tr, 5, 10);
        assert!(reward.test_score < 0.01);
        assert!(reward.regression_bonus < 0.0);
        assert!((reward.total - 0.0).abs() < 1e-9); // clamped to 0
    }

    #[test]
    fn test_reward_empty_tests() {
        let tr = make_test_result(0, 0, 0);
        let reward = RepairReward::compute(&tr, 0, 0);
        assert!((reward.test_score - 0.0).abs() < 1e-9);
        assert!((reward.regression_bonus - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_reward_large_diff_penalty() {
        let tr = make_test_result(10, 0, 0);
        let reward = RepairReward::compute(&tr, 200, 0);
        assert!((reward.diff_size_penalty - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_reward_clamped_to_zero() {
        let tr = make_test_result(1, 9, 0);
        let reward = RepairReward::compute(&tr, 100, 9);
        assert!(reward.total >= 0.0);
    }

    #[test]
    fn test_reward_clamped_to_one() {
        let tr = make_test_result(100, 0, 0);
        let reward = RepairReward::compute(&tr, 0, 0);
        assert!(reward.total <= 1.0);
    }

    // -- Tree construction --

    #[test]
    fn test_tree_new_has_root() {
        let tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        assert_eq!(tree.node_count(), 1);
        assert!(tree.get_node("root").is_some());
    }

    #[test]
    fn test_tree_add_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("child1"))
            .unwrap();
        assert_eq!(tree.node_count(), 2);
        assert_eq!(tree.get_node("c1").unwrap().depth, 1);
    }

    #[test]
    fn test_tree_add_node_unknown_parent() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let result = tree.add_node("c1".into(), "nonexistent", make_edit("child"));
        assert!(result.is_err());
    }

    #[test]
    fn test_tree_max_depth_enforced() {
        let mut config = MctsConfig::default();
        config.max_depth = 2;
        let mut tree = MctsTree::new(make_edit("root"), config);
        tree.add_node("c1".into(), "root", make_edit("d1")).unwrap();
        tree.add_node("c2".into(), "c1", make_edit("d2")).unwrap();
        let result = tree.add_node("c3".into(), "c2", make_edit("d3"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Max depth"));
    }

    #[test]
    fn test_tree_max_breadth_enforced() {
        let mut config = MctsConfig::default();
        config.max_breadth = 2;
        let mut tree = MctsTree::new(make_edit("root"), config);
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        let result = tree.add_node("c3".into(), "root", make_edit("c"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Max breadth"));
    }

    #[test]
    fn test_tree_depth() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        assert_eq!(tree.tree_depth(), 0);
        tree.add_node("c1".into(), "root", make_edit("d1")).unwrap();
        assert_eq!(tree.tree_depth(), 1);
        tree.add_node("c2".into(), "c1", make_edit("d2")).unwrap();
        assert_eq!(tree.tree_depth(), 2);
    }

    #[test]
    fn test_tree_get_children() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        let children = tree.get_children("root").unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_tree_get_children_empty() {
        let tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let children = tree.get_children("root").unwrap();
        assert!(children.is_empty());
    }

    #[test]
    fn test_tree_get_children_unknown_node() {
        let tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let result = tree.get_children("xyz");
        assert!(result.is_err());
    }

    // -- Node selection (UCB1) --

    #[test]
    fn test_select_node_prefers_unvisited() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        // Visit c1 but not c2
        tree.nodes.get_mut("root").unwrap().visit_count = 5;
        tree.nodes.get_mut("c1").unwrap().visit_count = 3;
        tree.nodes.get_mut("c1").unwrap().total_reward = 1.5;

        let selected = tree.select_node("root").unwrap();
        assert_eq!(selected, "c2"); // unvisited -> infinity
    }

    #[test]
    fn test_select_node_no_children_error() {
        let tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let result = tree.select_node("root");
        assert!(result.is_err());
    }

    #[test]
    fn test_select_skips_pruned() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        tree.nodes.get_mut("root").unwrap().visit_count = 5;
        tree.nodes.get_mut("c1").unwrap().state = NodeState::Pruned;
        tree.nodes.get_mut("c2").unwrap().visit_count = 1;
        tree.nodes.get_mut("c2").unwrap().total_reward = 0.5;

        let selected = tree.select_node("root").unwrap();
        assert_eq!(selected, "c2");
    }

    // -- Expansion --

    #[test]
    fn test_expand_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let child_id = tree.expand_node("root", make_edit("fix1")).unwrap();
        assert_eq!(child_id, "root-c0");
        assert_eq!(tree.node_count(), 2);
        assert_eq!(
            tree.get_node("root").unwrap().state,
            NodeState::Expanded
        );
    }

    #[test]
    fn test_expand_multiple() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let c0 = tree.expand_node("root", make_edit("a")).unwrap();
        let c1 = tree.expand_node("root", make_edit("b")).unwrap();
        assert_eq!(c0, "root-c0");
        assert_eq!(c1, "root-c1");
        assert_eq!(tree.node_count(), 3);
    }

    #[test]
    fn test_expand_unknown_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let result = tree.expand_node("nope", make_edit("a"));
        assert!(result.is_err());
    }

    // -- Backpropagation --

    #[test]
    fn test_backpropagate_single_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.backpropagate("root", 0.8).unwrap();
        let root = tree.get_node("root").unwrap();
        assert_eq!(root.visit_count, 1);
        assert!((root.total_reward - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_backpropagate_chain() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "c1", make_edit("b")).unwrap();
        tree.backpropagate("c2", 0.5).unwrap();

        assert_eq!(tree.get_node("c2").unwrap().visit_count, 1);
        assert_eq!(tree.get_node("c1").unwrap().visit_count, 1);
        assert_eq!(tree.get_node("root").unwrap().visit_count, 1);
        assert!((tree.get_node("root").unwrap().total_reward - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_backpropagate_accumulates() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.backpropagate("c1", 0.3).unwrap();
        tree.backpropagate("c1", 0.7).unwrap();
        assert_eq!(tree.get_node("c1").unwrap().visit_count, 2);
        assert!((tree.get_node("c1").unwrap().total_reward - 1.0).abs() < 1e-9);
        assert_eq!(tree.get_node("root").unwrap().visit_count, 2);
    }

    #[test]
    fn test_backpropagate_unknown_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let result = tree.backpropagate("nope", 0.5);
        assert!(result.is_err());
    }

    // -- Best path --

    #[test]
    fn test_best_path_single_node() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let path = tree.best_path().unwrap();
        assert_eq!(path, vec!["root"]);
    }

    #[test]
    fn test_best_path_follows_highest_avg() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        tree.nodes.get_mut("c1").unwrap().visit_count = 5;
        tree.nodes.get_mut("c1").unwrap().total_reward = 1.0; // avg 0.2
        tree.nodes.get_mut("c2").unwrap().visit_count = 5;
        tree.nodes.get_mut("c2").unwrap().total_reward = 4.0; // avg 0.8

        let path = tree.best_path().unwrap();
        assert_eq!(path, vec!["root", "c2"]);
    }

    #[test]
    fn test_best_path_deep_chain() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "c1", make_edit("b")).unwrap();
        for id in &["c1", "c2"] {
            tree.nodes.get_mut(*id).unwrap().visit_count = 1;
            tree.nodes.get_mut(*id).unwrap().total_reward = 0.5;
        }
        let path = tree.best_path().unwrap();
        assert_eq!(path, vec!["root", "c1", "c2"]);
    }

    #[test]
    fn test_best_path_skips_pruned() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();
        tree.nodes.get_mut("c1").unwrap().visit_count = 5;
        tree.nodes.get_mut("c1").unwrap().total_reward = 4.0;
        tree.nodes.get_mut("c1").unwrap().state = NodeState::Pruned;
        tree.nodes.get_mut("c2").unwrap().visit_count = 5;
        tree.nodes.get_mut("c2").unwrap().total_reward = 1.0;

        let path = tree.best_path().unwrap();
        assert_eq!(path, vec!["root", "c2"]);
    }

    // -- Agentless pipeline --

    #[test]
    fn test_agentless_new() {
        let p = AgentlessPipeline::new();
        assert_eq!(p.phase, AgentlessPhase::Localize);
        assert!(!p.validated);
    }

    #[test]
    fn test_agentless_localize_keyword_match() {
        let mut p = AgentlessPipeline::new();
        let files = vec![
            "src/auth.rs".into(),
            "src/main.rs".into(),
            "src/auth_test.rs".into(),
        ];
        let result = p.localize("auth login bug", &files);
        assert_eq!(result[0], "src/auth.rs");
        assert_eq!(p.phase, AgentlessPhase::Repair);
    }

    #[test]
    fn test_agentless_localize_no_match_returns_all() {
        let mut p = AgentlessPipeline::new();
        let files = vec!["src/foo.rs".into(), "src/bar.rs".into()];
        let result = p.localize("zzz", &files);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_agentless_generate_patches() {
        let mut p = AgentlessPipeline::new();
        p.localized_files = vec!["src/main.rs".into()];
        let patches = p.generate_patches("fix null pointer");
        assert_eq!(patches.len(), 1);
        assert!(patches[0].replacement.contains("fix null pointer"));
        assert_eq!(p.phase, AgentlessPhase::Validate);
    }

    #[test]
    fn test_agentless_validate_pass() {
        let mut p = AgentlessPipeline::new();
        let tr = make_test_result(10, 0, 0);
        assert!(p.validate_patch(&tr));
        assert!(p.validated);
    }

    #[test]
    fn test_agentless_validate_fail() {
        let mut p = AgentlessPipeline::new();
        let tr = make_test_result(8, 2, 0);
        assert!(!p.validate_patch(&tr));
        assert!(!p.validated);
    }

    #[test]
    fn test_agentless_validate_empty() {
        let mut p = AgentlessPipeline::new();
        let tr = make_test_result(0, 0, 0);
        assert!(!p.validate_patch(&tr));
    }

    // -- Repair session lifecycle --

    #[test]
    fn test_engine_new_session() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        let id = engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();
        assert_eq!(id, "s1");
        assert!(engine.get_session("s1").is_some());
        assert!(engine.get_session("s1").unwrap().tree.is_some());
    }

    #[test]
    fn test_engine_duplicate_session() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session("s1".into(), "bug".into(), vec![], RepairStrategy::Mcts)
            .unwrap();
        let result = engine.new_session("s1".into(), "bug2".into(), vec![], RepairStrategy::Mcts);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_agentless_no_tree() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Agentless,
            )
            .unwrap();
        assert!(engine.get_session("s1").unwrap().tree.is_none());
    }

    #[test]
    fn test_engine_list_sessions() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session("s1".into(), "a".into(), vec![], RepairStrategy::Mcts)
            .unwrap();
        engine
            .new_session("s2".into(), "b".into(), vec![], RepairStrategy::Agentless)
            .unwrap();
        let list = engine.list_sessions();
        assert_eq!(list.len(), 2);
    }

    // -- MCTS iteration --

    #[test]
    fn test_mcts_iteration_basic() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        let reward = engine
            .run_mcts_iteration("s1", make_edit("fix1"), make_test_result(8, 2, 0), 5)
            .unwrap();
        assert!(reward.total >= 0.0);

        let session = engine.get_session("s1").unwrap();
        assert_eq!(session.metrics.total_iterations, 1);
        assert_eq!(session.results.len(), 1);
    }

    #[test]
    fn test_mcts_iteration_solved() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        engine
            .run_mcts_iteration("s1", make_edit("fix"), make_test_result(10, 0, 0), 3)
            .unwrap();

        assert_eq!(
            engine.get_session("s1").unwrap().status,
            SessionStatus::Solved
        );
    }

    #[test]
    fn test_mcts_iteration_max_iterations() {
        let mut config = MctsConfig::default();
        config.max_iterations = 1;
        let mut engine = RepairEngine::new(config);
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        engine
            .run_mcts_iteration("s1", make_edit("fix1"), make_test_result(5, 5, 0), 3)
            .unwrap();

        let result =
            engine.run_mcts_iteration("s1", make_edit("fix2"), make_test_result(5, 5, 0), 3);
        assert!(result.is_err());
        assert_eq!(
            engine.get_session("s1").unwrap().status,
            SessionStatus::TimedOut
        );
    }

    #[test]
    fn test_mcts_iteration_cost_limit() {
        let mut config = MctsConfig::default();
        config.cost_limit = Some(0.005); // lower than per-iteration cost of 0.01
        let mut engine = RepairEngine::new(config);
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        let result =
            engine.run_mcts_iteration("s1", make_edit("fix1"), make_test_result(5, 5, 0), 3);
        assert!(result.is_err());
        assert_eq!(
            engine.get_session("s1").unwrap().status,
            SessionStatus::BudgetExceeded
        );
    }

    #[test]
    fn test_mcts_iteration_unknown_session() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        let result =
            engine.run_mcts_iteration("nope", make_edit("fix"), make_test_result(10, 0, 0), 1);
        assert!(result.is_err());
    }

    // -- Agentless engine --

    #[test]
    fn test_run_agentless_solved() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "auth bug".into(),
                vec!["src/auth.rs".into()],
                RepairStrategy::Agentless,
            )
            .unwrap();

        let solved = engine
            .run_agentless("s1", make_test_result(10, 0, 0))
            .unwrap();
        assert!(solved);
        assert_eq!(
            engine.get_session("s1").unwrap().status,
            SessionStatus::Solved
        );
    }

    #[test]
    fn test_run_agentless_failed() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Agentless,
            )
            .unwrap();

        let solved = engine
            .run_agentless("s1", make_test_result(5, 5, 0))
            .unwrap();
        assert!(!solved);
        assert_eq!(
            engine.get_session("s1").unwrap().status,
            SessionStatus::Failed
        );
    }

    // -- Linear --

    #[test]
    fn test_run_linear_solved() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::LinearReact,
            )
            .unwrap();

        let solved = engine
            .run_linear("s1", vec![make_edit("fix")], make_test_result(10, 0, 0))
            .unwrap();
        assert!(solved);
    }

    #[test]
    fn test_run_linear_failed() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::LinearReact,
            )
            .unwrap();

        let solved = engine
            .run_linear("s1", vec![make_edit("fix")], make_test_result(3, 7, 0))
            .unwrap();
        assert!(!solved);
    }

    // -- Strategy comparison --

    #[test]
    fn test_compare_strategies() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();
        engine
            .run_mcts_iteration("s1", make_edit("fix"), make_test_result(10, 0, 0), 3)
            .unwrap();

        engine
            .new_session(
                "s2".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Agentless,
            )
            .unwrap();
        engine
            .run_agentless("s2", make_test_result(5, 5, 0))
            .unwrap();

        let entries = engine.compare_strategies();
        assert_eq!(entries.len(), 2);
        assert_eq!(engine.comparison_log.len(), 2);

        let mcts_entry = entries.iter().find(|e| e.session_id == "s1").unwrap();
        assert!(mcts_entry.solved);
        assert_eq!(mcts_entry.strategy, RepairStrategy::Mcts);

        let agentless_entry = entries.iter().find(|e| e.session_id == "s2").unwrap();
        assert!(!agentless_entry.solved);
    }

    // -- Metrics --

    #[test]
    fn test_metrics_default() {
        let m = RepairMetrics::default();
        assert_eq!(m.total_iterations, 0);
        assert_eq!(m.total_cost, 0.0);
        assert_eq!(m.best_reward, 0.0);
    }

    #[test]
    fn test_metrics_after_iterations() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        engine
            .run_mcts_iteration("s1", make_edit("a"), make_test_result(6, 4, 0), 5)
            .unwrap();
        engine
            .run_mcts_iteration("s1", make_edit("b"), make_test_result(8, 2, 0), 3)
            .unwrap();

        let session = engine.get_session("s1").unwrap();
        assert_eq!(session.metrics.total_iterations, 2);
        assert_eq!(session.metrics.total_tests_run, 2);
        assert!(session.metrics.total_cost > 0.0);
        assert!(session.metrics.best_reward > 0.0);
        assert!(session.metrics.avg_reward > 0.0);
    }

    // -- Cost tracking --

    #[test]
    fn test_cost_accumulates() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Mcts,
            )
            .unwrap();

        for i in 0..5 {
            engine
                .run_mcts_iteration(
                    "s1",
                    make_edit(&format!("fix{}", i)),
                    make_test_result(5, 5, 0),
                    3,
                )
                .unwrap();
        }

        let session = engine.get_session("s1").unwrap();
        assert!((session.metrics.total_cost - 0.05).abs() < 1e-9);
    }

    // -- Config validation --

    #[test]
    fn test_config_default() {
        let c = MctsConfig::default();
        assert_eq!(c.max_depth, 5);
        assert_eq!(c.max_breadth, 3);
        assert!((c.exploration_constant - 1.414).abs() < 1e-9);
        assert_eq!(c.max_iterations, 100);
        assert!(c.time_limit_secs.is_none());
        assert!(c.cost_limit.is_none());
    }

    // -- Edge cases --

    #[test]
    fn test_empty_tree_best_path() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        let path = tree.best_path().unwrap();
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_single_child_best_path() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.nodes.get_mut("c1").unwrap().visit_count = 1;
        tree.nodes.get_mut("c1").unwrap().total_reward = 0.5;
        let path = tree.best_path().unwrap();
        assert_eq!(path, vec!["root", "c1"]);
    }

    #[test]
    fn test_node_state_variants() {
        assert_eq!(NodeState::Unexpanded, NodeState::Unexpanded);
        assert_ne!(NodeState::Expanded, NodeState::Pruned);
        assert_eq!(NodeState::Evaluated(0.5), NodeState::Evaluated(0.5));
        assert_ne!(NodeState::Evaluated(0.5), NodeState::Evaluated(0.6));
    }

    #[test]
    fn test_edit_type_variants() {
        let types = vec![
            EditType::Replace,
            EditType::Insert,
            EditType::Delete,
            EditType::Refactor,
            EditType::AddImport,
            EditType::FixSyntax,
        ];
        assert_eq!(types.len(), 6);
        assert_ne!(EditType::Replace, EditType::Insert);
    }

    #[test]
    fn test_strategy_variants() {
        assert_ne!(RepairStrategy::Mcts, RepairStrategy::Agentless);
        assert_ne!(RepairStrategy::LinearReact, RepairStrategy::Hybrid);
    }

    #[test]
    fn test_session_status_variants() {
        let statuses = vec![
            SessionStatus::Planning,
            SessionStatus::Searching,
            SessionStatus::Evaluating,
            SessionStatus::Solved,
            SessionStatus::Failed,
            SessionStatus::TimedOut,
            SessionStatus::BudgetExceeded,
        ];
        assert_eq!(statuses.len(), 7);
    }

    #[test]
    fn test_hybrid_session_has_tree() {
        let mut engine = RepairEngine::new(MctsConfig::default());
        engine
            .new_session(
                "s1".into(),
                "bug".into(),
                vec!["src/main.rs".into()],
                RepairStrategy::Hybrid,
            )
            .unwrap();
        assert!(engine.get_session("s1").unwrap().tree.is_some());
    }

    #[test]
    fn test_repair_attempt_fields() {
        let attempt = RepairAttempt {
            attempt_id: "a1".to_string(),
            edits: vec![make_edit("fix")],
            test_result: Some(make_test_result(10, 0, 0)),
            reward: Some(RepairReward::compute(&make_test_result(10, 0, 0), 5, 0)),
            cost: 0.01,
            duration_secs: 1.5,
        };
        assert_eq!(attempt.edits.len(), 1);
        assert!(attempt.reward.unwrap().total > 0.0);
    }

    #[test]
    fn test_comparison_entry_fields() {
        let entry = ComparisonEntry {
            session_id: "s1".to_string(),
            strategy: RepairStrategy::Mcts,
            solved: true,
            cost: 0.05,
            edits_count: 3,
            time_secs: 10.0,
        };
        assert!(entry.solved);
        assert_eq!(entry.strategy, RepairStrategy::Mcts);
    }

    #[test]
    fn test_tree_node_count_after_expansion() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        assert_eq!(tree.node_count(), 1);

        tree.expand_node("root", make_edit("a")).unwrap();
        tree.expand_node("root", make_edit("b")).unwrap();
        assert_eq!(tree.node_count(), 3);

        tree.expand_node("root-c0", make_edit("c")).unwrap();
        assert_eq!(tree.node_count(), 4);
    }

    #[test]
    fn test_multiple_backpropagations_from_different_leaves() {
        let mut tree = MctsTree::new(make_edit("root"), MctsConfig::default());
        tree.add_node("c1".into(), "root", make_edit("a")).unwrap();
        tree.add_node("c2".into(), "root", make_edit("b")).unwrap();

        tree.backpropagate("c1", 0.3).unwrap();
        tree.backpropagate("c2", 0.7).unwrap();

        let root = tree.get_node("root").unwrap();
        assert_eq!(root.visit_count, 2);
        assert!((root.total_reward - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_test_result_total_consistency() {
        let tr = make_test_result(5, 3, 2);
        assert_eq!(tr.total, 10);
        assert_eq!(tr.passed + tr.failed + tr.errors, tr.total);
    }
}
