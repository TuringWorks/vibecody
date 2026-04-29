//! Durable async job manager — M1 of the async-agents subsystem.
//!
//! Owns the SQLite store and in-memory event streams for background agent
//! jobs. Replaces the JSON-file-per-job persistence that lived in serve.rs.
//!
//! # Storage
//!
//! `~/.vibecli/jobs.db` — encrypted ChaCha20-Poly1305 BLOB columns for
//! `task`, `summary`, and `webhook_url`. Status / timestamps / counters
//! stay plaintext so the queue is queryable without decryption.
//!
//! The encryption key is derived from `SHA-256("vibecli-jobs-store-v1:" +
//! HOME + ":" + USER)` — machine-bound, workspace-agnostic. Async agents
//! follow the user across projects.
//!
//! # Scope (M1)
//!
//! M1 intentionally keeps in-process `tokio::spawn` dispatch. The
//! subprocess + sandbox story lands in M7, and real cancellation in M2.
//! This module therefore exposes durability + event fan-out only; the
//! agent-run orchestration stays in `serve.rs`.

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{broadcast, Mutex};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "complete" => Some(Self::Complete),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Cancelled)
    }
}

/// Durable record of an async agent job. Serialised shape is backward
/// compatible with the pre-M1 JSON files — new fields carry serde
/// defaults so old clients round-trip cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub session_id: String,
    pub task: String,
    /// "queued" | "running" | "complete" | "failed" | "cancelled"
    pub status: String,
    pub provider: String,
    pub started_at: u64,
    #[serde(default)]
    pub finished_at: Option<u64>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub queued_at: u64,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cancellation_reason: Option<String>,
    #[serde(default)]
    pub steps_completed: u64,
    #[serde(default)]
    pub tokens_used: u64,
    #[serde(default)]
    pub cost_cents: u64,
}

fn default_priority() -> u8 {
    5
}

/// Agent event shape shared by the SSE stream and (in M4) the durable
/// event log. Defined here so `serve.rs` is a thin consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventPayload {
    #[serde(rename = "type")]
    pub kind: String,
    pub content: Option<String>,
    pub step_num: Option<usize>,
    pub tool_name: Option<String>,
    pub success: Option<bool>,
}

impl AgentEventPayload {
    pub fn chunk(text: String) -> Self {
        Self {
            kind: "chunk".into(),
            content: Some(text),
            step_num: None,
            tool_name: None,
            success: None,
        }
    }
    pub fn step(step_num: usize, tool: &str, success: bool) -> Self {
        Self {
            kind: "step".into(),
            content: None,
            step_num: Some(step_num),
            tool_name: Some(tool.into()),
            success: Some(success),
        }
    }
    pub fn complete(summary: String) -> Self {
        Self {
            kind: "complete".into(),
            content: Some(summary),
            step_num: None,
            tool_name: None,
            success: None,
        }
    }
    pub fn error(msg: String) -> Self {
        Self {
            kind: "error".into(),
            content: Some(msg),
            step_num: None,
            tool_name: None,
            success: None,
        }
    }
}

#[derive(Debug)]
pub enum SubmitError {
    Storage(String),
    QuotaDenied {
        resource: String,
        used: u64,
        hard_limit: u64,
    },
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(e) => write!(f, "storage error: {e}"),
            Self::QuotaDenied {
                resource,
                used,
                hard_limit,
            } => write!(
                f,
                "quota denied: {resource} used {used} over hard limit {hard_limit}"
            ),
        }
    }
}

impl std::error::Error for SubmitError {}

/// Input to `JobManager::create`. `submitted_at` is filled in by the manager.
#[derive(Debug, Clone)]
pub struct CreateJobReq {
    pub task: String,
    pub provider: String,
    pub approval: String,
    pub workspace_root: String,
    pub priority: u8,
    pub webhook_url: Option<String>,
    pub tags: Vec<String>,
    /// Optional agent bucket for quota enforcement. When `None`, the job is
    /// submitted without consulting the quota engine. When `Some`, a `Tasks`
    /// quota is checked and consumed against this bucket; hard-limit breach
    /// returns `SubmitError::QuotaDenied` and the job is not persisted.
    pub quota_bucket: Option<String>,
}

/// A single entry in the per-job scratchpad — durable working state keyed
/// by `(session_id, key)`. Phase 3 of the memory-as-infrastructure
/// redesign; surfaced to agents via the Context Assembler's
/// `"agent_scratchpad"` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScratchpadEntry {
    pub key: String,
    pub value: String,
    pub updated_at: u64,
}

/// Summary returned by `migrate_json_jobs`.
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    pub imported: usize,
    pub skipped: usize,
    pub backed_up_dir: Option<PathBuf>,
}

/// Persisted outcome of a webhook delivery attempt. Populated by
/// `JobManager::deliver_webhook` once the retry loop terminates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub session_id: String,
    pub url: String,
    /// `"delivered"` or `"dead_letter"`.
    pub status: String,
    pub attempts: u32,
    pub last_http_status: Option<u16>,
    pub last_error: Option<String>,
    pub first_attempt_at: u64,
    pub last_attempt_at: u64,
}

// ── Encryption ────────────────────────────────────────────────────────────────

fn derive_key() -> [u8; 32] {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let mut h = Sha256::new();
    h.update(b"vibecli-jobs-store-v1:");
    h.update(home.as_bytes());
    h.update(b":");
    h.update(user.as_bytes());
    h.finalize().into()
}

fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    rand::rng().fill(&mut nonce);
    let cipher = ChaCha20Poly1305::new(key.into());
    let mut ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&nonce);
    blob.append(&mut ct);
    Ok(blob)
}

fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<String, String> {
    if blob.len() < 13 {
        return Err("blob too short".into());
    }
    let (nonce, ct) = blob.split_at(12);
    let cipher = ChaCha20Poly1305::new(key.into());
    let pt = cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|e| format!("decrypt: {e}"))?;
    String::from_utf8(pt).map_err(|e| format!("utf8: {e}"))
}

fn encrypt_opt(key: &[u8; 32], s: Option<&str>) -> Result<Option<Vec<u8>>, String> {
    s.map(|v| encrypt(key, v)).transpose()
}

fn decrypt_opt(key: &[u8; 32], b: Option<Vec<u8>>) -> Result<Option<String>, String> {
    b.map(|v| decrypt(key, &v)).transpose()
}

// ── DB path + schema ──────────────────────────────────────────────────────────

pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("jobs.db")
}

