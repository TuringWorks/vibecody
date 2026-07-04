//! Edge model — call/import/type relationships plus Graphify-style provenance.
//!
//! Every edge in a `CodeGraph` carries a [`Provenance`] recording *how* it was
//! discovered (`source`), *whether it was found or guessed* (`tag`), and a
//! `confidence` in `[0, 1]`. This is the single most important idea borrowed from
//! Graphify: a graph that knows what it measured vs what it inferred.

use serde::{Deserialize, Serialize};

/// Where an edge came from. Determines the default confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSource {
    /// Discovered by the tree-sitter AST backbone (confidence ~0.7).
    TreeSitter,
    /// Discovered by an LSP server call/type hierarchy (confidence ~0.95).
    Lsp,
    /// Discovered by a SCIP index (compiler-grade, confidence 1.0).
    Scip,
    /// Discovered by a regex/line-prefix heuristic (lowest tier, confidence ~0.5).
    Regex,
    /// Inferred by a model / heuristic without direct structural evidence.
    Inferred,
}

impl EdgeSource {
    /// The default confidence associated with a source.
    pub fn default_confidence(self) -> f32 {
        match self {
            Self::Scip => 1.0,
            Self::Lsp => 0.95,
            Self::TreeSitter => 0.7,
            Self::Regex => 0.5,
            Self::Inferred => 0.4,
        }
    }
}

/// Graphify provenance tag — was the edge found, inferred, or flagged for review?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceTag {
    /// Found directly in source (confidence typically 0.95–1.0).
    Extracted,
    /// Reasonable inference, with a confidence score (typically 0.4–0.8).
    Inferred,
    /// Flagged for human review — the extractor was not sure.
    Ambiguous,
}

/// Provenance attached to every edge. Lets a consumer trust-but-verify.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// How the edge was discovered.
    pub source: EdgeSource,
    /// Found / inferred / ambiguous.
    pub tag: ProvenanceTag,
    /// Confidence in `[0, 1]`.
    pub confidence: f32,
}

impl Provenance {
    /// Construct from a source, deriving the default confidence and tag.
    pub fn from_source(source: EdgeSource) -> Self {
        let confidence = source.default_confidence();
        let tag = match source {
            EdgeSource::Scip | EdgeSource::Lsp => ProvenanceTag::Extracted,
            EdgeSource::TreeSitter | EdgeSource::Regex | EdgeSource::Inferred => {
                ProvenanceTag::Inferred
            }
        };
        Self { source, tag, confidence }
    }

    /// An explicitly inferred edge with a custom confidence.
    pub fn inferred(confidence: f32) -> Self {
        Self { source: EdgeSource::Inferred, tag: ProvenanceTag::Inferred, confidence }
    }

    /// An explicitly extracted edge with a custom confidence.
    pub fn extracted(source: EdgeSource, confidence: f32) -> Self {
        Self { source, tag: ProvenanceTag::Extracted, confidence }
    }

    /// An ambiguous edge flagged for review.
    pub fn ambiguous(source: EdgeSource, confidence: f32) -> Self {
        Self { source, tag: ProvenanceTag::Ambiguous, confidence }
    }
}

/// The seven first-class relationship kinds in a code knowledge graph.
///
/// Matches the edge-type set promised by `vibecli/vibecli-cli/skills/knowledge-graph.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// `caller` calls `callee`.
    Calls,
    /// `source_file` imports `target` / `imported_symbols`.
    Imports,
    /// `impl` implements `trait` / interface.
    Implements,
    /// `child` extends `parent` (class inheritance).
    Extends,
    /// `parent` contains `child` (module/file → symbol, type → method).
    Contains,
    /// `referrer` references `target` (read/use, not a call).
    References,
    /// `from` depends on `to` (module/file granularity).
    DependsOn,
}

impl EdgeKind {
    /// Stable lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Implements => "implements",
            Self::Extends => "extends",
            Self::Contains => "contains",
            Self::References => "references",
            Self::DependsOn => "depends_on",
        }
    }
}

/// Type of function/method call (structural flavor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallType {
    /// A direct call `foo()`.
    Direct,
    /// A method call `obj.foo()`.
    Method,
    /// A constructor `Foo::new()`.
    Constructor,
    /// A callback / function-pointer invocation.
    Callback,
    /// An `await` / async call.
    Async,
    /// A dynamically-dispatched call (trait object, virtual, reflection).
    Dynamic,
}

/// A directed edge in the call graph (`Calls`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    /// Calling symbol's qualified name.
    pub caller: String,
    /// Called symbol's qualified name (best-effort; may be unresolved).
    pub callee: String,
    /// File containing the call site.
    pub file: String,
    /// 1-based line of the call site.
    pub line: usize,
    /// Structural call type.
    pub call_type: CallType,
    /// How this edge was discovered.
    pub provenance: Provenance,
}

/// Relationship between two types (`Implements` / `Extends`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeRelationType {
    /// Class inheritance.
    Inherits,
    /// Interface / trait implementation.
    Implements,
    /// Rust trait impl (specialized form of Implements).
    TraitImpl,
    /// Class extends (JS/TS/Java).
    Extends,
    /// Mixin (Dart/Ruby).
    Mixin,
}

/// A directed edge in the type hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRelation {
    /// Parent type qualified name.
    pub parent: String,
    /// Child type qualified name.
    pub child: String,
    /// Kind of relationship.
    pub relation: TypeRelationType,
    /// Provenance.
    pub provenance: Provenance,
}

/// Import type (structural flavor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportType {
    /// `use a::b;` / `import { b } from 'a'`.
    Named,
    /// `use a::*;` / `import * as a from 'a'`.
    Wildcard,
    /// `import Foo from 'a'` (default export).
    Default,
    /// `pub use a::b as c;` / re-export.
    Reexport,
}

/// A directed edge in the import graph (`Imports`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEdge {
    /// Source file path.
    pub source_file: String,
    /// Target module/path string as written in source.
    pub target: String,
    /// Symbols imported (may be empty for wildcard).
    pub imported_symbols: Vec<String>,
    /// Structural import type.
    pub import_type: ImportType,
    /// Provenance.
    pub provenance: Provenance,
}

/// API contract for a function/method (params, return, errors, async-ness).
///
/// Useful for emitting compact "signature-only" context chunks (Level-2 skeleton in
/// the infinite-context sense) instead of full bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiContract {
    /// Parameter names + types as text, e.g. `["provider: String", "model: String"]`.
    pub params: Vec<String>,
    /// Return type text, if known.
    pub return_type: Option<String>,
    /// Error types the function may yield, if known.
    pub error_types: Vec<String>,
    /// Whether the function is async.
    pub is_async: bool,
}