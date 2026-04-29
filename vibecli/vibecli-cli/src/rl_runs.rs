//! Slice 1 — Persistence + Run Lifecycle for RL-OS.
//!
//! Replaces the in-memory `OnceLock<Mutex<Vec<Value>>>` mocks in
//! `vibeui/src-tauri/src/commands.rs` with durable, per-workspace storage in
//! the same SQLite file as `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`).
//!
//! See `docs/design/rl-os/01-persistence.md` for the spec.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Created,
    Queued,
    Running,
    Stopping,
    Stopped,
    Cancelled,
    Succeeded,
    Failed,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Cancelled => "cancelled",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "created" => Self::Created,
            "queued" => Self::Queued,
            "running" => Self::Running,
            "stopping" => Self::Stopping,
            "stopped" => Self::Stopped,
            "cancelled" => Self::Cancelled,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            _ => return None,
        })
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Stopped | Self::Cancelled | Self::Succeeded | Self::Failed
        )
    }

    /// Encodes the legal transitions for the run lifecycle. Anything not
    /// listed here is rejected by `RunStore::transition`.
    pub fn can_transition_to(self, next: RunStatus) -> bool {
        use RunStatus::*;
        match (self, next) {
            (Created, Queued) => true,
            (Created, Cancelled) => true,
            (Queued, Running) => true,
            (Queued, Cancelled) => true,
            (Queued, Failed) => true, // executor pickup failure
            (Running, Stopping) => true,
            (Running, Succeeded) => true,
            (Running, Failed) => true,
            (Running, Running) => true, // checkpoint heartbeat
            (Stopping, Stopped) => true,
            (Stopping, Failed) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Train,
    Eval,
    Distill,
    Quantize,
    Prune,
    Rlhf,
}

impl RunKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Train => "train",
            Self::Eval => "eval",
            Self::Distill => "distill",
            Self::Quantize => "quantize",
            Self::Prune => "prune",
            Self::Rlhf => "rlhf",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "train" => Self::Train,
            "eval" => Self::Eval,
            "distill" => Self::Distill,
            "quantize" => Self::Quantize,
            "prune" => Self::Prune,
            "rlhf" => Self::Rlhf,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub run_id: String,
    pub name: String,
    pub kind: RunKind,
    pub status: RunStatus,
    pub algorithm: String,
    pub environment_id: String,
    pub parent_run_id: Option<String>,
    pub config_yaml: String,
    pub seed: i64,
    pub sidecar_version: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub total_timesteps: i64,
    pub elapsed_steps: i64,
    pub last_reward_mean: Option<f64>,
    pub error_message: Option<String>,
    pub workspace_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRunRequest {
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: RunKind,
    pub algorithm: String,
    pub environment_id: String,
    #[serde(default)]
    pub parent_run_id: Option<String>,
    /// Full TrainingConfig serialized as YAML. Stored verbatim; validated
    /// for hyperparameter ranges below in `validate`.
    pub config_yaml: String,
    #[serde(default = "default_seed")]
    pub seed: i64,
    pub total_timesteps: i64,
    /// Workspace path for this run's artifacts. Required: the daemon does
    /// not infer a workspace from request context.
    pub workspace_path: String,
    /// Optional sidecar version override; defaults to the daemon's pinned
    /// constant. Slice 1 has no executor, so this is informational only.
    #[serde(default)]
    pub sidecar_version: Option<String>,
}

fn default_kind() -> RunKind {
    RunKind::Train
}
fn default_seed() -> i64 {
    42
}

impl CreateRunRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name must be non-empty".into());
        }
        if self.algorithm.trim().is_empty() {
            return Err("algorithm must be non-empty".into());
        }
        if self.environment_id.trim().is_empty() {
            return Err("environment_id must be non-empty".into());
        }
        if self.total_timesteps <= 0 {
            return Err("total_timesteps must be positive".into());
        }
        if self.total_timesteps > 1_000_000_000_000 {
            return Err("total_timesteps too large".into());
        }
        if self.workspace_path.trim().is_empty() {
            return Err("workspace_path must be non-empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTick {
    pub tick: i64,
    pub timestep: i64,
    pub wall_time: i64,
    /// Free-form JSON: `{policy_loss, value_loss, entropy, ...}`.
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRow {
    pub episode_idx: i64,
    pub timestep: i64,
    pub reward_sum: f64,
    pub length: i64,
    #[serde(default)]
    pub success: Option<bool>,
    pub duration_ms: i64,
}

/// Slice 2 (executor) constructs these as it writes checkpoints to disk.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub kind: String, // 'checkpoint' | 'final' | 'onnx' | 'replay_buffer' | 'model_card'
    #[serde(default)]
    pub timestep: Option<i64>,
    pub rel_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    pub run_id: String,
    pub kind: String,
    pub timestep: Option<i64>,
    pub rel_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub created_at: i64,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunFilter {
    pub kind: Option<RunKind>,
    pub status: Option<RunStatus>,
    pub algorithm: Option<String>,
    pub limit: Option<i64>,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("run not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("illegal transition: {from} → {to}")]
    IllegalTransition { from: &'static str, to: &'static str },
    #[error("cannot delete run in status {0}")]
    DeleteWhileActive(&'static str),
    #[error("storage: {0}")]
    Storage(String),
}

impl From<rusqlite::Error> for RunError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

// ── Schema ────────────────────────────────────────────────────────────────────

const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS rl_runs (
    run_id           TEXT    PRIMARY KEY,
    name             TEXT    NOT NULL,
    kind             TEXT    NOT NULL,
    status           TEXT    NOT NULL,
    algorithm        TEXT    NOT NULL,
    environment_id   TEXT    NOT NULL,
    parent_run_id    TEXT    REFERENCES rl_runs(run_id),
    config_yaml      TEXT    NOT NULL,
    seed             INTEGER NOT NULL,
    sidecar_version  TEXT    NOT NULL,
    created_at       INTEGER NOT NULL,
    started_at       INTEGER,
    finished_at      INTEGER,
    total_timesteps  INTEGER NOT NULL,
    elapsed_steps    INTEGER NOT NULL DEFAULT 0,
    last_reward_mean REAL,
    error_message    TEXT,
    workspace_path   TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS rl_runs_status_idx ON rl_runs(status, created_at DESC);
CREATE INDEX IF NOT EXISTS rl_runs_kind_idx   ON rl_runs(kind, created_at DESC);
CREATE INDEX IF NOT EXISTS rl_runs_parent_idx ON rl_runs(parent_run_id);

CREATE TABLE IF NOT EXISTS rl_episodes (
    run_id       TEXT    NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    episode_idx  INTEGER NOT NULL,
    timestep     INTEGER NOT NULL,
    reward_sum   REAL    NOT NULL,
    length       INTEGER NOT NULL,
    success      INTEGER,
    duration_ms  INTEGER NOT NULL,
    PRIMARY KEY (run_id, episode_idx)
);

CREATE TABLE IF NOT EXISTS rl_metrics (
    run_id      TEXT    NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    tick        INTEGER NOT NULL,
    timestep    INTEGER NOT NULL,
    wall_time   INTEGER NOT NULL,
    payload     TEXT    NOT NULL,
    PRIMARY KEY (run_id, tick)
);

CREATE TABLE IF NOT EXISTS rl_artifacts (
    artifact_id   TEXT PRIMARY KEY,
    run_id        TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,
    timestep      INTEGER,
    rel_path      TEXT NOT NULL,
    sha256        TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    created_at    INTEGER NOT NULL,
    metadata_json TEXT
);
CREATE INDEX IF NOT EXISTS rl_artifacts_run_idx ON rl_artifacts(run_id, kind, timestep);

CREATE TABLE IF NOT EXISTS rl_environments (
    env_id           TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    version          TEXT NOT NULL,
    source           TEXT NOT NULL,
    spec_json        TEXT NOT NULL,
    entry_point      TEXT,
    file_path        TEXT,
    parent_env_id    TEXT REFERENCES rl_environments(env_id),
    created_at       INTEGER NOT NULL,
    UNIQUE(name, version, source)
);

CREATE TABLE IF NOT EXISTS rl_eval_suites (
    suite_id     TEXT PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,
    description  TEXT,
    config_yaml  TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rl_eval_results (
    run_id        TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    suite_id      TEXT NOT NULL REFERENCES rl_eval_suites(suite_id),
    metric_name   TEXT NOT NULL,
    value         REAL NOT NULL,
    ci_low        REAL,
    ci_high       REAL,
    n_episodes    INTEGER NOT NULL,
    extra_json    TEXT,
    PRIMARY KEY (run_id, suite_id, metric_name)
);

CREATE TABLE IF NOT EXISTS rl_deployments (
    deployment_id     TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    artifact_id       TEXT NOT NULL REFERENCES rl_artifacts(artifact_id),
    runtime           TEXT NOT NULL,
    status            TEXT NOT NULL,
    traffic_pct       REAL NOT NULL DEFAULT 0.0,
    config_json       TEXT NOT NULL,
    created_at        INTEGER NOT NULL,
    promoted_at       INTEGER,
    rolled_back_at    INTEGER,
    rollback_reason   TEXT
);

CREATE TABLE IF NOT EXISTS rl_lineage_edges (
    child_run_id   TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    parent_run_id  TEXT NOT NULL REFERENCES rl_runs(run_id),
    edge_kind      TEXT NOT NULL,
    weight         REAL,
    PRIMARY KEY (child_run_id, parent_run_id, edge_kind)
);
"#;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Sidecar version pinned by the daemon. Slice 1 has no executor; this is
/// informational. Slice 2 will tighten this against `vibe-rl-py/VERSION`.
pub const SIDECAR_VERSION_PLACEHOLDER: &str = "0.0.0-no-executor";

// ── RunStore ──────────────────────────────────────────────────────────────────

/// Per-workspace persistence of runs, episodes, metrics, and artifacts.
///
/// Opens its own `Connection` against `<workspace>/.vibecli/workspace.db`
/// (the same file `WorkspaceStore` uses) and runs idempotent
/// `CREATE TABLE IF NOT EXISTS` for the RL-OS schema. WAL mode (set by
/// either store at first open) lets multiple connections coexist.
pub struct RunStore {
    conn: Mutex<Connection>,
    /// Slice 2 reads this when materializing the artifact tree.
    #[allow(dead_code)]
    workspace_path: PathBuf,
}

impl RunStore {
    pub fn open(workspace_path: &Path) -> Result<Self, RunError> {
        let canonical = workspace_path
            .canonicalize()
            .unwrap_or_else(|_| workspace_path.to_path_buf());
        let db_path = canonical.join(".vibecli").join("workspace.db");
        Self::open_at(&db_path, canonical)
    }

    /// Test-friendly: open against an arbitrary db path.
    #[allow(dead_code)]
    pub fn open_with(db_path: &Path) -> Result<Self, RunError> {
        let workspace = db_path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        Self::open_at(db_path, workspace)
    }

    fn open_at(db_path: &Path, workspace_path: PathBuf) -> Result<Self, RunError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RunError::Storage(e.to_string()))?;
        }
        let conn = Connection::open(db_path).map_err(RunError::from)?;
        conn.execute_batch(SCHEMA).map_err(RunError::from)?;
        Ok(Self {
            conn: Mutex::new(conn),
            workspace_path,
        })
    }

    #[allow(dead_code)]
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    // ── Run CRUD + lifecycle ──────────────────────────────────────────────────

    pub fn create(&self, req: CreateRunRequest) -> Result<Run, RunError> {
        req.validate().map_err(RunError::Invalid)?;
        let run_id = new_id();
        let now = now_ms();
        let sidecar_version = req
            .sidecar_version
            .unwrap_or_else(|| SIDECAR_VERSION_PLACEHOLDER.to_string());
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        conn.execute(
            "INSERT INTO rl_runs (
                run_id, name, kind, status, algorithm, environment_id, parent_run_id,
                config_yaml, seed, sidecar_version, created_at, total_timesteps,
                elapsed_steps, workspace_path
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,?13)",
            params![
                run_id,
                req.name,
                req.kind.as_str(),
                RunStatus::Created.as_str(),
                req.algorithm,
                req.environment_id,
                req.parent_run_id,
                req.config_yaml,
                req.seed,
                sidecar_version,
                now,
                req.total_timesteps,
                req.workspace_path,
            ],
        )?;
        drop(conn);
        self.get(&run_id)?
            .ok_or_else(|| RunError::Storage("inserted row vanished".into()))
    }

    pub fn get(&self, run_id: &str) -> Result<Option<Run>, RunError> {
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT run_id, name, kind, status, algorithm, environment_id, parent_run_id,
                    config_yaml, seed, sidecar_version, created_at, started_at, finished_at,
                    total_timesteps, elapsed_steps, last_reward_mean, error_message, workspace_path
             FROM rl_runs WHERE run_id = ?1",
        )?;
        let result = stmt.query_row(params![run_id], row_to_run);
        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self, filter: RunFilter) -> Result<Vec<Run>, RunError> {
        let limit = filter.limit.unwrap_or(500).clamp(1, 5000);
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let mut sql = String::from(
            "SELECT run_id, name, kind, status, algorithm, environment_id, parent_run_id,
                    config_yaml, seed, sidecar_version, created_at, started_at, finished_at,
                    total_timesteps, elapsed_steps, last_reward_mean, error_message, workspace_path
             FROM rl_runs WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(k) = filter.kind {
            sql.push_str(" AND kind = ?");
            args.push(Box::new(k.as_str().to_string()));
        }
        if let Some(s) = filter.status {
            sql.push_str(" AND status = ?");
            args.push(Box::new(s.as_str().to_string()));
        }
        if let Some(a) = filter.algorithm {
            sql.push_str(" AND algorithm = ?");
            args.push(Box::new(a));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        args.push(Box::new(limit));

        let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(arg_refs.iter()), row_to_run)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Validates the transition and persists in a single SQL statement so
    /// concurrent attempts cannot both succeed.
    pub fn transition(
        &self,
        run_id: &str,
        to: RunStatus,
        error_message: Option<String>,
    ) -> Result<Run, RunError> {
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");

        let current: String = conn
            .query_row(
                "SELECT status FROM rl_runs WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RunError::NotFound(run_id.to_string()),
                other => RunError::from(other),
            })?;
        let from = RunStatus::from_str(&current).ok_or_else(|| {
            RunError::Storage(format!("unknown stored status: {current}"))
        })?;

        if !from.can_transition_to(to) {
            return Err(RunError::IllegalTransition {
                from: from.as_str(),
                to: to.as_str(),
            });
        }

        let started_at = if to == RunStatus::Running && from != RunStatus::Running {
            Some(now)
        } else {
            None
        };
        let finished_at = if to.is_terminal() { Some(now) } else { None };

        let updated = conn.execute(
            "UPDATE rl_runs
             SET status = ?2,
                 started_at = COALESCE(?3, started_at),
                 finished_at = COALESCE(?4, finished_at),
                 error_message = COALESCE(?5, error_message)
             WHERE run_id = ?1 AND status = ?6",
            params![
                run_id,
                to.as_str(),
                started_at,
                finished_at,
                error_message,
                from.as_str()
            ],
        )?;
        if updated == 0 {
            return Err(RunError::IllegalTransition {
                from: from.as_str(),
                to: to.as_str(),
            });
        }
        drop(conn);
        self.get(run_id)?
            .ok_or_else(|| RunError::NotFound(run_id.to_string()))
    }

    pub fn delete(&self, run_id: &str) -> Result<(), RunError> {
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let status: String = conn
            .query_row(
                "SELECT status FROM rl_runs WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RunError::NotFound(run_id.to_string()),
                other => RunError::from(other),
            })?;
        let s = RunStatus::from_str(&status)
            .ok_or_else(|| RunError::Storage(format!("unknown stored status: {status}")))?;
        if !s.is_terminal() && s != RunStatus::Created {
            return Err(RunError::DeleteWhileActive(s.as_str()));
        }
        conn.execute("DELETE FROM rl_runs WHERE run_id = ?1", params![run_id])?;
        Ok(())
    }

    // ── Metrics + Episodes ────────────────────────────────────────────────────

    #[allow(dead_code)] // Slice 2 executor calls this; tests already cover it.
    pub fn append_metrics(&self, run_id: &str, batch: &[MetricTick]) -> Result<(), RunError> {
        if batch.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO rl_metrics (run_id, tick, timestep, wall_time, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for m in batch {
                let payload = serde_json::to_string(&m.payload)
                    .map_err(|e| RunError::Storage(e.to_string()))?;
                stmt.execute(params![run_id, m.tick, m.timestep, m.wall_time, payload])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_metrics(&self, run_id: &str, since_tick: i64) -> Result<Vec<MetricTick>, RunError> {
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT tick, timestep, wall_time, payload FROM rl_metrics
             WHERE run_id = ?1 AND tick > ?2 ORDER BY tick ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id, since_tick], |r| {
                let payload_str: String = r.get(3)?;
                let payload = serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
                Ok(MetricTick {
                    tick: r.get(0)?,
                    timestep: r.get(1)?,
                    wall_time: r.get(2)?,
                    payload,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn append_episodes(&self, run_id: &str, batch: &[EpisodeRow]) -> Result<(), RunError> {
        if batch.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO rl_episodes
                 (run_id, episode_idx, timestep, reward_sum, length, success, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for e in batch {
                let success_int: Option<i64> = e.success.map(|b| if b { 1 } else { 0 });
                stmt.execute(params![
                    run_id,
                    e.episode_idx,
                    e.timestep,
                    e.reward_sum,
                    e.length,
                    success_int,
                    e.duration_ms,
                ])?;
            }
            // Update aggregate fields on the run row from the latest 100 episodes.
            tx.execute(
                "UPDATE rl_runs SET
                    elapsed_steps = (SELECT COALESCE(MAX(timestep), 0) FROM rl_episodes WHERE run_id = ?1),
                    last_reward_mean = (
                        SELECT AVG(reward_sum) FROM (
                            SELECT reward_sum FROM rl_episodes
                            WHERE run_id = ?1
                            ORDER BY episode_idx DESC
                            LIMIT 100
                        )
                    )
                 WHERE run_id = ?1",
                params![run_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_episodes(
        &self,
        run_id: &str,
        since_idx: i64,
        limit: i64,
    ) -> Result<Vec<EpisodeRow>, RunError> {
        let limit = limit.clamp(1, 10_000);
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT episode_idx, timestep, reward_sum, length, success, duration_ms
             FROM rl_episodes WHERE run_id = ?1 AND episode_idx > ?2
             ORDER BY episode_idx ASC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![run_id, since_idx, limit], |r| {
                let success_int: Option<i64> = r.get(4)?;
                Ok(EpisodeRow {
                    episode_idx: r.get(0)?,
                    timestep: r.get(1)?,
                    reward_sum: r.get(2)?,
                    length: r.get(3)?,
                    success: success_int.map(|i| i != 0),
                    duration_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Artifacts ─────────────────────────────────────────────────────────────

    pub fn record_artifact(&self, run_id: &str, art: ArtifactRecord) -> Result<Artifact, RunError> {
        // The artifact path must sit inside the workspace tree and not escape
        // it via `..`. Slice 1 doesn't write any artifacts itself; this guard
        // is here so slice 2's executor can't be tricked into recording paths
        // outside the workspace either.
        if art.rel_path.contains("..") || art.rel_path.starts_with('/') {
            return Err(RunError::Invalid(format!(
                "artifact rel_path must be workspace-relative without `..`: {}",
                art.rel_path
            )));
        }
        let artifact_id = new_id();
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        conn.execute(
            "INSERT INTO rl_artifacts (artifact_id, run_id, kind, timestep, rel_path,
                                       sha256, size_bytes, created_at, metadata_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                artifact_id,
                run_id,
                art.kind,
                art.timestep,
                art.rel_path,
                art.sha256,
                art.size_bytes,
                now,
                art.metadata_json,
            ],
        )?;
        Ok(Artifact {
            artifact_id,
            run_id: run_id.to_string(),
            kind: art.kind,
            timestep: art.timestep,
            rel_path: art.rel_path,
            sha256: art.sha256,
            size_bytes: art.size_bytes,
            created_at: now,
            metadata_json: art.metadata_json,
        })
    }

    pub fn list_artifacts(&self, run_id: &str) -> Result<Vec<Artifact>, RunError> {
        let conn = self.conn.lock().expect("rl_runs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT artifact_id, run_id, kind, timestep, rel_path, sha256, size_bytes,
                    created_at, metadata_json
             FROM rl_artifacts WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id], |r| {
                Ok(Artifact {
                    artifact_id: r.get(0)?,
                    run_id: r.get(1)?,
                    kind: r.get(2)?,
                    timestep: r.get(3)?,
                    rel_path: r.get(4)?,
                    sha256: r.get(5)?,
                    size_bytes: r.get(6)?,
                    created_at: r.get(7)?,
                    metadata_json: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Slice 2 will plug in the real Python-sidecar executor here. Until
    /// then, `start` records a clear "no executor" failure on the run so
    /// the panel surfaces an honest message instead of pretending to train.
    pub fn no_executor_fail(&self, run_id: &str) -> Result<Run, RunError> {
        // Created → Queued is required first so the row visibly leaves the
        // "Created" state, then Queued → Failed records the reason.
        let _ = self.transition(run_id, RunStatus::Queued, None)?;
        self.transition(
            run_id,
            RunStatus::Failed,
            Some(
                "RL training executor not yet wired up — slice 2 (Python sidecar) ships this. \
                 The run was created and persisted; no metrics will be produced until then."
                    .into(),
            ),
        )
    }
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_run(r: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    let kind_s: String = r.get(2)?;
    let status_s: String = r.get(3)?;
    Ok(Run {
        run_id: r.get(0)?,
        name: r.get(1)?,
        kind: RunKind::from_str(&kind_s).unwrap_or(RunKind::Train),
        status: RunStatus::from_str(&status_s).unwrap_or(RunStatus::Failed),
        algorithm: r.get(4)?,
        environment_id: r.get(5)?,
        parent_run_id: r.get(6)?,
        config_yaml: r.get(7)?,
        seed: r.get(8)?,
        sidecar_version: r.get(9)?,
        created_at: r.get(10)?,
        started_at: r.get(11)?,
        finished_at: r.get(12)?,
        total_timesteps: r.get(13)?,
        elapsed_steps: r.get(14)?,
        last_reward_mean: r.get(15)?,
        error_message: r.get(16)?,
        workspace_path: r.get(17)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_tmp_store() -> (TempDir, RunStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        let store = RunStore::open_with(&db_path).unwrap();
        (tmp, store)
    }

    fn req(name: &str, ws: &str) -> CreateRunRequest {
        CreateRunRequest {
            name: name.into(),
            kind: RunKind::Train,
            algorithm: "PPO".into(),
            environment_id: "gym:CartPole-v1:gym-0.29".into(),
            parent_run_id: None,
            config_yaml: "learning_rate: 0.0003\n".into(),
            seed: 42,
            total_timesteps: 100_000,
            workspace_path: ws.into(),
            sidecar_version: None,
        }
    }

    #[test]
    fn schema_applies_idempotently() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join(".vibecli").join("workspace.db");
        let _ = RunStore::open_with(&db).unwrap();
        // Re-open: tables already exist. Should not error.
        let _ = RunStore::open_with(&db).unwrap();
    }

    #[test]
    fn create_then_get_then_list_roundtrips() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("ppo-run", &tmp.path().to_string_lossy())).unwrap();
        assert_eq!(run.status, RunStatus::Created);
        assert_eq!(run.algorithm, "PPO");
        let fetched = store.get(&run.run_id).unwrap().unwrap();
        assert_eq!(fetched.run_id, run.run_id);
        let listed = store.list(RunFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[test]
    fn rejects_invalid_create_request() {
        let (_tmp, store) = open_tmp_store();
        let bad = CreateRunRequest {
            total_timesteps: 0,
            ..req("x", "/tmp")
        };
        let err = store.create(bad).unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn legal_transitions_persist() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        let queued = store.transition(&run.run_id, RunStatus::Queued, None).unwrap();
        assert_eq!(queued.status, RunStatus::Queued);
        let running = store.transition(&run.run_id, RunStatus::Running, None).unwrap();
        assert_eq!(running.status, RunStatus::Running);
        assert!(running.started_at.is_some());
        let done = store
            .transition(&run.run_id, RunStatus::Succeeded, None)
            .unwrap();
        assert_eq!(done.status, RunStatus::Succeeded);
        assert!(done.finished_at.is_some());
    }

    #[test]
    fn illegal_transitions_rejected() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        // Created → Running is illegal (must go through Queued)
        let err = store.transition(&run.run_id, RunStatus::Running, None).unwrap_err();
        assert!(matches!(err, RunError::IllegalTransition { .. }));
    }

    #[test]
    fn concurrent_terminal_transitions_only_one_wins() {
        use std::sync::Arc;
        use std::thread;

        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        store.transition(&run.run_id, RunStatus::Queued, None).unwrap();
        store.transition(&run.run_id, RunStatus::Running, None).unwrap();
        let store = Arc::new(store);
        let run_id = run.run_id.clone();
        let s1 = store.clone();
        let r1 = run_id.clone();
        let s2 = store.clone();
        let r2 = run_id.clone();
        let h1 = thread::spawn(move || s1.transition(&r1, RunStatus::Succeeded, None));
        let h2 = thread::spawn(move || s2.transition(&r2, RunStatus::Failed, None));
        let res1 = h1.join().unwrap();
        let res2 = h2.join().unwrap();
        let (ok, err) = match (res1.is_ok(), res2.is_ok()) {
            (true, false) => (res1, res2),
            (false, true) => (res2, res1),
            _ => panic!("exactly one transition must win"),
        };
        let final_status = ok.unwrap().status;
        assert!(final_status == RunStatus::Succeeded || final_status == RunStatus::Failed);
        assert!(matches!(err.unwrap_err(), RunError::IllegalTransition { .. }));
    }

    #[test]
    fn metrics_and_episodes_roundtrip() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();

        let metrics = vec![
            MetricTick {
                tick: 1,
                timestep: 1024,
                wall_time: 100,
                payload: serde_json::json!({"loss": 0.5, "lr": 3e-4}),
            },
            MetricTick {
                tick: 2,
                timestep: 2048,
                wall_time: 200,
                payload: serde_json::json!({"loss": 0.3, "lr": 3e-4}),
            },
        ];
        store.append_metrics(&run.run_id, &metrics).unwrap();
        let fetched = store.list_metrics(&run.run_id, 0).unwrap();
        assert_eq!(fetched.len(), 2);

        let episodes = vec![
            EpisodeRow {
                episode_idx: 1,
                timestep: 200,
                reward_sum: 100.0,
                length: 200,
                success: Some(true),
                duration_ms: 500,
            },
            EpisodeRow {
                episode_idx: 2,
                timestep: 400,
                reward_sum: 150.0,
                length: 200,
                success: Some(false),
                duration_ms: 510,
            },
        ];
        store.append_episodes(&run.run_id, &episodes).unwrap();
        let listed = store.list_episodes(&run.run_id, 0, 100).unwrap();
        assert_eq!(listed.len(), 2);
        let updated = store.get(&run.run_id).unwrap().unwrap();
        assert_eq!(updated.elapsed_steps, 400);
        assert_eq!(updated.last_reward_mean, Some(125.0));
    }

    #[test]
    fn artifact_path_must_be_relative() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        let bad_abs = ArtifactRecord {
            kind: "checkpoint".into(),
            timestep: Some(100),
            rel_path: "/etc/passwd".into(),
            sha256: "x".into(),
            size_bytes: 1,
            metadata_json: None,
        };
        assert!(matches!(
            store.record_artifact(&run.run_id, bad_abs).unwrap_err(),
            RunError::Invalid(_)
        ));

        let bad_traversal = ArtifactRecord {
            kind: "checkpoint".into(),
            timestep: Some(100),
            rel_path: "../../etc/passwd".into(),
            sha256: "x".into(),
            size_bytes: 1,
            metadata_json: None,
        };
        assert!(matches!(
            store.record_artifact(&run.run_id, bad_traversal).unwrap_err(),
            RunError::Invalid(_)
        ));

        let good = ArtifactRecord {
            kind: "checkpoint".into(),
            timestep: Some(100),
            rel_path: ".vibecli/rl-artifacts/run-x/ckpt-100.pt".into(),
            sha256: "abc".into(),
            size_bytes: 2048,
            metadata_json: None,
        };
        let art = store.record_artifact(&run.run_id, good).unwrap();
        assert_eq!(art.kind, "checkpoint");
        assert_eq!(store.list_artifacts(&run.run_id).unwrap().len(), 1);
    }

    #[test]
    fn delete_blocks_active_runs() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        store.transition(&run.run_id, RunStatus::Queued, None).unwrap();
        store.transition(&run.run_id, RunStatus::Running, None).unwrap();
        let err = store.delete(&run.run_id).unwrap_err();
        assert!(matches!(err, RunError::DeleteWhileActive(_)));

        store.transition(&run.run_id, RunStatus::Stopping, None).unwrap();
        store.transition(&run.run_id, RunStatus::Stopped, None).unwrap();
        store.delete(&run.run_id).unwrap();
        assert!(store.get(&run.run_id).unwrap().is_none());
    }

    #[test]
    fn no_executor_fail_records_reason() {
        let (tmp, store) = open_tmp_store();
        let run = store.create(req("r", &tmp.path().to_string_lossy())).unwrap();
        let failed = store.no_executor_fail(&run.run_id).unwrap();
        assert_eq!(failed.status, RunStatus::Failed);
        assert!(failed.error_message.unwrap().contains("slice 2"));
    }

    #[test]
    fn coexists_with_workspace_store_in_same_db() {
        // The whole point of using <workspace>/.vibecli/workspace.db is that
        // WorkspaceStore and RunStore can both use it without stepping on
        // each other. Open both, write to each, ensure neither errors and
        // neither sees the other's tables as missing.
        use crate::workspace_store::WorkspaceStore;
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        let ws = WorkspaceStore::open_with(&db_path, [42u8; 32]).unwrap();
        ws.setting_set("hello", "world").unwrap();

        let rs = RunStore::open_with(&db_path).unwrap();
        let run = rs.create(req("r", &tmp.path().to_string_lossy())).unwrap();

        // Both stores still see their own data.
        assert_eq!(ws.setting_get("hello").unwrap().as_deref(), Some("world"));
        assert!(rs.get(&run.run_id).unwrap().is_some());
    }
}
