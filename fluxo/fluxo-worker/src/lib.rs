//! # fluxo-worker
//!
//! A poll-by-task-type worker client for a `fluxo-server`. Register a handler per task type;
//! the worker polls `GET /tasks/poll/{type}`, runs the handler, and reports the result via
//! `POST /tasks/{id}/{complete,fail}`. Stateless and horizontally scalable — run as many as
//! you like against one server.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use fluxo_worker::Worker;
//! use serde_json::json;
//!
//! let mut worker = Worker::new("http://127.0.0.1:8080", "worker-1");
//! worker.register("charge", |ctx| async move {
//!     // ctx.input carries the resolved task input
//!     Ok(json!({ "txId": "abc", "charged": ctx.input.get("amount").cloned() }))
//! });
//! worker.run().await?;              // poll forever
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

use fluxo_core::run::TaskExecution;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Errors from the worker client.
#[derive(Debug, Error)]
pub enum WorkerError {
    /// The HTTP transport failed.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// The server returned a non-success status.
    #[error("server returned {0}: {1}")]
    Status(u16, String),
}

/// Convenience alias for worker results.
pub type Result<T> = std::result::Result<T, WorkerError>;

/// The outcome a handler returns: `Ok(output)` completes the task; `Err(reason)` fails it.
pub type HandlerResult = std::result::Result<Value, String>;

/// The context passed to a handler for one claimed task.
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Task-execution id.
    pub task_id: String,
    /// The owning run.
    pub workflow_id: String,
    /// The task's definition reference name.
    pub reference_name: String,
    /// The task's resolved input.
    pub input: Value,
}

type HandlerFuture = Pin<Box<dyn Future<Output = HandlerResult> + Send>>;
type BoxHandler = Arc<dyn Fn(TaskContext) -> HandlerFuture + Send + Sync>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollResponse {
    workflow_id: String,
    task: TaskExecution,
}

/// A worker that polls a `fluxo-server` for tasks and dispatches them to handlers.
pub struct Worker {
    base_url: String,
    worker_id: String,
    client: reqwest::Client,
    handlers: HashMap<String, BoxHandler>,
    poll_interval: Duration,
}

impl Worker {
    /// Create a worker targeting `base_url` (e.g. `http://127.0.0.1:8080`) with identity `worker_id`.
    pub fn new(base_url: impl Into<String>, worker_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            worker_id: worker_id.into(),
            client: reqwest::Client::new(),
            handlers: HashMap::new(),
            poll_interval: Duration::from_millis(500),
        }
    }

    /// Set the idle poll interval (how long to wait after an empty sweep).
    pub fn poll_interval(&mut self, interval: Duration) -> &mut Self {
        self.poll_interval = interval;
        self
    }

    /// Register an async handler for a task type.
    pub fn register<F, Fut>(&mut self, task_type: impl Into<String>, handler: F) -> &mut Self
    where
        F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: BoxHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.handlers.insert(task_type.into(), boxed);
        self
    }

    /// Poll each registered task type once, dispatching any claimed task. Returns the number processed.
    pub async fn poll_once(&self) -> Result<u32> {
        let mut processed = 0;
        for (task_type, handler) in &self.handlers {
            if let Some((workflow_id, task)) = self.poll(task_type).await? {
                let ctx = TaskContext {
                    task_id: task.task_id.clone(),
                    workflow_id: workflow_id.clone(),
                    reference_name: task.reference_name.clone(),
                    input: task.input.clone(),
                };
                match handler(ctx).await {
                    Ok(output) => self.complete(&workflow_id, &task.task_id, output).await?,
                    Err(reason) => self.fail(&workflow_id, &task.task_id, reason).await?,
                }
                processed += 1;
            }
        }
        Ok(processed)
    }

    /// Poll repeatedly until a full sweep claims nothing. Returns the total processed.
    /// Useful for tests and batch drains.
    pub async fn run_until_idle(&self) -> Result<u32> {
        let mut total = 0;
        loop {
            let n = self.poll_once().await?;
            total += n;
            if n == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Poll forever, sleeping `poll_interval` after each empty sweep.
    pub async fn run(&self) -> Result<()> {
        loop {
            if self.poll_once().await? == 0 {
                tokio::time::sleep(self.poll_interval).await;
            }
        }
    }

    async fn poll(&self, task_type: &str) -> Result<Option<(String, TaskExecution)>> {
        let url = format!(
            "{}/tasks/poll/{}?workerId={}",
            self.base_url, task_type, self.worker_id
        );
        let response = self.client.get(&url).send().await?;
        match response.status().as_u16() {
            200 => {
                let parsed: PollResponse = response.json().await?;
                Ok(Some((parsed.workflow_id, parsed.task)))
            }
            204 => Ok(None),
            code => Err(WorkerError::Status(code, response.text().await.unwrap_or_default())),
        }
    }

    async fn complete(&self, workflow_id: &str, task_id: &str, output: Value) -> Result<()> {
        let url = format!("{}/tasks/{}/complete", self.base_url, task_id);
        let response = self
            .client
            .post(&url)
            .json(&json!({ "workflowId": workflow_id, "output": output }))
            .send()
            .await?;
        ensure_success(response).await
    }

    async fn fail(&self, workflow_id: &str, task_id: &str, reason: String) -> Result<()> {
        let url = format!("{}/tasks/{}/fail", self.base_url, task_id);
        let response = self
            .client
            .post(&url)
            .json(&json!({ "workflowId": workflow_id, "output": { "error": reason } }))
            .send()
            .await?;
        ensure_success(response).await
    }
}

async fn ensure_success(response: reqwest::Response) -> Result<()> {
    if response.status().is_success() {
        Ok(())
    } else {
        let code = response.status().as_u16();
        Err(WorkerError::Status(code, response.text().await.unwrap_or_default()))
    }
}
