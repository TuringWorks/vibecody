//! # fluxo-store
//!
//! Persistence for the Fluxo engine: one [`Store`] trait with three backends —
//! [`memory::MemoryStore`] (always available), [`sqlite::SqliteStore`] (feature `sqlite`,
//! default), and [`postgres::PostgresStore`] (feature `postgres`).
//!
//! Runs and definitions are persisted as JSON documents with a few indexed columns; this
//! keeps the two SQL backends nearly identical and the schema trivial to evolve.

#![forbid(unsafe_code)]

pub mod memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

use async_trait::async_trait;
use fluxo_core::model::TaskType;
use fluxo_core::run::{TaskExecution, TaskStatus, WorkflowStatus};
use fluxo_core::{WorkflowDef, WorkflowRun};
use thiserror::Error;

/// Errors returned by a [`Store`] backend.
#[derive(Debug, Error)]
pub enum StoreError {
    /// The requested entity does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// A uniqueness or optimistic-concurrency constraint was violated.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The underlying backend failed.
    #[error("backend error: {0}")]
    Backend(String),
    /// JSON (de)serialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Convenience alias for store results.
pub type Result<T> = std::result::Result<T, StoreError>;

/// A task claimed by a worker via [`Store::poll_task`].
#[derive(Debug, Clone)]
pub struct PolledTask {
    /// The run the task belongs to.
    pub workflow_id: String,
    /// The claimed task, marked `InProgress`.
    pub task: TaskExecution,
}

/// Durable storage for workflow definitions and runs.
///
/// Backends implement the primitive methods; [`Store::poll_task`] has a portable default
/// implementation built on `list_runs` + `update_run`.
#[async_trait]
pub trait Store: Send + Sync {
    /// Register (or replace) a workflow definition at its `(name, version)`.
    async fn put_workflow_def(&self, def: &WorkflowDef) -> Result<()>;

    /// Fetch a definition by name; `version = None` returns the latest.
    async fn get_workflow_def(&self, name: &str, version: Option<u32>) -> Result<Option<WorkflowDef>>;

    /// List all registered `(name, version)` pairs.
    async fn list_workflow_defs(&self) -> Result<Vec<(String, u32)>>;

    /// Persist a newly-created run.
    async fn create_run(&self, run: &WorkflowRun) -> Result<()>;

    /// Fetch a run by id.
    async fn get_run(&self, workflow_id: &str) -> Result<Option<WorkflowRun>>;

    /// Persist the full current state of a run (upsert).
    async fn update_run(&self, run: &WorkflowRun) -> Result<()>;

    /// List runs, optionally filtered by status.
    async fn list_runs(&self, status: Option<WorkflowStatus>) -> Result<Vec<WorkflowRun>>;

    /// Claim one `Scheduled` worker task of `task_type`, marking it `InProgress` for `worker_id`.
    ///
    /// The default implementation scans running workflows; it is correct but not atomic
    /// across concurrent pollers in the same process. Backends may override for stronger
    /// semantics or a dedicated queue index.
    async fn poll_task(&self, task_type: &str, worker_id: &str) -> Result<Option<PolledTask>> {
        let running = self.list_runs(Some(WorkflowStatus::Running)).await?;
        for mut run in running {
            let candidate = run.tasks.iter().position(|t| {
                t.status == TaskStatus::Scheduled
                    && t.task_name == task_type
                    && matches!(t.task_type, TaskType::Simple | TaskType::Other)
            });
            if let Some(idx) = candidate {
                run.tasks[idx].status = TaskStatus::InProgress;
                run.tasks[idx].worker_id = Some(worker_id.to_string());
                let task = run.tasks[idx].clone();
                let workflow_id = run.workflow_id.clone();
                self.update_run(&run).await?;
                return Ok(Some(PolledTask { workflow_id, task }));
            }
        }
        Ok(None)
    }
}

/// Render a [`WorkflowStatus`] as its canonical string (e.g. `RUNNING`).
pub(crate) fn status_str(status: &WorkflowStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "RUNNING".to_string())
}
