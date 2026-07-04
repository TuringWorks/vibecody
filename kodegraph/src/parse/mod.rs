//! Parser + edge-provider traits and the tree-sitter backbone.
//!
//! Two extension points let consumers plug in their own analysis:
//! - [`Parser`] produces symbols + structural edges from a single file's source.
//! - [`EdgeProvider`] upgrades edges with cross-file / compiler-grade precision
//!   (LSP call/type hierarchy, SCIP, etc.).
//!
//! `kodegraph` ships a [`TreeSitterParser`] (`tree-sitter` feature) and, with the
//! `lsp` feature, an [`LspEdgeProvider`] that uses its own minimal stdio LSP client
//! so the crate stays independent of any particular editor's LSP wrapper.

pub mod treesitter;

#[cfg(feature = "lsp")]
pub mod lsp;

use std::path::Path;

use anyhow::Result;

use crate::model::edge::{ApiContract, CallEdge, ImportEdge, Provenance, TypeRelation};
use crate::model::symbol::{Language, Symbol};

pub use treesitter::TreeSitterParser;

#[cfg(feature = "lsp")]
pub use lsp::LspEdgeProvider;

/// The result of parsing one file: symbols + structural edges with provenance.
#[derive(Debug, Default, Clone)]
pub struct ParsedFile {
    /// Discovered symbols.
    pub symbols: Vec<Symbol>,
    /// Discovered call edges (caller qualified name + callee text).
    pub calls: Vec<CallEdge>,
    /// Discovered import edges.
    pub imports: Vec<ImportEdge>,
    /// Discovered type-hierarchy relations.
    pub type_relations: Vec<TypeRelation>,
    /// API contracts keyed by qualified name.
    pub api_contracts: Vec<(String, ApiContract)>,
}

/// A single-file structural parser. Implementations include [`TreeSitterParser`].
///
/// Implementations must be deterministic and side-effect free: the same `(path, src)`
/// always yields the same `ParsedFile`. This is what makes the incremental
/// content-hash cache sound.
pub trait Parser: Send + Sync {
    /// Parse `src` at `path` for the given `language`.
    fn parse_file(&self, path: &Path, src: &str, lang: Language) -> ParsedFile;

    /// Whether this parser can handle the language at all.
    fn supports(&self, lang: Language) -> bool;
}

/// A cross-file edge provider that upgrades the graph with compiler-grade precision.
///
/// Used as **Tier 2** enrichment on top of a tree-sitter backbone. Implementations
/// (LSP, SCIP) raise edge confidence and resolve cross-file / cross-package
/// references that a single-file AST cannot.
pub trait EdgeProvider: Send + Sync {
    /// Incoming callers of `sym` (who calls it).
    fn incoming_calls(&self, sym: &Symbol) -> Result<Vec<CallEdge>>;

    /// Outgoing callees of `sym` (what it calls).
    fn outgoing_calls(&self, sym: &Symbol) -> Result<Vec<CallEdge>>;

    /// Supertypes / implemented interfaces of `sym`.
    fn supertypes(&self, sym: &Symbol) -> Result<Vec<TypeRelation>>;

    /// Subtypes / implementors of `sym`.
    fn subtypes(&self, sym: &Symbol) -> Result<Vec<TypeRelation>>;

    /// Human-readable name for logging.
    fn name(&self) -> &'static str;
}

/// Default provenance for tree-sitter-discovered edges.
pub(crate) fn ts_provenance() -> Provenance {
    Provenance::from_source(crate::model::edge::EdgeSource::TreeSitter)
}

/// Detect language from a path; returns `Unknown` for unsupported extensions.
pub fn language_of(path: &Path) -> Language {
    path.extension()
        .and_then(|e| e.to_str())
        .map(Language::from_extension)
        .unwrap_or(Language::Unknown)
}