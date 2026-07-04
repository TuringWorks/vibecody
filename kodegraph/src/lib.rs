//! # kodegraph
//!
//! A tree-sitter + LSP **code knowledge graph** for token-efficient codebase navigation.
//!
//! `kodegraph` turns a source tree into a queryable graph of symbols and their
//! relationships (calls, imports, implements, extends, contains, references, depends-on).
//! Instead of dumping whole files into an LLM context window, an agent traverses a
//! bounded subgraph — a few hundred tokens instead of tens of thousands.
//!
//! ## Architecture (two-tier)
//!
//! - **Tier 1 — tree-sitter backbone** (default, zero-config): always works, no language
//!   server needed. Edges are AST-inferred at confidence `0.7`, tagged `Inferred`.
//! - **Tier 2 — LSP enrichment** (`lsp` feature): upgrades call-graph + type-hierarchy
//!   edges to confidence `0.95`, tagged `Extracted`. Skipped gracefully when no server
//!   is installed for a language.
//!
//! Every edge carries a [`Provenance`] — `source`, `tag` (`Extracted` / `Inferred` /
//! `Ambiguous`), and `confidence` — so consumers always know what was measured vs guessed.
//!
//! ## Quickstart
//!
//! ```no_run
//! use kodegraph::builder::CodeGraphBuilder;
//!
//! let (graph, _hashes) = CodeGraphBuilder::new()
//!     .scan_dir("src")?
//!     .build()?;
//!
//! // 1-hop callers of a symbol — a few hundred tokens, not whole files.
//! for edge in graph.callers("build_temp_provider") {
//!     println!("{} -> {}  [{:?}, conf {:.2}]",
//!         edge.caller, edge.callee, edge.provenance.tag, edge.provenance.confidence);
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Features
//!
//! | Feature | Default | Purpose |
//! |---|---|---|
//! | `tree-sitter` | yes | Zero-config AST backbone (Rust/TS/Python/Go) |
//! | `sqlite` | yes | Persistent graph store |
//! | `viz` | yes | Mermaid / DOT rendering |
//! | `lsp` | no | Optional LSP enrichment tier |
//! | `mcp` | no | stdio MCP server |
//! | `cli` | no | `kodegraph` binary |
//!
//! License: MIT.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::module_inception)]

pub mod model;
pub mod parse;
pub mod analyze;
pub mod query;
pub mod report;
pub mod incremental;

#[cfg(feature = "sqlite")]
pub mod store;

pub mod builder;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "cli")]
pub mod cli;

pub use model::{graph::CodeGraph, edge::Provenance, symbol::Symbol};
pub use builder::CodeGraphBuilder;