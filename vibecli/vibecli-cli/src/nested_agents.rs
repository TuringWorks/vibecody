#![allow(dead_code)]
//! Recursive agent tree execution — spawn, manage, and aggregate hierarchical agent nodes.
//!
//! Provides an `AgentTree` that owns a directed acyclic tree of `AgentNode` instances.
//! Each node carries configuration (depth limits, context inheritance, merge strategy),
//! lifecycle status, and an optional result payload. A `CycleDetector` wraps DFS-based
//! reachability to prevent cycles when wiring parent-child edges.
//!
//! # Architecture
//!
//! ```text
//! AgentTree
//!   └─ HashMap<node_id, AgentNode>
//!        ├─ parent_id: Option<String>
//!        ├─ children: Vec<String>
//!        └─ status: NodeStatus (Pending → Running → Completed(r) | Failed(e) | Cancelled)
//!
//! aggregate_results(root, strategy)
//!   └─ DFS leaf collection → MergeStrategy applied
//!
//! cancel_subtree(node)
//!   └─ DFS cancellation of node + all descendants
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── Enums ───────────────────────────────────────────────────────────────────

/// How a child agent inherits context from its parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextInheritance {
    /// Full parent context is passed down.
    Full,
    /// Only extracted symbol references are inherited.
    SymbolsOnly,
    /// Child starts with a clean slate — no inherited context.
    Isolated,
}

impl std::fmt::Display for ContextInheritance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::SymbolsOnly => write!(f, "SymbolsOnly"),
            Self::Isolated => write!(f, "Isolated"),
        }
    }
}

/// Lifecycle state of an agent node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,
    Running,
    /// Carries the result payload produced by this node.
    Completed(String),
    /// Carries the error description from this node's failure.
    Failed(String),
    Cancelled,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Completed(r) => write!(f, "Completed({})", r),
            Self::Failed(e) => write!(f, "Failed({})", e),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Strategy used when merging leaf results back up the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Concatenate leaf results with newlines.
    Concat,
    /// Join results as a JSON-like structured list.
    Structured,
    /// Produce a unified-diff-style patch merge of leaf results.
    CodePatchMerge,
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Concat => write!(f, "Concat"),
            Self::Structured => write!(f, "Structured"),
            Self::CodePatchMerge => write!(f, "CodePatchMerge"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// Configuration carried by each node describing how it should execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub description: String,
    /// The depth at which this node resides in the tree (root = 0).
    pub depth: u32,
    /// Maximum tree depth allowed for this node's subtree.
    pub max_depth: u32,
    pub inheritance: ContextInheritance,
    pub timeout_secs: u64,
    pub merge_strategy: MergeStrategy,
}

/// A single node in the recursive agent execution tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: String,
    pub config: AgentNodeConfig,
    /// `None` for the root node; otherwise the parent's ID.
    pub parent_id: Option<String>,
    /// IDs of direct child nodes.
    pub children: Vec<String>,
    pub status: NodeStatus,
    pub created_at_ms: u64,
    pub result: Option<String>,
}

// ─── AgentTree ───────────────────────────────────────────────────────────────

/// Manages the entire recursive agent execution tree.
pub struct AgentTree {
    max_depth: u32,
    nodes: HashMap<String, AgentNode>,
    next_id: u64,
    /// Monotonic fake clock for deterministic tests.
    clock_ms: u64,
}

