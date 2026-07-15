//! The workflow *definition* model — the declarative, versioned DSL.
//!
//! Field names and shapes mirror Netflix / Orkes Conductor JSON so that existing
//! Conductor definitions deserialize directly. Unknown task types deserialize to
//! [`TaskType::Other`] and are treated as external worker tasks.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The kind of a workflow task. System task kinds are resolved by the engine; the
/// [`TaskType::Simple`]/[`TaskType::Other`] kinds are dispatched to external workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskType {
    /// A unit of work dispatched to an external worker polling by task name.
    Simple,
    /// A branch on an evaluated case value (Conductor `SWITCH`, formerly `DECISION`).
    Switch,
    /// Fan out into parallel branches.
    ForkJoin,
    /// Fan out into a runtime-computed number of branches (deferred).
    ForkJoinDynamic,
    /// Barrier that completes when its joined branch tasks complete.
    Join,
    /// Loop while a condition holds (deferred).
    DoWhile,
    /// Execute another workflow as a child (external to the decider).
    SubWorkflow,
    /// Start another workflow without waiting (deferred).
    StartWorkflow,
    /// Set workflow-scoped variables from resolved inputs.
    SetVariable,
    /// Produce output inline from resolved inputs.
    Inline,
    /// Transform JSON with a JQ expression (deferred).
    JsonJqTransform,
    /// Pause durably until an external signal completes the task.
    Wait,
    /// Pause durably until a human submits a decision.
    Human,
    /// Make an HTTP call (external to the decider).
    Http,
    /// Emit or await an external event (external to the decider).
    Event,
    /// Terminate the workflow with a chosen status and output.
    Terminate,
    /// Any unrecognized type — treated as an external worker task.
    #[serde(other)]
    Other,
}

impl TaskType {
    /// Whether the decider resolves this task inline (no external worker/signal needed).
    pub fn resolves_inline(self) -> bool {
        matches!(
            self,
            TaskType::Switch
                | TaskType::ForkJoin
                | TaskType::Join
                | TaskType::SetVariable
                | TaskType::Inline
                | TaskType::Terminate
        )
    }

    /// Whether this task waits for an external actor (worker, signal, or human).
    pub fn is_external(self) -> bool {
        matches!(
            self,
            TaskType::Simple
                | TaskType::Other
                | TaskType::Wait
                | TaskType::Human
                | TaskType::SubWorkflow
                | TaskType::StartWorkflow
                | TaskType::Http
                | TaskType::Event
        )
    }
}

/// Parameters for a `SUB_WORKFLOW` task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubWorkflowParam {
    /// Name of the child workflow definition.
    pub name: String,
    /// Optional pinned version; latest when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
}

/// A single node in a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTask {
    /// Task name. For `SIMPLE` tasks this is the type workers poll for.
    pub name: String,
    /// Unique reference for this task instance within the definition tree.
    pub task_reference_name: String,
    /// The task kind. Defaults to [`TaskType::Simple`].
    #[serde(rename = "type", default = "default_task_type")]
    pub task_type: TaskType,
    /// Input parameters, possibly containing `${…}` expressions.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub input_parameters: Map<String, Value>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// When true, a failure of this task does not fail the workflow.
    #[serde(default)]
    pub optional: bool,

    // ---- SWITCH ----
    /// How the switch value is evaluated: `value-param` (default) or `javascript` (deferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluator_type: Option<String>,
    /// For `value-param`, the input-parameter name holding the case key; otherwise an `${…}` expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// Case key → tasks to run for that case.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub decision_cases: BTreeMap<String, Vec<WorkflowTask>>,
    /// Tasks to run when no case matches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_case: Vec<WorkflowTask>,

    // ---- FORK_JOIN ----
    /// Parallel branches, each a sequential task list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fork_tasks: Vec<Vec<WorkflowTask>>,

    // ---- JOIN ----
    /// Reference names this join waits on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub join_on: Vec<String>,

    // ---- DO_WHILE ----
    /// Loop continuation condition (deferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_condition: Option<String>,
    /// Tasks executed each loop iteration (deferred).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loop_over: Vec<WorkflowTask>,

    // ---- SUB_WORKFLOW ----
    /// Child workflow reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_workflow_param: Option<SubWorkflowParam>,

    /// Per-task retry override; falls back to engine defaults when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
}

fn default_task_type() -> TaskType {
    TaskType::Simple
}

/// A versioned, registrable workflow definition — the root of the DSL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDef {
    /// Unique workflow name.
    pub name: String,
    /// Definition version. Multiple versions of a name may coexist.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered top-level tasks.
    pub tasks: Vec<WorkflowTask>,
    /// Declared input-parameter names (documentation / validation).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_parameters: Vec<String>,
    /// Output mapping evaluated on completion; values may contain `${…}` expressions.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub output_parameters: Map<String, Value>,
    /// Overall workflow timeout in seconds (advisory in v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    /// Workflow to invoke on failure (deferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_workflow: Option<String>,
    /// Conductor schema version marker.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_version() -> u32 {
    1
}

fn default_schema_version() -> u32 {
    2
}

impl WorkflowDef {
    /// Iterate every task in the definition tree (top-level plus nested cases/branches).
    pub fn walk(&self) -> Vec<&WorkflowTask> {
        let mut out = Vec::new();
        collect(&self.tasks, &mut out);
        out
    }
}

fn collect<'a>(tasks: &'a [WorkflowTask], out: &mut Vec<&'a WorkflowTask>) {
    for t in tasks {
        out.push(t);
        for sub in t.decision_cases.values() {
            collect(sub, out);
        }
        collect(&t.default_case, out);
        for branch in &t.fork_tasks {
            collect(branch, out);
        }
        collect(&t.loop_over, out);
    }
}
