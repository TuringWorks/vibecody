//! Graph renderers — Mermaid + DOT. Generalizes
//! `vibecli/.../dep_visualizer.rs::render_mermaid` / `render_dot` to the full
//! `CodeGraph` (all edge kinds, not just imports).

use crate::model::graph::CodeGraph;

/// Render the graph backbone as a Mermaid flowchart. Node ids are sanitized to
/// alphanumerics for Mermaid identifier safety.
pub fn render_mermaid(graph: &CodeGraph, title: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("---\ntitle: {title}\n---\n"));
    s.push_str("flowchart LR\n");
    let bb = graph.backbone();
    for id in bb.node_indices() {
        let Some(nd) = bb.node_weight(id) else { continue };
        let label = mermaid_safe(&nd.label());
        s.push_str(&format!("  n{}[\"{}\"]\n", id.index(), label));
    }
    for e in bb.edge_indices() {
        let Some((from, to)) = bb.edge_endpoints(e) else { continue };
        let Some(ed) = bb.edge_weight(e) else { continue };
        s.push_str(&format!(
            "  n{} -.{}-> n{}\n",
            from.index(),
            ed.kind.as_str(),
            to.index()
        ));
    }
    s
}

/// Render the graph backbone in Graphviz DOT format.
pub fn render_dot(graph: &CodeGraph, name: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("digraph {} {{\n", mermaid_safe(name)));
    s.push_str("  rankdir=LR;\n");
    s.push_str("  node [shape=box];\n");
    let bb = graph.backbone();
    for id in bb.node_indices() {
        let Some(nd) = bb.node_weight(id) else { continue };
        s.push_str(&format!(
            "  n{} [label=\"{}\"];\n",
            id.index(),
            dot_escape(&nd.label())
        ));
    }
    for e in bb.edge_indices() {
        let Some((from, to)) = bb.edge_endpoints(e) else { continue };
        let Some(ed) = bb.edge_weight(e) else { continue };
        s.push_str(&format!(
            "  n{} -> n{} [label=\"{}\"];\n",
            from.index(),
            to.index(),
            ed.kind.as_str()
        ));
    }
    s.push_str("}\n");
    s
}

fn mermaid_safe(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
    fn mermaid_includes_nodes_and_edges() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("alpha"));
        let b = g.add_symbol(sym("beta"));
        g.add_edge(a, b, EdgeKind::Calls, Provenance::from_source(EdgeSource::TreeSitter));
        let m = render_mermaid(&g, "test");
        assert!(m.contains("flowchart LR"));
        assert!(m.contains("alpha"));
        assert!(m.contains("calls"));
    }

    #[test]
    fn dot_is_valid_enough() {
        let mut g = CodeGraph::new();
        let a = g.add_symbol(sym("a"));
        let b = g.add_symbol(sym("b"));
        g.add_edge(a, b, EdgeKind::Imports, Provenance::from_source(EdgeSource::TreeSitter));
        let d = render_dot(&g, "g");
        assert!(d.starts_with("digraph g {"));
        assert!(d.contains("->"));
    }
}