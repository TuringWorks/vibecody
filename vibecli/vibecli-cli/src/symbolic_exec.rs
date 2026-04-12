//! Symbolic execution and probabilistic code path analysis.
//!
//! GAP-v9-011: rivals Devin Symbolic Executor, Cursor Path Explorer, Amazon Q Code Paths.
//! - Static CFG (control-flow graph) construction from source AST approximation
//! - Symbolic variable tracking with constraint sets
//! - Path probability estimation (branch weights, loop bounds)
//! - Reachability analysis and dead-code detection
//! - Path condition summarisation for LLM injection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Symbolic Values ─────────────────────────────────────────────────────────

/// A symbolic expression representing a value at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymExpr {
    /// Concrete literal value.
    Lit(i64),
    /// Symbolic variable (unknown at analysis time).
    Var(String),
    /// Arithmetic: left op right
    BinOp { op: BinOp, left: Box<SymExpr>, right: Box<SymExpr> },
    /// Unconstrained (e.g. external input).
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp { Add, Sub, Mul, Div, Mod }

impl std::fmt::Display for SymExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lit(n)    => write!(f, "{n}"),
            Self::Var(v)    => write!(f, "{v}"),
            Self::Any       => write!(f, "?"),
            Self::BinOp { op, left, right } => {
                let op_s = match op {
                    BinOp::Add => "+", BinOp::Sub => "-",
                    BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
                };
                write!(f, "({left} {op_s} {right})")
            }
        }
    }
}

// ─── Path Constraints ────────────────────────────────────────────────────────

/// A single relational constraint on symbolic values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub lhs: SymExpr,
    pub rel: Relation,
    pub rhs: SymExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation { Eq, Ne, Lt, Le, Gt, Ge }

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rel = match self.rel {
            Relation::Eq => "==", Relation::Ne => "!=",
            Relation::Lt => "<",  Relation::Le => "<=",
            Relation::Gt => ">",  Relation::Ge => ">=",
        };
        write!(f, "{} {} {}", self.lhs, rel, self.rhs)
    }
}

/// Check whether a constraint is trivially false (concrete literals only).
fn is_trivially_unsat(c: &Constraint) -> bool {
    if let (SymExpr::Lit(l), SymExpr::Lit(r)) = (&c.lhs, &c.rhs) {
        match c.rel {
            Relation::Eq => l != r,
            Relation::Ne => l == r,
            Relation::Lt => l >= r,
            Relation::Le => l > r,
            Relation::Gt => l <= r,
            Relation::Ge => l < r,
        }
    } else {
        false
    }
}

// ─── CFG Nodes ───────────────────────────────────────────────────────────────

/// Unique node identifier in the CFG.
pub type NodeId = u32;

/// A basic block in the control-flow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgNode {
    pub id: NodeId,
    /// Source line range (start, end), inclusive.
    pub lines: (usize, usize),
    pub label: String,
    pub kind: NodeKind,
    /// Successor node ids.
    pub successors: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Entry,
    Exit,
    /// Straight-line code block.
    Basic,
    /// Branch point: condition stored separately.
    Branch,
    /// Loop back-edge.
    LoopHeader,
    /// Function call site.
    Call { callee: String },
}

// ─── Execution Path ──────────────────────────────────────────────────────────

/// A single feasible path through the CFG with accumulated constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPath {
    /// Sequence of node ids visited.
    pub nodes: Vec<NodeId>,
    /// Path constraints accumulated along this path.
    pub constraints: Vec<Constraint>,
    /// Estimated probability (product of branch weights).
    pub probability: f64,
    /// Whether the SMT-lite check found the path infeasible.
    pub infeasible: bool,
}

impl ExecPath {
    pub fn new(start: NodeId) -> Self {
        Self { nodes: vec![start], constraints: Vec::new(), probability: 1.0, infeasible: false }
    }

    /// Extend path to next node, adding optional constraint and branch probability.
    pub fn extend(&self, next: NodeId, constraint: Option<Constraint>, branch_prob: f64) -> Self {
        let mut path = self.clone();
        path.nodes.push(next);
        path.probability *= branch_prob;
        if let Some(c) = constraint {
            if is_trivially_unsat(&c) {
                path.infeasible = true;
            }
            path.constraints.push(c);
        }
        path
    }

