//! `CodeGraph` — the central code knowledge graph.
//!
//! Consolidates the audited `vibecli/.../semantic_index.rs::SemanticIndex` (rich typed
//! edge views) and `vibecli/.../dep_visualizer.rs::DepGraph` (adjacency + cycle/coupling
//! analysis) onto a single [`petgraph`] backbone.
//!
//! Design: a [`StableDiGraph`] holds resolved symbol→symbol / file→file edges for
//! traversal (BFS blast-radius, shortest path, cycle detection, coupling). Parallel
//! typed `Vec`s (`call_graph`, `import_graph`, `type_hierarchy`) hold the rich edge
//! records (with call sites, import lists, provenance) for the human-facing queries
//! — mirroring the shape of the existing `SemanticIndex` so the VibeCody adapter is
//! a thin wrapper.

use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::algo::kosaraju_scc;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

use super::edge::{ApiContract, CallEdge, EdgeKind, ImportEdge, Provenance,
                  TypeRelation, TypeRelationType};
use super::hyperedge::Hyperedge;
use super::symbol::Symbol;

/// Stable petgraph node id.
pub type NodeId = NodeIndex;

/// Payload stored at each graph node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeData {
    /// A rich code symbol.
    Symbol(Symbol),
    /// A module / namespace node (coarse granularity).
    Module {
        /// Fully-qualified module or namespace name.
        name: String,
        /// Path to the file that declares the module.
        file_path: String,
    },
    /// A file node (coarse granularity).
    File {
        /// Path to the source file.
        path: String,
    },
}

impl NodeData {
    /// Best-effort display label.
    pub fn label(&self) -> String {
        match self {
            Self::Symbol(s) => s.name.clone(),
            Self::Module { name, .. } => name.clone(),
            Self::File { path } => path.clone(),
        }
    }

    /// File path associated with the node, if any.
    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::Symbol(s) => Some(&s.file_path),
            Self::Module { file_path, .. } => Some(file_path),
            Self::File { path } => Some(path),
        }
    }

    /// The symbol payload, if this is a symbol node.
    pub fn as_symbol(&self) -> Option<&Symbol> {
        match self {
            Self::Symbol(s) => Some(s),
            _ => None,
        }
    }
}

/// Payload stored at each graph edge (the traversal backbone). The rich per-kind data
/// lives in the parallel typed `Vec`s on `CodeGraph`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeData {
    /// Relationship kind.
    pub kind: EdgeKind,
    /// Provenance.
    pub provenance: Provenance,
}

/// The code knowledge graph.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    /// Traversal backbone.
    g: StableDiGraph<NodeData, EdgeData>,
    /// `Symbol::node_key()` (or coarse id) → node id.
    by_key: HashMap<String, NodeId>,
    /// Unqualified name → node ids (fuzzy / ambiguous lookup).
    by_name: HashMap<String, Vec<NodeId>>,
    /// Rich call-graph edges (resolved + unresolved callee names).
    pub call_graph: Vec<CallEdge>,
    /// Rich import-graph edges.
    pub import_graph: Vec<ImportEdge>,
    /// Rich type-hierarchy edges.
    pub type_hierarchy: Vec<TypeRelation>,
    /// API contracts keyed by qualified name.
    pub api_contracts: HashMap<String, ApiContract>,
    /// Hyperedges (3+ node groups).
    pub hyperedges: Vec<Hyperedge>,
    /// Petgraph edge ids mirrored per kind, for dedup.
    edge_ids: Vec<(EdgeIndex, EdgeKind)>,
}

