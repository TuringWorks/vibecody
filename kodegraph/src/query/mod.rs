//! Query API — Graphify-compatible tool names for graph traversal.
//!
//! These mirror the MCP tool names adopted from Graphify (`query_graph`, `get_node`,
//! `get_neighbors`, `shortest_path`) plus VibeCody-specific `blast_radius` and
//! `communities`/`god_nodes`. An MCP server or CLI wraps these; an embedder calls
//! them directly.

pub mod traversal;

use crate::analyze::{blast_radius as blast_radius_fn, BlastRadius};
use crate::analyze::{detect_communities, god_nodes as god_nodes_fn, surprising_edges,
                     Community, GodNode, SurprisingEdge};
use crate::model::edge::{EdgeKind, Provenance};
use crate::model::graph::{CodeGraph, NodeData, NodeId};
use petgraph::visit::EdgeRef;
use std::collections::HashSet;

/// A compact, token-budgeted subgraph result for `query_graph`.
#[derive(Debug, Clone)]
pub struct Subgraph {
    /// Matched seed nodes.
    pub seeds: Vec<NodeId>,
    /// Nodes included in the subgraph.
    pub nodes: Vec<NodeData>,
    /// Edges (from, to, kind, provenance) included.
    pub edges: Vec<(NodeId, NodeId, EdgeKind, Provenance)>,
    /// Approximate token cost of serializing this subgraph (rough: 4 chars/token).
    pub est_tokens: usize,
}

/// `query_graph` — pull a focused subgraph matching `query` within a token `budget`.
///
/// Matching: symbols whose name (case-insensitive) contains any query term are seeds;
/// the subgraph expands 1 hop out + 1 hop in until the token budget is hit.
pub fn query_graph(graph: &CodeGraph, query: &str, budget: usize) -> Subgraph {
    let terms: Vec<String> = query.split_whitespace().map(|s| s.to_ascii_lowercase()).collect();
    let mut seeds: Vec<NodeId> = Vec::new();
    for id in graph.backbone().node_indices() {
        if let Some(NodeData::Symbol(s)) = graph.node(id) {
            let name_lc = s.name.to_ascii_lowercase();
            if terms.iter().any(|t| name_lc.contains(t.as_str())) {
                seeds.push(id);
            }
        }
    }
    seeds.dedup();

    let mut included: HashSet<NodeId> = HashSet::new();
    let mut nodes: Vec<NodeData> = Vec::new();
    let mut edges: Vec<(NodeId, NodeId, EdgeKind, Provenance)> = Vec::new();
    let mut est_tokens = 0usize;

    let push_node = |nodes: &mut Vec<NodeData>, est: &mut usize, id: NodeId, graph: &CodeGraph| {
        if let Some(nd) = graph.node(id) {
            *est += label_tokens(&nd.label());
            nodes.push(nd.clone());
        }
    };

    for &seed in &seeds {
        if included.insert(seed) {
            push_node(&mut nodes, &mut est_tokens, seed, graph);
        }
        if est_tokens > budget {
            break;
        }
        // 1 hop out + 1 hop in.
        let bb = graph.backbone();
        for er in bb.edges_directed(seed, petgraph::Direction::Outgoing) {
            let t = er.target();
            let ed = er.weight();
            edges.push((seed, t, ed.kind, ed.provenance));
            est_tokens += 4;
            if included.insert(t) {
                push_node(&mut nodes, &mut est_tokens, t, graph);
            }
            if est_tokens > budget {
                break;
            }
        }
        for er in bb.edges_directed(seed, petgraph::Direction::Incoming) {
            let s = er.source();
            let ed = er.weight();
            edges.push((s, seed, ed.kind, ed.provenance));
            est_tokens += 4;
            if included.insert(s) {
                push_node(&mut nodes, &mut est_tokens, s, graph);
            }
            if est_tokens > budget {
                break;
            }
        }
    }

    Subgraph { seeds, nodes, edges, est_tokens }
}

/// `get_node` — retrieve a single node's payload by name (qualified or unqualified).
pub fn get_node(graph: &CodeGraph, name: &str) -> Option<NodeData> {
    let id = graph.find_by_qualified(name).or_else(|| graph.find_by_name(name))?;
    graph.node(id).cloned()
}

/// `get_neighbors` — adjacent nodes (outgoing + incoming) of `name`.
pub fn get_neighbors(graph: &CodeGraph, name: &str) -> Vec<NodeData> {
    let Some(id) = graph.find_by_qualified(name).or_else(|| graph.find_by_name(name)) else {
        return Vec::new();
    };
    let bb = graph.backbone();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for er in bb.edges_directed(id, petgraph::Direction::Outgoing) {
        let t = er.target();
        if seen.insert(t) {
            if let Some(nd) = graph.node(t) {
                out.push(nd.clone());
            }
        }
    }
    for er in bb.edges_directed(id, petgraph::Direction::Incoming) {
        let s = er.source();
        if seen.insert(s) {
            if let Some(nd) = graph.node(s) {
                out.push(nd.clone());
            }
        }
    }
    out
}

/// `shortest_path` — BFS shortest path between two named nodes (edge count + node path).
pub fn shortest_path(graph: &CodeGraph, from: &str, to: &str) -> Option<(usize, Vec<NodeData>)> {
    use crate::query::traversal::shortest_path_bfs;
    let a = graph.find_by_qualified(from).or_else(|| graph.find_by_name(from))?;
    let b = graph.find_by_qualified(to).or_else(|| graph.find_by_name(to))?;
    let path = shortest_path_bfs(graph, a, b)?;
    let nodes: Vec<NodeData> = path.into_iter().filter_map(|id| graph.node(id).cloned()).collect();
    let hops = nodes.len().saturating_sub(1);
    Some((hops, nodes))
}

/// `blast_radius` — reachable nodes within `max_hops` of `name` (both directions).
pub fn blast_radius(graph: &CodeGraph, name: &str, max_hops: usize) -> BlastRadius {
    blast_radius_fn(graph, name, max_hops)
}

/// `communities` — detected communities (Leiden-style label propagation).
pub fn communities(graph: &CodeGraph) -> Vec<Community> {
    detect_communities(graph)
}

/// `god_nodes` — the `n` highest-coupling keystones.
pub fn god_nodes(graph: &CodeGraph, n: usize) -> Vec<GodNode> {
    god_nodes_fn(graph, n)
}

/// `surprising_edges` — cross-file edges between token-disjoint endpoint names.
pub fn surprising_edges_query(graph: &CodeGraph) -> Vec<SurprisingEdge> {
    surprising_edges(graph)
}

fn label_tokens(label: &str) -> usize {
    // Rough: 4 chars per token, min 1.
    (label.len() / 4).max(1)
}