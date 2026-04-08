#![allow(dead_code)]
//! SQLite-backed company store for VibeCody's zero-human company orchestration.
//!
//! Stores companies and their agents in a SQLite database at
//! `~/.vibecli/company.db`. Follows the same pattern as `session_store.rs`.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompanyStatus {
    Active,
    Paused,
    Archived,
}

impl CompanyStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "paused" => Self::Paused,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompanyRole {
    Ceo,
    Manager,
    Agent,
    Specialist(String),
}

impl CompanyRole {
    pub fn as_str(&self) -> String {
        match self {
            Self::Ceo => "ceo".to_string(),
            Self::Manager => "manager".to_string(),
            Self::Agent => "agent".to_string(),
            Self::Specialist(s) => format!("specialist:{s}"),
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "ceo" => Self::Ceo,
            "manager" => Self::Manager,
            "agent" => Self::Agent,
            s if s.starts_with("specialist:") => {
                Self::Specialist(s.trim_start_matches("specialist:").to_string())
            }
            other => Self::Specialist(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Active,
    Paused,
    Terminated,
}

impl AgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Terminated => "terminated",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "terminated" => Self::Terminated,
            _ => Self::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    Internal,
    Claude,
    Codex,
    Cursor,
    Http,
    Process,
}

impl AdapterType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Internal => "internal",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Http => "http",
            Self::Process => "process",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "claude" => Self::Claude,
            "codex" => Self::Codex,
            "cursor" => Self::Cursor,
            "http" => Self::Http,
            "process" => Self::Process,
            _ => Self::Internal,
        }
    }
}

// ── Data Structs ──────────────────────────────────────────────────────────────

/// A company — the root entity for all orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mission: String,
    pub status: CompanyStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub settings: serde_json::Value,
}

/// An agent belonging to a company, with org-chart position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyAgent {
    pub id: String,
    pub company_id: String,
    pub name: String,
    pub title: String,
    pub role: CompanyRole,
    /// Parent agent ID (None = root / CEO).
    pub reports_to: Option<String>,
    pub status: AgentStatus,
    pub skills: Vec<String>,
    pub adapter_type: AdapterType,
    pub adapter_config: serde_json::Value,
    pub monthly_budget_cents: i64,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Activity log entry for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: i64,
    pub company_id: String,
    pub actor_agent_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: serde_json::Value,
    pub created_at: u64,
}

// ── CompanyStore ──────────────────────────────────────────────────────────────

pub struct CompanyStore {
    conn: Connection,
}

