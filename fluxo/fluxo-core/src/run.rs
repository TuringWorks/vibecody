//! The workflow *execution* model — runtime state persisted by the store.

use crate::model::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Lifecycle status of a single task execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    /// Created and queued, awaiting a worker or inline resolution.
    Scheduled,
    /// Picked up by a worker (or otherwise started).
    InProgress,
    /// Finished successfully.
    Completed,
    /// Finished with a recoverable failure.
    Failed,
    /// Finished with a terminal failure (no retry).
    FailedWithTerminalError,
    /// Exceeded its timeout.
    TimedOut,
    /// Deliberately skipped (e.g. an unchosen switch branch).
    Skipped,
    /// Canceled as part of workflow termination.
    Canceled,
}

impl TaskStatus {
    /// Whether no further transitions are expected.
    pub fn is_terminal(self) -> bool {
        !matches!(self, TaskStatus::Scheduled | TaskStatus::InProgress)
    }

    /// Whether the task finished in a way that lets successors proceed.
    pub fn is_success(self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Skipped)
    }
}

/// Lifecycle status of a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowStatus {
    /// Actively progressing.
    Running,
    /// All tasks completed successfully.
    Completed,
    /// A non-optional task failed.
    Failed,
    /// Exceeded its timeout.
    TimedOut,
    /// Terminated by a `TERMINATE` task or an operator.
    Terminated,
    /// Paused by an operator.
    Paused,
}

impl WorkflowStatus {
    /// Whether the run has reached a final state.
    pub fn is_terminal(self) -> bool {
        !matches!(self, WorkflowStatus::Running | WorkflowStatus::Paused)
    }
}

/// One executed (or scheduled) task instance within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskExecution {
    /// Unique task-execution id (assigned by the engine).
    pub task_id: String,
    /// The definition reference name; unique within a non-looping run.
    pub reference_name: String,
    /// The task kind.
    pub task_type: TaskType,
    /// The definition `name` (for `SIMPLE`, the type workers poll for).
    pub task_name: String,
    /// Current status.
    pub status: TaskStatus,
    /// Resolved input.
    pub input: Value,
    /// Output produced on completion.
    pub output: Value,
    /// Retry attempts made so far.
    #[serde(default)]
    pub retry_count: u32,
    /// Epoch millis when scheduled.
    pub scheduled_at: i64,
    /// Epoch millis of the last update.
    pub updated_at: i64,
    /// Worker that owns/completed the task, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    /// Reason recorded when a task fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_for_incompletion: Option<String>,
}

impl TaskExecution {
    /// Construct a scheduled task execution with empty output.
    pub fn scheduled(
        reference_name: impl Into<String>,
        task_name: impl Into<String>,
        task_type: TaskType,
        input: Value,
        now_ms: i64,
    ) -> Self {
        TaskExecution {
            task_id: String::new(),
            reference_name: reference_name.into(),
            task_type,
            task_name: task_name.into(),
            status: TaskStatus::Scheduled,
            input,
            output: Value::Null,
            retry_count: 0,
            scheduled_at: now_ms,
            updated_at: now_ms,
            worker_id: None,
            reason_for_incompletion: None,
        }
    }
}

/// A workflow execution and all its task instances — the unit the store persists.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    /// Unique run id.
    pub workflow_id: String,
    /// Definition name.
    pub workflow_name: String,
    /// Definition version.
    pub workflow_version: u32,
    /// Current status.
    pub status: WorkflowStatus,
    /// Workflow input.
    pub input: Value,
    /// Workflow output (populated on completion).
    pub output: Value,
    /// Workflow-scoped variables (set via `SET_VARIABLE`).
    #[serde(default)]
    pub variables: Map<String, Value>,
    /// All task executions, in creation order.
    #[serde(default)]
    pub tasks: Vec<TaskExecution>,
    /// Optional caller correlation id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Reason recorded when the run fails or is terminated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_for_incompletion: Option<String>,
    /// Epoch millis at creation.
    pub created_at: i64,
    /// Epoch millis of the last update.
    pub updated_at: i64,
}

impl WorkflowRun {
    /// Find a task execution by reference name.
    pub fn task_by_ref(&self, reference_name: &str) -> Option<&TaskExecution> {
        self.tasks.iter().find(|t| t.reference_name == reference_name)
    }

    /// Find a task execution by id.
    pub fn task_by_id(&self, task_id: &str) -> Option<&TaskExecution> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    /// Mutable lookup by id.
    pub fn task_by_id_mut(&mut self, task_id: &str) -> Option<&mut TaskExecution> {
        self.tasks.iter_mut().find(|t| t.task_id == task_id)
    }
}