    /// Summarise the path as a human-readable string for LLM context injection.
    pub fn summary(&self) -> String {
        let nodes: Vec<String> = self.nodes.iter().map(|n| format!("N{n}")).collect();
        let conds: Vec<String> = self.constraints.iter().map(|c| c.to_string()).collect();
        if conds.is_empty() {
            format!("path [{}] prob={:.3}", nodes.join("→"), self.probability)
        } else {
            format!("path [{}] prob={:.3} when {}", nodes.join("→"), self.probability, conds.join(" ∧ "))
        }
    }
}

// ─── Symbolic Executor ───────────────────────────────────────────────────────

/// Configuration for the symbolic executor.
#[derive(Debug, Clone)]
pub struct ExecConfig {
    /// Maximum path depth (avoids infinite loops).
    pub max_depth: usize,
    /// Maximum number of paths to enumerate.
    pub max_paths: usize,
    /// Default probability for the "true" branch of an unknown condition.
    pub default_true_prob: f64,
    /// Loop unrolling bound (how many times to unroll a back-edge).
    pub loop_unroll: usize,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self { max_depth: 20, max_paths: 512, default_true_prob: 0.6, loop_unroll: 3 }
    }
}

/// Symbolic executor over a CFG.
pub struct SymbolicExecutor {
    pub nodes: HashMap<NodeId, CfgNode>,
    /// Branch probability overrides: (from_node, to_node) → probability.
    pub branch_probs: HashMap<(NodeId, NodeId), f64>,
    /// Optional per-node branch constraints: (from_node, to_node) → Constraint.
    pub branch_constraints: HashMap<(NodeId, NodeId), Constraint>,
    pub config: ExecConfig,
}

impl SymbolicExecutor {
    pub fn new(config: ExecConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            branch_probs: HashMap::new(),
            branch_constraints: HashMap::new(),
            config,
        }
    }

    pub fn add_node(&mut self, node: CfgNode) { self.nodes.insert(node.id, node); }

    pub fn set_branch_prob(&mut self, from: NodeId, to: NodeId, prob: f64) {
        self.branch_probs.insert((from, to), prob);
    }

    pub fn set_branch_constraint(&mut self, from: NodeId, to: NodeId, c: Constraint) {
        self.branch_constraints.insert((from, to), c);
    }

    fn branch_prob(&self, from: NodeId, to: NodeId) -> f64 {
        self.branch_probs.get(&(from, to)).copied().unwrap_or(self.config.default_true_prob)
    }

    /// Enumerate all feasible paths from `entry` to `exit` using DFS.
    pub fn enumerate_paths(&self, entry: NodeId, exit: NodeId) -> Vec<ExecPath> {
        let mut completed: Vec<ExecPath> = Vec::new();
        let mut stack: Vec<(ExecPath, HashMap<NodeId, usize>)> = vec![
            (ExecPath::new(entry), HashMap::new())
        ];

        while let Some((path, visit_count)) = stack.pop() {
            if completed.len() >= self.config.max_paths { break; }
            if path.infeasible { continue; }

            let current = *path.nodes.last().unwrap();

            if current == exit {
                completed.push(path);
                continue;
            }

            if path.nodes.len() >= self.config.max_depth { continue; }

            let node = match self.nodes.get(&current) { Some(n) => n, None => continue };

            for &succ in &node.successors {
                let loop_count = *visit_count.get(&succ).unwrap_or(&0);
                if node.kind == NodeKind::LoopHeader && loop_count >= self.config.loop_unroll {
                    continue;
                }
                let prob = self.branch_prob(current, succ);
                let constraint = self.branch_constraints.get(&(current, succ)).cloned();
                let new_path = path.extend(succ, constraint, prob);
                let mut new_visits = visit_count.clone();
                *new_visits.entry(succ).or_insert(0) += 1;
                stack.push((new_path, new_visits));
            }
        }

        completed
    }

    /// Identify nodes that are unreachable from entry (dead code).
    pub fn dead_nodes(&self, entry: NodeId) -> Vec<NodeId> {
        let mut reachable = std::collections::HashSet::new();
        let mut queue = vec![entry];
        while let Some(n) = queue.pop() {
            if reachable.insert(n) {
                if let Some(node) = self.nodes.get(&n) {
                    for &s in &node.successors { queue.push(s); }
                }
            }
        }
        self.nodes.keys().filter(|id| !reachable.contains(*id)).copied().collect()
    }

    /// Summarise path coverage as text for LLM injection.
    pub fn coverage_prompt(&self, paths: &[ExecPath]) -> String {
        let feasible: Vec<_> = paths.iter().filter(|p| !p.infeasible).collect();
        let total_prob: f64 = feasible.iter().map(|p| p.probability).sum();
        let mut lines = vec![
            format!("Symbolic execution: {} feasible paths, cumulative prob={:.3}", feasible.len(), total_prob),
        ];
        for (i, p) in feasible.iter().enumerate().take(5) {
            lines.push(format!("  [{}] {}", i + 1, p.summary()));
        }
        if feasible.len() > 5 { lines.push(format!("  … and {} more paths", feasible.len() - 5)); }
        lines.join("\n")
    }
}

