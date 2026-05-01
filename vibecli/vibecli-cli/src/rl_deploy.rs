//! Slice 6 — Deployment lifecycle.
//!
//! CRUD + state machine on the `rl_deployments` table (created in slice 1).
//! Slice 6 ships the **management** surface: register a deployment from a
//! `Policy`, promote it through staging → canary → production with traffic
//! split, manual rollback. The actual inference path (`/v1/rl/serve/.../act`)
//! is **slice 6.5** — it requires either:
//!
//! - the `ort` Rust crate (ONNX Runtime) — adds a C++ build dep, ~5 MB
//!   binary growth, but cleanest one-process inference, OR
//! - a long-lived sidecar in inference mode that `Policy.framework =
//!   pytorch` requires anyway.
//!
//! Both choices have non-trivial deployment implications (vendored
//! libonnxruntime on macOS, Python venv lifecycle on the host) and the
//! design doc 06-deployment.md explicitly leaves the runtime selection
//! as opt-in. Slice 6 in this commit therefore stops at "deployment is
//! registered, lifecycle is correct, panel shows real status" — the
//! `act` route returns 501 with a clear "wire a runtime first" message.
//!
//! See `docs/design/rl-os/06-deployment.md`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::rl_runs::RunError;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Staging,
    Canary,
    Production,
    RolledBack,
    Stopped,
}

