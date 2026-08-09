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
                // Tie-break on the label id, not on hash order. `max_by_key`
                // returns the *last* maximum it sees, and `HashMap` iteration
                // order is seeded randomly per process — so with two labels
                // tied, which one won varied run to run and propagation could
                // settle differently on the same graph. That made
                // `detects_a_community_from_a_cluster` fail roughly one run in
                // ten, and any caller's community ids unstable between
                // processes. Ordering by `(count, label)` makes the outcome a
                // function of the graph alone.
                let best = tally
                    .into_iter()
                    .max_by_key(|(l, c)| (*c, *l))
                    .map(|(l, _)| l);
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
            out.push(Community {
                id: seed,
                label,
                members,
            });
        }
        // Largest communities first, then by id so the order is total. The
        // groups come out of a `HashMap`, and `sort_by` is stable — so without
        // the id tiebreak, equal-sized communities kept their hash order and
        // the returned sequence differed between processes on identical input.
        out.sort_by(|a, b| {
            b.members
                .len()
                .cmp(&a.members.len())
                .then_with(|| a.id.cmp(&b.id))
        });
        // Members likewise, so a community's contents render identically run
        // to run.
        for c in &mut out {
            c.members.sort();
        }
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

    /// The a/b/c triangle must land in one community, and the result must be a
    /// function of the graph alone.
    ///
    /// The weaker assertion above ("some community of size >= 2") passed most
    /// of the time even while label propagation was seed-dependent: ties were
    /// resolved by `HashMap` iteration order, which Rust seeds randomly per
    /// process, so this graph settled differently roughly one run in ten and
    /// the workspace suite failed intermittently. With ties broken on the label
    /// id, the exact grouping is pinnable — so pin it.
    #[test]
    fn community_detection_is_a_function_of_the_graph() {
        let build = || {
            let mut g = CodeGraph::new();
            let a = g.add_symbol(sym("a", "a.rs"));
            let b = g.add_symbol(sym("b", "a.rs"));
            let c = g.add_symbol(sym("c", "a.rs"));
            let x = g.add_symbol(sym("x", "b.rs"));
            let p = Provenance::from_source(EdgeSource::TreeSitter);
            g.add_edge(a, b, EdgeKind::Calls, p);
            g.add_edge(b, c, EdgeKind::Calls, p);
            g.add_edge(c, a, EdgeKind::Calls, p);
            g.add_edge(a, x, EdgeKind::Calls, p);
            g
        };

        let first = detect_communities(&build());
        let biggest = first
            .iter()
            .max_by_key(|c| c.members.len())
            .expect("the triangle must form a community");
        assert!(
            biggest.members.len() >= 3,
            "a/b/c are mutually connected and must group together, got {:?}",
            first.iter().map(|c| c.members.len()).collect::<Vec<_>>(),
        );

        // Identical input, freshly built: identical output, including order.
        for _ in 0..25 {
            let again = detect_communities(&build());
            assert_eq!(
                again.len(),
                first.len(),
                "community count must not vary for the same graph",
            );
            let a: Vec<_> = again.iter().map(|c| (c.id, c.members.clone())).collect();
            let b: Vec<_> = first.iter().map(|c| (c.id, c.members.clone())).collect();
            assert_eq!(a, b, "ids, members and ordering must all be reproducible");
        }
    }
}
