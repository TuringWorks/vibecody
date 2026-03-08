//! Cross-repository knowledge graph for semantic code intelligence.
//!
//! Builds a graph of symbols (functions, structs, traits, modules) and their
//! relationships (calls, imports, implements) across multiple repositories.
//! Supports queries like "who calls this?", "who implements this trait?",
//! and shortest-path between symbols.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

// ── Node types ───────────────────────────────────────────────────────────────

pub type NodeId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeKind {
    Function,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    Module,
    File,
    Repo,
    Constant,
    Type,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Class => "class",
            Self::Module => "module",
            Self::File => "file",
            Self::Repo => "repo",
            Self::Constant => "constant",
            Self::Type => "type",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub repo: String,
    pub file: PathBuf,
    pub line: usize,
    pub signature: String,
}

// ── Edge types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeKind {
    Calls,
    Implements,
    Extends,
    Imports,
    Contains,
    References,
    DependsOn,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Implements => "implements",
            Self::Extends => "extends",
            Self::Imports => "imports",
            Self::Contains => "contains",
            Self::References => "references",
            Self::DependsOn => "depends_on",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

// ── Graph stats ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes_per_repo: HashMap<String, usize>,
    pub edges_per_repo: HashMap<String, usize>,
    pub cross_repo_edges: usize,
    pub most_connected: Vec<(String, usize)>,
    pub orphan_count: usize,
}

