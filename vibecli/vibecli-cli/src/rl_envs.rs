//! Slice 3 — Environment Registry.
//!
//! CRUD on the `rl_environments` table (created in slice 1's migration).
//! Three sources of truth for env definitions:
//!
//! - **Gymnasium** — discovered via the sidecar's `probe-envs` subcommand
//!   when the user clicks "Refresh" or on first daemon startup. The
//!   probe requires a working `vibe-rl-py` venv; when absent we seed a
//!   small bundled set so the dashboard is never empty.
//! - **PettingZoo** — same probe path, slice 7b uses it for MARL.
//! - **custom_python** — user points at a `.py` file in the workspace
//!   that defines a `gymnasium.Env` subclass. Registered via
//!   `POST /v1/rl/envs/custom`. The file path is workspace-relative;
//!   sidecar imports + introspects + writes the spec_json.
//!
//! See `docs/design/rl-os/03-environments.md` for the full design.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::rl_runs::RunError; // reuse the same error enum

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub env_id: String,
    pub name: String,
    pub version: String,
    pub source: String, // 'gymnasium' | 'pettingzoo' | 'custom_python' | 'custom_dsl'
    /// `ObservationSpace`/`ActionSpace` info as captured from the env spec.
    /// Schema lives outside Rust because slice 1's plan stores it as JSON.
    pub spec_json: serde_json::Value,
    pub entry_point: Option<String>,
    pub file_path: Option<String>,
    pub parent_env_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnvFilter {
    pub source: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomEnvRequest {
    pub name: String,
    pub version: String,
    /// Workspace-relative path to a `.py` file defining a gym.Env subclass.
    pub file_path: String,
    /// Optional pre-computed spec_json. If None, the daemon asks the
    /// sidecar to import the file and introspect.
    #[serde(default)]
    pub spec_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshReport {
    pub source: String,
    pub added: i64,
    pub updated: i64,
    pub total: i64,
    pub sidecar_invoked: bool,
    pub error: Option<String>,
}

// ── EnvStore ─────────────────────────────────────────────────────────────────

pub struct EnvStore {
    conn: Mutex<Connection>,
    workspace_path: PathBuf,
}

impl EnvStore {
    pub fn open(workspace_path: &Path) -> Result<Self, RunError> {
        let canonical = workspace_path
            .canonicalize()
            .unwrap_or_else(|_| workspace_path.to_path_buf());
        let db_path = canonical.join(".vibecli").join("workspace.db");
        Self::open_at(&db_path, canonical)
    }

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
        // Ensure the slice-1 schema is in place on this DB. Idempotent
        // (CREATE TABLE IF NOT EXISTS) so it's safe even when RunStore
        // has already initialized the same file.
        crate::rl_runs::ensure_schema(&conn).map_err(RunError::from)?;
        Ok(Self {
            conn: Mutex::new(conn),
            workspace_path,
        })
    }

    #[allow(dead_code)]
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    pub fn get(&self, env_id: &str) -> Result<Option<Environment>, RunError> {
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT env_id, name, version, source, spec_json, entry_point, file_path, parent_env_id, created_at
             FROM rl_environments WHERE env_id = ?1",
        )?;
        match stmt.query_row(params![env_id], row_to_env) {
            Ok(env) => Ok(Some(env)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self, filter: EnvFilter) -> Result<Vec<Environment>, RunError> {
        let limit = filter.limit.unwrap_or(500).clamp(1, 5000);
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        let mut sql = String::from(
            "SELECT env_id, name, version, source, spec_json, entry_point, file_path, parent_env_id, created_at
             FROM rl_environments WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = filter.source {
            sql.push_str(" AND source = ?");
            args.push(Box::new(s));
        }
        if let Some(q) = filter.search {
            sql.push_str(" AND name LIKE ?");
            args.push(Box::new(format!("%{q}%")));
        }
        sql.push_str(" ORDER BY source ASC, name ASC LIMIT ?");
        args.push(Box::new(limit));

        let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(arg_refs.iter()), row_to_env)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// UPSERT an environment row. Used by `seed_defaults` and by the
    /// sidecar probe path. Keyed by `(name, version, source)` per the
    /// schema's UNIQUE constraint; on conflict updates spec_json + entry_point.
    pub fn upsert(&self, env: &Environment) -> Result<(), RunError> {
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        conn.execute(
            "INSERT INTO rl_environments
                (env_id, name, version, source, spec_json, entry_point, file_path, parent_env_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(name, version, source) DO UPDATE SET
                spec_json = excluded.spec_json,
                entry_point = excluded.entry_point",
            params![
                env.env_id,
                env.name,
                env.version,
                env.source,
                serde_json::to_string(&env.spec_json).map_err(|e| RunError::Storage(e.to_string()))?,
                env.entry_point,
                env.file_path,
                env.parent_env_id,
                env.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, env_id: &str) -> Result<(), RunError> {
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        // Only custom_* sources can be deleted; gym/pettingzoo are
        // refresh-managed (delete + re-probe is the right pattern there).
        let source: String = conn
            .query_row(
                "SELECT source FROM rl_environments WHERE env_id = ?1",
                params![env_id],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RunError::NotFound(env_id.to_string()),
                other => RunError::from(other),
            })?;
        if !source.starts_with("custom") {
            return Err(RunError::Invalid(format!(
                "cannot delete env with source '{source}' — only custom_* envs are user-deletable"
            )));
        }
        conn.execute("DELETE FROM rl_environments WHERE env_id = ?1", params![env_id])?;
        Ok(())
    }

    // ── Bootstrap ─────────────────────────────────────────────────────────────

    /// Insert a minimal set of envs so the dashboard isn't empty when the
    /// user hasn't yet installed `vibe-rl-py`. Idempotent (UPSERT). Slice
    /// 3's `Refresh` button replaces these with the live Gymnasium probe.
    pub fn seed_defaults(&self) -> Result<i64, RunError> {
        let now = now_ms();
        let mut count = 0;
        for spec in DEFAULT_ENVS.iter() {
            let env = Environment {
                env_id: format!("gym:{}:gym-bundled", spec.id),
                name: spec.id.to_string(),
                version: "gym-bundled".to_string(),
                source: "gymnasium".to_string(),
                spec_json: spec.spec_json.clone(),
                entry_point: Some(spec.entry_point.to_string()),
                file_path: None,
                parent_env_id: None,
                created_at: now,
            };
            self.upsert(&env)?;
            count += 1;
        }
        Ok(count)
    }

    /// Run `python -m vibe_rl probe-envs` and upsert the result. Returns
    /// a `RefreshReport` so the panel can surface "Added 3, updated 24,
    /// total 27" feedback.
    pub fn refresh_from_sidecar(
        &self,
        cfg: &crate::rl_executor::ExecutorConfig,
    ) -> Result<RefreshReport, RunError> {
        // Spawn synchronously — refresh is a one-shot probe; no streaming.
        let mut cmd = std::process::Command::new(&cfg.interpreter);
        cmd.arg("-m")
            .arg("vibe_rl")
            .arg("probe-envs")
            .env("PYTHONPATH", &cfg.sidecar_root)
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| RunError::Storage(format!("spawn vibe-rl-py: {e}")))?;
        if !output.status.success() {
            return Ok(RefreshReport {
                source: "gymnasium".into(),
                added: 0,
                updated: 0,
                total: self.count("gymnasium")?,
                sidecar_invoked: true,
                error: Some(format!(
                    "vibe-rl-py probe-envs exited {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| RunError::Storage(format!("parse probe-envs stdout: {e}")))?;
        let sdk_version = parsed
            .get("sdk_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let envs = parsed
            .get("envs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut added = 0;
        let mut updated = 0;
        let now = now_ms();
        for entry in envs {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            let entry_point = entry
                .get("entry_point")
                .and_then(|v| v.as_str())
                .map(String::from);
            let env_id = format!("gym:{id}:gym-{sdk_version}");
            let already_exists = self.get(&env_id)?.is_some();
            let env = Environment {
                env_id: env_id.clone(),
                name: id,
                version: format!("gym-{sdk_version}"),
                source: "gymnasium".to_string(),
                spec_json: entry.clone(),
                entry_point,
                file_path: None,
                parent_env_id: None,
                created_at: now,
            };
            self.upsert(&env)?;
            if already_exists {
                updated += 1;
            } else {
                added += 1;
            }
        }

        Ok(RefreshReport {
            source: "gymnasium".into(),
            added,
            updated,
            total: self.count("gymnasium")?,
            sidecar_invoked: true,
            error: None,
        })
    }

    pub fn count(&self, source: &str) -> Result<i64, RunError> {
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM rl_environments WHERE source = ?1",
            params![source],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    /// Register a workspace-local custom Python env. The `file_path`
    /// must already be a workspace-relative `.py` that exists.
    pub fn register_custom(
        &self,
        req: CustomEnvRequest,
        cfg: &crate::rl_executor::ExecutorConfig,
    ) -> Result<Environment, RunError> {
        let trimmed = req.name.trim();
        if trimmed.is_empty() {
            return Err(RunError::Invalid("env name must be non-empty".into()));
        }
        if req.version.trim().is_empty() {
            return Err(RunError::Invalid("env version must be non-empty".into()));
        }
        let file_rel = req.file_path.trim().to_string();
        if file_rel.is_empty() || file_rel.contains("..") || file_rel.starts_with('/') {
            return Err(RunError::Invalid(format!(
                "file_path must be a workspace-relative path without `..`: {file_rel}"
            )));
        }
        let abs = self.workspace_path.join(&file_rel);
        if !abs.is_file() {
            return Err(RunError::Invalid(format!(
                "file_path does not exist or is not a file: {file_rel}"
            )));
        }

        // If the caller provided a spec_json, trust it; otherwise we'd
        // ideally invoke the sidecar to introspect. For slice 3 we keep
        // it simple: stamp a minimal placeholder spec; slice 3.5 calls
        // a `python -m vibe_rl probe-custom-env --file <path>` to fill.
        let _ = cfg; // reserved for future probe-custom-env wiring
        let spec_json = req.spec_json.unwrap_or_else(|| {
            serde_json::json!({
                "id": req.name,
                "source": "custom_python",
                "note": "spec not yet introspected — slice 3.5 calls the sidecar",
            })
        });

        let now = now_ms();
        let env_id = format!("custom_python:{}:{}", req.name, req.version);
        let env = Environment {
            env_id: env_id.clone(),
            name: req.name,
            version: req.version,
            source: "custom_python".to_string(),
            spec_json,
            entry_point: None,
            file_path: Some(file_rel),
            parent_env_id: self.find_predecessor(&env_id)?,
            created_at: now,
        };
        self.upsert(&env)?;
        self.get(&env_id)?
            .ok_or_else(|| RunError::Storage("inserted env vanished".into()))
    }

    /// Look for an existing env with the same name + custom_python source
    /// and pick the most recent as the parent for the version DAG.
    fn find_predecessor(&self, env_id: &str) -> Result<Option<String>, RunError> {
        let conn = self.conn.lock().expect("rl_envs mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT env_id FROM rl_environments
             WHERE source = 'custom_python' AND env_id != ?1
             ORDER BY created_at DESC LIMIT 1",
        )?;
        match stmt.query_row(params![env_id], |r| r.get::<_, String>(0)) {
            Ok(parent) => Ok(Some(parent)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_env(r: &rusqlite::Row<'_>) -> rusqlite::Result<Environment> {
    let spec_str: String = r.get(4)?;
    let spec_json: serde_json::Value = serde_json::from_str(&spec_str).unwrap_or(serde_json::Value::Null);
    Ok(Environment {
        env_id: r.get(0)?,
        name: r.get(1)?,
        version: r.get(2)?,
        source: r.get(3)?,
        spec_json,
        entry_point: r.get(5)?,
        file_path: r.get(6)?,
        parent_env_id: r.get(7)?,
        created_at: r.get(8)?,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── Default seed (Gymnasium classic-control bundle) ──────────────────────────

struct DefaultSpec {
    id: &'static str,
    entry_point: &'static str,
    spec_json: serde_json::Value,
}

// `serde_json::json!` can't be const, so we build at first use.
impl DefaultSpec {
    fn new(id: &'static str, entry_point: &'static str, spec: serde_json::Value) -> Self {
        Self {
            id,
            entry_point,
            spec_json: spec,
        }
    }
}

static DEFAULT_ENVS: std::sync::LazyLock<Vec<DefaultSpec>> = std::sync::LazyLock::new(|| {
    vec![
        DefaultSpec::new(
            "CartPole-v1",
            "gymnasium.envs.classic_control.cartpole:CartPoleEnv",
            serde_json::json!({
                "id": "CartPole-v1",
                "observation_space": {"kind": "box", "shape": [4]},
                "action_space": {"kind": "discrete", "n": 2},
                "max_episode_steps": 500,
                "reward_threshold": 475.0,
                "nondeterministic": false,
            }),
        ),
        DefaultSpec::new(
            "MountainCar-v0",
            "gymnasium.envs.classic_control.mountain_car:MountainCarEnv",
            serde_json::json!({
                "id": "MountainCar-v0",
                "observation_space": {"kind": "box", "shape": [2]},
                "action_space": {"kind": "discrete", "n": 3},
                "max_episode_steps": 200,
                "reward_threshold": -110.0,
                "nondeterministic": false,
            }),
        ),
        DefaultSpec::new(
            "Acrobot-v1",
            "gymnasium.envs.classic_control.acrobot:AcrobotEnv",
            serde_json::json!({
                "id": "Acrobot-v1",
                "observation_space": {"kind": "box", "shape": [6]},
                "action_space": {"kind": "discrete", "n": 3},
                "max_episode_steps": 500,
                "reward_threshold": -100.0,
                "nondeterministic": false,
            }),
        ),
        DefaultSpec::new(
            "Pendulum-v1",
            "gymnasium.envs.classic_control.pendulum:PendulumEnv",
            serde_json::json!({
                "id": "Pendulum-v1",
                "observation_space": {"kind": "box", "shape": [3]},
                "action_space": {"kind": "box", "shape": [1]},
                "max_episode_steps": 200,
                "reward_threshold": null,
                "nondeterministic": false,
            }),
        ),
        DefaultSpec::new(
            "LunarLander-v2",
            "gymnasium.envs.box2d.lunar_lander:LunarLander",
            serde_json::json!({
                "id": "LunarLander-v2",
                "observation_space": {"kind": "box", "shape": [8]},
                "action_space": {"kind": "discrete", "n": 4},
                "max_episode_steps": 1000,
                "reward_threshold": 200.0,
                "nondeterministic": false,
                "extras": ["box2d (install via vibe-rl-py[box2d])"],
            }),
        ),
    ]
});

// Allow unused — slice 2's executor doesn't need this constant directly.
#[allow(dead_code)]
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_tmp_store() -> (TempDir, EnvStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        // Make sure the underlying schema exists by opening a RunStore first.
        let _ = crate::rl_runs::RunStore::open_with(&db_path).unwrap();
        let store = EnvStore::open_with(&db_path).unwrap();
        (tmp, store)
    }

    #[test]
    fn seed_defaults_inserts_classic_control_envs() {
        let (_tmp, store) = open_tmp_store();
        let n = store.seed_defaults().unwrap();
        assert!(n >= 5);
        let listed = store.list(EnvFilter::default()).unwrap();
        assert!(listed.iter().any(|e| e.name == "CartPole-v1"));
        assert!(listed.iter().any(|e| e.name == "Pendulum-v1"));
        assert!(listed.iter().all(|e| e.source == "gymnasium"));
    }

    #[test]
    fn upsert_is_idempotent_on_unique_key() {
        let (_tmp, store) = open_tmp_store();
        store.seed_defaults().unwrap();
        let count_before = store.count("gymnasium").unwrap();
        store.seed_defaults().unwrap();
        let count_after = store.count("gymnasium").unwrap();
        assert_eq!(count_before, count_after, "seeding twice must not double up");
    }

    #[test]
    fn list_filters_by_source_and_search() {
        let (_tmp, store) = open_tmp_store();
        store.seed_defaults().unwrap();
        let cartpole = store
            .list(EnvFilter {
                source: Some("gymnasium".into()),
                search: Some("CartPole".into()),
                limit: None,
            })
            .unwrap();
        assert_eq!(cartpole.len(), 1);
        assert_eq!(cartpole[0].name, "CartPole-v1");

        let none_match = store
            .list(EnvFilter {
                source: Some("custom_python".into()),
                ..EnvFilter::default()
            })
            .unwrap();
        assert!(none_match.is_empty());
    }

    #[test]
    fn register_custom_env_validates_path_and_existence() {
        let (tmp, store) = open_tmp_store();
        let cfg = crate::rl_executor::ExecutorConfig::from_env();

        // Missing file:
        let err = store
            .register_custom(
                CustomEnvRequest {
                    name: "my-env".into(),
                    version: "0.1.0".into(),
                    file_path: "envs/my_env.py".into(),
                    spec_json: None,
                },
                &cfg,
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));

        // Path traversal:
        let err = store
            .register_custom(
                CustomEnvRequest {
                    name: "x".into(),
                    version: "0.1.0".into(),
                    file_path: "../etc/passwd".into(),
                    spec_json: None,
                },
                &cfg,
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));

        // Happy path with a real file in the workspace:
        let envs_dir = tmp.path().join("envs");
        std::fs::create_dir_all(&envs_dir).unwrap();
        let env_file = envs_dir.join("my_env.py");
        std::fs::write(&env_file, "class MyEnv: pass\n").unwrap();
        let env = store
            .register_custom(
                CustomEnvRequest {
                    name: "my-env".into(),
                    version: "0.1.0".into(),
                    file_path: "envs/my_env.py".into(),
                    spec_json: None,
                },
                &cfg,
            )
            .unwrap();
        assert_eq!(env.source, "custom_python");
        assert_eq!(env.file_path.as_deref(), Some("envs/my_env.py"));
    }

    #[test]
    fn delete_blocks_gymnasium_envs() {
        let (_tmp, store) = open_tmp_store();
        store.seed_defaults().unwrap();
        let cp = store.list(EnvFilter::default()).unwrap();
        let cartpole_id = cp.iter().find(|e| e.name == "CartPole-v1").unwrap().env_id.clone();
        let err = store.delete(&cartpole_id).unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn delete_allows_custom_envs() {
        let (tmp, store) = open_tmp_store();
        let envs_dir = tmp.path().join("envs");
        std::fs::create_dir_all(&envs_dir).unwrap();
        std::fs::write(envs_dir.join("env.py"), "class E: pass\n").unwrap();
        let cfg = crate::rl_executor::ExecutorConfig::from_env();
        let env = store
            .register_custom(
                CustomEnvRequest {
                    name: "e".into(),
                    version: "0.1.0".into(),
                    file_path: "envs/env.py".into(),
                    spec_json: None,
                },
                &cfg,
            )
            .unwrap();
        store.delete(&env.env_id).unwrap();
        assert!(store.get(&env.env_id).unwrap().is_none());
    }
}
