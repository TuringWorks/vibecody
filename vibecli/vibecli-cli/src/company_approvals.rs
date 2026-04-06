#![allow(dead_code)]
//! Approval workflows for company orchestration.
//!
//! Approvals gate high-impact operations: hiring agents, budget changes,
//! strategic pivots, task execution, and deploys. Requests flow through
//! policy_engine for role-based gating before landing in the pending queue.
//!
//! Workflow: request → [policy check] → pending → decided (approved/rejected)
//!   or cancelled by requester.

use anyhow::{anyhow, Context, Result};
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
pub enum ApprovalRequestType {
    Hire,
    Strategy,
    Budget,
    Task,
    Deploy,
}

impl ApprovalRequestType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hire => "hire",
            Self::Strategy => "strategy",
            Self::Budget => "budget",
            Self::Task => "task",
            Self::Deploy => "deploy",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "strategy" => Self::Strategy,
            "budget" => Self::Budget,
            "task" => Self::Task,
            "deploy" => Self::Deploy,
            _ => Self::Hire,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub id: String,
    pub company_id: String,
    pub request_type: ApprovalRequestType,
    /// ID of the entity being acted upon (agent_id, task_id, etc.).
    pub subject_id: String,
    pub requester_id: String,
    pub status: ApprovalStatus,
    pub reason: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<u64>,
    pub created_at: u64,
}

// ── ApprovalStore ─────────────────────────────────────────────────────────────

pub struct ApprovalStore<'a> {
    conn: &'a Connection,
}

