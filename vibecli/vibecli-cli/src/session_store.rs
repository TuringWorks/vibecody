#![allow(dead_code)]
//! SQLite-backed session store for VibeCLI agent sessions.
//!
//! Stores sessions, their messages, and tool-call steps in a SQLite database
//! at `~/.vibecli/sessions.db`. Enables fast full-text search, filtering,
//! and the web session viewer served by the daemon.
//!
//! The store is written to **in parallel** with the existing JSONL trace files,
//! so existing trace viewing/resume functionality is unaffected.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub task: String,
    pub provider: String,
    pub model: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub status: String, // "running" | "complete" | "failed"
    pub summary: Option<String>,
    pub step_count: i64,
    /// Parent session ID for recursive subagent trees (None for root sessions).
    pub parent_session_id: Option<String>,
    /// Nesting depth (0 for root sessions).
    pub depth: i64,
    /// Absolute path of the workspace this session ran against. Enables
    /// project-scoped FTS5 search for long-running agent self-recall.
    #[serde(default)]
    pub project_path: Option<String>,
}

/// Scope filter for [`SessionStore::search_fts`].
#[derive(Debug, Clone)]
pub enum SearchScope {
    /// Limit hits to sessions whose `project_path` matches exactly.
    Project(String),
    /// Search every session regardless of project.
    All,
}

/// One hit from the message-level FTS5 index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtsHit {
    pub session_id: String,
    pub message_id: i64,
    pub role: String,
    pub content: String,
    /// FTS5 `snippet()` output with `<mark>` highlight markers.
    pub snippet: String,
    /// FTS5 bm25 rank (lower = better match).
    pub rank: f64,
    pub created_at: u64,
    pub project_path: Option<String>,
}

/// A tree node for recursive subagent visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTreeNode {
    pub session: SessionRow,
    pub children: Vec<AgentTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
    pub id: i64,
    pub session_id: String,
    pub role: String,   // "system" | "user" | "assistant"
    pub content: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRow {
    pub id: i64,
    pub session_id: String,
    pub step_num: i64,
    pub tool_name: String,
    pub input_summary: String,
    pub output: String,
    pub success: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: SessionRow,
    pub messages: Vec<MessageRow>,
    pub steps: Vec<StepRow>,
}

// ── SessionStore ──────────────────────────────────────────────────────────────

pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// Open (or create) the session database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {:?}", parent))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open SQLite at {:?}", path))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let store = Self { conn };
        store.create_schema()?;
        Ok(store)
    }

    /// Open from the default path: `~/.vibecli/sessions.db`.
    pub fn open_default() -> Result<Self> {
        let path = default_db_path();
        Self::open(path)
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id          TEXT PRIMARY KEY,
                task        TEXT NOT NULL,
                provider    TEXT NOT NULL DEFAULT '',
                model       TEXT NOT NULL DEFAULT '',
                started_at  INTEGER NOT NULL,
                finished_at INTEGER,
                status      TEXT NOT NULL DEFAULT 'running',
                summary     TEXT,
                step_count  INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role        TEXT NOT NULL,
                content     TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS steps (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                step_num      INTEGER NOT NULL,
                tool_name     TEXT NOT NULL,
                input_summary TEXT NOT NULL DEFAULT '',
                output        TEXT NOT NULL DEFAULT '',
                success       INTEGER NOT NULL DEFAULT 1,
                created_at    INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_steps_session    ON steps(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);
            "#,
        )?;
        // Migration: add parent_session_id and depth columns (idempotent).
        self.maybe_add_column("sessions", "parent_session_id", "TEXT")?;
        self.maybe_add_column("sessions", "depth", "INTEGER NOT NULL DEFAULT 0")?;
        // Phase 1: project scoping for long-running agent self-recall.
        self.maybe_add_column("sessions", "project_path", "TEXT")?;
        // Index for tree queries.
        let _ = self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);\n             CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_path);"
        );

        // Phase 1: FTS5 over message content (external-content pattern, no duplicate storage).
        // Triggers keep the index in lockstep with the `messages` table so ON DELETE CASCADE
        // from `sessions` flows through to FTS automatically.
        self.conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                content='messages',
                content_rowid='id',
                tokenize='porter unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;
            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content)
                VALUES ('delete', old.id, old.content);
            END;
            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content)
                VALUES ('delete', old.id, old.content);
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;
            "#,
        )?;

        self.backfill_fts_if_needed()?;

        // Recap & Resume — Phase F1.1. New `recaps` table for Session
        // recaps; Job and DiffChain recaps live in their own stores per
        // the storage matrix in docs/design/recap-resume/README.md.
        // Idempotent CREATE — safe to run on every open.
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS recaps (
                id                 TEXT PRIMARY KEY,
                kind               TEXT NOT NULL DEFAULT 'session',
                subject_id         TEXT NOT NULL,
                last_message_id    INTEGER,
                workspace          TEXT,
                generated_at       TEXT NOT NULL,
                generator_kind     TEXT NOT NULL,
                generator_provider TEXT,
                generator_model    TEXT,
                headline           TEXT NOT NULL,
                body_json          TEXT NOT NULL,
                token_input        INTEGER,
                token_output       INTEGER,
                schema_version     INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (subject_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_recaps_subject ON recaps(subject_id);
            CREATE INDEX IF NOT EXISTS idx_recaps_generated ON recaps(generated_at);
            CREATE UNIQUE INDEX IF NOT EXISTS uq_recaps_subject_last_msg
                ON recaps(subject_id, last_message_id);
            "#,
        )?;

        Ok(())
    }

    /// If the FTS index is empty but `messages` already has rows (true migration case),
    /// populate the index from existing messages.
    fn backfill_fts_if_needed(&self) -> Result<()> {
        let fts_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM messages_fts", [], |r| r.get(0))?;
        if fts_count > 0 {
            return Ok(());
        }
        let msg_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
        if msg_count == 0 {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO messages_fts(rowid, content) SELECT id, content FROM messages",
            [],
        )?;
        Ok(())
    }

    /// Idempotent ALTER TABLE — silently ignores "duplicate column" errors.
    fn maybe_add_column(&self, table: &str, column: &str, col_type: &str) -> Result<()> {
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, col_type);
        match self.conn.execute(&sql, []) {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("duplicate column") => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Insert a new session record (status = "running").
    pub fn insert_session(
        &self,
        id: &str,
        task: &str,
        provider: &str,
        model: &str,
    ) -> Result<()> {
        self.insert_session_with_parent(id, task, provider, model, None, 0)
    }

    /// Insert a new session record with parent tracking for recursive subagent trees.
    pub fn insert_session_with_parent(
        &self,
        id: &str,
        task: &str,
        provider: &str,
        model: &str,
        parent_session_id: Option<&str>,
        depth: u32,
    ) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions (id, task, provider, model, started_at, status, parent_session_id, depth)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
            params![id, task, provider, model, now, parent_session_id, depth as i64],
        )?;
        Ok(())
    }

    /// Insert a new session bound to a workspace path. Used by long-running agents
    /// so [`search_fts`](Self::search_fts) can scope recall to the current project.
    pub fn insert_session_with_project(
        &self,
        id: &str,
        task: &str,
        provider: &str,
        model: &str,
        project_path: &str,
    ) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions
             (id, task, provider, model, started_at, status, parent_session_id, depth, project_path)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', NULL, 0, ?6)",
            params![id, task, provider, model, now, project_path],
        )?;
        Ok(())
    }

    /// Delete a session and all of its messages and steps. FK cascade handles the
    /// child rows; the `messages_ad` trigger propagates to the FTS index.
    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Recap & Resume — Phase F1.1 CRUD ───────────────────────────────────

    /// Insert a recap row, **idempotent** on `(subject_id, last_message_id)`.
    /// If a recap already exists for that pair, the existing row is
    /// returned unchanged — preserving the original `id` and
    /// `generated_at`. To force a regeneration, call `delete_recap`
    /// first or pass a different `last_message_id` (i.e. the session
    /// has new messages since the prior recap).
    ///
    /// Returns the stored recap (which may differ from the input
    /// when the idempotency rule fires).
    pub fn insert_recap(
        &self,
        recap: &crate::recap::Recap,
    ) -> Result<crate::recap::Recap> {
        // Idempotency check first — same subject + last_message_id =>
        // return the existing row. NULL equality in SQL: a stored
        // NULL `last_message_id` matches an input None, an integer
        // matches an equal integer.
        if let Some(existing) = self.get_recap_by_subject_and_last_msg(
            &recap.subject_id,
            recap.last_message_id,
        )? {
            return Ok(existing);
        }

        let body = serde_json::to_string(&RecapBodyJson::from(recap))
            .map_err(|e| anyhow::anyhow!("recap body serialize: {e}"))?;

        let (gen_kind, gen_provider, gen_model) = match &recap.generator {
            crate::recap::RecapGenerator::Heuristic => ("heuristic", None, None),
            crate::recap::RecapGenerator::Llm { provider, model } => {
                ("llm", Some(provider.as_str()), Some(model.as_str()))
            }
            crate::recap::RecapGenerator::UserEdited => ("user_edited", None, None),
        };
        let kind_str = match recap.kind {
            crate::recap::RecapKind::Session => "session",
            crate::recap::RecapKind::Job => "job",
            crate::recap::RecapKind::DiffChain => "diff_chain",
        };
        let workspace_str = recap
            .workspace
            .as_ref()
            .and_then(|p| p.to_str())
            .map(str::to_string);
        let generated_at_iso = recap.generated_at.to_rfc3339();
        let token_in = recap.token_usage.map(|t| t.input as i64);
        let token_out = recap.token_usage.map(|t| t.output as i64);

        self.conn.execute(
            "INSERT INTO recaps
             (id, kind, subject_id, last_message_id, workspace, generated_at,
              generator_kind, generator_provider, generator_model,
              headline, body_json, token_input, token_output, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                recap.id,
                kind_str,
                recap.subject_id,
                recap.last_message_id,
                workspace_str,
                generated_at_iso,
                gen_kind,
                gen_provider,
                gen_model,
                recap.headline,
                body,
                token_in,
                token_out,
                recap.schema_version as i64,
            ],
        )?;

        Ok(recap.clone())
    }

    /// Fetch a recap by ID. Returns `Ok(None)` if the row doesn't
    /// exist (caller distinguishes from DB error).
    pub fn get_recap_by_id(&self, id: &str) -> Result<Option<crate::recap::Recap>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, subject_id, last_message_id, workspace, generated_at,
                    generator_kind, generator_provider, generator_model,
                    headline, body_json, token_input, token_output, schema_version
             FROM recaps WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(r) => Ok(Some(row_to_recap(r)?)),
            None => Ok(None),
        }
    }

    /// Fetch the recap row for a given `(subject_id, last_message_id)`
    /// pair. Used by `insert_recap` for idempotency and by callers
    /// that want to know whether a fresh recap is needed.
    pub fn get_recap_by_subject_and_last_msg(
        &self,
        subject_id: &str,
        last_message_id: Option<i64>,
    ) -> Result<Option<crate::recap::Recap>> {
        let sql = match last_message_id {
            Some(_) => "SELECT id, kind, subject_id, last_message_id, workspace, generated_at,
                              generator_kind, generator_provider, generator_model,
                              headline, body_json, token_input, token_output, schema_version
                       FROM recaps WHERE subject_id = ?1 AND last_message_id = ?2",
            None => "SELECT id, kind, subject_id, last_message_id, workspace, generated_at,
                            generator_kind, generator_provider, generator_model,
                            headline, body_json, token_input, token_output, schema_version
                     FROM recaps WHERE subject_id = ?1 AND last_message_id IS NULL",
        };
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = match last_message_id {
            Some(id) => stmt.query(params![subject_id, id])?,
            None => stmt.query(params![subject_id])?,
        };
        match rows.next()? {
            Some(r) => Ok(Some(row_to_recap(r)?)),
            None => Ok(None),
        }
    }

    /// List recaps for a subject, newest-first by `generated_at`.
    /// `limit = 0` is treated as "no limit" — caller's responsibility
    /// to set sane bounds for paginated UIs.
    pub fn list_recaps_for_subject(
        &self,
        subject_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::recap::Recap>> {
        let sql = if limit == 0 {
            "SELECT id, kind, subject_id, last_message_id, workspace, generated_at,
                    generator_kind, generator_provider, generator_model,
                    headline, body_json, token_input, token_output, schema_version
             FROM recaps WHERE subject_id = ?1 ORDER BY generated_at DESC".to_string()
        } else {
            format!(
                "SELECT id, kind, subject_id, last_message_id, workspace, generated_at,
                        generator_kind, generator_provider, generator_model,
                        headline, body_json, token_input, token_output, schema_version
                 FROM recaps WHERE subject_id = ?1 ORDER BY generated_at DESC LIMIT {limit}"
            )
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![subject_id], |r| {
            row_to_recap(r).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::other(e.to_string())),
                )
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Delete a recap by ID. No-op if the row doesn't exist.
    pub fn delete_recap(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM recaps WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Apply a user edit to a recap. Updates headline + the JSON body
    /// (bullets, next_actions, artifacts, resume_hint) and flips
    /// `generator_kind` to `'user_edited'`. The original `id`,
    /// `subject_id`, `last_message_id`, and `generated_at` are
    /// preserved so the recap keeps its place in the timeline. Returns
    /// the updated row, or `Ok(None)` if no row matched the id.
    pub fn update_recap(
        &self,
        id: &str,
        headline: &str,
        bullets: &[String],
        next_actions: &[String],
        artifacts: &[crate::recap::RecapArtifact],
        resume_hint: Option<&crate::recap::ResumeHint>,
    ) -> Result<Option<crate::recap::Recap>> {
        // Load the existing row first so we can re-emit the full
        // shape after the update — and so we 404 cleanly when the
        // caller hits a stale id.
        let Some(_existing) = self.get_recap_by_id(id)? else {
            return Ok(None);
        };
        let body = RecapBodyJson {
            bullets: bullets.to_vec(),
            next_actions: next_actions.to_vec(),
            artifacts: artifacts.to_vec(),
            resume_hint: resume_hint.cloned(),
        };
        let body_str = serde_json::to_string(&body)
            .map_err(|e| anyhow::anyhow!("recap body serialize: {e}"))?;

        self.conn.execute(
            "UPDATE recaps
             SET headline = ?1,
                 body_json = ?2,
                 generator_kind = 'user_edited',
                 generator_provider = NULL,
                 generator_model = NULL
             WHERE id = ?3",
            params![headline, body_str, id],
        )?;
        self.get_recap_by_id(id)
    }

    /// Mark a session complete (or failed) with an optional summary.
    pub fn finish_session(
        &self,
        id: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE sessions
             SET status = ?1, summary = ?2, finished_at = ?3,
                 step_count = (SELECT COUNT(*) FROM steps WHERE session_id = ?4)
             WHERE id = ?4",
            params![status, summary, now, id],
        )?;
        Ok(())
    }

    /// Append a message to a session.
    pub fn insert_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        )?;
        Ok(())
    }

    /// Append a tool-call step to a session.
    pub fn insert_step(
        &self,
        session_id: &str,
        step_num: usize,
        tool_name: &str,
        input_summary: &str,
        output: &str,
        success: bool,
    ) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO steps (session_id, step_num, tool_name, input_summary, output, success, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session_id, step_num as i64, tool_name, input_summary, output, success as i32, now],
        )?;
        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// List the most recent sessions (newest first).
    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth, project_path
             FROM sessions ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get a single session by ID.
    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth, project_path
             FROM sessions WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], row_to_session)?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    /// Get all messages for a session.
    pub fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, created_at
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get::<_, i64>(4)? as u64,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get all steps for a session.
    pub fn get_steps(&self, session_id: &str) -> Result<Vec<StepRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, step_num, tool_name, input_summary, output, success, created_at
             FROM steps WHERE session_id = ?1 ORDER BY step_num ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(StepRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                step_num: row.get(2)?,
                tool_name: row.get(3)?,
                input_summary: row.get(4)?,
                output: row.get(5)?,
                success: row.get::<_, i32>(6)? != 0,
                created_at: row.get::<_, i64>(7)? as u64,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get a session with all its messages and steps.
    pub fn get_session_detail(&self, id: &str) -> Result<Option<SessionDetail>> {
        match self.get_session(id)? {
            None => Ok(None),
            Some(session) => {
                let messages = self.get_messages(id)?;
                let steps = self.get_steps(id)?;
                Ok(Some(SessionDetail { session, messages, steps }))
            }
        }
    }

    /// FTS5-backed message search with optional project scope. Returns ranked hits
    /// (best match first) with `<mark>`-highlighted snippets — the primary self-recall
    /// surface for long-running agents. `query` is an FTS5 MATCH expression.
    pub fn search_fts(
        &self,
        query: &str,
        scope: SearchScope,
        limit: usize,
    ) -> Result<Vec<FtsHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let project_filter: Option<String> = match scope {
            SearchScope::Project(p) => Some(p),
            SearchScope::All => None,
        };

        let mut stmt = self.conn.prepare(
            r#"SELECT
                m.session_id,
                m.id,
                m.role,
                m.content,
                m.created_at,
                s.project_path,
                snippet(messages_fts, 0, '<mark>', '</mark>', '…', 16),
                bm25(messages_fts)
             FROM messages_fts
             JOIN messages  m ON m.id = messages_fts.rowid
             JOIN sessions  s ON s.id = m.session_id
             WHERE messages_fts MATCH ?1
               AND (?2 IS NULL OR s.project_path = ?2)
             ORDER BY bm25(messages_fts)
             LIMIT ?3"#,
        )?;
        let rows = stmt.query_map(
            params![trimmed, project_filter, limit as i64],
            |row| {
                Ok(FtsHit {
                    session_id: row.get(0)?,
                    message_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get::<_, i64>(4)? as u64,
                    project_path: row.get(5)?,
                    snippet: row.get(6)?,
                    rank: row.get(7)?,
                })
            },
        )?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Full-text search: sessions whose task, steps, or messages contain all keywords.
    pub fn search(&self, query: &str) -> Result<Vec<SessionRow>> {
        let keywords: Vec<&str> = query.split_whitespace().collect();
        if keywords.is_empty() {
            return self.list_sessions(20);
        }

        // Build a LIKE pattern for each keyword across sessions + steps + messages
        // Using a sub-query approach: get session IDs that match all keywords
        let like_pattern = |kw: &str| {
            let escaped = kw.to_lowercase()
                .replace('%', "\\%")
                .replace('_', "\\_");
            format!("%{}%", escaped)
        };

        // Collect candidate session IDs from steps + messages + task
        let mut candidate_sets: Vec<std::collections::HashSet<String>> = Vec::new();

        for kw in &keywords {
            let pat = like_pattern(kw);
            let mut stmt = self.conn.prepare(
                "SELECT DISTINCT session_id FROM steps
                 WHERE LOWER(tool_name) LIKE ?1 ESCAPE '\\' OR LOWER(input_summary) LIKE ?1 ESCAPE '\\'
                 UNION
                 SELECT DISTINCT session_id FROM messages
                 WHERE LOWER(content) LIKE ?1 ESCAPE '\\'
                 UNION
                 SELECT id FROM sessions WHERE LOWER(task) LIKE ?1 ESCAPE '\\'",
            )?;
            let ids: std::collections::HashSet<String> = stmt
                .query_map(params![pat], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            candidate_sets.push(ids);
        }

        // Intersect all sets (AND semantics)
        let matching_ids: std::collections::HashSet<String> = if candidate_sets.is_empty() {
            std::collections::HashSet::new()
        } else {
            candidate_sets
                .into_iter()
                .reduce(|a, b| a.intersection(&b).cloned().collect())
                .unwrap_or_default()
        };

        if matching_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch matching sessions sorted by recency
        let mut results = Vec::new();
        for id in &matching_ids {
            if let Some(s) = self.get_session(id)? {
                results.push(s);
            }
        }
        results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(results)
    }

    /// Get all direct children of a session.
    pub fn get_children(&self, parent_id: &str) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth, project_path
             FROM sessions WHERE parent_session_id = ?1 ORDER BY started_at ASC",
        )?;
        let rows = stmt.query_map(params![parent_id], row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Build a full agent tree rooted at a given session ID.
    pub fn get_tree(&self, root_id: &str) -> Result<Option<AgentTreeNode>> {
        let session = match self.get_session(root_id)? {
            Some(s) => s,
            None => return Ok(None),
        };
        let children = self.build_tree_children(root_id)?;
        Ok(Some(AgentTreeNode { session, children }))
    }

    fn build_tree_children(&self, parent_id: &str) -> Result<Vec<AgentTreeNode>> {
        let children = self.get_children(parent_id)?;
        let mut nodes = Vec::new();
        for child in children {
            let grandchildren = self.build_tree_children(&child.id)?;
            nodes.push(AgentTreeNode {
                session: child,
                children: grandchildren,
            });
        }
        Ok(nodes)
    }

    /// List root sessions (no parent) from the last 24 hours with child counts.
    pub fn list_root_sessions(&self, limit: usize) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth, project_path
             FROM sessions WHERE parent_session_id IS NULL ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Count of all sessions.
    pub fn count(&self) -> Result<i64> {
        let n: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        Ok(n)
    }

    /// Update the task/name of a session (used by AI auto-naming).
    pub fn rename_session(&self, id: &str, new_name: &str) -> Result<()> {
        self.conn.execute("UPDATE sessions SET task = ?1 WHERE id = ?2", params![new_name, id])?;
        Ok(())
    }

    /// Fork a session: create a copy with a new ID and `parent_session_id` pointing to the source.
    /// All messages and steps up to the current point are duplicated into the fork.
    pub fn fork_session(&self, source_id: &str, new_id: &str) -> Result<()> {
        // Check the source exists
        let source = self.get_session(source_id)?
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", source_id))?;

        // Insert forked session row
        let task = format!("[fork of {}] {}", &source_id[..source_id.len().min(12)], source.task);
        self.insert_session_with_parent(new_id, &task, &source.provider, &source.model, Some(source_id), 0)?;

        // Copy messages
        for msg in self.get_messages(source_id)? {
            self.insert_message(new_id, &msg.role, &msg.content)?;
        }

        // Copy steps
        for step in self.get_steps(source_id)? {
            self.insert_step(new_id, step.step_num as usize, &step.tool_name,
                &step.input_summary, &step.output, step.success)?;
        }

        Ok(())
    }
}

// ── Recap helpers (Phase F1.1) ────────────────────────────────────────────────

/// JSON shape stored in `body_json` — the subset of `Recap` that
/// doesn't have its own column. Keeping this separate from
/// `crate::recap::Recap` lets us evolve the row schema without
/// breaking the in-memory shape (and vice versa).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RecapBodyJson {
    bullets: Vec<String>,
    next_actions: Vec<String>,
    artifacts: Vec<crate::recap::RecapArtifact>,
    resume_hint: Option<crate::recap::ResumeHint>,
}

