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
    /// Lifecycle timestamps (see docs/design/worktree-lifecycle/). All three
    /// nullable; the derived state is Active when all are NULL. Archived keeps
    /// the branch but reclaims the worktree dir; Trashed is a soft-delete with a
    /// grace window; Reaped means the reaper has reclaimed the worktree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trashed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaped_at: Option<i64>,
}

/// The `tasks` column list, in the order `row_to_task` reads them. Kept in one
/// place so every SELECT stays in sync with the row decoder.
const TASK_COLS: &str = "id, title, status, provider, model, branch, worktree_path, \
     session_id, project_path, created_at, updated_at, archived_at, trashed_at, reaped_at";

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
        self.migrate_lifecycle_columns()?;
        Ok(())
    }

    /// Additive, idempotent migration for the worktree-lifecycle columns
    /// (docs/design/worktree-lifecycle/). Existing rows get NULL for all three →
    /// derived state Active, which is correct. Guarded by `PRAGMA table_info`
    /// so re-running on an already-migrated DB is a no-op.
    fn migrate_lifecycle_columns(&self) -> Result<()> {
        let mut have: std::collections::HashSet<String> = std::collections::HashSet::new();
        {
            let mut stmt = self.conn.prepare("PRAGMA table_info(tasks)")?;
            let cols = stmt.query_map([], |r| r.get::<_, String>(1))?;
            for c in cols {
                have.insert(c?);
            }
        }
        for col in ["archived_at", "trashed_at", "reaped_at"] {
            if !have.contains(col) {
                // Column names are a fixed allow-list above, not user input.
                self.conn
                    .execute(&format!("ALTER TABLE tasks ADD COLUMN {col} INTEGER"), [])?;
            }
        }
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tasks_trashed  ON tasks(trashed_at);
             CREATE INDEX IF NOT EXISTS idx_tasks_archived ON tasks(archived_at);",
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
    pub fn set_worktree(
        &self,
        id: &str,
        branch: &str,
        worktree_path: &str,
        now: i64,
    ) -> Result<()> {
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

    /// Soft-delete: move a task to the Trashed state (sets `trashed_at`). The
    /// worktree is left untouched — the reaper reclaims it after the grace
    /// window. Reversible via [`restore`]. This is the default delete path so an
    /// accidental delete never loses work. Returns `true` if a row was updated.
    pub fn trash(&self, id: &str, now: i64) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE tasks SET trashed_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(n > 0)
    }

    /// Archive a task: keep the branch forever, but mark it so the reaper
    /// reclaims the worktree directory (disk). Reversible via [`restore`], which
    /// re-creates the worktree from the kept branch.
    pub fn archive(&self, id: &str, now: i64) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE tasks SET archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(n > 0)
    }

    /// Restore a task to Active — clears `trashed_at`/`archived_at`/`reaped_at`.
    /// The caller re-creates the worktree from the branch if the directory was
    /// already reclaimed. Returns `true` if a row was updated.
    pub fn restore(&self, id: &str, now: i64) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE tasks SET trashed_at = NULL, archived_at = NULL, reaped_at = NULL, updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(n > 0)
    }

    /// Mark a task as reaped (worktree directory reclaimed by the reaper) and
    /// clear the now-stale `worktree_path`. The branch field is left as-is: it
    /// still records what was preserved (kept ref, `refs/trash/<id>`, or merged).
    pub fn mark_reaped(&self, id: &str, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET reaped_at = ?2, worktree_path = '', updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(())
    }

    /// Hard-delete a task row. Returns `true` if a row was removed, `false` if
    /// the id didn't exist. The caller is responsible for any worktree cleanup —
    /// this only touches the `tasks` table. Used by the reaper's permanent-purge
    /// path and by `merge` once the branch is integrated.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Trashed rows whose grace window has elapsed (`trashed_at <= before`) and
    /// that are not currently running/reviewing — i.e. safe for the reaper to
    /// reclaim. Includes rows not yet reaped only.
    pub fn reapable_trashed(&self, before: i64) -> Result<Vec<TaskRow>> {
        let sql = format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE trashed_at IS NOT NULL AND trashed_at <= ?1 AND reaped_at IS NULL
               AND status NOT IN ('running','reviewing')"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![before], map_row)?;
        collect(rows)
    }

    /// Archived rows whose worktree directory has not yet been reclaimed
    /// (`worktree_path` non-empty, not yet reaped). The reaper frees their disk
    /// while keeping the branch.
    pub fn archived_with_worktree(&self) -> Result<Vec<TaskRow>> {
        let sql = format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE archived_at IS NOT NULL AND reaped_at IS NULL AND worktree_path <> ''
               AND status NOT IN ('running','reviewing')"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_row)?;
        collect(rows)
    }

    /// Every worktree path the store currently knows about (Active, Archived, or
    /// Trashed but not yet reaped). The reaper uses this set to tell a tracked
    /// worktree from an orphan during filesystem reconciliation.
    pub fn known_worktree_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT worktree_path FROM tasks WHERE worktree_path <> ''")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Distinct project repos referenced by tasks — the set of repos whose
    /// `.vibecli/worktrees/` directories the reaper scans for orphans.
    pub fn distinct_project_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT project_path FROM tasks WHERE project_path <> ''")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Fetch one task by id, regardless of lifecycle state (so trashed/archived
    /// rows can be restored or inspected).
    pub fn get(&self, id: &str) -> Result<Option<TaskRow>> {
        let sql = format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(row_to_task(r)?))
        } else {
            Ok(None)
        }
    }

    /// List Active tasks (excludes Trashed) — the default VibeX card view. Use
    /// [`list_in_state`] for the Trash/Archive views.
    pub fn list(&self, limit: usize) -> Result<Vec<TaskRow>> {
        let sql = format!(
            "SELECT {TASK_COLS} FROM tasks
             WHERE trashed_at IS NULL
             ORDER BY created_at DESC LIMIT ?1"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], map_row)?;
        collect(rows)
    }

    /// List tasks filtered by lifecycle state for the recovery UIs.
    /// `state`: "trashed" → soft-deleted rows; "archived" → archived rows;
    /// "all" → everything; anything else → Active (same as [`list`]).
    pub fn list_in_state(&self, state: &str, limit: usize) -> Result<Vec<TaskRow>> {
        let filter = match state {
            "trashed" => "trashed_at IS NOT NULL",
            "archived" => "archived_at IS NOT NULL AND trashed_at IS NULL",
            "all" => "1=1",
            _ => "trashed_at IS NULL",
        };
        let sql = format!(
            "SELECT {TASK_COLS} FROM tasks WHERE {filter} ORDER BY created_at DESC LIMIT ?1"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], map_row)?;
        collect(rows)
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
        archived_at: r.get(11)?,
        trashed_at: r.get(12)?,
        reaped_at: r.get(13)?,
    })
}

