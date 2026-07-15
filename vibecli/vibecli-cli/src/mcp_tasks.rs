//! MCP Tasks extension + stateless `_meta` (MCP 2026-07-28 RC, gap C3).
//!
//! The 2026-07-28 release candidate reshapes the lifecycle around a **stateless
//! core**: there is no `initialize`/`initialized` handshake and no
//! `Mcp-Session-Id` header pinning a client to one server instance. Instead the
//! protocol version, client info, and capabilities ride a `_meta` field on each
//! request ([`RequestMeta`]). Any instance can serve any request.
//!
//! The **Tasks extension** lets a server answer `tools/call` with a *task
//! handle* instead of an immediate result for long-running work. The client then
//! drives it with `tasks/get`, `tasks/update`, and `tasks/cancel`. Task creation
//! is server-directed: the client advertises the extension via `_meta`, and the
//! server decides when a call runs as a task.
//!
//! This module is the pure protocol layer — the in-memory task registry, the
//! status state machine, and `_meta` parsing — so it is fully unit-testable
//! without a live HTTP transport. The streamable server ([`crate::mcp_streamable`])
//! owns the wire I/O and delegates task bookkeeping here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The extension capability key a client advertises in `_meta` to opt into Tasks.
pub const TASKS_EXTENSION_KEY: &str = "io.modelcontextprotocol/tasks";

/// Per-request `_meta` carried inline on every request in the stateless model.
///
/// Replaces the old `initialize` handshake + `Mcp-Session-Id` header: protocol
/// version, client identity, and advertised capabilities/extensions travel with
/// each request so any stateless instance can serve it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RequestMeta {
    /// Protocol version string, e.g. `"2026-07-28"`.
    #[serde(default, rename = "protocolVersion")]
    pub protocol_version: String,
    /// Free-form client identifier (name/version).
    #[serde(default, rename = "clientInfo")]
    pub client_info: String,
    /// Extension keys the client advertises (e.g. [`TASKS_EXTENSION_KEY`]).
    #[serde(default)]
    pub extensions: Vec<String>,
}

impl RequestMeta {
    /// Parse `_meta` from a JSON-RPC request body (`{"params":{"_meta":{...}}}`
    /// or a top-level `{"_meta":{...}}`). Missing `_meta` yields defaults — the
    /// stateless server treats an absent `_meta` as a legacy/minimal client.
    pub fn from_request_json(body: &serde_json::Value) -> Self {
        let meta = body
            .get("params")
            .and_then(|p| p.get("_meta"))
            .or_else(|| body.get("_meta"));
        match meta {
            Some(m) => serde_json::from_value(m.clone()).unwrap_or_default(),
            None => RequestMeta::default(),
        }
    }

    /// Whether the client advertised the Tasks extension.
    pub fn supports_tasks(&self) -> bool {
        self.extensions.iter().any(|e| e == TASKS_EXTENSION_KEY)
    }
}

/// Lifecycle status of a task (mirrors the RC's task status set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Server is working on it.
    Working,
    /// Server needs more input from the client before continuing.
    InputRequired,
    /// Finished successfully; `result` is populated.
    Completed,
    /// Finished with an error; `error` is populated.
    Failed,
    /// Cancelled by the client via `tasks/cancel`.
    Cancelled,
}

impl TaskState {
    /// Terminal states can't transition further.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }
}

/// A long-running task handle returned from `tools/call`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    /// The tool this task is executing.
    pub tool: String,
    pub state: TaskState,
    /// 0–100 progress hint, when known.
    #[serde(default)]
    pub progress: u8,
    /// Result payload once `Completed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message once `Failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Task {
    fn new(id: String, tool: String) -> Self {
        Self {
            id,
            tool,
            state: TaskState::Working,
            progress: 0,
            result: None,
            error: None,
        }
    }
}

/// In-memory registry implementing the `tasks/*` verbs. Stateless at the
/// protocol layer (no client sessions); a shared instance can be fronted by a
/// load balancer if backed by a shared store — the verbs are store-agnostic.
#[derive(Debug, Default, Clone)]
pub struct TaskRegistry {
    tasks: HashMap<String, Task>,
    counter: u64,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Server-directed task creation: turn a `tools/call` into a task handle.
    /// Returns the new task (state `Working`).
    pub fn create(&mut self, tool: &str) -> Task {
        self.counter += 1;
        let id = format!("task-{:06}", self.counter);
        let task = Task::new(id.clone(), tool.to_string());
        self.tasks.insert(id, task.clone());
        task
    }

