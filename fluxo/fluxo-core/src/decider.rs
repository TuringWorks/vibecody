//! The pure decider: given a definition and the current run, decide what happens next.
//!
//! [`decide`] is a pure function — no clock, no I/O, no randomness. The engine supplies
//! `now_ms`, applies the returned [`Decision`], and re-invokes `decide` to a fixed point.

use crate::error::{FluxoError, Result};
use crate::expr::EvalContext;
use crate::model::{TaskType, WorkflowDef, WorkflowTask};
use crate::run::{TaskExecution, TaskStatus, WorkflowRun, WorkflowStatus};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// A terminal workflow outcome.
#[derive(Debug, Clone)]
pub struct Terminal {
    /// Final workflow status.
    pub status: WorkflowStatus,
    /// Final workflow output.
    pub output: Value,
    /// Optional reason (failure/termination).
    pub reason: Option<String>,
}

/// The result of one decide pass.
#[derive(Debug, Clone, Default)]
pub struct Decision {
    /// New task instances to append to the run. Inline system tasks arrive already resolved
    /// (status `Completed`); external tasks arrive `Scheduled`.
    pub schedule: Vec<TaskExecution>,
    /// Set when the run reaches a terminal state this pass.
    pub terminal: Option<Terminal>,
}

/// Decide the next step for a running workflow. Returns tasks to schedule and/or a terminal outcome.
pub fn decide(def: &WorkflowDef, run: &WorkflowRun, now_ms: i64) -> Result<Decision> {
    if run.status != WorkflowStatus::Running {
        return Ok(Decision::default());
    }
    let plan = ExecPlan::compile(def)?;

    let task_outputs: BTreeMap<String, Value> = run
        .tasks
        .iter()
        .map(|t| (t.reference_name.clone(), t.output.clone()))
        .collect();
    let task_inputs: BTreeMap<String, Value> = run
        .tasks
        .iter()
        .map(|t| (t.reference_name.clone(), t.input.clone()))
        .collect();
    let ctx = EvalContext {
        workflow_input: &run.input,
        workflow_variables: &run.variables,
        workflow_output: &run.output,
        task_outputs: &task_outputs,
        task_inputs: &task_inputs,
    };

    // Fail fast on the first non-optional failure.
    if let Some(failed) = run.tasks.iter().find(|t| {
        matches!(
            t.status,
            TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut
        ) && !plan.is_optional(&t.reference_name)
    }) {
        let reason = failed
            .reason_for_incompletion
            .clone()
            .unwrap_or_else(|| format!("task '{}' failed", failed.reference_name));
        return Ok(Decision {
            schedule: Vec::new(),
            terminal: Some(Terminal {
                status: WorkflowStatus::Failed,
                output: Value::Null,
                reason: Some(reason),
            }),
        });
    }

    let scheduled: BTreeSet<&str> = run.tasks.iter().map(|t| t.reference_name.as_str()).collect();

    // Which references become schedulable this pass.
    let mut to_schedule: Vec<String> = Vec::new();
    if run.tasks.is_empty() {
        to_schedule.extend(plan.entry.iter().cloned());
    } else {
        for t in &run.tasks {
            if plan.progresses(t) {
                for succ in plan.successors_of(&t.reference_name, run) {
                    if !scheduled.contains(succ.as_str()) && !to_schedule.contains(&succ) {
                        to_schedule.push(succ);
                    }
                }
            }
        }
        // JOIN barriers are pull-based: schedulable once every joined ref has succeeded.
        for (join_ref, deps) in &plan.join_deps {
            if scheduled.contains(join_ref.as_str()) || to_schedule.contains(join_ref) {
                continue;
            }
            let ready = deps
                .iter()
                .all(|d| run.task_by_ref(d).map(|t| t.status.is_success()).unwrap_or(false));
            if ready {
                to_schedule.push(join_ref.clone());
            }
        }
    }

    // Materialize task executions, resolving inline system tasks immediately.
    let mut schedule: Vec<TaskExecution> = Vec::new();
    for reference in &to_schedule {
        let task = plan.task_by_ref.get(reference.as_str()).ok_or_else(|| {
            FluxoError::InvalidState(format!("no definition for reference '{}'", reference))
        })?;
        let input = Value::Object(ctx.resolve_map(&task.input_parameters));
        let mut exec = TaskExecution::scheduled(
            reference.clone(),
            task.name.clone(),
            task.task_type,
            input.clone(),
            now_ms,
        );

        match task.task_type {
            TaskType::Switch => {
                exec.output = json!({ "selectedCase": evaluate_switch(task, &input, &ctx) });
                exec.status = TaskStatus::Completed;
            }
            TaskType::ForkJoin => {
                exec.status = TaskStatus::Completed;
            }
            TaskType::Join => {
                let deps = plan.join_deps.get(reference).cloned().unwrap_or_default();
                let aggregated: Map<String, Value> = deps
                    .iter()
                    .filter_map(|d| run.task_by_ref(d).map(|t| (d.clone(), t.output.clone())))
                    .collect();
                exec.output = Value::Object(aggregated);
                exec.status = TaskStatus::Completed;
            }
            TaskType::SetVariable | TaskType::Inline => {
                exec.output = input.clone();
                exec.status = TaskStatus::Completed;
            }
            TaskType::Terminate => {
                exec.status = TaskStatus::Completed;
                let status = match input.get("terminationStatus").and_then(Value::as_str) {
                    Some("FAILED") => WorkflowStatus::Failed,
                    Some("TERMINATED") => WorkflowStatus::Terminated,
                    _ => WorkflowStatus::Completed,
                };
                let output = input.get("workflowOutput").cloned().unwrap_or(Value::Null);
                let reason = input
                    .get("terminationReason")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                schedule.push(exec);
                return Ok(Decision {
                    schedule,
                    terminal: Some(Terminal { status, output, reason }),
                });
            }
            TaskType::DoWhile
            | TaskType::ForkJoinDynamic
            | TaskType::JsonJqTransform
            | TaskType::StartWorkflow => {
                return Err(FluxoError::UnsupportedTaskType(format!("{:?}", task.task_type)));
            }
            // External tasks (Simple/Other/Wait/Human/SubWorkflow/Http/Event) stay Scheduled.
            _ => {}
        }
        schedule.push(exec);
    }

    // Completion: nothing new to schedule and nothing pending → the run is done.
    if schedule.is_empty() {
        let any_pending = run.tasks.iter().any(|t| !t.status.is_terminal());
        if !run.tasks.is_empty() && !any_pending {
            let output = Value::Object(ctx.resolve_map(&def.output_parameters));
            return Ok(Decision {
                schedule: Vec::new(),
                terminal: Some(Terminal {
                    status: WorkflowStatus::Completed,
                    output,
                    reason: None,
                }),
            });
        }
    }

    Ok(Decision { schedule, terminal: None })
}

