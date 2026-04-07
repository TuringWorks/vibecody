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
    /// Delivery mode: "none", "email", "slack", etc.
    pub delivery_mode: String,
    /// Optional skill name to invoke.
    pub skill_name: Option<String>,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional thinking level override.
    pub thinking_level: Option<String>,
    /// Optional timeout in seconds.
    pub timeout_secs: Option<i64>,
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
        // Schema migrations for new columns
        let _ = self.conn.execute_batch("ALTER TABLE routines ADD COLUMN IF NOT EXISTS delivery_mode TEXT NOT NULL DEFAULT 'none'");
        let _ = self.conn.execute_batch("ALTER TABLE routines ADD COLUMN IF NOT EXISTS skill_name TEXT");
        let _ = self.conn.execute_batch("ALTER TABLE routines ADD COLUMN IF NOT EXISTS model TEXT");
        let _ = self.conn.execute_batch("ALTER TABLE routines ADD COLUMN IF NOT EXISTS thinking_level TEXT");
        let _ = self.conn.execute_batch("ALTER TABLE routines ADD COLUMN IF NOT EXISTS timeout_secs INTEGER");
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
        self.create_with_delivery(company_id, agent_id, name, prompt, interval_secs, "none", None)
    }

    pub fn create_with_delivery(
        &self,
        company_id: &str,
        agent_id: &str,
        name: &str,
        prompt: &str,
        interval_secs: i64,
        delivery_mode: &str,
        skill_name: Option<&str>,
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
            delivery_mode: delivery_mode.to_string(),
            skill_name: skill_name.map(|s| s.to_string()),
            model: None,
            thinking_level: None,
            timeout_secs: None,
        };
        self.conn.execute(
            "INSERT INTO routines (id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at, delivery_mode, skill_name)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                routine.id, routine.company_id, routine.agent_id, routine.name,
                routine.prompt, routine.interval_secs,
                routine.next_run_at as i64, routine.last_run_at as i64,
                routine.active as i64, routine.max_concurrent, routine.created_at as i64,
                routine.delivery_mode, routine.skill_name,
            ],
        )?;
        Ok(routine)
    }

    pub fn set_delivery_mode(&self, id: &str, mode: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE routines SET delivery_mode = ?1 WHERE id = ?2",
            params![mode, id],
        )?;
        Ok(())
    }

    pub fn list_json(&self) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at, delivery_mode, skill_name, model, thinking_level, timeout_secs
             FROM routines ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |row| row_to_routine_full(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let values = rows.into_iter()
            .map(|r| serde_json::to_value(&r).unwrap_or(serde_json::Value::Null))
            .collect();
        Ok(values)
    }

    pub fn get(&self, id: &str) -> Result<Option<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at, COALESCE(delivery_mode,'none'), skill_name, model, thinking_level, timeout_secs
             FROM routines WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_routine_full(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<Routine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at, COALESCE(delivery_mode,'none'), skill_name, model, thinking_level, timeout_secs
             FROM routines WHERE company_id = ?1 ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| row_to_routine_full(row))?
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
            "SELECT id, company_id, agent_id, name, prompt, interval_secs, next_run_at, last_run_at, active, max_concurrent, created_at, COALESCE(delivery_mode,'none'), skill_name, model, thinking_level, timeout_secs
             FROM routines WHERE active = 1 AND next_run_at <= ?1 ORDER BY next_run_at ASC",
        )?;
        let rows = stmt.query_map(params![now], |row| row_to_routine_full(row))?
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

