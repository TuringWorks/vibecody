//! `kodegraph` CLI — `build`, `query`, `path`, `explain`, `viz`, `report`, `serve`.
//!
//! Behind the `cli` feature. The binary entry point (`src/bin/cli.rs`) is a thin
//! wrapper that calls [`run`].

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, Subcommand};

use crate::builder::CodeGraphBuilder;
use crate::mcp::McpServer;
use crate::query;
use crate::report::{render_dot, render_mermaid, render_report};
use crate::store::{SQLiteStore, Store};

/// Build and query code knowledge graphs.
#[derive(ClapParser, Debug)]
#[command(name = "kodegraph", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,

    /// Path to the graph database (default: `./kodegraph-out/codegraph.db`).
    #[arg(long, global = true, default_value = "kodegraph-out/codegraph.db")]
    pub db: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Build a graph from a source directory and persist it.
    Build {
        /// Directory to scan.
        path: PathBuf,
        /// Output directory for artifacts (report + viz). Default: `./kodegraph-out`.
        #[arg(long, default_value = "kodegraph-out")]
        out: PathBuf,
        /// Print a one-line summary.
        #[arg(long)]
        quiet: bool,
    },
    /// Query the graph for a focused subgraph under a token budget.
    Query {
        /// Query terms (space-separated).
        query: String,
        /// Max approximate tokens in the result.
        #[arg(long, default_value_t = 2000)]
        budget: usize,
    },
    /// Find the shortest path between two named symbols.
    Path {
        from: String,
        to: String,
    },
    /// Explain a symbol: node + neighbors + 2-hop blast radius.
    Explain {
        name: String,
    },
    /// Render the graph backbone.
    Viz {
        /// Emit Mermaid.
        #[arg(long)]
        mermaid: bool,
        /// Emit Graphviz DOT.
        #[arg(long)]
        dot: bool,
    },
    /// Print the GRAPH_REPORT.md to stdout.
    Report,
    /// Run the stdio MCP server over the persisted graph.
    Serve,
}

/// Entry point.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Build { path, out, quiet } => build(&path, &out, &cli.db, quiet),
        Cmd::Query { query, budget } => {
            let g = load(&cli.db)?;
            let sub = query::query_graph(&g, &query, budget);
            println!("{}", crate::mcp::format_subgraph(&sub));
            println!("-- est_tokens: {}", sub.est_tokens);
            Ok(())
        }
        Cmd::Path { from, to } => {
            let g = load(&cli.db)?;
            match query::shortest_path(&g, &from, &to) {
                Some((hops, nodes)) => {
                    let labels: Vec<String> = nodes.iter().map(|n| n.label()).collect();
                    println!("hops={hops}");
                    println!("{}", labels.join(" -> "));
                }
                None => println!("no path between {from} and {to}"),
            }
            Ok(())
        }
        Cmd::Explain { name } => {
            let g = load(&cli.db)?;
            if let Some(node) = query::get_node(&g, &name) {
                println!("node: {:#?}", node);
            } else {
                println!("no node named {name}");
            }
            let nbrs = query::get_neighbors(&g, &name);
            println!(
                "neighbors: {}",
                nbrs.iter().map(|n| n.label()).collect::<Vec<_>>().join(", ")
            );
            let br = query::blast_radius(&g, &name, 2);
            println!("blast radius (2 hops): {} affected", br.affected());
            Ok(())
        }
        Cmd::Viz { mermaid, dot } => {
            let g = load(&cli.db)?;
            if dot {
                println!("{}", render_dot(&g, "kodegraph"));
            } else {
                // default to mermaid if neither flag set
                println!("{}", render_mermaid(&g, "kodegraph"));
            }
            if mermaid && dot {
                println!("{}", render_mermaid(&g, "kodegraph"));
            }
            Ok(())
        }
        Cmd::Report => {
            let g = load(&cli.db)?;
            println!("{}", render_report(&g));
            Ok(())
        }
        Cmd::Serve => {
            let g = load(&cli.db)?;
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(McpServer::new(g).serve())
        }
    }
}

fn build(path: &Path, out: &Path, db: &Path, quiet: bool) -> Result<()> {
    std::fs::create_dir_all(out)?;
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Load existing graph + hashes for incremental update.
    let store = SQLiteStore::open(db)?;
    let existing = store.load_graph()?;
    let (graph, hashes) = CodeGraphBuilder::new()
        .scan_dir(path)?
        .with_existing_graph(existing.unwrap_or_default())
        .build()?;

    // Persist.
    store.save_graph(&graph)?;
    store.save_hashes(&hashes)?;

    // Artifacts.
    let report = render_report(&graph);
    std::fs::write(out.join("GRAPH_REPORT.md"), report)?;
    std::fs::write(out.join("graph.mmd"), render_mermaid(&graph, "kodegraph"))?;
    std::fs::write(
        out.join("graph.json"),
        serde_json::to_string_pretty(&graph)?,
    )?;

    if !quiet {
        println!(
            "kodegraph: built {} nodes, {} backbone edges, {} call edges from {}",
            graph.node_count(),
            graph.edge_count(),
            graph.call_edge_count(),
            path.display()
        );
        println!("  db:      {}", db.display());
        println!("  out:     {}", out.display());
        println!("  report:  {}/GRAPH_REPORT.md", out.display());
    }
    Ok(())
}

fn load(db: &Path) -> Result<crate::model::graph::CodeGraph> {
    let store = SQLiteStore::open(db)?;
    store
        .load_graph()?
        .ok_or_else(|| anyhow!("no graph at {}. Run `kodegraph build <dir>` first.", db.display()))
}