/// Evaluate a switch task to its selected case key.
fn evaluate_switch(task: &WorkflowTask, input: &Value, ctx: &EvalContext) -> String {
    let evaluator = task.evaluator_type.as_deref().unwrap_or("value-param");
    if evaluator == "value-param" {
        let key = task.expression.as_deref().unwrap_or("switchCaseValue");
        return input.get(key).map(value_to_case).unwrap_or_default();
    }
    // Any other evaluator: treat the expression as a reference or literal.
    match &task.expression {
        Some(e) => {
            let inner = e.trim().trim_start_matches("${").trim_end_matches('}');
            ctx.lookup(inner).map(|v| value_to_case(&v)).unwrap_or_default()
        }
        None => String::new(),
    }
}

fn value_to_case(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A compiled execution graph derived from a [`WorkflowDef`].
struct ExecPlan<'a> {
    task_by_ref: BTreeMap<String, &'a WorkflowTask>,
    entry: Vec<String>,
    successors: BTreeMap<String, Vec<String>>,
    switch_cases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    switch_default: BTreeMap<String, Vec<String>>,
    fork_branch_entries: BTreeMap<String, Vec<String>>,
    join_deps: BTreeMap<String, Vec<String>>,
}

impl<'a> ExecPlan<'a> {
    fn compile(def: &'a WorkflowDef) -> Result<Self> {
        let mut plan = ExecPlan {
            task_by_ref: BTreeMap::new(),
            entry: first_refs(&def.tasks, &[]),
            successors: BTreeMap::new(),
            switch_cases: BTreeMap::new(),
            switch_default: BTreeMap::new(),
            fork_branch_entries: BTreeMap::new(),
            join_deps: BTreeMap::new(),
        };
        plan.compile_seq(&def.tasks, &[])?;
        Ok(plan)
    }