impl CompanyStore {
    /// Open (or create) the company database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {parent:?}"))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open SQLite at {path:?}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let store = Self { conn };
        store.create_schema()?;
        store.run_migrations()?;
        Ok(store)
    }

    /// Open from the default path: `~/.vibecli/company.db`.
    pub fn open_default() -> Result<Self> {
        Self::open(default_db_path())
    }

    /// Borrow the underlying SQLite connection (used by sub-stores that share the same DB).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS companies (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL UNIQUE,
                description     TEXT NOT NULL DEFAULT '',
                mission         TEXT NOT NULL DEFAULT '',
                status          TEXT NOT NULL DEFAULT 'active',
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                settings_json   TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS company_agents (
                id                    TEXT PRIMARY KEY,
                company_id            TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
                name                  TEXT NOT NULL,
                title                 TEXT NOT NULL DEFAULT '',
                role                  TEXT NOT NULL DEFAULT 'agent',
                reports_to            TEXT REFERENCES company_agents(id) ON DELETE SET NULL,
                status                TEXT NOT NULL DEFAULT 'idle',
                skills_json           TEXT NOT NULL DEFAULT '[]',
                adapter_type          TEXT NOT NULL DEFAULT 'internal',
                adapter_config_json   TEXT NOT NULL DEFAULT '{}',
                monthly_budget_cents  INTEGER NOT NULL DEFAULT 0,
                created_at            INTEGER NOT NULL,
                updated_at            INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agents_company ON company_agents(company_id);
            CREATE INDEX IF NOT EXISTS idx_agents_reports_to ON company_agents(reports_to);

            CREATE TABLE IF NOT EXISTS activity_log (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id      TEXT NOT NULL,
                actor_agent_id  TEXT,
                action          TEXT NOT NULL,
                entity_type     TEXT NOT NULL,
                entity_id       TEXT NOT NULL,
                details_json    TEXT NOT NULL DEFAULT '{}',
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_activity_company ON activity_log(company_id);
            CREATE INDEX IF NOT EXISTS idx_activity_entity ON activity_log(entity_type, entity_id);
        "#)?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<()> {
        // Idempotent column additions for future schema evolution
        // (modelled after session_store.rs::maybe_add_column)
        Ok(())
    }

    // ── Company CRUD ──────────────────────────────────────────────────────────

    pub fn create_company(
        &self,
        name: &str,
        description: &str,
        mission: &str,
    ) -> Result<Company> {
        let company = Company {
            id: new_id(),
            name: name.to_string(),
            description: description.to_string(),
            mission: mission.to_string(),
            status: CompanyStatus::Active,
            created_at: now_ms(),
            updated_at: now_ms(),
            settings: serde_json::json!({}),
        };
        self.conn.execute(
            "INSERT INTO companies (id, name, description, mission, status, created_at, updated_at, settings_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                company.id,
                company.name,
                company.description,
                company.mission,
                company.status.as_str(),
                company.created_at as i64,
                company.updated_at as i64,
                company.settings.to_string(),
            ],
        )?;
        self.log_activity(
            &company.id, None, "company.created", "company", &company.id,
            serde_json::json!({"name": company.name}),
        )?;
        Ok(company)
    }

    pub fn list_companies(&self) -> Result<Vec<Company>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, mission, status, created_at, updated_at, settings_json
             FROM companies ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut companies = Vec::new();
        for row in rows {
            let (id, name, description, mission, status, created_at, updated_at, settings_str) = row?;
            companies.push(Company {
                id,
                name,
                description,
                mission,
                status: CompanyStatus::from_str(&status),
                created_at: created_at as u64,
                updated_at: updated_at as u64,
                settings: serde_json::from_str(&settings_str).unwrap_or(serde_json::json!({})),
            });
        }
        Ok(companies)
    }

    pub fn get_company(&self, id: &str) -> Result<Option<Company>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, mission, status, created_at, updated_at, settings_json
             FROM companies WHERE id = ?1 OR name = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        if let Some(row) = rows.next() {
            let (id, name, description, mission, status, created_at, updated_at, settings_str) = row?;
            return Ok(Some(Company {
                id,
                name,
                description,
                mission,
                status: CompanyStatus::from_str(&status),
                created_at: created_at as u64,
                updated_at: updated_at as u64,
                settings: serde_json::from_str(&settings_str).unwrap_or(serde_json::json!({})),
            }));
        }
        Ok(None)
    }

    pub fn update_company(
        &self,
        id: &str,
        description: Option<&str>,
        mission: Option<&str>,
        status: Option<CompanyStatus>,
    ) -> Result<Company> {
        let updated_at = now_ms() as i64;
        if let Some(desc) = description {
            self.conn.execute(
                "UPDATE companies SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![desc, updated_at, id],
            )?;
        }
        if let Some(m) = mission {
            self.conn.execute(
                "UPDATE companies SET mission = ?1, updated_at = ?2 WHERE id = ?3",
                params![m, updated_at, id],
            )?;
        }
        if let Some(s) = status {
            self.conn.execute(
                "UPDATE companies SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![s.as_str(), updated_at, id],
            )?;
        }
        self.get_company(id)?.context("company not found after update")
    }

    pub fn delete_company(&self, id: &str) -> Result<()> {
        // Soft delete — set status to archived
        self.conn.execute(
            "UPDATE companies SET status = 'archived', updated_at = ?1 WHERE id = ?2",
            params![now_ms() as i64, id],
        )?;
        Ok(())
    }

    // ── Agent CRUD ────────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn hire_agent(
        &self,
        company_id: &str,
        name: &str,
        title: &str,
        role: CompanyRole,
        reports_to: Option<&str>,
        skills: &[String],
        adapter_type: AdapterType,
        monthly_budget_cents: i64,
    ) -> Result<CompanyAgent> {
        // Validate company exists
        self.get_company(company_id)?.context("company not found")?;
        let agent = CompanyAgent {
            id: new_id(),
            company_id: company_id.to_string(),
            name: name.to_string(),
            title: title.to_string(),
            role,
            reports_to: reports_to.map(|s| s.to_string()),
            status: AgentStatus::Idle,
            skills: skills.to_vec(),
            adapter_type,
            adapter_config: serde_json::json!({}),
            monthly_budget_cents,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.conn.execute(
            "INSERT INTO company_agents
             (id, company_id, name, title, role, reports_to, status, skills_json,
              adapter_type, adapter_config_json, monthly_budget_cents, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                agent.id,
                agent.company_id,
                agent.name,
                agent.title,
                agent.role.as_str(),
                agent.reports_to,
                agent.status.as_str(),
                serde_json::to_string(&agent.skills)?,
                agent.adapter_type.as_str(),
                agent.adapter_config.to_string(),
                agent.monthly_budget_cents,
                agent.created_at as i64,
                agent.updated_at as i64,
            ],
        )?;
        self.log_activity(
            company_id, None, "agent.hired", "company_agent", &agent.id,
            serde_json::json!({"name": agent.name, "title": agent.title}),
        )?;
        Ok(agent)
    }

    pub fn list_agents(&self, company_id: &str) -> Result<Vec<CompanyAgent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, name, title, role, reports_to, status, skills_json,
                    adapter_type, adapter_config_json, monthly_budget_cents, created_at, updated_at
             FROM company_agents WHERE company_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, i64>(12)?,
            ))
        })?;
        let mut agents = Vec::new();
        for row in rows {
            let (id, company_id, name, title, role, reports_to, status,
                 skills_str, adapter_type, adapter_config_str, budget, created_at, updated_at) = row?;
            agents.push(CompanyAgent {
                id,
                company_id,
                name,
                title,
                role: CompanyRole::from_str(&role),
                reports_to,
                status: AgentStatus::from_str(&status),
                skills: serde_json::from_str(&skills_str).unwrap_or_default(),
                adapter_type: AdapterType::from_str(&adapter_type),
                adapter_config: serde_json::from_str(&adapter_config_str).unwrap_or(serde_json::json!({})),
                monthly_budget_cents: budget,
                created_at: created_at as u64,
                updated_at: updated_at as u64,
            });
        }
        Ok(agents)
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<CompanyAgent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, name, title, role, reports_to, status, skills_json,
                    adapter_type, adapter_config_json, monthly_budget_cents, created_at, updated_at
             FROM company_agents WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, i64>(12)?,
            ))
        })?;
        if let Some(row) = rows.next() {
            let (id, company_id, name, title, role, reports_to, status,
                 skills_str, adapter_type, adapter_config_str, budget, created_at, updated_at) = row?;
            return Ok(Some(CompanyAgent {
                id,
                company_id,
                name,
                title,
                role: CompanyRole::from_str(&role),
                reports_to,
                status: AgentStatus::from_str(&status),
                skills: serde_json::from_str(&skills_str).unwrap_or_default(),
                adapter_type: AdapterType::from_str(&adapter_type),
                adapter_config: serde_json::from_str(&adapter_config_str).unwrap_or(serde_json::json!({})),
                monthly_budget_cents: budget,
                created_at: created_at as u64,
                updated_at: updated_at as u64,
            }));
        }
        Ok(None)
    }

    pub fn fire_agent(&self, id: &str) -> Result<()> {
        let agent = self.get_agent(id)?.context("agent not found")?;
        self.conn.execute(
            "UPDATE company_agents SET status = 'terminated', updated_at = ?1 WHERE id = ?2",
            params![now_ms() as i64, id],
        )?;
        self.log_activity(
            &agent.company_id, None, "agent.fired", "company_agent", id,
            serde_json::json!({"name": agent.name}),
        )?;
        Ok(())
    }

    pub fn update_agent_status(&self, id: &str, status: AgentStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE company_agents SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now_ms() as i64, id],
        )?;
        Ok(())
    }

    // ── Activity Log ──────────────────────────────────────────────────────────

    pub fn log_activity(
        &self,
        company_id: &str,
        actor_agent_id: Option<&str>,
        action: &str,
        entity_type: &str,
        entity_id: &str,
        details: serde_json::Value,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO activity_log (company_id, actor_agent_id, action, entity_type, entity_id, details_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                company_id,
                actor_agent_id,
                action,
                entity_type,
                entity_id,
                details.to_string(),
                now_ms() as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_activity(&self, company_id: &str, limit: usize) -> Result<Vec<ActivityEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, actor_agent_id, action, entity_type, entity_id, details_json, created_at
             FROM activity_log WHERE company_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![company_id, limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?;
        let mut entries = Vec::new();
        for row in rows {
            let (id, company_id, actor, action, entity_type, entity_id, details_str, created_at) = row?;
            entries.push(ActivityEntry {
                id,
                company_id,
                actor_agent_id: actor,
                action,
                entity_type,
                entity_id,
                details: serde_json::from_str(&details_str).unwrap_or(serde_json::json!({})),
                created_at: created_at as u64,
            });
        }
        Ok(entries)
    }

    // ── Org Tree Queries ─────────────────────────────────────────────────────

    /// Get all direct reports of an agent.
    pub fn get_direct_reports(&self, agent_id: &str) -> Result<Vec<CompanyAgent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, name, title, role, reports_to, status, skills_json,
                    adapter_type, adapter_config_json, monthly_budget_cents, created_at, updated_at
             FROM company_agents WHERE reports_to = ?1 AND status != 'terminated'",
        )?;
        let rows = stmt.query_map(params![agent_id], |row| {
            Ok(self.row_to_agent(row))
        })?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row??);
        }
        Ok(agents)
    }

    /// Get the full subtree of an agent (including the agent itself) using a recursive CTE.
    pub fn get_agent_subtree(&self, root_id: &str) -> Result<Vec<CompanyAgent>> {
        let mut stmt = self.conn.prepare(
            r#"WITH RECURSIVE tree(id) AS (
                SELECT id FROM company_agents WHERE id = ?1
                UNION ALL
                SELECT a.id FROM company_agents a INNER JOIN tree t ON a.reports_to = t.id
               )
               SELECT ca.id, ca.company_id, ca.name, ca.title, ca.role, ca.reports_to,
                      ca.status, ca.skills_json, ca.adapter_type, ca.adapter_config_json,
                      ca.monthly_budget_cents, ca.created_at, ca.updated_at
               FROM company_agents ca INNER JOIN tree t ON ca.id = t.id
               WHERE ca.status != 'terminated'"#,
        )?;
        let rows = stmt.query_map(params![root_id], |row| {
            Ok(self.row_to_agent(row))
        })?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row??);
        }
        Ok(agents)
    }

    /// Get the CEO (root agent with no reports_to) for a company.
    pub fn get_ceo(&self, company_id: &str) -> Result<Option<CompanyAgent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, name, title, role, reports_to, status, skills_json,
                    adapter_type, adapter_config_json, monthly_budget_cents, created_at, updated_at
             FROM company_agents
             WHERE company_id = ?1 AND reports_to IS NULL AND status != 'terminated'
             ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![company_id], |row| {
            Ok(self.row_to_agent(row))
        })?;
        if let Some(row) = rows.next() {
            return Ok(Some(row??));
        }
        Ok(None)
    }

    /// Build a flat list of (agent, depth) pairs for ASCII org-chart display.
    pub fn build_org_chart(&self, company_id: &str) -> Result<Vec<(CompanyAgent, usize)>> {
        let agents = self.list_agents(company_id)?;
        let mut by_id: std::collections::HashMap<String, CompanyAgent> = agents
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();
        let mut result: Vec<(CompanyAgent, usize)> = Vec::new();
        // Find root nodes (no reports_to, or reports_to not in company)
        let ids: Vec<String> = by_id.keys().cloned().collect();
        let roots: Vec<String> = ids.iter()
            .filter(|id| {
                by_id[*id].reports_to.as_ref()
                    .map(|rt| !by_id.contains_key(rt.as_str()))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        let mut stack: Vec<(String, usize)> = roots.into_iter().map(|id| (id, 0)).collect();
        stack.sort_by(|(a, _), (b, _)| a.cmp(b));
        // DFS traversal
        while let Some((id, depth)) = stack.pop() {
            if let Some(agent) = by_id.remove(&id) {
                let mut children: Vec<String> = by_id.values()
                    .filter(|a| a.reports_to.as_deref() == Some(&id))
                    .map(|a| a.id.clone())
                    .collect();
                children.sort();
                result.push((agent, depth));
                for child_id in children.into_iter().rev() {
                    stack.push((child_id, depth + 1));
                }
            }
        }
        Ok(result)
    }

    /// Helper: convert a sqlite Row to a CompanyAgent.
    /// Used internally to avoid code duplication.
    fn row_to_agent(&self, row: &rusqlite::Row) -> std::result::Result<CompanyAgent, rusqlite::Error> {
        Ok(CompanyAgent {
            id: row.get(0)?,
            company_id: row.get(1)?,
            name: row.get(2)?,
            title: row.get(3)?,
            role: CompanyRole::from_str(&row.get::<_, String>(4)?),
            reports_to: row.get(5)?,
            status: AgentStatus::from_str(&row.get::<_, String>(6)?),
            skills: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            adapter_type: AdapterType::from_str(&row.get::<_, String>(8)?),
            adapter_config: serde_json::from_str(&row.get::<_, String>(9)?)
                .unwrap_or(serde_json::json!({})),
            monthly_budget_cents: row.get(10)?,
            created_at: row.get::<_, i64>(11)? as u64,
            updated_at: row.get::<_, i64>(12)? as u64,
        })
    }

    // ── Company Stats ─────────────────────────────────────────────────────────

    pub fn agent_count(&self, company_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM company_agents WHERE company_id = ?1 AND status != 'terminated'",
            params![company_id],
            |r| r.get(0),
        )?;
        Ok(count)
    }
}