impl From<&crate::recap::Recap> for RecapBodyJson {
    fn from(r: &crate::recap::Recap) -> Self {
        Self {
            bullets: r.bullets.clone(),
            next_actions: r.next_actions.clone(),
            artifacts: r.artifacts.clone(),
            resume_hint: r.resume_hint.clone(),
        }
    }
}

fn row_to_recap(row: &rusqlite::Row<'_>) -> Result<crate::recap::Recap> {
    let id: String = row.get(0)?;
    let kind_str: String = row.get(1)?;
    let subject_id: String = row.get(2)?;
    let last_message_id: Option<i64> = row.get(3)?;
    let workspace: Option<String> = row.get(4)?;
    let generated_at_iso: String = row.get(5)?;
    let generator_kind: String = row.get(6)?;
    let generator_provider: Option<String> = row.get(7)?;
    let generator_model: Option<String> = row.get(8)?;
    let headline: String = row.get(9)?;
    let body_json: String = row.get(10)?;
    let token_input: Option<i64> = row.get(11)?;
    let token_output: Option<i64> = row.get(12)?;
    let schema_version: i64 = row.get(13)?;

    let kind = match kind_str.as_str() {
        "session" => crate::recap::RecapKind::Session,
        "job" => crate::recap::RecapKind::Job,
        "diff_chain" => crate::recap::RecapKind::DiffChain,
        other => anyhow::bail!("unknown recap kind in DB: {other:?}"),
    };
    let generator = match generator_kind.as_str() {
        "heuristic" => crate::recap::RecapGenerator::Heuristic,
        "llm" => {
            let provider = generator_provider.unwrap_or_default();
            let model = generator_model.unwrap_or_default();
            crate::recap::RecapGenerator::Llm { provider, model }
        }
        "user_edited" => crate::recap::RecapGenerator::UserEdited,
        other => anyhow::bail!("unknown generator_kind in DB: {other:?}"),
    };
    let body: RecapBodyJson = serde_json::from_str(&body_json)
        .map_err(|e| anyhow::anyhow!("recap body deserialize: {e}"))?;
    let generated_at = chrono::DateTime::parse_from_rfc3339(&generated_at_iso)
        .map_err(|e| anyhow::anyhow!("recap generated_at parse: {e}"))?
        .with_timezone(&chrono::Utc);
    let token_usage = match (token_input, token_output) {
        (Some(i), Some(o)) => Some(crate::recap::RecapTokenUsage {
            input: i as u32,
            output: o as u32,
        }),
        _ => None,
    };

    Ok(crate::recap::Recap {
        id,
        kind,
        subject_id,
        last_message_id,
        workspace: workspace.map(std::path::PathBuf::from),
        generated_at,
        generator,
        headline,
        bullets: body.bullets,
        next_actions: body.next_actions,
        artifacts: body.artifacts,
        resume_hint: body.resume_hint,
        token_usage,
        schema_version: schema_version as u16,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        id: row.get(0)?,
        task: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        started_at: row.get::<_, i64>(4)? as u64,
        finished_at: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        status: row.get(6)?,
        summary: row.get(7)?,
        step_count: row.get(8)?,
        parent_session_id: row.get(9).unwrap_or(None),
        depth: row.get(10).unwrap_or(0),
        project_path: row.get::<_, Option<String>>(11).unwrap_or(None),
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Default path: `~/.vibecli/sessions.db`
pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("sessions.db")
}

// ── Web Viewer HTML ───────────────────────────────────────────────────────────

/// Generate a self-contained HTML page for a session detail view.
pub fn render_session_html(detail: &SessionDetail) -> String {
    let s = &detail.session;
    let started = format_ts(s.started_at);
    let finished = s.finished_at.map(format_ts).unwrap_or_else(|| "in progress…".to_string());
    let status_badge = match s.status.as_str() {
        "complete" => r#"<span class="badge ok">✅ complete</span>"#,
        "failed"   => r#"<span class="badge err">❌ failed</span>"#,
        _          => r#"<span class="badge run">🟡 running</span>"#,
    };
    let duration = if let Some(fin) = s.finished_at {
        let ms = fin.saturating_sub(s.started_at);
        format!(" &nbsp;·&nbsp; {}s", ms / 1000)
    } else {
        String::new()
    };

    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Session {id}</title>
<style>
  :root {{ --bg: #0d1117; --surface: #161b22; --border: #30363d; --text: #e6edf3;
           --muted: #8b949e; --green: #3fb950; --red: #f85149; --blue: #58a6ff;
           --yellow: #d29922; --code-bg: #1c2128; --tool-bg: #1a2233; }}
  *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ background: var(--bg); color: var(--text); font-family: -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
          font-size: 14px; line-height: 1.6; padding: 24px; max-width: 900px; margin: 0 auto; }}
  h1 {{ font-size: 18px; font-weight: 600; margin-bottom: 4px; }}
  .meta {{ color: var(--muted); font-size: 12px; margin-bottom: 20px; }}
  .badge {{ display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 12px; font-weight: 600; }}
  .badge.ok  {{ background: #1a3a1a; color: var(--green); }}
  .badge.err {{ background: #3a1a1a; color: var(--red); }}
  .badge.run {{ background: #3a2d0a; color: var(--yellow); }}
  .summary {{ background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
              padding: 14px 18px; margin-bottom: 20px; white-space: pre-wrap; word-break: break-word; }}
  .section-title {{ font-size: 13px; font-weight: 600; text-transform: uppercase;
                    letter-spacing: .08em; color: var(--muted); margin: 24px 0 10px; }}
  .msg {{ border-radius: 8px; padding: 12px 16px; margin-bottom: 10px; }}
  .msg.system {{ background: #1a1a2e; border: 1px solid #2a2a4a; }}
  .msg.user   {{ background: #0d2137; border: 1px solid #1a3f5c; }}
  .msg.assistant {{ background: var(--surface); border: 1px solid var(--border); }}
  .msg-role {{ font-size: 11px; font-weight: 700; text-transform: uppercase;
               letter-spacing: .06em; margin-bottom: 6px; }}
  .msg.system .msg-role    {{ color: #6e7dbf; }}
  .msg.user .msg-role      {{ color: var(--blue); }}
  .msg.assistant .msg-role {{ color: var(--green); }}
  .msg-content {{ white-space: pre-wrap; word-break: break-word; font-size: 13px; }}
  .step {{ background: var(--tool-bg); border: 1px solid var(--border); border-radius: 8px;
           padding: 10px 14px; margin-bottom: 8px; }}
  .step-header {{ display: flex; align-items: center; gap: 10px; margin-bottom: 4px; }}
  .step-num {{ color: var(--muted); font-size: 11px; min-width: 42px; }}
  .step-tool {{ font-weight: 600; font-size: 13px; color: var(--blue); }}
  .step-ok   {{ color: var(--green); font-size: 12px; }}
  .step-err  {{ color: var(--red);   font-size: 12px; }}
  .step-summary {{ color: var(--muted); font-size: 12px; white-space: nowrap; overflow: hidden;
                   text-overflow: ellipsis; max-width: 600px; }}
  .step-output {{ margin-top: 8px; background: var(--code-bg); border-radius: 6px;
                  padding: 8px 12px; font-family: monospace; font-size: 12px;
                  white-space: pre-wrap; word-break: break-all; max-height: 200px; overflow-y: auto;
                  display: none; }}
  .step:hover .step-output {{ display: block; }}
  footer {{ margin-top: 40px; color: var(--muted); font-size: 11px; text-align: center; }}
</style>
</head>
<body>
<h1>🤖 Agent Session</h1>
<div class="meta">
  {status_badge} &nbsp;
  <strong>{provider}</strong>{model_str} &nbsp;·&nbsp;
  {started} – {finished}{duration} &nbsp;·&nbsp;
  {step_count} step{s_plural} &nbsp;·&nbsp;
  ID: <code>{id}</code>
</div>
<div class="summary"><strong>Task:</strong> {task}</div>
"#,
        id = escape_html(&s.id),
        status_badge = status_badge,
        provider = escape_html(&s.provider),
        model_str = if s.model.is_empty() { String::new() } else { format!(" / {}", escape_html(&s.model)) },
        started = escape_html(&started),
        finished = escape_html(&finished),
        duration = duration,
        step_count = s.step_count,
        s_plural = if s.step_count == 1 { "" } else { "s" },
        task = escape_html(&s.task),
    );

    if let Some(ref summary) = s.summary {
        html.push_str(&format!(
            "<div class=\"section-title\">Summary</div>\n<div class=\"summary\">{}</div>\n",
            escape_html(summary)
        ));
    }

    // Steps
    if !detail.steps.is_empty() {
        html.push_str("<div class=\"section-title\">Tool Calls</div>\n");
        for step in &detail.steps {
            let status_icon = if step.success { "✅" } else { "❌" };
            let status_cls = if step.success { "step-ok" } else { "step-err" };
            html.push_str(&format!(
                r#"<div class="step">
  <div class="step-header">
    <span class="step-num">#{}</span>
    <span class="step-tool">{}</span>
    <span class="{}">{}</span>
  </div>
  <div class="step-summary">{}</div>
  <div class="step-output">{}</div>
</div>
"#,
                step.step_num,
                escape_html(&step.tool_name),
                status_cls, status_icon,
                escape_html(&step.input_summary),
                escape_html(&step.output),
            ));
        }
    }

    // Messages
    if !detail.messages.is_empty() {
        html.push_str("<div class=\"section-title\">Conversation</div>\n");
        for msg in &detail.messages {
            let role_cls = match msg.role.as_str() {
                "system" => "system",
                "user"   => "user",
                _        => "assistant",
            };
            // Truncate very long messages (char-safe)
            let preview = if msg.content.len() > 4000 {
                let truncated: String = msg.content.chars().take(4000).collect();
                format!("{}…\n[truncated {} chars]", truncated, msg.content.chars().count())
            } else {
                msg.content.clone()
            };
            html.push_str(&format!(
                "<div class=\"msg {}\"><div class=\"msg-role\">{}</div><div class=\"msg-content\">{}</div></div>\n",
                role_cls,
                escape_html(&msg.role),
                escape_html(&preview),
            ));
        }
    }

    html.push_str("<footer>Generated by VibeCLI &nbsp;·&nbsp; <a href=\"/sessions\" style=\"color:#58a6ff\">All sessions</a></footer>\n</body>\n</html>");
    html
}

/// Generate the sessions index HTML page.
pub fn render_sessions_index_html(sessions: &[SessionRow]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>VibeCLI Sessions</title>
<style>
  :root { --bg: #0d1117; --surface: #161b22; --border: #30363d; --text: #e6edf3;
          --muted: #8b949e; --green: #3fb950; --red: #f85149; --blue: #58a6ff; --yellow: #d29922; }
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: var(--bg); color: var(--text); font-family: -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
         font-size: 14px; line-height: 1.6; padding: 24px; max-width: 900px; margin: 0 auto; }
  h1 { font-size: 20px; font-weight: 700; margin-bottom: 4px; }
  .sub { color: var(--muted); font-size: 13px; margin-bottom: 24px; }
  .session { border: 1px solid var(--border); border-radius: 8px; padding: 14px 18px;
             margin-bottom: 10px; text-decoration: none; display: block; color: var(--text); }
  .session:hover { border-color: var(--blue); background: #0d1f3c; }
  .session-top { display: flex; align-items: center; gap: 10px; margin-bottom: 4px; }
  .badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 11px; font-weight: 600; }
  .badge.ok  { background: #1a3a1a; color: var(--green); }
  .badge.err { background: #3a1a1a; color: var(--red); }
  .badge.run { background: #3a2d0a; color: var(--yellow); }
  .session-task { font-weight: 600; flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .session-meta { color: var(--muted); font-size: 12px; }
  .empty { color: var(--muted); text-align: center; padding: 40px; }
  footer { margin-top: 40px; color: var(--muted); font-size: 11px; text-align: center; }
</style>
</head>
<body>
<h1>🤖 VibeCLI Sessions</h1>
"#,
    );

    if sessions.is_empty() {
        html.push_str("<div class=\"empty\">No sessions yet. Run <code>vibecli</code> with a task to create one.</div>\n");
    } else {
        html.push_str(&format!(
            "<div class=\"sub\">{} session{}</div>\n",
            sessions.len(),
            if sessions.len() == 1 { "" } else { "s" }
        ));
        for s in sessions {
            let badge = match s.status.as_str() {
                "complete" => r#"<span class="badge ok">✅ complete</span>"#,
                "failed"   => r#"<span class="badge err">❌ failed</span>"#,
                _          => r#"<span class="badge run">🟡 running</span>"#,
            };
            let task_preview: String = s.task.chars().take(80).collect();
            let task_preview = if s.task.len() > 80 {
                format!("{}…", task_preview)
            } else {
                task_preview
            };
            let age = format_age(s.started_at);
            html.push_str(&format!(
                r#"<a class="session" href="/view/{id}">
  <div class="session-top">
    {badge}
    <span class="session-task">{task}</span>
  </div>
  <div class="session-meta">{age} &nbsp;·&nbsp; {steps} steps &nbsp;·&nbsp; {provider}</div>
</a>
"#,
                id = escape_html(&s.id),
                badge = badge,
                task = escape_html(&task_preview),
                age = escape_html(&age),
                steps = s.step_count,
                provider = escape_html(&s.provider),
            ));
        }
    }

    html.push_str("<footer>Generated by VibeCLI</footer>\n</body>\n</html>");
    html
}

fn format_ts(ms: u64) -> String {
    // Simple ISO-like: seconds since epoch → human-readable
    let secs = ms / 1000;
    // We just show relative + rough absolute
    
    chrono_simple(secs)
}

fn format_age(started_ms: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let elapsed_secs = now.saturating_sub(started_ms) / 1000;
    if elapsed_secs < 60 {
        format!("{}s ago", elapsed_secs)
    } else if elapsed_secs < 3600 {
        format!("{}m ago", elapsed_secs / 60)
    } else if elapsed_secs < 86400 {
        format!("{}h ago", elapsed_secs / 3600)
    } else {
        format!("{}d ago", elapsed_secs / 86400)
    }
}

/// Very simple epoch → "YYYY-MM-DD HH:MM" formatter (no external deps).
fn chrono_simple(epoch_secs: u64) -> String {
    // Days since 1970-01-01
    let days = epoch_secs / 86400;
    let time_of_day = epoch_secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    // Gregorian calendar approximation
    let (y, mo, d) = days_to_ymd(days as i64);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, mo, d, h, m)
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn open_temp() -> SessionStore {
        let f = NamedTempFile::new().unwrap();
        let path = f.path().to_owned();
        // Don't delete the temp file while we use it
        std::mem::forget(f);
        SessionStore::open(path).unwrap()
    }

    #[test]
    fn test_insert_and_list() {
        let store = open_temp();
        store.insert_session("s1", "Write a hello world", "claude", "claude-3").unwrap();
        store.insert_session("s2", "Fix the bug", "ollama", "llama3").unwrap();
        let sessions = store.list_sessions(10).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_finish_session() {
        let store = open_temp();
        store.insert_session("s1", "task", "ollama", "llama3").unwrap();
        store.insert_step("s1", 1, "read_file", "src/main.rs", "fn main() {}", true).unwrap();
        store.finish_session("s1", "complete", Some("Done!")).unwrap();
        let s = store.get_session("s1").unwrap().unwrap();
        assert_eq!(s.status, "complete");
        assert_eq!(s.summary.as_deref(), Some("Done!"));
        assert_eq!(s.step_count, 1);
    }

    #[test]
    fn test_messages_and_steps() {
        let store = open_temp();
        store.insert_session("s1", "task", "claude", "claude-3").unwrap();
        store.insert_message("s1", "user", "Hello agent").unwrap();
        store.insert_message("s1", "assistant", "I'll help you").unwrap();
        store.insert_step("s1", 1, "bash", "ls -la", "total 42", true).unwrap();
        let detail = store.get_session_detail("s1").unwrap().unwrap();
        assert_eq!(detail.messages.len(), 2);
        assert_eq!(detail.steps.len(), 1);
    }

    #[test]
    fn test_search() {
        let store = open_temp();
        store.insert_session("s1", "Write tests for auth module", "claude", "claude-3").unwrap();
        store.insert_message("s1", "assistant", "I'll write tests for authentication").unwrap();
        store.insert_session("s2", "Fix deploy pipeline", "ollama", "llama3").unwrap();
        store.insert_step("s2", 1, "bash", "git push --force", "error", false).unwrap();

        let results = store.search("auth").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");

        let results2 = store.search("tests auth").unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_render_html_no_panic() {
        let store = open_temp();
        store.insert_session("s1", "hello world task", "claude", "claude-3").unwrap();
        store.insert_message("s1", "user", "Do something <dangerous> & tricky").unwrap();
        store.insert_step("s1", 1, "bash", "ls", "file.txt", true).unwrap();
        store.finish_session("s1", "complete", Some("All done")).unwrap();
        let detail = store.get_session_detail("s1").unwrap().unwrap();
        let html = render_session_html(&detail);
        assert!(html.contains("All done"));
        assert!(!html.contains("<dangerous>")); // must be escaped
    }

    #[test]
    fn test_insert_session_with_parent() {
        let store = open_temp();
        store.insert_session("root", "Main task", "claude", "claude-3").unwrap();
        store.insert_session_with_parent("child1", "Sub-task 1", "claude", "claude-3", Some("root"), 1).unwrap();
        let child = store.get_session("child1").unwrap().unwrap();
        assert_eq!(child.parent_session_id.as_deref(), Some("root"));
        assert_eq!(child.depth, 1);
    }

    #[test]
    fn test_get_children() {
        let store = open_temp();
        store.insert_session("root", "Main task", "claude", "claude-3").unwrap();
        store.insert_session_with_parent("c1", "Child 1", "claude", "claude-3", Some("root"), 1).unwrap();
        store.insert_session_with_parent("c2", "Child 2", "claude", "claude-3", Some("root"), 1).unwrap();
        store.insert_session_with_parent("gc1", "Grandchild", "claude", "claude-3", Some("c1"), 2).unwrap();
        let children = store.get_children("root").unwrap();
        assert_eq!(children.len(), 2);
        let grandchildren = store.get_children("c1").unwrap();
        assert_eq!(grandchildren.len(), 1);
    }

    #[test]
    fn test_get_tree() {
        let store = open_temp();
        store.insert_session("root", "Main task", "claude", "claude-3").unwrap();
        store.insert_session_with_parent("c1", "Child 1", "claude", "claude-3", Some("root"), 1).unwrap();
        store.insert_session_with_parent("c2", "Child 2", "claude", "claude-3", Some("root"), 1).unwrap();
        store.insert_session_with_parent("gc1", "Grandchild", "claude", "claude-3", Some("c1"), 2).unwrap();
        let tree = store.get_tree("root").unwrap().unwrap();
        assert_eq!(tree.session.id, "root");
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].session.id, "gc1");
    }

    #[test]
    fn test_list_root_sessions() {
        let store = open_temp();
        store.insert_session("root1", "Task 1", "claude", "claude-3").unwrap();
        store.insert_session("root2", "Task 2", "ollama", "llama3").unwrap();
        store.insert_session_with_parent("child", "Sub", "claude", "claude-3", Some("root1"), 1).unwrap();
        let roots = store.list_root_sessions(10).unwrap();
        assert_eq!(roots.len(), 2); // only root sessions, not child
    }

    #[test]
    fn test_migration_idempotent() {
        let store = open_temp();
        // Calling create_schema again should not error (columns already exist).
        assert!(store.create_schema().is_ok());
    }

    #[test]
    fn test_days_to_ymd() {
        // 2024-01-01 = days since 1970-01-01: 19723
        let (y, mo, d) = days_to_ymd(19723);
        assert_eq!(y, 2024);
        assert_eq!(mo, 1);
        assert_eq!(d, 1);
    }

    // ── New tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_escape_html_ampersand() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_html_lt_gt() {
        assert_eq!(escape_html("<script>alert(1)</script>"), "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    #[test]
    fn test_escape_html_double_quote() {
        assert_eq!(escape_html(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn test_escape_html_all_entities() {
        // Combined: &, <, >, "
        let input = r#"x & y < z > 0 "q""#;
        let out = escape_html(input);
        assert!(out.contains("&amp;"));
        assert!(out.contains("&lt;"));
        assert!(out.contains("&gt;"));
        assert!(out.contains("&quot;"));
        // No raw special chars remain
        assert!(!out.contains('&') || out.replace("&amp;", "").replace("&lt;", "").replace("&gt;", "").replace("&quot;", "").find('&').is_none());
    }

    #[test]
    fn test_escape_html_no_change_for_plain_text() {
        assert_eq!(escape_html("Hello world 123"), "Hello world 123");
    }

    #[test]
    fn test_format_ts_known_epoch() {
        // 2020-06-15 12:00 UTC = 1592222400 seconds → 1592222400000 ms
        let result = format_ts(1_592_222_400_000);
        assert_eq!(result, "2020-06-15 12:00");
    }

    #[test]
    fn test_format_ts_epoch_zero() {
        let result = format_ts(0);
        assert_eq!(result, "1970-01-01 00:00");
    }

    #[test]
    fn test_format_age_seconds_ago() {
        // Use a timestamp that is 30 seconds ago from now
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let result = format_age(now_ms - 30_000);
        assert!(result.ends_with("s ago"), "Expected 's ago', got: {}", result);
    }

    #[test]
    fn test_format_age_minutes_ago() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let result = format_age(now_ms - 300_000); // 5 minutes ago
        assert!(result.ends_with("m ago"), "Expected 'm ago', got: {}", result);
        assert!(result.starts_with('5') || result.starts_with('4'), "Expected ~5m, got: {}", result);
    }

    #[test]
    fn test_format_age_hours_ago() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let result = format_age(now_ms - 7_200_000); // 2 hours ago
        assert!(result.ends_with("h ago"), "Expected 'h ago', got: {}", result);
    }

    #[test]
    fn test_format_age_days_ago() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let result = format_age(now_ms - 172_800_000); // 2 days ago
        assert!(result.ends_with("d ago"), "Expected 'd ago', got: {}", result);
    }

    #[test]
    fn test_render_sessions_index_html_structure() {
        let sessions = vec![
            SessionRow {
                id: "abc123".into(),
                task: "Build feature X".into(),
                provider: "claude".into(),
                model: "claude-3".into(),
                started_at: 1_700_000_000_000,
                finished_at: Some(1_700_000_060_000),
                status: "complete".into(),
                summary: Some("Done".into()),
                step_count: 3,
                parent_session_id: None,
                depth: 0,
                project_path: None,
            },
        ];
        let html = render_sessions_index_html(&sessions);
        assert!(html.contains("<!DOCTYPE html>"), "Missing doctype");
        assert!(html.contains("<title>VibeCLI Sessions</title>"), "Missing title");
        assert!(html.contains("VibeCLI Sessions"), "Missing heading text");
        assert!(html.contains("Build feature X"), "Missing task text");
        assert!(html.contains("abc123"), "Missing session ID in link");
        assert!(html.contains("1 session"), "Missing session count");
        assert!(html.contains("complete"), "Missing status badge");
        assert!(html.contains("Generated by VibeCLI"), "Missing footer");
    }

    #[test]
    fn test_render_sessions_index_html_empty() {
        let html = render_sessions_index_html(&[]);
        assert!(html.contains("No sessions yet"), "Missing empty state message");
    }

    #[test]
    fn test_render_sessions_index_html_plural() {
        let sessions = vec![
            SessionRow {
                id: "s1".into(), task: "Task 1".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_000_000, finished_at: None,
                status: "running".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            SessionRow {
                id: "s2".into(), task: "Task 2".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_001_000, finished_at: None,
                status: "failed".into(), summary: None, step_count: 1,
                parent_session_id: None, depth: 0, project_path: None,
            },
        ];
        let html = render_sessions_index_html(&sessions);
        assert!(html.contains("2 sessions"), "Missing plural session count");
    }

    #[test]
    fn test_render_session_html_structure() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "sess-42".into(),
                task: "Refactor the parser".into(),
                provider: "ollama".into(),
                model: "llama3".into(),
                started_at: 1_700_000_000_000,
                finished_at: Some(1_700_000_120_000),
                status: "complete".into(),
                summary: Some("Refactored successfully".into()),
                step_count: 2,
                parent_session_id: None,
                depth: 0,
                project_path: None,
            },
            messages: vec![
                MessageRow {
                    id: 1, session_id: "sess-42".into(), role: "user".into(),
                    content: "Please refactor".into(), created_at: 1_700_000_000_000,
                },
                MessageRow {
                    id: 2, session_id: "sess-42".into(), role: "assistant".into(),
                    content: "Done refactoring".into(), created_at: 1_700_000_010_000,
                },
            ],
            steps: vec![
                StepRow {
                    id: 1, session_id: "sess-42".into(), step_num: 1,
                    tool_name: "read_file".into(), input_summary: "src/parser.rs".into(),
                    output: "fn parse() {}".into(), success: true, created_at: 1_700_000_005_000,
                },
                StepRow {
                    id: 2, session_id: "sess-42".into(), step_num: 2,
                    tool_name: "write_file".into(), input_summary: "src/parser.rs".into(),
                    output: "Written".into(), success: true, created_at: 1_700_000_008_000,
                },
            ],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("<!DOCTYPE html>"), "Missing doctype");
        assert!(html.contains("Session sess-42"), "Missing session ID in title");
        assert!(html.contains("Agent Session"), "Missing heading");
        assert!(html.contains("Refactor the parser"), "Missing task text");
        assert!(html.contains("Refactored successfully"), "Missing summary");
        assert!(html.contains("ollama"), "Missing provider");
        assert!(html.contains("llama3"), "Missing model");
        assert!(html.contains("Tool Calls"), "Missing tool calls section");
        assert!(html.contains("Conversation"), "Missing conversation section");
        assert!(html.contains("read_file"), "Missing step tool name");
        assert!(html.contains("write_file"), "Missing second step tool name");
        assert!(html.contains("Please refactor"), "Missing user message");
        assert!(html.contains("Done refactoring"), "Missing assistant message");
        assert!(html.contains("complete"), "Missing status badge");
        assert!(html.contains("2 steps"), "Missing step count");
    }

    #[test]
    fn test_render_session_html_failed_status() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "f1".into(), task: "Fail task".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_000_000, finished_at: Some(2_000_000),
                status: "failed".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            messages: vec![],
            steps: vec![],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("failed"), "Missing failed status");
    }

    #[test]
    fn test_render_session_html_running_status_no_finished() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "r1".into(), task: "Running".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_000_000, finished_at: None,
                status: "running".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            messages: vec![],
            steps: vec![],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("running"), "Missing running status");
        assert!(html.contains("in progress"), "Missing in progress text for None finished_at");
    }

    #[test]
    fn test_count_empty() {
        let store = open_temp();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_count_after_inserts() {
        let store = open_temp();
        store.insert_session("a", "t1", "p", "m").unwrap();
        assert_eq!(store.count().unwrap(), 1);
        store.insert_session("b", "t2", "p", "m").unwrap();
        assert_eq!(store.count().unwrap(), 2);
        store.insert_session("c", "t3", "p", "m").unwrap();
        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_search_multi_keyword_and() {
        let store = open_temp();
        store.insert_session("s1", "rust parser module", "claude", "claude-3").unwrap();
        store.insert_session("s2", "rust deploy script", "ollama", "llama3").unwrap();
        store.insert_session("s3", "python parser lib", "openai", "gpt-4").unwrap();

        // "rust parser" should match only s1 (AND of both keywords)
        let results = store.search("rust parser").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");
    }

    #[test]
    fn test_search_no_results() {
        let store = open_temp();
        store.insert_session("s1", "Write tests", "claude", "claude-3").unwrap();
        let results = store.search("nonexistent_keyword_xyz").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_partial_match_in_steps() {
        let store = open_temp();
        store.insert_session("s1", "Generic task", "p", "m").unwrap();
        store.insert_step("s1", 1, "bash", "cargo build", "compiled successfully", true).unwrap();
        // Search for content only present in step output
        let results = store.search("compiled").unwrap();
        assert_eq!(results.len(), 0); // "compiled" is in output, not in tool_name or input_summary or content or task
        // But searching for tool_name or input_summary should work
        let results = store.search("cargo").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");
    }

    #[test]
    fn test_search_empty_query_returns_sessions() {
        let store = open_temp();
        store.insert_session("s1", "Task one", "p", "m").unwrap();
        store.insert_session("s2", "Task two", "p", "m").unwrap();
        // Empty query falls through to list_sessions(20)
        let results = store.search("").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_session_row_serde_roundtrip() {
        let row = SessionRow {
            id: "serde-1".into(),
            task: "Serde roundtrip".into(),
            provider: "claude".into(),
            model: "claude-3".into(),
            started_at: 1_700_000_000_000,
            finished_at: Some(1_700_000_060_000),
            status: "complete".into(),
            summary: Some("Done".into()),
            step_count: 5,
            parent_session_id: Some("parent-1".into()),
            depth: 2,
            project_path: None,
        };
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: SessionRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "serde-1");
        assert_eq!(deserialized.task, "Serde roundtrip");
        assert_eq!(deserialized.status, "complete");
        assert_eq!(deserialized.step_count, 5);
        assert_eq!(deserialized.parent_session_id.as_deref(), Some("parent-1"));
        assert_eq!(deserialized.depth, 2);
        assert_eq!(deserialized.summary.as_deref(), Some("Done"));
    }

    #[test]
    fn test_message_row_serde_roundtrip() {
        let row = MessageRow {
            id: 42,
            session_id: "msg-sess".into(),
            role: "assistant".into(),
            content: "Hello from the assistant".into(),
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: MessageRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 42);
        assert_eq!(deserialized.role, "assistant");
        assert_eq!(deserialized.content, "Hello from the assistant");
    }

    #[test]
    fn test_step_row_serde_roundtrip() {
        let row = StepRow {
            id: 7,
            session_id: "step-sess".into(),
            step_num: 3,
            tool_name: "write_file".into(),
            input_summary: "path=/src/lib.rs".into(),
            output: "Written 42 bytes".into(),
            success: false,
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: StepRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 7);
        assert_eq!(deserialized.step_num, 3);
        assert_eq!(deserialized.tool_name, "write_file");
        assert_eq!(deserialized.success, false);
    }

    #[test]
    fn test_agent_tree_node_serde_roundtrip() {
        let node = AgentTreeNode {
            session: SessionRow {
                id: "tree-root".into(), task: "Root".into(), provider: "p".into(),
                model: "m".into(), started_at: 100, finished_at: None,
                status: "running".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            children: vec![
                AgentTreeNode {
                    session: SessionRow {
                        id: "tree-child".into(), task: "Child".into(), provider: "p".into(),
                        model: "m".into(), started_at: 200, finished_at: None,
                        status: "running".into(), summary: None, step_count: 0,
                        parent_session_id: Some("tree-root".into()), depth: 1, project_path: None,
                    },
                    children: vec![],
                },
            ],
        };
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: AgentTreeNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session.id, "tree-root");
        assert_eq!(deserialized.children.len(), 1);
        assert_eq!(deserialized.children[0].session.id, "tree-child");
        assert_eq!(deserialized.children[0].session.depth, 1);
    }

    #[test]
    fn test_full_lifecycle() {
        // Insert session -> messages -> steps -> get detail -> finish -> verify status
        let store = open_temp();

        store.insert_session("life-1", "Full lifecycle test", "claude", "claude-3").unwrap();

        // Verify initial state
        let session = store.get_session("life-1").unwrap().unwrap();
        assert_eq!(session.status, "running");
        assert!(session.finished_at.is_none());

        // Insert messages
        store.insert_message("life-1", "system", "You are a helpful assistant").unwrap();
        store.insert_message("life-1", "user", "Help me refactor").unwrap();
        store.insert_message("life-1", "assistant", "I will help you refactor").unwrap();

        // Insert steps
        store.insert_step("life-1", 1, "read_file", "src/main.rs", "fn main() {}", true).unwrap();
        store.insert_step("life-1", 2, "write_file", "src/main.rs", "written", true).unwrap();
        store.insert_step("life-1", 3, "bash", "cargo test", "test failed", false).unwrap();

        // Verify messages
        let messages = store.get_messages("life-1").unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[2].role, "assistant");

        // Verify steps
        let steps = store.get_steps("life-1").unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].tool_name, "read_file");
        assert_eq!(steps[1].tool_name, "write_file");
        assert_eq!(steps[2].tool_name, "bash");
        assert!(steps[0].success);
        assert!(!steps[2].success);

        // Finish session
        store.finish_session("life-1", "complete", Some("Refactored with minor test failure")).unwrap();

        // Verify final state
        let session = store.get_session("life-1").unwrap().unwrap();
        assert_eq!(session.status, "complete");
        assert!(session.finished_at.is_some());
        assert_eq!(session.step_count, 3);
        assert_eq!(session.summary.as_deref(), Some("Refactored with minor test failure"));

        // Verify full detail
        let detail = store.get_session_detail("life-1").unwrap().unwrap();
        assert_eq!(detail.messages.len(), 3);
        assert_eq!(detail.steps.len(), 3);
    }

    #[test]
    fn test_list_sessions_ordering_newest_first() {
        let store = open_temp();
        // Insert sessions with small delays to guarantee ordering
        store.insert_session("old", "Old task", "p", "m").unwrap();
        // Force a small delay so started_at differs
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.insert_session("mid", "Mid task", "p", "m").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.insert_session("new", "New task", "p", "m").unwrap();

        let sessions = store.list_sessions(10).unwrap();
        assert_eq!(sessions.len(), 3);
        // Newest first
        assert_eq!(sessions[0].id, "new");
        assert_eq!(sessions[1].id, "mid");
        assert_eq!(sessions[2].id, "old");
    }

    #[test]
    fn test_list_sessions_limit() {
        let store = open_temp();
        for i in 0..10 {
            store.insert_session(&format!("s{}", i), &format!("Task {}", i), "p", "m").unwrap();
        }
        let sessions = store.list_sessions(3).unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_get_session_nonexistent() {
        let store = open_temp();
        let result = store.get_session("does-not-exist").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_tree_nonexistent_root() {
        let store = open_temp();
        let result = store.get_tree("no-such-root").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_children_no_children() {
        let store = open_temp();
        store.insert_session("lonely", "No children", "p", "m").unwrap();
        let children = store.get_children("lonely").unwrap();
        assert!(children.is_empty());
    }

    #[test]
    fn test_list_root_sessions_excludes_children() {
        let store = open_temp();
        store.insert_session("r1", "Root 1", "p", "m").unwrap();
        store.insert_session("r2", "Root 2", "p", "m").unwrap();
        store.insert_session_with_parent("c1", "Child of r1", "p", "m", Some("r1"), 1).unwrap();
        store.insert_session_with_parent("c2", "Child of r1", "p", "m", Some("r1"), 1).unwrap();
        store.insert_session_with_parent("gc1", "Grandchild", "p", "m", Some("c1"), 2).unwrap();

        let roots = store.list_root_sessions(100).unwrap();
        // Only r1 and r2 should appear, not c1/c2/gc1
        assert_eq!(roots.len(), 2);
        let root_ids: Vec<&str> = roots.iter().map(|s| s.id.as_str()).collect();
        assert!(root_ids.contains(&"r1"));
        assert!(root_ids.contains(&"r2"));
        assert!(!root_ids.contains(&"c1"));
    }

    #[test]
    fn test_chrono_simple_known_dates() {
        // 1970-01-01 00:00
        assert_eq!(chrono_simple(0), "1970-01-01 00:00");
        // 2000-01-01 00:00 UTC = 946684800 seconds
        assert_eq!(chrono_simple(946_684_800), "2000-01-01 00:00");
        // 2024-03-15 14:30 UTC = ?
        // 2024 is a leap year. Days from 1970-01-01 to 2024-03-15:
        // Let's verify with the function
        let result = chrono_simple(1_710_513_000); // 2024-03-15 14:30:00 UTC
        assert_eq!(result, "2024-03-15 14:30");
    }

    #[test]
    fn test_insert_session_duplicate_ignored() {
        let store = open_temp();
        store.insert_session("dup", "First", "p1", "m1").unwrap();
        // INSERT OR IGNORE — second insert with same ID should succeed silently
        store.insert_session("dup", "Second", "p2", "m2").unwrap();
        let session = store.get_session("dup").unwrap().unwrap();
        // The first insert wins
        assert_eq!(session.task, "First");
        assert_eq!(session.provider, "p1");
    }

    #[test]
    fn test_search_case_insensitive() {
        let store = open_temp();
        store.insert_session("s1", "Build the PARSER", "claude", "claude-3").unwrap();
        // Search with lowercase should match uppercase task
        let results = store.search("parser").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");
        // And vice versa
        let results2 = store.search("PARSER").unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_search_matches_in_messages() {
        let store = open_temp();
        store.insert_session("s1", "Generic task", "p", "m").unwrap();
        store.insert_message("s1", "assistant", "Found a segfault in the allocator").unwrap();
        // Search for word only present in message content
        let results = store.search("segfault").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s1");
    }

    #[test]
    fn test_get_tree_deep_hierarchy() {
        let store = open_temp();
        store.insert_session("d0", "Depth 0", "p", "m").unwrap();
        store.insert_session_with_parent("d1", "Depth 1", "p", "m", Some("d0"), 1).unwrap();
        store.insert_session_with_parent("d2", "Depth 2", "p", "m", Some("d1"), 2).unwrap();
        store.insert_session_with_parent("d3", "Depth 3", "p", "m", Some("d2"), 3).unwrap();

        let tree = store.get_tree("d0").unwrap().unwrap();
        assert_eq!(tree.session.id, "d0");
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].session.id, "d1");
        assert_eq!(tree.children[0].children[0].session.id, "d2");
        assert_eq!(tree.children[0].children[0].children[0].session.id, "d3");
        assert!(tree.children[0].children[0].children[0].children.is_empty());
    }

    // ── Additional pure-function and edge-case tests ─────────────────────────

    #[test]
    fn test_escape_html_single_quote_passthrough() {
        // The current implementation does NOT escape single quotes.
        // Document this behavior explicitly (the function only escapes &, <, >, ").
        let input = "it's a test";
        let out = escape_html(input);
        assert_eq!(out, "it's a test", "Single quotes should pass through unchanged");
    }

    #[test]
    fn test_escape_html_empty_string() {
        assert_eq!(escape_html(""), "");
    }

    #[test]
    fn test_escape_html_only_special_chars() {
        assert_eq!(escape_html("&<>\""), "&amp;&lt;&gt;&quot;");
    }

    #[test]
    fn test_escape_html_nested_html_tags() {
        let input = "<div class=\"evil\"><script>alert('xss')</script></div>";
        let out = escape_html(input);
        assert!(!out.contains('<'), "No raw '<' should remain");
        assert!(!out.contains('>'), "No raw '>' should remain");
        assert!(out.contains("&lt;div"), "Opening tag should be escaped");
        assert!(out.contains("&lt;/div&gt;"), "Closing tag should be escaped");
        assert!(out.contains("&lt;script&gt;"), "Script tag should be escaped");
    }

    #[test]
    fn test_escape_html_double_ampersand_no_double_escape() {
        // "&amp;" in input: the & should be escaped, producing "&amp;amp;"
        let input = "&amp;";
        let out = escape_html(input);
        assert_eq!(out, "&amp;amp;", "Ampersand in entity-like sequences should still be escaped");
    }

    #[test]
    fn test_chrono_simple_unix_epoch() {
        assert_eq!(chrono_simple(0), "1970-01-01 00:00");
    }

    #[test]
    fn test_chrono_simple_leap_year_feb29() {
        // 2024-02-29 00:00 UTC = 1709164800 seconds since epoch
        assert_eq!(chrono_simple(1_709_164_800), "2024-02-29 00:00");
    }

    #[test]
    fn test_chrono_simple_end_of_day() {
        // 2024-01-01 23:59 UTC = 946684800 + (24*365+6)*86400 - 60 ... compute directly:
        // 2024-01-01 23:59:00 = 1704153540
        let result = chrono_simple(1_704_153_540);
        assert_eq!(result, "2024-01-01 23:59");
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_leap_year_boundary() {
        // 2024-02-29: days since epoch = 19782
        let (y, m, d) = days_to_ymd(19_782);
        assert_eq!((y, m, d), (2024, 2, 29));
    }

    #[test]
    fn test_days_to_ymd_new_years_eve() {
        // 2023-12-31: days since epoch = 19722
        let (y, m, d) = days_to_ymd(19_722);
        assert_eq!((y, m, d), (2023, 12, 31));
    }

    #[test]
    fn test_format_ts_with_time_component() {
        // 2024-06-15 18:30 UTC = 1718476200 seconds = 1718476200000 ms
        let result = format_ts(1_718_476_200_000);
        assert_eq!(result, "2024-06-15 18:30");
    }

    #[test]
    fn test_format_age_future_timestamp_saturates() {
        // A timestamp in the far future should saturate to "0s ago" (no underflow)
        let far_future_ms = u64::MAX / 2;
        let result = format_age(far_future_ms);
        assert_eq!(result, "0s ago", "Future timestamps should show 0s ago due to saturating_sub");
    }

    #[test]
    fn test_render_sessions_index_html_xss_in_task() {
        let sessions = vec![SessionRow {
            id: "xss-1".into(),
            task: "<img src=x onerror=alert(1)>".into(),
            provider: "p".into(),
            model: "m".into(),
            started_at: 1_700_000_000_000,
            finished_at: None,
            status: "running".into(),
            summary: None,
            step_count: 0,
            parent_session_id: None,
            depth: 0,
            project_path: None,
        }];
        let html = render_sessions_index_html(&sessions);
        assert!(!html.contains("<img"), "XSS payload in task must be escaped in index HTML");
        assert!(html.contains("&lt;img"), "Angle brackets in task should be escaped");
    }

    #[test]
    fn test_render_sessions_index_html_xss_in_provider() {
        let sessions = vec![SessionRow {
            id: "xss-2".into(),
            task: "Normal task".into(),
            provider: "\"><script>alert(1)</script>".into(),
            model: "m".into(),
            started_at: 1_700_000_000_000,
            finished_at: None,
            status: "complete".into(),
            summary: None,
            step_count: 0,
            parent_session_id: None,
            depth: 0,
            project_path: None,
        }];
        let html = render_sessions_index_html(&sessions);
        assert!(!html.contains("<script>"), "XSS payload in provider must be escaped");
    }

    #[test]
    fn test_render_sessions_index_html_task_truncation() {
        let long_task: String = "A".repeat(120);
        let sessions = vec![SessionRow {
            id: "trunc-1".into(),
            task: long_task.clone(),
            provider: "p".into(),
            model: "m".into(),
            started_at: 1_700_000_000_000,
            finished_at: None,
            status: "running".into(),
            summary: None,
            step_count: 0,
            parent_session_id: None,
            depth: 0,
            project_path: None,
        }];
        let html = render_sessions_index_html(&sessions);
        // The task should be truncated to 80 chars in the index view
        let full_a_120 = "A".repeat(120);
        assert!(!html.contains(&full_a_120), "Full 120-char task should be truncated");
        // Should contain the first 80 chars
        let a_80 = "A".repeat(80);
        assert!(html.contains(&a_80), "Index should contain first 80 chars of task");
    }

    #[test]
    fn test_search_sql_wildcard_injection() {
        // Ensure that SQL LIKE wildcards (% and _) in user input are escaped
        let store = open_temp();
        store.insert_session("s1", "100% complete", "p", "m").unwrap();
        store.insert_session("s2", "a_b_c pattern", "p", "m").unwrap();
        store.insert_session("s3", "normal task", "p", "m").unwrap();

        // Searching for "%" should only match sessions that literally contain %
        let results = store.search("%").unwrap();
        assert_eq!(results.len(), 1, "% wildcard should be escaped, matching only literal %");
        assert_eq!(results[0].id, "s1");

        // Searching for "_" should only match sessions that literally contain _
        let results = store.search("_").unwrap();
        assert_eq!(results.len(), 1, "_ wildcard should be escaped, matching only literal _");
        assert_eq!(results[0].id, "s2");
    }

    #[test]
    fn test_search_whitespace_only_query() {
        let store = open_temp();
        store.insert_session("s1", "Task one", "p", "m").unwrap();
        // Whitespace-only query should behave like empty (split_whitespace yields nothing)
        let results = store.search("   \t  ").unwrap();
        assert_eq!(results.len(), 1, "Whitespace-only query should fall back to list_sessions");
    }

    #[test]
    fn test_render_session_html_duration_calculation() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "dur-1".into(),
                task: "Duration test".into(),
                provider: "p".into(),
                model: "".into(), // empty model
                started_at: 1_700_000_000_000,
                finished_at: Some(1_700_000_065_000), // 65 seconds later
                status: "complete".into(),
                summary: None,
                step_count: 0,
                parent_session_id: None,
                depth: 0,
                project_path: None,
            },
            messages: vec![],
            steps: vec![],
        };
        let html = render_session_html(&detail);
        // Duration should be 65s
        assert!(html.contains("65s"), "Duration should show 65s for 65000ms difference");
        // Empty model should not produce " / " separator
        assert!(!html.contains(" / "), "Empty model should not add model separator");
    }

    #[test]
    fn test_render_session_html_step_singular() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "sing-1".into(),
                task: "Singular step".into(),
                provider: "p".into(),
                model: "m".into(),
                started_at: 1_700_000_000_000,
                finished_at: Some(1_700_000_010_000),
                status: "complete".into(),
                summary: None,
                step_count: 1,
                parent_session_id: None,
                depth: 0,
                project_path: None,
            },
            messages: vec![],
            steps: vec![],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("1 step"), "Should show '1 step' (singular)");
        // Make sure it doesn't say "1 steps"
        assert!(!html.contains("1 steps"), "Should not show '1 steps' (plural)");
    }

    #[test]
    fn test_count_after_duplicate_insert() {
        let store = open_temp();
        store.insert_session("dup", "First", "p", "m").unwrap();
        store.insert_session("dup", "Second", "p", "m").unwrap(); // INSERT OR IGNORE
        assert_eq!(store.count().unwrap(), 1, "Duplicate insert should not increase count");
    }

    // ── escape_html additional edge cases ─────────────────────────────────

    #[test]
    fn test_escape_html_consecutive_special_chars() {
        assert_eq!(escape_html("<<>>&&\"\""), "&lt;&lt;&gt;&gt;&amp;&amp;&quot;&quot;");
    }

    #[test]
    fn test_escape_html_mixed_normal_and_special() {
        assert_eq!(
            escape_html("Hello <world> & \"friends\""),
            "Hello &lt;world&gt; &amp; &quot;friends&quot;"
        );
    }

    // ── chrono_simple / days_to_ymd additional cases ──────────────────────

    #[test]
    fn test_chrono_simple_y2k() {
        // 2000-01-01 00:00 UTC = 946684800
        assert_eq!(chrono_simple(946_684_800), "2000-01-01 00:00");
    }

    #[test]
    fn test_chrono_simple_midday_1970() {
        // 1970-01-01 12:30 UTC = 45000
        assert_eq!(chrono_simple(45_000), "1970-01-01 12:30");
    }

    #[test]
    fn test_chrono_simple_end_of_year_2023() {
        // 2023-12-31 23:59 UTC = 1704067140
        assert_eq!(chrono_simple(1_704_067_140), "2023-12-31 23:59");
    }

    #[test]
    fn test_days_to_ymd_2023_feb28_non_leap() {
        let (y, m, d) = days_to_ymd(19_416);
        assert_eq!((y, m, d), (2023, 2, 28));
    }

    #[test]
    fn test_days_to_ymd_2000_feb29_leap() {
        let (y, m, d) = days_to_ymd(11_016);
        assert_eq!((y, m, d), (2000, 2, 29));
    }

    #[test]
    fn test_days_to_ymd_1900_century_non_leap() {
        // 1900-03-01
        let (y, m, d) = days_to_ymd(-25_508);
        assert_eq!((y, m, d), (1900, 3, 1));
    }

    // ── format_ts edge cases ──────────────────────────────────────────────

    #[test]
    fn test_format_ts_sub_second_ms_rounds() {
        assert_eq!(format_ts(1500), "1970-01-01 00:00");
    }

    #[test]
    fn test_format_ts_exact_one_hour_ms() {
        // 3600000 ms = 3600s = 1 hour
        assert_eq!(format_ts(3_600_000), "1970-01-01 01:00");
    }

    // ── SessionStore: search case insensitive ─────────────────────────────

    #[test]
    fn test_search_case_insensitive_matching() {
        let store = open_temp();
        store.insert_session("ci1", "Build the PARSER module", "claude", "c3").unwrap();
        let results = store.search("parser").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "ci1");
    }

    #[test]
    fn test_search_matches_in_message_content() {
        let store = open_temp();
        store.insert_session("sm1", "Generic task", "p", "m").unwrap();
        store.insert_message("sm1", "assistant", "The authentication module needs work").unwrap();
        let results = store.search("authentication").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "sm1");
    }

    #[test]
    fn test_search_three_keyword_and_semantics() {
        let store = open_temp();
        store.insert_session("tk1", "rust auth parser test", "p", "m").unwrap();
        store.insert_session("tk2", "rust auth helper", "p", "m").unwrap();
        store.insert_session("tk3", "rust parser deploy", "p", "m").unwrap();
        let results = store.search("rust auth parser").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "tk1");
    }

    // ── SessionStore: finish edge cases ───────────────────────────────────

    #[test]
    fn test_finish_updates_step_count_correctly() {
        let store = open_temp();
        store.insert_session("fsc1", "step counting", "p", "m").unwrap();
        store.insert_step("fsc1", 1, "read_file", "a.rs", "ok", true).unwrap();
        store.insert_step("fsc1", 2, "write_file", "b.rs", "ok", true).unwrap();
        store.insert_step("fsc1", 3, "bash", "ls", "ok", true).unwrap();
        store.finish_session("fsc1", "complete", None).unwrap();
        let s = store.get_session("fsc1").unwrap().unwrap();
        assert_eq!(s.step_count, 3);
    }

    // ── render_session_html: step with failure ────────────────────────────

    #[test]
    fn test_render_session_html_failed_step_output() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "sf1".into(), task: "Step failure test".into(),
                provider: "p".into(), model: "m".into(),
                started_at: 1_700_000_000_000, finished_at: Some(1_700_000_010_000),
                status: "complete".into(), summary: None,
                step_count: 1, parent_session_id: None, depth: 0, project_path: None,
            },
            messages: vec![],
            steps: vec![StepRow {
                id: 1, session_id: "sf1".into(), step_num: 1,
                tool_name: "bash".into(), input_summary: "rm -rf /".into(),
                output: "Permission denied".into(), success: false,
                created_at: 1_700_000_005_000,
            }],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("Tool Calls"), "Should have tool calls section");
        assert!(html.contains("bash"), "Should show tool name");
        assert!(html.contains("Permission denied"), "Should show output");
    }

    #[test]
    fn test_render_session_html_system_message_role_class() {
        let detail = SessionDetail {
            session: SessionRow {
                id: "sysm1".into(), task: "System msg test".into(),
                provider: "p".into(), model: "m".into(),
                started_at: 1_700_000_000_000, finished_at: Some(1_700_000_010_000),
                status: "complete".into(), summary: None,
                step_count: 0, parent_session_id: None, depth: 0, project_path: None,
            },
            messages: vec![MessageRow {
                id: 1, session_id: "sysm1".into(), role: "system".into(),
                content: "You are a helpful assistant".into(), created_at: 1_700_000_000_000,
            }],
            steps: vec![],
        };
        let html = render_session_html(&detail);
        assert!(html.contains("msg system"), "System message should have system class");
        assert!(html.contains("You are a helpful assistant"));
    }

    // ── render_sessions_index_html: status badges ─────────────────────────

    #[test]
    fn test_render_sessions_index_all_status_badges() {
        let sessions = vec![
            SessionRow {
                id: "c1".into(), task: "Complete".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_003_000, finished_at: Some(1_700_000_010_000),
                status: "complete".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            SessionRow {
                id: "f1".into(), task: "Failed".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_002_000, finished_at: Some(1_700_000_010_000),
                status: "failed".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
            SessionRow {
                id: "r1".into(), task: "Running".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_001_000, finished_at: None,
                status: "running".into(), summary: None, step_count: 0,
                parent_session_id: None, depth: 0, project_path: None,
            },
        ];
        let html = render_sessions_index_html(&sessions);
        assert!(html.contains("badge ok"), "Should have complete badge");
        assert!(html.contains("badge err"), "Should have failed badge");
        assert!(html.contains("badge run"), "Should have running badge");
    }

    // ── Deep tree traversal ───────────────────────────────────────────────

    #[test]
    fn test_get_tree_deep_four_levels() {
        let store = open_temp();
        store.insert_session("d0", "Root", "p", "m").unwrap();
        store.insert_session_with_parent("d1", "Depth 1", "p", "m", Some("d0"), 1).unwrap();
        store.insert_session_with_parent("d2", "Depth 2", "p", "m", Some("d1"), 2).unwrap();
        store.insert_session_with_parent("d3", "Depth 3", "p", "m", Some("d2"), 3).unwrap();
        let tree = store.get_tree("d0").unwrap().unwrap();
        assert_eq!(tree.session.id, "d0");
        assert_eq!(tree.children[0].session.id, "d1");
        assert_eq!(tree.children[0].children[0].session.id, "d2");
        assert_eq!(tree.children[0].children[0].children[0].session.id, "d3");
        assert!(tree.children[0].children[0].children[0].children.is_empty());
    }

    // ── Steps returned in order ───────────────────────────────────────────

    #[test]
    fn test_steps_ordered_by_step_num() {
        let store = open_temp();
        store.insert_session("ord", "Step order test", "p", "m").unwrap();
        store.insert_step("ord", 3, "tool_c", "", "", true).unwrap();
        store.insert_step("ord", 1, "tool_a", "", "", true).unwrap();
        store.insert_step("ord", 2, "tool_b", "", "", true).unwrap();
        let steps = store.get_steps("ord").unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].step_num, 1);
        assert_eq!(steps[1].step_num, 2);
        assert_eq!(steps[2].step_num, 3);
    }

    // ── Messages for nonexistent session ──────────────────────────────────

    #[test]
    fn test_get_messages_empty_for_no_session() {
        let store = open_temp();
        let msgs = store.get_messages("no_such_session").unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_get_steps_empty_for_no_session() {
        let store = open_temp();
        let steps = store.get_steps("no_such_session").unwrap();
        assert!(steps.is_empty());
    }

    // ── Phase 1: FTS5 + project scoping ───────────────────────────────────
    // These tests drive the new API. They reference `SearchScope`, `FtsHit`,
    // `insert_session_with_project`, `search_fts`, and `delete_session` —
    // all of which are introduced alongside the FTS5 virtual table.

    #[test]
    fn test_insert_session_with_project_stores_path() {
        let store = open_temp();
        store
            .insert_session_with_project(
                "s1",
                "refactor auth module",
                "claude",
                "claude-3",
                "/Users/me/projects/app",
            )
            .unwrap();
        let s = store.get_session("s1").unwrap().unwrap();
        assert_eq!(s.project_path.as_deref(), Some("/Users/me/projects/app"));
    }

    #[test]
    fn test_fts_search_finds_message_by_content() {
        let store = open_temp();
        store
            .insert_session_with_project("s1", "task", "claude", "claude-3", "/p/a")
            .unwrap();
        store
            .insert_message("s1", "user", "please refactor the authentication flow")
            .unwrap();
        store
            .insert_message("s1", "assistant", "I'll start with the login module")
            .unwrap();

        let hits = store.search_fts("authentication", SearchScope::All, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].session_id, "s1");
        assert!(hits[0].content.contains("authentication"));
    }

    #[test]
    fn test_fts_search_scoped_to_project() {
        let store = open_temp();
        store
            .insert_session_with_project("s1", "t", "c", "m", "/p/alpha")
            .unwrap();
        store
            .insert_session_with_project("s2", "t", "c", "m", "/p/beta")
            .unwrap();
        store.insert_message("s1", "user", "deploy the service").unwrap();
        store.insert_message("s2", "user", "deploy the service").unwrap();

        let hits_alpha = store
            .search_fts("deploy", SearchScope::Project("/p/alpha".into()), 10)
            .unwrap();
        assert_eq!(hits_alpha.len(), 1);
        assert_eq!(hits_alpha[0].session_id, "s1");

        let hits_all = store.search_fts("deploy", SearchScope::All, 10).unwrap();
        assert_eq!(hits_all.len(), 2);
    }

    #[test]
    fn test_fts_cascade_on_session_delete() {
        let store = open_temp();
        store
            .insert_session_with_project("s1", "t", "c", "m", "/p/x")
            .unwrap();
        store
            .insert_message("s1", "user", "searchable unique phrase xyzzy")
            .unwrap();
        assert_eq!(
            store.search_fts("xyzzy", SearchScope::All, 10).unwrap().len(),
            1,
            "message should be in FTS before delete"
        );

        store.delete_session("s1").unwrap();

        assert_eq!(
            store.search_fts("xyzzy", SearchScope::All, 10).unwrap().len(),
            0,
            "FTS rows should be cleaned up when the session is deleted"
        );
    }

    #[test]
    fn test_fts_snippet_contains_highlight_markers() {
        let store = open_temp();
        store
            .insert_session_with_project("s1", "t", "c", "m", "/p/x")
            .unwrap();
        store
            .insert_message(
                "s1",
                "user",
                "the quick brown fox jumped over the lazy dog many times",
            )
            .unwrap();

        let hits = store.search_fts("fox", SearchScope::All, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0].snippet.contains("<mark>"),
            "snippet should contain highlight markers, got: {}",
            hits[0].snippet
        );
    }

    #[test]
    fn test_fts_backfills_existing_messages_on_migration() {
        // A fresh store inserts messages normally. Re-opening must not lose FTS rows.
        let f = tempfile::NamedTempFile::new().unwrap();
        let path = f.path().to_owned();
        std::mem::forget(f);
        {
            let store = SessionStore::open(&path).unwrap();
            store
                .insert_session_with_project("s1", "t", "c", "m", "/p/x")
                .unwrap();
            store
                .insert_message("s1", "user", "persistent content here")
                .unwrap();
        }
        // Re-open — migration/index must survive.
        let store = SessionStore::open(&path).unwrap();
        let hits = store.search_fts("persistent", SearchScope::All, 10).unwrap();
        assert_eq!(hits.len(), 1, "FTS entries must persist across open()");
    }

    // ── F1.1 recap CRUD ──────────────────────────────────────────────────

    /// Build a heuristic recap against a fresh tempdir-scoped store
    /// with one user message. Returns (store, session_id, recap).
    fn store_with_recap_fixture() -> (SessionStore, String, crate::recap::Recap) {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().join("sessions.db")).unwrap();
        // Keep dir alive by leaking — tests are short-lived.
        std::mem::forget(dir);
        let sid = "F1-recap-test".to_string();
        store
            .insert_session_with_parent(&sid, "test task", "mock", "test-model", None, 0)
            .unwrap();
        store.insert_message(&sid, "user", "Refactor auth module").unwrap();
        let detail = store
            .get_session_detail(&sid)
            .unwrap()
            .expect("session must exist");
        let recap = crate::recap::heuristic_recap(&detail);
        (store, sid, recap)
    }

    #[test]
    fn insert_recap_round_trips_full_shape() {
        let (store, _sid, recap) = store_with_recap_fixture();
        let stored = store.insert_recap(&recap).expect("insert");
        let fetched = store
            .get_recap_by_id(&recap.id)
            .expect("fetch")
            .expect("recap row must exist");
        assert_eq!(fetched, stored);
        assert_eq!(fetched.headline, recap.headline);
        assert_eq!(fetched.bullets, recap.bullets);
        assert_eq!(fetched.subject_id, recap.subject_id);
    }

    #[test]
    fn insert_recap_idempotent_on_subject_and_last_message_id() {
        // Pin: re-insert with the same (subject_id, last_message_id)
        // returns the existing row, original `id` preserved. This is
        // the behavior on which `POST /v1/recap` (F1.2) builds the
        // "force=false ⇒ return current recap" semantics.
        let (store, _sid, recap1) = store_with_recap_fixture();
        let _ = store.insert_recap(&recap1).expect("first insert");
        // Build a *different* recap shape against the same fixture
        // (same subject_id + same last_message_id), then re-insert.
        let mut recap2 = recap1.clone();
        recap2.id = "different-id".to_string();
        recap2.headline = "Different headline".to_string();
        let stored2 = store.insert_recap(&recap2).expect("second insert");
        // Idempotency: stored result is the *original* recap, not the
        // new shape. Headline must match the first insert.
        assert_eq!(stored2.id, recap1.id);
        assert_eq!(stored2.headline, recap1.headline);
        // And exactly one row exists for this subject.
        let list = store
            .list_recaps_for_subject(&recap1.subject_id, 10)
            .expect("list");
        assert_eq!(list.len(), 1, "idempotency must not produce duplicates");
    }

    #[test]
    fn list_recaps_for_subject_orders_newest_first() {
        let (store, sid, recap1) = store_with_recap_fixture();
        let _ = store.insert_recap(&recap1).expect("insert 1");
        // Add a new message so the next recap has a different
        // last_message_id (escapes idempotency).
        store.insert_message(&sid, "assistant", "ack").unwrap();
        let detail = store.get_session_detail(&sid).unwrap().unwrap();
        let mut recap2 = crate::recap::heuristic_recap(&detail);
        // Force a *later* generated_at by bumping by 1s — small but
        // deterministic for the ORDER BY assertion. Without this the
        // two recaps could share a timestamp on fast hardware and the
        // ORDER BY tie-break would be undefined.
        recap2.generated_at =
            recap1.generated_at + chrono::Duration::seconds(1);
        let _ = store.insert_recap(&recap2).expect("insert 2");

        let list = store.list_recaps_for_subject(&sid, 10).expect("list");
        assert_eq!(list.len(), 2);
        assert!(
            list[0].generated_at > list[1].generated_at,
            "list must be newest-first; got {:?} then {:?}",
            list[0].generated_at,
            list[1].generated_at
        );
    }

    #[test]
    fn delete_recap_removes_only_the_targeted_row() {
        let (store, sid, recap1) = store_with_recap_fixture();
        let _ = store.insert_recap(&recap1).expect("insert");
        // Add a second recap on the same subject.
        store.insert_message(&sid, "assistant", "ack").unwrap();
        let detail = store.get_session_detail(&sid).unwrap().unwrap();
        let mut recap2 = crate::recap::heuristic_recap(&detail);
        recap2.generated_at = recap1.generated_at + chrono::Duration::seconds(1);
        let _ = store.insert_recap(&recap2).expect("insert 2");

        store.delete_recap(&recap1.id).expect("delete");
        assert!(
            store.get_recap_by_id(&recap1.id).unwrap().is_none(),
            "deleted recap must be gone"
        );
        assert!(
            store.get_recap_by_id(&recap2.id).unwrap().is_some(),
            "untouched recap must still exist"
        );
    }

    #[test]
    fn get_recap_by_subject_and_last_msg_handles_null_last_message() {
        // An empty session has no last_message_id (None). Inserting a
        // recap with last_message_id = None must round-trip cleanly:
        // the lookup uses `IS NULL` SQL semantics, not parameter
        // binding, because `NULL = NULL` is FALSE in SQL.
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().join("sessions.db")).unwrap();
        std::mem::forget(dir);
        let sid = "empty-session".to_string();
        store
            .insert_session_with_parent(&sid, "task", "mock", "model", None, 0)
            .unwrap();
        let detail = store.get_session_detail(&sid).unwrap().unwrap();
        let recap = crate::recap::heuristic_recap(&detail);
        assert!(recap.last_message_id.is_none());

        let _ = store.insert_recap(&recap).expect("insert");
        let fetched = store
            .get_recap_by_subject_and_last_msg(&sid, None)
            .expect("query")
            .expect("recap with NULL last_message_id must round-trip");
        assert_eq!(fetched.id, recap.id);
    }

    #[test]
    fn cascade_delete_session_removes_recaps() {
        // Pin: deleting a session via FK cascade must take its recaps
        // with it. UIs that "forget this session" rely on this so a
        // forgotten session doesn't leak through the recaps list.
        let (store, sid, recap) = store_with_recap_fixture();
        let _ = store.insert_recap(&recap).unwrap();
        store.delete_session(&sid).unwrap();
        let list = store.list_recaps_for_subject(&sid, 10).unwrap();
        assert!(
            list.is_empty(),
            "FK cascade must delete recaps; got: {list:?}"
        );
    }
}


