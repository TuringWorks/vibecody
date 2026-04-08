#![allow(dead_code)]
//! Per-agent monthly budget management with hard-stop enforcement.
//!
//! Each budget record covers one (company, agent, month) triple.
//! Cost events are ingested from model calls and accumulate against the
//! monthly limit. When `hard_stop` is true and `spent_cents >= limit_cents`,
//! the agent is automatically paused via AgentPool.

use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

/// Current month as "YYYY-MM".
pub fn current_month() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Days since epoch → year/month (Gregorian, no leap-second awareness needed)
    let days = secs / 86400;
    let (y, m) = days_to_year_month(days);
    format!("{:04}-{:02}", y, m)
}

fn days_to_year_month(mut days: u64) -> (u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let yd = if leap { 366 } else { 365 };
        if days < yd { break; }
        days -= yd;
        year += 1;
    }
    let leap = is_leap(year);
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &months {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    (year, month)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyBudget {
    pub id: String,
    pub company_id: String,
    pub agent_id: String,
    /// "YYYY-MM" format.
    pub month: String,
    pub limit_cents: i64,
    pub spent_cents: i64,
    /// If true, agent is paused when limit is reached.
    pub hard_stop: bool,
    /// Alert threshold 0–100 (percent).
    pub alert_pct: i64,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEvent {
    pub id: i64,
    pub company_id: String,
    pub agent_id: String,
    pub budget_id: String,
    pub amount_cents: i64,
    pub model: String,
    pub task_id: Option<String>,
    pub description: String,
    pub created_at: u64,
}

// ── BudgetStore ───────────────────────────────────────────────────────────────

pub struct BudgetStore<'a> {
    conn: &'a Connection,
}

impl<'a> BudgetStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }

    pub fn current_month_static() -> String { current_month() }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS budgets (
                id            TEXT PRIMARY KEY,
                company_id    TEXT NOT NULL,
                agent_id      TEXT NOT NULL,
                month         TEXT NOT NULL,
                limit_cents   INTEGER NOT NULL DEFAULT 0,
                spent_cents   INTEGER NOT NULL DEFAULT 0,
                hard_stop     INTEGER NOT NULL DEFAULT 0,
                alert_pct     INTEGER NOT NULL DEFAULT 80,
                created_at    INTEGER NOT NULL,
                UNIQUE(company_id, agent_id, month)
            );
            CREATE INDEX IF NOT EXISTS idx_budgets_company ON budgets(company_id);
            CREATE INDEX IF NOT EXISTS idx_budgets_agent ON budgets(company_id, agent_id);

            CREATE TABLE IF NOT EXISTS cost_events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id    TEXT NOT NULL,
                agent_id      TEXT NOT NULL,
                budget_id     TEXT NOT NULL,
                amount_cents  INTEGER NOT NULL,
                model         TEXT NOT NULL DEFAULT '',
                task_id       TEXT,
                description   TEXT NOT NULL DEFAULT '',
                created_at    INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cost_events_budget ON cost_events(budget_id);
            CREATE INDEX IF NOT EXISTS idx_cost_events_agent ON cost_events(company_id, agent_id);
        "#)?;
        Ok(())
    }

    /// Upsert a budget record for (company, agent, month).
    pub fn set_budget(
        &self,
        company_id: &str,
        agent_id: &str,
        month: &str,
        limit_cents: i64,
        hard_stop: bool,
        alert_pct: i64,
    ) -> Result<CompanyBudget> {
        // Check if exists
        let existing: Option<String> = self.conn.query_row(
            "SELECT id FROM budgets WHERE company_id = ?1 AND agent_id = ?2 AND month = ?3",
            params![company_id, agent_id, month],
            |r| r.get(0),
        ).ok();

        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE budgets SET limit_cents = ?1, hard_stop = ?2, alert_pct = ?3 WHERE id = ?4",
                params![limit_cents, hard_stop as i64, alert_pct, id],
            )?;
            self.get_by_id(&id)?.context("budget not found after update")
        } else {
            let id = new_id();
            let now = now_ms();
            self.conn.execute(
                "INSERT INTO budgets (id, company_id, agent_id, month, limit_cents, spent_cents, hard_stop, alert_pct, created_at)
                 VALUES (?1,?2,?3,?4,?5,0,?6,?7,?8)",
                params![id, company_id, agent_id, month, limit_cents, hard_stop as i64, alert_pct, now as i64],
            )?;
            self.get_by_id(&id)?.context("budget not found after insert")
        }
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<CompanyBudget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, month, limit_cents, spent_cents, hard_stop, alert_pct, created_at
             FROM budgets WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], row_to_budget)?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn get_for_month(
        &self,
        company_id: &str,
        agent_id: &str,
        month: &str,
    ) -> Result<Option<CompanyBudget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, month, limit_cents, spent_cents, hard_stop, alert_pct, created_at
             FROM budgets WHERE company_id = ?1 AND agent_id = ?2 AND month = ?3",
        )?;
        let mut rows = stmt.query_map(params![company_id, agent_id, month], row_to_budget)?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<CompanyBudget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, agent_id, month, limit_cents, spent_cents, hard_stop, alert_pct, created_at
             FROM budgets WHERE company_id = ?1 ORDER BY month DESC, agent_id ASC",
        )?;
        let rows = stmt.query_map(params![company_id], row_to_budget)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Ingest a cost event and accumulate against the active month budget.
    /// Returns the updated budget and whether the hard-stop was triggered.
    pub fn ingest_cost(
        &self,
        company_id: &str,
        agent_id: &str,
        amount_cents: i64,
        model: &str,
        task_id: Option<&str>,
        description: &str,
    ) -> Result<(CompanyBudget, bool)> {
        let month = current_month();
        // Ensure budget record exists (auto-create with $0 limit if missing)
        let budget = match self.get_for_month(company_id, agent_id, &month)? {
            Some(b) => b,
            None => self.set_budget(company_id, agent_id, &month, 0, false, 80)?,
        };

        // Record cost event
        self.conn.execute(
            "INSERT INTO cost_events (company_id, agent_id, budget_id, amount_cents, model, task_id, description, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![company_id, agent_id, budget.id, amount_cents, model, task_id, description, now_ms() as i64],
        )?;

        // Update spent
        self.conn.execute(
            "UPDATE budgets SET spent_cents = spent_cents + ?1 WHERE id = ?2",
            params![amount_cents, budget.id],
        )?;

        let updated = self.get_by_id(&budget.id)?.context("budget vanished")?;
        let hard_stop_triggered = updated.hard_stop
            && updated.limit_cents > 0
            && updated.spent_cents >= updated.limit_cents;

        Ok((updated, hard_stop_triggered))
    }

    pub fn list_events(&self, company_id: &str, agent_id: Option<&str>) -> Result<Vec<CostEvent>> {
        let (sql, use_agent) = if agent_id.is_some() {
            ("SELECT id, company_id, agent_id, budget_id, amount_cents, model, task_id, description, created_at
              FROM cost_events WHERE company_id = ?1 AND agent_id = ?2 ORDER BY created_at DESC LIMIT 100", true)
        } else {
            ("SELECT id, company_id, agent_id, budget_id, amount_cents, model, task_id, description, created_at
              FROM cost_events WHERE company_id = ?1 ORDER BY created_at DESC LIMIT 100", false)
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = if use_agent {
            stmt.query_map(params![company_id, agent_id.unwrap()], row_to_event)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![company_id], row_to_event)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    /// Budget utilization 0.0–1.0 (0.0 if no limit set).
    pub fn utilization(budget: &CompanyBudget) -> f64 {
        if budget.limit_cents == 0 { return 0.0; }
        (budget.spent_cents as f64 / budget.limit_cents as f64).clamp(0.0, 1.0)
    }

    /// True if spent >= alert threshold.
    pub fn is_over_alert(budget: &CompanyBudget) -> bool {
        if budget.limit_cents == 0 { return false; }
        let pct = (budget.spent_cents as f64 / budget.limit_cents as f64) * 100.0;
        pct >= budget.alert_pct as f64
    }
}

// ── Row helpers ───────────────────────────────────────────────────────────────

fn row_to_budget(row: &rusqlite::Row) -> rusqlite::Result<CompanyBudget> {
    Ok(CompanyBudget {
        id: row.get(0)?,
        company_id: row.get(1)?,
        agent_id: row.get(2)?,
        month: row.get(3)?,
        limit_cents: row.get(4)?,
        spent_cents: row.get(5)?,
        hard_stop: row.get::<_, i64>(6)? != 0,
        alert_pct: row.get(7)?,
        created_at: row.get::<_, i64>(8)? as u64,
    })
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<CostEvent> {
    Ok(CostEvent {
        id: row.get(0)?,
        company_id: row.get(1)?,
        agent_id: row.get(2)?,
        budget_id: row.get(3)?,
        amount_cents: row.get(4)?,
        model: row.get(5)?,
        task_id: row.get(6)?,
        description: row.get(7)?,
        created_at: row.get::<_, i64>(8)? as u64,
    })
}

// ── Display helpers ───────────────────────────────────────────────────────────

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    // ── set_budget ───────────────────────────────────────────────────────────

    #[test]
    fn given_new_budget_when_set_then_returned_with_zero_spent() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let b = store.set_budget("co1", "ag1", "2026-04", 10_000, false, 80).unwrap();
        assert_eq!(b.limit_cents, 10_000);
        assert_eq!(b.spent_cents, 0);
        assert_eq!(b.month, "2026-04");
        assert!(!b.hard_stop);
    }

    #[test]
    fn given_existing_budget_when_set_budget_called_again_then_limit_updated() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set_budget("co1", "ag1", "2026-04", 5_000, false, 80).unwrap();
        let updated = store.set_budget("co1", "ag1", "2026-04", 20_000, true, 90).unwrap();
        assert_eq!(updated.limit_cents, 20_000);
        assert!(updated.hard_stop);
        assert_eq!(updated.alert_pct, 90);
        // Only one record exists
        let list = store.list("co1").unwrap();
        assert_eq!(list.len(), 1);
    }

    // ── get_for_month ────────────────────────────────────────────────────────

    #[test]
    fn given_budget_when_get_for_month_then_returns_correct_record() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set_budget("co1", "ag1", "2026-04", 1_000, false, 80).unwrap();
        let result = store.get_for_month("co1", "ag1", "2026-04").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn given_no_budget_when_get_for_month_then_returns_none() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.get_for_month("co1", "ag1", "2025-01").unwrap();
        assert!(result.is_none());
    }

    // ── ingest_cost ──────────────────────────────────────────────────────────

    #[test]
    fn given_cost_ingested_when_get_for_month_then_spent_accumulates() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 50_000, false, 80).unwrap();
        store.ingest_cost("co1", "ag1", 1_500, "gpt-4", None, "task-run").unwrap();
        store.ingest_cost("co1", "ag1", 2_000, "claude-3", None, "task-run-2").unwrap();
        let budget = store.get_for_month("co1", "ag1", &month).unwrap().unwrap();
        assert_eq!(budget.spent_cents, 3_500);
    }

    #[test]
    fn given_no_existing_budget_when_ingest_cost_then_auto_creates_budget() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let (budget, _triggered) = store.ingest_cost("co1", "ag-new", 100, "gpt-4", None, "desc").unwrap();
        assert_eq!(budget.agent_id, "ag-new");
        assert_eq!(budget.spent_cents, 100);
    }

    #[test]
    fn given_hard_stop_true_and_limit_reached_when_ingest_then_triggered_flag_is_true() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 1_000, true, 80).unwrap();
        let (_, triggered) = store.ingest_cost("co1", "ag1", 1_000, "gpt-4", None, "final").unwrap();
        assert!(triggered);
    }

    #[test]
    fn given_hard_stop_false_and_limit_reached_when_ingest_then_triggered_flag_is_false() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 1_000, false, 80).unwrap();
        let (_, triggered) = store.ingest_cost("co1", "ag1", 1_000, "gpt-4", None, "final").unwrap();
        assert!(!triggered);
    }

    #[test]
    fn given_zero_limit_when_ingest_cost_then_hard_stop_never_triggers() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 0, true, 80).unwrap();
        let (_, triggered) = store.ingest_cost("co1", "ag1", 99_999, "gpt-4", None, "desc").unwrap();
        assert!(!triggered);
    }

    // ── utilization ──────────────────────────────────────────────────────────

    #[test]
    fn given_half_spent_budget_when_utilization_then_returns_0_point_5() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 2_000, false, 80).unwrap();
        store.ingest_cost("co1", "ag1", 1_000, "m", None, "d").unwrap();
        let budget = store.get_for_month("co1", "ag1", &month).unwrap().unwrap();
        let util = BudgetStore::utilization(&budget);
        assert!((util - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn given_zero_limit_budget_when_utilization_then_returns_zero() {
        let b = CompanyBudget {
            id: "x".into(), company_id: "c".into(), agent_id: "a".into(),
            month: "2026-04".into(), limit_cents: 0, spent_cents: 999,
            hard_stop: false, alert_pct: 80, created_at: 0,
        };
        assert_eq!(BudgetStore::utilization(&b), 0.0);
    }

    // ── is_over_alert ────────────────────────────────────────────────────────

    #[test]
    fn given_budget_over_alert_threshold_when_is_over_alert_then_true() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 1_000, false, 80).unwrap();
        store.ingest_cost("co1", "ag1", 850, "m", None, "d").unwrap();
        let budget = store.get_for_month("co1", "ag1", &month).unwrap().unwrap();
        assert!(BudgetStore::is_over_alert(&budget));
    }

    #[test]
    fn given_budget_under_alert_threshold_when_is_over_alert_then_false() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        let month = current_month();
        store.set_budget("co1", "ag1", &month, 1_000, false, 80).unwrap();
        store.ingest_cost("co1", "ag1", 500, "m", None, "d").unwrap();
        let budget = store.get_for_month("co1", "ag1", &month).unwrap().unwrap();
        assert!(!BudgetStore::is_over_alert(&budget));
    }

    // ── list_events ──────────────────────────────────────────────────────────

    #[test]
    fn given_multiple_cost_events_when_list_events_then_returns_all_for_company() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        store.ingest_cost("co1", "ag1", 100, "gpt-4", None, "a").unwrap();
        store.ingest_cost("co1", "ag2", 200, "claude-3", None, "b").unwrap();
        store.ingest_cost("co2", "ag3", 300, "llama", None, "c").unwrap();
        let events = store.list_events("co1", None).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn given_multiple_agents_when_list_events_with_agent_filter_then_only_that_agent() {
        let conn = make_conn();
        let store = BudgetStore::new(&conn);
        store.ensure_schema().unwrap();
        store.ingest_cost("co1", "ag1", 100, "m", None, "d").unwrap();
        store.ingest_cost("co1", "ag2", 200, "m", None, "d").unwrap();
        let events = store.list_events("co1", Some("ag1")).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].agent_id, "ag1");
    }
}

impl CompanyBudget {
    pub fn summary_line(&self) -> String {
        let limit_str = if self.limit_cents == 0 {
            "unlimited".to_string()
        } else {
            format!("${:.2}", self.limit_cents as f64 / 100.0)
        };
        let spent_str = format!("${:.2}", self.spent_cents as f64 / 100.0);
        let pct = if self.limit_cents > 0 {
            format!("{:.1}%", (self.spent_cents as f64 / self.limit_cents as f64) * 100.0)
        } else {
            "—".to_string()
        };
        let alert = if BudgetStore::is_over_alert(self) { " ⚠" } else { "" };
        let hard = if self.hard_stop { " [hard-stop]" } else { "" };
        format!(
            "[{}] {} {} / {}  ({}){}{} — agent:{}",
            self.month, &self.agent_id[..8.min(self.agent_id.len())],
            spent_str, limit_str, pct, alert, hard,
            &self.agent_id[..8.min(self.agent_id.len())]
        )
    }
}