// ── Default path ──────────────────────────────────────────────────────────────

pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("company.db")
}

// ── Active company context ────────────────────────────────────────────────────

/// Get the currently active company ID from the context file.
pub fn get_active_company_id() -> Option<String> {
    let path = active_company_path();
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Set the active company by ID.
pub fn set_active_company_id(id: &str) -> Result<()> {
    let path = active_company_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, id)?;
    Ok(())
}

fn active_company_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("active_company")
}

// ── Display helpers ───────────────────────────────────────────────────────────

impl Company {
    /// One-line summary for REPL display.
    pub fn summary_line(&self) -> String {
        let status_badge = match self.status {
            CompanyStatus::Active => "●",
            CompanyStatus::Paused => "⏸",
            CompanyStatus::Archived => "✗",
        };
        format!("{status_badge} {} [{}]  {}", self.name, &self.id[..8], self.description)
    }
}

impl CompanyAgent {
    /// One-line summary for REPL display.
    pub fn summary_line(&self) -> String {
        let status_badge = match self.status {
            AgentStatus::Idle => "○",
            AgentStatus::Active => "●",
            AgentStatus::Paused => "⏸",
            AgentStatus::Terminated => "✗",
        };
        format!(
            "{status_badge} {} ({})  {}  [{}]",
            self.name,
            self.role.as_str(),
            self.title,
            &self.id[..8]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_store() -> CompanyStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        // CompanyStore::open uses a file path, so we build the store manually
        let store = CompanyStore { conn };
        store.create_schema().unwrap();
        store
    }

    // ── Company CRUD ─────────────────────────────────────────────────────────

    #[test]
    fn given_new_company_when_created_then_returned_with_active_status() {
        let store = make_store();
        let c = store.create_company("Acme", "A description", "Build things").unwrap();
        assert_eq!(c.name, "Acme");
        assert_eq!(c.description, "A description");
        assert_eq!(c.mission, "Build things");
        assert_eq!(c.status, CompanyStatus::Active);
    }

    #[test]
    fn given_new_company_when_listed_then_appears_in_list() {
        let store = make_store();
        store.create_company("WidgetCo", "", "").unwrap();
        let list = store.list_companies().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "WidgetCo");
    }

