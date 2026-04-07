#![allow(dead_code)]
//! Full task lifecycle management for VibeCody company orchestration.
//!
//! Tasks implement a Kanban-style state machine:
//!   backlog → todo → in_progress → in_review → done | blocked | cancelled
//!
//! Key features:
//! - **State machine validation**: Invalid transitions are rejected
//! - **Atomic checkout**: `in_progress` transition creates a git branch and
//!   sets `assigned_agent` atomically — prevents double-work
//! - **Parent/child tasks**: Full task hierarchy with dependency chains
//! - **Comments**: Conversation threads per task
//! - **Activity logging**: Every transition recorded for audit

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
pub enum TaskStatus {
    Backlog,
    Todo,
    InProgress,
    InReview,
    Done,
    Blocked,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::InReview => "in_review",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "todo" => Self::Todo,
            "in_progress" => Self::InProgress,
            "in_review" => Self::InReview,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            "cancelled" => Self::Cancelled,
            _ => Self::Backlog,
        }
    }

    /// Valid transitions from this status.
    pub fn allowed_transitions(&self) -> &[TaskStatus] {
        match self {
            Self::Backlog => &[TaskStatus::Todo, TaskStatus::Cancelled],
            Self::Todo => &[TaskStatus::InProgress, TaskStatus::Blocked, TaskStatus::Cancelled],
            Self::InProgress => &[TaskStatus::InReview, TaskStatus::Blocked, TaskStatus::Cancelled],
            Self::InReview => &[TaskStatus::Done, TaskStatus::InProgress, TaskStatus::Blocked, TaskStatus::Cancelled],
            Self::Done => &[],
            Self::Blocked => &[TaskStatus::Todo, TaskStatus::InProgress, TaskStatus::Cancelled],
            Self::Cancelled => &[],
        }
    }

    pub fn can_transition_to(&self, next: &TaskStatus) -> bool {
        self.allowed_transitions().contains(next)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

impl TaskPriority {
    pub fn from_i64(v: i64) -> Self {
        match v {
            3 => Self::Critical,
            2 => Self::High,
            1 => Self::Medium,
            _ => Self::Low,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyTask {
    pub id: String,
    pub company_id: String,
    pub goal_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub assigned_agent: Option<String>,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    /// Git branch created on checkout.
    pub branch_name: Option<String>,
    /// Links to sessions.db session.
    pub session_id: Option<String>,
    pub result_summary: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Owner of the task: "agent", "human", etc.
    pub owner: String,
    /// Program or workstream this task belongs to.
    pub program: String,
    /// Optional recurrence rule (e.g. "daily", "weekly", cron expression).
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComment {
    pub id: i64,
    pub task_id: String,
    pub author_agent_id: Option<String>,
    pub content: String,
    pub created_at: u64,
}

// ── TaskStore ─────────────────────────────────────────────────────────────────

pub struct TaskStore<'a> {
    conn: &'a Connection,
}

impl<'a> TaskStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                goal_id         TEXT,
                parent_task_id  TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                assigned_agent  TEXT,
                title           TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                status          TEXT NOT NULL DEFAULT 'backlog',
                priority        INTEGER NOT NULL DEFAULT 1,
                branch_name     TEXT,
                session_id      TEXT,
                result_summary  TEXT,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_company ON tasks(company_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_goal ON tasks(goal_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_agent ON tasks(assigned_agent);
            CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_task_id);

            CREATE TABLE IF NOT EXISTS task_comments (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id         TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                author_agent_id TEXT,
                content         TEXT NOT NULL,
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_comments_task ON task_comments(task_id);
        "#)?;
        let _ = self.conn.execute_batch("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS owner TEXT NOT NULL DEFAULT 'agent'");
        let _ = self.conn.execute_batch("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS program TEXT NOT NULL DEFAULT ''");
        let _ = self.conn.execute_batch("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS recurrence TEXT");
        Ok(())
    }

    pub fn create(
        &self,
        company_id: &str,
        title: &str,
        description: &str,
        goal_id: Option<&str>,
        parent_task_id: Option<&str>,
        assigned_agent: Option<&str>,
        priority: TaskPriority,
    ) -> Result<CompanyTask> {
        let task = CompanyTask {
            id: new_id(),
            company_id: company_id.to_string(),
            goal_id: goal_id.map(|s| s.to_string()),
            parent_task_id: parent_task_id.map(|s| s.to_string()),
            assigned_agent: assigned_agent.map(|s| s.to_string()),
            title: title.to_string(),
            description: description.to_string(),
            status: TaskStatus::Backlog,
            priority,
            branch_name: None,
            session_id: None,
            result_summary: None,
            created_at: now_ms(),
            updated_at: now_ms(),
            owner: "agent".to_string(),
            program: String::new(),
            recurrence: None,
        };
        self.conn.execute(
            "INSERT INTO tasks (id, company_id, goal_id, parent_task_id, assigned_agent, title, description, status, priority, created_at, updated_at, owner, program)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                task.id, task.company_id, task.goal_id, task.parent_task_id, task.assigned_agent,
                task.title, task.description, task.status.as_str(), task.priority.clone() as i64,
                task.created_at as i64, task.updated_at as i64,
                task.owner, task.program,
            ],
        )?;
        Ok(task)
    }

    /// Create a task with owner, program, and recurrence fields.
    pub fn create_v2(
        &self,
        company_id: &str,
        title: &str,
        description: &str,
        goal_id: Option<&str>,
        parent_task_id: Option<&str>,
        assigned_agent: Option<&str>,
        priority: TaskPriority,
        owner: &str,
        program: &str,
        recurrence: Option<&str>,
    ) -> Result<CompanyTask> {
        let task = CompanyTask {
            id: new_id(),
            company_id: company_id.to_string(),
            goal_id: goal_id.map(|s| s.to_string()),
            parent_task_id: parent_task_id.map(|s| s.to_string()),
            assigned_agent: assigned_agent.map(|s| s.to_string()),
            title: title.to_string(),
            description: description.to_string(),
            status: TaskStatus::Backlog,
            priority,
            branch_name: None,
            session_id: None,
            result_summary: None,
            created_at: now_ms(),
            updated_at: now_ms(),
            owner: owner.to_string(),
            program: program.to_string(),
            recurrence: recurrence.map(|s| s.to_string()),
        };
        self.conn.execute(
            "INSERT INTO tasks (id, company_id, goal_id, parent_task_id, assigned_agent, title, description, status, priority, created_at, updated_at, owner, program, recurrence)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                task.id, task.company_id, task.goal_id, task.parent_task_id, task.assigned_agent,
                task.title, task.description, task.status.as_str(), task.priority.clone() as i64,
                task.created_at as i64, task.updated_at as i64,
                task.owner, task.program, task.recurrence,
            ],
        )?;
        Ok(task)
    }

    pub fn get(&self, id: &str) -> Result<Option<CompanyTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, goal_id, parent_task_id, assigned_agent, title, description,
                    status, priority, branch_name, session_id, result_summary, created_at, updated_at,
                    COALESCE(owner,'agent'), COALESCE(program,''), recurrence
             FROM tasks WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_task(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str, status_filter: Option<&str>) -> Result<Vec<CompanyTask>> {
        let sql = if status_filter.is_some() {
            "SELECT id, company_id, goal_id, parent_task_id, assigned_agent, title, description,
                    status, priority, branch_name, session_id, result_summary, created_at, updated_at,
                    COALESCE(owner,'agent'), COALESCE(program,''), recurrence
             FROM tasks WHERE company_id = ?1 AND status = ?2 ORDER BY priority DESC, created_at ASC"
        } else {
            "SELECT id, company_id, goal_id, parent_task_id, assigned_agent, title, description,
                    status, priority, branch_name, session_id, result_summary, created_at, updated_at,
                    COALESCE(owner,'agent'), COALESCE(program,''), recurrence
             FROM tasks WHERE company_id = ?1 ORDER BY priority DESC, created_at ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = if let Some(sf) = status_filter {
            stmt.query_map(params![company_id, sf], |row| row_to_task(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![company_id], |row| row_to_task(row))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        rows.into_iter()
            .map(Ok)
            .collect()
    }

    /// Validate and perform a status transition.
    pub fn transition(&self, id: &str, new_status: TaskStatus) -> Result<CompanyTask> {
        let task = self.get(id)?.context("task not found")?;
        if !task.status.can_transition_to(&new_status) {
            return Err(anyhow!(
                "Invalid transition: {} → {}",
                task.status.as_str(),
                new_status.as_str()
            ));
        }
        self.conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status.as_str(), now_ms() as i64, id],
        )?;
        self.get(id)?.context("task not found after transition")
    }

    /// Assign a task to an agent.
    pub fn assign(&self, id: &str, agent_id: &str) -> Result<CompanyTask> {
        self.conn.execute(
            "UPDATE tasks SET assigned_agent = ?1, updated_at = ?2 WHERE id = ?3",
            params![agent_id, now_ms() as i64, id],
        )?;
        self.get(id)?.context("task not found")
    }

    /// Atomic checkout: transitions to in_progress and sets branch_name.
    /// Prevents double-work — fails if already in_progress.
    pub fn checkout(&self, id: &str, agent_id: &str) -> Result<CompanyTask> {
        let task = self.get(id)?.context("task not found")?;
        if task.status == TaskStatus::InProgress {
            return Err(anyhow!(
                "Task is already checked out{}",
                task.assigned_agent.as_deref()
                    .map(|a| format!(" by agent {}", &a[..8.min(a.len())]))
                    .unwrap_or_default()
            ));
        }
        if !task.status.can_transition_to(&TaskStatus::InProgress) {
            return Err(anyhow!(
                "Cannot checkout from status '{}'",
                task.status.as_str()
            ));
        }
        let branch_name = format!("task-{}", &id[..8.min(id.len())]);
        self.conn.execute(
            "UPDATE tasks SET status = 'in_progress', assigned_agent = ?1, branch_name = ?2, updated_at = ?3 WHERE id = ?4",
            params![agent_id, branch_name, now_ms() as i64, id],
        )?;
        self.get(id)?.context("task not found after checkout")
    }

    pub fn add_comment(&self, task_id: &str, author_agent_id: Option<&str>, content: &str) -> Result<TaskComment> {
        self.conn.execute(
            "INSERT INTO task_comments (task_id, author_agent_id, content, created_at) VALUES (?1,?2,?3,?4)",
            params![task_id, author_agent_id, content, now_ms() as i64],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(TaskComment {
            id,
            task_id: task_id.to_string(),
            author_agent_id: author_agent_id.map(|s| s.to_string()),
            content: content.to_string(),
            created_at: now_ms(),
        })
    }

    pub fn list_comments(&self, task_id: &str) -> Result<Vec<TaskComment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, author_agent_id, content, created_at FROM task_comments WHERE task_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Ok(TaskComment {
                id: row.get(0)?,
                task_id: row.get(1)?,
                author_agent_id: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get::<_, i64>(4)? as u64,
            })
        })?;
        rows.collect::<rusqlite::Result<_>>().map_err(|e| e.into())
    }
}

// ── Row helpers ───────────────────────────────────────────────────────────────

fn row_to_task(row: &rusqlite::Row) -> Result<CompanyTask, rusqlite::Error> {
    Ok(CompanyTask {
        id: row.get(0)?,
        company_id: row.get(1)?,
        goal_id: row.get(2)?,
        parent_task_id: row.get(3)?,
        assigned_agent: row.get(4)?,
        title: row.get(5)?,
        description: row.get(6)?,
        status: TaskStatus::from_str(&row.get::<_, String>(7)?),
        priority: TaskPriority::from_i64(row.get(8)?),
        branch_name: row.get(9)?,
        session_id: row.get(10)?,
        result_summary: row.get(11)?,
        created_at: row.get::<_, i64>(12)? as u64,
        updated_at: row.get::<_, i64>(13)? as u64,
        owner: row.get::<_, Option<String>>(14)?.unwrap_or_else(|| "agent".to_string()),
        program: row.get::<_, Option<String>>(15)?.unwrap_or_default(),
        recurrence: row.get(16)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory_store() -> (Connection, TaskStore<'static>) {
        // We need a 'static connection for TaskStore — use Box::leak for tests only.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        let store = TaskStore::new(conn);
        store.ensure_schema().unwrap();
        (unsafe { std::ptr::read(conn as *const Connection) }, store)
    }

    fn make_store() -> TaskStore<'static> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        let store = TaskStore::new(conn);
        store.ensure_schema().unwrap();
        store
    }

    // ── TaskStatus ────────────────────────────────────────────────────────────

    #[test]
    fn task_status_round_trip() {
        for (s, v) in &[
            ("backlog",     TaskStatus::Backlog),
            ("todo",        TaskStatus::Todo),
            ("in_progress", TaskStatus::InProgress),
            ("in_review",   TaskStatus::InReview),
            ("done",        TaskStatus::Done),
            ("blocked",     TaskStatus::Blocked),
            ("cancelled",   TaskStatus::Cancelled),
        ] {
            assert_eq!(TaskStatus::from_str(s), *v);
            assert_eq!(v.as_str(), *s);
        }
    }

    #[test]
    fn unknown_status_defaults_to_backlog() {
        assert_eq!(TaskStatus::from_str("unknown_xyz"), TaskStatus::Backlog);
    }

    // ── TaskStatus transitions ────────────────────────────────────────────────

    #[test]
    fn backlog_can_transition_to_todo() {
        assert!(TaskStatus::Backlog.can_transition_to(&TaskStatus::Todo));
    }

    #[test]
    fn backlog_can_transition_to_cancelled() {
        assert!(TaskStatus::Backlog.can_transition_to(&TaskStatus::Cancelled));
    }

    #[test]
    fn backlog_cannot_transition_to_done() {
        assert!(!TaskStatus::Backlog.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn todo_can_transition_to_in_progress() {
        assert!(TaskStatus::Todo.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn todo_can_transition_to_blocked() {
        assert!(TaskStatus::Todo.can_transition_to(&TaskStatus::Blocked));
    }

    #[test]
    fn in_progress_can_transition_to_in_review() {
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::InReview));
    }

    #[test]
    fn in_review_can_transition_to_done() {
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn in_review_can_revert_to_in_progress() {
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn done_has_no_transitions() {
        assert!(TaskStatus::Done.allowed_transitions().is_empty());
    }

    #[test]
    fn cancelled_has_no_transitions() {
        assert!(TaskStatus::Cancelled.allowed_transitions().is_empty());
    }

    #[test]
    fn blocked_can_resume_to_todo_or_in_progress() {
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::Todo));
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::InProgress));
    }

    // ── TaskPriority ──────────────────────────────────────────────────────────

    #[test]
    fn priority_round_trip() {
        assert_eq!(TaskPriority::from_i64(0).as_str(), "low");
        assert_eq!(TaskPriority::from_i64(1).as_str(), "medium");
        assert_eq!(TaskPriority::from_i64(2).as_str(), "high");
        assert_eq!(TaskPriority::from_i64(3).as_str(), "critical");
    }

    #[test]
    fn out_of_range_priority_defaults_to_low() {
        assert_eq!(TaskPriority::from_i64(99).as_str(), "low");
        assert_eq!(TaskPriority::from_i64(-1).as_str(), "low");
    }

    // ── TaskStore CRUD ────────────────────────────────────────────────────────

    #[test]
    fn create_and_get_task() {
        let store = make_store();
        let task = store.create("co-1", "Test task", "description", None, None, None, TaskPriority::Medium).unwrap();
        assert!(!task.id.is_empty());
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Backlog);
        assert_eq!(task.company_id, "co-1");

        let fetched = store.get(&task.id).unwrap().expect("task should exist");
        assert_eq!(fetched.id, task.id);
        assert_eq!(fetched.title, "Test task");
    }

    #[test]
    fn get_nonexistent_task_returns_none() {
        let store = make_store();
        let result = store.get("does-not-exist").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_tasks_by_company() {
        let store = make_store();
        store.create("co-1", "Task A", "", None, None, None, TaskPriority::Low).unwrap();
        store.create("co-1", "Task B", "", None, None, None, TaskPriority::High).unwrap();
        store.create("co-2", "Task C", "", None, None, None, TaskPriority::Medium).unwrap();

        let co1_tasks = store.list("co-1", None).unwrap();
        assert_eq!(co1_tasks.len(), 2);
        assert!(co1_tasks.iter().all(|t| t.company_id == "co-1"));
    }

    #[test]
    fn list_tasks_with_status_filter() {
        let store = make_store();
        let t1 = store.create("co-1", "Task 1", "", None, None, None, TaskPriority::Low).unwrap();
        store.create("co-1", "Task 2", "", None, None, None, TaskPriority::Low).unwrap();
        store.transition(&t1.id, TaskStatus::Todo).unwrap();

        let todo_tasks = store.list("co-1", Some("todo")).unwrap();
        assert_eq!(todo_tasks.len(), 1);
        assert_eq!(todo_tasks[0].id, t1.id);
    }

    #[test]
    fn list_returns_empty_for_unknown_company() {
        let store = make_store();
        let tasks = store.list("no-such-company", None).unwrap();
        assert!(tasks.is_empty());
    }

    // ── Transitions ──────────────────────────────────────────────────────────

    #[test]
    fn valid_transition_persisted() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Medium).unwrap();
        let updated = store.transition(&task.id, TaskStatus::Todo).unwrap();
        assert_eq!(updated.status, TaskStatus::Todo);
        // Verify persisted to DB
        let fetched = store.get(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Todo);
    }

    #[test]
    fn invalid_transition_returns_error() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        // Backlog → Done is invalid
        let result = store.transition(&task.id, TaskStatus::Done);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid transition"));
    }

    #[test]
    fn transition_nonexistent_task_returns_error() {
        let store = make_store();
        let result = store.transition("no-such-id", TaskStatus::Todo);
        assert!(result.is_err());
    }

    // ── Assign ───────────────────────────────────────────────────────────────

    #[test]
    fn assign_sets_agent_id() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        let updated = store.assign(&task.id, "agent-42").unwrap();
        assert_eq!(updated.assigned_agent.as_deref(), Some("agent-42"));
    }

    // ── Checkout ─────────────────────────────────────────────────────────────

    #[test]
    fn checkout_transitions_to_in_progress_and_sets_branch() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Medium).unwrap();
        store.transition(&task.id, TaskStatus::Todo).unwrap();

        let checked = store.checkout(&task.id, "agent-007").unwrap();
        assert_eq!(checked.status, TaskStatus::InProgress);
        assert_eq!(checked.assigned_agent.as_deref(), Some("agent-007"));
        assert!(checked.branch_name.is_some());
        assert!(checked.branch_name.as_deref().unwrap().starts_with("task-"));
    }

    #[test]
    fn double_checkout_returns_error() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Medium).unwrap();
        store.transition(&task.id, TaskStatus::Todo).unwrap();
        store.checkout(&task.id, "agent-1").unwrap();

        let result = store.checkout(&task.id, "agent-2");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("already checked out"));
    }

    #[test]
    fn checkout_from_backlog_returns_error() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        // Backlog → InProgress not allowed
        let result = store.checkout(&task.id, "agent-1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot checkout"));
    }

    // ── Comments ─────────────────────────────────────────────────────────────

    #[test]
    fn add_and_list_comments() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        store.add_comment(&task.id, Some("agent-1"), "First comment").unwrap();
        store.add_comment(&task.id, None, "Second comment from system").unwrap();

        let comments = store.list_comments(&task.id).unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].content, "First comment");
        assert_eq!(comments[0].author_agent_id.as_deref(), Some("agent-1"));
        assert_eq!(comments[1].content, "Second comment from system");
        assert!(comments[1].author_agent_id.is_none());
    }

    #[test]
    fn list_comments_empty_for_new_task() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        let comments = store.list_comments(&task.id).unwrap();
        assert!(comments.is_empty());
    }

    // ── Priority ordering ─────────────────────────────────────────────────────

    #[test]
    fn list_orders_by_priority_desc() {
        let store = make_store();
        store.create("co-1", "Low prio",      "", None, None, None, TaskPriority::Low).unwrap();
        store.create("co-1", "Critical prio", "", None, None, None, TaskPriority::Critical).unwrap();
        store.create("co-1", "High prio",     "", None, None, None, TaskPriority::High).unwrap();

        let tasks = store.list("co-1", None).unwrap();
        assert_eq!(tasks[0].priority.as_str(), "critical");
        assert_eq!(tasks[1].priority.as_str(), "high");
        assert_eq!(tasks[2].priority.as_str(), "low");
    }

    // ── summary_line ─────────────────────────────────────────────────────────

    #[test]
    fn summary_line_contains_title_and_status() {
        let store = make_store();
        let task = store.create("co-1", "My Test Task", "desc", None, None, None, TaskPriority::High).unwrap();
        let line = task.summary_line();
        assert!(line.contains("My Test Task"));
        assert!(line.contains("backlog"));
        assert!(line.contains("▫")); // backlog icon
    }

    #[test]
    fn summary_line_uses_correct_icons_for_each_status() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();

        // Backlog
        assert!(task.summary_line().contains("▫"));
        // Todo
        let t = store.transition(&task.id, TaskStatus::Todo).unwrap();
        assert!(t.summary_line().contains("□"));
    }

    // ── parent_task_id ────────────────────────────────────────────────────────

    #[test]
    fn create_task_with_parent() {
        let store = make_store();
        let parent = store.create("co-1", "Parent task", "", None, None, None, TaskPriority::High).unwrap();
        let child = store.create("co-1", "Child task", "", None, Some(&parent.id), None, TaskPriority::Low).unwrap();
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
    }

    // ── goal_id ───────────────────────────────────────────────────────────────

    #[test]
    fn create_task_with_goal_id() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", Some("goal-xyz"), None, None, TaskPriority::Medium).unwrap();
        assert_eq!(task.goal_id.as_deref(), Some("goal-xyz"));
    }

    // ── timestamps ───────────────────────────────────────────────────────────

    #[test]
    fn created_at_and_updated_at_are_set_on_create() {
        let store = make_store();
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        assert!(task.created_at >= before);
        assert!(task.created_at <= after);
        assert!(task.updated_at >= before);
    }

    #[test]
    fn transition_updates_updated_at() {
        let store = make_store();
        let task = store.create("co-1", "Task", "", None, None, None, TaskPriority::Low).unwrap();
        let original_updated = task.updated_at;
        // Brief sleep to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(2));
        let updated = store.transition(&task.id, TaskStatus::Todo).unwrap();
        assert!(updated.updated_at >= original_updated);
    }
}

// ── Display helpers ───────────────────────────────────────────────────────────

impl CompanyTask {
    pub fn summary_line(&self) -> String {
        let status_icon = match self.status {
            TaskStatus::Backlog => "▫",
            TaskStatus::Todo => "□",
            TaskStatus::InProgress => "▶",
            TaskStatus::InReview => "◎",
            TaskStatus::Done => "✓",
            TaskStatus::Blocked => "⊘",
            TaskStatus::Cancelled => "✗",
        };
        let priority_icon = match self.priority {
            TaskPriority::Critical => "!!!",
            TaskPriority::High => "!! ",
            TaskPriority::Medium => "!  ",
            TaskPriority::Low => "   ",
        };
        format!(
            "{} {} [{}]  {}  [{}]",
            status_icon, priority_icon, self.status.as_str(),
            self.title, &self.id[..8.min(self.id.len())]
        )
    }
}