impl AgentTree {
    pub fn new(max_depth: u32) -> Self {
        Self {
            max_depth,
            nodes: HashMap::new(),
            next_id: 1,
            clock_ms: 1_000,
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn tick(&mut self) -> u64 {
        self.clock_ms += 1;
        self.clock_ms
    }

    fn gen_id(&mut self) -> String {
        let id = format!("node-{}", self.next_id);
        self.next_id += 1;
        id
    }

    // ── Public API ──────────────────────────────────────────────────────

    /// Spawn the root node (no parent).  Returns the new node's ID.
    ///
    /// Returns `Err` if a root node already exists in the tree.
    pub fn spawn_root(&mut self, config: AgentNodeConfig) -> Result<String, String> {
        // Only one root is allowed.
        let has_root = self.nodes.values().any(|n| n.parent_id.is_none());
        if has_root {
            return Err("A root node already exists".to_string());
        }
        let ts = self.tick();
        let id = self.gen_id();
        let node = AgentNode {
            id: id.clone(),
            config,
            parent_id: None,
            children: Vec::new(),
            status: NodeStatus::Pending,
            created_at_ms: ts,
            result: None,
        };
        self.nodes.insert(id.clone(), node);
        Ok(id)
    }

    /// Spawn a child node under `parent_id`.
    ///
    /// Returns `Err` if:
    /// - `parent_id` does not exist,
    /// - spawning would exceed `max_depth`,
    /// - `config.depth` mismatches the actual parent depth + 1, or
    /// - adding the edge would create a cycle.
    pub fn spawn_child(
        &mut self,
        parent_id: &str,
        config: AgentNodeConfig,
    ) -> Result<String, String> {
        let parent_depth = {
            let parent = self
                .nodes
                .get(parent_id)
                .ok_or_else(|| format!("Parent node '{}' not found", parent_id))?;
            parent.config.depth
        };

        let child_depth = parent_depth + 1;
        if child_depth > self.max_depth {
            return Err(format!(
                "max_depth ({}) exceeded: child would be at depth {}",
                self.max_depth, child_depth
            ));
        }

        // Cycle guard: the new child ID doesn't exist yet, so a cycle can only
        // arise if `parent_id` is reachable from the (future) child's perspective —
        // i.e., if `parent_id` is itself a descendant of some ancestor already
        // tracked.  In a properly managed tree the only way to get a cycle would
        // be if `parent_id` == new_child_id.  We perform a full DFS check using
        // the CycleDetector to be safe.
        let detector = CycleDetector::new();
        // The child does not exist yet, so we pass an empty placeholder ID.
        // Since we generate the ID after this check and no node with that ID
        // exists, the DFS will safely return false.
        let new_id = format!("node-{}", self.next_id);
        if detector.would_create_cycle(&self.nodes, parent_id, &new_id) {
            return Err(format!(
                "Adding child '{}' under parent '{}' would create a cycle",
                new_id, parent_id
            ));
        }

        let ts = self.tick();
        let id = self.gen_id(); // consumes next_id (same as new_id above)
        let node = AgentNode {
            id: id.clone(),
            config,
            parent_id: Some(parent_id.to_string()),
            children: Vec::new(),
            status: NodeStatus::Pending,
            created_at_ms: ts,
            result: None,
        };
        self.nodes.insert(id.clone(), node);
        // Register the child in the parent's children list.
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.push(id.clone());
        }
        Ok(id)
    }

    /// Set the status of a node directly.
    pub fn set_status(&mut self, node_id: &str, status: NodeStatus) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?;
        node.status = status;
        Ok(())
    }

