//! Build a kodegraph graph on a directory and print a summary + a sample query.
//!
//! Run: `cargo run --example build_graph -- path/to/src`

use kodegraph::builder::CodeGraphBuilder;
use kodegraph::query;

fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    let (graph, _hashes) = CodeGraphBuilder::new().scan_dir(&dir)?.build()?;

    println!(
        "Built {} nodes, {} backbone edges, {} call edges, {} imports.",
        graph.node_count(),
        graph.edge_count(),
        graph.call_edge_count(),
        graph.import_graph.len(),
    );

    // Sample: top god nodes.
    for g in kodegraph::analyze::god_nodes(&graph, 5) {
        println!("  god node: {} (coupling {})", g.name, g.coupling);
    }

    // Sample query: anything matching "build".
    let sub = query::query_graph(&graph, "build", 500);
    println!("query 'build' -> {} nodes, ~{} tokens", sub.nodes.len(), sub.est_tokens);

    Ok(())
}