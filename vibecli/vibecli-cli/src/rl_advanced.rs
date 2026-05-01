//! Slice 7 — Advanced surfaces: Optimization (7a), Multi-Agent (7b), RLHF (7c).
//!
//! Slice 7 ships the **management** layer: storage, lifecycle, run-kind
//! routing, and panel data. The *compute* (vendoring TRL for RLHF,
//! MAPPO/QMIX for MARL, real distillation pipelines) lives in the sidecar
//! and ships behind a `[rlhf]` extra (TRL pulls HuggingFace transformers,
//! ~GB of weights for any real run) and a `[marl]` extra (PettingZoo).
//!
//! See `docs/design/rl-os/07-advanced.md`.
//!
//! What this module owns:
//!
//! - `PreferenceStore` — RLHF preference collection + judging (slice 7c)
//! - Helpers that query the existing `rl_runs` / `rl_metrics` tables to
//!   build the data the Optimization / Multi-Agent / RLHF panels need
//!
//! What lives elsewhere:
//!
//! - Run lifecycle for distill/quantize/prune/rlhf — already in
//!   `rl_runs::RunKind`, reuses the slice-2 executor.
//! - Lineage edges added at distill / merge / RLHF time — already in
//!   `rl_lineage_edges` (slice 1 schema, slice 5 walks it).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::rl_runs::{RunError, RunFilter, RunKind};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub pref_id: String,
    pub suite_id: Option<String>,
    pub prompt: String,
    pub completion_a: String,
    pub completion_b: String,
    pub chosen: Option<String>, // 'a' | 'b' | 'tie' | 'reject_both'
    pub rationale: Option<String>,
    pub reviewer: Option<String>,
    pub created_at: i64,
    pub judged_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePreferenceRequest {
    #[serde(default)]
    pub suite_id: Option<String>,
    pub prompt: String,
    pub completion_a: String,
    pub completion_b: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JudgePreferenceRequest {
    pub chosen: String, // 'a' | 'b' | 'tie' | 'reject_both'
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub reviewer: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlignmentScoreRow {
    pub run_id: String,
    pub metric: String,
    pub value: f64,
    pub timestep: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationRunSummary {
    pub run_id: String,
    pub name: String,
    pub kind: String, // 'distill' | 'quantize' | 'prune'
    pub algorithm: String,
    pub environment_id: String,
    pub status: String,
    pub total_timesteps: i64,
    pub elapsed_steps: i64,
    pub last_reward_mean: Option<f64>,
    pub created_at: i64,
    pub config_yaml: String,
}

// ── PreferenceStore ──────────────────────────────────────────────────────────

pub struct PreferenceStore {
    conn: Mutex<Connection>,
    workspace_path: PathBuf,
}

impl PreferenceStore {
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

    pub fn create(&self, req: CreatePreferenceRequest) -> Result<Preference, RunError> {
        if req.prompt.trim().is_empty() {
            return Err(RunError::Invalid("prompt must be non-empty".into()));
        }
        let pref_id = format!("pref-{}", uuid::Uuid::new_v4());
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_advanced mutex poisoned");
        conn.execute(
            "INSERT INTO rl_preferences (pref_id, suite_id, prompt, completion_a, completion_b, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pref_id,
                req.suite_id,
                req.prompt,
                req.completion_a,
                req.completion_b,
                now,
            ],
        )?;
        Ok(Preference {
            pref_id,
            suite_id: req.suite_id,
            prompt: req.prompt,
            completion_a: req.completion_a,
            completion_b: req.completion_b,
            chosen: None,
            rationale: None,
            reviewer: None,
            created_at: now,
            judged_at: None,
        })
    }

    pub fn judge(
        &self,
        pref_id: &str,
        req: JudgePreferenceRequest,
    ) -> Result<Preference, RunError> {
        match req.chosen.as_str() {
            "a" | "b" | "tie" | "reject_both" => {}
            other => {
                return Err(RunError::Invalid(format!(
                    "chosen must be one of a / b / tie / reject_both — got {other}"
                )))
            }
        }
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_advanced mutex poisoned");
        let updated = conn.execute(
            "UPDATE rl_preferences
             SET chosen = ?2, rationale = ?3, reviewer = ?4, judged_at = ?5
             WHERE pref_id = ?1",
            params![pref_id, req.chosen, req.rationale, req.reviewer, now],
        )?;
        if updated == 0 {
            return Err(RunError::NotFound(pref_id.to_string()));
        }
        drop(conn);
        self.get(pref_id)?
            .ok_or_else(|| RunError::NotFound(pref_id.to_string()))
    }

    pub fn get(&self, pref_id: &str) -> Result<Option<Preference>, RunError> {
        let conn = self.conn.lock().expect("rl_advanced mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT pref_id, suite_id, prompt, completion_a, completion_b,
                    chosen, rationale, reviewer, created_at, judged_at
             FROM rl_preferences WHERE pref_id = ?1",
        )?;
        match stmt.query_row(params![pref_id], row_to_pref) {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self, suite_id: Option<&str>) -> Result<Vec<Preference>, RunError> {
        let conn = self.conn.lock().expect("rl_advanced mutex poisoned");
        let (sql, args): (&str, Vec<String>) = match suite_id {
            Some(s) => (
                "SELECT pref_id, suite_id, prompt, completion_a, completion_b,
                        chosen, rationale, reviewer, created_at, judged_at
                 FROM rl_preferences WHERE suite_id = ?1 ORDER BY created_at DESC",
                vec![s.to_string()],
            ),
            None => (
                "SELECT pref_id, suite_id, prompt, completion_a, completion_b,
                        chosen, rationale, reviewer, created_at, judged_at
                 FROM rl_preferences ORDER BY created_at DESC",
                vec![],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if args.is_empty() {
            stmt.query_map([], row_to_pref)?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params_from_iter(args.iter()), row_to_pref)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    /// Read alignment scores written by an RLHF run. Slice 7c sidecar
    /// extension emits these in addition to the regular tick stream.
    pub fn alignment_scores(&self, run_id: &str) -> Result<Vec<AlignmentScoreRow>, RunError> {
        let conn = self.conn.lock().expect("rl_advanced mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT run_id, metric, value, timestep
             FROM rl_alignment_scores WHERE run_id = ?1 ORDER BY metric, timestep ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id], |r| {
                Ok(AlignmentScoreRow {
                    run_id: r.get(0)?,
                    metric: r.get(1)?,
                    value: r.get(2)?,
                    timestep: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

// ── Optimization run queries (Slice 7a) ──────────────────────────────────────
//
// Optimization runs are stored as regular rl_runs rows with kind in
// {distill, quantize, prune}. The `RLOptimizationReport` panel shows
// the most recent N of each kind plus their config + final metrics.

pub fn list_optimization_runs(
    runs: &crate::rl_runs::RunStore,
) -> Result<Vec<OptimizationRunSummary>, RunError> {
    let mut out = Vec::new();
    for kind in [RunKind::Distill, RunKind::Quantize, RunKind::Prune] {
        let rs = runs.list(RunFilter {
            kind: Some(kind.clone()),
            ..RunFilter::default()
        })?;
        for r in rs {
            out.push(OptimizationRunSummary {
                run_id: r.run_id,
                name: r.name,
                kind: kind.as_str().to_string(),
                algorithm: r.algorithm,
                environment_id: r.environment_id,
                status: r
                    .status
                    .as_str()
                    .to_string(),
                total_timesteps: r.total_timesteps,
                elapsed_steps: r.elapsed_steps,
                last_reward_mean: r.last_reward_mean,
                created_at: r.created_at,
                config_yaml: r.config_yaml,
            });
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

// ── Multi-agent helpers (Slice 7b) ───────────────────────────────────────────
//
// Multi-agent runs are regular Train runs with a multi-agent algorithm
// (MAPPO / QMIX / VDN / MADDPG). The `RLMultiAgentView` panel reads
// per-agent metric ticks straight out of `rl_metrics`. Slice 7b's
// management surface is the "is this run multi-agent?" classifier and
// the per-agent rollup of the existing tick stream.

pub fn is_multi_agent_algorithm(algo: &str) -> bool {
    matches!(algo, "MAPPO" | "QMIX" | "VDN" | "MADDPG")
}

#[derive(Debug, Clone, Serialize)]
pub struct MultiAgentRunSummary {
    pub run_id: String,
    pub name: String,
    pub algorithm: String,
    pub environment_id: String,
    pub status: String,
    /// Number of agents inferred from the run's config_yaml. None when
    /// the config doesn't declare it; the sidecar fills this in once the
    /// MARL extension lands.
    pub n_agents: Option<i64>,
    pub created_at: i64,
}

pub fn list_multi_agent_runs(
    runs: &crate::rl_runs::RunStore,
) -> Result<Vec<MultiAgentRunSummary>, RunError> {
    let all = runs.list(RunFilter {
        kind: Some(RunKind::Train),
        ..RunFilter::default()
    })?;
    let summaries = all
        .into_iter()
        .filter(|r| is_multi_agent_algorithm(&r.algorithm))
        .map(|r| {
            let n_agents = serde_yaml::from_str::<serde_yaml::Value>(&r.config_yaml)
                .ok()
                .and_then(|v| {
                    v.get("num_agents")
                        .or_else(|| v.get("n_agents"))
                        .and_then(|x| x.as_i64())
                });
            MultiAgentRunSummary {
                run_id: r.run_id,
                name: r.name,
                algorithm: r.algorithm,
                environment_id: r.environment_id,
                status: r.status.as_str().to_string(),
                n_agents,
                created_at: r.created_at,
            }
        })
        .collect();
    Ok(summaries)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_pref(r: &rusqlite::Row<'_>) -> rusqlite::Result<Preference> {
    Ok(Preference {
        pref_id: r.get(0)?,
        suite_id: r.get(1)?,
        prompt: r.get(2)?,
        completion_a: r.get(3)?,
        completion_b: r.get(4)?,
        chosen: r.get(5)?,
        rationale: r.get(6)?,
        reviewer: r.get(7)?,
        created_at: r.get(8)?,
        judged_at: r.get(9)?,
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
    use crate::rl_runs::{CreateRunRequest, RunKind, RunStatus, RunStore};
    use tempfile::TempDir;

    fn open_stores() -> (TempDir, RunStore, PreferenceStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        let runs = RunStore::open_with(&db_path).unwrap();
        let prefs = PreferenceStore::open_with(&db_path).unwrap();
        (tmp, runs, prefs)
    }

    fn create_run(store: &RunStore, ws: &Path, name: &str, kind: RunKind, algo: &str) -> String {
        store
            .create(CreateRunRequest {
                name: name.into(),
                kind,
                algorithm: algo.into(),
                environment_id: "gym:CartPole-v1:gym-bundled".into(),
                parent_run_id: None,
                config_yaml: "lr: 3e-4\nnum_agents: 4\n".into(),
                seed: 42,
                total_timesteps: 1000,
                workspace_path: ws.to_string_lossy().into(),
                sidecar_version: None,
            })
            .unwrap()
            .run_id
    }

    #[test]
    fn create_and_judge_preference() {
        let (_tmp, _runs, prefs) = open_stores();
        let p = prefs
            .create(CreatePreferenceRequest {
                suite_id: Some("suite-1".into()),
                prompt: "Summarize this".into(),
                completion_a: "Short".into(),
                completion_b: "Longer".into(),
            })
            .unwrap();
        assert!(p.pref_id.starts_with("pref-"));
        assert!(p.chosen.is_none());
        let judged = prefs
            .judge(
                &p.pref_id,
                JudgePreferenceRequest {
                    chosen: "b".into(),
                    rationale: Some("more thorough".into()),
                    reviewer: Some("alice".into()),
                },
            )
            .unwrap();
        assert_eq!(judged.chosen.as_deref(), Some("b"));
        assert!(judged.judged_at.is_some());
    }

    #[test]
    fn create_preference_rejects_empty_prompt() {
        let (_tmp, _runs, prefs) = open_stores();
        let err = prefs
            .create(CreatePreferenceRequest {
                suite_id: None,
                prompt: "  ".into(),
                completion_a: "a".into(),
                completion_b: "b".into(),
            })
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn judge_preference_rejects_invalid_chosen() {
        let (_tmp, _runs, prefs) = open_stores();
        let p = prefs
            .create(CreatePreferenceRequest {
                suite_id: None,
                prompt: "p".into(),
                completion_a: "a".into(),
                completion_b: "b".into(),
            })
            .unwrap();
        let err = prefs
            .judge(
                &p.pref_id,
                JudgePreferenceRequest {
                    chosen: "neither".into(),
                    rationale: None,
                    reviewer: None,
                },
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn list_preferences_filters_by_suite() {
        let (_tmp, _runs, prefs) = open_stores();
        prefs
            .create(CreatePreferenceRequest {
                suite_id: Some("s1".into()),
                prompt: "p".into(),
                completion_a: "a".into(),
                completion_b: "b".into(),
            })
            .unwrap();
        prefs
            .create(CreatePreferenceRequest {
                suite_id: Some("s2".into()),
                prompt: "p".into(),
                completion_a: "a".into(),
                completion_b: "b".into(),
            })
            .unwrap();
        assert_eq!(prefs.list(Some("s1")).unwrap().len(), 1);
        assert_eq!(prefs.list(Some("s2")).unwrap().len(), 1);
        assert_eq!(prefs.list(None).unwrap().len(), 2);
    }

    #[test]
    fn list_optimization_runs_picks_up_distill_quantize_prune() {
        let (tmp, runs, _prefs) = open_stores();
        create_run(&runs, tmp.path(), "train1", RunKind::Train, "PPO");
        create_run(&runs, tmp.path(), "distill1", RunKind::Distill, "PPO");
        create_run(&runs, tmp.path(), "quant1", RunKind::Quantize, "PPO");
        let opts = list_optimization_runs(&runs).unwrap();
        assert_eq!(opts.len(), 2);
        let kinds: std::collections::HashSet<&str> = opts.iter().map(|o| o.kind.as_str()).collect();
        assert!(kinds.contains("distill"));
        assert!(kinds.contains("quantize"));
        assert!(!kinds.contains("train"));
    }

    #[test]
    fn list_multi_agent_runs_filters_by_algorithm() {
        let (tmp, runs, _prefs) = open_stores();
        create_run(&runs, tmp.path(), "ppo1", RunKind::Train, "PPO");
        let mappo_id = create_run(&runs, tmp.path(), "mappo1", RunKind::Train, "MAPPO");
        create_run(&runs, tmp.path(), "qmix1", RunKind::Train, "QMIX");
        let ma = list_multi_agent_runs(&runs).unwrap();
        assert_eq!(ma.len(), 2);
        let mappo = ma.iter().find(|r| r.run_id == mappo_id).unwrap();
        assert_eq!(mappo.n_agents, Some(4)); // pulled from config_yaml
    }

    #[test]
    fn alignment_scores_returns_empty_for_run_without_rlhf_data() {
        let (tmp, runs, prefs) = open_stores();
        let r = create_run(&runs, tmp.path(), "rlhf-x", RunKind::Rlhf, "PPO");
        // Mark it Succeeded so the lifecycle is consistent.
        runs.transition(&r, RunStatus::Queued, None).unwrap();
        runs.transition(&r, RunStatus::Running, None).unwrap();
        runs.transition(&r, RunStatus::Succeeded, None).unwrap();
        assert!(prefs.alignment_scores(&r).unwrap().is_empty());
    }
}