impl DeploymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
            Self::RolledBack => "rolled_back",
            Self::Stopped => "stopped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "staging" => Self::Staging,
            "canary" => Self::Canary,
            "production" => Self::Production,
            "rolled_back" => Self::RolledBack,
            "stopped" => Self::Stopped,
            _ => return None,
        })
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::RolledBack | Self::Stopped)
    }

    /// Lifecycle transitions:
    /// - staging   → canary | production | rolled_back | stopped
    /// - canary    → production | rolled_back | stopped
    /// - production → rolled_back | stopped
    /// - rolled_back → (terminal)
    /// - stopped → (terminal)
    pub fn can_transition_to(self, next: DeploymentStatus) -> bool {
        use DeploymentStatus::*;
        match (self, next) {
            (Staging, Canary) | (Staging, Production) | (Staging, RolledBack) | (Staging, Stopped) => true,
            (Canary, Production) | (Canary, RolledBack) | (Canary, Stopped) => true,
            (Production, RolledBack) | (Production, Stopped) => true,
            // Same-state transition allowed for traffic_pct updates.
            (Staging, Staging) | (Canary, Canary) | (Production, Production) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub deployment_id: String,
    pub name: String,
    pub artifact_id: String,
    pub runtime: String, // 'onnx' | 'python' | 'native_candle' (slice 6: just stored)
    pub status: DeploymentStatus,
    pub traffic_pct: f64,
    pub config: serde_json::Value,
    pub created_at: i64,
    pub promoted_at: Option<i64>,
    pub rolled_back_at: Option<i64>,
    pub rollback_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDeploymentRequest {
    pub name: String,
    /// `artifact_id` from `rl_artifacts` (typically the `kind='final'`
    /// artifact of a finished run, or — once slice 5 is wired — a
    /// policy's `primary_artifact`/`onnx_artifact`).
    pub artifact_id: String,
    /// `'onnx' | 'python' | 'native_candle'`. Slice 6 stores the choice
    /// but doesn't validate against an installed runtime; slice 6.5
    /// gates this on a per-runtime feature flag.
    #[serde(default = "default_runtime")]
    pub runtime: String,
    #[serde(default)]
    pub traffic_pct: f64,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

fn default_runtime() -> String {
    "python".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromoteRequest {
    pub to: String, // 'canary' | 'production'
    #[serde(default)]
    pub traffic_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RollbackRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthSnapshot {
    pub deployment_id: String,
    pub name: String,
    pub status: String,
    pub runtime: String,
    pub traffic_pct: f64,
    pub uptime_seconds: i64,
    pub created_at: i64,
    pub promoted_at: Option<i64>,
    pub rolled_back_at: Option<i64>,
    pub rollback_reason: Option<String>,
    /// Slice 6 doesn't measure real latency; we surface the storage
    /// fields and a structured "metrics not yet implemented" marker.
    /// Slice 6.5 swaps this for a `tdigest`-backed snapshot.
    pub note: Option<String>,
}

// ── DeploymentStore ──────────────────────────────────────────────────────────

pub struct DeploymentStore {
    conn: Mutex<Connection>,
    workspace_path: PathBuf,
}

impl DeploymentStore {
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

    // ── Create + read ─────────────────────────────────────────────────────────

    pub fn create(&self, req: CreateDeploymentRequest) -> Result<Deployment, RunError> {
        if req.name.trim().is_empty() {
            return Err(RunError::Invalid("deployment name must be non-empty".into()));
        }
        if req.traffic_pct < 0.0 || req.traffic_pct > 100.0 {
            return Err(RunError::Invalid(
                "traffic_pct must be between 0 and 100".into(),
            ));
        }
        if !matches!(req.runtime.as_str(), "onnx" | "python" | "native_candle") {
            return Err(RunError::Invalid(format!(
                "runtime '{}' is not supported. Slice 6 accepts: onnx, python, native_candle.",
                req.runtime
            )));
        }
        let deployment_id = format!("dep-{}", uuid::Uuid::new_v4());
        let now = now_ms();
        let config = req
            .config
            .clone()
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let conn = self.conn.lock().expect("rl_deploy mutex poisoned");
        conn.execute(
            "INSERT INTO rl_deployments
                (deployment_id, name, artifact_id, runtime, status, traffic_pct,
                 config_json, created_at)
             VALUES (?1, ?2, ?3, ?4, 'staging', ?5, ?6, ?7)",
            params![
                deployment_id,
                req.name,
                req.artifact_id,
                req.runtime,
                req.traffic_pct,
                serde_json::to_string(&config).map_err(|e| RunError::Storage(e.to_string()))?,
                now,
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(_, ref m) if m.as_deref().map_or(false, |x| x.contains("FOREIGN KEY")) => {
                RunError::Invalid(format!(
                    "artifact {} not found in rl_artifacts — register the run's artifact first",
                    req.artifact_id
                ))
            }
            other => RunError::from(other),
        })?;
        drop(conn);
        self.get(&deployment_id)?
            .ok_or_else(|| RunError::Storage("inserted deployment vanished".into()))
    }

    pub fn get(&self, deployment_id: &str) -> Result<Option<Deployment>, RunError> {
        let conn = self.conn.lock().expect("rl_deploy mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT deployment_id, name, artifact_id, runtime, status, traffic_pct,
                    config_json, created_at, promoted_at, rolled_back_at, rollback_reason
             FROM rl_deployments WHERE deployment_id = ?1",
        )?;
        match stmt.query_row(params![deployment_id], row_to_deployment) {
            Ok(d) => Ok(Some(d)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self) -> Result<Vec<Deployment>, RunError> {
        let conn = self.conn.lock().expect("rl_deploy mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT deployment_id, name, artifact_id, runtime, status, traffic_pct,
                    config_json, created_at, promoted_at, rolled_back_at, rollback_reason
             FROM rl_deployments ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], row_to_deployment)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    pub fn promote(
        &self,
        deployment_id: &str,
        req: PromoteRequest,
    ) -> Result<Deployment, RunError> {
        let target = DeploymentStatus::from_str(&req.to).ok_or_else(|| {
            RunError::Invalid(format!(
                "promotion target '{}' is not a deployment status",
                req.to
            ))
        })?;
        if let Some(pct) = req.traffic_pct {
            if !(0.0..=100.0).contains(&pct) {
                return Err(RunError::Invalid("traffic_pct must be between 0 and 100".into()));
            }
        }
        self.transition_internal(deployment_id, target, req.traffic_pct, None)
    }

    pub fn rollback(
        &self,
        deployment_id: &str,
        req: RollbackRequest,
    ) -> Result<Deployment, RunError> {
        if req.reason.trim().is_empty() {
            return Err(RunError::Invalid("rollback reason must be non-empty".into()));
        }
        self.transition_internal(
            deployment_id,
            DeploymentStatus::RolledBack,
            Some(0.0),
            Some(req.reason),
        )
    }

    pub fn stop(&self, deployment_id: &str) -> Result<Deployment, RunError> {
        self.transition_internal(deployment_id, DeploymentStatus::Stopped, Some(0.0), None)
    }

    fn transition_internal(
        &self,
        deployment_id: &str,
        target: DeploymentStatus,
        traffic_pct: Option<f64>,
        rollback_reason: Option<String>,
    ) -> Result<Deployment, RunError> {
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_deploy mutex poisoned");

        let current_str: String = conn
            .query_row(
                "SELECT status FROM rl_deployments WHERE deployment_id = ?1",
                params![deployment_id],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    RunError::NotFound(deployment_id.to_string())
                }
                other => RunError::from(other),
            })?;
        let from = DeploymentStatus::from_str(&current_str).ok_or_else(|| {
            RunError::Storage(format!("unknown stored deployment status: {current_str}"))
        })?;
        if !from.can_transition_to(target) {
            return Err(RunError::IllegalTransition {
                from: from.as_str(),
                to: target.as_str(),
            });
        }

        let promoted_at = if matches!(target, DeploymentStatus::Production) && from != DeploymentStatus::Production {
            Some(now)
        } else {
            None
        };
        let rolled_back_at = if matches!(target, DeploymentStatus::RolledBack) {
            Some(now)
        } else {
            None
        };

        let updated = conn.execute(
            "UPDATE rl_deployments
             SET status = ?2,
                 traffic_pct = COALESCE(?3, traffic_pct),
                 promoted_at = COALESCE(?4, promoted_at),
                 rolled_back_at = COALESCE(?5, rolled_back_at),
                 rollback_reason = COALESCE(?6, rollback_reason)
             WHERE deployment_id = ?1 AND status = ?7",
            params![
                deployment_id,
                target.as_str(),
                traffic_pct,
                promoted_at,
                rolled_back_at,
                rollback_reason,
                from.as_str()
            ],
        )?;
        if updated == 0 {
            return Err(RunError::IllegalTransition {
                from: from.as_str(),
                to: target.as_str(),
            });
        }
        drop(conn);
        self.get(deployment_id)?
            .ok_or_else(|| RunError::NotFound(deployment_id.to_string()))
    }

    // ── Health ────────────────────────────────────────────────────────────────

    pub fn health(&self, deployment_id: &str) -> Result<HealthSnapshot, RunError> {
        let d = self
            .get(deployment_id)?
            .ok_or_else(|| RunError::NotFound(deployment_id.to_string()))?;
        let now = now_ms();
        let started_at = d.promoted_at.unwrap_or(d.created_at);
        let uptime_seconds = if d.status.is_terminal() {
            d.rolled_back_at.unwrap_or(now).saturating_sub(started_at) / 1000
        } else {
            (now - started_at).max(0) / 1000
        };
        Ok(HealthSnapshot {
            deployment_id: d.deployment_id.clone(),
            name: d.name.clone(),
            status: d.status.as_str().to_string(),
            runtime: d.runtime.clone(),
            traffic_pct: d.traffic_pct,
            uptime_seconds,
            created_at: d.created_at,
            promoted_at: d.promoted_at,
            rolled_back_at: d.rolled_back_at,
            rollback_reason: d.rollback_reason.clone(),
            note: Some(
                "Slice 6 reports lifecycle health only. Latency / throughput / error rates land in slice 6.5 alongside the inference runtime.".into(),
            ),
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_deployment(r: &rusqlite::Row<'_>) -> rusqlite::Result<Deployment> {
    let cfg_str: String = r.get(6)?;
    let cfg = serde_json::from_str(&cfg_str).unwrap_or(serde_json::Value::Null);
    let status_str: String = r.get(4)?;
    let status = DeploymentStatus::from_str(&status_str).unwrap_or(DeploymentStatus::Stopped);
    Ok(Deployment {
        deployment_id: r.get(0)?,
        name: r.get(1)?,
        artifact_id: r.get(2)?,
        runtime: r.get(3)?,
        status,
        traffic_pct: r.get(5)?,
        config: cfg,
        created_at: r.get(7)?,
        promoted_at: r.get(8)?,
        rolled_back_at: r.get(9)?,
        rollback_reason: r.get(10)?,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rl_runs::{ArtifactRecord, CreateRunRequest, RunKind, RunStore};
    use tempfile::TempDir;

    fn open_stores() -> (TempDir, RunStore, DeploymentStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        let runs = RunStore::open_with(&db_path).unwrap();
        let deps = DeploymentStore::open_with(&db_path).unwrap();
        (tmp, runs, deps)
    }

    fn run_and_artifact(store: &RunStore, ws: &Path) -> String {
        let run = store
            .create(CreateRunRequest {
                name: "r".into(),
                kind: RunKind::Train,
                algorithm: "PPO".into(),
                environment_id: "gym:CartPole-v1:gym-bundled".into(),
                parent_run_id: None,
                config_yaml: "lr: 3e-4\n".into(),
                seed: 42,
                total_timesteps: 1000,
                workspace_path: ws.to_string_lossy().into(),
                sidecar_version: None,
            })
            .unwrap();
        let art = store
            .record_artifact(
                &run.run_id,
                ArtifactRecord {
                    kind: "final".into(),
                    timestep: Some(1000),
                    rel_path: format!(".vibecli/rl-artifacts/{}/final.pt", run.run_id),
                    sha256: "deadbeef".into(),
                    size_bytes: 4096,
                    metadata_json: None,
                },
            )
            .unwrap();
        art.artifact_id
    }

    #[test]
    fn create_and_round_trip() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let d = deps
            .create(CreateDeploymentRequest {
                name: "cartpole-prod".into(),
                artifact_id: art_id.clone(),
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap();
        assert_eq!(d.status, DeploymentStatus::Staging);
        assert_eq!(d.runtime, "python");
        assert_eq!(deps.list().unwrap().len(), 1);
        assert_eq!(deps.get(&d.deployment_id).unwrap().unwrap().name, "cartpole-prod");
    }

    #[test]
    fn create_rejects_unknown_artifact() {
        let (_tmp, _runs, deps) = open_stores();
        let err = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: "art-does-not-exist".into(),
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn create_rejects_unsupported_runtime() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let err = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: art_id,
                runtime: "burn".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn lifecycle_legal_transitions() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let d = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: art_id,
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap();

        // staging → canary @ 10%
        let c = deps
            .promote(
                &d.deployment_id,
                PromoteRequest {
                    to: "canary".into(),
                    traffic_pct: Some(10.0),
                },
            )
            .unwrap();
        assert_eq!(c.status, DeploymentStatus::Canary);
        assert_eq!(c.traffic_pct, 10.0);

        // canary → production @ 100%
        let p = deps
            .promote(
                &d.deployment_id,
                PromoteRequest {
                    to: "production".into(),
                    traffic_pct: Some(100.0),
                },
            )
            .unwrap();
        assert_eq!(p.status, DeploymentStatus::Production);
        assert!(p.promoted_at.is_some());
        assert_eq!(p.traffic_pct, 100.0);

        // production → rolled_back
        let r = deps
            .rollback(
                &d.deployment_id,
                RollbackRequest {
                    reason: "p99 latency breach".into(),
                },
            )
            .unwrap();
        assert_eq!(r.status, DeploymentStatus::RolledBack);
        assert_eq!(r.rollback_reason.as_deref(), Some("p99 latency breach"));
    }

    #[test]
    fn lifecycle_rejects_illegal_transitions() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let d = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: art_id,
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap();
        deps.stop(&d.deployment_id).unwrap();
        let err = deps
            .promote(
                &d.deployment_id,
                PromoteRequest {
                    to: "production".into(),
                    traffic_pct: None,
                },
            )
            .unwrap_err();
        assert!(matches!(err, RunError::IllegalTransition { .. }));
    }

    #[test]
    fn rollback_requires_reason() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let d = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: art_id,
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap();
        let err = deps
            .rollback(
                &d.deployment_id,
                RollbackRequest {
                    reason: "  ".into(),
                },
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn health_reports_uptime_and_lifecycle_only() {
        let (tmp, runs, deps) = open_stores();
        let art_id = run_and_artifact(&runs, tmp.path());
        let d = deps
            .create(CreateDeploymentRequest {
                name: "x".into(),
                artifact_id: art_id,
                runtime: "python".into(),
                traffic_pct: 0.0,
                config: None,
            })
            .unwrap();
        let h = deps.health(&d.deployment_id).unwrap();
        assert_eq!(h.deployment_id, d.deployment_id);
        assert_eq!(h.status, "staging");
        assert!(h.note.is_some(), "slice 6 health note must explain that latency lands in slice 6.5");
        assert!(h.uptime_seconds >= 0);
    }
}
