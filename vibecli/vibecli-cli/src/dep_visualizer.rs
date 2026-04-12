#![allow(dead_code)]
//! Dependency graph visualizer — generates import graphs in Mermaid and DOT
//! format, detects cycles, and computes coupling metrics.
//!
//! Matches Cursor 4.0's dependency graph visualizer.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Graph types
// ---------------------------------------------------------------------------

/// A node in the dependency graph.
#[derive(Debug, Clone)]
pub struct DepNode {
    pub id: String,
    /// Display label (often the module/file name).
    pub label: String,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// Node kind (module, file, package, etc.).
    pub kind: NodeKind,
}

impl DepNode {
    pub fn module(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            path: None,
            kind: NodeKind::Module,
        }
    }

    pub fn file(id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let label = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        Self {
            id: id.into(),
            label,
            path: Some(path),
            kind: NodeKind::File,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Module,
    File,
    Package,
    External,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeKind::Module => write!(f, "module"),
            NodeKind::File => write!(f, "file"),
            NodeKind::Package => write!(f, "package"),
            NodeKind::External => write!(f, "external"),
        }
    }
}

/// A directed edge (dependency) in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    /// Edge label (e.g. "imports", "uses").
    pub label: Option<String>,
}

impl DepEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Dependency graph
// ---------------------------------------------------------------------------

