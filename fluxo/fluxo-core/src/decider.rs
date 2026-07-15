//! The pure decider: given a definition and the current run, decide what happens next.
//!
//! [`decide`] is a pure function — no clock, no I/O, no randomness. The engine supplies
//! `now_ms`, applies the returned [`Decision`], and re-invokes `decide` to a fixed point.
//!
//! Beyond routing (linear / switch / fork-join / set-variable / inline / terminate), the
//! decider enforces **timeouts** and **retries**, and drives **`DO_WHILE` loops** by
//! instancing each iteration's body tasks with a `__{iteration}` suffix.

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

/// A status/output change applied to an existing task execution (timeouts, loop completion).
#[derive(Debug, Clone)]
pub struct TaskUpdate {
    /// The task-execution id to update.
    pub task_id: String,
    /// The new status.
    pub status: TaskStatus,
    /// New output, if any.
    pub output: Option<Value>,
    /// Reason recorded on the task.
    pub reason: Option<String>,
}

/// The result of one decide pass.
#[derive(Debug, Clone, Default)]
pub struct Decision {
    /// New task instances to append to the run.
    pub schedule: Vec<TaskExecution>,
    /// In-place changes for existing tasks (timeouts, loop-task completion).
    pub updates: Vec<TaskUpdate>,
    /// Set when the run reaches a terminal state this pass.
    pub terminal: Option<Terminal>,
}

