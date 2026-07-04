//! Blast radius — the set of symbols/files reachable within `N` hops of a change.
//!
//! This is the **token-reduction primitive**. Instead of dumping every file that
//! mentions a symbol into an LLM context, a consumer computes the blast radius of
//! the changed symbol and injects only those signatures (Level-2 skeleton) — a few
//! hundred tokens instead of tens of thousands.

use std::collections::{HashMap, HashSet};

use petgraph::visit::EdgeRef;

use crate::model::graph::{CodeGraph, NodeId};

/// A blast-radius result: reachable nodes grouped by hop distance from the seed.
#[derive(Debug, Clone)]
pub struct BlastRadius {
    /// Seed node id (if resolved).
    pub seed: Option<NodeId>,
    /// Seed display name.
    pub seed_name: String,
    /// `hop -> node ids` (hop 0 = the seed itself).
    pub by_hop: HashMap<usize, Vec<NodeId>>,
    /// All reachable node ids (union of `by_hop`).
    pub all: HashSet<NodeId>,
    /// Total distinct nodes reachable (excluding seed).
    pub affected_count: usize,
}

impl BlastRadius {
    /// Total affected nodes (excluding the seed).
    pub fn affected(&self) -> usize {
        self.affected_count
    }

    /// Nodes at exactly `hop` distance (excluding the seed at hop 0).
    pub fn at_hop(&self, hop: usize) -> &[NodeId] {
        self.by_hop.get(&hop).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Compute the blast radius of `symbol_name` within `max_hops` hops, following
/// **both** incoming (callers) and outgoing (callees) edges — a change to a symbol
/// affects both who calls it and what it transitively touches.
///
/// Resolution: tries exact qualified name, then unqualified name.
pub fn blast_radius(graph: &CodeGraph, symbol_name: &str, max_hops: usize) -> BlastRadius {
    let seed = graph
        .find_by_qualified(symbol_name)
        .or_else(|| graph.find_by_name(symbol_name));
    let seed_name = symbol_name.to_string();
    let mut by_hop: HashMap<usize, Vec<NodeId>> = HashMap::new();
    let mut all: HashSet<NodeId> = HashSet::new();

    if let Some(seed) = seed {
        by_hop.insert(0, vec![seed]);
        all.insert(seed);
        let mut frontier: Vec<(NodeId, usize)> = vec![(seed, 0)];
        while let Some((cur, hop)) = frontier.pop() {
            if hop >= max_hops {
                continue;
            }
            let next_hop = hop + 1;
            let backbone = graph.backbone();
            // Outgoing.
            for er in backbone.edges_directed(cur, petgraph::Direction::Outgoing) {
                let t = er.target();
                if all.insert(t) {
                    by_hop.entry(next_hop).or_default().push(t);
                    frontier.push((t, next_hop));
                }
            }
            // Incoming.
            for er in backbone.edges_directed(cur, petgraph::Direction::Incoming) {
                let s = er.source();
                if all.insert(s) {
                    by_hop.entry(next_hop).or_default().push(s);
                    frontier.push((s, next_hop));
                }
            }
        }
    }

    let affected_count = all.len().saturating_sub(if seed.is_some() { 1 } else { 0 });
    BlastRadius { seed, seed_name, by_hop, all, affected_count }
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
    fn blast_radius_reaches_callers_and_callees() {
        let mut g = CodeGraph::new();
        let caller = g.add_symbol(sym("caller", "a.rs"));
        let target = g.add_symbol(sym("target", "b.rs"));
        let callee = g.add_symbol(sym("callee", "c.rs"));
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(caller, target, EdgeKind::Calls, p);
        g.add_edge(target, callee, EdgeKind::Calls, p);

        let br = blast_radius(&g, "target", 2);
        assert_eq!(br.seed, Some(target));
        assert_eq!(br.affected(), 2); // caller + callee
        assert!(br.all.contains(&caller));
        assert!(br.all.contains(&callee));
    }

    #[test]
    fn blast_radius_respects_hop_limit() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a", "a.rs"));
        let b = g.add_symbol(sym("b", "b.rs"));
        let c = g.add_symbol(sym("c", "c.rs"));
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(a, b, EdgeKind::Calls, p);
        g.add_edge(b, c, EdgeKind::Calls, p);

        let br = blast_radius(&g, "a", 1);
        assert_eq!(br.affected(), 1); // only b at hop 1
        assert!(!br.all.contains(&c));
    }

    #[test]
    fn blast_radius_unresolved_seed_is_empty() {
        let g = CodeGraph::new();
        let br = blast_radius(&g, "nope", 3);
        assert_eq!(br.affected(), 0);
        assert!(br.seed.is_none());
    }
}