//! In-memory [`Store`] backend — always available, ideal for tests and ephemeral runs.

use crate::{status_str, Result, Store, StoreError};
use async_trait::async_trait;
use fluxo_core::run::WorkflowStatus;
use fluxo_core::{WorkflowDef, WorkflowRun};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// A thread-safe, process-local store backed by `BTreeMap`s.
#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    defs: BTreeMap<(String, u32), WorkflowDef>,
    runs: BTreeMap<String, WorkflowRun>,
}

impl MemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| StoreError::Backend("memory store mutex poisoned".into()))
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn put_workflow_def(&self, def: &WorkflowDef) -> Result<()> {
        let mut inner = self.lock()?;
        inner.defs.insert((def.name.clone(), def.version), def.clone());
        Ok(())
    }

    async fn get_workflow_def(&self, name: &str, version: Option<u32>) -> Result<Option<WorkflowDef>> {
        let inner = self.lock()?;
        let found = match version {
            Some(v) => inner.defs.get(&(name.to_string(), v)).cloned(),
            None => inner
                .defs
                .iter()
                .filter(|((n, _), _)| n == name)
                .max_by_key(|((_, v), _)| *v)
                .map(|(_, def)| def.clone()),
        };
        Ok(found)
    }

    async fn list_workflow_defs(&self) -> Result<Vec<(String, u32)>> {
        let inner = self.lock()?;
        Ok(inner.defs.keys().cloned().collect())
    }

    async fn create_run(&self, run: &WorkflowRun) -> Result<()> {
        let mut inner = self.lock()?;
        if inner.runs.contains_key(&run.workflow_id) {
            return Err(StoreError::Conflict(format!("run {} exists", run.workflow_id)));
        }
        inner.runs.insert(run.workflow_id.clone(), run.clone());
        Ok(())
    }

    async fn get_run(&self, workflow_id: &str) -> Result<Option<WorkflowRun>> {
        let inner = self.lock()?;
        Ok(inner.runs.get(workflow_id).cloned())
    }

    async fn update_run(&self, run: &WorkflowRun) -> Result<()> {
        let mut inner = self.lock()?;
        inner.runs.insert(run.workflow_id.clone(), run.clone());
        Ok(())
    }

    async fn list_runs(&self, status: Option<WorkflowStatus>) -> Result<Vec<WorkflowRun>> {
        let inner = self.lock()?;
        let wanted = status.map(|s| status_str(&s));
        Ok(inner
            .runs
            .values()
            .filter(|r| wanted.as_deref().map(|w| status_str(&r.status) == w).unwrap_or(true))
            .cloned()
            .collect())
    }
}