/// Decide the next step for a running workflow.
pub fn decide(def: &WorkflowDef, run: &WorkflowRun, now_ms: i64) -> Result<Decision> {
    if run.status != WorkflowStatus::Running {
        return Ok(Decision::default());
    }
    let plan = ExecPlan::compile(def)?;

    // Latest attempt per reference (later attempts overwrite earlier retries).
    let mut latest: BTreeMap<&str, &TaskExecution> = BTreeMap::new();
    for t in &run.tasks {
        latest.insert(t.reference_name.as_str(), t);
    }

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

    // --- Phase 1: timeouts. Overdue non-terminal tasks flip to TimedOut; re-drive handles them.
    let mut updates = Vec::new();
    for (reference, t) in &latest {
        if t.status.is_terminal() {
            continue;
        }
        let timeout = plan.resolve_ref(reference).and_then(|task| task.timeout_ms());
        if let Some(timeout) = timeout {
            if now_ms - t.scheduled_at > timeout {
                updates.push(TaskUpdate {
                    task_id: t.task_id.clone(),
                    status: TaskStatus::TimedOut,
                    output: None,
                    reason: Some(format!("task '{}' timed out", reference)),
                });
            }
        }
    }
    if !updates.is_empty() {
        return Ok(Decision { updates, ..Default::default() });
    }

    // --- Phase 2: fail the workflow on an exhausted, non-optional failure.
    for (reference, t) in &latest {
        if plan.is_optional(reference) {
            continue;
        }
        if is_failure(t.status) {
            let max = plan
                .resolve_ref(reference)
                .map(|task| task.retry_policy().max_retries)
                .unwrap_or(0);
            if t.status == TaskStatus::FailedWithTerminalError || t.retry_count >= max {
                let reason = t
                    .reason_for_incompletion
                    .clone()
                    .unwrap_or_else(|| format!("task '{}' failed", reference));
                return Ok(Decision {
                    terminal: Some(Terminal {
                        status: WorkflowStatus::Failed,
                        output: Value::Null,
                        reason: Some(reason),
                    }),
                    ..Default::default()
                });
            }
        }
    }

    // --- Phase 3: reschedule retryable failures (with backoff). Works for loop bodies too.
    let mut retries = Vec::new();
    for (reference, t) in &latest {
        if plan.is_optional(reference) {
            continue;
        }
        if matches!(t.status, TaskStatus::Failed | TaskStatus::TimedOut) {
            let task = plan.resolve_ref(reference).ok_or_else(|| {
                FluxoError::InvalidState(format!("no definition for reference '{}'", reference))
            })?;
            let attempt = t.retry_count + 1;
            let backoff = task.retry_policy().backoff_ms(attempt);
            let mut exec = if task.task_type == TaskType::ForkJoinDynamic {
                // Dynamic branch: name/input are carried on the execution, not the definition.
                TaskExecution::scheduled(
                    reference.to_string(),
                    t.task_name.clone(),
                    t.task_type,
                    t.input.clone(),
                    now_ms + backoff,
                )
            } else {
                let input = Value::Object(ctx.resolve_map(&task.input_parameters));
                TaskExecution::scheduled(
                    reference.to_string(),
                    task.name.clone(),
                    task.task_type,
                    input,
                    now_ms + backoff,
                )
            };
            exec.retry_count = attempt;
            retries.push(exec);
        }
    }
    if !retries.is_empty() {
        return Ok(Decision { schedule: retries, ..Default::default() });
    }

    // --- Phase 4: normal scheduling.
    let scheduled: BTreeSet<&str> = latest.keys().copied().collect();
    let mut to_schedule: Vec<String> = Vec::new();
    if run.tasks.is_empty() {
        to_schedule.extend(plan.entry.iter().cloned());
    } else {
        for (reference, t) in &latest {
            if plan.progresses(t) {
                for succ in plan.successors_of(reference, run) {
                    if !scheduled.contains(succ.as_str()) && !to_schedule.contains(&succ) {
                        to_schedule.push(succ);
                    }
                }
            }
        }
        for (join_ref, deps) in &plan.join_deps {
            if plan.dynamic_joins.contains_key(join_ref)
                || scheduled.contains(join_ref.as_str())
                || to_schedule.contains(join_ref)
            {
                continue;
            }
            let ready = deps
                .iter()
                .all(|d| latest.get(d.as_str()).map(|t| t.status.is_success()).unwrap_or(false));
            if ready {
                to_schedule.push(join_ref.clone());
            }
        }
        // Dynamic JOINs: ready once every runtime branch of their fork has succeeded.
        for (join_ref, fork_ref) in &plan.dynamic_joins {
            if scheduled.contains(join_ref.as_str()) || to_schedule.contains(join_ref) {
                continue;
            }
            if let Some(refs) = plan.forked_refs(fork_ref, run) {
                let ready = refs
                    .iter()
                    .all(|r| latest.get(r.as_str()).map(|t| t.status.is_success()).unwrap_or(false));
                if ready {
                    to_schedule.push(join_ref.clone());
                }
            }
        }
    }

    let mut schedule: Vec<TaskExecution> = Vec::new();
    for reference in &to_schedule {
        let task = plan.resolve_ref(reference).ok_or_else(|| {
            FluxoError::InvalidState(format!("no definition for reference '{}'", reference))
        })?;
        if task.task_type == TaskType::ForkJoinDynamic {
            // Spawn one branch per element of the resolved `forkedTasks` list.
            let input = Value::Object(ctx.resolve_map(&task.input_parameters));
            let specs = input
                .get("forkedTasks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut forked = Vec::new();
            for (k, spec) in specs.iter().enumerate() {
                let name = spec.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                let branch_input = spec
                    .get("input")
                    .or_else(|| spec.get("inputParameters"))
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let branch_ref = instance_ref(reference, k as u32);
                schedule.push(TaskExecution::scheduled(
                    branch_ref.clone(),
                    name,
                    TaskType::Simple,
                    branch_input,
                    now_ms,
                ));
                forked.push(branch_ref);
            }
            let mut fork_exec = TaskExecution::scheduled(
                reference.clone(),
                task.name.clone(),
                TaskType::ForkJoinDynamic,
                input,
                now_ms,
            );
            fork_exec.status = TaskStatus::Completed;
            fork_exec.output = json!({ "forkedTasks": forked });
            schedule.push(fork_exec);
            continue;
        }
        if task.task_type == TaskType::DoWhile {
            // Enter the loop: create the loop task (InProgress) and schedule iteration 1.
            let mut loop_exec = TaskExecution::scheduled(
                reference.clone(),
                task.name.clone(),
                TaskType::DoWhile,
                Value::Null,
                now_ms,
            );
            loop_exec.status = TaskStatus::InProgress;
            loop_exec.output = json!({ "iteration": 1 });
            schedule.push(loop_exec);
            if let Some(loop_def) = plan.loops.get(reference) {
                if let Some(first) = loop_def.body_refs.first() {
                    let inst = instance_ref(first, 1);
                    schedule.push(materialize(&plan, &ctx, run, &inst, now_ms)?);
                }
            }
            continue;
        }
        let exec = materialize(&plan, &ctx, run, reference, now_ms)?;
        if exec.task_type == TaskType::Terminate {
            let terminal = terminate_outcome(&exec.input);
            schedule.push(exec);
            return Ok(Decision { schedule, terminal: Some(terminal), ..Default::default() });
        }
        schedule.push(exec);
    }

    // Advance any active DO_WHILE loops.
    let (loop_refs, loop_updates) = process_loops(&plan, run, &latest)?;
    for inst in &loop_refs {
        schedule.push(materialize(&plan, &ctx, run, inst, now_ms)?);
    }

    // Completion: nothing new to schedule/update and nothing pending → the run is done.
    if schedule.is_empty() && loop_updates.is_empty() {
        let any_pending = latest.values().any(|t| !t.status.is_terminal());
        if !run.tasks.is_empty() && !any_pending {
            let output = Value::Object(ctx.resolve_map(&def.output_parameters));
            return Ok(Decision {
                terminal: Some(Terminal {
                    status: WorkflowStatus::Completed,
                    output,
                    reason: None,
                }),
                ..Default::default()
            });
        }
    }

    Ok(Decision { schedule, updates: loop_updates, terminal: None })
}

fn is_failure(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut
    )
}

