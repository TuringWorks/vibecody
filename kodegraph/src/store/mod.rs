//! Persistence — `Store` trait and a SQLite implementation.
//!
//! v0.1 persists the `CodeGraph` as a versioned JSON blob in SQLite plus a
//! `file_hashes` table that backs the incremental cache. A finer-grained
//! relational schema (symbols / edges / hyperedges as rows) is a documented
//! follow-up; the blob approach is robust against schema drift and adequate for
//! the graph sizes a single workspace produces.

pub mod sqlite;

pub use sqlite::SQLiteStore;

use anyhow::Result;

use crate::incremental::FileHashes;
use crate::model::graph::CodeGraph;

/// A persistence backend for a `CodeGraph` + incremental file-hash cache.
pub trait Store: Send + Sync {
    /// Save the full graph (replaces any prior content).
    fn save_graph(&self, graph: &CodeGraph) -> Result<()>;

    /// Load the full graph, or `None` if the store is empty.
    fn load_graph(&self) -> Result<Option<CodeGraph>>;

    /// Save the file-hash cache (replaces any prior content).
    fn save_hashes(&self, hashes: &FileHashes) -> Result<()>;

    /// Load the file-hash cache, or an empty cache if absent.
    fn load_hashes(&self) -> Result<FileHashes>;
}