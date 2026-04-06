#![allow(dead_code)]
//! Company portability — export/import blueprints.
//!
//! Export serializes a company snapshot (agents, goals, tasks, documents,
//! routines, budgets, approvals) to a JSON blueprint file. Secrets are
//! scrubbed (key names preserved, values replaced with placeholder).
//! Import re-creates the company from a blueprint with fresh ID remapping.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::company_store::{CompanyStore, Company, CompanyAgent};
use crate::company_goals::Goal;
use crate::company_tasks::CompanyTask;
use crate::company_documents::Document;
use crate::company_routines::Routine;
use crate::company_budget::CompanyBudget;
use crate::company_approvals::Approval;

// ── Blueprint ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyBlueprint {
    pub version: String,
    pub exported_at: u64,
    pub company: Company,
    pub agents: Vec<CompanyAgent>,
    pub goals: Vec<Goal>,
    pub tasks: Vec<CompanyTask>,
    pub documents: Vec<Document>,
    pub routines: Vec<Routine>,
    pub budgets: Vec<CompanyBudget>,
    pub approvals: Vec<Approval>,
    /// Scrubbed secret key names (values replaced with "<redacted>").
    pub secret_keys: Vec<String>,
}

// ── Export ────────────────────────────────────────────────────────────────────

/// Export a company to a blueprint JSON file.
pub fn export_company(company_id: &str, output_path: &Path) -> Result<CompanyBlueprint> {
    let store = CompanyStore::open_default()?;
    let conn = store.conn();

    // Company
    let company = store.get_company(company_id)?
        .ok_or_else(|| anyhow!("company not found: {}", company_id))?;

    // Agents
    let agents = store.list_agents(company_id)?;

    // Goals
    let goals = {
        let gs = crate::company_goals::GoalStore::new(conn);
        gs.ensure_schema()?;
        gs.list(company_id)?
    };

    // Tasks
    let tasks = {
        let ts = crate::company_tasks::TaskStore::new(conn);
        ts.ensure_schema()?;
        ts.list(company_id, None)?
    };

    // Documents
    let documents = {
        let ds = crate::company_documents::DocumentStore::new(conn);
        ds.ensure_schema()?;
        ds.list(company_id)?
    };

    // Routines
    let routines = {
        let rs = crate::company_routines::RoutineStore::new(conn);
        rs.ensure_schema()?;
        rs.list(company_id)?
    };

    // Budgets
    let budgets = {
        let bs = crate::company_budget::BudgetStore::new(conn);
        bs.ensure_schema()?;
        bs.list(company_id)?
    };

    // Approvals
    let approvals = {
        let ap = crate::company_approvals::ApprovalStore::new(conn);
        ap.ensure_schema()?;
        ap.list(company_id, None)?
    };

    // Secrets — keys only, values scrubbed
    let secret_keys = {
        let ss = crate::company_secrets::SecretStore::new(conn);
        ss.ensure_schema()?;
        ss.list(company_id)?.into_iter().map(|s| s.key_name).collect()
    };

    let blueprint = CompanyBlueprint {
        version: "1.0".to_string(),
        exported_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        company,
        agents,
        goals,
        tasks,
        documents,
        routines,
        budgets,
        approvals,
        secret_keys,
    };

    let json = serde_json::to_string_pretty(&blueprint).context("serializing blueprint")?;
    std::fs::write(output_path, json).context("writing blueprint")?;
    Ok(blueprint)
}

// ── Import ────────────────────────────────────────────────────────────────────

/// Import a company from a blueprint JSON file.
/// Returns the new company ID.
pub fn import_company(input_path: &Path, new_name: Option<&str>) -> Result<String> {
    let json = std::fs::read_to_string(input_path).context("reading blueprint")?;
    let bp: CompanyBlueprint = serde_json::from_str(&json).context("parsing blueprint")?;

    let store = CompanyStore::open_default()?;
    let conn = store.conn();

    // Remap: old_id -> new_id
    let mut id_map: HashMap<String, String> = HashMap::new();

    // Create new company
    let company_name = new_name.unwrap_or(&bp.company.name);
    let new_company = store.create_company(
        company_name,
        &bp.company.description,
        &bp.company.mission,
    )?;
    id_map.insert(bp.company.id.clone(), new_company.id.clone());
    let new_cid = new_company.id.clone();

    // Re-hire agents (in order — CEO first to preserve reports_to chain)
    for agent in &bp.agents {
        let new_reports_to = agent.reports_to.as_ref().and_then(|old| id_map.get(old)).cloned();
        let new_agent = store.hire_agent(
            &new_cid,
            &agent.name,
            &agent.title,
            agent.role.clone(),
            new_reports_to.as_deref(),
            &agent.skills,
            agent.adapter_type.clone(),
            agent.monthly_budget_cents,
        )?;
        id_map.insert(agent.id.clone(), new_agent.id.clone());
    }

    // Goals
    {
        let gs = crate::company_goals::GoalStore::new(conn);
        gs.ensure_schema()?;
        for goal in &bp.goals {
            let new_parent = goal.parent_goal_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_owner = goal.owner_agent_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_goal = gs.create(
                &new_cid, &goal.title, &goal.description,
                new_parent.as_deref(), new_owner.as_deref(), goal.priority,
            )?;
            id_map.insert(goal.id.clone(), new_goal.id.clone());
        }
    }

    // Tasks
    {
        let ts = crate::company_tasks::TaskStore::new(conn);
        ts.ensure_schema()?;
        for task in &bp.tasks {
            let new_goal = task.goal_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_parent = task.parent_task_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_agent = task.assigned_agent.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_task = ts.create(
                &new_cid, &task.title, &task.description,
                new_goal.as_deref(), new_parent.as_deref(), new_agent.as_deref(),
                task.priority.clone(),
            )?;
            id_map.insert(task.id.clone(), new_task.id.clone());
        }
    }

    // Documents
    {
        let ds = crate::company_documents::DocumentStore::new(conn);
        ds.ensure_schema()?;
        for doc in &bp.documents {
            let new_task = doc.linked_task_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            let new_goal = doc.linked_goal_id.as_ref().and_then(|old| id_map.get(old)).cloned();
            ds.create(
                &new_cid, &doc.title, &doc.content,
                doc.author_agent_id.as_deref(),
                new_task.as_deref(), new_goal.as_deref(),
            )?;
        }
    }

    // Routines
    {
        let rs = crate::company_routines::RoutineStore::new(conn);
        rs.ensure_schema()?;
        for routine in &bp.routines {
            let new_agent = id_map.get(&routine.agent_id).cloned().unwrap_or_else(|| routine.agent_id.clone());
            rs.create(&new_cid, &new_agent, &routine.name, &routine.prompt, routine.interval_secs)?;
        }
    }

    Ok(new_cid)
}

// ── Summary ───────────────────────────────────────────────────────────────────

impl CompanyBlueprint {
    pub fn summary(&self) -> String {
        format!(
            "Blueprint v{} — '{}'\n  Agents: {}  Goals: {}  Tasks: {}  Docs: {}  Routines: {}  Secrets: {} keys\n  Exported: {}",
            self.version, self.company.name,
            self.agents.len(), self.goals.len(), self.tasks.len(),
            self.documents.len(), self.routines.len(), self.secret_keys.len(),
            self.exported_at / 1000
        )
    }
}