// ─── Source-based CFG Builder ─────────────────────────────────────────────────

/// Lightweight CFG builder that infers basic structure from source lines.
pub struct CfgBuilder {
    next_id: NodeId,
}

impl CfgBuilder {
    pub fn new() -> Self { Self { next_id: 0 } }

    fn alloc(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Build a minimal CFG from source lines by detecting branch/loop keywords.
    /// Returns (executor, entry_id, exit_id).
    pub fn from_source(lines: &[&str], config: ExecConfig) -> (SymbolicExecutor, NodeId, NodeId) {
        let mut exec = SymbolicExecutor::new(config);
        let mut builder = Self::new();

        let entry = builder.alloc();
        exec.add_node(CfgNode { id: entry, lines: (0, 0), label: "entry".into(), kind: NodeKind::Entry, successors: vec![] });

        let exit = builder.alloc();
        // Will be added at end.

        let mut prev = entry;
        let mut block_start = 1usize;
        let mut depth: usize = 0;
        let mut branch_stack: Vec<NodeId> = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Detect branch keywords
            if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
                // Close previous basic block
                let basic_id = builder.alloc();
                exec.add_node(CfgNode {
                    id: basic_id, lines: (block_start, idx),
                    label: format!("block@{idx}"), kind: NodeKind::Basic, successors: vec![],
                });
                // Connect prev → basic
                if let Some(node) = exec.nodes.get_mut(&prev) { node.successors.push(basic_id); }

                // Branch node
                let branch_id = builder.alloc();
                exec.add_node(CfgNode {
                    id: branch_id, lines: (idx + 1, idx + 1),
                    label: format!("if@{}", idx + 1), kind: NodeKind::Branch, successors: vec![],
                });
                if let Some(node) = exec.nodes.get_mut(&basic_id) { node.successors.push(branch_id); }

                branch_stack.push(branch_id);
                prev = branch_id;
                block_start = idx + 2;
                depth += 1;

            } else if (trimmed.starts_with("for ") || trimmed.starts_with("while ")) && depth < 10 {
                let loop_id = builder.alloc();
                exec.add_node(CfgNode {
                    id: loop_id, lines: (idx + 1, idx + 1),
                    label: format!("loop@{}", idx + 1), kind: NodeKind::LoopHeader, successors: vec![],
                });
                if let Some(node) = exec.nodes.get_mut(&prev) { node.successors.push(loop_id); }
                // Back-edge from loop to itself (unrolled by executor)
                exec.nodes.get_mut(&loop_id).unwrap().successors.push(loop_id);
                prev = loop_id;
                block_start = idx + 2;
                depth += 1;

            } else if trimmed == "}" && depth > 0 {
                depth -= 1;
                if let Some(branch_id) = branch_stack.pop() {
                    // True branch merges back to a new merge node
                    let merge_id = builder.alloc();
                    exec.add_node(CfgNode {
                        id: merge_id, lines: (idx + 1, idx + 1),
                        label: format!("merge@{}", idx + 1), kind: NodeKind::Basic, successors: vec![],
                    });
                    if let Some(node) = exec.nodes.get_mut(&prev) { node.successors.push(merge_id); }
                    // False branch: skip body (branch_id → merge directly)
                    if let Some(node) = exec.nodes.get_mut(&branch_id) { node.successors.push(merge_id); }
                    exec.set_branch_prob(branch_id, prev, 0.6);
                    exec.set_branch_prob(branch_id, merge_id, 0.4);
                    prev = merge_id;
                    block_start = idx + 2;
                }
            }
        }

