//! # fluxo-core
//!
//! The pure core of the Fluxo durable workflow engine: the Conductor-compatible DSL
//! ([`model`]), the runtime execution model ([`run`]), `${…}` expression resolution
//! ([`expr`]), DSL parsing/validation ([`dsl`]), and the pure decider ([`decider`]).
//!
//! This crate performs no I/O. Effects (persistence, clocks, worker dispatch) live in
//! `fluxo-store` and `fluxo-engine`.

#![forbid(unsafe_code)]

pub mod decider;
pub mod dsl;
pub mod error;
pub mod expr;
pub mod model;
pub mod run;

pub use decider::{decide, Decision, Terminal};
pub use dsl::{parse_workflow_def, validate};
pub use error::{FluxoError, Result};
pub use model::{SubWorkflowParam, TaskType, WorkflowDef, WorkflowTask};
pub use run::{TaskExecution, TaskStatus, WorkflowRun, WorkflowStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn new_run(def: &WorkflowDef, input: Value) -> WorkflowRun {
        WorkflowRun {
            workflow_id: "wf-1".into(),
            workflow_name: def.name.clone(),
            workflow_version: def.version,
            status: WorkflowStatus::Running,
            input,
            output: Value::Null,
            variables: serde_json::Map::new(),
            tasks: Vec::new(),
            correlation_id: None,
            reason_for_incompletion: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// Apply a decision to a run the way the engine would, assigning ids and merging variables.
    fn apply(run: &mut WorkflowRun, decision: &Decision) {
        for (i, exec) in decision.schedule.iter().enumerate() {
            let mut exec = exec.clone();
            exec.task_id = format!("{}-{}", exec.reference_name, run.tasks.len() + i);
            if exec.task_type == TaskType::SetVariable && exec.status == TaskStatus::Completed {
                if let Value::Object(map) = &exec.output {
                    for (k, v) in map {
                        run.variables.insert(k.clone(), v.clone());
                    }
                }
            }
            run.tasks.push(exec);
        }
        if let Some(term) = &decision.terminal {
            run.status = term.status;
            run.output = term.output.clone();
            run.reason_for_incompletion = term.reason.clone();
        }
    }

    /// Drive the decider to a fixed point, completing external tasks with `complete_ext`.
    fn drive(
        def: &WorkflowDef,
        run: &mut WorkflowRun,
        mut complete_ext: impl FnMut(&TaskExecution) -> Option<(TaskStatus, Value)>,
    ) {
        for _ in 0..100 {
            let decision = decide(def, run, 1).expect("decide");
            let progressed = !decision.schedule.is_empty() || decision.terminal.is_some();
            apply(run, &decision);
            if run.status.is_terminal() {
                return;
            }
            // Resolve any external tasks that are ready.
            let mut changed = false;
            let updates: Vec<(String, TaskStatus, Value)> = run
                .tasks
                .iter()
                .filter(|t| !t.status.is_terminal())
                .filter_map(|t| complete_ext(t).map(|(s, o)| (t.task_id.clone(), s, o)))
                .collect();
            for (id, status, output) in updates {
                if let Some(t) = run.task_by_id_mut(&id) {
                    t.status = status;
                    t.output = output;
                    changed = true;
                }
            }
            if !progressed && !changed {
                return;
            }
        }
        panic!("did not converge");
    }

    #[test]
    fn parses_conductor_json() {
        let def = parse_workflow_def(
            r#"{
                "name": "greet",
                "version": 3,
                "tasks": [
                    { "name": "say_hello", "taskReferenceName": "hello", "type": "SIMPLE" }
                ]
            }"#,
        )
        .expect("parse");
        assert_eq!(def.name, "greet");
        assert_eq!(def.version, 3);
        assert_eq!(def.tasks[0].task_type, TaskType::Simple);
    }

    #[test]
    fn rejects_duplicate_refs() {
        let err = parse_workflow_def(
            r#"{ "name": "dup", "tasks": [
                { "name": "a", "taskReferenceName": "x" },
                { "name": "b", "taskReferenceName": "x" }
            ]}"#,
        );
        assert!(matches!(err, Err(FluxoError::InvalidDefinition(_))));
    }

    #[test]
    fn resolves_expressions() {
        use expr::EvalContext;
        use std::collections::BTreeMap;
        let input = json!({ "user": { "name": "Ada" } });
        let vars = serde_json::Map::new();
        let out = Value::Null;
        let mut outputs = BTreeMap::new();
        outputs.insert("prev".to_string(), json!({ "items": [ {"id": 7} ] }));
        let inputs = BTreeMap::new();
        let ctx = EvalContext {
            workflow_input: &input,
            workflow_variables: &vars,
            workflow_output: &out,
            task_outputs: &outputs,
            task_inputs: &inputs,
        };
        assert_eq!(ctx.lookup("workflow.input.user.name"), Some(json!("Ada")));
        assert_eq!(ctx.lookup("prev.output.items[0].id"), Some(json!(7)));
        assert_eq!(
            ctx.resolve(&json!("hello ${workflow.input.user.name}!")),
            json!("hello Ada!")
        );
        assert_eq!(ctx.resolve(&json!("${prev.output.items[0].id}")), json!(7));
    }

    #[test]
    fn runs_linear_workflow() {
        let def = parse_workflow_def(
            r#"{ "name": "linear", "tasks": [
                { "name": "step_a", "taskReferenceName": "a" },
                { "name": "step_b", "taskReferenceName": "b",
                  "inputParameters": { "from": "${a.output.value}" } }
            ], "outputParameters": { "result": "${b.output.value}" } }"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        drive(&def, &mut run, |t| {
            Some((TaskStatus::Completed, json!({ "value": format!("{}-done", t.reference_name) })))
        });
        assert_eq!(run.status, WorkflowStatus::Completed);
        // b received a's output via expression.
        let b = run.task_by_ref("b").unwrap();
        assert_eq!(b.input, json!({ "from": "a-done" }));
        assert_eq!(run.output, json!({ "result": "b-done" }));
    }

    #[test]
    fn switch_selects_case() {
        let def = parse_workflow_def(
            r#"{ "name": "route", "tasks": [
                { "name": "decide", "taskReferenceName": "sw", "type": "SWITCH",
                  "evaluatorType": "value-param", "expression": "lang",
                  "inputParameters": { "lang": "${workflow.input.lang}" },
                  "decisionCases": {
                      "en": [ { "name": "english", "taskReferenceName": "en_task" } ],
                      "fr": [ { "name": "french",  "taskReferenceName": "fr_task" } ]
                  },
                  "defaultCase": [ { "name": "fallback", "taskReferenceName": "def_task" } ]
                },
                { "name": "finish", "taskReferenceName": "done" }
            ]}"#,
        )
        .expect("parse");

        // Selects the "fr" case.
        let mut run = new_run(&def, json!({ "lang": "fr" }));
        drive(&def, &mut run, |_| Some((TaskStatus::Completed, json!({}))));
        assert_eq!(run.status, WorkflowStatus::Completed);
        assert!(run.task_by_ref("fr_task").is_some());
        assert!(run.task_by_ref("en_task").is_none());
        assert!(run.task_by_ref("done").is_some());

        // Unknown value falls through to the default case.
        let mut run2 = new_run(&def, json!({ "lang": "de" }));
        drive(&def, &mut run2, |_| Some((TaskStatus::Completed, json!({}))));
        assert!(run2.task_by_ref("def_task").is_some());
        assert_eq!(run2.status, WorkflowStatus::Completed);
    }

    #[test]
    fn fork_join_runs_branches() {
        let def = parse_workflow_def(
            r#"{ "name": "parallel", "tasks": [
                { "name": "fork", "taskReferenceName": "fork1", "type": "FORK_JOIN",
                  "forkTasks": [
                      [ { "name": "left",  "taskReferenceName": "l" } ],
                      [ { "name": "right", "taskReferenceName": "r" } ]
                  ] },
                { "name": "join", "taskReferenceName": "join1", "type": "JOIN",
                  "joinOn": [ "l", "r" ] },
                { "name": "after", "taskReferenceName": "after1" }
            ]}"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        drive(&def, &mut run, |t| {
            Some((TaskStatus::Completed, json!({ "who": t.reference_name })))
        });
        assert_eq!(run.status, WorkflowStatus::Completed);
        // Both branches ran, join aggregated, and the follow-up ran.
        let join = run.task_by_ref("join1").unwrap();
        assert_eq!(join.output.get("l").unwrap(), &json!({ "who": "l" }));
        assert_eq!(join.output.get("r").unwrap(), &json!({ "who": "r" }));
        assert!(run.task_by_ref("after1").is_some());
    }

    #[test]
    fn set_variable_feeds_later_tasks() {
        let def = parse_workflow_def(
            r#"{ "name": "vars", "tasks": [
                { "name": "seed", "taskReferenceName": "seed", "type": "SET_VARIABLE",
                  "inputParameters": { "tenant": "acme" } },
                { "name": "use", "taskReferenceName": "use",
                  "inputParameters": { "t": "${workflow.variables.tenant}" } }
            ]}"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        drive(&def, &mut run, |_| Some((TaskStatus::Completed, json!({}))));
        assert_eq!(run.variables.get("tenant"), Some(&json!("acme")));
        assert_eq!(run.task_by_ref("use").unwrap().input, json!({ "t": "acme" }));
    }

    #[test]
    fn non_optional_failure_fails_workflow() {
        let def = parse_workflow_def(
            r#"{ "name": "fails", "tasks": [
                { "name": "boom", "taskReferenceName": "boom" }
            ]}"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        drive(&def, &mut run, |_| Some((TaskStatus::Failed, json!({}))));
        assert_eq!(run.status, WorkflowStatus::Failed);
    }

    #[test]
    fn terminate_ends_workflow_early() {
        let def = parse_workflow_def(
            r#"{ "name": "term", "tasks": [
                { "name": "stop", "taskReferenceName": "stop", "type": "TERMINATE",
                  "inputParameters": { "terminationStatus": "COMPLETED",
                                       "workflowOutput": { "ok": true } } },
                { "name": "never", "taskReferenceName": "never" }
            ]}"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        drive(&def, &mut run, |_| Some((TaskStatus::Completed, json!({}))));
        assert_eq!(run.status, WorkflowStatus::Completed);
        assert_eq!(run.output, json!({ "ok": true }));
        assert!(run.task_by_ref("never").is_none());
    }

    #[test]
    fn waits_for_external_task() {
        let def = parse_workflow_def(
            r#"{ "name": "wait", "tasks": [
                { "name": "worker_task", "taskReferenceName": "w" }
            ]}"#,
        )
        .expect("parse");
        let mut run = new_run(&def, json!({}));
        // Never completes the external task → workflow stays Running.
        drive(&def, &mut run, |_| None);
        assert_eq!(run.status, WorkflowStatus::Running);
        assert_eq!(run.task_by_ref("w").unwrap().status, TaskStatus::Scheduled);
    }
}
