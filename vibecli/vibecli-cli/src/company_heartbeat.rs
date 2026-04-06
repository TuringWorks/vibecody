#![allow(dead_code)]
//! Agent heartbeat system for company orchestration.
//!
//! HeartbeatRun records track when an agent was activated, what triggered it
//! (scheduled routine, event, or manual), and the resulting session/output.
//!
//! The HeartbeatManager provides:
//! - Recording heartbeat run lifecycle (start → complete/fail)
//! - History queries per agent or company
//! - Run status for concurrent limit enforcement (used by RoutineStore)

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatTrigger {
    Scheduled,
    Event,
    Manual,
}

impl HeartbeatTrigger {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Event => "event",
            Self::Manual => "manual",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "event" => Self::Event,
            "manual" => Self::Manual,
            _ => Self::Scheduled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatStatus {
    Running,
    Completed,
    Failed,
}

impl HeartbeatStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Running,
        }
    }
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRun {
    pub id: String,
    pub company_id: String,
    pub agent_id: String,
    pub trigger: HeartbeatTrigger,
    pub status: HeartbeatStatus,
    /// Linked session ID (if agent spawned a session).
    pub session_id: Option<String>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub summary: Option<String>,
}

// ── HeartbeatStore ────────────────────────────────────────────────────────────

pub struct HeartbeatStore<'a> {
    conn: &'a Connection,
}

impl<'a> HeartbeatStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS heartbeat_runs (
                id          TEXT PRIMARY KEY,
                company_id  TEXT NOT NULL,
                agent_id    TEXT NOT NULL,
                trigger     TEXT NOT NULL DEFAULT 'scheduled',
                status      TEXT NOT NULL DEFAULT 'running',
                session_id  TEXT,
                started_at  INTEGER NOT NULL,
                finished_at INTEGER,
                summary     TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_hb_company ON heartbeat_runs(company_id);
            CREATE INDEX IF NOT EXISTS idx_hb_agent ON heartbeat_runs(agent_id);
            CREATE INDEX IF NOT EXISTS idx_hb_status ON heartbeat_runs(agent_id, status);
        "#)?;
        Ok(())
    }

    /// Start a new heartbeat run.
    pub fn start(
        &self,
        company_id: &str,
        agent_id: &str,
        trigger: HeartbeatTrigger,
        session_id: Option<&str>,
    ) -> Result<HeartbeatRun> {
        let run = HeartbeatRun {
            id: new_id(),
            company_id: company_id.to_string(),
            agent_id: agent_id.to_string(),
            trigger,
            status: HeartbeatStatus::Running,
            session_id: session_id.map(|s| s.to_string()),
            started_at: now_ms(),
            finished_at: None,
            summary: None,
        };
        self.conn.execute(
            "INSERT INTO heartbeat_runs (id, company_id, agent_id, trigger, status, session_id, started_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                run.id, run.company_id, run.agent_id,
                run.trigger.as_str(), run.status.as_str(),
                run.session_id, run.started_at as i64,
            ],
        )?;
        Ok(run)
    }

    /// Complete a heartbeat run.
    pub fn complete(&self, id: &str, summary: Option<&str>) -> Result<HeartbeatRun> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE heartbeat_runs SET status = 'completed', finished_at = ?1, summary = ?2 WHERE id = ?3",
            params![now as i64, summary, id],
        )?;
        self.get(id)?.ok_or_else(|| anyhow!("heartbeat run not found"))
    }

    /// Fail a heartbeat run.
    pub fn fail(&self, id: &str, error: &str) -> Result<HeartbeatRun> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE heartbeat_runs SET status = 'failed', finished_at = ?1, summary = ?2 WHERE id = ?3",
            params![now as i64, error, id],
        )?;
        self.get(id)?.ok_or_else(|| anyhow!("heartbeat run not found"))
    }

    pub fn get(&self, id: &str) -> Result<Option<HeartbeatRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, trigger, status, session_id, started_at, finished_at, summary
             FROM heartbeat_runs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_run(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    /// List recent heartbeat runs for an agent.
    pub fn history(&self, agent_id: &str, limit: i64) -> Result<Vec<HeartbeatRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, trigger, status, session_id, started_at, finished_at, summary
             FROM heartbeat_runs WHERE agent_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![agent_id, limit], |row| row_to_run(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// List recent heartbeat runs for a company.
    pub fn company_history(&self, company_id: &str, limit: i64) -> Result<Vec<HeartbeatRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, trigger, status, session_id, started_at, finished_at, summary
             FROM heartbeat_runs WHERE company_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![company_id, limit], |row| row_to_run(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Count running heartbeats for an agent (for concurrent limit enforcement).
    pub fn running_count(&self, agent_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM heartbeat_runs WHERE agent_id = ?1 AND status = 'running'",
            params![agent_id],
            |r| r.get(0),
        )?;
        Ok(count)
    }

    /// Manual trigger: start a heartbeat run for an agent immediately.
    pub fn trigger_manual(
        &self,
        company_id: &str,
        agent_id: &str,
    ) -> Result<HeartbeatRun> {
        self.start(company_id, agent_id, HeartbeatTrigger::Manual, None)
    }
}

fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<HeartbeatRun> {
    Ok(HeartbeatRun {
        id: row.get(0)?,
        company_id: row.get(1)?,
        agent_id: row.get(2)?,
        trigger: HeartbeatTrigger::from_str(&row.get::<_, String>(3)?),
        status: HeartbeatStatus::from_str(&row.get::<_, String>(4)?),
        session_id: row.get(5)?,
        started_at: row.get::<_, i64>(6)? as u64,
        finished_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        summary: row.get(8)?,
    })
}

impl HeartbeatRun {
    pub fn summary_line(&self) -> String {
        let status_icon = match self.status {
            HeartbeatStatus::Running => "▶",
            HeartbeatStatus::Completed => "✓",
            HeartbeatStatus::Failed => "✗",
        };
        let duration = match self.finished_at {
            Some(end) => {
                let ms = end.saturating_sub(self.started_at);
                if ms < 1000 { format!("{}ms", ms) } else { format!("{:.1}s", ms as f64 / 1000.0) }
            }
            None => "running".to_string(),
        };
        format!(
            "{} [{}] {}  agent:{}  {}  [{}]",
            status_icon, self.trigger.as_str(), duration,
            &self.agent_id[..8.min(self.agent_id.len())],
            self.summary.as_deref().unwrap_or(""),
            &self.id[..8.min(self.id.len())]
        )
    }
}
