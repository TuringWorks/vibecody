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
        Ok(())
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
        let now = now_ms();
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions (id, task, provider, model, started_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running')",
            params![id, task, provider, model, now],
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
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count
             FROM sessions ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Get a single session by ID.
    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task, provider, model, started_at, finished_at, status, summary, step_count
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
        let like_pattern = |kw: &str| format!("%{}%", kw.to_lowercase());

        // Collect candidate session IDs from steps + messages + task
        let mut candidate_sets: Vec<std::collections::HashSet<String>> = Vec::new();

        for kw in &keywords {
            let pat = like_pattern(kw);
            let mut stmt = self.conn.prepare(
                "SELECT DISTINCT session_id FROM steps
                 WHERE LOWER(tool_name) LIKE ?1 OR LOWER(input_summary) LIKE ?1
                 UNION
                 SELECT DISTINCT session_id FROM messages
                 WHERE LOWER(content) LIKE ?1
                 UNION
                 SELECT id FROM sessions WHERE LOWER(task) LIKE ?1",
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
            // Truncate very long messages
            let preview = if msg.content.len() > 4000 {
                format!("{}…\n[truncated {} chars]", &msg.content[..4000], msg.content.len())
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
    let dt = chrono_simple(secs);
    dt
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
    fn test_days_to_ymd() {
        // 2024-01-01 = days since 1970-01-01: 19723
        let (y, mo, d) = days_to_ymd(19723);
        assert_eq!(y, 2024);
        assert_eq!(mo, 1);
        assert_eq!(d, 1);
    }
}