    /// `tasks/get` — fetch current task state.
    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// `tasks/update` — advance progress and optionally transition state.
    /// Rejects updates to a terminal task. A `Completed` transition requires a
    /// result; a `Failed` transition requires an error message.
    pub fn update(
        &mut self,
        id: &str,
        progress: Option<u8>,
        state: Option<TaskState>,
        result: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<&Task, String> {
        let task = self
            .tasks
            .get_mut(id)
            .ok_or_else(|| format!("no task {id}"))?;
        if task.state.is_terminal() {
            return Err(format!("task {id} is already {:?}", task.state));
        }
        if let Some(p) = progress {
            task.progress = p.min(100);
        }
        if let Some(s) = state {
            match s {
                TaskState::Completed if result.is_none() => {
                    return Err("completing a task requires a result".to_string());
                }
                TaskState::Failed if error.is_none() => {
                    return Err("failing a task requires an error message".to_string());
                }
                _ => {}
            }
            task.state = s;
            if s == TaskState::Completed {
                task.progress = 100;
            }
        }
        if result.is_some() {
            task.result = result;
        }
        if error.is_some() {
            task.error = error;
        }
        Ok(task)
    }

    /// `tasks/cancel` — cancel a non-terminal task. Idempotent on already-
    /// cancelled tasks; errors if the task finished some other way.
    pub fn cancel(&mut self, id: &str) -> Result<&Task, String> {
        let task = self
            .tasks
            .get_mut(id)
            .ok_or_else(|| format!("no task {id}"))?;
        match task.state {
            TaskState::Cancelled => Ok(task),
            s if s.is_terminal() => Err(format!("task {id} already {s:?}, cannot cancel")),
            _ => {
                task.state = TaskState::Cancelled;
                Ok(task)
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_meta_parses_and_detects_tasks() {
        let body = json!({
            "method": "tools/call",
            "params": {
                "_meta": {
                    "protocolVersion": "2026-07-28",
                    "clientInfo": "vibecli/0.5.8",
                    "extensions": [TASKS_EXTENSION_KEY]
                }
            }
        });
        let meta = RequestMeta::from_request_json(&body);
        assert_eq!(meta.protocol_version, "2026-07-28");
        assert_eq!(meta.client_info, "vibecli/0.5.8");
        assert!(meta.supports_tasks());
    }

    #[test]
    fn request_meta_absent_is_default_no_tasks() {
        let meta = RequestMeta::from_request_json(&json!({"method": "tools/list"}));
        assert_eq!(meta, RequestMeta::default());
        assert!(!meta.supports_tasks());
    }

    #[test]
    fn task_lifecycle_create_update_complete() {
        let mut reg = TaskRegistry::new();
        let t = reg.create("build_project");
        assert_eq!(t.state, TaskState::Working);
        let id = t.id.clone();

        reg.update(&id, Some(50), None, None, None).unwrap();
        assert_eq!(reg.get(&id).unwrap().progress, 50);

        let done = reg
            .update(
                &id,
                None,
                Some(TaskState::Completed),
                Some(json!({"ok": true})),
                None,
            )
            .unwrap();
        assert_eq!(done.state, TaskState::Completed);
        assert_eq!(done.progress, 100);
        assert_eq!(done.result, Some(json!({"ok": true})));
    }

    #[test]
    fn complete_without_result_is_rejected() {
        let mut reg = TaskRegistry::new();
        let id = reg.create("x").id;
        let err = reg
            .update(&id, None, Some(TaskState::Completed), None, None)
            .unwrap_err();
        assert!(err.contains("requires a result"));
    }

    #[test]
    fn cannot_update_terminal_task() {
        let mut reg = TaskRegistry::new();
        let id = reg.create("x").id;
        reg.cancel(&id).unwrap();
        assert_eq!(reg.get(&id).unwrap().state, TaskState::Cancelled);
        // Cancel is idempotent.
        assert!(reg.cancel(&id).is_ok());
        // But further updates are rejected.
        assert!(reg.update(&id, Some(10), None, None, None).is_err());
    }

    #[test]
    fn get_and_cancel_unknown_task_errors() {
        let mut reg = TaskRegistry::new();
        assert!(reg.get("nope").is_none());
        assert!(reg.cancel("nope").is_err());
    }
}
