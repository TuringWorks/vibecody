#![allow(dead_code)]
//! Top-level company orchestrator — wires all company modules together.
//!
//! Provides a single entry point for:
//! - Initializing all company schemas on first use
//! - Emitting SSE events for company state changes
//! - Running the routine tick loop (called periodically)
//! - Activity log access across modules

use anyhow::Result;

use crate::company_store::CompanyStore;
use crate::company_goals::GoalStore;
use crate::company_tasks::TaskStore;
use crate::company_documents::DocumentStore;
use crate::company_budget::BudgetStore;
use crate::company_approvals::ApprovalStore;
use crate::company_secrets::SecretStore;
use crate::company_routines::RoutineStore;
use crate::company_heartbeat::HeartbeatStore;

/// Initialize all company module schemas on a CompanyStore connection.
pub fn ensure_all_schemas(store: &CompanyStore) -> Result<()> {
    let conn = store.conn();
    GoalStore::new(conn).ensure_schema()?;
    TaskStore::new(conn).ensure_schema()?;
    DocumentStore::new(conn).ensure_schema()?;
    BudgetStore::new(conn).ensure_schema()?;
    ApprovalStore::new(conn).ensure_schema()?;
    SecretStore::new(conn).ensure_schema()?;
    RoutineStore::new(conn).ensure_schema()?;
    HeartbeatStore::new(conn).ensure_schema()?;
    Ok(())
}

/// Company-wide dashboard summary for the active company.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompanyDashboard {
    pub company_name: String,
    pub company_id: String,
    pub agent_count: usize,
    pub task_counts: TaskCounts,
    pub pending_approvals: i64,
    pub active_routines: usize,
    pub running_heartbeats: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskCounts {
    pub backlog: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub in_review: usize,
    pub done: usize,
    pub blocked: usize,
}

pub fn build_dashboard(company_id: &str) -> Result<CompanyDashboard> {
    let store = CompanyStore::open_default()?;
    let company = store.get_company(company_id)?
        .ok_or_else(|| anyhow::anyhow!("company not found"))?;
    let conn = store.conn();

    let agents = store.list_agents(company_id)?;

    let all_tasks = TaskStore::new(conn).list(company_id, None)?;
    let task_counts = TaskCounts {
        backlog: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::Backlog)).count(),
        todo: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::Todo)).count(),
        in_progress: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::InProgress)).count(),
        in_review: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::InReview)).count(),
        done: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::Done)).count(),
        blocked: all_tasks.iter().filter(|t| matches!(t.status, crate::company_tasks::TaskStatus::Blocked)).count(),
    };

    let pending_approvals = ApprovalStore::new(conn).pending_count(company_id)?;

    let routines = RoutineStore::new(conn).list(company_id)?;
    let active_routines = routines.iter().filter(|r| r.active).count();

    let running_heartbeats = agents.iter()
        .map(|a| HeartbeatStore::new(conn).running_count(&a.id).unwrap_or(0))
        .sum::<i64>() as usize;

    Ok(CompanyDashboard {
        company_name: company.name,
        company_id: company_id.to_string(),
        agent_count: agents.len(),
        task_counts,
        pending_approvals,
        active_routines,
        running_heartbeats,
    })
}

/// Tick routine loop — call periodically (e.g., every 60s) to fire due routines.
/// Returns the list of routine IDs that were due (caller is responsible for running them).
pub fn tick_routines(company_id: &str) -> Result<Vec<String>> {
    let store = CompanyStore::open_default()?;
    let conn = store.conn();
    let rs = RoutineStore::new(conn);
    rs.ensure_schema()?;

    let due = rs.due_routines()?;
    let mut fired = Vec::new();
    for routine in due {
        if routine.company_id != company_id { continue; }
        rs.mark_ran(&routine.id)?;
        fired.push(routine.id);
    }
    Ok(fired)
}

/// Render a human-readable dashboard string.
pub fn render_dashboard(d: &CompanyDashboard) -> String {
    format!(
        "Company: {} [{}]\n\
         Agents: {}\n\
         Tasks: backlog={} todo={} in_progress={} in_review={} done={} blocked={}\n\
         Pending approvals: {}\n\
         Active routines: {}\n\
         Running heartbeats: {}",
        d.company_name, &d.company_id[..8.min(d.company_id.len())],
        d.agent_count,
        d.task_counts.backlog, d.task_counts.todo, d.task_counts.in_progress,
        d.task_counts.in_review, d.task_counts.done, d.task_counts.blocked,
        d.pending_approvals,
        d.active_routines,
        d.running_heartbeats,
    )
}
