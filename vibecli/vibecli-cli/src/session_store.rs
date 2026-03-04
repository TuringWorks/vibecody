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
        // Index for tree queries.
        let _ = self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);"
        );
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
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth
             FROM sessions ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get a single session by ID.
    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth
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
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth
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
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count, parent_session_id, depth
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
                parent_session_id: None, depth: 0,
            },
            SessionRow {
                id: "s2".into(), task: "Task 2".into(), provider: "p".into(),
                model: "m".into(), started_at: 1_700_000_001_000, finished_at: None,
                status: "failed".into(), summary: None, step_count: 1,
                parent_session_id: None, depth: 0,
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
                parent_session_id: None, depth: 0,
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
                parent_session_id: None, depth: 0,
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
                parent_session_id: None, depth: 0,
            },
            children: vec![
                AgentTreeNode {
                    session: SessionRow {
                        id: "tree-child".into(), task: "Child".into(), provider: "p".into(),
                        model: "m".into(), started_at: 200, finished_at: None,
                        status: "running".into(), summary: None, step_count: 0,
                        parent_session_id: Some("tree-root".into()), depth: 1,
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
}