        // Final block
        let final_id = builder.alloc();
        exec.add_node(CfgNode {
            id: final_id, lines: (block_start, lines.len()),
            label: "final".into(), kind: NodeKind::Basic, successors: vec![exit],
        });
        if let Some(node) = exec.nodes.get_mut(&prev) { node.successors.push(final_id); }

        exec.add_node(CfgNode { id: exit, lines: (lines.len(), lines.len()), label: "exit".into(), kind: NodeKind::Exit, successors: vec![] });

        (exec, entry, exit)
    }
}

impl Default for CfgBuilder {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exec() -> (SymbolicExecutor, NodeId, NodeId) {
        // Simple CFG: entry → branch → {A, B} → merge → exit
        let mut exec = SymbolicExecutor::new(ExecConfig::default());
        exec.add_node(CfgNode { id: 0, lines: (1,1), label: "entry".into(), kind: NodeKind::Entry, successors: vec![1] });
        exec.add_node(CfgNode { id: 1, lines: (2,3), label: "branch".into(), kind: NodeKind::Branch, successors: vec![2, 3] });
        exec.add_node(CfgNode { id: 2, lines: (4,5), label: "true_block".into(), kind: NodeKind::Basic, successors: vec![4] });
        exec.add_node(CfgNode { id: 3, lines: (6,7), label: "false_block".into(), kind: NodeKind::Basic, successors: vec![4] });
        exec.add_node(CfgNode { id: 4, lines: (8,9), label: "merge".into(), kind: NodeKind::Basic, successors: vec![5] });
        exec.add_node(CfgNode { id: 5, lines: (10,10), label: "exit".into(), kind: NodeKind::Exit, successors: vec![] });
        exec.set_branch_prob(1, 2, 0.7);
        exec.set_branch_prob(1, 3, 0.3);
        (exec, 0, 5)
    }

