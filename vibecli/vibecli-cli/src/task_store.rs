//! Task store — backs VibeX's `/api/tasks` CRUD (VX-112).
//!
//! VibeX frames every code-changing interaction as a *task* with a lifecycle
//! state, a branch, and a worktree. This is a thin SQLite store living
//! alongside the session DB (`~/.vibecli/sessions.db`), modeled on
//! `session_store.rs` — plain rusqlite, no encryption (tasks are not secrets).
//!
//! A task is distinct from a session: a session is the conversation/agent run;
//! a task is the unit-of-work wrapper VibeX shows as a card, carrying status,
//! branch, worktree path, and (later) diff/cost. A task references its
//! `session_id` once the agent run starts.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Lifecycle state of a task. Mirrors the VibeX state machine
/// (Draft → Queued → Running → Reviewing → Completed / Failed). Stored as the
/// lowercase string in the `status` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Draft,
    Queued,
    Running,
    Reviewing,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Draft => "draft",
            TaskStatus::Queued => "queued",
            TaskStatus::Running => "running",
            TaskStatus::Reviewing => "reviewing",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> TaskStatus {
        match s {
            "queued" => TaskStatus::Queued,
            "running" => TaskStatus::Running,
            "reviewing" => TaskStatus::Reviewing,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Draft,
        }
    }
}

/// A row in the `tasks` table — the wire shape returned by `/api/tasks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub provider: String,
    pub model: String,
    /// Branch created for this task (empty until a worktree is spawned).
    pub branch: String,
    /// Worktree path on disk (empty until spawned).
    pub worktree_path: String,
    /// The session/agent run id, once started (empty while Draft/Queued).
    pub session_id: String,
    /// Source repo the task operates on.
    pub project_path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct TaskStore {
    conn: Connection,
}

impl TaskStore {
    /// Open (or create) the task database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {:?}", parent))?;
        }
        let conn = Connection::open(path).with_context(|| format!("open SQLite at {:?}", path))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let store = Self { conn };
        store.create_schema()?;
        Ok(store)
    }

    /// Open from the default path: `~/.vibecli/sessions.db` (shared with the
    /// session store — the `tasks` table coexists with `sessions`).
    pub fn open_default() -> Result<Self> {
        Self::open(default_db_path())
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id            TEXT PRIMARY KEY,
                title         TEXT NOT NULL,
                status        TEXT NOT NULL DEFAULT 'draft',
                provider      TEXT NOT NULL DEFAULT '',
                model         TEXT NOT NULL DEFAULT '',
                branch        TEXT NOT NULL DEFAULT '',
                worktree_path TEXT NOT NULL DEFAULT '',
                session_id    TEXT NOT NULL DEFAULT '',
                project_path  TEXT NOT NULL DEFAULT '',
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_tasks_status  ON tasks(status);
            "#,
        )?;
        Ok(())
    }

    /// Insert a new task. `id` should be a fresh uuid-ish string from the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        id: &str,
        title: &str,
        status: TaskStatus,
        provider: &str,
        model: &str,
        project_path: &str,
        now: i64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tasks (id, title, status, provider, model, branch, worktree_path, session_id, project_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, '', '', '', ?6, ?7, ?7)",
            params![id, title, status.as_str(), provider, model, project_path, now],
        )?;
        Ok(())
    }

    /// Update the lifecycle status of a task.
    pub fn set_status(&self, id: &str, status: TaskStatus, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), now],
        )?;
        Ok(())
    }

    /// Attach the branch + worktree path once a worktree is spawned (VX-113).
    pub fn set_worktree(&self, id: &str, branch: &str, worktree_path: &str, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET branch = ?2, worktree_path = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, branch, worktree_path, now],
        )?;
        Ok(())
    }

    /// Link the agent run's session id once the agent starts.
    pub fn set_session(&self, id: &str, session_id: &str, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET session_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, session_id, now],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<TaskRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, provider, model, branch, worktree_path, session_id, project_path, created_at, updated_at
             FROM tasks WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(row_to_task(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<TaskRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, provider, model, branch, worktree_path, session_id, project_path, created_at, updated_at
             FROM tasks ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            row_to_task(r).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            )))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

fn row_to_task(r: &rusqlite::Row) -> Result<TaskRow> {
    Ok(TaskRow {
        id: r.get(0)?,
        title: r.get(1)?,
        status: r.get(2)?,
        provider: r.get(3)?,
        model: r.get(4)?,
        branch: r.get(5)?,
        worktree_path: r.get(6)?,
        session_id: r.get(7)?,
        project_path: r.get(8)?,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
    })
}

/// Default DB path: `~/.vibecli/sessions.db` (shared with the session store).
pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("sessions.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> i64 {
        1_700_000_000
    }

    #[test]
    fn insert_get_list_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks-{}", std::process::id()));
        let db = dir.join("t.db");
        let store = TaskStore::open(&db).unwrap();

        store
            .insert("t1", "fix the auth timeout", TaskStatus::Queued, "ollama", "qwen3", "/repo", now())
            .unwrap();
        let got = store.get("t1").unwrap().unwrap();
        assert_eq!(got.title, "fix the auth timeout");
        assert_eq!(got.status, "queued");
        assert_eq!(got.provider, "ollama");
        assert_eq!(got.project_path, "/repo");
        assert!(got.branch.is_empty());

        let list = store.list(10).unwrap();
        assert_eq!(list.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_and_worktree_updates() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks2-{}", std::process::id()));
        let db = dir.join("t.db");
        let store = TaskStore::open(&db).unwrap();

        store
            .insert("t1", "task", TaskStatus::Draft, "", "", "/repo", now())
            .unwrap();
        store.set_status("t1", TaskStatus::Running, now() + 1).unwrap();
        store
            .set_worktree("t1", "task/t1-fix", "/tmp/wt/t1", now() + 2)
            .unwrap();
        store.set_session("t1", "sess-abc", now() + 3).unwrap();

        let got = store.get("t1").unwrap().unwrap();
        assert_eq!(got.status, "running");
        assert_eq!(got.branch, "task/t1-fix");
        assert_eq!(got.worktree_path, "/tmp/wt/t1");
        assert_eq!(got.session_id, "sess-abc");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_str_roundtrip() {
        for s in [
            TaskStatus::Draft,
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Reviewing,
            TaskStatus::Completed,
            TaskStatus::Failed,
        ] {
            assert_eq!(TaskStatus::from_str(s.as_str()), s);
        }
    }
}
