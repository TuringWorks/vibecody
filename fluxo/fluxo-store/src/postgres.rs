//! PostgreSQL [`Store`] backend for horizontal scale (feature `postgres`).
//!
//! Uses the runtime `sqlx` query API (no compile-time `DATABASE_URL` needed). Runs and
//! definitions are stored as JSON text with indexed columns, mirroring the SQLite backend.

use crate::{status_str, Result, Store, StoreError};
use async_trait::async_trait;
use fluxo_core::run::WorkflowStatus;
use fluxo_core::{WorkflowDef, WorkflowRun};
use sqlx::{PgPool, Row};

/// A durable store backed by PostgreSQL.
pub struct PostgresStore {
    pool: PgPool,
}

fn backend<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl PostgresStore {
    /// Connect to Postgres at `url` and ensure the schema exists.
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPool::connect(url).await.map_err(backend)?;
        Self::init(&pool).await?;
        Ok(Self { pool })
    }

    /// Build a store from an existing pool.
    pub async fn from_pool(pool: PgPool) -> Result<Self> {
        Self::init(&pool).await?;
        Ok(Self { pool })
    }

    async fn init(pool: &PgPool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS workflow_def (
                 name TEXT NOT NULL, version INTEGER NOT NULL, json TEXT NOT NULL,
                 PRIMARY KEY (name, version)
             )",
        )
        .execute(pool)
        .await
        .map_err(backend)?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS workflow_run (
                 id TEXT PRIMARY KEY, name TEXT NOT NULL, version INTEGER NOT NULL,
                 status TEXT NOT NULL, correlation_id TEXT, json TEXT NOT NULL,
                 created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL
             )",
        )
        .execute(pool)
        .await
        .map_err(backend)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_run_status ON workflow_run(status)")
            .execute(pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

#[async_trait]
impl Store for PostgresStore {
    async fn put_workflow_def(&self, def: &WorkflowDef) -> Result<()> {
        let json = serde_json::to_string(def)?;
        sqlx::query(
            "INSERT INTO workflow_def (name, version, json) VALUES ($1, $2, $3)
             ON CONFLICT (name, version) DO UPDATE SET json = EXCLUDED.json",
        )
        .bind(&def.name)
        .bind(def.version as i32)
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn get_workflow_def(&self, name: &str, version: Option<u32>) -> Result<Option<WorkflowDef>> {
        let row = match version {
            Some(v) => sqlx::query("SELECT json FROM workflow_def WHERE name = $1 AND version = $2")
                .bind(name)
                .bind(v as i32)
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?,
            None => sqlx::query(
                "SELECT json FROM workflow_def WHERE name = $1 ORDER BY version DESC LIMIT 1",
            )
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?,
        };
        match row {
            Some(r) => {
                let json: String = r.try_get("json").map_err(backend)?;
                Ok(Some(serde_json::from_str(&json)?))
            }
            None => Ok(None),
        }
    }

    async fn list_workflow_defs(&self) -> Result<Vec<(String, u32)>> {
        let rows = sqlx::query("SELECT name, version FROM workflow_def ORDER BY name, version")
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.into_iter()
            .map(|r| {
                let name: String = r.try_get("name").map_err(backend)?;
                let version: i32 = r.try_get("version").map_err(backend)?;
                Ok((name, version as u32))
            })
            .collect()
    }

    async fn create_run(&self, run: &WorkflowRun) -> Result<()> {
        let json = serde_json::to_string(run)?;
        sqlx::query(
            "INSERT INTO workflow_run (id, name, version, status, correlation_id, json, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&run.workflow_id)
        .bind(&run.workflow_name)
        .bind(run.workflow_version as i32)
        .bind(status_str(&run.status))
        .bind(&run.correlation_id)
        .bind(json)
        .bind(run.created_at)
        .bind(run.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Conflict(e.to_string()))?;
        Ok(())
    }

    async fn get_run(&self, workflow_id: &str) -> Result<Option<WorkflowRun>> {
        let row = sqlx::query("SELECT json FROM workflow_run WHERE id = $1")
            .bind(workflow_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(r) => {
                let json: String = r.try_get("json").map_err(backend)?;
                Ok(Some(serde_json::from_str(&json)?))
            }
            None => Ok(None),
        }
    }

    async fn update_run(&self, run: &WorkflowRun) -> Result<()> {
        let json = serde_json::to_string(run)?;
        sqlx::query(
            "INSERT INTO workflow_run (id, name, version, status, correlation_id, json, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                 status = EXCLUDED.status, json = EXCLUDED.json, updated_at = EXCLUDED.updated_at",
        )
        .bind(&run.workflow_id)
        .bind(&run.workflow_name)
        .bind(run.workflow_version as i32)
        .bind(status_str(&run.status))
        .bind(&run.correlation_id)
        .bind(json)
        .bind(run.created_at)
        .bind(run.updated_at)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn list_runs(&self, status: Option<WorkflowStatus>) -> Result<Vec<WorkflowRun>> {
        let rows = match status {
            Some(s) => sqlx::query("SELECT json FROM workflow_run WHERE status = $1 ORDER BY created_at")
                .bind(status_str(&s))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?,
            None => sqlx::query("SELECT json FROM workflow_run ORDER BY created_at")
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?,
        };
        rows.into_iter()
            .map(|r| {
                let json: String = r.try_get("json").map_err(backend)?;
                serde_json::from_str(&json).map_err(StoreError::from)
            })
            .collect()
    }
}
