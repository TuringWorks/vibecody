//! Reporting — `GRAPH_REPORT.md`, Mermaid, and DOT output.
//!
//! `GRAPH_REPORT.md` is the human/agent-readable summary a consumer reads *before*
//! grepping raw files (mirroring Graphify's always-on-nudge pattern): god nodes,
//! community structure, and surprising cross-file connections.
//!
//! Mermaid / DOT renderers generalize `vibecli/.../dep_visualizer.rs`'s
//! `render_mermaid` / `render_dot` to the full `CodeGraph`.

pub mod markdown;
pub mod render;

pub use markdown::render_report;
pub use render::{render_dot, render_mermaid};