// ── CLI command helpers ───────────────────────────────────────────────────────

/// CLI handler for `vibecli --fork <session-id>`.
pub async fn fork_session_cmd(source_id: &str) -> anyhow::Result<()> {
    let store = SessionStore::open_default()?;
    let new_id = format!("fork-{}-{}", &source_id[..source_id.len().min(12)],
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
    store.fork_session(source_id, &new_id)?;
    println!("✔ Forked session '{}' → '{}'", source_id, new_id);
    println!("  Resume the fork with: vibecli --resume {}", new_id);
    Ok(())
}

/// A6: AI Session Auto-Naming.
///
/// Generates a concise title for a session from its first user message,
/// using a simple heuristic (no API call needed for the common case).
/// Falls back to the raw task string if the heuristic produces nothing useful.
pub fn auto_name_session(raw_task: &str) -> String {
    // Strip leading/trailing whitespace and truncate long tasks
    let task = raw_task.trim();
    if task.is_empty() { return "Unnamed session".to_string(); }

    // If the task is already short enough, use it as-is
    if task.chars().count() <= 60 { return task.to_string(); }

    // Extract the first sentence (up to '. ', '? ', or '! ')
    for pat in &[". ", "? ", "! ", "\n"] {
        if let Some(idx) = task.find(pat) {
            let sentence = &task[..idx + 1];
            let trimmed = sentence.trim_end_matches(|c: char| !c.is_alphanumeric());
            if trimmed.chars().count() >= 10 {
                return trimmed[..trimmed.len().min(60)].to_string();
            }
        }
    }

    // Fall back: first 60 characters + ellipsis
    let end = task.char_indices()
        .nth(57)
        .map(|(i, _)| i)
        .unwrap_or(task.len());
    format!("{}…", &task[..end])
}

#[cfg(test)]
mod auto_name_tests {
    use super::auto_name_session;

    #[test]
    fn test_short_task_unchanged() {
        assert_eq!(auto_name_session("Fix login bug"), "Fix login bug");
    }

    #[test]
    fn test_extracts_first_sentence() {
        let t = "Implement OAuth2. Then add tests for each flow and document everything.";
        assert_eq!(auto_name_session(t), "Implement OAuth2");
    }

    #[test]
    fn test_truncates_long_no_sentence() {
        let t = "a".repeat(100);
        assert!(auto_name_session(&t).ends_with('…'));
        assert!(auto_name_session(&t).chars().count() <= 60);
    }
}
