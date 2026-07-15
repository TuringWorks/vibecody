//! SQLite [`Store`] backend — zero-config, local-first durability (feature `sqlite`).
//!
//! Runs and definitions are stored as JSON documents with a handful of indexed columns.
//! Calls are synchronous under the hood (SQLite is local and fast); no `.await` is held
//! across the connection lock.

use crate::{status_str, Result, Store, StoreError};
use async_trait::async_trait;
use fluxo_core::run::WorkflowStatus;
use fluxo_core::{WorkflowDef, WorkflowRun};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

/// A durable store backed by a single SQLite database (WAL mode).
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

fn backend<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl SqliteStore {
    /// Open (creating if needed) a database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).map_err(backend)?;
        Self::init(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open a private in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(backend)?;
        Self::init(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS workflow_def (
                 name TEXT NOT NULL, version INTEGER NOT NULL, json TEXT NOT NULL,
                 PRIMARY KEY (name, version)
             );
             CREATE TABLE IF NOT EXISTS workflow_run (
                 id TEXT PRIMARY KEY, name TEXT NOT NULL, version INTEGER NOT NULL,
                 status TEXT NOT NULL, correlation_id TEXT, json TEXT NOT NULL,
                 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_run_status ON workflow_run(status);",
        )
        .map_err(backend)?;
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| StoreError::Backend("sqlite connection mutex poisoned".into()))
    }
}

#[async_trait]
impl Store for SqliteStore {
    async fn put_workflow_def(&self, def: &WorkflowDef) -> Result<()> {
        let json = serde_json::to_string(def)?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO workflow_def (name, version, json) VALUES (?1, ?2, ?3)",
            params![def.name, def.version, json],
        )
        .map_err(backend)?;
        Ok(())
    }

    async fn get_workflow_def(&self, name: &str, version: Option<u32>) -> Result<Option<WorkflowDef>> {
        let conn = self.lock()?;
        let json: Option<String> = match version {
            Some(v) => conn
                .query_row(
                    "SELECT json FROM workflow_def WHERE name = ?1 AND version = ?2",
                    params![name, v],
                    |row| row.get(0),
                )
                .optional()
                .map_err(backend)?,
            None => conn
                .query_row(
                    "SELECT json FROM workflow_def WHERE name = ?1 ORDER BY version DESC LIMIT 1",
                    params![name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(backend)?,
        };
        json.map(|j| serde_json::from_str(&j).map_err(StoreError::from)).transpose()
    }

    async fn list_workflow_defs(&self) -> Result<Vec<(String, u32)>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT name, version FROM workflow_def ORDER BY name, version")
            .map_err(backend)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)))
            .map_err(backend)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(backend)
    }

    async fn create_run(&self, run: &WorkflowRun) -> Result<()> {
        let json = serde_json::to_string(run)?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO workflow_run (id, name, version, status, correlation_id, json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run.workflow_id,
                run.workflow_name,
                run.workflow_version,
                status_str(&run.status),
                run.correlation_id,
                json,
                run.created_at,
                run.updated_at
            ],
        )
        .map_err(|e| StoreError::Conflict(e.to_string()))?;
        Ok(())
    }

    async fn get_run(&self, workflow_id: &str) -> Result<Option<WorkflowRun>> {
        let conn = self.lock()?;
        let json: Option<String> = conn
            .query_row(
                "SELECT json FROM workflow_run WHERE id = ?1",
                params![workflow_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        json.map(|j| serde_json::from_str(&j).map_err(StoreError::from)).transpose()
    }

    async fn update_run(&self, run: &WorkflowRun) -> Result<()> {
        let json = serde_json::to_string(run)?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO workflow_run (id, name, version, status, correlation_id, json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run.workflow_id,
                run.workflow_name,
                run.workflow_version,
                status_str(&run.status),
                run.correlation_id,
                json,
                run.created_at,
                run.updated_at
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    async fn list_runs(&self, status: Option<WorkflowStatus>) -> Result<Vec<WorkflowRun>> {
        let conn = self.lock()?;
        let jsons: Vec<String> = match status {
            Some(s) => {
                let mut stmt = conn
                    .prepare("SELECT json FROM workflow_run WHERE status = ?1 ORDER BY created_at")
                    .map_err(backend)?;
                let rows = stmt
                    .query_map(params![status_str(&s)], |row| row.get::<_, String>(0))
                    .map_err(backend)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(backend)?
            }
            None => {
                let mut stmt = conn
                    .prepare("SELECT json FROM workflow_run ORDER BY created_at")
                    .map_err(backend)?;
                let rows = stmt
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(backend)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(backend)?
            }
        };
        jsons
            .into_iter()
            .map(|j| serde_json::from_str(&j).map_err(StoreError::from))
            .collect()
    }
}