fn instance_ref(base: &str, iteration: u32) -> String {
    format!("{}__{}", base, iteration)
}

fn terminate_outcome(input: &Value) -> Terminal {
    let status = match input.get("terminationStatus").and_then(Value::as_str) {
        Some("FAILED") => WorkflowStatus::Failed,
        Some("TERMINATED") => WorkflowStatus::Terminated,
        _ => WorkflowStatus::Completed,
    };
    Terminal {
        status,
        output: input.get("workflowOutput").cloned().unwrap_or(Value::Null),
        reason: input.get("terminationReason").and_then(Value::as_str).map(str::to_string),
    }
}

/// Build one task execution, resolving inline system tasks. `reference` may be an instanced
/// loop-body ref (`base__i`); the base task definition is resolved either way.
fn materialize(
    plan: &ExecPlan,
    ctx: &EvalContext,
    run: &WorkflowRun,
    reference: &str,
    now_ms: i64,
) -> Result<TaskExecution> {
    let task = plan
        .resolve_ref(reference)
        .ok_or_else(|| FluxoError::InvalidState(format!("no definition for reference '{}'", reference)))?;
    let input = Value::Object(ctx.resolve_map(&task.input_parameters));
    let mut exec = TaskExecution::scheduled(
        reference.to_string(),
        task.name.clone(),
        task.task_type,
        input.clone(),
        now_ms,
    );
    match task.task_type {
        TaskType::Switch => {
            exec.output = json!({ "selectedCase": evaluate_switch(task, &input, ctx) });
            exec.status = TaskStatus::Completed;
        }
        TaskType::ForkJoin => {
            exec.status = TaskStatus::Completed;
        }
        TaskType::Join => {
            let deps = match plan.dynamic_joins.get(reference) {
                Some(fork_ref) => plan.forked_refs(fork_ref, run).unwrap_or_default(),
                None => plan.join_deps.get(reference).cloned().unwrap_or_default(),
            };
            let aggregated: Map<String, Value> = deps
                .iter()
                .filter_map(|d| run.task_by_ref(d).map(|t| (d.clone(), t.output.clone())))
                .collect();
            exec.output = Value::Object(aggregated);
            exec.status = TaskStatus::Completed;
        }
        TaskType::SetVariable | TaskType::Inline => {
            exec.output = input;
            exec.status = TaskStatus::Completed;
        }
        TaskType::Terminate => {
            exec.status = TaskStatus::Completed;
        }
        TaskType::ForkJoinDynamic | TaskType::JsonJqTransform | TaskType::StartWorkflow => {
            return Err(FluxoError::UnsupportedTaskType(format!("{:?}", task.task_type)));
        }
        // External (Simple/Other/Wait/Human/SubWorkflow/Http/Event) and DoWhile bodies stay Scheduled.
        _ => {}
    }
    Ok(exec)
}

