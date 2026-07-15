//! # fluxo-engine
//!
//! The execution engine: it wires the pure [`fluxo_core`] decider to a [`fluxo_store::Store`]
//! and drives workflows to a fixed point.
//!
//! Lifecycle:
//! 1. [`Engine::register`] a versioned [`WorkflowDef`].
//! 2. [`Engine::start`] a run from input.
//! 3. Workers [`Engine::poll`] tasks by type and report results with [`Engine::complete_task`].
//! 4. `WAIT`/`HUMAN` tasks resume via [`Engine::signal`].
//!
//! Every state transition is persisted through the store, so runs survive process restarts.

#![forbid(unsafe_code)]

use fluxo_core::model::TaskType;
use fluxo_core::run::{TaskStatus, WorkflowRun, WorkflowStatus};
use fluxo_core::{decide, validate, Decision, WorkflowDef};
use fluxo_store::{PolledTask, Store};
use serde_json::{Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Upper bound on decide iterations per drive, guarding against a malformed definition.
const MAX_ITERS: usize = 10_000;

/// Errors surfaced by the engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A core (DSL/decider) error.
    #[error("core error: {0}")]
    Core(#[from] fluxo_core::FluxoError),
    /// A storage error.
    #[error("store error: {0}")]
    Store(#[from] fluxo_store::StoreError),
    /// A referenced entity was not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// The request was invalid for the current state.
    #[error("invalid: {0}")]
    Invalid(String),
}

/// Convenience alias for engine results.
pub type Result<T> = std::result::Result<T, EngineError>;

/// The workflow engine over a pluggable [`Store`].
pub struct Engine<S: Store> {
    store: S,
}

impl<S: Store> Engine<S> {
    /// Build an engine over `store`.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Borrow the underlying store (e.g. for a worker poll loop).
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Validate and register a workflow definition.
    pub async fn register(&self, def: &WorkflowDef) -> Result<()> {
        validate(def)?;
        self.store.put_workflow_def(def).await?;
        Ok(())
    }

    /// Start a new run of `name` (latest version unless pinned) and drive it forward.
    /// Returns the new `workflow_id`.
    pub async fn start(
        &self,
        name: &str,
        version: Option<u32>,
        input: Value,
        correlation_id: Option<String>,
    ) -> Result<String> {
        let def = self
            .store
            .get_workflow_def(name, version)
            .await?
            .ok_or_else(|| EngineError::NotFound(format!("workflow def '{}'", name)))?;

        let now = now_ms();
        let run = WorkflowRun {
            workflow_id: new_id(),
            workflow_name: def.name.clone(),
            workflow_version: def.version,
            status: WorkflowStatus::Running,
            input,
            output: Value::Null,
            variables: Map::new(),
            tasks: Vec::new(),
            correlation_id,
            reason_for_incompletion: None,
            created_at: now,
            updated_at: now,
        };
        let id = run.workflow_id.clone();
        self.store.create_run(&run).await?;
        self.drive(&def, &id).await?;
        Ok(id)
    }

    /// Fetch a run by id.
    pub async fn get_run(&self, workflow_id: &str) -> Result<WorkflowRun> {
        self.store
            .get_run(workflow_id)
            .await?
            .ok_or_else(|| EngineError::NotFound(format!("run '{}'", workflow_id)))
    }

    /// Claim one scheduled worker task of `task_type` for `worker_id`.
    pub async fn poll(&self, task_type: &str, worker_id: &str) -> Result<Option<PolledTask>> {
        Ok(self.store.poll_task(task_type, worker_id).await?)
    }

    /// Report the result of a task (worker completion) and drive the run forward.
    pub async fn complete_task(
        &self,
        workflow_id: &str,
        task_id: &str,
        status: TaskStatus,
        output: Value,
    ) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        let now = now_ms();
        {
            let task = run
                .task_by_id_mut(task_id)
                .ok_or_else(|| EngineError::NotFound(format!("task '{}'", task_id)))?;
            if task.status.is_terminal() {
                return Err(EngineError::Invalid(format!(
                    "task '{}' already terminal ({:?})",
                    task_id, task.status
                )));
            }
            task.status = status;
            task.output = output;
            task.updated_at = now;
            if !status.is_success() {
                task.reason_for_incompletion = Some("reported failure".into());
            }
        }
        run.updated_at = now;
        self.store.update_run(&run).await?;
        let def = self.def_for(&run).await?;
        self.drive(&def, workflow_id).await
    }

    /// Complete a durable `WAIT`/`HUMAN` task identified by its reference name.
    pub async fn signal(
        &self,
        workflow_id: &str,
        reference_name: &str,
        output: Value,
    ) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        let now = now_ms();
        let idx = run
            .tasks
            .iter()
            .position(|t| {
                t.reference_name == reference_name
                    && !t.status.is_terminal()
                    && matches!(t.task_type, TaskType::Wait | TaskType::Human)
            })
            .ok_or_else(|| {
                EngineError::NotFound(format!("waiting task '{}'", reference_name))
            })?;
        run.tasks[idx].status = TaskStatus::Completed;
        run.tasks[idx].output = output;
        run.tasks[idx].updated_at = now;
        run.updated_at = now;
        self.store.update_run(&run).await?;
        let def = self.def_for(&run).await?;
        self.drive(&def, workflow_id).await
    }

    /// Pause a running workflow. No-op if not `Running`.
    pub async fn pause(&self, workflow_id: &str) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        if run.status == WorkflowStatus::Running {
            run.status = WorkflowStatus::Paused;
            run.updated_at = now_ms();
            self.store.update_run(&run).await?;
        }
        Ok(())
    }

    /// Resume a paused workflow and drive it forward. No-op if not `Paused`.
    pub async fn resume(&self, workflow_id: &str) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        if run.status == WorkflowStatus::Paused {
            run.status = WorkflowStatus::Running;
            run.updated_at = now_ms();
            self.store.update_run(&run).await?;
            let def = self.def_for(&run).await?;
            self.drive(&def, workflow_id).await?;
        }
        Ok(())
    }

    /// Terminate a workflow with an optional reason. No-op if already terminal.
    pub async fn terminate(&self, workflow_id: &str, reason: Option<String>) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        if !run.status.is_terminal() {
            run.status = WorkflowStatus::Terminated;
            run.reason_for_incompletion = reason;
            run.updated_at = now_ms();
            self.store.update_run(&run).await?;
        }
        Ok(())
    }

    /// Sweep all running workflows, applying task timeouts and re-driving. Returns the number
    /// of runs that changed state. Call this periodically from a background loop to enforce
    /// per-task `timeoutSeconds`.
    pub async fn reap(&self) -> Result<usize> {
        let running = self.store.list_runs(Some(WorkflowStatus::Running)).await?;
        let mut changed = 0;
        for run in running {
            let before = run.status;
            let task_states: Vec<TaskStatus> = run.tasks.iter().map(|t| t.status).collect();
            let def = self.def_for(&run).await?;
            self.drive(&def, &run.workflow_id).await?;
            let after = self.get_run(&run.workflow_id).await?;
            let after_states: Vec<TaskStatus> = after.tasks.iter().map(|t| t.status).collect();
            if before != after.status || task_states != after_states {
                changed += 1;
            }
        }
        Ok(changed)
    }

    async fn def_for(&self, run: &WorkflowRun) -> Result<WorkflowDef> {
        self.store
            .get_workflow_def(&run.workflow_name, Some(run.workflow_version))
            .await?
            .ok_or_else(|| {
                EngineError::NotFound(format!(
                    "workflow def '{}' v{}",
                    run.workflow_name, run.workflow_version
                ))
            })
    }

    /// Drive a run to a fixed point: decide → apply → persist, until nothing progresses.
    async fn drive(&self, def: &WorkflowDef, workflow_id: &str) -> Result<()> {
        let mut run = self.get_run(workflow_id).await?;
        for _ in 0..MAX_ITERS {
            if run.status != WorkflowStatus::Running {
                break;
            }
            let decision = decide(def, &run, now_ms())?;
            let progressed = !decision.schedule.is_empty()
                || !decision.updates.is_empty()
                || decision.terminal.is_some();
            apply(&mut run, decision);
            run.updated_at = now_ms();
            self.store.update_run(&run).await?;
            if run.status != WorkflowStatus::Running || !progressed {
                break;
            }
        }
        Ok(())
    }
}

