//! God nodes — highest-degree keystones — and "surprising" cross-file edges.
//!
//! God nodes (Graphify terminology) are the symbols everything else connects through;
//! they're the natural entry points for a compact `GRAPH_REPORT.md` and for
//! "where do I start reading this codebase" prompts. Surprising edges are
//! cross-file relationships between symbols that share no obvious structural link.

use crate::model::edge::EdgeKind;
use crate::model::graph::{CodeGraph, NodeId};

/// A god node: a high-degree symbol + its total coupling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GodNode {
    /// Node id.
    pub id: NodeId,
    /// Symbol name (or node label).
    pub name: String,
    /// File path.
    pub file: String,
    /// `fan_in + fan_out`.
    pub coupling: usize,
}

/// An edge between two symbols in *different files* whose endpoints share no
/// obvious naming/structural link — a candidate "surprising connection".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurprisingEdge {
    /// Source symbol name.
    pub from: String,
    /// Target symbol name.
    pub to: String,
    /// Edge kind.
    pub kind: EdgeKind,
    /// Source file.
    pub from_file: String,
    /// Target file.
    pub to_file: String,
}

/// Return the `n` god nodes (highest coupling) in the graph.
pub fn god_nodes(graph: &CodeGraph, n: usize) -> Vec<GodNode> {
    graph
        .top_coupled(n)
        .into_iter()
        .filter_map(|(id, coupling)| {
            let node = graph.node(id)?;
            let name = node.label();
            let file = node.file_path().unwrap_or("").to_string();
            Some(GodNode { id, name, file, coupling })
        })
        .collect()
}

/// Return cross-file edges whose endpoint names share no token overlap (a rough
/// "surprising connection" heuristic, mirroring Graphify's cross-file scoring).
pub fn surprising_edges(graph: &CodeGraph) -> Vec<SurprisingEdge> {
    let backbone = graph.backbone();
    let mut out = Vec::new();
    for e in backbone.edge_indices() {
        let Some((from, to)) = backbone.edge_endpoints(e) else { continue };
        let Some(ed) = backbone.edge_weight(e) else { continue };
        let (Some(from_node), Some(to_node)) = (backbone.node_weight(from), backbone.node_weight(to))
        else {
            continue;
        };
        let from_file = from_node.file_path().unwrap_or("").to_string();
        let to_file = to_node.file_path().unwrap_or("").to_string();
        if from_file.is_empty() || to_file.is_empty() || from_file == to_file {
            continue;
        }
        let from_name = from_node.label();
        let to_name = to_node.label();
        if !shares_token(&from_name, &to_name) {
            out.push(SurprisingEdge {
                from: from_name,
                to: to_name,
                kind: ed.kind,
                from_file,
                to_file,
            });
        }
    }
    out
}

fn shares_token(a: &str, b: &str) -> bool {
    let at = tokens(a);
    let bt = tokens(b);
    at.iter().any(|t| bt.contains(t))
}

fn tokens(s: &str) -> Vec<String> {
    s.split(['_', ':', '.', '-', '/'])
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::Provenance;
    use crate::model::edge::EdgeSource;
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
    fn god_nodes_rank_by_coupling() {
        let mut g = CodeGraph::new();
        let hub = g.add_symbol(sym("hub", "h.rs"));
        let a = g.add_symbol(sym("spoke_a", "a.rs"));
        let b = g.add_symbol(sym("spoke_b", "b.rs"));
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(hub, a, EdgeKind::Calls, p);
        g.add_edge(hub, b, EdgeKind::Calls, p);
        let gods = god_nodes(&g, 1);
        assert_eq!(gods.len(), 1);
        assert_eq!(gods[0].name, "hub");
        assert_eq!(gods[0].coupling, 2);
    }

    #[test]
    fn surprising_edges_are_cross_file_and_token_disjoint() {
        let mut g = CodeGraph::new();
        let auth = g.add_symbol(sym("authenticate", "auth.rs"));
        let render = g.add_symbol(sym("render_tree", "ui.rs"));
        let p = Provenance::from_source(EdgeSource::TreeSitter);
        g.add_edge(auth, render, EdgeKind::Calls, p);
        let surprising = surprising_edges(&g);
        assert!(surprising.iter().any(|s| s.from == "authenticate" && s.to == "render_tree"));
    }
}