/// Directed dependency graph with cycle detection and metrics.
pub struct DepGraph {
    nodes: HashMap<String, DepNode>,
    edges: Vec<DepEdge>,
    /// Adjacency list (outgoing).
    adj: HashMap<String, Vec<String>>,
    /// Reverse adjacency list (incoming).
    rev_adj: HashMap<String, Vec<String>>,
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: vec![],
            adj: HashMap::new(),
            rev_adj: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: DepNode) {
        self.adj.entry(node.id.clone()).or_default();
        self.rev_adj.entry(node.id.clone()).or_default();
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: DepEdge) {
        self.adj
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        self.rev_adj
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        self.edges.push(edge);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Direct dependencies of a node.
    pub fn dependencies(&self, id: &str) -> &[String] {
        self.adj.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Direct dependents of a node (nodes that import it).
    pub fn dependents(&self, id: &str) -> &[String] {
        self.rev_adj.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Transitive dependencies (BFS).
    pub fn transitive_deps(&self, id: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(id.to_string());
        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            for dep in self.adj.get(&current).into_iter().flatten() {
                queue.push_back(dep.clone());
            }
        }
        visited.remove(id);
        visited
    }

    /// Detect cycles using DFS. Returns list of cycles (each as a path).
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = vec![];
        let mut cycles = vec![];

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                self.dfs_cycle(
                    node_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }
        cycles
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        for neighbor in self.adj.get(node).into_iter().flatten() {
            if !visited.contains(neighbor) {
                self.dfs_cycle(neighbor, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(neighbor) {
                // Found a cycle — extract it.
                let start = path.iter().position(|n| n == neighbor).unwrap_or(0);
                let cycle: Vec<String> = path[start..].to_vec();
                cycles.push(cycle);
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Compute in-degree (number of dependents) for each node.
    pub fn in_degrees(&self) -> HashMap<String, usize> {
        self.nodes
            .keys()
            .map(|id| {
                let deg = self.rev_adj.get(id).map(|v| v.len()).unwrap_or(0);
                (id.clone(), deg)
            })
            .collect()
    }

    /// Coupling metric: fan-out (outgoing edges) + fan-in (incoming edges).
    pub fn coupling(&self, id: &str) -> usize {
        let fan_out = self.adj.get(id).map(|v| v.len()).unwrap_or(0);
        let fan_in = self.rev_adj.get(id).map(|v| v.len()).unwrap_or(0);
        fan_out + fan_in
    }

    /// Most coupled nodes (highest fan-in + fan-out), descending.
    pub fn top_coupled(&self, n: usize) -> Vec<(String, usize)> {
        let mut scores: Vec<(String, usize)> = self
            .nodes
            .keys()
            .map(|id| (id.clone(), self.coupling(id)))
            .collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        scores.truncate(n);
        scores
    }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Renderers
// ---------------------------------------------------------------------------

/// Render a dependency graph as a Mermaid diagram.
pub fn render_mermaid(graph: &DepGraph, title: &str) -> String {
    let mut out = format!("---\ntitle: {title}\n---\ngraph TD\n");
    for node in graph.nodes.values() {
        let label = &node.label;
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("    {}[\"{}\"]\n", sanitize_id(&node.id), label),
        );
    }
    for edge in &graph.edges {
        let label_str = edge
            .label
            .as_deref()
            .map(|l| format!(" -- {l} -->"))
            .unwrap_or_else(|| " --> ".to_string());
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "    {}{}{}\n",
                sanitize_id(&edge.from),
                label_str,
                sanitize_id(&edge.to)
            ),
        );
    }
    out
}

/// Render a dependency graph as a DOT (Graphviz) diagram.
pub fn render_dot(graph: &DepGraph, name: &str) -> String {
    let mut out = format!("digraph {name} {{\n  rankdir=LR;\n  node [shape=box];\n");
    for node in graph.nodes.values() {
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("  \"{}\" [label=\"{}\"];\n", node.id, node.label),
        );
    }
    for edge in &graph.edges {
        let label = edge
            .label
            .as_deref()
            .map(|l| format!(" [label=\"{l}\"]"))
            .unwrap_or_default();
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("  \"{}\" -> \"{}\"{};\n", edge.from, edge.to, label),
        );
    }
    out.push('}');
    out
}

fn sanitize_id(id: &str) -> String {
    id.replace(['/', '.', '-', ':'], "_")
}

// ---------------------------------------------------------------------------
// Graph builder (from file paths + heuristic imports)
// ---------------------------------------------------------------------------

/// Builds a DepGraph from a list of (file_path, imports) pairs.
pub fn build_from_imports(items: &[(PathBuf, Vec<String>)]) -> DepGraph {
    let mut g = DepGraph::new();

    // First pass: add all nodes.
    for (path, _) in items {
        let id = path.to_string_lossy().to_string();
        g.add_node(DepNode::file(id, path));
    }

    // Second pass: add edges.
    for (path, imports) in items {
        let from_id = path.to_string_lossy().to_string();
        for import in imports {
            // If the import matches a known node, add an edge.
            let to_id = import.clone();
            if g.nodes.contains_key(&to_id) {
                g.add_edge(DepEdge::new(&from_id, &to_id));
            } else {
                // Add as external node.
                g.add_node(DepNode {
                    id: to_id.clone(),
                    label: to_id.clone(),
                    path: None,
                    kind: NodeKind::External,
                });
                g.add_edge(DepEdge::new(&from_id, &to_id));
            }
        }
    }
    g
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn graph_with_cycle() -> DepGraph {
        let mut g = DepGraph::new();
        g.add_node(DepNode::module("a", "A"));
        g.add_node(DepNode::module("b", "B"));
        g.add_node(DepNode::module("c", "C"));
        g.add_edge(DepEdge::new("a", "b"));
        g.add_edge(DepEdge::new("b", "c"));
        g.add_edge(DepEdge::new("c", "a")); // cycle
        g
    }

    fn acyclic_graph() -> DepGraph {
        let mut g = DepGraph::new();
        g.add_node(DepNode::module("utils", "Utils"));
        g.add_node(DepNode::module("lib", "Lib"));
        g.add_node(DepNode::module("main", "Main"));
        g.add_edge(DepEdge::new("lib", "utils"));
        g.add_edge(DepEdge::new("main", "lib"));
        g
    }

    #[test]
    fn test_node_and_edge_count() {
        let g = acyclic_graph();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn test_dependencies() {
        let g = acyclic_graph();
        let deps = g.dependencies("lib");
        assert!(deps.contains(&"utils".to_string()));
    }

    #[test]
    fn test_dependents() {
        let g = acyclic_graph();
        let deps = g.dependents("lib");
        assert!(deps.contains(&"main".to_string()));
    }

    #[test]
    fn test_transitive_deps() {
        let g = acyclic_graph();
        let trans = g.transitive_deps("main");
        assert!(trans.contains("lib"));
        assert!(trans.contains("utils"));
    }

    #[test]
    fn test_cycle_detection() {
        let g = graph_with_cycle();
        let cycles = g.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_no_cycles_in_acyclic_graph() {
        let g = acyclic_graph();
        let cycles = g.detect_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_in_degrees() {
        let g = acyclic_graph();
        let degrees = g.in_degrees();
        assert_eq!(degrees["utils"], 1); // imported by lib
        assert_eq!(degrees["main"], 0); // nothing imports main
    }

    #[test]
    fn test_coupling_score() {
        let g = acyclic_graph();
        // lib: fan-in=1 (main), fan-out=1 (utils) → coupling = 2
        assert_eq!(g.coupling("lib"), 2);
    }

    #[test]
    fn test_top_coupled() {
        let g = acyclic_graph();
        let top = g.top_coupled(1);
        assert_eq!(top[0].0, "lib"); // lib has highest coupling
    }

    #[test]
    fn test_render_mermaid_contains_nodes() {
        let g = acyclic_graph();
        let mermaid = render_mermaid(&g, "Test Deps");
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("Utils") || mermaid.contains("utils"));
    }

    #[test]
    fn test_render_mermaid_contains_edges() {
        let g = acyclic_graph();
        let mermaid = render_mermaid(&g, "Test");
        assert!(mermaid.contains("-->"));
    }

    #[test]
    fn test_render_dot_contains_digraph() {
        let g = acyclic_graph();
        let dot = render_dot(&g, "MyGraph");
        assert!(dot.contains("digraph MyGraph"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn test_build_from_imports() {
        let items = vec![
            (PathBuf::from("src/lib.rs"), vec!["src/utils.rs".to_string()]),
            (PathBuf::from("src/utils.rs"), vec![]),
        ];
        let g = build_from_imports(&items);
        assert!(g.node_count() >= 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_edge_label() {
        let mut g = DepGraph::new();
        g.add_node(DepNode::module("a", "A"));
        g.add_node(DepNode::module("b", "B"));
        g.add_edge(DepEdge::new("a", "b").with_label("uses"));
        let dot = render_dot(&g, "test");
        assert!(dot.contains("uses"));
    }

    #[test]
    fn test_external_node_added_on_unknown_import() {
        let items = vec![(
            PathBuf::from("src/main.rs"),
            vec!["external_crate".to_string()],
        )];
        let g = build_from_imports(&items);
        assert!(g.nodes.contains_key("external_crate"));
    }
}
