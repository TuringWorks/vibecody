//! `GRAPH_REPORT.md` generator — god nodes, communities, surprising edges, and
//! suggested starter questions. Mirrors Graphify's report format.

use crate::analyze::{communities::detect_communities, god_nodes, surprising_edges};
use crate::model::graph::CodeGraph;

/// Render a `GRAPH_REPORT.md`-style string for `graph`.
pub fn render_report(graph: &CodeGraph) -> String {
    let mut s = String::new();
    s.push_str("# GRAPH_REPORT\n\n");
    s.push_str(&format!(
        "## Overview\n\n- Nodes: {}\n- Backbone edges: {}\n- Call edges: {}\n- Import edges: {}\n- Type relations: {}\n\n",
        graph.node_count(),
        graph.edge_count(),
        graph.call_edge_count(),
        graph.import_graph.len(),
        graph.type_hierarchy.len(),
    ));

    let gods = god_nodes(graph, 10);
    s.push_str("## God nodes (highest-degree keystones)\n\n");
    if gods.is_empty() {
        s.push_str("_No high-degree symbols detected._\n\n");
    } else {
        s.push_str("| Symbol | File | Coupling (in+out) |\n|---|---|---|\n");
        for g in &gods {
            s.push_str(&format!("| {} | {} | {} |\n", g.name, g.file, g.coupling));
        }
        s.push('\n');
    }

    let comms = detect_communities(graph);
    s.push_str("## Community structure\n\n");
    if comms.is_empty() {
        s.push_str("_No multi-node communities detected (graph may be sparse).\n\n");
    } else {
        for (i, c) in comms.iter().take(15).enumerate() {
            let members: Vec<String> = c
                .members
                .iter()
                .filter_map(|id| graph.node(*id).map(|n| n.label()))
                .take(8)
                .collect();
            s.push_str(&format!(
                "{}. **{}** ({} nodes): {}\n",
                i + 1,
                c.label,
                c.members.len(),
                members.join(", ")
            ));
        }
        s.push('\n');
    }

    let surprising = surprising_edges(graph);
    s.push_str("## Surprising cross-file connections\n\n");
    if surprising.is_empty() {
        s.push_str("_None detected._\n\n");
    } else {
        s.push_str("| From | To | Kind | From file | To file |\n|---|---|---|---|---|\n");
        for e in surprising.iter().take(20) {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                e.from, e.to, e.kind.as_str(), e.from_file, e.to_file
            ));
        }
        s.push('\n');
    }

    s.push_str("## Suggested questions\n\n");
    if let Some(first) = gods.first() {
        s.push_str(&format!(
            "- `kodegraph query \"{}\"` — what does the top god node touch?\n",
            first.name
        ));
    }
    if let Some(c) = comms.first() {
        s.push_str(&format!(
            "- `kodegraph blast-radius \"{}\"` — change impact for the largest community's seed\n",
            c.label
        ));
    }
    s.push_str("- `kodegraph viz --mermaid` — render the full dependency graph\n");

    s
}