//! Parsing and structural validation of the Conductor-compatible workflow DSL.

use crate::error::{FluxoError, Result};
use crate::model::{TaskType, WorkflowDef, WorkflowTask};
use std::collections::BTreeSet;

/// Parse a workflow definition from Conductor-compatible JSON and validate it.
pub fn parse_workflow_def(json: &str) -> Result<WorkflowDef> {
    let def: WorkflowDef = serde_json::from_str(json)?;
    validate(&def)?;
    Ok(def)
}

/// Serialize a workflow definition back to canonical JSON.
pub fn to_json(def: &WorkflowDef) -> Result<String> {
    serde_json::to_string_pretty(def).map_err(FluxoError::from)
}

/// Validate structural invariants: non-empty name/tasks, globally-unique reference names,
/// forks have branches, joins target existing references, switch cases are non-empty.
pub fn validate(def: &WorkflowDef) -> Result<()> {
    if def.name.trim().is_empty() {
        return Err(FluxoError::InvalidDefinition("name is empty".into()));
    }
    if def.tasks.is_empty() {
        return Err(FluxoError::InvalidDefinition("workflow has no tasks".into()));
    }

    let mut refs = BTreeSet::new();
    collect_refs(&def.tasks, &mut refs)?;

    validate_tasks(&def.tasks, &refs)
}

fn collect_refs(tasks: &[WorkflowTask], refs: &mut BTreeSet<String>) -> Result<()> {
    for t in tasks {
        if t.task_reference_name.trim().is_empty() {
            return Err(FluxoError::InvalidDefinition(format!(
                "task '{}' has an empty taskReferenceName",
                t.name
            )));
        }
        if !refs.insert(t.task_reference_name.clone()) {
            return Err(FluxoError::InvalidDefinition(format!(
                "duplicate taskReferenceName: {}",
                t.task_reference_name
            )));
        }
        for sub in t.decision_cases.values() {
            collect_refs(sub, refs)?;
        }
        collect_refs(&t.default_case, refs)?;
        for branch in &t.fork_tasks {
            collect_refs(branch, refs)?;
        }
        collect_refs(&t.loop_over, refs)?;
    }
    Ok(())
}

fn validate_tasks(tasks: &[WorkflowTask], all_refs: &BTreeSet<String>) -> Result<()> {
    for t in tasks {
        match t.task_type {
            TaskType::Switch => {
                if t.decision_cases.is_empty() {
                    return Err(FluxoError::InvalidDefinition(format!(
                        "switch '{}' has no decisionCases",
                        t.task_reference_name
                    )));
                }
                for sub in t.decision_cases.values() {
                    validate_tasks(sub, all_refs)?;
                }
                validate_tasks(&t.default_case, all_refs)?;
            }
            TaskType::ForkJoin => {
                if t.fork_tasks.is_empty() {
                    return Err(FluxoError::InvalidDefinition(format!(
                        "fork '{}' has no forkTasks",
                        t.task_reference_name
                    )));
                }
                for branch in &t.fork_tasks {
                    validate_tasks(branch, all_refs)?;
                }
            }
            TaskType::Join => {
                for dep in &t.join_on {
                    if !all_refs.contains(dep) {
                        return Err(FluxoError::InvalidDefinition(format!(
                            "join '{}' waits on unknown reference '{}'",
                            t.task_reference_name, dep
                        )));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