// ── Knowledge Graph ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: Vec<GraphEdge>,
    repos: HashMap<String, PathBuf>,
    next_id: NodeId,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            repos: HashMap::new(),
            next_id: 1,
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Register a repository and scan it for symbols.
    pub fn add_repo(&mut self, name: &str, path: &Path) -> Result<usize> {
        self.repos.insert(name.to_string(), path.to_path_buf());

        // Add a repo node
        let repo_id = self.alloc_id();
        self.nodes.insert(repo_id, GraphNode {
            id: repo_id,
            name: name.to_string(),
            kind: NodeKind::Repo,
            repo: name.to_string(),
            file: path.to_path_buf(),
            line: 0,
            signature: String::new(),
        });

        let mut count = 0;
        // Walk the directory and extract symbols
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if p.is_dir() { continue; }

            // Skip hidden dirs, node_modules, target, .git
            let path_str = p.to_string_lossy();
            if path_str.contains("/.") || path_str.contains("/node_modules/")
                || path_str.contains("/target/") || path_str.contains("/.git/") {
                continue;
            }

            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" | "mjs" => "javascript",
                "py" | "pyi" => "python",
                "go" => "go",
                _ => continue,
            };

            let content = match std::fs::read_to_string(p) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Add file node
            let file_id = self.alloc_id();
            let file_name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            self.nodes.insert(file_id, GraphNode {
                id: file_id,
                name: file_name,
                kind: NodeKind::File,
                repo: name.to_string(),
                file: p.to_path_buf(),
                line: 0,
                signature: String::new(),
            });
            self.edges.push(GraphEdge {
                from: repo_id,
                to: file_id,
                kind: EdgeKind::Contains,
            });

            // Extract symbols from content
            let symbols = extract_symbols_for_graph(p, &content, lang);
            for (sym_name, sym_kind, line, sig) in symbols {
                let sym_id = self.alloc_id();
                self.nodes.insert(sym_id, GraphNode {
                    id: sym_id,
                    name: sym_name,
                    kind: sym_kind,
                    repo: name.to_string(),
                    file: p.to_path_buf(),
                    line,
                    signature: sig,
                });
                self.edges.push(GraphEdge {
                    from: file_id,
                    to: sym_id,
                    kind: EdgeKind::Contains,
                });
                count += 1;
            }

            // Extract import edges
            let imports = extract_imports(&content, lang);
            for imported in imports {
                // Store as a deferred reference — resolved in build_edges
                let imp_id = self.alloc_id();
                self.nodes.insert(imp_id, GraphNode {
                    id: imp_id,
                    name: imported.clone(),
                    kind: NodeKind::Module,
                    repo: name.to_string(),
                    file: p.to_path_buf(),
                    line: 0,
                    signature: format!("import {}", imported),
                });
                self.edges.push(GraphEdge {
                    from: file_id,
                    to: imp_id,
                    kind: EdgeKind::Imports,
                });
            }
        }

        Ok(count)
    }

    /// Add a node directly (for testing or manual construction).
    pub fn add_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        id
    }

    /// Add an edge directly.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Resolve cross-references by matching import names to actual symbol nodes.
    pub fn build_edges(&mut self) {
        let symbol_names: HashMap<String, Vec<NodeId>> = {
            let mut map: HashMap<String, Vec<NodeId>> = HashMap::new();
            for (id, node) in &self.nodes {
                if !matches!(node.kind, NodeKind::File | NodeKind::Repo) {
                    map.entry(node.name.clone()).or_default().push(*id);
                }
            }
            map
        };

        let mut new_edges = Vec::new();

        // For each symbol, look for call patterns in the same file
        let nodes_snapshot: Vec<(NodeId, GraphNode)> = self.nodes.iter()
            .map(|(id, n)| (*id, n.clone()))
            .collect();

        for (id, node) in &nodes_snapshot {
            if matches!(node.kind, NodeKind::Function | NodeKind::Struct | NodeKind::Class) {
                // Find references to other symbols in this node's signature
                for (name, targets) in &symbol_names {
                    if name != &node.name && node.signature.contains(name.as_str()) {
                        for &target_id in targets {
                            if target_id != *id {
                                new_edges.push(GraphEdge {
                                    from: *id,
                                    to: target_id,
                                    kind: EdgeKind::References,
                                });
                            }
                        }
                    }
                }
            }
        }

        self.edges.extend(new_edges);
    }

    /// Find all nodes that have an edge pointing to the given symbol.
    pub fn query_callers(&self, symbol: &str) -> Vec<&GraphNode> {
        let target_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.name == symbol)
            .map(|(id, _)| *id)
            .collect();

        let caller_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| target_ids.contains(&e.to) && matches!(e.kind, EdgeKind::Calls | EdgeKind::References))
            .map(|e| e.from)
            .collect();

        caller_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Find all nodes that the given symbol has edges pointing to.
    pub fn query_callees(&self, symbol: &str) -> Vec<&GraphNode> {
        let source_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.name == symbol)
            .map(|(id, _)| *id)
            .collect();

        let callee_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| source_ids.contains(&e.from) && matches!(e.kind, EdgeKind::Calls | EdgeKind::References))
            .map(|e| e.to)
            .collect();

        callee_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Find all nodes that implement a given trait/interface.
    pub fn query_implementors(&self, trait_name: &str) -> Vec<&GraphNode> {
        let trait_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.name == trait_name && matches!(n.kind, NodeKind::Trait | NodeKind::Interface))
            .map(|(id, _)| *id)
            .collect();

        let impl_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| trait_ids.contains(&e.to) && matches!(e.kind, EdgeKind::Implements))
            .map(|e| e.from)
            .collect();

        impl_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Find what a file depends on (imports).
    pub fn query_dependencies(&self, file: &Path) -> Vec<&GraphNode> {
        let file_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.file == file && matches!(n.kind, NodeKind::File))
            .map(|(id, _)| *id)
            .collect();

        let dep_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| file_ids.contains(&e.from) && matches!(e.kind, EdgeKind::Imports | EdgeKind::DependsOn))
            .map(|e| e.to)
            .collect();

        dep_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Find what depends on a file.
    pub fn query_dependents(&self, file: &Path) -> Vec<&GraphNode> {
        let file_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.file == file && matches!(n.kind, NodeKind::File))
            .map(|(id, _)| *id)
            .collect();

        // Find symbols in this file
        let symbol_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| file_ids.contains(&e.from) && matches!(e.kind, EdgeKind::Contains))
            .map(|e| e.to)
            .collect();

        let all_ids: HashSet<NodeId> = file_ids.union(&symbol_ids).copied().collect();

        let dependent_ids: HashSet<NodeId> = self.edges.iter()
            .filter(|e| all_ids.contains(&e.to) && matches!(e.kind, EdgeKind::Imports | EdgeKind::DependsOn | EdgeKind::References))
            .map(|e| e.from)
            .collect();

        dependent_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Find edges that cross repository boundaries.
    pub fn cross_repo_references(&self, repo: &str) -> Vec<(&GraphNode, &GraphNode, &EdgeKind)> {
        self.edges.iter()
            .filter_map(|e| {
                let from = self.nodes.get(&e.from)?;
                let to = self.nodes.get(&e.to)?;
                if (from.repo == repo || to.repo == repo) && from.repo != to.repo {
                    Some((from, to, &e.kind))
                } else {
                    None
                }
            })
            .collect()
    }

    /// BFS shortest path between two symbols by name.
    pub fn shortest_path(&self, from_name: &str, to_name: &str) -> Option<Vec<NodeId>> {
        let from_id = self.nodes.iter()
            .find(|(_, n)| n.name == from_name)
            .map(|(id, _)| *id)?;
        let to_id = self.nodes.iter()
            .find(|(_, n)| n.name == to_name)
            .map(|(id, _)| *id)?;

        if from_id == to_id {
            return Some(vec![from_id]);
        }

        // Build adjacency list (undirected)
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for e in &self.edges {
            adj.entry(e.from).or_default().push(e.to);
            adj.entry(e.to).or_default().push(e.from);
        }

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<Vec<NodeId>> = VecDeque::new();
        visited.insert(from_id);
        queue.push_back(vec![from_id]);

        while let Some(path) = queue.pop_front() {
            let current = *path.last().expect("path should not be empty");
            if let Some(neighbors) = adj.get(&current) {
                for &next in neighbors {
                    if next == to_id {
                        let mut result = path.clone();
                        result.push(next);
                        return Some(result);
                    }
                    if visited.insert(next) {
                        let mut new_path = path.clone();
                        new_path.push(next);
                        queue.push_back(new_path);
                    }
                }
            }
        }

        None
    }

    /// Extract a neighborhood subgraph around a symbol.
    pub fn subgraph(&self, symbol: &str, depth: usize) -> KnowledgeGraph {
        let mut sub = KnowledgeGraph::new();
        sub.repos = self.repos.clone();

        let start_ids: HashSet<NodeId> = self.nodes.iter()
            .filter(|(_, n)| n.name == symbol)
            .map(|(id, _)| *id)
            .collect();

        if start_ids.is_empty() {
            return sub;
        }

        // BFS expansion
        let mut visited: HashSet<NodeId> = start_ids.clone();
        let mut frontier: HashSet<NodeId> = start_ids;

        // Build adjacency
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for e in &self.edges {
            adj.entry(e.from).or_default().push(e.to);
            adj.entry(e.to).or_default().push(e.from);
        }

        for _ in 0..depth {
            let mut next_frontier = HashSet::new();
            for &node_id in &frontier {
                if let Some(neighbors) = adj.get(&node_id) {
                    for &n in neighbors {
                        if visited.insert(n) {
                            next_frontier.insert(n);
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        // Copy nodes and edges
        for &id in &visited {
            if let Some(node) = self.nodes.get(&id) {
                sub.add_node(node.clone());
            }
        }
        for e in &self.edges {
            if visited.contains(&e.from) && visited.contains(&e.to) {
                sub.add_edge(e.clone());
            }
        }

        sub
    }

    /// Compute graph statistics.
    pub fn stats(&self) -> GraphStats {
        let mut nodes_per_repo: HashMap<String, usize> = HashMap::new();
        let mut edges_per_repo: HashMap<String, usize> = HashMap::new();
        let mut degree: HashMap<NodeId, usize> = HashMap::new();
        let mut connected: HashSet<NodeId> = HashSet::new();
        let mut cross_repo = 0;

        for (_, node) in &self.nodes {
            *nodes_per_repo.entry(node.repo.clone()).or_default() += 1;
        }

        for e in &self.edges {
            connected.insert(e.from);
            connected.insert(e.to);
            *degree.entry(e.from).or_default() += 1;
            *degree.entry(e.to).or_default() += 1;

            if let (Some(from), Some(to)) = (self.nodes.get(&e.from), self.nodes.get(&e.to)) {
                *edges_per_repo.entry(from.repo.clone()).or_default() += 1;
                if from.repo != to.repo {
                    cross_repo += 1;
                }
            }
        }

        let mut most_connected: Vec<(String, usize)> = degree.iter()
            .filter_map(|(id, deg)| {
                self.nodes.get(id).map(|n| (n.name.clone(), *deg))
            })
            .collect();
        most_connected.sort_by(|a, b| b.1.cmp(&a.1));
        most_connected.truncate(10);

        let orphan_count = self.nodes.len() - connected.len();

        GraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            nodes_per_repo,
            edges_per_repo,
            cross_repo_edges: cross_repo,
            most_connected,
            orphan_count,
        }
    }

    /// Export to Graphviz DOT format.
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph KnowledgeGraph {\n  rankdir=LR;\n  node [shape=box];\n\n");

        for (id, node) in &self.nodes {
            let shape = match node.kind {
                NodeKind::Function => "ellipse",
                NodeKind::Struct | NodeKind::Class => "box",
                NodeKind::Trait | NodeKind::Interface => "diamond",
                NodeKind::Module => "folder",
                NodeKind::File => "note",
                NodeKind::Repo => "house",
                _ => "box",
            };
            dot.push_str(&format!(
                "  n{} [label=\"{}\\n({})\" shape={}];\n",
                id, node.name, node.kind.as_str(), shape
            ));
        }

        dot.push('\n');

        for e in &self.edges {
            dot.push_str(&format!(
                "  n{} -> n{} [label=\"{}\"];\n",
                e.from, e.to, e.kind.as_str()
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Save graph to JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("failed to serialize knowledge graph")?;
        std::fs::write(path, json)
            .context("failed to write knowledge graph file")?;
        Ok(())
    }

    /// Load graph from JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .context("failed to read knowledge graph file")?;
        let graph: Self = serde_json::from_str(&json)
            .context("failed to deserialize knowledge graph")?;
        Ok(graph)
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
    pub fn repo_count(&self) -> usize { self.repos.len() }

    pub fn get_node(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.get(&id)
    }

    pub fn find_nodes_by_name(&self, name: &str) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.name == name).collect()
    }

    pub fn find_nodes_by_kind(&self, kind: &NodeKind) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| &n.kind == kind).collect()
    }

    pub fn find_nodes_in_repo(&self, repo: &str) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.repo == repo).collect()
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ── Symbol extraction helpers ────────────────────────────────────────────────

fn extract_symbols_for_graph(
    path: &Path,
    content: &str,
    lang: &str,
) -> Vec<(String, NodeKind, usize, String)> {
    let patterns: &[(&str, NodeKind)] = match lang {
        "rust" => &[
            (r"(?:pub\s+)?fn\s+(\w+)", NodeKind::Function),
            (r"(?:pub\s+)?struct\s+(\w+)", NodeKind::Struct),
            (r"(?:pub\s+)?enum\s+(\w+)", NodeKind::Enum),
            (r"(?:pub\s+)?trait\s+(\w+)", NodeKind::Trait),
            (r"(?:pub\s+)?type\s+(\w+)", NodeKind::Type),
            (r"(?:pub\s+)?const\s+(\w+)", NodeKind::Constant),
            (r"(?:pub\s+)?mod\s+(\w+)", NodeKind::Module),
        ],
        "typescript" | "javascript" => &[
            (r"(?:export\s+)?function\s+(\w+)", NodeKind::Function),
            (r"(?:export\s+)?class\s+(\w+)", NodeKind::Class),
            (r"(?:export\s+)?interface\s+(\w+)", NodeKind::Interface),
            (r"(?:export\s+)?enum\s+(\w+)", NodeKind::Enum),
            (r"(?:export\s+)?type\s+(\w+)", NodeKind::Type),
            (r"(?:export\s+)?const\s+(\w+)", NodeKind::Constant),
        ],
        "python" => &[
            (r"def\s+(\w+)", NodeKind::Function),
            (r"class\s+(\w+)", NodeKind::Class),
        ],
        "go" => &[
            (r"func\s+(\w+)", NodeKind::Function),
            (r"type\s+(\w+)\s+struct", NodeKind::Struct),
            (r"type\s+(\w+)\s+interface", NodeKind::Interface),
        ],
        _ => return vec![],
    };

    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut seen: HashSet<(usize, String)> = HashSet::new();

    for (pattern, kind) in patterns {
        let re = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (line_idx, &line) in lines.iter().enumerate() {
            if let Some(cap) = re.captures(line) {
                if let Some(m) = cap.get(1) {
                    let name = m.as_str().to_string();
                    let key = (line_idx, name.clone());
                    if seen.insert(key) {
                        results.push((
                            name,
                            kind.clone(),
                            line_idx + 1,
                            line.trim().chars().take(120).collect(),
                        ));
                    }
                }
            }
        }
    }

    let _ = path; // used for context but not needed in extraction
    results
}

fn extract_imports(content: &str, lang: &str) -> Vec<String> {
    let patterns: &[&str] = match lang {
        "rust" => &[r"use\s+(?:crate::)?(\w+)", r"mod\s+(\w+)"],
        "typescript" | "javascript" => &[
            r#"import\s+.*?from\s+['"]([\w@/.-]+)['"]"#,
            r#"require\(['"]([\w@/.-]+)['"]\)"#,
        ],
        "python" => &[
            r"from\s+([\w.]+)\s+import",
            r"import\s+([\w.]+)",
        ],
        "go" => &[r#""([\w./]+)""#],
        _ => return vec![],
    };

    let mut imports = Vec::new();
    for pattern in patterns {
        let re = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                imports.push(m.as_str().to_string());
            }
        }
    }
    imports
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_node(id: NodeId, name: &str, kind: NodeKind, repo: &str) -> GraphNode {
        GraphNode {
            id,
            name: name.to_string(),
            kind,
            repo: repo.to_string(),
            file: PathBuf::from(format!("{}/src/lib.rs", repo)),
            line: 1,
            signature: String::new(),
        }
    }

    fn build_test_graph() -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(1, "foo", NodeKind::Function, "repo_a"));
        g.add_node(make_node(2, "bar", NodeKind::Function, "repo_a"));
        g.add_node(make_node(3, "baz", NodeKind::Function, "repo_b"));
        g.add_node(make_node(4, "MyTrait", NodeKind::Trait, "repo_a"));
        g.add_node(make_node(5, "MyStruct", NodeKind::Struct, "repo_b"));
        g.add_node(make_node(6, "helper", NodeKind::Function, "repo_b"));

        g.add_edge(GraphEdge { from: 1, to: 2, kind: EdgeKind::Calls });
        g.add_edge(GraphEdge { from: 1, to: 3, kind: EdgeKind::Calls });
        g.add_edge(GraphEdge { from: 2, to: 6, kind: EdgeKind::References });
        g.add_edge(GraphEdge { from: 5, to: 4, kind: EdgeKind::Implements });
        g.add_edge(GraphEdge { from: 3, to: 6, kind: EdgeKind::Calls });

        g.repos.insert("repo_a".to_string(), PathBuf::from("/tmp/repo_a"));
        g.repos.insert("repo_b".to_string(), PathBuf::from("/tmp/repo_b"));
        g
    }

    #[test]
    fn test_new_graph() {
        let g = KnowledgeGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.repo_count(), 0);
    }

    #[test]
    fn test_default_graph() {
        let g = KnowledgeGraph::default();
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut g = KnowledgeGraph::new();
        let node = make_node(10, "test_fn", NodeKind::Function, "repo");
        g.add_node(node);
        assert_eq!(g.node_count(), 1);
        assert!(g.get_node(10).is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(1, "a", NodeKind::Function, "r"));
        g.add_node(make_node(2, "b", NodeKind::Function, "r"));
        g.add_edge(GraphEdge { from: 1, to: 2, kind: EdgeKind::Calls });
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_query_callers() {
        let g = build_test_graph();
        let callers = g.query_callers("bar");
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].name, "foo");
    }

    #[test]
    fn test_query_callers_multiple() {
        let g = build_test_graph();
        let callers = g.query_callers("helper");
        assert_eq!(callers.len(), 2); // bar (references) and baz (calls)
    }

    #[test]
    fn test_query_callers_no_results() {
        let g = build_test_graph();
        let callers = g.query_callers("foo");
        assert_eq!(callers.len(), 0);
    }

    #[test]
    fn test_query_callees() {
        let g = build_test_graph();
        let callees = g.query_callees("foo");
        assert_eq!(callees.len(), 2); // bar and baz
    }

    #[test]
    fn test_query_callees_no_results() {
        let g = build_test_graph();
        let callees = g.query_callees("helper");
        assert_eq!(callees.len(), 0);
    }

    #[test]
    fn test_query_implementors() {
        let g = build_test_graph();
        let impls = g.query_implementors("MyTrait");
        assert_eq!(impls.len(), 1);
        assert_eq!(impls[0].name, "MyStruct");
    }

    #[test]
    fn test_query_implementors_no_results() {
        let g = build_test_graph();
        let impls = g.query_implementors("NonExistent");
        assert_eq!(impls.len(), 0);
    }

    #[test]
    fn test_cross_repo_references() {
        let g = build_test_graph();
        let cross = g.cross_repo_references("repo_a");
        assert!(!cross.is_empty());
        // foo(repo_a) -> baz(repo_b), bar(repo_a) -> helper(repo_b), MyStruct(repo_b) -> MyTrait(repo_a)
        assert!(cross.len() >= 2);
    }

    #[test]
    fn test_cross_repo_no_results() {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(1, "a", NodeKind::Function, "r1"));
        g.add_node(make_node(2, "b", NodeKind::Function, "r1"));
        g.add_edge(GraphEdge { from: 1, to: 2, kind: EdgeKind::Calls });
        let cross = g.cross_repo_references("r1");
        assert!(cross.is_empty());
    }

    #[test]
    fn test_shortest_path_direct() {
        let g = build_test_graph();
        let path = g.shortest_path("foo", "bar");
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0], 1); // foo
        assert_eq!(p[1], 2); // bar
    }

    #[test]
    fn test_shortest_path_indirect() {
        let g = build_test_graph();
        let path = g.shortest_path("foo", "helper");
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.len() <= 3); // foo -> bar/baz -> helper
    }

    #[test]
    fn test_shortest_path_same_node() {
        let g = build_test_graph();
        let path = g.shortest_path("foo", "foo");
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);
    }

    #[test]
    fn test_shortest_path_not_found() {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(1, "isolated_a", NodeKind::Function, "r"));
        g.add_node(make_node(2, "isolated_b", NodeKind::Function, "r"));
        let path = g.shortest_path("isolated_a", "isolated_b");
        assert!(path.is_none());
    }

    #[test]
    fn test_shortest_path_nonexistent() {
        let g = build_test_graph();
        let path = g.shortest_path("foo", "nonexistent");
        assert!(path.is_none());
    }

    #[test]
    fn test_subgraph_depth_0() {
        let g = build_test_graph();
        let sub = g.subgraph("foo", 0);
        assert_eq!(sub.node_count(), 1);
    }

    #[test]
    fn test_subgraph_depth_1() {
        let g = build_test_graph();
        let sub = g.subgraph("foo", 1);
        assert!(sub.node_count() >= 3); // foo + bar + baz
    }

    #[test]
    fn test_subgraph_nonexistent() {
        let g = build_test_graph();
        let sub = g.subgraph("nonexistent", 2);
        assert_eq!(sub.node_count(), 0);
    }

    #[test]
    fn test_stats() {
        let g = build_test_graph();
        let stats = g.stats();
        assert_eq!(stats.total_nodes, 6);
        assert_eq!(stats.total_edges, 5);
        assert!(stats.cross_repo_edges > 0);
        assert!(!stats.most_connected.is_empty());
    }

    #[test]
    fn test_stats_empty_graph() {
        let g = KnowledgeGraph::new();
        let stats = g.stats();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
        assert_eq!(stats.orphan_count, 0);
    }

    #[test]
    fn test_orphan_count() {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(1, "connected", NodeKind::Function, "r"));
        g.add_node(make_node(2, "connected2", NodeKind::Function, "r"));
        g.add_node(make_node(3, "orphan", NodeKind::Function, "r"));
        g.add_edge(GraphEdge { from: 1, to: 2, kind: EdgeKind::Calls });
        let stats = g.stats();
        assert_eq!(stats.orphan_count, 1);
    }

    #[test]
    fn test_to_dot() {
        let g = build_test_graph();
        let dot = g.to_dot();
        assert!(dot.starts_with("digraph"));
        assert!(dot.contains("foo"));
        assert!(dot.contains("bar"));
        assert!(dot.contains("calls"));
        assert!(dot.contains("implements"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_save_and_load() {
        let g = build_test_graph();
        let tmp = std::env::temp_dir().join("vibecody_kg_test.json");
        g.save(&tmp).expect("save failed");
        let loaded = KnowledgeGraph::load(&tmp).expect("load failed");
        assert_eq!(loaded.node_count(), g.node_count());
        assert_eq!(loaded.edge_count(), g.edge_count());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_find_nodes_by_name() {
        let g = build_test_graph();
        let nodes = g.find_nodes_by_name("foo");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].kind, NodeKind::Function);
    }

    #[test]
    fn test_find_nodes_by_kind() {
        let g = build_test_graph();
        let fns = g.find_nodes_by_kind(&NodeKind::Function);
        assert_eq!(fns.len(), 4); // foo, bar, baz, helper
    }

    #[test]
    fn test_find_nodes_in_repo() {
        let g = build_test_graph();
        let nodes = g.find_nodes_in_repo("repo_a");
        assert_eq!(nodes.len(), 3); // foo, bar, MyTrait
    }

    #[test]
    fn test_node_kind_as_str() {
        assert_eq!(NodeKind::Function.as_str(), "function");
        assert_eq!(NodeKind::Trait.as_str(), "trait");
        assert_eq!(NodeKind::Repo.as_str(), "repo");
    }

    #[test]
    fn test_edge_kind_as_str() {
        assert_eq!(EdgeKind::Calls.as_str(), "calls");
        assert_eq!(EdgeKind::Implements.as_str(), "implements");
        assert_eq!(EdgeKind::DependsOn.as_str(), "depends_on");
    }

    #[test]
    fn test_extract_symbols_rust() {
        let content = "pub fn hello() {}\nstruct Foo {}\ntrait Bar {}";
        let syms = extract_symbols_for_graph(Path::new("test.rs"), content, "rust");
        assert_eq!(syms.len(), 3);
    }

    #[test]
    fn test_extract_symbols_typescript() {
        let content = "export function greet() {}\nclass App {}\ninterface Config {}";
        let syms = extract_symbols_for_graph(Path::new("test.ts"), content, "typescript");
        assert_eq!(syms.len(), 3);
    }

    #[test]
    fn test_extract_symbols_python() {
        let content = "def hello():\n    pass\nclass World:\n    pass";
        let syms = extract_symbols_for_graph(Path::new("test.py"), content, "python");
        assert_eq!(syms.len(), 2);
    }

    #[test]
    fn test_extract_symbols_go() {
        let content = "func main() {}\ntype Config struct {}";
        let syms = extract_symbols_for_graph(Path::new("test.go"), content, "go");
        assert_eq!(syms.len(), 2);
    }

    #[test]
    fn test_extract_symbols_unknown_lang() {
        let syms = extract_symbols_for_graph(Path::new("test.xyz"), "anything", "unknown");
        assert!(syms.is_empty());
    }

    #[test]
    fn test_extract_imports_rust() {
        let content = "use std::path::Path;\nuse crate::config;\nmod utils;";
        let imports = extract_imports(content, "rust");
        assert!(imports.len() >= 2);
    }

    #[test]
    fn test_extract_imports_typescript() {
        let content = r#"import { foo } from "./utils";
import bar from "lodash";"#;
        let imports = extract_imports(content, "typescript");
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_extract_imports_python() {
        let content = "from os.path import join\nimport sys";
        let imports = extract_imports(content, "python");
        // "from" pattern matches "os.path", "import" pattern matches both "join" and "sys"
        assert!(imports.len() >= 2);
        assert!(imports.contains(&"os.path".to_string()));
        assert!(imports.contains(&"sys".to_string()));
    }

    #[test]
    fn test_build_edges_resolves_references() {
        let mut g = KnowledgeGraph::new();
        g.add_node(GraphNode {
            id: 1, name: "process".to_string(), kind: NodeKind::Function,
            repo: "r".to_string(), file: PathBuf::from("a.rs"), line: 1,
            signature: "fn process(data: Config)".to_string(),
        });
        g.add_node(GraphNode {
            id: 2, name: "Config".to_string(), kind: NodeKind::Struct,
            repo: "r".to_string(), file: PathBuf::from("b.rs"), line: 1,
            signature: "struct Config {}".to_string(),
        });
        g.build_edges();
        assert!(g.edge_count() >= 1);
    }

    #[test]
    fn test_query_dependencies() {
        let mut g = KnowledgeGraph::new();
        let file = PathBuf::from("/tmp/repo/src/main.rs");
        g.add_node(GraphNode {
            id: 1, name: "main.rs".to_string(), kind: NodeKind::File,
            repo: "r".to_string(), file: file.clone(), line: 0,
            signature: String::new(),
        });
        g.add_node(GraphNode {
            id: 2, name: "utils".to_string(), kind: NodeKind::Module,
            repo: "r".to_string(), file: PathBuf::from("/tmp/repo/src/utils.rs"), line: 0,
            signature: String::new(),
        });
        g.add_edge(GraphEdge { from: 1, to: 2, kind: EdgeKind::Imports });
        let deps = g.query_dependencies(&file);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "utils");
    }

    #[test]
    fn test_alloc_id_increments() {
        let mut g = KnowledgeGraph::new();
        let id1 = g.alloc_id();
        let id2 = g.alloc_id();
        assert_eq!(id2, id1 + 1);
    }
}