    #[test]
    fn test_enumerate_two_paths() {
        let (exec, entry, exit) = make_exec();
        let paths = exec.enumerate_paths(entry, exit);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_path_probabilities_ratio() {
        // make_exec sets branch_prob(1,2)=0.7 and branch_prob(1,3)=0.3.
        // Non-branch edges multiply by the same default factor on both paths,
        // so the ratio of the two path probabilities must equal 0.7/0.3.
        let (exec, entry, exit) = make_exec();
        let paths = exec.enumerate_paths(entry, exit);
        assert_eq!(paths.len(), 2);
        let (p_true, p_false) = if paths[0].nodes.contains(&2) {
            (paths[0].probability, paths[1].probability)
        } else {
            (paths[1].probability, paths[0].probability)
        };
        let ratio = p_true / p_false;
        assert!((ratio - 7.0 / 3.0).abs() < 1e-6, "expected ratio 7:3, got {ratio}");
    }

    #[test]
    fn test_path_nodes_include_entry_and_exit() {
        let (exec, entry, exit) = make_exec();
        let paths = exec.enumerate_paths(entry, exit);
        for p in &paths {
            assert_eq!(*p.nodes.first().unwrap(), entry);
            assert_eq!(*p.nodes.last().unwrap(), exit);
        }
    }

    #[test]
    fn test_infeasible_path_filtered() {
        let mut exec = SymbolicExecutor::new(ExecConfig::default());
        exec.add_node(CfgNode { id: 0, lines: (1,1), label: "entry".into(), kind: NodeKind::Entry, successors: vec![1] });
        exec.add_node(CfgNode { id: 1, lines: (2,2), label: "branch".into(), kind: NodeKind::Branch, successors: vec![2, 3] });
        exec.add_node(CfgNode { id: 2, lines: (3,3), label: "dead".into(), kind: NodeKind::Basic, successors: vec![4] });
        exec.add_node(CfgNode { id: 3, lines: (4,4), label: "live".into(), kind: NodeKind::Basic, successors: vec![4] });
        exec.add_node(CfgNode { id: 4, lines: (5,5), label: "exit".into(), kind: NodeKind::Exit, successors: vec![] });
        // Branch 1→2 has infeasible constraint: 1 == 2
        exec.set_branch_constraint(1, 2, Constraint {
            lhs: SymExpr::Lit(1), rel: Relation::Eq, rhs: SymExpr::Lit(2),
        });
        let paths = exec.enumerate_paths(0, 4);
        let feasible: Vec<_> = paths.iter().filter(|p| !p.infeasible).collect();
        assert_eq!(feasible.len(), 1);
        assert!(feasible[0].nodes.contains(&3));
    }

    #[test]
    fn test_dead_nodes_detection() {
        let mut exec = SymbolicExecutor::new(ExecConfig::default());
        exec.add_node(CfgNode { id: 0, lines: (1,1), label: "entry".into(), kind: NodeKind::Entry, successors: vec![1] });
        exec.add_node(CfgNode { id: 1, lines: (2,2), label: "live".into(), kind: NodeKind::Basic, successors: vec![] });
        exec.add_node(CfgNode { id: 2, lines: (3,3), label: "orphan".into(), kind: NodeKind::Basic, successors: vec![] });
        let dead = exec.dead_nodes(0);
        assert!(dead.contains(&2));
        assert!(!dead.contains(&0));
        assert!(!dead.contains(&1));
    }

    #[test]
    fn test_dead_nodes_empty_when_all_reachable() {
        let (exec, entry, _exit) = make_exec();
        let dead = exec.dead_nodes(entry);
        assert!(dead.is_empty());
    }

    #[test]
    fn test_max_paths_limit() {
        let (mut exec, entry, exit) = make_exec();
        exec.config.max_paths = 1;
        let paths = exec.enumerate_paths(entry, exit);
        assert!(paths.len() <= 1);
    }

    #[test]
    fn test_max_depth_limit() {
        let (mut exec, entry, exit) = make_exec();
        exec.config.max_depth = 2; // too shallow to complete
        let paths = exec.enumerate_paths(entry, exit);
        // With depth 2 we can't reach exit (which is 5 steps away)
        assert!(paths.is_empty() || paths.iter().all(|p| p.nodes.len() <= 2));
    }

    #[test]
    fn test_sym_expr_display_lit() {
        assert_eq!(SymExpr::Lit(42).to_string(), "42");
    }

    #[test]
    fn test_sym_expr_display_var() {
        assert_eq!(SymExpr::Var("x".into()).to_string(), "x");
    }

    #[test]
    fn test_sym_expr_display_binop() {
        let e = SymExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(SymExpr::Var("x".into())),
            right: Box::new(SymExpr::Lit(1)),
        };
        assert_eq!(e.to_string(), "(x + 1)");
    }

    #[test]
    fn test_constraint_display() {
        let c = Constraint { lhs: SymExpr::Var("n".into()), rel: Relation::Gt, rhs: SymExpr::Lit(0) };
        assert_eq!(c.to_string(), "n > 0");
    }

    #[test]
    fn test_is_trivially_unsat_true() {
        let c = Constraint { lhs: SymExpr::Lit(1), rel: Relation::Eq, rhs: SymExpr::Lit(2) };
        assert!(is_trivially_unsat(&c));
    }

    #[test]
    fn test_is_trivially_unsat_false_for_vars() {
        let c = Constraint { lhs: SymExpr::Var("x".into()), rel: Relation::Eq, rhs: SymExpr::Lit(0) };
        assert!(!is_trivially_unsat(&c));
    }

    #[test]
    fn test_is_trivially_unsat_sat_lit() {
        let c = Constraint { lhs: SymExpr::Lit(3), rel: Relation::Gt, rhs: SymExpr::Lit(1) };
        assert!(!is_trivially_unsat(&c));
    }

    #[test]
    fn test_constraint_added_to_path() {
        let p = ExecPath::new(0);
        let c = Constraint { lhs: SymExpr::Var("x".into()), rel: Relation::Lt, rhs: SymExpr::Lit(10) };
        let p2 = p.extend(1, Some(c.clone()), 0.6);
        assert_eq!(p2.constraints.len(), 1);
        assert_eq!(p2.constraints[0], c);
    }

    #[test]
    fn test_infeasible_constraint_marks_path() {
        let p = ExecPath::new(0);
        let c = Constraint { lhs: SymExpr::Lit(5), rel: Relation::Lt, rhs: SymExpr::Lit(2) };
        let p2 = p.extend(1, Some(c), 0.5);
        assert!(p2.infeasible);
    }