/// Apply a decision to a run: assign task ids, merge `SET_VARIABLE` output into variables,
/// append tasks, and record any terminal outcome.
fn apply(run: &mut WorkflowRun, decision: Decision) {
    let now = now_ms();
    for update in decision.updates {
        if let Some(task) = run.task_by_id_mut(&update.task_id) {
            task.status = update.status;
            task.reason_for_incompletion = update.reason;
            task.updated_at = now;
        }
    }
    for mut exec in decision.schedule {
        exec.task_id = new_id();
        if exec.task_type == TaskType::SetVariable && exec.status == TaskStatus::Completed {
            if let Value::Object(map) = &exec.output {
                for (k, v) in map {
                    run.variables.insert(k.clone(), v.clone());
                }
            }
        }
        run.tasks.push(exec);
    }
    if let Some(term) = decision.terminal {
        run.status = term.status;
        run.output = term.output;
        run.reason_for_incompletion = term.reason;
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluxo_core::parse_workflow_def;
    use fluxo_store::memory::MemoryStore;
    use serde_json::json;

    #[tokio::test]
    async fn end_to_end_order_flow() {
        let engine = Engine::new(MemoryStore::new());
        let def = parse_workflow_def(
            r#"{
                "name": "order",
                "version": 1,
                "tasks": [
                    { "name": "set_ctx", "taskReferenceName": "ctx", "type": "SET_VARIABLE",
                      "inputParameters": { "region": "${workflow.input.region}" } },
                    { "name": "charge", "taskReferenceName": "charge",
                      "inputParameters": { "amount": "${workflow.input.amount}" } },
                    { "name": "route", "taskReferenceName": "route", "type": "SWITCH",
                      "evaluatorType": "value-param", "expression": "region",
                      "inputParameters": { "region": "${workflow.variables.region}" },
                      "decisionCases": {
                          "eu": [ { "name": "ship_eu", "taskReferenceName": "ship_eu" } ]
                      },
                      "defaultCase": [ { "name": "ship_row", "taskReferenceName": "ship_row" } ]
                    }
                ],
                "outputParameters": { "shipped": "${route.output.selectedCase}" }
            }"#,
        )
        .expect("parse");

        engine.register(&def).await.expect("register");
        let id = engine
            .start("order", None, json!({ "region": "eu", "amount": 42 }), None)
            .await
            .expect("start");

        // First external task is `charge` — a worker claims it by name.
        let polled = engine.poll("charge", "worker-1").await.expect("poll").expect("a task");
        assert_eq!(polled.task.reference_name, "charge");
        assert_eq!(polled.task.input, json!({ "amount": 42 }));
        engine
            .complete_task(&id, &polled.task.task_id, TaskStatus::Completed, json!({ "ok": true }))
            .await
            .expect("complete charge");

        // Switch routed to the "eu" case → ship_eu is now the pollable task.
        let ship = engine.poll("ship_eu", "worker-1").await.expect("poll").expect("ship task");
        assert_eq!(ship.task.reference_name, "ship_eu");
        engine
            .complete_task(&id, &ship.task.task_id, TaskStatus::Completed, json!({}))
            .await
            .expect("complete ship");

        let run = engine.get_run(&id).await.expect("run");
        assert_eq!(run.status, WorkflowStatus::Completed);
        assert_eq!(run.variables.get("region"), Some(&json!("eu")));
        assert_eq!(run.output, json!({ "shipped": "eu" }));
    }

    #[tokio::test]
    async fn human_signal_resumes_run() {
        let engine = Engine::new(MemoryStore::new());
        let def = parse_workflow_def(
            r#"{ "name": "approval", "tasks": [
                { "name": "review", "taskReferenceName": "review", "type": "HUMAN" },
                { "name": "finalize", "taskReferenceName": "finalize" }
            ]}"#,
        )
        .expect("parse");
        engine.register(&def).await.expect("register");
        let id = engine.start("approval", None, json!({}), None).await.expect("start");

        // Blocked on the human task.
        let run = engine.get_run(&id).await.expect("run");
        assert_eq!(run.status, WorkflowStatus::Running);
        assert_eq!(run.task_by_ref("review").unwrap().status, TaskStatus::Scheduled);
        assert!(engine.poll("finalize", "w").await.expect("poll").is_none());

        engine.signal(&id, "review", json!({ "approved": true })).await.expect("signal");

        // Now the follow-up worker task is available.
        let fin = engine.poll("finalize", "w").await.expect("poll").expect("finalize");
        engine
            .complete_task(&id, &fin.task.task_id, TaskStatus::Completed, json!({}))
            .await
            .expect("complete");
        assert_eq!(engine.get_run(&id).await.unwrap().status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn engine_retries_a_failed_task() {
        let engine = Engine::new(MemoryStore::new());
        let def = parse_workflow_def(
            r#"{ "name": "retryable", "tasks": [
                { "name": "work", "taskReferenceName": "w", "retryCount": 1 }
            ]}"#,
        )
        .expect("parse");
        engine.register(&def).await.expect("register");
        let id = engine.start("retryable", None, json!({}), None).await.expect("start");

        // First attempt fails.
        let a1 = engine.poll("work", "w1").await.expect("poll").expect("task");
        assert_eq!(a1.task.retry_count, 0);
        engine
            .complete_task(&id, &a1.task.task_id, TaskStatus::Failed, json!({}))
            .await
            .expect("fail 1");

        // A retry attempt is available (zero backoff) and succeeds.
        let a2 = engine.poll("work", "w1").await.expect("poll").expect("retry task");
        assert_eq!(a2.task.retry_count, 1);
        engine
            .complete_task(&id, &a2.task.task_id, TaskStatus::Completed, json!({ "ok": true }))
            .await
            .expect("complete 2");

        assert_eq!(engine.get_run(&id).await.unwrap().status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn reap_times_out_a_stuck_task() {
        let engine = Engine::new(MemoryStore::new());
        let def = parse_workflow_def(
            r#"{ "name": "stuck", "tasks": [
                { "name": "slow", "taskReferenceName": "s", "timeoutSeconds": 0 }
            ]}"#,
        )
        .expect("parse");
        engine.register(&def).await.expect("register");
        let id = engine.start("stuck", None, json!({}), None).await.expect("start");
        let _ = engine.poll("slow", "w1").await.expect("poll");

        // Let the clock move past the (0s) timeout, then sweep.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let changed = engine.reap().await.expect("reap");
        assert!(changed >= 1);
        assert_eq!(engine.get_run(&id).await.unwrap().status, WorkflowStatus::Failed);
    }
}