impl CodeGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    // ── node insertion ────────────────────────────────────────────────────────

    /// Insert a symbol node (idempotent on `node_key`).
    pub fn add_symbol(&mut self, sym: Symbol) -> NodeId {
        let key = sym.node_key();
        if let Some(id) = self.by_key.get(&key) {
            self.g[*id] = NodeData::Symbol(sym.clone());
            return *id;
        }
        let id = self.g.add_node(NodeData::Symbol(sym.clone()));
        self.by_key.insert(key, id);
        self.by_name.entry(sym.name.clone()).or_default().push(id);
        id
    }

    /// Insert a module node (idempotent on `name`).
    pub fn add_module(&mut self, name: impl Into<String>, file_path: impl Into<String>) -> NodeId {
        let name = name.into();
        let file_path = file_path.into();
        let key = format!("module:{name}");
        if let Some(id) = self.by_key.get(&key) {
            return *id;
        }
        let id = self.g.add_node(NodeData::Module { name: name.clone(), file_path });
        self.by_key.insert(key, id);
        self.by_name.entry(name).or_default().push(id);
        id
    }

    /// Insert a file node (idempotent on `path`).
    pub fn add_file(&mut self, path: impl Into<String>) -> NodeId {
        let path = path.into();
        let key = format!("file:{path}");
        if let Some(id) = self.by_key.get(&key) {
            return *id;
        }
        let id = self.g.add_node(NodeData::File { path: path.clone() });
        self.by_key.insert(key, id);
        self.by_name.entry(path).or_default().push(id);
        id
    }

    /// Add a traversal edge between two nodes (idempotent on endpoints+kind).
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        kind: EdgeKind,
        provenance: Provenance,
    ) -> EdgeIndex {
        for (eidx, k) in &self.edge_ids {
            if *k == kind && self.g.edge_endpoints(*eidx) == Some((from, to)) {
                return *eidx;
            }
        }
        let eidx = self.g.add_edge(from, to, EdgeData { kind, provenance });
        self.edge_ids.push((eidx, kind));
        eidx
    }

    // ── rich typed edge insertion ─────────────────────────────────────────────

    /// Record a call edge. Resolved endpoints (when both caller+callee are symbol
    /// nodes) are mirrored into the traversal backbone.
    pub fn add_call(&mut self, edge: CallEdge) {
        // Tree-sitter produces unqualified callee text; LSP produces qualified. Try both.
        let from = self
            .find_by_qualified(&edge.caller)
            .or_else(|| self.find_by_name(&edge.caller));
        let to = self
            .find_by_qualified(&edge.callee)
            .or_else(|| self.find_by_name(&edge.callee));
        if let (Some(f), Some(t)) = (from, to) {
            self.add_edge(f, t, EdgeKind::Calls, edge.provenance);
        }
        self.call_graph.push(edge);
    }

    /// Record an import edge (file → file when both are known).
    pub fn add_import(&mut self, edge: ImportEdge) {
        let from = self.by_key.get(&format!("file:{}", edge.source_file)).copied();
        let to_key = format!("file:{}", edge.target);
        if let (Some(f), Some(t)) = (from, self.by_key.get(&to_key).copied()) {
            self.add_edge(f, t, EdgeKind::Imports, edge.provenance);
        }
        self.import_graph.push(edge);
    }

    /// Record a type-hierarchy edge.
    pub fn add_type_relation(&mut self, rel: TypeRelation) {
        let from = self.find_by_qualified(&rel.child);
        let to = self.find_by_qualified(&rel.parent);
        if let (Some(f), Some(t)) = (from, to) {
            let kind = match rel.relation {
                TypeRelationType::Implements | TypeRelationType::TraitImpl => EdgeKind::Implements,
                TypeRelationType::Inherits | TypeRelationType::Extends | TypeRelationType::Mixin => {
                    EdgeKind::Extends
                }
            };
            self.add_edge(f, t, kind, rel.provenance);
        }
        self.type_hierarchy.push(rel);
    }

    /// Record an API contract for a qualified name.
    pub fn add_api_contract(&mut self, qualified_name: impl Into<String>, contract: ApiContract) {
        self.api_contracts.insert(qualified_name.into(), contract);
    }

    /// Add a `Contains` backbone edge (parent → child) for module/file nesting.
    pub fn add_contains(&mut self, parent: NodeId, child: NodeId, provenance: Provenance) {
        self.add_edge(parent, child, EdgeKind::Contains, provenance);
    }

    /// Add a `DependsOn` backbone edge (file/module granularity).
    pub fn add_depends_on(&mut self, from: NodeId, to: NodeId, provenance: Provenance) {
        self.add_edge(from, to, EdgeKind::DependsOn, provenance);
    }

    /// Add a hyperedge group.
    pub fn add_hyperedge(&mut self, h: Hyperedge) {
        self.hyperedges.push(h);
    }

    // ── lookups ───────────────────────────────────────────────────────────────

    /// Find a symbol node by qualified name (exact).
    pub fn find_by_qualified(&self, qualified: &str) -> Option<NodeId> {
        let tail = qualified.rsplit("::").next().unwrap_or(qualified);
        if let Some(cands) = self.by_name.get(tail) {
            for id in cands {
                if let Some(NodeData::Symbol(s)) = self.g.node_weight(*id) {
                    if s.qualified_name == qualified {
                        return Some(*id);
                    }
                }
            }
        }
        for id in self.g.node_indices() {
            if let Some(NodeData::Symbol(s)) = self.g.node_weight(id) {
                if s.qualified_name == qualified {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Find the first symbol node whose unqualified name matches.
    pub fn find_by_name(&self, name: &str) -> Option<NodeId> {
        self.by_name.get(name).and_then(|v| v.first().copied())
    }

    /// Find a node by its key (`Symbol::node_key`, `module:…`, `file:…`).
    pub fn find_by_key(&self, key: &str) -> Option<NodeId> {
        self.by_key.get(key).copied()
    }

    /// The payload at a node.
    pub fn node(&self, id: NodeId) -> Option<&NodeData> {
        self.g.node_weight(id)
    }

    /// Iterate all symbol nodes.
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.g.node_weights().filter_map(|n| n.as_symbol())
    }

    /// Total node count.
    pub fn node_count(&self) -> usize {
        self.g.node_count()
    }

    /// Total backbone edge count.
    pub fn edge_count(&self) -> usize {
        self.g.edge_count()
    }

    // ── call-graph queries ────────────────────────────────────────────────────

    /// All callers of `name` (records where callee matches, resolved or not).
    pub fn callers(&self, name: &str) -> Vec<&CallEdge> {
        self.call_graph
            .iter()
            .filter(|e| e.callee == name || e.callee.ends_with(&format!("::{name}")))
            .collect()
    }

    /// All callees of `name` (records where caller matches, resolved or not).
    pub fn callees(&self, name: &str) -> Vec<&CallEdge> {
        self.call_graph
            .iter()
            .filter(|e| e.caller == name || e.caller.ends_with(&format!("::{name}")))
            .collect()
    }

    /// All types implementing the given trait / interface qualified name.
    pub fn implementations(&self, trait_qualified: &str) -> Vec<String> {
        self.type_hierarchy
            .iter()
            .filter(|r| r.parent == trait_qualified)
            .map(|r| r.child.clone())
            .collect()
    }

    /// All files/modules that import symbols from `target`.
    pub fn dependents(&self, target: &str) -> Vec<&ImportEdge> {
        self.import_graph.iter().filter(|e| e.target == target).collect()
    }

    /// All targets imported by `source_file`.
    pub fn dependencies(&self, source_file: &str) -> Vec<&ImportEdge> {
        self.import_graph.iter().filter(|e| e.source_file == source_file).collect()
    }

    // ── graph analysis (ported from dep_visualizer.rs) ────────────────────────

    /// Transitive dependencies (forward BFS) from a node, returning visited node ids
    /// in BFS order (start node first).
    pub fn transitive_deps(&self, start: NodeId) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut q = VecDeque::new();
        let mut out = Vec::new();
        q.push_back(start);
        visited.insert(start);
        while let Some(cur) = q.pop_front() {
            out.push(cur);
            for er in self.g.edges_directed(cur, petgraph::Direction::Outgoing) {
                let t = er.target();
                if visited.insert(t) {
                    q.push_back(t);
                }
            }
        }
        out
    }

    /// Detect cycles via Kosaraju strongly-connected components. Returns one node-id
    /// vec per SCC that contains a cycle (size > 1, or a self-loop).
    pub fn detect_cycles(&self) -> Vec<Vec<NodeId>> {
        kosaraju_scc(&self.g)
            .into_iter()
            .filter(|scc| {
                if scc.len() > 1 {
                    return true;
                }
                // Self-loop check.
                let n = scc[0];
                self.g
                    .edges_directed(n, petgraph::Direction::Outgoing)
                    .any(|er| er.target() == n)
            })
            .collect()
    }

    /// Coupling for a node: `(fan_in, fan_out)` — incoming + outgoing edge counts.
    pub fn coupling(&self, id: NodeId) -> (usize, usize) {
        let fan_in = self.g.edges_directed(id, petgraph::Direction::Incoming).count();
        let fan_out = self.g.edges_directed(id, petgraph::Direction::Outgoing).count();
        (fan_in, fan_out)
    }

    /// The `n` most-coupled nodes by `fan_in + fan_out` (the "god nodes").
    pub fn top_coupled(&self, n: usize) -> Vec<(NodeId, usize)> {
        let mut scored: Vec<(NodeId, usize)> = self
            .g
            .node_indices()
            .map(|id| {
                let (i, o) = self.coupling(id);
                (id, i + o)
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(n);
        scored
    }

    // ── incremental cleanup ───────────────────────────────────────────────────

    /// Remove all symbol nodes + typed edges belonging to `file_path`. Used by the
    /// incremental builder before re-inserting an updated file.
    pub fn remove_file(&mut self, file_path: &str) {
        self.call_graph.retain(|e| e.file != file_path);
        self.import_graph.retain(|e| e.source_file != file_path);

        let to_remove: Vec<NodeId> = self
            .g
            .node_indices()
            .filter(|id| {
                self.g.node_weight(*id).map_or(false, |n| n.file_path() == Some(file_path))
            })
            .collect();
        for id in to_remove {
            if let Some(NodeData::Symbol(s)) = self.g.node_weight(id).cloned() {
                self.by_key.remove(&s.node_key());
                if let Some(v) = self.by_name.get_mut(&s.name) {
                    v.retain(|x| *x != id);
                    if v.is_empty() {
                        self.by_name.remove(&s.name);
                    }
                }
            }
            self.g.remove_node(id);
        }

        // Re-validate type relations against surviving symbol nodes.
        let surviving: HashSet<String> =
            self.symbols().map(|s| s.qualified_name.clone()).collect();
        self.type_hierarchy
            .retain(|r| surviving.contains(&r.child) || surviving.contains(&r.parent));
    }

    /// Total number of recorded call edges (rich view).
    pub fn call_edge_count(&self) -> usize {
        self.call_graph.len()
    }

    /// Read-only access to the petgraph backbone (for advanced traversal).
    pub fn backbone(&self) -> &StableDiGraph<NodeData, EdgeData> {
        &self.g
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::{CallType, EdgeSource, ProvenanceTag};
    use crate::model::symbol::{Language, SymbolKind, Visibility};

    fn sym(name: &str, file: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind: SymbolKind::Function,
            qualified_name: format!("pkg::{name}"),
            file_path: file.into(),
            line_start: 1,
            line_end: 10,
            signature: None,
            doc_comment: None,
            visibility: Visibility::Public,
            language: Language::Rust,
        }
    }

    fn call(caller: &str, callee: &str, file: &str) -> CallEdge {
        CallEdge {
            caller: format!("pkg::{caller}"),
            callee: format!("pkg::{callee}"),
            file: file.into(),
            line: 5,
            call_type: CallType::Direct,
            provenance: Provenance::from_source(EdgeSource::TreeSitter),
        }
    }

    #[test]
    fn add_symbol_is_idempotent() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("foo", "a.rs"));
        let a2 = g.add_symbol(sym("foo", "a.rs"));
        assert_eq!(a, a2);
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn call_edge_mirrors_into_backbone_when_resolved() {
        let mut g = CodeGraph::new();
        g.add_symbol(sym("caller", "a.rs"));
        g.add_symbol(sym("callee", "b.rs"));
        g.add_call(call("caller", "callee", "a.rs"));
        assert_eq!(g.callers("callee").len(), 1);
        assert_eq!(g.callees("caller").len(), 1);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn detect_cycles_finds_simple_loop() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a", "a.rs"));
        let b = g.add_symbol(sym("b", "b.rs"));
        let c = g.add_symbol(sym("c", "c.rs"));
        g.add_edge(a, b, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        g.add_edge(b, c, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        g.add_edge(c, a, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        let cycles = g.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn coupling_and_top_coupled_work() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a", "a.rs"));
        let b = g.add_symbol(sym("b", "b.rs"));
        g.add_edge(a, b, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        let (in_b, out_b) = g.coupling(b);
        assert_eq!((in_b, out_b), (1, 0));
        let (in_a, out_a) = g.coupling(a);
        assert_eq!((in_a, out_a), (0, 1));
        let top = g.top_coupled(2);
        assert_eq!(top.len(), 2);
        // both have total coupling 1; ordering among ties is stable but not asserted
        assert_eq!(top[0].1, 1);
    }

    #[test]
    fn remove_file_drops_symbols_and_edges() {
        let mut g = CodeGraph::new();
        g.add_symbol(sym("foo", "a.rs"));
        g.add_symbol(sym("bar", "b.rs"));
        g.add_call(call("foo", "bar", "a.rs"));
        g.remove_file("a.rs");
        assert!(g.find_by_name("foo").is_none());
        assert!(g.call_graph.is_empty());
    }

    #[test]
    fn provenance_defaults_match_source() {
        assert_eq!(Provenance::from_source(EdgeSource::Scip).confidence, 1.0);
        assert_eq!(Provenance::from_source(EdgeSource::Lsp).confidence, 0.95);
        assert_eq!(Provenance::from_source(EdgeSource::TreeSitter).confidence, 0.7);
        assert_eq!(Provenance::from_source(EdgeSource::Lsp).tag, ProvenanceTag::Extracted);
        assert_eq!(Provenance::from_source(EdgeSource::TreeSitter).tag, ProvenanceTag::Inferred);
    }

    #[test]
    fn api_contract_roundtrip() {
        let c = ApiContract {
            params: vec!["x: i32".into()],
            return_type: Some("i32".into()),
            error_types: vec![],
            is_async: false,
        };
        let mut g = CodeGraph::new();
        g.add_api_contract("pkg::foo", c.clone());
        assert_eq!(g.api_contracts.get("pkg::foo"), Some(&c));
    }

    #[test]
    fn transitive_deps_bfs_visits_all_reachable() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a", "a.rs"));
        let b = g.add_symbol(sym("b", "b.rs"));
        let c = g.add_symbol(sym("c", "c.rs"));
        g.add_edge(a, b, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        g.add_edge(b, c, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        let deps = g.transitive_deps(a);
        assert!(deps.contains(&a));
        assert!(deps.contains(&b));
        assert!(deps.contains(&c));
    }
}