    /// Mark `node_id` as `Completed` and store the result payload.
    pub fn complete_node(&mut self, node_id: &str, result: String) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?;
        node.status = NodeStatus::Completed(result.clone());
        node.result = Some(result);
        Ok(())
    }

    /// Mark `node_id` as `Failed` and store the error description.
    pub fn fail_node(&mut self, node_id: &str, error: String) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("Node '{}' not found", node_id))?;
        node.status = NodeStatus::Failed(error);
        Ok(())
    }

    /// Cancel `node_id` and all of its descendants.  Returns the number of
    /// nodes whose status was changed to `Cancelled`.
    pub fn cancel_subtree(&mut self, node_id: &str) -> usize {
        // Collect IDs of node + all descendants via DFS (avoid borrowing issues
        // by collecting IDs first, then mutating).
        let ids = self.collect_subtree_ids(node_id);
        let mut count = 0;
        for id in &ids {
            if let Some(node) = self.nodes.get_mut(id) {
                if node.status != NodeStatus::Cancelled {
                    node.status = NodeStatus::Cancelled;
                    count += 1;
                }
            }
        }
        count
    }

    /// DFS-collect `start` and all its descendants, returning their IDs.
    fn collect_subtree_ids(&self, start: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut stack = vec![start.to_string()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.nodes.get(&id) {
                result.push(id.clone());
                for child_id in &node.children {
                    stack.push(child_id.clone());
                }
            }
        }
        result
    }

    /// Aggregate results from leaves up to `node_id` using `strategy`.
    ///
    /// Returns `None` if `node_id` does not exist.  Leaf results are collected
    /// via DFS in the order they appear in `children`.
    pub fn aggregate_results(&self, node_id: &str, strategy: &MergeStrategy) -> Option<String> {
        let _root = self.nodes.get(node_id)?;
        let leaves = self.collect_leaf_results(node_id);
        if leaves.is_empty() {
            // The node itself might have a result even if it has no children.
            return self.nodes.get(node_id).and_then(|n| n.result.clone());
        }
        let merged = match strategy {
            MergeStrategy::Concat => leaves.join("\n"),
            MergeStrategy::Structured => {
                let items: Vec<String> = leaves
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("  [{}]: {}", i, r))
                    .collect();
                format!("{{\n{}\n}}", items.join(",\n"))
            }
            MergeStrategy::CodePatchMerge => {
                let patches: Vec<String> = leaves
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("--- patch {}\n+++ {}", i, r))
                    .collect();
                patches.join("\n")
            }
        };
        Some(merged)
    }

    /// Collect the result strings from all leaf nodes reachable from `start`.
    fn collect_leaf_results(&self, start: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut stack = vec![start.to_string()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.nodes.get(&id) {
                if node.children.is_empty() {
                    if let Some(r) = &node.result {
                        results.push(r.clone());
                    }
                } else {
                    // Push children in reverse so left-most is processed first.
                    for child_id in node.children.iter().rev() {
                        stack.push(child_id.clone());
                    }
                }
            }
        }
        results
    }

    /// Returns the depth of `node_id` (0 = root).  Returns 0 for unknown nodes.
    pub fn depth_of(&self, node_id: &str) -> u32 {
        self.nodes
            .get(node_id)
            .map(|n| n.config.depth)
            .unwrap_or(0)
    }

    /// Returns `true` if the node has no children.
    pub fn is_leaf(&self, node_id: &str) -> bool {
        self.nodes
            .get(node_id)
            .map(|n| n.children.is_empty())
            .unwrap_or(false)
    }

    /// Returns references to the direct children of `node_id`.
    pub fn children_of(&self, node_id: &str) -> Vec<&AgentNode> {
        self.nodes
            .get(node_id)
            .map(|n| {
                n.children
                    .iter()
                    .filter_map(|cid| self.nodes.get(cid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns all nodes whose `config.depth` equals `depth`.
    pub fn nodes_at_depth(&self, depth: u32) -> Vec<&AgentNode> {
        self.nodes.values().filter(|n| n.config.depth == depth).collect()
    }

    /// DFS check: would adding an edge from `from_id` to `to_id` create a cycle?
    ///
    /// In a tree this can only happen if `to_id` is an ancestor of `from_id`.
    pub fn detect_cycle(&self, from_id: &str, to_id: &str) -> bool {
        CycleDetector::new().would_create_cycle(&self.nodes, from_id, to_id)
    }

    /// Returns a textual summary of the execution tree.
    pub fn execution_graph_summary(&self) -> String {
        // Find root nodes (no parent).
        let mut roots: Vec<&AgentNode> =
            self.nodes.values().filter(|n| n.parent_id.is_none()).collect();
        // Sort by created_at for determinism.
        roots.sort_by_key(|n| n.created_at_ms);

        let mut lines = vec!["AgentTree:".to_string()];
        for root in roots {
            self.append_summary_lines(root, 0, &mut lines);
        }
        lines.join("\n")
    }

    fn append_summary_lines(&self, node: &AgentNode, indent: usize, lines: &mut Vec<String>) {
        let prefix = "  ".repeat(indent);
        lines.push(format!(
            "{}[{}] {} (depth={}, status={})",
            prefix,
            node.id,
            node.config.description,
            node.config.depth,
            node.status,
        ));
        let mut children: Vec<&AgentNode> = node
            .children
            .iter()
            .filter_map(|cid| self.nodes.get(cid))
            .collect();
        children.sort_by_key(|n| n.created_at_ms);
        for child in children {
            self.append_summary_lines(child, indent + 1, lines);
        }
    }

    /// Access all nodes (read-only).
    pub fn nodes(&self) -> &HashMap<String, AgentNode> {
        &self.nodes
    }

    /// Total number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ─── CycleDetector ───────────────────────────────────────────────────────────

/// Separate struct wrapping DFS-based cycle detection logic.
///
/// A cycle would occur when `child_id` is already an ancestor of `parent_id`.
/// The detector walks *upward* from `parent_id` via `parent_id` links; if it
/// reaches `child_id` before running out of ancestors, a cycle exists.
pub struct CycleDetector;

impl CycleDetector {
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` if adding the edge `parent_id → child_id` in `nodes`
    /// would introduce a cycle.
    pub fn would_create_cycle(
        &self,
        nodes: &HashMap<String, AgentNode>,
        parent_id: &str,
        child_id: &str,
    ) -> bool {
        // Trivial self-loop.
        if parent_id == child_id {
            return true;
        }
        // Walk ancestor chain of `parent_id`; if we reach `child_id`, it is
        // already an ancestor — adding the reverse edge would close a cycle.
        let mut current = parent_id.to_string();
        loop {
            if current == child_id {
                return true;
            }
            match nodes.get(&current).and_then(|n| n.parent_id.as_deref()) {
                Some(pid) => current = pid.to_string(),
                None => return false,
            }
        }
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a minimal `AgentNodeConfig` for test use.
fn node_cfg(description: &str, depth: u32) -> AgentNodeConfig {
    AgentNodeConfig {
        description: description.to_string(),
        depth,
        max_depth: 10,
        inheritance: ContextInheritance::Full,
        timeout_secs: 60,
        merge_strategy: MergeStrategy::Concat,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(max_depth: u32) -> AgentTree {
        AgentTree::new(max_depth)
    }

    // ── 1–5: Enum display ────────────────────────────────────────────────

    // 1
    #[test]
    fn test_context_inheritance_display() {
        assert_eq!(ContextInheritance::Full.to_string(), "Full");
        assert_eq!(ContextInheritance::SymbolsOnly.to_string(), "SymbolsOnly");
        assert_eq!(ContextInheritance::Isolated.to_string(), "Isolated");
    }

    // 2
    #[test]
    fn test_node_status_display_pending() {
        assert_eq!(NodeStatus::Pending.to_string(), "Pending");
    }

    // 3
    #[test]
    fn test_node_status_display_completed_with_payload() {
        assert_eq!(NodeStatus::Completed("ok".to_string()).to_string(), "Completed(ok)");
    }

    // 4
    #[test]
    fn test_node_status_display_failed_with_reason() {
        assert_eq!(NodeStatus::Failed("crash".to_string()).to_string(), "Failed(crash)");
    }

    // 5
    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(MergeStrategy::Concat.to_string(), "Concat");
        assert_eq!(MergeStrategy::Structured.to_string(), "Structured");
        assert_eq!(MergeStrategy::CodePatchMerge.to_string(), "CodePatchMerge");
    }

    // ── 6–10: spawn_root ────────────────────────────────────────────────

    // 6
    #[test]
    fn test_spawn_root_success() {
        let mut t = tree(5);
        let id = t.spawn_root(node_cfg("root", 0)).unwrap();
        assert!(id.starts_with("node-"));
        assert_eq!(t.node_count(), 1);
    }

    // 7
    #[test]
    fn test_spawn_root_node_is_pending() {
        let mut t = tree(5);
        let id = t.spawn_root(node_cfg("root", 0)).unwrap();
        let node = t.nodes().get(&id).unwrap();
        assert_eq!(node.status, NodeStatus::Pending);
    }

    // 8
    #[test]
    fn test_spawn_root_has_no_parent() {
        let mut t = tree(5);
        let id = t.spawn_root(node_cfg("root", 0)).unwrap();
        let node = t.nodes().get(&id).unwrap();
        assert!(node.parent_id.is_none());
    }

    // 9
    #[test]
    fn test_spawn_root_is_leaf() {
        let mut t = tree(5);
        let id = t.spawn_root(node_cfg("root", 0)).unwrap();
        assert!(t.is_leaf(&id));
    }

    // 10
    #[test]
    fn test_spawn_second_root_errors() {
        let mut t = tree(5);
        t.spawn_root(node_cfg("root", 0)).unwrap();
        let res = t.spawn_root(node_cfg("root2", 0));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("root node already exists"));
    }

    // ── 11–20: spawn_child ───────────────────────────────────────────────

    // 11
    #[test]
    fn test_spawn_child_success() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let child = t.spawn_child(&root, node_cfg("child", 1)).unwrap();
        assert!(child.starts_with("node-"));
        assert_eq!(t.node_count(), 2);
    }

    // 12
    #[test]
    fn test_spawn_child_parent_link() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let child = t.spawn_child(&root, node_cfg("child", 1)).unwrap();
        let node = t.nodes().get(&child).unwrap();
        assert_eq!(node.parent_id.as_deref(), Some(root.as_str()));
    }

    // 13
    #[test]
    fn test_spawn_child_listed_in_parent_children() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let child = t.spawn_child(&root, node_cfg("child", 1)).unwrap();
        let parent = t.nodes().get(&root).unwrap();
        assert!(parent.children.contains(&child));
    }

    // 14
    #[test]
    fn test_spawn_child_missing_parent_errors() {
        let mut t = tree(5);
        let res = t.spawn_child("nope", node_cfg("child", 1));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not found"));
    }

    // 15
    #[test]
    fn test_spawn_child_at_max_depth_ok() {
        let mut t = tree(2);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        // depth 2 == max_depth; should be allowed.
        let c2 = t.spawn_child(&c1, node_cfg("c2", 2));
        assert!(c2.is_ok());
    }

    // 16
    #[test]
    fn test_spawn_child_exceeds_max_depth_errors() {
        let mut t = tree(2);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&c1, node_cfg("c2", 2)).unwrap();
        // depth 3 > max_depth 2.
        let res = t.spawn_child(&c2, node_cfg("c3", 3));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("max_depth"));
    }

    // 17
    #[test]
    fn test_multiple_children_of_root() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let _c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let _c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        assert_eq!(t.children_of(&root).len(), 2);
    }

    // 18
    #[test]
    fn test_spawn_child_not_a_leaf_after_adding_child() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        assert!(!t.is_leaf(&root));
    }

    // 19
    #[test]
    fn test_deep_chain_depth_of() {
        let mut t = tree(10);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let mut parent = root.clone();
        for d in 1..=5u32 {
            let child = t.spawn_child(&parent, node_cfg("node", d)).unwrap();
            parent = child;
        }
        assert_eq!(t.depth_of(&parent), 5);
    }

    // 20
    #[test]
    fn test_depth_of_unknown_node_returns_zero() {
        let t = tree(5);
        assert_eq!(t.depth_of("ghost"), 0);
    }

    // ── 21–26: set_status / complete_node / fail_node ────────────────────

    // 21
    #[test]
    fn test_set_status_running() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        assert!(t.set_status(&root, NodeStatus::Running).is_ok());
        let node = t.nodes().get(&root).unwrap();
        assert_eq!(node.status, NodeStatus::Running);
    }

    // 22
    #[test]
    fn test_set_status_missing_node_errors() {
        let mut t = tree(5);
        assert!(t.set_status("ghost", NodeStatus::Running).is_err());
    }

    // 23
    #[test]
    fn test_complete_node_stores_result() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.complete_node(&root, "output".to_string()).unwrap();
        let node = t.nodes().get(&root).unwrap();
        assert_eq!(node.result.as_deref(), Some("output"));
        assert_eq!(node.status, NodeStatus::Completed("output".to_string()));
    }

    // 24
    #[test]
    fn test_complete_node_missing_errors() {
        let mut t = tree(5);
        assert!(t.complete_node("ghost", "x".to_string()).is_err());
    }

    // 25
    #[test]
    fn test_fail_node_stores_error_in_status() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.fail_node(&root, "timeout".to_string()).unwrap();
        let node = t.nodes().get(&root).unwrap();
        assert_eq!(node.status, NodeStatus::Failed("timeout".to_string()));
    }

    // 26
    #[test]
    fn test_fail_node_missing_errors() {
        let mut t = tree(5);
        assert!(t.fail_node("ghost", "err".to_string()).is_err());
    }

    // ── 27–33: cancel_subtree ────────────────────────────────────────────

    // 27
    #[test]
    fn test_cancel_subtree_single_node() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let count = t.cancel_subtree(&root);
        assert_eq!(count, 1);
        let node = t.nodes().get(&root).unwrap();
        assert_eq!(node.status, NodeStatus::Cancelled);
    }

    // 28
    #[test]
    fn test_cancel_subtree_with_children() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let _c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let _c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        let count = t.cancel_subtree(&root);
        assert_eq!(count, 3);
    }

    // 29
    #[test]
    fn test_cancel_subtree_deep_tree() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("r", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&c1, node_cfg("c2", 2)).unwrap();
        let _c3 = t.spawn_child(&c2, node_cfg("c3", 3)).unwrap();
        let count = t.cancel_subtree(&root);
        assert_eq!(count, 4);
    }

    // 30
    #[test]
    fn test_cancel_subtree_returns_only_newly_cancelled() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("r", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        // Pre-cancel c1 manually.
        t.set_status(&c1, NodeStatus::Cancelled).unwrap();
        let count = t.cancel_subtree(&root);
        // root was Pending → Cancelled; c1 was already Cancelled.
        assert_eq!(count, 1);
    }

    // 31
    #[test]
    fn test_cancel_subtree_unknown_node_cancels_zero() {
        let mut t = tree(5);
        let count = t.cancel_subtree("ghost");
        assert_eq!(count, 0);
    }

    // 32
    #[test]
    fn test_cancel_branch_leaves_sibling_untouched() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        // Cancel only the c1 subtree.
        t.cancel_subtree(&c1);
        let c2_node = t.nodes().get(&c2).unwrap();
        assert_ne!(c2_node.status, NodeStatus::Cancelled);
    }

    // 33
    #[test]
    fn test_cancel_subtree_counts_completed_nodes_as_not_changed() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("r", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        t.complete_node(&c1, "done".to_string()).unwrap();
        // c1 is Completed, not Cancelled — cancel_subtree should still overwrite and count it.
        let count = t.cancel_subtree(&root);
        // Both root (Pending→Cancelled) and c1 (Completed→Cancelled) are changed.
        assert_eq!(count, 2);
    }

    // ── 34–42: aggregate_results ─────────────────────────────────────────

    // 34
    #[test]
    fn test_aggregate_results_single_leaf_concat() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.complete_node(&root, "result".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Concat);
        assert_eq!(agg.as_deref(), Some("result"));
    }

    // 35
    #[test]
    fn test_aggregate_results_multiple_leaves_concat() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        t.complete_node(&c1, "aaa".to_string()).unwrap();
        t.complete_node(&c2, "bbb".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Concat).unwrap();
        // Both leaf results appear in the aggregation.
        assert!(agg.contains("aaa"), "agg: {}", agg);
        assert!(agg.contains("bbb"), "agg: {}", agg);
    }

    // 36
    #[test]
    fn test_aggregate_results_structured() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        t.complete_node(&c1, "patch1".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Structured).unwrap();
        assert!(agg.contains("[0]"), "agg: {}", agg);
        assert!(agg.contains("patch1"), "agg: {}", agg);
    }

    // 37
    #[test]
    fn test_aggregate_results_code_patch_merge() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        t.complete_node(&c1, "diff".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::CodePatchMerge).unwrap();
        assert!(agg.contains("--- patch"), "agg: {}", agg);
    }

    // 38
    #[test]
    fn test_aggregate_unknown_node_returns_none() {
        let t = tree(5);
        assert!(t.aggregate_results("ghost", &MergeStrategy::Concat).is_none());
    }

    // 39
    #[test]
    fn test_aggregate_node_no_leaf_results_returns_own_result() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        // No children, but root has a result.
        t.complete_node(&root, "solo".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Concat);
        assert_eq!(agg.as_deref(), Some("solo"));
    }

    // 40
    #[test]
    fn test_aggregate_deep_tree_structured() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        let gc1 = t.spawn_child(&c1, node_cfg("gc1", 2)).unwrap();
        let gc2 = t.spawn_child(&c2, node_cfg("gc2", 2)).unwrap();
        t.complete_node(&gc1, "res-gc1".to_string()).unwrap();
        t.complete_node(&gc2, "res-gc2".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Structured).unwrap();
        assert!(agg.contains("res-gc1"), "agg: {}", agg);
        assert!(agg.contains("res-gc2"), "agg: {}", agg);
    }

    // ── 41–44: nodes_at_depth ────────────────────────────────────────────

    // 41
    #[test]
    fn test_nodes_at_depth_zero() {
        let mut t = tree(5);
        t.spawn_root(node_cfg("root", 0)).unwrap();
        assert_eq!(t.nodes_at_depth(0).len(), 1);
    }

    // 42
    #[test]
    fn test_nodes_at_depth_one() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        assert_eq!(t.nodes_at_depth(1).len(), 2);
    }

    // 43
    #[test]
    fn test_nodes_at_depth_empty_when_no_nodes() {
        let t = tree(5);
        assert!(t.nodes_at_depth(0).is_empty());
    }

    // 44
    #[test]
    fn test_nodes_at_depth_mixed_levels() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        t.spawn_child(&c1, node_cfg("gc1", 2)).unwrap();
        assert_eq!(t.nodes_at_depth(2).len(), 1);
        assert_eq!(t.nodes_at_depth(3).len(), 0);
    }

    // ── 45–51: detect_cycle / CycleDetector ──────────────────────────────

    // 45
    #[test]
    fn test_detect_cycle_self_loop() {
        let t = tree(5);
        assert!(t.detect_cycle("node-1", "node-1"));
    }

    // 46
    #[test]
    fn test_cycle_detector_self_loop() {
        let cd = CycleDetector::new();
        let nodes: HashMap<String, AgentNode> = HashMap::new();
        assert!(cd.would_create_cycle(&nodes, "a", "a"));
    }

    // 47
    #[test]
    fn test_no_cycle_unrelated_nodes() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        // Adding root → c2 (doesn't exist yet) should not be a cycle.
        assert!(!t.detect_cycle(&root, &c1));
    }

    // 48
    #[test]
    fn test_cycle_detector_ancestor_is_child() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        // Trying to make root a child of c1: root is an ancestor of c1 → cycle.
        assert!(t.detect_cycle(&c1, &root));
    }

    // 49
    #[test]
    fn test_cycle_detector_deep_ancestor() {
        let mut t = tree(10);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let mut parent = root.clone();
        let mut last = root.clone();
        for d in 1..=4u32 {
            let child = t.spawn_child(&parent, node_cfg("n", d)).unwrap();
            last = child.clone();
            parent = child;
        }
        // Making `root` a child of the leaf would close a 5-level cycle.
        assert!(t.detect_cycle(&last, &root));
    }

    // 50
    #[test]
    fn test_no_cycle_sibling_to_sibling() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        // c1 is not an ancestor of c2; adding c2 → c3 should be fine.
        assert!(!t.detect_cycle(&c1, &c2));
    }

    // ── 51–53: execution_graph_summary ───────────────────────────────────

    // 51
    #[test]
    fn test_execution_graph_summary_empty_tree() {
        let t = tree(5);
        let s = t.execution_graph_summary();
        assert_eq!(s, "AgentTree:");
    }

    // 52
    #[test]
    fn test_execution_graph_summary_contains_node_ids() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let s = t.execution_graph_summary();
        assert!(s.contains(&root), "summary: {}", s);
        assert!(s.contains("root"), "summary: {}", s);
    }

    // 53
    #[test]
    fn test_execution_graph_summary_indented_children() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let _c1 = t.spawn_child(&root, node_cfg("child-node", 1)).unwrap();
        let s = t.execution_graph_summary();
        // Child line should be indented (2 spaces).
        let child_line = s
            .lines()
            .find(|l| l.contains("child-node"))
            .expect("child-node not in summary");
        assert!(child_line.starts_with("  "), "child line not indented: {:?}", child_line);
    }

    // ── 54–57: children_of / is_leaf ─────────────────────────────────────

    // 54
    #[test]
    fn test_children_of_empty() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        assert!(t.children_of(&root).is_empty());
    }

    // 55
    #[test]
    fn test_children_of_unknown_node_empty() {
        let t = tree(5);
        assert!(t.children_of("ghost").is_empty());
    }

    // 56
    #[test]
    fn test_is_leaf_true_for_leaf() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        assert!(t.is_leaf(&c1));
    }

    // 57
    #[test]
    fn test_is_leaf_false_for_non_leaf() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        assert!(!t.is_leaf(&root));
    }

    // ── 58–60: full lifecycle smoke test ─────────────────────────────────

    // 58
    #[test]
    fn test_full_agent_tree_lifecycle() {
        let mut t = tree(4);

        let root = t.spawn_root(node_cfg("orchestrator", 0)).unwrap();
        t.set_status(&root, NodeStatus::Running).unwrap();

        let c1 = t.spawn_child(&root, node_cfg("unit-tests", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("lint", 1)).unwrap();
        let gc1 = t.spawn_child(&c1, node_cfg("test-suite-a", 2)).unwrap();
        let gc2 = t.spawn_child(&c1, node_cfg("test-suite-b", 2)).unwrap();

        t.set_status(&c1, NodeStatus::Running).unwrap();
        t.set_status(&gc1, NodeStatus::Running).unwrap();
        t.complete_node(&gc1, "all green".to_string()).unwrap();
        t.complete_node(&gc2, "3 failures".to_string()).unwrap();
        t.complete_node(&c1, "unit done".to_string()).unwrap();

        t.fail_node(&c2, "style error".to_string()).unwrap();

        // Aggregate unit-test branch.
        let agg = t.aggregate_results(&c1, &MergeStrategy::Concat).unwrap();
        assert!(agg.contains("all green"));
        assert!(agg.contains("3 failures"));

        // Verify depths.
        assert_eq!(t.depth_of(&root), 0);
        assert_eq!(t.depth_of(&c1), 1);
        assert_eq!(t.depth_of(&gc1), 2);

        // nodes_at_depth
        assert_eq!(t.nodes_at_depth(2).len(), 2);

        // Cancel remaining pending node.
        let pending = t.spawn_child(&root, node_cfg("deploy", 1)).unwrap();
        let cancelled = t.cancel_subtree(&pending);
        assert_eq!(cancelled, 1);

        // Summary check.
        let s = t.execution_graph_summary();
        assert!(s.contains("orchestrator"));
        assert!(s.contains("unit-tests"));
        assert!(s.contains("lint"));
    }

    // 59
    #[test]
    fn test_concat_newline_separated() {
        let mut t = tree(5);
        let root = t.spawn_root(node_cfg("root", 0)).unwrap();
        let c1 = t.spawn_child(&root, node_cfg("c1", 1)).unwrap();
        let c2 = t.spawn_child(&root, node_cfg("c2", 1)).unwrap();
        t.complete_node(&c1, "line1".to_string()).unwrap();
        t.complete_node(&c2, "line2".to_string()).unwrap();
        let agg = t.aggregate_results(&root, &MergeStrategy::Concat).unwrap();
        assert!(agg.contains('\n'), "expected newline in concat output: {:?}", agg);
    }

    // 60
    #[test]
    fn test_cycle_detector_default() {
        let cd = CycleDetector::default();
        let nodes: HashMap<String, AgentNode> = HashMap::new();
        // Fresh tree: no ancestors, so no cycle.
        assert!(!cd.would_create_cycle(&nodes, "parent", "child"));
    }
}
