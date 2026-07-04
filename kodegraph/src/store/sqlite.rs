//! SQLite-backed `Store`. Uses `rusqlite` (bundled) so there is no external SQLite
//! dependency — zero-config.

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use crate::incremental::FileHashes;
use crate::model::graph::CodeGraph;
use crate::store::Store;

/// A SQLite graph store. Single row keyed by an integer id; thread-safe via a
/// `Mutex<Connection>` (SQLite connections are not `Sync`).
pub struct SQLiteStore {
    conn: std::sync::Mutex<Connection>,
}

impl SQLiteStore {
    /// Open (or create) a store at `path`.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS graph (
                id INTEGER PRIMARY KEY,
                payload TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS file_hashes (
                path TEXT PRIMARY KEY,
                hash TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }

    /// Open an in-memory store (useful for tests + ephemeral sessions).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS graph (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS file_hashes (path TEXT PRIMARY KEY, hash TEXT NOT NULL);",
        )?;
        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }
}

impl Store for SQLiteStore {
    fn save_graph(&self, graph: &CodeGraph) -> Result<()> {
        let payload = serde_json::to_string(graph)?;
        let conn = self.conn.lock().unwrap();
        let now = rfc3339_now();
        conn.execute(
            "INSERT OR REPLACE INTO graph (id, payload, updated_at) VALUES (1, ?1, ?2)",
            params![payload, now],
        )?;
        Ok(())
    }

    fn load_graph(&self) -> Result<Option<CodeGraph>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT payload FROM graph WHERE id = 1")?;
        let row: rusqlite::Result<String> = stmt.query_row([], |r| r.get(0));
        match row {
            Ok(payload) => {
                let g: CodeGraph = serde_json::from_str(&payload)
                    .map_err(|e| anyhow!("decode graph: {e}"))?;
                Ok(Some(g))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save_hashes(&self, hashes: &FileHashes) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_hashes", [])?;
        let json = hashes.to_json()?;
        // Store the whole cache as one JSON blob in a synthetic row so we don't need
        // per-row insert batching in v0.1. Keyed by a reserved path sentinel.
        conn.execute(
            "INSERT OR REPLACE INTO file_hashes (path, hash) VALUES (?1, ?2)",
            params!["__cache_blob__", json],
        )?;
        Ok(())
    }

    fn load_hashes(&self) -> Result<FileHashes> {
        let conn = self.conn.lock().unwrap();
        let row: rusqlite::Result<String> = conn.query_row(
            "SELECT hash FROM file_hashes WHERE path = '__cache_blob__'",
            [],
            |r| r.get(0),
        );
        match row {
            Ok(json) => Ok(FileHashes::from_json(&json)
                .unwrap_or_else(|_| FileHashes::new())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FileHashes::new()),
            Err(e) => Err(e.into()),
        }
    }
}

fn rfc3339_now() -> String {
    // Deterministic-enough timestamp; std::time wall clock is fine for a "updated_at"
    // column used only for human inspection.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::{CallEdge, CallType, EdgeSource, Provenance};
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
    fn roundtrip_graph_and_hashes() {
        let store = SQLiteStore::open_memory().unwrap();
        let mut g = CodeGraph::new();
        g.add_symbol(sym("foo"));
        g.add_symbol(sym("bar"));
        g.add_call(CallEdge {
            caller: "pkg::foo".into(),
            callee: "pkg::bar".into(),
            file: "foo.rs".into(),
            line: 2,
            call_type: CallType::Direct,
            provenance: Provenance::from_source(EdgeSource::TreeSitter),
        });
        store.save_graph(&g).unwrap();

        let loaded = store.load_graph().unwrap().unwrap();
        assert_eq!(loaded.node_count(), g.node_count());
        assert_eq!(loaded.call_edge_count(), 1);

        let mut hashes = FileHashes::new();
        hashes.set("foo.rs", "abc");
        store.save_hashes(&hashes).unwrap();
        let h2 = store.load_hashes().unwrap();
        assert_eq!(h2.get("foo.rs"), Some("abc"));
    }
}