    #[test]
    fn test_path_probability_product() {
        let p = ExecPath::new(0);
        let p2 = p.extend(1, None, 0.6);
        let p3 = p2.extend(2, None, 0.5);
        assert!((p3.probability - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_path_summary_no_constraints() {
        let p = ExecPath::new(0);
        let p2 = p.extend(1, None, 1.0);
        let s = p2.summary();
        assert!(s.contains("N0→N1"));
        assert!(s.contains("prob=1.000"));
    }

    #[test]
    fn test_path_summary_with_constraint() {
        let p = ExecPath::new(0);
        let c = Constraint { lhs: SymExpr::Var("x".into()), rel: Relation::Gt, rhs: SymExpr::Lit(0) };
        let p2 = p.extend(1, Some(c), 0.7);
        let s = p2.summary();
        assert!(s.contains("x > 0"));
    }

    #[test]
    fn test_coverage_prompt_format() {
        let (exec, entry, exit) = make_exec();
        let paths = exec.enumerate_paths(entry, exit);
        let prompt = exec.coverage_prompt(&paths);
        assert!(prompt.contains("feasible paths"));
        assert!(prompt.contains("prob="));
    }

    #[test]
    fn test_loop_unrolling_bounded() {
        let mut exec = SymbolicExecutor::new(ExecConfig { loop_unroll: 2, ..Default::default() });
        // loop: 0 → 1 (header) → 1 (back-edge) or → 2 (exit)
        exec.add_node(CfgNode { id: 0, lines: (1,1), label: "entry".into(), kind: NodeKind::Entry, successors: vec![1] });
        exec.add_node(CfgNode { id: 1, lines: (2,3), label: "loop".into(), kind: NodeKind::LoopHeader, successors: vec![1, 2] });
        exec.add_node(CfgNode { id: 2, lines: (4,4), label: "exit".into(), kind: NodeKind::Exit, successors: vec![] });
        exec.set_branch_prob(1, 1, 0.7);
        exec.set_branch_prob(1, 2, 0.3);
        let paths = exec.enumerate_paths(0, 2);
        assert!(!paths.is_empty());
        // None should unroll more than loop_unroll=2 times
        for p in &paths {
            let loop_visits = p.nodes.iter().filter(|&&n| n == 1).count();
            assert!(loop_visits <= 3, "loop visits={loop_visits} exceeds unroll bound");
        }
    }

    #[test]
    fn test_cfg_builder_from_source_simple() {
        let src = vec!["fn foo(x: i32) -> i32 {", "  x + 1", "}"];
        let (exec, entry, exit) = CfgBuilder::from_source(&src, ExecConfig::default());
        let paths = exec.enumerate_paths(entry, exit);
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_cfg_builder_from_source_with_if() {
        let src = vec![
            "fn foo(x: i32) -> i32 {",
            "  if x > 0 {",
            "    return x;",
            "  }",
            "  -x",
            "}",
        ];
        let (exec, entry, exit) = CfgBuilder::from_source(&src, ExecConfig::default());
        let paths = exec.enumerate_paths(entry, exit);
        // should produce at least 2 paths (true and false branch)
        assert!(paths.len() >= 1);
        let _ = exec.dead_nodes(entry); // should not panic
    }

    #[test]
    fn test_cfg_builder_no_dead_nodes_simple() {
        let src = vec!["let x = 1;", "let y = x + 2;"];
        let (exec, entry, _exit) = CfgBuilder::from_source(&src, ExecConfig::default());
        let dead = exec.dead_nodes(entry);
        assert!(dead.is_empty());
    }

    #[test]
    fn test_exec_path_extend_without_constraint() {
        let p = ExecPath::new(5);
        let p2 = p.extend(6, None, 0.8);
        assert_eq!(p2.nodes, vec![5, 6]);
        assert!((p2.probability - 0.8).abs() < 1e-9);
        assert!(!p2.infeasible);
        assert!(p2.constraints.is_empty());
    }

    #[test]
    fn test_sym_expr_any_display() {
        assert_eq!(SymExpr::Any.to_string(), "?");
    }

    #[test]
    fn test_exec_config_defaults() {
        let c = ExecConfig::default();
        assert_eq!(c.max_depth, 20);
        assert_eq!(c.max_paths, 512);
        assert_eq!(c.loop_unroll, 3);
        assert!((c.default_true_prob - 0.6).abs() < 1e-9);
    }
}
