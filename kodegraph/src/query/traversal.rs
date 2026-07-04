//! Traversal helpers over the petgraph backbone.

use std::collections::{HashMap, VecDeque};

use petgraph::visit::EdgeRef;

use crate::model::graph::{CodeGraph, NodeId};

/// BFS shortest path (treating edges as directed) from `start` to `target`.
/// Returns the node-id sequence (inclusive) or `None` if unreachable.
pub fn shortest_path_bfs(graph: &CodeGraph, start: NodeId, target: NodeId) -> Option<Vec<NodeId>> {
    if start == target {
        return Some(vec![start]);
    }
    let bb = graph.backbone();
    let mut prev: HashMap<NodeId, NodeId> = HashMap::new();
    let mut q: VecDeque<NodeId> = VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);
    q.push_back(start);
    while let Some(cur) = q.pop_front() {
        for er in bb.edges_directed(cur, petgraph::Direction::Outgoing) {
            let t = er.target();
            if visited.insert(t) {
                prev.insert(t, cur);
                if t == target {
                    // Reconstruct.
                    let mut path = vec![target];
                    let mut at = target;
                    while let Some(&p) = prev.get(&at) {
                        path.push(p);
                        at = p;
                    }
                    path.reverse();
                    return Some(path);
                }
                q.push_back(t);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::{EdgeKind, EdgeSource, Provenance};
    use crate::model::symbol::{Language, Symbol, SymbolKind, Visibility};

    fn sym(name: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind: SymbolKind::Function,
            qualified_name: format!("pkg::{name}"),
            file_path: format!("{name}.rs"),
            line_start: 1,
            line_end: 5,
            signature: None,
            doc_comment: None,
            visibility: Visibility::Public,
            language: Language::Rust,
        }
    }

    #[test]
    fn shortest_path_finds_direct_route() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a"));
        let b = g.add_symbol(sym("b"));
        let c = g.add_symbol(sym("c"));
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(a, b, EdgeKind::Calls, p);
        g.add_edge(b, c, EdgeKind::Calls, p);
        let path = shortest_path_bfs(&g, a, c).unwrap();
        assert_eq!(path, vec![a, b, c]);
    }

    #[test]
    fn shortest_path_none_when_unreachable() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a"));
        let b = g.add_symbol(sym("b"));
        assert!(shortest_path_bfs(&g, a, b).is_none());
    }
}