/// Advance active `DO_WHILE` loops: chain the next body task, start the next iteration, or
/// complete the loop task. Returns instanced refs to schedule and loop-task updates.
fn process_loops(
    plan: &ExecPlan,
    run: &WorkflowRun,
    latest: &BTreeMap<&str, &TaskExecution>,
) -> Result<(Vec<String>, Vec<TaskUpdate>)> {
    let mut refs = Vec::new();
    let mut updates = Vec::new();

    for (loop_ref, loop_def) in &plan.loops {
        let loop_exec = match latest.get(loop_ref.as_str()) {
            Some(e) if e.status == TaskStatus::InProgress => *e,
            _ => continue,
        };
        let body = &loop_def.body_refs;
        if body.is_empty() {
            continue;
        }

        // Current iteration = highest i for which the first body task has been instanced.
        let mut iteration: u32 = 0;
        while latest.contains_key(instance_ref(&body[0], iteration + 1).as_str()) {
            iteration += 1;
        }
        if iteration == 0 {
            continue; // just entered this pass; entry already scheduled iteration 1.
        }

        // How many body tasks of the current iteration have been instanced.
        let mut filled = 0usize;
        for base in body {
            if latest.contains_key(instance_ref(base, iteration).as_str()) {
                filled += 1;
            } else {
                break;
            }
        }
        if filled == 0 {
            continue;
        }

        let last_ref = instance_ref(&body[filled - 1], iteration);
        let last = match latest.get(last_ref.as_str()) {
            Some(t) => *t,
            None => continue,
        };
        if !last.status.is_success() {
            continue; // still running, or a failure handled by the retry/fail phases.
        }

        if filled < body.len() {
            // Chain the next body task in this iteration.
            refs.push(instance_ref(&body[filled], iteration));
        } else if evaluate_condition(&loop_def.condition, iteration as i64, run, body, iteration) {
            // Continue: start the next iteration.
            refs.push(instance_ref(&body[0], iteration + 1));
        } else {
            // Stop: complete the loop task.
            updates.push(TaskUpdate {
                task_id: loop_exec.task_id.clone(),
                status: TaskStatus::Completed,
                output: Some(json!({ "iteration": iteration })),
                reason: None,
            });
        }
    }
    Ok((refs, updates))
}

/// Evaluate a loop condition. Supports `true`/`false`, `iteration <op> N`, and
/// `${ref.path} <op> N` / lone `${ref.path}` where references see the current iteration's
/// body outputs (by base ref) plus `workflow.*`.
fn evaluate_condition(
    condition: &str,
    iteration: i64,
    run: &WorkflowRun,
    body: &[String],
    current: u32,
) -> bool {
    let c = condition.trim();
    if c == "true" || c.is_empty() {
        return true;
    }
    if c == "false" {
        return false;
    }

    // Expose this iteration's body outputs under their base reference names.
    let mut iter_outputs: BTreeMap<String, Value> = BTreeMap::new();
    for base in body {
        if let Some(t) = run.task_by_ref(&instance_ref(base, current)) {
            iter_outputs.insert(base.clone(), t.output.clone());
        }
    }
    let empty = BTreeMap::new();
    let ctx = EvalContext {
        workflow_input: &run.input,
        workflow_variables: &run.variables,
        workflow_output: &run.output,
        task_outputs: &iter_outputs,
        task_inputs: &empty,
    };

    for op in ["<=", ">=", "==", "!=", "<", ">"] {
        if let Some(pos) = c.find(op) {
            let lhs = operand(c[..pos].trim(), iteration, &ctx);
            let rhs = operand(c[pos + op.len()..].trim(), iteration, &ctx);
            return compare(&lhs, op, &rhs);
        }
    }
    matches!(operand(c, iteration, &ctx), Operand::Bool(true))
}

enum Operand {
    Num(f64),
    Bool(bool),
    Null,
}

fn operand(token: &str, iteration: i64, ctx: &EvalContext) -> Operand {
    let t = token.trim();
    if t == "iteration" {
        return Operand::Num(iteration as f64);
    }
    if t == "true" {
        return Operand::Bool(true);
    }
    if t == "false" {
        return Operand::Bool(false);
    }
    if t.starts_with("${") {
        let inner = t.trim_start_matches("${").trim_end_matches('}');
        return match ctx.lookup(inner) {
            Some(Value::Number(n)) => n.as_f64().map(Operand::Num).unwrap_or(Operand::Null),
            Some(Value::Bool(b)) => Operand::Bool(b),
            _ => Operand::Null,
        };
    }
    match t.parse::<f64>() {
        Ok(n) => Operand::Num(n),
        Err(_) => Operand::Null,
    }
}