impl<'a> ApprovalStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS approvals (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                request_type    TEXT NOT NULL,
                subject_id      TEXT NOT NULL,
                requester_id    TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'pending',
                reason          TEXT NOT NULL DEFAULT '',
                decided_by      TEXT,
                decided_at      INTEGER,
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_approvals_company ON approvals(company_id);
            CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(company_id, status);
            CREATE INDEX IF NOT EXISTS idx_approvals_requester ON approvals(requester_id);
        "#)?;
        Ok(())
    }

    /// Create a new approval request.
    pub fn request(
        &self,
        company_id: &str,
        request_type: ApprovalRequestType,
        subject_id: &str,
        requester_id: &str,
        reason: &str,
    ) -> Result<Approval> {
        let approval = Approval {
            id: new_id(),
            company_id: company_id.to_string(),
            request_type,
            subject_id: subject_id.to_string(),
            requester_id: requester_id.to_string(),
            status: ApprovalStatus::Pending,
            reason: reason.to_string(),
            decided_by: None,
            decided_at: None,
            created_at: now_ms(),
        };
        self.conn.execute(
            "INSERT INTO approvals (id, company_id, request_type, subject_id, requester_id, status, reason, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                approval.id, approval.company_id, approval.request_type.as_str(),
                approval.subject_id, approval.requester_id, approval.status.as_str(),
                approval.reason, approval.created_at as i64,
            ],
        )?;
        Ok(approval)
    }

    pub fn get(&self, id: &str) -> Result<Option<Approval>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, request_type, subject_id, requester_id, status,
                    reason, decided_by, decided_at, created_at
             FROM approvals WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_approval(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str, status_filter: Option<&str>) -> Result<Vec<Approval>> {
        let (sql, use_status) = if status_filter.is_some() {
            ("SELECT id, company_id, request_type, subject_id, requester_id, status,
                     reason, decided_by, decided_at, created_at
              FROM approvals WHERE company_id = ?1 AND status = ?2 ORDER BY created_at DESC", true)
        } else {
            ("SELECT id, company_id, request_type, subject_id, requester_id, status,
                     reason, decided_by, decided_at, created_at
              FROM approvals WHERE company_id = ?1 ORDER BY created_at DESC", false)
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = if use_status {
            stmt.query_map(params![company_id, status_filter.unwrap()], |row| row_to_approval(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![company_id], |row| row_to_approval(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    /// Decide on an approval request (approve or reject).
    pub fn decide(
        &self,
        id: &str,
        approved: bool,
        decided_by: &str,
    ) -> Result<Approval> {
        let approval = self.get(id)?.context("approval not found")?;
        if approval.status != ApprovalStatus::Pending {
            return Err(anyhow!("Approval is already {} — cannot re-decide", approval.status.as_str()));
        }
        let new_status = if approved { ApprovalStatus::Approved } else { ApprovalStatus::Rejected };
        let now = now_ms();
        self.conn.execute(
            "UPDATE approvals SET status = ?1, decided_by = ?2, decided_at = ?3 WHERE id = ?4",
            params![new_status.as_str(), decided_by, now as i64, id],
        )?;
        self.get(id)?.context("approval not found after decision")
    }

    /// Cancel a pending approval request (by requester).
    pub fn cancel(&self, id: &str) -> Result<Approval> {
        let approval = self.get(id)?.context("approval not found")?;
        if approval.status != ApprovalStatus::Pending {
            return Err(anyhow!("Only pending approvals can be cancelled"));
        }
        self.conn.execute(
            "UPDATE approvals SET status = 'cancelled' WHERE id = ?1",
            params![id],
        )?;
        self.get(id)?.context("approval not found after cancel")
    }

    /// Count pending approvals for a company.
    pub fn pending_count(&self, company_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM approvals WHERE company_id = ?1 AND status = 'pending'",
            params![company_id],
            |r| r.get(0),
        )?;
        Ok(count)
    }
}

// ── Row helper ────────────────────────────────────────────────────────────────

fn row_to_approval(row: &rusqlite::Row) -> rusqlite::Result<Approval> {
    Ok(Approval {
        id: row.get(0)?,
        company_id: row.get(1)?,
        request_type: ApprovalRequestType::from_str(&row.get::<_, String>(2)?),
        subject_id: row.get(3)?,
        requester_id: row.get(4)?,
        status: ApprovalStatus::from_str(&row.get::<_, String>(5)?),
        reason: row.get(6)?,
        decided_by: row.get(7)?,
        decided_at: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
        created_at: row.get::<_, i64>(9)? as u64,
    })
}

// ── Display helpers ───────────────────────────────────────────────────────────

impl Approval {
    pub fn summary_line(&self) -> String {
        let status_icon = match self.status {
            ApprovalStatus::Pending => "⏳",
            ApprovalStatus::Approved => "✓",
            ApprovalStatus::Rejected => "✗",
            ApprovalStatus::Cancelled => "○",
        };
        let decided = match (&self.decided_by, &self.decided_at) {
            (Some(by), Some(_)) => format!(" decided by:{}", &by[..8.min(by.len())]),
            _ => String::new(),
        };
        format!(
            "{} [{}] {} → subject:{}{}  [{}]",
            status_icon,
            self.request_type.as_str(),
            self.status.as_str(),
            &self.subject_id[..8.min(self.subject_id.len())],
            decided,
            &self.id[..8.min(self.id.len())]
        )
    }
}

// ── Policy integration helpers ────────────────────────────────────────────────

/// Company resource kinds for policy_engine checks.
pub mod company_policy {
    /// Resource kinds for company entities.
    pub const RES_COMPANY: &str = "company";
    pub const RES_AGENT: &str = "company_agent";
    pub const RES_TASK: &str = "company_task";
    pub const RES_GOAL: &str = "company_goal";
    pub const RES_APPROVAL: &str = "company_approval";
    pub const RES_BUDGET: &str = "company_budget";
    pub const RES_SECRET: &str = "company_secret";

    /// Default roles for company agents.
    pub const ROLE_CEO: &str = "company_ceo";
    pub const ROLE_MANAGER: &str = "company_manager";
    pub const ROLE_AGENT: &str = "company_agent";

    /// Actions.
    pub const ACTION_APPROVE: &str = "approve";
    pub const ACTION_REJECT: &str = "reject";
    pub const ACTION_REQUEST: &str = "request";
    pub const ACTION_READ: &str = "read";
    pub const ACTION_WRITE: &str = "write";
    pub const ACTION_DELETE: &str = "delete";
}