/// `query_map` adapter — decodes a row, converting our `anyhow::Error` into the
/// `rusqlite::Error` the closure must return.
fn map_row(r: &rusqlite::Row) -> rusqlite::Result<TaskRow> {
    row_to_task(r).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )))
    })
}

/// Drain a `query_map` iterator into a `Vec`, propagating the first error.
fn collect(rows: impl Iterator<Item = rusqlite::Result<TaskRow>>) -> Result<Vec<TaskRow>> {
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
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
            .insert(
                "t1",
                "fix the auth timeout",
                TaskStatus::Queued,
                "ollama",
                "qwen3",
                "/repo",
                now(),
            )
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
        store
            .set_status("t1", TaskStatus::Running, now() + 1)
            .unwrap();
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
    fn delete_removes_row_and_reports_hit() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks-del-{}", std::process::id()));
        let db = dir.join("t.db");
        let store = TaskStore::open(&db).unwrap();

        store
            .insert("t1", "task", TaskStatus::Queued, "", "", "/repo", now())
            .unwrap();
        assert!(store.get("t1").unwrap().is_some());

        // Deleting a present row returns true and removes it.
        assert!(store.delete("t1").unwrap());
        assert!(store.get("t1").unwrap().is_none());
        assert_eq!(store.list(10).unwrap().len(), 0);

        // Deleting an absent row returns false (idempotent, no error).
        assert!(!store.delete("t1").unwrap());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trash_hides_from_list_and_restore_brings_back() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks-trash-{}", std::process::id()));
        let db = dir.join("t.db");
        let store = TaskStore::open(&db).unwrap();
        store
            .insert("t1", "task", TaskStatus::Queued, "", "", "/repo", now())
            .unwrap();

        // Trash → excluded from the default list, but still fetchable + listed
        // under the trashed filter.
        assert!(store.trash("t1", now() + 1).unwrap());
        assert_eq!(store.list(10).unwrap().len(), 0);
        assert_eq!(store.list_in_state("trashed", 10).unwrap().len(), 1);
        assert!(store.get("t1").unwrap().unwrap().trashed_at.is_some());

        // Restore → back in the active list, flags cleared.
        assert!(store.restore("t1", now() + 2).unwrap());
        assert_eq!(store.list(10).unwrap().len(), 1);
        assert!(store.get("t1").unwrap().unwrap().trashed_at.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn archive_keeps_in_list_in_state_and_reapable_respects_grace() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks-arch-{}", std::process::id()));
        let db = dir.join("t.db");
        let store = TaskStore::open(&db).unwrap();
        store
            .insert("t1", "task", TaskStatus::Completed, "", "", "/repo", now())
            .unwrap();
        store
            .set_worktree("t1", "task/t1-x", "/tmp/wt/t1", now())
            .unwrap();

        store.archive("t1", now()).unwrap();
        assert_eq!(store.list_in_state("archived", 10).unwrap().len(), 1);
        assert_eq!(store.archived_with_worktree().unwrap().len(), 1);

        // A running task is never reapable even if trashed long ago.
        store
            .insert("t2", "live", TaskStatus::Running, "", "", "/repo", now())
            .unwrap();
        store.trash("t2", 1).unwrap();
        store.trash("t1", 1).unwrap(); // t1 is Completed → eligible
        let reapable = store.reapable_trashed(now()).unwrap();
        assert_eq!(reapable.len(), 1);
        assert_eq!(reapable[0].id, "t1");

        // Before the grace cutoff → nothing reapable.
        assert_eq!(store.reapable_trashed(0).unwrap().len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("vibex-tasks-mig-{}", std::process::id()));
        let db = dir.join("t.db");
        // Open twice — the second open re-runs create_schema/migrate and must
        // not error on already-present columns.
        {
            let store = TaskStore::open(&db).unwrap();
            store
                .insert("t1", "task", TaskStatus::Queued, "", "", "/repo", now())
                .unwrap();
        }
        let store = TaskStore::open(&db).unwrap();
        let row = store.get("t1").unwrap().unwrap();
        assert!(row.archived_at.is_none() && row.trashed_at.is_none() && row.reaped_at.is_none());
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