fn compare(l: &Operand, op: &str, r: &Operand) -> bool {
    match (l, r) {
        (Operand::Num(a), Operand::Num(b)) => match op {
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            ">=" => a >= b,
            "==" => (a - b).abs() < f64::EPSILON,
            "!=" => (a - b).abs() >= f64::EPSILON,
            _ => false,
        },
        (Operand::Bool(a), Operand::Bool(b)) => match op {
            "==" => a == b,
            "!=" => a != b,
            _ => false,
        },
        _ => false,
    }
}

/// Evaluate a switch task to its selected case key.
fn evaluate_switch(task: &WorkflowTask, input: &Value, ctx: &EvalContext) -> String {
    let evaluator = task.evaluator_type.as_deref().unwrap_or("value-param");
    if evaluator == "value-param" {
        let key = task.expression.as_deref().unwrap_or("switchCaseValue");
        return input.get(key).map(value_to_case).unwrap_or_default();
    }
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

/// A loop body template compiled from a `DO_WHILE` task.
struct LoopDef {
    body_refs: Vec<String>,
    condition: String,
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
    loops: BTreeMap<String, LoopDef>,
    /// Dynamic fork ref → the JOIN ref that waits on its runtime branches.
    dynamic_forks: BTreeMap<String, String>,
    /// JOIN ref → the dynamic fork ref whose branches it joins.
    dynamic_joins: BTreeMap<String, String>,
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
            loops: BTreeMap::new(),
            dynamic_forks: BTreeMap::new(),
            dynamic_joins: BTreeMap::new(),
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
                TaskType::DoWhile => {
                    // Loop body tasks are instanced at runtime; register them by base ref so
                    // materialize/retry/timeout can resolve them, but create no static edges.
                    self.successors.insert(t.task_reference_name.clone(), next);
                    let mut body_refs = Vec::new();
                    for b in &t.loop_over {
                        self.task_by_ref.insert(b.task_reference_name.clone(), b);
                        body_refs.push(b.task_reference_name.clone());
                    }
                    self.loops.insert(
                        t.task_reference_name.clone(),
                        LoopDef { body_refs, condition: t.loop_condition.clone().unwrap_or_default() },
                    );
                }
                TaskType::Terminate => {
                    self.successors.insert(t.task_reference_name.clone(), Vec::new());
                }
                TaskType::ForkJoinDynamic => {
                    // Branches are created at runtime from resolved input; the following JOIN
                    // waits on them. No static edges from the fork.
                    if let (Some(join_ref), Some(join_task)) = (next.first(), tasks.get(i + 1)) {
                        if join_task.task_type == TaskType::Join {
                            self.dynamic_forks
                                .insert(t.task_reference_name.clone(), join_ref.clone());
                            self.dynamic_joins
                                .insert(join_ref.clone(), t.task_reference_name.clone());
                        }
                    }
                }
                _ => {
                    self.successors.insert(t.task_reference_name.clone(), next);
                }
            }
        }
        Ok(())
    }

    /// Resolve a reference to its task definition, handling instanced loop refs (`base__i`).
    fn resolve_ref(&self, reference: &str) -> Option<&'a WorkflowTask> {
        if let Some(task) = self.task_by_ref.get(reference).copied() {
            return Some(task);
        }
        let base = reference.rsplit_once("__").map(|(b, _)| b)?;
        self.task_by_ref.get(base).copied()
    }

    fn is_optional(&self, reference: &str) -> bool {
        self.resolve_ref(reference).map(|t| t.optional).unwrap_or(false)
    }

    /// The runtime branch refs spawned by a completed dynamic fork, if it has completed.
    fn forked_refs(&self, fork_ref: &str, run: &WorkflowRun) -> Option<Vec<String>> {
        let exec = run.task_by_ref(fork_ref)?;
        if exec.status != TaskStatus::Completed {
            return None;
        }
        let list = exec.output.get("forkedTasks")?.as_array()?;
        Some(list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
    }

    /// Whether a completed/optional-failed task should unlock its successors.
    fn progresses(&self, t: &TaskExecution) -> bool {
        t.status.is_success() || (is_failure(t.status) && self.is_optional(&t.reference_name))
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
            // Dynamic-fork branches are scheduled at entry; the JOIN is scheduled by readiness.
            TaskType::ForkJoinDynamic => Vec::new(),
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
