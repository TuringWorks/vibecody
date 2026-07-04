//! `CodeGraphBuilder` — walks a workspace, parses files, merges into a `CodeGraph`,
//! optionally enriches with an `EdgeProvider` (LSP/SCIP tier), and persists.
//!
//! Pipeline:
//! ```text
//! walk(workspace) ──► per file: hash ──► [unchanged?] skip
//!                                  ──► [changed/new] remove_file + parse + insert
//! optional EdgeProvider ──► upgrade call/type edges (Tier 2)
//! persist (graph + file_hashes)
//! ```

use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::incremental::{hash_content, FileHashes};
use crate::model::graph::CodeGraph;
use crate::parse::{language_of, EdgeProvider, Parser, TreeSitterParser};

/// Directories to skip during the walk.
const DEFAULT_IGNORED: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".next",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    "kodegraph-out",
];

/// Builds a `CodeGraph` from a workspace directory.
pub struct CodeGraphBuilder {
    root: Option<PathBuf>,
    parser: Box<dyn Parser>,
    edge_provider: Option<Box<dyn EdgeProvider>>,
    ignored: Vec<String>,
    /// Existing graph (for incremental updates). Loaded from a store or built fresh.
    existing: Option<CodeGraph>,
}

impl CodeGraphBuilder {
    /// Create a builder with the default tree-sitter parser.
    pub fn new() -> Self {
        Self {
            root: None,
            parser: Box::new(TreeSitterParser::new()),
            edge_provider: None,
            ignored: DEFAULT_IGNORED.iter().map(|s| s.to_string()).collect(),
            existing: None,
        }
    }

    /// Set the workspace root to scan.
    pub fn scan_dir(mut self, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            return Err(anyhow::anyhow!("scan dir does not exist: {}", root.display()));
        }
        self.root = Some(root);
        Ok(self)
    }

    /// Replace the parser (e.g. with a custom one implementing [`Parser`]).
    pub fn with_parser(mut self, parser: Box<dyn Parser>) -> Self {
        self.parser = parser;
        self
    }

    /// Attach a Tier-2 [`EdgeProvider`] (e.g. [`crate::parse::LspEdgeProvider`]) to
    /// upgrade call/type edges to compiler-grade confidence.
    pub fn with_edge_provider(mut self, provider: Box<dyn EdgeProvider>) -> Self {
        self.edge_provider = Some(provider);
        self
    }

    /// Add directories to the ignore list (in addition to the defaults).
    pub fn ignore_dirs(mut self, dirs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for d in dirs {
            self.ignored.push(d.into());
        }
        self
    }

    /// Seed the builder with an existing graph (incremental update). Usually loaded
    /// from a [`crate::store::Store`].
    pub fn with_existing_graph(mut self, graph: CodeGraph) -> Self {
        self.existing = Some(graph);
        self
    }

    /// Build the graph. Returns the new graph and the updated file-hash cache.
    pub fn build(self) -> Result<(CodeGraph, FileHashes)> {
        let root = self.root.as_ref().ok_or_else(|| anyhow::anyhow!("no scan dir set"))?;
        let mut graph = self.existing.unwrap_or_default();
        let mut hashes = FileHashes::new();

        let files = collect_files(root, &self.ignored);
        for file in files {
            let rel = file.to_string_lossy().to_string();
            let Ok(src) = std::fs::read_to_string(&file) else {
                continue;
            };
            let lang = language_of(&file);
            if !self.parser.supports(lang) {
                continue;
            }
            let hash = hash_content(&src);
            if hashes.is_unchanged(&rel, &hash) {
                continue;
            }
            // File changed (or new): drop its old contribution, re-parse, re-insert.
            graph.remove_file(&rel);
            let parsed = self.parser.parse_file(&file, &src, lang);
            for sym in parsed.symbols {
                graph.add_symbol(sym);
            }
            // Second pass: insert typed edges now that symbols exist (for backbone resolution).
            for call in parsed.calls {
                graph.add_call(call);
            }
            for imp in parsed.imports {
                graph.add_import(imp);
            }
            for rel_edge in parsed.type_relations {
                graph.add_type_relation(rel_edge);
            }
            for (qn, contract) in parsed.api_contracts {
                graph.add_api_contract(qn, contract);
            }
            // Ensure a file node exists for import-graph join.
            graph.add_file(&rel);
            hashes.set(rel.clone(), hash);
        }

        // Tier-2 enrichment (optional). Best-effort: failures don't abort the build.
        if let Some(provider) = self.edge_provider.as_ref() {
            let symbols: Vec<_> = graph.symbols().cloned().collect();
            for sym in symbols {
                if let Ok(outgoing) = provider.outgoing_calls(&sym) {
                    for c in outgoing {
                        graph.add_call(c);
                    }
                }
                if let Ok(supers) = provider.supertypes(&sym) {
                    for r in supers {
                        graph.add_type_relation(r);
                    }
                }
            }
        }

        Ok((graph, hashes))
    }
}

impl Default for CodeGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_files(root: &Path, ignored: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, ignored))
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|e| e.to_str()).is_some() {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

fn is_ignored(entry: &walkdir::DirEntry, ignored: &[String]) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let Some(name) = entry.file_name().to_str() else {
        return false;
    };
    ignored.iter().any(|i| i == name)
}