#![allow(dead_code)]
//! Recurring routines for company agent automation.
//!
//! Routines define periodic tasks that an agent should perform automatically.
//! The HeartbeatManager (company_heartbeat.rs) drives routine execution by
//! calling `tick()` on the routine store, which returns all due routines.

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    pub id: String,
    pub company_id: String,
    pub agent_id: String,
    pub name: String,
    /// Prompt/instruction the agent executes.
    pub prompt: String,
    /// Run interval in seconds.
    pub interval_secs: i64,
    /// Unix timestamp (ms) of next scheduled run.
    pub next_run_at: u64,
    /// Unix timestamp (ms) of last run (0 if never).
    pub last_run_at: u64,
    /// Whether the routine is active.
    pub active: bool,
    /// Max simultaneous runs (0 = unlimited).
    pub max_concurrent: i64,
    pub created_at: u64,
}

// ── RoutineStore ──────────────────────────────────────────────────────────────

pub struct RoutineStore<'a> {
    conn: &'a Connection,
}

impl<'a> RoutineStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS routines (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                agent_id        TEXT NOT NULL,
                name            TEXT NOT NULL,
                prompt          TEXT NOT NULL DEFAULT '',
                interval_secs   INTEGER NOT NULL DEFAULT 3600,
                next_run_at     INTEGER NOT NULL,
                last_run_at     INTEGER NOT NULL DEFAULT 0,
                active          INTEGER NOT NULL DEFAULT 1,
                max_concurrent  INTEGER NOT NULL DEFAULT 1,
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_routines_company ON routines(company_id);
            CREATE INDEX IF NOT EXISTS idx_routines_agent ON routines(agent_id);
            CREATE INDEX IF NOT EXISTS idx_routines_due ON routines(active, next_run_at);
        "#)?;
        Ok(())
    }

    pub fn create(
        &self,
        company_id: &str,
        agent_id: &str,
        name: &str,
        prompt: &str,
        interval_secs: i64,
    ) -> Result<Routine> {
        let now = now_ms();
        let routine = Routine {
            id: new_id(),
            company_id: company_id.to_string(),
            agent_id: agent_id.to_string(),
            name: name.to_string(),
            prompt: prompt.to_string(),
            interval_secs,
            next_run_at: now + (interval_secs as u64 * 1000),
            last_run_at: 0,
            active: true,
            max_concurrent: 1,
            created_at: now,
        };
        self.conn.execute(
            "INSERT INTO routines (id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                routine.id, routine.company_id, routine.agent_id, routine.name,
                routine.prompt, routine.interval_secs,
                routine.next_run_at as i64, routine.last_run_at as i64,
                routine.active as i64, routine.max_concurrent, routine.created_at as i64,
            ],
        )?;
        Ok(routine)
    }

    pub fn get(&self, id: &str) -> Result<Option<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at
             FROM routines WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_routine(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at
             FROM routines WHERE company_id = ?1 ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| row_to_routine(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Toggle active/inactive.
    pub fn toggle(&self, id: &str) -> Result<Routine> {
        let routine = self.get(id)?.ok_or_else(|| anyhow!("routine not found"))?;
        self.conn.execute(
            "UPDATE routines SET active = ?1 WHERE id = ?2",
            params![(!routine.active) as i64, id],
        )?;
        self.get(id)?.ok_or_else(|| anyhow!("routine not found after toggle"))
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let n = self.conn.execute("DELETE FROM routines WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Return all due routines (active=1, next_run_at <= now).
    pub fn due_routines(&self) -> Result<Vec<Routine>> {
        let now = now_ms() as i64;
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at
             FROM routines WHERE active = 1 AND next_run_at <= ?1 ORDER BY next_run_at ASC",
        )?;
        let rows = stmt.query_map(params![now], |row| row_to_routine(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Mark a routine as having run — advance next_run_at.
    pub fn mark_ran(&self, id: &str) -> Result<()> {
        let routine = self.get(id)?.ok_or_else(|| anyhow!("routine not found"))?;
        let now = now_ms();
        let next = now + (routine.interval_secs as u64 * 1000);
        self.conn.execute(
            "UPDATE routines SET last_run_at = ?1, next_run_at = ?2 WHERE id = ?3",
            params![now as i64, next as i64, id],
        )?;
        Ok(())
    }
}

fn row_to_routine(row: &rusqlite::Row) -> rusqlite::Result<Routine> {
    Ok(Routine {
        id: row.get(0)?,
        company_id: row.get(1)?,
        agent_id: row.get(2)?,
        name: row.get(3)?,
        prompt: row.get(4)?,
        interval_secs: row.get(5)?,
        next_run_at: row.get::<_, i64>(6)? as u64,
        last_run_at: row.get::<_, i64>(7)? as u64,
        active: row.get::<_, i64>(8)? != 0,
        max_concurrent: row.get(9)?,
        created_at: row.get::<_, i64>(10)? as u64,
    })
}

impl Routine {
    pub fn summary_line(&self) -> String {
        let status = if self.active { "●" } else { "○" };
        let interval = if self.interval_secs < 3600 {
            format!("{}m", self.interval_secs / 60)
        } else {
            format!("{}h", self.interval_secs / 3600)
        };
        format!(
            "{} {} [every {}]  agent:{}  [{}]",
            status, self.name, interval,
            &self.agent_id[..8.min(self.agent_id.len())],
            &self.id[..8.min(self.id.len())]
        )
    }
}