fn row_to_routine_full(row: &rusqlite::Row) -> rusqlite::Result<Routine> {
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
        delivery_mode: row.get::<_, Option<String>>(11)?.unwrap_or_else(|| "none".to_string()),
        skill_name: row.get(12)?,
        model: row.get(13)?,
        thinking_level: row.get(14)?,
        timeout_secs: row.get(15)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    // ── create ───────────────────────────────────────────────────────────────

    #[test]
    fn given_new_routine_when_created_then_active_and_last_run_is_zero() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "daily-report", "Run daily report", 86400).unwrap();
        assert_eq!(r.name, "daily-report");
        assert!(r.active);
        assert_eq!(r.last_run_at, 0);
        assert_eq!(r.interval_secs, 86400);
    }

    #[test]
    fn given_new_routine_when_created_then_next_run_is_in_future() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let before_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let r = store.create("co1", "ag1", "check", "Check things", 3600).unwrap();
        assert!(r.next_run_at > before_ms);
    }

    // ── get / list ───────────────────────────────────────────────────────────

    #[test]
    fn given_created_routine_when_get_by_id_then_returned() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "ping", "Ping the world", 60).unwrap();
        let fetched = store.get(&r.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, r.id);
    }

    #[test]
    fn given_multiple_routines_when_list_then_returns_all_for_company() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        store.create("co1", "ag1", "alpha", "", 3600).unwrap();
        store.create("co1", "ag2", "beta", "", 3600).unwrap();
        store.create("co2", "ag3", "gamma", "", 3600).unwrap();
        let list = store.list("co1").unwrap();
        assert_eq!(list.len(), 2);
    }

    // ── toggle ───────────────────────────────────────────────────────────────

    #[test]
    fn given_active_routine_when_toggled_then_becomes_inactive() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "toggler", "", 3600).unwrap();
        assert!(r.active);
        let toggled = store.toggle(&r.id).unwrap();
        assert!(!toggled.active);
    }

    #[test]
    fn given_inactive_routine_when_toggled_then_becomes_active() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "flip", "", 3600).unwrap();
        store.toggle(&r.id).unwrap(); // deactivate
        let re_activated = store.toggle(&r.id).unwrap(); // re-activate
        assert!(re_activated.active);
    }

    // ── delete ───────────────────────────────────────────────────────────────

    #[test]
    fn given_existing_routine_when_deleted_then_returns_true_and_not_found() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "delete-me", "", 3600).unwrap();
        let deleted = store.delete(&r.id).unwrap();
        assert!(deleted);
        assert!(store.get(&r.id).unwrap().is_none());
    }

    #[test]
    fn given_nonexistent_id_when_delete_then_returns_false() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.delete("ghost-routine-id").unwrap();
        assert!(!result);
    }

    // ── due_routines ─────────────────────────────────────────────────────────

    #[test]
    fn given_routine_just_created_when_due_routines_checked_then_not_yet_due() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        // interval of 1 hour means next_run_at = now + 3600s, not due yet
        store.create("co1", "ag1", "future", "", 3600).unwrap();
        let due = store.due_routines().unwrap();
        assert!(due.is_empty());
    }

    #[test]
    fn given_routine_with_past_next_run_when_due_routines_checked_then_returned() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "overdue", "", 3600).unwrap();
        // Manually backdate next_run_at to 1 ms ago
        let past = 1i64; // epoch start, definitely in the past
        conn.execute(
            "UPDATE routines SET next_run_at = ?1 WHERE id = ?2",
            rusqlite::params![past, r.id],
        ).unwrap();
        let due = store.due_routines().unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, r.id);
    }

    #[test]
    fn given_inactive_routine_with_past_next_run_when_due_routines_checked_then_not_returned() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "off-overdue", "", 3600).unwrap();
        conn.execute(
            "UPDATE routines SET next_run_at = 1, active = 0 WHERE id = ?1",
            rusqlite::params![r.id],
        ).unwrap();
        let due = store.due_routines().unwrap();
        assert!(due.is_empty());
    }

    // ── mark_ran ─────────────────────────────────────────────────────────────

    #[test]
    fn given_routine_when_mark_ran_called_then_last_run_at_is_updated() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "ran", "", 3600).unwrap();
        assert_eq!(r.last_run_at, 0);
        store.mark_ran(&r.id).unwrap();
        let updated = store.get(&r.id).unwrap().unwrap();
        assert!(updated.last_run_at > 0);
    }

    #[test]
    fn given_routine_when_mark_ran_called_then_next_run_advances_by_interval() {
        let conn = make_conn();
        let store = RoutineStore::new(&conn);
        store.ensure_schema().unwrap();
        let r = store.create("co1", "ag1", "advance", "", 3600).unwrap();
        let before_next = r.next_run_at;
        store.mark_ran(&r.id).unwrap();
        let updated = store.get(&r.id).unwrap().unwrap();
        // new next_run_at should be approximately now + interval (at least > old next_run_at)
        assert!(updated.next_run_at > before_next || updated.last_run_at > 0);
        // The next_run_at should be last_run_at + interval_ms
        let expected_diff = r.interval_secs as u64 * 1000;
        let actual_diff = updated.next_run_at.saturating_sub(updated.last_run_at);
        assert_eq!(actual_diff, expected_diff);
    }
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