    #[test]
    fn given_company_exists_when_get_by_id_then_returns_it() {
        let store = make_store();
        let c = store.create_company("Globex", "", "").unwrap();
        let found = store.get_company(&c.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, c.id);
    }

    #[test]
    fn given_company_exists_when_get_by_name_then_returns_it() {
        let store = make_store();
        store.create_company("Initech", "", "").unwrap();
        let found = store.get_company("Initech").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Initech");
    }

    #[test]
    fn given_nonexistent_id_when_get_called_then_returns_none() {
        let store = make_store();
        let result = store.get_company("non-existent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn given_company_when_update_mission_then_reflected_on_fetch() {
        let store = make_store();
        let c = store.create_company("Springfield Nuclear", "", "Original mission").unwrap();
        let updated = store.update_company(&c.id, None, Some("New mission"), None).unwrap();
        assert_eq!(updated.mission, "New mission");
    }

    #[test]
    fn given_company_when_update_status_to_paused_then_status_reflects() {
        let store = make_store();
        let c = store.create_company("PauseCo", "", "").unwrap();
        let updated = store.update_company(&c.id, None, None, Some(CompanyStatus::Paused)).unwrap();
        assert_eq!(updated.status, CompanyStatus::Paused);
    }

    #[test]
    fn given_company_when_deleted_then_archived_not_removed() {
        let store = make_store();
        let c = store.create_company("MortCo", "", "").unwrap();
        store.delete_company(&c.id).unwrap();
        let fetched = store.get_company(&c.id).unwrap().unwrap();
        assert_eq!(fetched.status, CompanyStatus::Archived);
        // Still appears in list
        let list = store.list_companies().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn given_duplicate_name_when_create_company_then_error() {
        let store = make_store();
        store.create_company("UniqueInc", "", "").unwrap();
        let result = store.create_company("UniqueInc", "dup", "dup");
        assert!(result.is_err());
    }

    // ── Agent CRUD ───────────────────────────────────────────────────────────

    #[test]
    fn given_company_when_agent_hired_then_appears_in_list() {
        let store = make_store();
        let c = store.create_company("AgentCo", "", "").unwrap();
        let agent = store.hire_agent(
            &c.id, "Alice", "CEO", CompanyRole::Ceo,
            None, &[], AdapterType::Internal, 100_000,
        ).unwrap();
        let agents = store.list_agents(&c.id).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, agent.id);
    }

    #[test]
    fn given_nonexistent_company_when_hire_agent_then_error() {
        let store = make_store();
        let result = store.hire_agent(
            "no-such-company", "Bob", "Manager", CompanyRole::Manager,
            None, &[], AdapterType::Internal, 0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn given_agent_when_fire_agent_then_status_is_terminated() {
        let store = make_store();
        let c = store.create_company("FireCo", "", "").unwrap();
        let agent = store.hire_agent(
            &c.id, "Charlie", "Intern", CompanyRole::Agent,
            None, &[], AdapterType::Internal, 0,
        ).unwrap();
        store.fire_agent(&agent.id).unwrap();
        let fetched = store.get_agent(&agent.id).unwrap().unwrap();
        assert_eq!(fetched.status, AgentStatus::Terminated);
    }

    #[test]
    fn given_agent_when_update_status_to_active_then_status_reflects() {
        let store = make_store();
        let c = store.create_company("StatusCo", "", "").unwrap();
        let agent = store.hire_agent(
            &c.id, "Dana", "Analyst", CompanyRole::Agent,
            None, &[], AdapterType::Internal, 0,
        ).unwrap();
        assert_eq!(agent.status, AgentStatus::Idle);
        store.update_agent_status(&agent.id, AgentStatus::Active).unwrap();
        let fetched = store.get_agent(&agent.id).unwrap().unwrap();
        assert_eq!(fetched.status, AgentStatus::Active);
    }

    #[test]
    fn given_agent_nonexistent_when_get_then_returns_none() {
        let store = make_store();
        let result = store.get_agent("ghost-agent-id").unwrap();
        assert!(result.is_none());
    }

    // ── Org chart ────────────────────────────────────────────────────────────

    #[test]
    fn given_ceo_and_report_when_get_direct_reports_then_returns_report() {
        let store = make_store();
        let c = store.create_company("OrgCo", "", "").unwrap();
        let ceo = store.hire_agent(
            &c.id, "Eve", "CEO", CompanyRole::Ceo,
            None, &[], AdapterType::Internal, 0,
        ).unwrap();
        let report = store.hire_agent(
            &c.id, "Frank", "Manager", CompanyRole::Manager,
            Some(&ceo.id), &[], AdapterType::Internal, 0,
        ).unwrap();
        let reports = store.get_direct_reports(&ceo.id).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].id, report.id);
    }

    #[test]
    fn given_ceo_when_get_ceo_then_returns_root_agent() {
        let store = make_store();
        let c = store.create_company("TopCo", "", "").unwrap();
        let ceo = store.hire_agent(
            &c.id, "Grace", "CEO", CompanyRole::Ceo,
            None, &[], AdapterType::Internal, 0,
        ).unwrap();
        let found = store.get_ceo(&c.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, ceo.id);
    }

    #[test]
    fn given_subtree_when_get_agent_subtree_then_returns_all_descendants() {
        let store = make_store();
        let c = store.create_company("TreeCo", "", "").unwrap();
        let ceo = store.hire_agent(
            &c.id, "CEO", "CEO", CompanyRole::Ceo,
            None, &[], AdapterType::Internal, 0,
        ).unwrap();
        let mgr = store.hire_agent(
            &c.id, "Mgr", "Mgr", CompanyRole::Manager,
            Some(&ceo.id), &[], AdapterType::Internal, 0,
        ).unwrap();
        let _ = store.hire_agent(
            &c.id, "Dev", "Dev", CompanyRole::Agent,
            Some(&mgr.id), &[], AdapterType::Internal, 0,
        ).unwrap();
        let subtree = store.get_agent_subtree(&ceo.id).unwrap();
        assert_eq!(subtree.len(), 3);
    }

    #[test]
    fn given_company_when_agent_count_then_reflects_active_agents() {
        let store = make_store();
        let c = store.create_company("CountCo", "", "").unwrap();
        let a1 = store.hire_agent(&c.id, "A1", "T", CompanyRole::Agent, None, &[], AdapterType::Internal, 0).unwrap();
        let _ = store.hire_agent(&c.id, "A2", "T", CompanyRole::Agent, None, &[], AdapterType::Internal, 0).unwrap();
        assert_eq!(store.agent_count(&c.id).unwrap(), 2);
        store.fire_agent(&a1.id).unwrap();
        assert_eq!(store.agent_count(&c.id).unwrap(), 1);
    }

    // ── Activity log ─────────────────────────────────────────────────────────

    #[test]
    fn given_company_created_when_activity_listed_then_creation_event_present() {
        let store = make_store();
        let c = store.create_company("LogCo", "", "").unwrap();
        let log = store.list_activity(&c.id, 10).unwrap();
        assert!(!log.is_empty());
        assert_eq!(log[0].action, "company.created");
    }

    #[test]
    fn given_agent_hired_when_activity_listed_then_hire_event_present() {
        let store = make_store();
        let c = store.create_company("HireCo", "", "").unwrap();
        let _ = store.hire_agent(&c.id, "Hank", "Dev", CompanyRole::Agent, None, &[], AdapterType::Internal, 0).unwrap();
        let log = store.list_activity(&c.id, 10).unwrap();
        let hire_event = log.iter().find(|e| e.action == "agent.hired");
        assert!(hire_event.is_some());
    }
}