fn open_conn(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;

         CREATE TABLE IF NOT EXISTS jobs (
             session_id          TEXT    PRIMARY KEY,
             encrypted_task      BLOB    NOT NULL,
             status              TEXT    NOT NULL,
             provider            TEXT    NOT NULL,
             approval            TEXT    NOT NULL DEFAULT 'auto',
             priority            INTEGER NOT NULL DEFAULT 5,
             workspace_root      TEXT    NOT NULL DEFAULT '',
             encrypted_webhook   BLOB,
             tags_json           TEXT    NOT NULL DEFAULT '[]',
             queued_at           INTEGER NOT NULL,
             started_at          INTEGER,
             finished_at         INTEGER,
             encrypted_summary   BLOB,
             cancellation_reason TEXT,
             steps_completed     INTEGER NOT NULL DEFAULT 0,
             tokens_used         INTEGER NOT NULL DEFAULT 0,
             cost_cents          INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS idx_jobs_status_priority
             ON jobs(status, priority DESC, queued_at ASC);
         CREATE INDEX IF NOT EXISTS idx_jobs_started_at ON jobs(started_at DESC);

         CREATE TABLE IF NOT EXISTS jobs_meta (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );

         -- M4: durable event log for SSE reconnect/replay. `seq` is a
         -- per-session monotonic counter, so clients can resume with
         -- ?since_seq=N and receive any events emitted while disconnected.
         -- The payload is stored encrypted because it frequently contains
         -- task content (stream chunks) that are sensitive.
         CREATE TABLE IF NOT EXISTS job_events (
             session_id        TEXT    NOT NULL,
             seq               INTEGER NOT NULL,
             kind              TEXT    NOT NULL,
             encrypted_payload BLOB    NOT NULL,
             created_at        INTEGER NOT NULL,
             PRIMARY KEY (session_id, seq)
         );
         CREATE INDEX IF NOT EXISTS idx_job_events_sid_seq
             ON job_events(session_id, seq);

         -- M5: webhook delivery outcome tracking. One row per job that has
         -- a webhook_url configured. `status` is 'delivered' or
         -- 'dead_letter'. The URL itself is encrypted since it may carry
         -- auth tokens in the query string.
         CREATE TABLE IF NOT EXISTS webhook_deliveries (
             session_id       TEXT PRIMARY KEY,
             encrypted_url    BLOB NOT NULL,
             status           TEXT NOT NULL,
             attempts         INTEGER NOT NULL,
             last_http_status INTEGER,
             last_error       TEXT,
             first_attempt_at INTEGER NOT NULL,
             last_attempt_at  INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status
             ON webhook_deliveries(status, last_attempt_at DESC);

         -- Phase 3 (memory-as-infrastructure): durable agent scratchpad.
         -- A per-session key/value store for an agent's working state —
         -- plans, hypotheses, file-location notes — so a long-running
         -- agent can pick up where it left off after a restart or pause.
         -- Values are encrypted because they frequently contain excerpts
         -- of the task / source code.
         CREATE TABLE IF NOT EXISTS job_scratchpad (
             session_id       TEXT    NOT NULL,
             key              TEXT    NOT NULL,
             encrypted_value  BLOB    NOT NULL,
             updated_at       INTEGER NOT NULL,
             PRIMARY KEY (session_id, key)
         );
         CREATE INDEX IF NOT EXISTS idx_job_scratchpad_sid_updated
             ON job_scratchpad(session_id, updated_at DESC);

         -- Recap & Resume — Phase J1.1. Job-kind recaps live on jobs.db
         -- (matching the surrounding store's encryption posture). Headline
         -- + body are encrypted because they frequently contain task
         -- excerpts that are sensitive. Idempotency on
         -- (subject_id, last_event_seq) — regenerating the same recap
         -- with the same cursor returns the existing row unchanged.
         CREATE TABLE IF NOT EXISTS recaps (
             id                 TEXT PRIMARY KEY,
             kind               TEXT NOT NULL DEFAULT 'job',
             subject_id         TEXT NOT NULL,
             last_event_seq     INTEGER,
             workspace          TEXT,
             generated_at       TEXT NOT NULL,
             generator_kind     TEXT NOT NULL,
             generator_provider TEXT,
             generator_model    TEXT,
             headline_enc       BLOB NOT NULL,
             body_enc           BLOB NOT NULL,
             token_input        INTEGER,
             token_output       INTEGER,
             cost_cents         INTEGER,
             schema_version     INTEGER NOT NULL DEFAULT 1,
             FOREIGN KEY (subject_id) REFERENCES jobs(session_id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_jobrecaps_subject ON recaps(subject_id);
         CREATE UNIQUE INDEX IF NOT EXISTS uq_jobrecaps_subject_seq
             ON recaps(subject_id, last_event_seq);",
    )
    .map_err(|e| e.to_string())?;

    // Idempotent ALTER TABLE pattern for the resume-lineage columns.
    // Errors with "duplicate column" code are swallowed; anything else
    // surfaces. Mirrors the maybe_add_column helper in session_store.rs.
    add_jobs_column_if_missing(&conn, "parent_job_id", "TEXT")?;
    add_jobs_column_if_missing(&conn, "resumed_from_recap_id", "TEXT")?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_jobs_parent ON jobs(parent_job_id);",
    )
    .map_err(|e| e.to_string())?;

    Ok(conn)
}

fn add_jobs_column_if_missing(conn: &Connection, column: &str, ty: &str) -> Result<(), String> {
    let sql = format!("ALTER TABLE jobs ADD COLUMN {column} {ty}");
    match conn.execute(&sql, []) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, Some(msg)))
            if msg.contains("duplicate column name") =>
        {
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// JSON shape stored in the `recaps.body_enc` blob. Carries the parts of
/// the cross-cutting `Recap` shape that aren't already split into
/// dedicated columns. Headline + ids + cursor + generator metadata are
/// columnar; bullets / next_actions / artifacts / resume_hint travel
/// inside this JSON so the schema stays small.
#[derive(serde::Serialize, serde::Deserialize)]
struct JobRecapBody {
    bullets: Vec<String>,
    next_actions: Vec<String>,
    artifacts: Vec<crate::recap::RecapArtifact>,
    resume_hint: Option<crate::recap::ResumeHint>,
}

impl From<&crate::recap::Recap> for JobRecapBody {
    fn from(r: &crate::recap::Recap) -> Self {
        JobRecapBody {
            bullets: r.bullets.clone(),
            next_actions: r.next_actions.clone(),
            artifacts: r.artifacts.clone(),
            resume_hint: r.resume_hint.clone(),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── JobsDb ────────────────────────────────────────────────────────────────────

/// Synchronous SQLite wrapper. Intended to live behind the async-side
/// `Mutex` inside `JobManager` so callers see an async API.
pub struct JobsDb {
    conn: Connection,
    key: [u8; 32],
}

impl JobsDb {
    pub fn open(path: &Path) -> Result<Self, String> {
        Ok(Self {
            conn: open_conn(path)?,
            key: derive_key(),
        })
    }

    /// For tests: open against an arbitrary path with a supplied key.
    pub fn open_with(path: &Path, key: [u8; 32]) -> Result<Self, String> {
        Ok(Self {
            conn: open_conn(path)?,
            key,
        })
    }

    // ── Recap & Resume — Phase J1.1 ─────────────────────────────────────
    //
    // Persistence for job-kind recaps. Headline + body are encrypted to
    // match the surrounding store's posture. Idempotency on
    // (subject_id, last_event_seq): an upsert with the same cursor
    // returns the existing row id unchanged.

    /// Insert a job recap. If a recap already exists for the same
    /// `(subject_id, last_event_seq)` pair, returns its existing id
    /// (idempotent — no rewrite). The `recap.kind` must be `Job`.
    pub fn insert_job_recap(
        &self,
        recap: &crate::recap::Recap,
    ) -> Result<String, String> {
        if !matches!(recap.kind, crate::recap::RecapKind::Job) {
            return Err(format!(
                "insert_job_recap expects RecapKind::Job, got {:?}",
                recap.kind
            ));
        }
        // Last-event-seq lives in the `last_message_id` slot on the
        // shared Recap shape — for jobs the cursor is the event seq,
        // for sessions it's the message id (matching the storage tables).
        let seq = recap.last_message_id;

        if let Some(existing) = self.get_job_recap_by_subject_and_seq(&recap.subject_id, seq)? {
            return Ok(existing.id);
        }

        let body = serde_json::to_string(&JobRecapBody::from(recap))
            .map_err(|e| format!("serialize recap body: {e}"))?;
        let headline_blob = encrypt(&self.key, &recap.headline)?;
        let body_blob = encrypt(&self.key, &body)?;
        let (gen_kind, gen_provider, gen_model) = match &recap.generator {
            crate::recap::RecapGenerator::Heuristic => ("heuristic".to_string(), None, None),
            crate::recap::RecapGenerator::Llm { provider, model } => (
                "llm".to_string(),
                Some(provider.clone()),
                Some(model.clone()),
            ),
            crate::recap::RecapGenerator::UserEdited => ("user_edited".to_string(), None, None),
        };
        let workspace = recap
            .workspace
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        let token_in = recap.token_usage.as_ref().map(|t| t.input as i64);
        let token_out = recap.token_usage.as_ref().map(|t| t.output as i64);
        self.conn
            .execute(
                "INSERT INTO recaps (id, kind, subject_id, last_event_seq, workspace,
                                     generated_at, generator_kind, generator_provider,
                                     generator_model, headline_enc, body_enc,
                                     token_input, token_output, schema_version)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    recap.id,
                    "job",
                    recap.subject_id,
                    seq,
                    workspace,
                    recap.generated_at.to_rfc3339(),
                    gen_kind,
                    gen_provider,
                    gen_model,
                    headline_blob,
                    body_blob,
                    token_in,
                    token_out,
                    recap.schema_version as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(recap.id.clone())
    }

    pub fn get_job_recap_by_id(
        &self,
        id: &str,
    ) -> Result<Option<crate::recap::Recap>, String> {
        self.fetch_recap_one(
            "SELECT id, subject_id, last_event_seq, workspace, generated_at,
                    generator_kind, generator_provider, generator_model,
                    headline_enc, body_enc, token_input, token_output, schema_version
             FROM recaps WHERE id = ?1",
            params![id],
        )
    }

    pub fn get_job_recap_by_subject_and_seq(
        &self,
        subject_id: &str,
        last_event_seq: Option<i64>,
    ) -> Result<Option<crate::recap::Recap>, String> {
        // SQLite NULL semantics: `WHERE last_event_seq = NULL` doesn't
        // match. Use IS NULL when seq is None.
        match last_event_seq {
            Some(seq) => self.fetch_recap_one(
                "SELECT id, subject_id, last_event_seq, workspace, generated_at,
                        generator_kind, generator_provider, generator_model,
                        headline_enc, body_enc, token_input, token_output, schema_version
                 FROM recaps WHERE subject_id = ?1 AND last_event_seq = ?2",
                params![subject_id, seq],
            ),
            None => self.fetch_recap_one(
                "SELECT id, subject_id, last_event_seq, workspace, generated_at,
                        generator_kind, generator_provider, generator_model,
                        headline_enc, body_enc, token_input, token_output, schema_version
                 FROM recaps WHERE subject_id = ?1 AND last_event_seq IS NULL",
                params![subject_id],
            ),
        }
    }

    pub fn list_job_recaps_for_subject(
        &self,
        subject_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::recap::Recap>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, subject_id, last_event_seq, workspace, generated_at,
                        generator_kind, generator_provider, generator_model,
                        headline_enc, body_enc, token_input, token_output, schema_version
                 FROM recaps
                 WHERE subject_id = ?1
                 ORDER BY generated_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![subject_id, limit as i64], |row| Ok(self.row_to_recap(row)))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?.map_err(|e| e)?);
        }
        Ok(out)
    }

    pub fn delete_job_recap(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM recaps WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn fetch_recap_one(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Option<crate::recap::Recap>, String> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let row = stmt.query_row(params, |row| Ok(self.row_to_recap(row)));
        match row {
            Ok(Ok(r)) => Ok(Some(r)),
            Ok(Err(e)) => Err(e),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn row_to_recap(&self, row: &rusqlite::Row) -> Result<crate::recap::Recap, String> {
        use chrono::{DateTime, Utc};
        let id: String = row.get(0).map_err(|e| e.to_string())?;
        let subject_id: String = row.get(1).map_err(|e| e.to_string())?;
        let last_event_seq: Option<i64> = row.get(2).map_err(|e| e.to_string())?;
        let workspace: Option<String> = row.get(3).map_err(|e| e.to_string())?;
        let generated_at_str: String = row.get(4).map_err(|e| e.to_string())?;
        let generator_kind: String = row.get(5).map_err(|e| e.to_string())?;
        let generator_provider: Option<String> = row.get(6).map_err(|e| e.to_string())?;
        let generator_model: Option<String> = row.get(7).map_err(|e| e.to_string())?;
        let headline_enc: Vec<u8> = row.get(8).map_err(|e| e.to_string())?;
        let body_enc: Vec<u8> = row.get(9).map_err(|e| e.to_string())?;
        let token_input: Option<i64> = row.get(10).map_err(|e| e.to_string())?;
        let token_output: Option<i64> = row.get(11).map_err(|e| e.to_string())?;
        let schema_version: i64 = row.get(12).map_err(|e| e.to_string())?;

        let headline = decrypt(&self.key, &headline_enc)?;
        let body_json = decrypt(&self.key, &body_enc)?;
        let body: JobRecapBody =
            serde_json::from_str(&body_json).map_err(|e| format!("decode body: {e}"))?;

        let generator = match generator_kind.as_str() {
            "heuristic" => crate::recap::RecapGenerator::Heuristic,
            "llm" => crate::recap::RecapGenerator::Llm {
                provider: generator_provider.unwrap_or_default(),
                model: generator_model.unwrap_or_default(),
            },
            "user_edited" => crate::recap::RecapGenerator::UserEdited,
            other => return Err(format!("unknown generator_kind {other}")),
        };
        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)
            .map_err(|e| format!("generated_at parse: {e}"))?
            .with_timezone(&Utc);

        let token_usage = match (token_input, token_output) {
            (Some(i), Some(o)) => Some(crate::recap::RecapTokenUsage {
                input: i as u32,
                output: o as u32,
            }),
            _ => None,
        };

        Ok(crate::recap::Recap {
            id,
            kind: crate::recap::RecapKind::Job,
            subject_id,
            last_message_id: last_event_seq,
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

    pub fn insert(&self, rec: &JobRecord, workspace_root: &str, approval: &str) -> Result<(), String> {
        let task_blob = encrypt(&self.key, &rec.task)?;
        let summary_blob = encrypt_opt(&self.key, rec.summary.as_deref())?;
        let webhook_blob = encrypt_opt(&self.key, rec.webhook_url.as_deref())?;
        let tags_json = serde_json::to_string(&rec.tags).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "INSERT INTO jobs (session_id, encrypted_task, status, provider, approval,
                                   priority, workspace_root, encrypted_webhook, tags_json,
                                   queued_at, started_at, finished_at, encrypted_summary,
                                   cancellation_reason, steps_completed, tokens_used, cost_cents)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![
                    rec.session_id,
                    task_blob,
                    rec.status,
                    rec.provider,
                    approval,
                    rec.priority as i64,
                    workspace_root,
                    webhook_blob,
                    tags_json,
                    rec.queued_at as i64,
                    rec.started_at as i64,
                    rec.finished_at.map(|v| v as i64),
                    summary_blob,
                    rec.cancellation_reason,
                    rec.steps_completed as i64,
                    rec.tokens_used as i64,
                    rec.cost_cents as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_running(&self, sid: &str) -> Result<bool, String> {
        let n = self
            .conn
            .execute(
                "UPDATE jobs SET status='running', started_at=?1
                 WHERE session_id=?2 AND status IN ('queued','running')",
                params![now_ms() as i64, sid],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn mark_terminal(
        &self,
        sid: &str,
        status: JobStatus,
        summary: Option<&str>,
        reason: Option<&str>,
    ) -> Result<bool, String> {
        if !status.is_terminal() {
            return Err(format!("mark_terminal called with non-terminal status {:?}", status));
        }
        let summary_blob = encrypt_opt(&self.key, summary)?;
        let n = self
            .conn
            .execute(
                "UPDATE jobs SET status=?1,
                                 finished_at=?2,
                                 encrypted_summary=COALESCE(?3, encrypted_summary),
                                 cancellation_reason=COALESCE(?4, cancellation_reason)
                 WHERE session_id=?5",
                params![status.as_str(), now_ms() as i64, summary_blob, reason, sid],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn set_counters(
        &self,
        sid: &str,
        steps_completed: u64,
        tokens_used: u64,
        cost_cents: u64,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE jobs SET steps_completed=?1, tokens_used=?2, cost_cents=?3
                 WHERE session_id=?4",
                params![
                    steps_completed as i64,
                    tokens_used as i64,
                    cost_cents as i64,
                    sid
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get(&self, sid: &str) -> Result<Option<JobRecord>, String> {
        let mut stmt = self
            .conn
            .prepare(SELECT_COLUMNS)
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![sid]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => Ok(Some(self.row_to_record(row)?)),
            None => Ok(None),
        }
    }

    pub fn list(&self) -> Result<Vec<JobRecord>, String> {
        let mut stmt = self
            .conn
            .prepare(SELECT_ALL)
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(self.row_to_record(row)?);
        }
        Ok(out)
    }

    /// Count jobs currently in a given status. Used to expose queue-depth
    /// / running-count gauges in metrics snapshots.
    pub fn count_by_status(&self, status: JobStatus) -> Result<u64, String> {
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE status=?1",
                params![status.as_str()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(n as u64)
    }

    pub fn recover_interrupted(&self) -> Result<usize, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE jobs SET status='failed',
                                 finished_at=?1,
                                 cancellation_reason='daemon restart'
                 WHERE status IN ('queued','running')",
                params![now_ms() as i64],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub fn meta_get(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM jobs_meta WHERE key=?1",
                params![key],
                |r| r.get(0),
            )
            .ok()
    }

    /// M4: append an event to the durable log for `sid`. Returns the new
    /// per-session `seq`. Payload is encrypted under the store key. The
    /// (session_id, seq) primary key gives us ordering for replay.
    pub fn append_event(&self, sid: &str, payload: &AgentEventPayload) -> Result<u64, String> {
        let json = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        let blob = encrypt(&self.key, &json)?;
        let next_seq: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(seq),0)+1 FROM job_events WHERE session_id=?1",
                params![sid],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO job_events (session_id, seq, kind, encrypted_payload, created_at)
                 VALUES (?1,?2,?3,?4,?5)",
                params![sid, next_seq, payload.kind, blob, now_ms() as i64],
            )
            .map_err(|e| e.to_string())?;
        Ok(next_seq as u64)
    }

    /// M4: list events for `sid` with `seq > since_seq`, ordered ascending.
    /// Pass `since_seq=0` to replay from the beginning.
    pub fn list_events_since(
        &self,
        sid: &str,
        since_seq: u64,
    ) -> Result<Vec<(u64, AgentEventPayload)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT seq, encrypted_payload
                 FROM job_events
                 WHERE session_id=?1 AND seq>?2
                 ORDER BY seq ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![sid, since_seq as i64], |r| {
                let seq: i64 = r.get(0)?;
                let blob: Vec<u8> = r.get(1)?;
                Ok((seq, blob))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (seq, blob) = row.map_err(|e| e.to_string())?;
            let json = decrypt(&self.key, &blob)?;
            let payload: AgentEventPayload =
                serde_json::from_str(&json).map_err(|e| e.to_string())?;
            out.push((seq as u64, payload));
        }
        Ok(out)
    }

    /// M5: record the terminal outcome of a webhook delivery. Upsert so
    /// retries to an idempotent endpoint don't spawn duplicate rows.
    pub fn record_webhook_delivery(&self, rec: &WebhookDelivery) -> Result<(), String> {
        let url_blob = encrypt(&self.key, &rec.url)?;
        self.conn
            .execute(
                "INSERT INTO webhook_deliveries (session_id, encrypted_url, status, attempts,
                                                 last_http_status, last_error,
                                                 first_attempt_at, last_attempt_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(session_id) DO UPDATE SET
                     encrypted_url    = excluded.encrypted_url,
                     status           = excluded.status,
                     attempts         = excluded.attempts,
                     last_http_status = excluded.last_http_status,
                     last_error       = excluded.last_error,
                     last_attempt_at  = excluded.last_attempt_at",
                params![
                    rec.session_id,
                    url_blob,
                    rec.status,
                    rec.attempts as i64,
                    rec.last_http_status.map(|s| s as i64),
                    rec.last_error,
                    rec.first_attempt_at as i64,
                    rec.last_attempt_at as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_webhook_delivery(&self, sid: &str) -> Result<Option<WebhookDelivery>, String> {
        let row = self.conn.query_row(
            "SELECT session_id, encrypted_url, status, attempts, last_http_status,
                    last_error, first_attempt_at, last_attempt_at
             FROM webhook_deliveries WHERE session_id=?1",
            params![sid],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, i64>(7)?,
                ))
            },
        );
        let (session_id, url_blob, status, attempts, http, err, first, last) = match row {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let url = decrypt(&self.key, &url_blob)?;
        Ok(Some(WebhookDelivery {
            session_id,
            url,
            status,
            attempts: attempts as u32,
            last_http_status: http.map(|n| n as u16),
            last_error: err,
            first_attempt_at: first as u64,
            last_attempt_at: last as u64,
        }))
    }

    /// List webhook deliveries whose status matches, ordered by most recent.
    pub fn list_webhook_deliveries_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<WebhookDelivery>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, encrypted_url, status, attempts, last_http_status,
                        last_error, first_attempt_at, last_attempt_at
                 FROM webhook_deliveries WHERE status=?1
                 ORDER BY last_attempt_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![status], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, i64>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (sid, blob, st, attempts, http, err, first, last) =
                row.map_err(|e| e.to_string())?;
            let url = decrypt(&self.key, &blob)?;
            out.push(WebhookDelivery {
                session_id: sid,
                url,
                status: st,
                attempts: attempts as u32,
                last_http_status: http.map(|n| n as u16),
                last_error: err,
                first_attempt_at: first as u64,
                last_attempt_at: last as u64,
            });
        }
        Ok(out)
    }

    // ── Scratchpad (Phase 3) ───────────────────────────────────────────

    /// Upsert a scratchpad entry for (session_id, key). Values are
    /// encrypted at rest.
    pub fn scratchpad_set(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        let blob = encrypt(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO job_scratchpad (session_id, key, encrypted_value, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(session_id, key)
                 DO UPDATE SET encrypted_value = excluded.encrypted_value,
                               updated_at      = excluded.updated_at",
                params![session_id, key, blob, now_ms() as i64],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read a single scratchpad entry. Returns `None` when missing.
    pub fn scratchpad_get(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT encrypted_value FROM job_scratchpad
                 WHERE session_id = ?1 AND key = ?2",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![session_id, key])
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let blob: Vec<u8> = row.get(0).map_err(|e| e.to_string())?;
            Ok(Some(decrypt(&self.key, &blob)?))
        } else {
            Ok(None)
        }
    }

    /// List all scratchpad entries for a session, newest first.
    pub fn scratchpad_list(
        &self,
        session_id: &str,
    ) -> Result<Vec<ScratchpadEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT key, encrypted_value, updated_at FROM job_scratchpad
                 WHERE session_id = ?1
                 ORDER BY updated_at DESC, key ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            let (key, blob, updated_at) = r.map_err(|e| e.to_string())?;
            let value = decrypt(&self.key, &blob)?;
            out.push(ScratchpadEntry {
                key,
                value,
                updated_at: updated_at as u64,
            });
        }
        Ok(out)
    }

    /// Delete a single scratchpad entry. Returns `true` iff a row was
    /// removed (i.e., the entry existed).
    pub fn scratchpad_delete(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<bool, String> {
        let n = self
            .conn
            .execute(
                "DELETE FROM job_scratchpad WHERE session_id = ?1 AND key = ?2",
                params![session_id, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    /// Clear every scratchpad entry for a session. Returns the row count.
    pub fn scratchpad_clear(&self, session_id: &str) -> Result<usize, String> {
        let n = self
            .conn
            .execute(
                "DELETE FROM job_scratchpad WHERE session_id = ?1",
                params![session_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(n)
    }

    pub fn meta_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO jobs_meta (key, value) VALUES (?1,?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn row_to_record(&self, r: &rusqlite::Row) -> Result<JobRecord, String> {
        let get = |i: usize| -> Result<_, String> {
            r.get::<_, rusqlite::types::Value>(i).map_err(|e| e.to_string())
        };
        use rusqlite::types::Value;

        let session_id = match get(0)? {
            Value::Text(s) => s,
            _ => return Err("session_id not text".into()),
        };
        let task_blob: Vec<u8> = match get(1)? {
            Value::Blob(b) => b,
            _ => return Err("encrypted_task not blob".into()),
        };
        let status = match get(2)? {
            Value::Text(s) => s,
            _ => return Err("status not text".into()),
        };
        let provider = match get(3)? {
            Value::Text(s) => s,
            _ => return Err("provider not text".into()),
        };
        let priority: i64 = match get(4)? {
            Value::Integer(n) => n,
            _ => 5,
        };
        let webhook_blob: Option<Vec<u8>> = match get(5)? {
            Value::Blob(b) => Some(b),
            Value::Null => None,
            _ => return Err("encrypted_webhook not blob/null".into()),
        };
        let tags_json = match get(6)? {
            Value::Text(s) => s,
            _ => "[]".into(),
        };
        let queued_at: i64 = match get(7)? {
            Value::Integer(n) => n,
            _ => 0,
        };
        let started_at: Option<i64> = match get(8)? {
            Value::Integer(n) => Some(n),
            Value::Null => None,
            _ => None,
        };
        let finished_at: Option<i64> = match get(9)? {
            Value::Integer(n) => Some(n),
            Value::Null => None,
            _ => None,
        };
        let summary_blob: Option<Vec<u8>> = match get(10)? {
            Value::Blob(b) => Some(b),
            Value::Null => None,
            _ => None,
        };
        let cancellation_reason: Option<String> = match get(11)? {
            Value::Text(s) => Some(s),
            Value::Null => None,
            _ => None,
        };
        let steps_completed: i64 = match get(12)? {
            Value::Integer(n) => n,
            _ => 0,
        };
        let tokens_used: i64 = match get(13)? {
            Value::Integer(n) => n,
            _ => 0,
        };
        let cost_cents: i64 = match get(14)? {
            Value::Integer(n) => n,
            _ => 0,
        };

        let task = decrypt(&self.key, &task_blob)?;
        let summary = decrypt_opt(&self.key, summary_blob)?;
        let webhook_url = decrypt_opt(&self.key, webhook_blob)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(JobRecord {
            session_id,
            task,
            status,
            provider,
            started_at: started_at.unwrap_or(0) as u64,
            finished_at: finished_at.map(|v| v as u64),
            summary,
            priority: priority as u8,
            queued_at: queued_at as u64,
            webhook_url,
            tags,
            cancellation_reason,
            steps_completed: steps_completed as u64,
            tokens_used: tokens_used as u64,
            cost_cents: cost_cents as u64,
        })
    }
}

const SELECT_COLUMNS: &str = "SELECT session_id, encrypted_task, status, provider, priority,
                                     encrypted_webhook, tags_json, queued_at, started_at,
                                     finished_at, encrypted_summary, cancellation_reason,
                                     steps_completed, tokens_used, cost_cents
                              FROM jobs WHERE session_id=?1";

const SELECT_ALL: &str = "SELECT session_id, encrypted_task, status, provider, priority,
                                 encrypted_webhook, tags_json, queued_at, started_at,
                                 finished_at, encrypted_summary, cancellation_reason,
                                 steps_completed, tokens_used, cost_cents
                          FROM jobs
                          ORDER BY COALESCE(started_at, queued_at) DESC";

// ── JobManager ────────────────────────────────────────────────────────────────

type EventStreams = Arc<Mutex<HashMap<String, broadcast::Sender<AgentEventPayload>>>>;

/// Owner of the async-job queue and live event streams. Storage +
/// broadcast only in M1; agent orchestration stays in the caller.
#[cfg(unix)]
type DispatchSenders =
    Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<crate::subprocess_dispatch::DispatchFrame>>>>;

/// Atomic counters tracking JobManager activity. Cheap to increment from
/// any task (load-ordering `Relaxed`) and sampled via `metrics_snapshot`.
/// The gauge-like `queue_depth` isn't stored here — it's computed from
/// SQL on demand — so counters are all monotonic.
#[derive(Debug, Default)]
struct Metrics {
    jobs_created: std::sync::atomic::AtomicU64,
    jobs_completed: std::sync::atomic::AtomicU64,
    jobs_failed: std::sync::atomic::AtomicU64,
    jobs_cancelled: std::sync::atomic::AtomicU64,
    quota_denied: std::sync::atomic::AtomicU64,
    subprocesses_spawned: std::sync::atomic::AtomicU64,
    events_published: std::sync::atomic::AtomicU64,
    events_replayed: std::sync::atomic::AtomicU64,
    webhooks_delivered: std::sync::atomic::AtomicU64,
    webhooks_dead_lettered: std::sync::atomic::AtomicU64,
}

/// A point-in-time snapshot of JobManager metrics. Derived fields
/// (`queued`, `running`) are read from the DB at snapshot time.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobManagerMetrics {
    pub jobs_created: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub jobs_cancelled: u64,
    pub quota_denied: u64,
    pub subprocesses_spawned: u64,
    pub events_published: u64,
    pub events_replayed: u64,
    pub webhooks_delivered: u64,
    pub webhooks_dead_lettered: u64,
    /// Current depth of the queue (status='queued').
    pub queued: u64,
    /// Current count of jobs mid-run (status='running').
    pub running: u64,
}

#[derive(Clone)]
pub struct JobManager {
    db: Arc<Mutex<JobsDb>>,
    streams: EventStreams,
    /// Per-sid outgoing senders for live subprocess workers. Used by
    /// `cancel()` to push a `DispatchFrame::Cancel` to the child.
    #[cfg(unix)]
    dispatch_senders: DispatchSenders,
    /// Optional quota manager. When set, `create()` consults it for the
    /// `Tasks` resource against `req.quota_bucket` and rejects submissions
    /// that would breach the hard limit.
    quotas: Arc<Mutex<crate::agent_quota::QuotaManager>>,
    metrics: Arc<Metrics>,
}

impl std::fmt::Debug for JobManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobManager").finish_non_exhaustive()
    }
}

impl JobManager {
    pub fn new(db_path: &Path) -> Result<Self, String> {
        Ok(Self {
            db: Arc::new(Mutex::new(JobsDb::open(db_path)?)),
            streams: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(unix)]
            dispatch_senders: Arc::new(Mutex::new(HashMap::new())),
            quotas: Arc::new(Mutex::new(crate::agent_quota::QuotaManager::new())),
            metrics: Arc::new(Metrics::default()),
        })
    }

    pub fn new_with(db: JobsDb) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            streams: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(unix)]
            dispatch_senders: Arc::new(Mutex::new(HashMap::new())),
            quotas: Arc::new(Mutex::new(crate::agent_quota::QuotaManager::new())),
            metrics: Arc::new(Metrics::default()),
        }
    }

    /// Snapshot the atomic counters and derive queue-depth / running counts
    /// from the DB. All counter reads use `Relaxed` ordering; operators
    /// who need a perfectly consistent view should pause job submission
    /// first.
    pub async fn metrics_snapshot(&self) -> JobManagerMetrics {
        use std::sync::atomic::Ordering::Relaxed;
        let m = &self.metrics;
        let (queued, running) = {
            let db = self.db.lock().await;
            let q = db.count_by_status(JobStatus::Queued).unwrap_or(0);
            let r = db.count_by_status(JobStatus::Running).unwrap_or(0);
            (q, r)
        };
        JobManagerMetrics {
            jobs_created: m.jobs_created.load(Relaxed),
            jobs_completed: m.jobs_completed.load(Relaxed),
            jobs_failed: m.jobs_failed.load(Relaxed),
            jobs_cancelled: m.jobs_cancelled.load(Relaxed),
            quota_denied: m.quota_denied.load(Relaxed),
            subprocesses_spawned: m.subprocesses_spawned.load(Relaxed),
            events_published: m.events_published.load(Relaxed),
            events_replayed: m.events_replayed.load(Relaxed),
            webhooks_delivered: m.webhooks_delivered.load(Relaxed),
            webhooks_dead_lettered: m.webhooks_dead_lettered.load(Relaxed),
            queued,
            running,
        }
    }

    /// Configure per-agent quotas. The bucket name becomes the
    /// `quota_bucket` value passed in `CreateJobReq`.
    pub async fn set_agent_quotas(&self, bucket: &str, quotas: Vec<crate::agent_quota::Quota>) {
        use crate::agent_registry::AgentId;
        let mut mgr = self.quotas.lock().await;
        mgr.set_quotas(&AgentId(bucket.to_string()), quotas);
    }

    /// Configure a global quota shared across all buckets.
    pub async fn add_global_quota(&self, quota: crate::agent_quota::Quota) {
        let mut mgr = self.quotas.lock().await;
        mgr.add_global_quota(quota);
    }

    /// Insert a fresh job in `queued` state and return its session_id.
    /// M1 defers dispatch to the caller — they immediately call
    /// `mark_running` + `spawn_agent_run`.
    pub async fn create(&self, req: CreateJobReq) -> Result<String, SubmitError> {
        // M3: per-bucket quota enforcement. Checks (and if allowed, consumes)
        // one unit of the `Tasks` resource before persisting. The check is
        // skipped entirely when `quota_bucket` is None so existing callers
        // stay unaffected until they opt in.
        if let Some(bucket) = req.quota_bucket.as_deref() {
            use crate::agent_quota::{QuotaDecision, ResourceKind};
            use crate::agent_registry::AgentId;
            let mut mgr = self.quotas.lock().await;
            let aid = AgentId(bucket.to_string());
            let decision = mgr.check_and_consume(&aid, &ResourceKind::Tasks, 1);
            if let QuotaDecision::Deny { resource, used, hard_limit } = decision {
                self.metrics
                    .quota_denied
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(SubmitError::QuotaDenied { resource, used, hard_limit });
            }
        }

        let session_id = format!("{:032x}", rand::rng().random::<u128>());
        let now = now_ms();
        let record = JobRecord {
            session_id: session_id.clone(),
            task: req.task,
            status: JobStatus::Queued.as_str().to_string(),
            provider: req.provider,
            started_at: 0,
            finished_at: None,
            summary: None,
            priority: req.priority,
            queued_at: now,
            webhook_url: req.webhook_url,
            tags: req.tags,
            cancellation_reason: None,
            steps_completed: 0,
            tokens_used: 0,
            cost_cents: 0,
        };
        let db = self.db.lock().await;
        db.insert(&record, &req.workspace_root, &req.approval)
            .map_err(SubmitError::Storage)?;
        self.metrics
            .jobs_created
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(session_id)
    }

    pub async fn mark_running(&self, sid: &str) -> Result<bool, String> {
        self.db.lock().await.mark_running(sid)
    }

    pub async fn mark_terminal(
        &self,
        sid: &str,
        status: JobStatus,
        summary: Option<String>,
        reason: Option<String>,
    ) -> Result<bool, String> {
        let changed = self
            .db
            .lock()
            .await
            .mark_terminal(sid, status, summary.as_deref(), reason.as_deref())?;
        if changed {
            use std::sync::atomic::Ordering::Relaxed;
            match status {
                JobStatus::Complete => {
                    self.metrics.jobs_completed.fetch_add(1, Relaxed);
                }
                JobStatus::Failed => {
                    self.metrics.jobs_failed.fetch_add(1, Relaxed);
                }
                JobStatus::Cancelled => {
                    self.metrics.jobs_cancelled.fetch_add(1, Relaxed);
                }
                _ => {}
            }
        }
        Ok(changed)
    }

    pub async fn set_counters(
        &self,
        sid: &str,
        steps_completed: u64,
        tokens_used: u64,
        cost_cents: u64,
    ) -> Result<(), String> {
        self.db
            .lock()
            .await
            .set_counters(sid, steps_completed, tokens_used, cost_cents)
    }

    pub async fn get(&self, sid: &str) -> Option<JobRecord> {
        self.db.lock().await.get(sid).ok().flatten()
    }

    pub async fn list(&self) -> Vec<JobRecord> {
        self.db.lock().await.list().unwrap_or_default()
    }

    /// Open a broadcast channel for this session and register it.
    /// Returns the sender so the caller can push events.
    pub async fn open_stream(&self, sid: &str) -> broadcast::Sender<AgentEventPayload> {
        let (tx, _) = broadcast::channel::<AgentEventPayload>(256);
        let mut s = self.streams.lock().await;
        s.insert(sid.to_string(), tx.clone());
        tx
    }

    pub async fn close_stream(&self, sid: &str) {
        let mut s = self.streams.lock().await;
        s.remove(sid);
    }

    /// Subscribe to an active session's event stream, or `None` if the
    /// session is not currently streaming (terminal or unknown).
    pub async fn subscribe(&self, sid: &str) -> Option<broadcast::Receiver<AgentEventPayload>> {
        let s = self.streams.lock().await;
        s.get(sid).map(|tx| tx.subscribe())
    }

    /// Persist `payload` to the durable event log and fan out on the live
    /// broadcast channel. Returns the assigned `seq` on success — callers
    /// can echo this back to clients so they can reconnect with
    /// `?since_seq=N`. If persistence fails we still broadcast so live
    /// subscribers don't lose events; the returned `None` flags the write
    /// miss.
    pub async fn publish_event(&self, sid: &str, payload: AgentEventPayload) -> Option<u64> {
        let seq = {
            let db = self.db.lock().await;
            db.append_event(sid, &payload).ok()
        };
        if seq.is_some() {
            self.metrics
                .events_published
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let s = self.streams.lock().await;
        if let Some(tx) = s.get(sid) {
            let _ = tx.send(payload);
        }
        seq
    }

    /// Fetch persisted events for `sid` with `seq > since_seq`, ordered
    /// ascending. Used by SSE reconnect to replay events the client missed.
    pub async fn replay_events(
        &self,
        sid: &str,
        since_seq: u64,
    ) -> Vec<(u64, AgentEventPayload)> {
        let events = {
            let db = self.db.lock().await;
            db.list_events_since(sid, since_seq).unwrap_or_default()
        };
        if !events.is_empty() {
            self.metrics
                .events_replayed
                .fetch_add(events.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }
        events
    }

    /// M5: run the webhook retry loop against a caller-supplied send
    /// closure, persist the final outcome, and return it. Generic over
    /// the sender so tests can exercise failure sequences without HTTP.
    pub async fn deliver_webhook_with<F, Fut>(
        &self,
        sid: &str,
        url: &str,
        cfg: &crate::webhook::RetryConfig,
        send: F,
    ) -> WebhookDelivery
    where
        F: FnMut(u32) -> Fut,
        Fut: std::future::Future<Output = crate::webhook::AttemptOutcome>,
    {
        let first_attempt_at = now_ms();
        let outcome = crate::webhook::deliver_with_retry(cfg, send).await;
        let last_attempt_at = now_ms();

        let (status, attempts, last_http_status, last_error) = match &outcome {
            crate::webhook::WebhookOutcome::Delivered { status, attempts } => {
                self.metrics
                    .webhooks_delivered
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                ("delivered".to_string(), *attempts, Some(*status), None)
            }
            crate::webhook::WebhookOutcome::DeadLetter { attempts, last_error } => {
                self.metrics
                    .webhooks_dead_lettered
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                (
                    "dead_letter".to_string(),
                    *attempts,
                    None,
                    Some(last_error.clone()),
                )
            }
        };

        let record = WebhookDelivery {
            session_id: sid.to_string(),
            url: url.to_string(),
            status,
            attempts,
            last_http_status,
            last_error,
            first_attempt_at,
            last_attempt_at,
        };
        let db = self.db.lock().await;
        let _ = db.record_webhook_delivery(&record);
        record
    }

    /// Production path: deliver a JSON payload via reqwest with retries,
    /// record the outcome. Thin wrapper around `deliver_webhook_with`.
    pub async fn deliver_webhook(
        &self,
        sid: &str,
        url: &str,
        payload: &serde_json::Value,
    ) -> WebhookDelivery {
        let client = reqwest::Client::new();
        let cfg = crate::webhook::RetryConfig::default();
        self.deliver_webhook_with(sid, url, &cfg, |_attempt| async {
            match client
                .post(url)
                .json(payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(r) => {
                    let status = r.status().as_u16();
                    if r.status().is_success() {
                        crate::webhook::AttemptOutcome::Success(status)
                    } else {
                        crate::webhook::AttemptOutcome::Transient(format!("HTTP {status}"))
                    }
                }
                Err(e) => {
                    crate::webhook::AttemptOutcome::Transient(format!("send error: {e}"))
                }
            }
        })
        .await
    }

    /// Look up a prior delivery outcome. `None` if no POST has been
    /// attempted for this sid (e.g., job had no webhook_url).
    pub async fn get_webhook_delivery(&self, sid: &str) -> Option<WebhookDelivery> {
        let db = self.db.lock().await;
        db.get_webhook_delivery(sid).ok().flatten()
    }

    /// List deliveries currently in the dead-letter state. Operators can
    /// use this to triage unacknowledged webhooks.
    pub async fn list_dead_letter_webhooks(&self) -> Vec<WebhookDelivery> {
        let db = self.db.lock().await;
        db.list_webhook_deliveries_by_status("dead_letter")
            .unwrap_or_default()
    }

    /// Cancel a job. Closes the broadcast stream, marks terminal with
    /// `cancelled`, and — if the job is running as a subprocess — pushes
    /// a `Cancel` frame to the child so its agent loop actually stops.
    /// The child may emit an `Error` frame before exiting, which the
    /// bridge ignores because the job is already in a terminal state.
    pub async fn cancel(&self, sid: &str, reason: Option<String>) -> Option<JobRecord> {
        // If there's a live subprocess, tell it to stop BEFORE we tear
        // down the stream. Best-effort — a full mpsc buffer or closed
        // channel is non-fatal; the bridge will eventually reap.
        #[cfg(unix)]
        {
            use crate::subprocess_dispatch::DispatchFrame;
            let sender = {
                let senders = self.dispatch_senders.lock().await;
                senders.get(sid).cloned()
            };
            if let Some(tx) = sender {
                let _ = tx
                    .send(DispatchFrame::Cancel {
                        reason: reason.clone().unwrap_or_else(|| "user requested".into()),
                    })
                    .await;
            }
        }

        // Close stream first so SSE clients disconnect promptly.
        self.close_stream(sid).await;

        let db = self.db.lock().await;
        let current = db.get(sid).ok().flatten()?;
        if JobStatus::parse(&current.status).map(|s| s.is_terminal()).unwrap_or(false) {
            return Some(current);
        }
        let changed = db
            .mark_terminal(
                sid,
                JobStatus::Cancelled,
                None,
                reason.as_deref().or(Some("user requested")),
            )
            .unwrap_or(false);
        if changed {
            self.metrics
                .jobs_cancelled
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        db.get(sid).ok().flatten()
    }

    /// Test hook: register a dispatch sender for a session id so that a
    /// subsequent `cancel()` will push a `Cancel` frame through it. In
    /// production this is set up internally by `spawn_subprocess`.
    #[cfg(unix)]
    pub async fn register_dispatch_sender(
        &self,
        sid: &str,
        tx: tokio::sync::mpsc::Sender<crate::subprocess_dispatch::DispatchFrame>,
    ) {
        let mut senders = self.dispatch_senders.lock().await;
        senders.insert(sid.to_string(), tx);
    }

    /// Spawn a child `vibecli worker` subprocess, hand it the task, and
    /// bridge child events into this job's broadcast stream. The child
    /// inherits one end of a Unix socketpair on fd 3; the PSK for the
    /// Noise_NNpsk0 handshake is delivered via a private env var.
    ///
    /// When the child emits `Complete` / `Error`, this method marks the
    /// job terminal and closes the stream. When the channel closes without
    /// a terminal frame (child crash, daemon kill), the job is marked
    /// `failed` with reason `"worker channel closed"`.
    ///
    /// Returns immediately after the Run frame is sent — the bridge task
    /// runs in the background.
    #[cfg(unix)]
    pub async fn spawn_subprocess(
        &self,
        sid: &str,
        task: &str,
        provider: &str,
        approval: &str,
        workspace_root: &str,
    ) -> Result<(), String> {
        use crate::subprocess_dispatch::{spawn_worker, generate_psk, DispatchFrame};

        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        let psk = generate_psk();
        let handle = spawn_worker(&exe, sid, &psk)
            .await
            .map_err(|e| format!("spawn_worker: {e}"))?;
        self.metrics
            .subprocesses_spawned
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        handle
            .outgoing
            .send(DispatchFrame::Run {
                job_id: sid.into(),
                task: task.into(),
                provider: provider.into(),
                approval: approval.into(),
                workspace_root: workspace_root.into(),
                max_turns: 25,
            })
            .await
            .map_err(|e| format!("send Run: {e}"))?;

        // Register the broadcast channel for live subscribers; the bridge
        // publishes through `publish_event`, which looks it up.
        let _ = self.open_stream(sid).await;
        let sid_owned = sid.to_string();
        let jm = self.clone();

        // Register the child's outgoing sender so `cancel(sid)` can push a
        // `Cancel` frame. Unregistered on bridge exit.
        {
            let mut senders = self.dispatch_senders.lock().await;
            senders.insert(sid_owned.clone(), handle.outgoing.clone());
        }

        tokio::spawn(async move {
            let mut child = handle.child;
            let mut incoming = handle.incoming;
            let outgoing = handle.outgoing; // kept alive so reverse Cancel path works
            let mut got_terminal = false;
            while let Some(frame) = incoming.recv().await {
                match frame {
                    DispatchFrame::Event(ev) => {
                        // Persist before fanning out so reconnecting SSE
                        // clients can replay even if the broadcast buffer
                        // lapped them.
                        let _ = jm.publish_event(&sid_owned, ev).await;
                    }
                    DispatchFrame::Complete { summary } => {
                        let _ = jm
                            .mark_terminal(
                                &sid_owned,
                                JobStatus::Complete,
                                Some(summary),
                                None,
                            )
                            .await;
                        got_terminal = true;
                        break;
                    }
                    DispatchFrame::Error { message } => {
                        let _ = jm
                            .mark_terminal(
                                &sid_owned,
                                JobStatus::Failed,
                                None,
                                Some(message),
                            )
                            .await;
                        got_terminal = true;
                        break;
                    }
                    DispatchFrame::Ready => {}
                    DispatchFrame::Run { .. } | DispatchFrame::Cancel { .. } => {
                        // parent→child direction; ignore if it somehow comes back
                    }
                }
            }
            if !got_terminal {
                let _ = jm
                    .mark_terminal(
                        &sid_owned,
                        JobStatus::Failed,
                        None,
                        Some("worker channel closed".into()),
                    )
                    .await;
            }
            jm.close_stream(&sid_owned).await;
            {
                let mut senders = jm.dispatch_senders.lock().await;
                senders.remove(&sid_owned);
            }
            drop(outgoing);
            let _ = child.wait().await;
        });

        Ok(())
    }

    /// Daemon-boot sweep: any jobs left in `queued` or `running` from a
    /// prior process become `failed` with reason `"daemon restart"`.
    /// Returns the count of rows changed.
    pub async fn recover_interrupted(&self) -> Result<usize, String> {
        self.db.lock().await.recover_interrupted()
    }

    /// One-shot migration from the pre-M1 `~/.vibecli/jobs/*.json` layout.
    /// Idempotent — a sentinel row in `jobs_meta` prevents re-import.
    /// Successfully migrated files are moved to `<jobs_dir>.bak/`.
    pub async fn migrate_json_jobs(&self, jobs_dir: &Path) -> Result<MigrationReport, String> {
        let db = self.db.lock().await;
        if db.meta_get("json_migrated").as_deref() == Some("1") {
            return Ok(MigrationReport::default());
        }
        if !jobs_dir.exists() {
            db.meta_set("json_migrated", "1")?;
            return Ok(MigrationReport::default());
        }

        let entries = std::fs::read_dir(jobs_dir).map_err(|e| e.to_string())?;
        let mut report = MigrationReport::default();
        let backup_dir = jobs_dir
            .parent()
            .map(|p| p.join("jobs.bak"))
            .unwrap_or_else(|| PathBuf::from("jobs.bak"));

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                report.skipped += 1;
                continue;
            };
            let Ok(rec) = serde_json::from_str::<JobRecord>(&text) else {
                report.skipped += 1;
                continue;
            };
            // Skip if this id already exists (e.g. partial prior migration).
            if db.get(&rec.session_id).ok().flatten().is_some() {
                report.skipped += 1;
                continue;
            }
            // Fill in defaults for fields that didn't exist in the old shape.
            let mut rec = rec;
            if rec.queued_at == 0 {
                rec.queued_at = rec.started_at;
            }
            if rec.priority == 0 {
                rec.priority = default_priority();
            }
            // Running jobs from the old layout are orphaned — promote to failed.
            if rec.status == "running" {
                rec.status = "failed".into();
                rec.cancellation_reason = Some("daemon restart (migrated)".into());
                if rec.finished_at.is_none() {
                    rec.finished_at = Some(now_ms());
                }
            }
            if let Err(e) = db.insert(&rec, "", "auto") {
                tracing::warn!("jobs migration: failed to import {}: {e}", rec.session_id);
                report.skipped += 1;
                continue;
            }
            // Move source file to backup directory.
            if std::fs::create_dir_all(&backup_dir).is_ok() {
                let dst = backup_dir.join(path.file_name().unwrap_or_default());
                let _ = std::fs::rename(&path, &dst);
                report.backed_up_dir = Some(backup_dir.clone());
            }
            report.imported += 1;
        }

        db.meta_set("json_migrated", "1")?;
        Ok(report)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn mgr() -> (JobManager, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let db = JobsDb::open_with(&tmp.path().join("jobs.db"), [42u8; 32]).unwrap();
        (JobManager::new_with(db), tmp)
    }

    fn req(task: &str) -> CreateJobReq {
        CreateJobReq {
            task: task.into(),
            provider: "mock".into(),
            approval: "auto".into(),
            workspace_root: "/tmp/ws".into(),
            priority: 5,
            webhook_url: None,
            tags: vec![],
            quota_bucket: None,
        }
    }

    #[tokio::test]
    async fn create_inserts_queued_row() {
        let (m, _t) = mgr();
        let id = m.create(req("hello")).await.unwrap();
        let rec = m.get(&id).await.expect("record exists");
        assert_eq!(rec.status, "queued");
        assert_eq!(rec.task, "hello");
        assert_eq!(rec.priority, 5);
        assert!(rec.queued_at > 0);
        assert_eq!(rec.started_at, 0);
    }

    #[tokio::test]
    async fn mark_running_then_complete() {
        let (m, _t) = mgr();
        let id = m.create(req("run me")).await.unwrap();
        assert!(m.mark_running(&id).await.unwrap());
        let rec = m.get(&id).await.unwrap();
        assert_eq!(rec.status, "running");
        assert!(rec.started_at > 0);

        m.mark_terminal(&id, JobStatus::Complete, Some("ok".into()), None)
            .await
            .unwrap();
        let rec = m.get(&id).await.unwrap();
        assert_eq!(rec.status, "complete");
        assert_eq!(rec.summary.as_deref(), Some("ok"));
        assert!(rec.finished_at.is_some());
    }

    #[tokio::test]
    async fn quota_denies_submission_over_hard_limit() {
        use crate::agent_quota::{Quota, ResourceKind};
        let (m, _t) = mgr();
        m.set_agent_quotas("team-alpha", vec![Quota::new(ResourceKind::Tasks, 2)])
            .await;

        let r = |n: u32| CreateJobReq {
            task: format!("task-{n}"),
            provider: "mock".into(),
            approval: "auto".into(),
            workspace_root: "/tmp/ws".into(),
            priority: 5,
            webhook_url: None,
            tags: vec![],
            quota_bucket: Some("team-alpha".into()),
        };

        m.create(r(1)).await.expect("first allowed");
        m.create(r(2)).await.expect("second allowed");
        match m.create(r(3)).await {
            Err(SubmitError::QuotaDenied { resource, used, hard_limit }) => {
                assert_eq!(resource, "tasks");
                assert_eq!(hard_limit, 2);
                assert_eq!(used, 2);
            }
            other => panic!("expected QuotaDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn quota_other_buckets_are_independent() {
        use crate::agent_quota::{Quota, ResourceKind};
        let (m, _t) = mgr();
        m.set_agent_quotas("bucket-a", vec![Quota::new(ResourceKind::Tasks, 1)])
            .await;
        m.set_agent_quotas("bucket-b", vec![Quota::new(ResourceKind::Tasks, 1)])
            .await;

        let mut r = req("x");
        r.quota_bucket = Some("bucket-a".into());
        m.create(r.clone()).await.expect("first to bucket-a");

        let mut r2 = req("y");
        r2.quota_bucket = Some("bucket-b".into());
        m.create(r2).await.expect("first to bucket-b not affected by bucket-a");

        assert!(matches!(
            m.create(r).await,
            Err(SubmitError::QuotaDenied { .. })
        ));
    }

    #[tokio::test]
    async fn quota_bucket_none_skips_check() {
        use crate::agent_quota::{Quota, ResourceKind};
        let (m, _t) = mgr();
        m.add_global_quota(Quota::new(ResourceKind::Tasks, 1)).await;
        for i in 0..5 {
            m.create(req(&format!("untracked-{i}")))
                .await
                .expect("None bucket never consults quotas");
        }
    }

    #[tokio::test]
    async fn cancel_marks_cancelled_with_reason() {
        let (m, _t) = mgr();
        let id = m.create(req("cancel me")).await.unwrap();
        m.mark_running(&id).await.unwrap();
        let rec = m.cancel(&id, Some("user test".into())).await.unwrap();
        assert_eq!(rec.status, "cancelled");
        assert_eq!(rec.cancellation_reason.as_deref(), Some("user test"));
    }

    #[tokio::test]
    async fn recover_interrupted_marks_running_as_failed() {
        let (m, _t) = mgr();
        let id = m.create(req("orphan")).await.unwrap();
        m.mark_running(&id).await.unwrap();
        let n = m.recover_interrupted().await.unwrap();
        assert_eq!(n, 1);
        let rec = m.get(&id).await.unwrap();
        assert_eq!(rec.status, "failed");
        assert_eq!(
            rec.cancellation_reason.as_deref(),
            Some("daemon restart")
        );
    }

    #[tokio::test]
    async fn list_orders_newest_first() {
        let (m, _t) = mgr();
        let a = m.create(req("a")).await.unwrap();
        let b = m.create(req("b")).await.unwrap();
        m.mark_running(&a).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        m.mark_running(&b).await.unwrap();
        let list = m.list().await;
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].session_id, b);
        assert_eq!(list[1].session_id, a);
    }

    #[tokio::test]
    async fn migrate_json_jobs_imports_and_backs_up() {
        let (m, t) = mgr();
        let jobs_dir = t.path().join("legacy-jobs");
        std::fs::create_dir_all(&jobs_dir).unwrap();
        let rec = JobRecord {
            session_id: "abc123".into(),
            task: "legacy".into(),
            status: "complete".into(),
            provider: "ollama".into(),
            started_at: 1000,
            finished_at: Some(2000),
            summary: Some("done".into()),
            priority: 0,
            queued_at: 0,
            webhook_url: None,
            tags: vec![],
            cancellation_reason: None,
            steps_completed: 0,
            tokens_used: 0,
            cost_cents: 0,
        };
        std::fs::write(
            jobs_dir.join("abc123.json"),
            serde_json::to_string(&rec).unwrap(),
        )
        .unwrap();

        let rep = m.migrate_json_jobs(&jobs_dir).await.unwrap();
        assert_eq!(rep.imported, 1);
        let imported = m.get("abc123").await.unwrap();
        assert_eq!(imported.task, "legacy");
        assert_eq!(imported.status, "complete");
        // Second call is a no-op thanks to the sentinel.
        let rep2 = m.migrate_json_jobs(&jobs_dir).await.unwrap();
        assert_eq!(rep2.imported, 0);
    }

    #[tokio::test]
    async fn migrate_promotes_running_to_failed() {
        let (m, t) = mgr();
        let jobs_dir = t.path().join("legacy-jobs");
        std::fs::create_dir_all(&jobs_dir).unwrap();
        let rec = JobRecord {
            session_id: "xyz".into(),
            task: "interrupted".into(),
            status: "running".into(),
            provider: "mock".into(),
            started_at: 500,
            finished_at: None,
            summary: None,
            priority: 5,
            queued_at: 0,
            webhook_url: None,
            tags: vec![],
            cancellation_reason: None,
            steps_completed: 0,
            tokens_used: 0,
            cost_cents: 0,
        };
        std::fs::write(
            jobs_dir.join("xyz.json"),
            serde_json::to_string(&rec).unwrap(),
        )
        .unwrap();
        m.migrate_json_jobs(&jobs_dir).await.unwrap();
        let got = m.get("xyz").await.unwrap();
        assert_eq!(got.status, "failed");
        assert!(got.cancellation_reason.is_some());
    }

    #[tokio::test]
    async fn stream_open_subscribe_close() {
        let (m, _t) = mgr();
        let id = m.create(req("stream me")).await.unwrap();
        let tx = m.open_stream(&id).await;
        let mut rx = m.subscribe(&id).await.expect("subscriber registered");
        let _ = tx.send(AgentEventPayload::chunk("hi".into()));
        let ev = rx.recv().await.expect("received");
        assert_eq!(ev.kind, "chunk");
        assert_eq!(ev.content.as_deref(), Some("hi"));
        m.close_stream(&id).await;
        assert!(m.subscribe(&id).await.is_none());
    }

    #[tokio::test]
    async fn publish_event_persists_and_broadcasts() {
        let (m, _t) = mgr();
        let id = m.create(req("replay me")).await.unwrap();
        let _ = m.open_stream(&id).await;
        let mut rx = m.subscribe(&id).await.expect("live subscriber");

        let seq1 = m
            .publish_event(&id, AgentEventPayload::chunk("one".into()))
            .await
            .expect("persisted");
        let seq2 = m
            .publish_event(&id, AgentEventPayload::chunk("two".into()))
            .await
            .expect("persisted");

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);

        // Both events arrived on the broadcast channel.
        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        assert_eq!(e1.content.as_deref(), Some("one"));
        assert_eq!(e2.content.as_deref(), Some("two"));

        // Replay from the start returns both, in order.
        let replay = m.replay_events(&id, 0).await;
        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0].0, 1);
        assert_eq!(replay[0].1.content.as_deref(), Some("one"));
        assert_eq!(replay[1].0, 2);
        assert_eq!(replay[1].1.content.as_deref(), Some("two"));

        // Replay with since_seq=1 returns only the tail.
        let tail = m.replay_events(&id, 1).await;
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].0, 2);
    }

    #[tokio::test]
    async fn publish_event_persists_even_without_live_subscriber() {
        let (m, _t) = mgr();
        let id = m.create(req("no subscribers")).await.unwrap();
        // Never call open_stream — broadcast has no channel. Persistence
        // should still happen so a late-arriving client can replay.
        let seq = m
            .publish_event(&id, AgentEventPayload::chunk("solo".into()))
            .await
            .expect("persisted without broadcast");
        assert_eq!(seq, 1);
        let replay = m.replay_events(&id, 0).await;
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].1.content.as_deref(), Some("solo"));
    }

    #[tokio::test]
    async fn webhook_delivered_records_success_outcome() {
        use crate::webhook::{AttemptOutcome, RetryConfig};
        let (m, _t) = mgr();
        let id = m.create(req("webhook me")).await.unwrap();

        let cfg = RetryConfig::instant_for_tests(3);
        let out = m
            .deliver_webhook_with(&id, "https://example.test/hook", &cfg, |_| async {
                AttemptOutcome::Success(200)
            })
            .await;

        assert_eq!(out.status, "delivered");
        assert_eq!(out.attempts, 1);
        assert_eq!(out.last_http_status, Some(200));
        assert!(out.last_error.is_none());

        // Round-trips through the encrypted URL blob.
        let persisted = m.get_webhook_delivery(&id).await.expect("persisted");
        assert_eq!(persisted.url, "https://example.test/hook");
        assert_eq!(persisted.status, "delivered");
        assert_eq!(persisted.attempts, 1);
    }

    #[tokio::test]
    async fn webhook_transient_failures_retry_then_succeed() {
        use crate::webhook::{AttemptOutcome, RetryConfig};
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let (m, _t) = mgr();
        let id = m.create(req("retry me")).await.unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let cfg = RetryConfig::instant_for_tests(5);
        let out = m
            .deliver_webhook_with(&id, "https://example.test/hook", &cfg, move |_| {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                    if n < 3 {
                        AttemptOutcome::Transient(format!("HTTP 502 (n={n})"))
                    } else {
                        AttemptOutcome::Success(200)
                    }
                }
            })
            .await;

        assert_eq!(out.status, "delivered");
        assert_eq!(out.attempts, 3);
    }

    #[tokio::test]
    async fn webhook_dead_letter_is_persisted_and_listable() {
        use crate::webhook::{AttemptOutcome, RetryConfig};
        let (m, _t) = mgr();
        let id = m.create(req("dead letter me")).await.unwrap();

        let cfg = RetryConfig::instant_for_tests(3);
        let out = m
            .deliver_webhook_with(&id, "https://example.test/hook", &cfg, |n| async move {
                AttemptOutcome::Transient(format!("always fails, attempt {n}"))
            })
            .await;

        assert_eq!(out.status, "dead_letter");
        assert_eq!(out.attempts, 3);
        assert!(
            out.last_error.as_deref().unwrap_or("").contains("attempt 3"),
            "unexpected last_error: {:?}",
            out.last_error
        );

        let dead = m.list_dead_letter_webhooks().await;
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].session_id, id);
        assert_eq!(dead[0].status, "dead_letter");
    }

    #[tokio::test]
    async fn webhook_redelivery_upserts_same_row() {
        use crate::webhook::{AttemptOutcome, RetryConfig};
        let (m, _t) = mgr();
        let id = m.create(req("upsert me")).await.unwrap();

        let cfg = RetryConfig::instant_for_tests(2);
        // First attempt: dead letter.
        let first = m
            .deliver_webhook_with(&id, "https://example.test/hook", &cfg, |_| async {
                AttemptOutcome::Transient("500".into())
            })
            .await;
        assert_eq!(first.status, "dead_letter");

        // Retry: now succeeds. Should upsert to delivered, not create a
        // second row.
        let second = m
            .deliver_webhook_with(&id, "https://example.test/hook", &cfg, |_| async {
                AttemptOutcome::Success(200)
            })
            .await;
        assert_eq!(second.status, "delivered");

        let dead = m.list_dead_letter_webhooks().await;
        assert!(dead.is_empty(), "upsert should have cleared dead_letter state");

        let cur = m.get_webhook_delivery(&id).await.expect("row");
        assert_eq!(cur.status, "delivered");
    }

    #[tokio::test]
    async fn replay_events_is_scoped_per_session() {
        let (m, _t) = mgr();
        let a = m.create(req("session-a")).await.unwrap();
        let b = m.create(req("session-b")).await.unwrap();

        m.publish_event(&a, AgentEventPayload::chunk("a1".into())).await;
        m.publish_event(&b, AgentEventPayload::chunk("b1".into())).await;
        m.publish_event(&a, AgentEventPayload::chunk("a2".into())).await;

        let ra = m.replay_events(&a, 0).await;
        let rb = m.replay_events(&b, 0).await;

        assert_eq!(ra.len(), 2);
        assert_eq!(rb.len(), 1);
        // Seqs are per-session — both sessions start at 1.
        assert_eq!(ra[0].0, 1);
        assert_eq!(ra[1].0, 2);
        assert_eq!(rb[0].0, 1);
        assert_eq!(ra[0].1.content.as_deref(), Some("a1"));
        assert_eq!(ra[1].1.content.as_deref(), Some("a2"));
        assert_eq!(rb[0].1.content.as_deref(), Some("b1"));
    }

    #[tokio::test]
    async fn metrics_snapshot_reports_counters_and_gauges() {
        use crate::agent_quota::{Quota, ResourceKind};
        let (m, _t) = mgr();

        // Baseline — every counter is zero, nothing queued or running.
        let s0 = m.metrics_snapshot().await;
        assert_eq!(s0, JobManagerMetrics::default());

        // Two creates — one completes, one stays queued.
        let a = m.create(req("alpha")).await.unwrap();
        let _b = m.create(req("bravo")).await.unwrap();
        m.mark_terminal(&a, JobStatus::Complete, Some("ok".into()), None)
            .await
            .unwrap();

        // A third create that runs and fails.
        let c = m.create(req("charlie")).await.unwrap();
        m.mark_running(&c).await.unwrap();
        m.mark_terminal(&c, JobStatus::Failed, None, Some("boom".into()))
            .await
            .unwrap();

        // Quota denial path.
        m.set_agent_quotas("cap", vec![Quota::new(ResourceKind::Tasks, 0)])
            .await;
        let mut denied = req("nope");
        denied.quota_bucket = Some("cap".into());
        let err = m.create(denied).await.unwrap_err();
        assert!(matches!(err, SubmitError::QuotaDenied { .. }));

        // Cancel a queued job.
        let d = m.create(req("delta")).await.unwrap();
        m.cancel(&d, Some("user asked".into())).await.unwrap();

        // Event publish + replay.
        let e = m.create(req("echo")).await.unwrap();
        m.publish_event(&e, AgentEventPayload::chunk("e1".into()))
            .await;
        m.publish_event(&e, AgentEventPayload::chunk("e2".into()))
            .await;
        let replayed = m.replay_events(&e, 0).await;
        assert_eq!(replayed.len(), 2);

        // Webhook delivery — one succeeds, one dead-letters.
        let cfg = crate::webhook::RetryConfig::instant_for_tests(2);
        let ok_sid = m.create(req("wh-ok")).await.unwrap();
        let _ = m
            .deliver_webhook_with(&ok_sid, "https://example.test/ok", &cfg, |_| async {
                crate::webhook::AttemptOutcome::Success(204)
            })
            .await;
        let dl_sid = m.create(req("wh-dl")).await.unwrap();
        let _ = m
            .deliver_webhook_with(&dl_sid, "https://example.test/dl", &cfg, |_| async {
                crate::webhook::AttemptOutcome::Transient("nope".into())
            })
            .await;

        let s = m.metrics_snapshot().await;
        // 6 successful creates (alpha, bravo, charlie, delta, echo, wh-ok, wh-dl).
        assert_eq!(s.jobs_created, 7);
        assert_eq!(s.jobs_completed, 1);
        assert_eq!(s.jobs_failed, 1);
        assert_eq!(s.jobs_cancelled, 1);
        assert_eq!(s.quota_denied, 1);
        assert_eq!(s.events_published, 2);
        assert_eq!(s.events_replayed, 2);
        assert_eq!(s.webhooks_delivered, 1);
        assert_eq!(s.webhooks_dead_lettered, 1);
        // bravo, echo, wh-ok, wh-dl never transitioned out of queued.
        assert_eq!(s.queued, 4);
        assert_eq!(s.running, 0);
        // subprocess path is cfg(unix) and not exercised here.
        assert_eq!(s.subprocesses_spawned, 0);
    }

    // ── Scratchpad (Phase 3 of the memory-as-infrastructure redesign) ─────

    fn db() -> (JobsDb, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let db = JobsDb::open_with(&tmp.path().join("jobs.db"), [42u8; 32]).unwrap();
        (db, tmp)
    }

    #[test]
    fn scratchpad_set_then_get_roundtrips_utf8() {
        let (db, _t) = db();
        db.scratchpad_set("sid-1", "plan", "1. read\n2. write\n3. verify")
            .unwrap();
        assert_eq!(
            db.scratchpad_get("sid-1", "plan").unwrap().as_deref(),
            Some("1. read\n2. write\n3. verify")
        );
    }

    #[test]
    fn scratchpad_get_missing_returns_none() {
        let (db, _t) = db();
        assert!(db.scratchpad_get("sid-ghost", "nope").unwrap().is_none());
    }

    #[test]
    fn scratchpad_set_overwrites_existing_key() {
        let (db, _t) = db();
        db.scratchpad_set("sid-1", "k", "first").unwrap();
        db.scratchpad_set("sid-1", "k", "second").unwrap();
        assert_eq!(
            db.scratchpad_get("sid-1", "k").unwrap().as_deref(),
            Some("second")
        );
    }

    #[test]
    fn scratchpad_list_returns_entries_newest_first() {
        let (db, _t) = db();
        db.scratchpad_set("sid-1", "alpha", "A").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(3));
        db.scratchpad_set("sid-1", "beta", "B").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(3));
        db.scratchpad_set("sid-1", "gamma", "C").unwrap();
        let entries = db.scratchpad_list("sid-1").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].key, "gamma");
        assert_eq!(entries[0].value, "C");
        assert_eq!(entries[2].key, "alpha");
    }

    #[test]
    fn scratchpad_delete_removes_single_entry_and_returns_true_first_time() {
        let (db, _t) = db();
        db.scratchpad_set("sid-1", "k", "v").unwrap();
        assert!(db.scratchpad_delete("sid-1", "k").unwrap());
        assert!(!db.scratchpad_delete("sid-1", "k").unwrap());
        assert!(db.scratchpad_get("sid-1", "k").unwrap().is_none());
    }

    #[test]
    fn scratchpad_clear_removes_all_entries_for_session() {
        let (db, _t) = db();
        db.scratchpad_set("sid-1", "a", "1").unwrap();
        db.scratchpad_set("sid-1", "b", "2").unwrap();
        db.scratchpad_set("sid-2", "c", "3").unwrap();
        assert_eq!(db.scratchpad_clear("sid-1").unwrap(), 2);
        assert!(db.scratchpad_list("sid-1").unwrap().is_empty());
        // Other sessions untouched.
        assert_eq!(db.scratchpad_list("sid-2").unwrap().len(), 1);
    }

    #[test]
    fn scratchpad_isolated_between_sessions() {
        let (db, _t) = db();
        db.scratchpad_set("sid-A", "plan", "alpha-plan").unwrap();
        db.scratchpad_set("sid-B", "plan", "beta-plan").unwrap();
        assert_eq!(
            db.scratchpad_get("sid-A", "plan").unwrap().as_deref(),
            Some("alpha-plan")
        );
        assert_eq!(
            db.scratchpad_get("sid-B", "plan").unwrap().as_deref(),
            Some("beta-plan")
        );
    }

    // ── J1.1 job-recap CRUD ─────────────────────────────────────────────

    fn fresh_db() -> (JobsDb, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let db = JobsDb::open_with(&tmp.path().join("jobs.db"), [42u8; 32]).unwrap();
        (db, tmp)
    }

    fn seed_job(db: &JobsDb, session_id: &str) {
        let rec = JobRecord {
            session_id: session_id.to_string(),
            task: "test task".into(),
            status: "complete".into(),
            provider: "mock".into(),
            started_at: 1,
            finished_at: Some(2),
            summary: Some("ok".into()),
            priority: 5,
            queued_at: 0,
            webhook_url: None,
            tags: vec![],
            cancellation_reason: None,
            steps_completed: 0,
            tokens_used: 0,
            cost_cents: 0,
        };
        db.insert(&rec, "/tmp/ws", "auto").unwrap();
    }

    fn job_recap(subject_id: &str, last_seq: Option<i64>, headline: &str) -> crate::recap::Recap {
        crate::recap::Recap {
            id: format!("recap-{}", uuid::Uuid::new_v4().simple()),
            kind: crate::recap::RecapKind::Job,
            subject_id: subject_id.to_string(),
            last_message_id: last_seq,
            workspace: None,
            generated_at: chrono::Utc::now(),
            generator: crate::recap::RecapGenerator::Heuristic,
            headline: headline.to_string(),
            bullets: vec!["did a thing".into()],
            next_actions: vec![],
            artifacts: vec![],
            resume_hint: None,
            token_usage: None,
            schema_version: 1,
        }
    }

    #[test]
    fn job_recap_round_trips_via_jobsdb() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let recap = job_recap("job-A", Some(7), "Refactored the auth flow");
        let id = db.insert_job_recap(&recap).unwrap();
        let got = db.get_job_recap_by_id(&id).unwrap().expect("present");
        assert_eq!(got.headline, "Refactored the auth flow");
        assert_eq!(got.subject_id, "job-A");
        assert_eq!(got.last_message_id, Some(7));
        assert!(matches!(got.kind, crate::recap::RecapKind::Job));
        assert!(matches!(got.generator, crate::recap::RecapGenerator::Heuristic));
        assert_eq!(got.schema_version, 1);
    }

    #[test]
    fn job_recap_insert_is_idempotent_on_subject_and_seq() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let first = job_recap("job-A", Some(7), "first headline");
        let id1 = db.insert_job_recap(&first).unwrap();
        // Re-insert with the SAME (subject_id, last_event_seq) but a
        // different recap object — should return the original id, not
        // overwrite.
        let second = job_recap("job-A", Some(7), "second headline (ignored)");
        let id2 = db.insert_job_recap(&second).unwrap();
        assert_eq!(id1, id2);
        let got = db.get_job_recap_by_id(&id1).unwrap().unwrap();
        assert_eq!(got.headline, "first headline");
    }

    #[test]
    fn job_recap_idempotency_treats_null_seq_distinctly() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let with_seq = job_recap("job-A", Some(7), "with seq");
        let null_seq = job_recap("job-A", None, "null seq");
        let id_with = db.insert_job_recap(&with_seq).unwrap();
        let id_null = db.insert_job_recap(&null_seq).unwrap();
        assert_ne!(id_with, id_null,
            "null-seq recap should be a separate row");
    }

    #[test]
    fn job_recap_list_returns_newest_first() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let mut a = job_recap("job-A", Some(1), "older");
        a.generated_at = chrono::Utc::now() - chrono::Duration::seconds(60);
        let mut b = job_recap("job-A", Some(2), "newer");
        b.generated_at = chrono::Utc::now();
        db.insert_job_recap(&a).unwrap();
        db.insert_job_recap(&b).unwrap();
        let list = db.list_job_recaps_for_subject("job-A", 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].headline, "newer");
        assert_eq!(list[1].headline, "older");
    }

    #[test]
    fn job_recap_delete_removes_only_targeted_row() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let a = job_recap("job-A", Some(1), "keeper");
        let b = job_recap("job-A", Some(2), "victim");
        let id_a = db.insert_job_recap(&a).unwrap();
        let id_b = db.insert_job_recap(&b).unwrap();
        db.delete_job_recap(&id_b).unwrap();
        assert!(db.get_job_recap_by_id(&id_a).unwrap().is_some());
        assert!(db.get_job_recap_by_id(&id_b).unwrap().is_none());
    }

    #[test]
    fn job_recap_rejects_session_kind() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let mut wrong = job_recap("job-A", Some(1), "x");
        wrong.kind = crate::recap::RecapKind::Session;
        let err = db.insert_job_recap(&wrong).unwrap_err();
        assert!(err.contains("RecapKind::Job"), "got: {err}");
    }

    #[test]
    fn job_recap_cascade_delete_removes_recaps_when_job_deleted() {
        let (db, _t) = fresh_db();
        seed_job(&db, "job-A");
        let r = job_recap("job-A", Some(1), "doomed");
        let id = db.insert_job_recap(&r).unwrap();
        // Delete the parent job. With FK CASCADE, the recap row goes too.
        db.conn
            .execute("DELETE FROM jobs WHERE session_id = ?1", params!["job-A"])
            .unwrap();
        assert!(db.get_job_recap_by_id(&id).unwrap().is_none());
    }
}
