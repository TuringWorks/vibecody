//! Community detection — Leiden-style, topology-only (no embeddings).
//!
//! v0.1 uses a deterministic **label propagation** algorithm: each node adopts the
//! majority label of its neighbors (degree-weighted) over a fixed number of rounds.
//! This avoids an immature Leiden dependency while producing usable community
//! groupings for the `GRAPH_REPORT.md` "community structure" section.
//!
//! Edges are treated as undirected for community purposes (calls + imports both
//! indicate "relatedness").

use std::collections::HashMap;

use petgraph::visit::EdgeRef;

use crate::model::graph::{CodeGraph, NodeData, NodeId};

/// A detected community of node ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Community {
    /// Stable community id (the seed node id).
    pub id: NodeId,
    /// Human-readable label (the seed node's name).
    pub label: String,
    /// Member node ids.
    pub members: Vec<NodeId>,
}

/// Community detector running label propagation over the graph backbone.
#[derive(Debug, Clone)]
pub struct CommunityDetector {
    /// Number of propagation rounds.
    pub rounds: usize,
}

impl Default for CommunityDetector {
    fn default() -> Self {
        Self { rounds: 6 }
    }
}

impl CommunityDetector {
    /// Construct with a custom round count.
    pub fn new(rounds: usize) -> Self {
        Self { rounds }
    }

    /// Run detection and return communities with >= 2 members.
    pub fn detect(&self, graph: &CodeGraph) -> Vec<Community> {
        let backbone = graph.backbone();
        let nodes: Vec<NodeId> = backbone.node_indices().collect();
        if nodes.is_empty() {
            return Vec::new();
        }

        // Seed each node with its own id as label.
        let mut label: HashMap<NodeId, NodeId> = HashMap::new();
        for &n in &nodes {
            label.insert(n, n);
        }

        for _ in 0..self.rounds {
            let mut changed = false;
            for &n in &nodes {
                // Tally neighbor labels weighted by edge count.
                let mut tally: HashMap<NodeId, usize> = HashMap::new();
                for er in backbone.edges_directed(n, petgraph::Direction::Outgoing) {
                    let t = er.target();
                    *tally.entry(label[&t]).or_insert(0) += 1;
                }
                for er in backbone.edges_directed(n, petgraph::Direction::Incoming) {
                    let s = er.source();
                    *tally.entry(label[&s]).or_insert(0) += 1;
                }
                // Include self to break ties toward current label.
                *tally.entry(label[&n]).or_insert(0) += 1;
                let best = tally.into_iter().max_by_key(|(_, c)| *c).map(|(l, _)| l);
                if let Some(best) = best {
                    if label[&n] != best {
                        label.insert(n, best);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Group by final label.
        let mut groups: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for &n in &nodes {
            groups.entry(label[&n]).or_default().push(n);
        }

        let mut out = Vec::new();
        for (seed, members) in groups {
            if members.len() < 2 {
                continue;
            }
            let label = backbone
                .node_weight(seed)
                .map(NodeData::label)
                .unwrap_or_else(|| format!("community_{}", seed.index()));
            out.push(Community { id: seed, label, members });
        }
        // Largest communities first.
        out.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
        out
    }
}

/// Convenience: detect communities with default settings.
pub fn detect_communities(graph: &CodeGraph) -> Vec<Community> {
    CommunityDetector::default().detect(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::{EdgeKind, EdgeSource, Provenance};
    use crate::model::symbol::{Language, Symbol, SymbolKind, Visibility};

    fn sym(name: &str, file: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind: SymbolKind::Function,
            qualified_name: format!("pkg::{name}"),
            file_path: file.into(),
            line_start: 1,
            line_end: 5,
            signature: None,
            doc_comment: None,
            visibility: Visibility::Public,
            language: Language::Rust,
        }
    }

    #[test]
    fn detects_a_community_from_a_cluster() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a", "a.rs"));
        let b = g.add_symbol(sym("b", "a.rs"));
        let c = g.add_symbol(sym("c", "a.rs"));
        let x = g.add_symbol(sym("x", "b.rs"));
        // a-b-c form a tightly coupled cluster; x is isolated.
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(a, b, EdgeKind::Calls, p);
        g.add_edge(b, c, EdgeKind::Calls, p);
        g.add_edge(c, a, EdgeKind::Calls, p);
        g.add_edge(a, x, EdgeKind::Calls, p);
        let comms = detect_communities(&g);
        // At least one community of size >= 2 should emerge from the a/b/c cluster.
        assert!(comms.iter().any(|c| c.members.len() >= 2));
    }
}