    fn compile_seq(&mut self, tasks: &'a [WorkflowTask], cont: &[String]) -> Result<()> {
        for (i, t) in tasks.iter().enumerate() {
            self.task_by_ref.insert(t.task_reference_name.clone(), t);
            let next: Vec<String> = if i + 1 < tasks.len() {
                vec![tasks[i + 1].task_reference_name.clone()]
            } else {
                cont.to_vec()
            };
            match t.task_type {
                TaskType::Switch => {
                    let mut cases = BTreeMap::new();
                    for (case, sub) in &t.decision_cases {
                        cases.insert(case.clone(), first_refs(sub, &next));
                        self.compile_seq(sub, &next)?;
                    }
                    self.switch_cases.insert(t.task_reference_name.clone(), cases);
                    self.switch_default
                        .insert(t.task_reference_name.clone(), first_refs(&t.default_case, &next));
                    self.compile_seq(&t.default_case, &next)?;
                }
                TaskType::ForkJoin => {
                    let mut entries = Vec::new();
                    for branch in &t.fork_tasks {
                        entries.extend(first_refs(branch, &[]));
                        // Branch tasks flow to their own end; the JOIN is scheduled via join_deps.
                        self.compile_seq(branch, &[])?;
                    }
                    self.fork_branch_entries
                        .insert(t.task_reference_name.clone(), entries);
                }
                TaskType::Join => {
                    self.join_deps
                        .insert(t.task_reference_name.clone(), t.join_on.clone());
                    self.successors.insert(t.task_reference_name.clone(), next);
                }
                TaskType::Terminate => {
                    self.successors.insert(t.task_reference_name.clone(), Vec::new());
                }
                TaskType::DoWhile | TaskType::ForkJoinDynamic => {
                    return Err(FluxoError::UnsupportedTaskType(format!("{:?}", t.task_type)));
                }
                _ => {
                    self.successors.insert(t.task_reference_name.clone(), next);
                }
            }
        }
        Ok(())
    }

    fn is_optional(&self, reference: &str) -> bool {
        self.task_by_ref.get(reference).map(|t| t.optional).unwrap_or(false)
    }

    /// Whether a completed/optional-failed task should unlock its successors.
    fn progresses(&self, t: &TaskExecution) -> bool {
        t.status.is_success()
            || (matches!(
                t.status,
                TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut
            ) && self.is_optional(&t.reference_name))
    }

    /// The references that become schedulable after `reference` succeeds.
    fn successors_of(&self, reference: &str, run: &WorkflowRun) -> Vec<String> {
        let task = match self.task_by_ref.get(reference) {
            Some(t) => t,
            None => return Vec::new(),
        };
        match task.task_type {
            TaskType::Switch => {
                let selected = run
                    .task_by_ref(reference)
                    .and_then(|t| t.output.get("selectedCase"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                match selected {
                    Some(case) => self
                        .switch_cases
                        .get(reference)
                        .and_then(|m| m.get(&case))
                        .cloned()
                        .unwrap_or_else(|| {
                            self.switch_default.get(reference).cloned().unwrap_or_default()
                        }),
                    None => self.switch_default.get(reference).cloned().unwrap_or_default(),
                }
            }
            TaskType::ForkJoin => self.fork_branch_entries.get(reference).cloned().unwrap_or_default(),
            _ => self.successors.get(reference).cloned().unwrap_or_default(),
        }
    }
}

/// The entry references of a task list: the first task, or the continuation when empty.
fn first_refs(tasks: &[WorkflowTask], cont: &[String]) -> Vec<String> {
    match tasks.first() {
        Some(t) => vec![t.task_reference_name.clone()],
        None => cont.to_vec(),
    }
}
