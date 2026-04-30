//! Slice 4 — Evaluation Suites.
//!
//! CRUD on `rl_eval_suites` + `rl_eval_results` (created in slice 1's
//! schema). An eval is just a special kind of run (kind='eval' in
//! `rl_runs`); slice 4 ships:
//!
//! 1. Suite definitions (YAML stored in `rl_eval_suites.config_yaml`).
//! 2. A way to launch an eval against a finished training run's
//!    artifact via the sidecar (`python -m vibe_rl eval ...`).
//! 3. Per-suite/per-metric result storage with bootstrap CIs.
//! 4. Pairwise comparison helpers used by `RLPolicyComparison`.
//!
//! See `docs/design/rl-os/04-evaluation.md`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::rl_runs::RunError;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    pub suite_id: String,
    pub name: String,
    pub description: Option<String>,
    pub config_yaml: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSuiteRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub config_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub run_id: String,
    pub suite_id: String,
    pub metric_name: String,
    pub value: f64,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
    pub n_episodes: i64,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultUpsert {
    pub metric_name: String,
    pub value: f64,
    #[serde(default)]
    pub ci_low: Option<f64>,
    #[serde(default)]
    pub ci_high: Option<f64>,
    pub n_episodes: i64,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRow {
    pub metric_name: String,
    pub value_a: Option<f64>,
    pub value_b: Option<f64>,
    pub difference: Option<f64>,
    pub n_a: i64,
    pub n_b: i64,
    /// Cohen's d, computed when both runs report n_episodes ≥ 2.
    pub effect_size: Option<f64>,
    /// Whether B improves on A (defined per-metric):
    /// rewards / success_rate higher is better; loss-style metrics not
    /// in the slice-4 set yet.
    pub improved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    pub run_a: String,
    pub run_b: String,
    pub suite_id: Option<String>,
    pub rows: Vec<ComparisonRow>,
}

// ── EvalStore ────────────────────────────────────────────────────────────────

pub struct EvalStore {
    conn: Mutex<Connection>,
    workspace_path: PathBuf,
}

impl EvalStore {
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

    // ── Suites ────────────────────────────────────────────────────────────────

    pub fn create_suite(&self, req: CreateSuiteRequest) -> Result<EvalSuite, RunError> {
        if req.name.trim().is_empty() {
            return Err(RunError::Invalid("suite name must be non-empty".into()));
        }
        if req.config_yaml.trim().is_empty() {
            return Err(RunError::Invalid("config_yaml must be non-empty".into()));
        }
        // Quick sanity-check the YAML parses; full schema validation
        // happens when the sidecar consumes the suite at run time.
        if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&req.config_yaml) {
            return Err(RunError::Invalid(format!("config_yaml is not valid YAML: {e}")));
        }
        let suite_id = format!("suite-{}", uuid::Uuid::new_v4());
        let now = now_ms();
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        conn.execute(
            "INSERT INTO rl_eval_suites (suite_id, name, description, config_yaml, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![suite_id, req.name, req.description, req.config_yaml, now],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(_, ref msg)
                if msg.as_deref().map_or(false, |m| m.contains("UNIQUE")) =>
            {
                RunError::Invalid(format!(
                    "a suite named '{}' already exists in this workspace",
                    req.name
                ))
            }
            other => RunError::from(other),
        })?;
        Ok(EvalSuite {
            suite_id,
            name: req.name,
            description: req.description,
            config_yaml: req.config_yaml,
            created_at: now,
        })
    }

    pub fn list_suites(&self) -> Result<Vec<EvalSuite>, RunError> {
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT suite_id, name, description, config_yaml, created_at
             FROM rl_eval_suites ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], row_to_suite)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_suite(&self, suite_id: &str) -> Result<Option<EvalSuite>, RunError> {
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT suite_id, name, description, config_yaml, created_at
             FROM rl_eval_suites WHERE suite_id = ?1",
        )?;
        match stmt.query_row(params![suite_id], row_to_suite) {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_suite(&self, suite_id: &str) -> Result<(), RunError> {
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let n = conn.execute(
            "DELETE FROM rl_eval_suites WHERE suite_id = ?1",
            params![suite_id],
        )?;
        if n == 0 {
            return Err(RunError::NotFound(suite_id.to_string()));
        }
        Ok(())
    }

    // ── Results ───────────────────────────────────────────────────────────────

    pub fn upsert_results(
        &self,
        run_id: &str,
        suite_id: &str,
        rows: &[ResultUpsert],
    ) -> Result<(), RunError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO rl_eval_results
                    (run_id, suite_id, metric_name, value, ci_low, ci_high, n_episodes, extra_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(run_id, suite_id, metric_name) DO UPDATE SET
                    value = excluded.value,
                    ci_low = excluded.ci_low,
                    ci_high = excluded.ci_high,
                    n_episodes = excluded.n_episodes,
                    extra_json = excluded.extra_json",
            )?;
            for r in rows {
                let extra_str = r
                    .extra
                    .as_ref()
                    .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".into()));
                stmt.execute(params![
                    run_id,
                    suite_id,
                    r.metric_name,
                    r.value,
                    r.ci_low,
                    r.ci_high,
                    r.n_episodes,
                    extra_str,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_results_for_run(&self, run_id: &str) -> Result<Vec<EvalResult>, RunError> {
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT run_id, suite_id, metric_name, value, ci_low, ci_high, n_episodes, extra_json
             FROM rl_eval_results WHERE run_id = ?1
             ORDER BY suite_id ASC, metric_name ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id], row_to_result)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_results_for_suite(
        &self,
        suite_id: &str,
    ) -> Result<Vec<EvalResult>, RunError> {
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT run_id, suite_id, metric_name, value, ci_low, ci_high, n_episodes, extra_json
             FROM rl_eval_results WHERE suite_id = ?1
             ORDER BY metric_name ASC, run_id ASC",
        )?;
        let rows = stmt
            .query_map(params![suite_id], row_to_result)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Comparison ────────────────────────────────────────────────────────────

    pub fn compare(
        &self,
        run_a: &str,
        run_b: &str,
        suite_id: Option<&str>,
    ) -> Result<ComparisonReport, RunError> {
        // Pull all results for both runs, restricted to suite if given.
        let conn = self.conn.lock().expect("rl_eval mutex poisoned");
        let mut sql = String::from(
            "SELECT run_id, suite_id, metric_name, value, ci_low, ci_high, n_episodes, extra_json
             FROM rl_eval_results WHERE run_id IN (?1, ?2)",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        args.push(Box::new(run_a.to_string()));
        args.push(Box::new(run_b.to_string()));
        if let Some(s) = suite_id {
            sql.push_str(" AND suite_id = ?");
            args.push(Box::new(s.to_string()));
        }
        let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(arg_refs.iter()), row_to_result)?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(conn);

        // Pivot by metric_name.
        use std::collections::HashMap;
        let mut by_metric: HashMap<String, (Option<EvalResult>, Option<EvalResult>)> =
            HashMap::new();
        for r in rows {
            let entry = by_metric.entry(r.metric_name.clone()).or_default();
            if r.run_id == run_a {
                entry.0 = Some(r);
            } else {
                entry.1 = Some(r);
            }
        }

        let mut report_rows: Vec<ComparisonRow> = Vec::new();
        let mut metric_names: Vec<String> = by_metric.keys().cloned().collect();
        metric_names.sort();
        for metric in metric_names {
            let (a, b) = by_metric.remove(&metric).unwrap();
            let value_a = a.as_ref().map(|r| r.value);
            let value_b = b.as_ref().map(|r| r.value);
            let n_a = a.as_ref().map(|r| r.n_episodes).unwrap_or(0);
            let n_b = b.as_ref().map(|r| r.n_episodes).unwrap_or(0);
            let difference = match (value_a, value_b) {
                (Some(va), Some(vb)) => Some(vb - va),
                _ => None,
            };
            let effect_size = cohens_d(a.as_ref(), b.as_ref());
            // For metrics where higher-is-better is the convention (most
            // RL evals: mean_return, success_rate, fqe_estimate), `improved`
            // means B's value strictly exceeds A. Lower-is-better metrics
            // (none in slice 4) would invert this.
            let improved = match (value_a, value_b) {
                (Some(va), Some(vb)) => vb > va,
                _ => false,
            };
            report_rows.push(ComparisonRow {
                metric_name: metric,
                value_a,
                value_b,
                difference,
                n_a,
                n_b,
                effect_size,
                improved,
            });
        }

        Ok(ComparisonReport {
            run_a: run_a.to_string(),
            run_b: run_b.to_string(),
            suite_id: suite_id.map(String::from),
            rows: report_rows,
        })
    }
}

/// Cohen's d effect size, computed when both rows expose CI bounds we can
/// back-derive a stddev estimate from. Approximates std as
/// `(ci_high - ci_low) / (2 * 1.96)` (the standard 95%-CI width formula
/// for normal-distributed estimates). When CIs aren't available we
/// return None.
fn cohens_d(a: Option<&EvalResult>, b: Option<&EvalResult>) -> Option<f64> {
    let a = a?;
    let b = b?;
    let std_a = ci_to_std(a)?;
    let std_b = ci_to_std(b)?;
    if a.n_episodes < 2 || b.n_episodes < 2 {
        return None;
    }
    let pooled = (((a.n_episodes - 1) as f64 * std_a.powi(2))
        + ((b.n_episodes - 1) as f64 * std_b.powi(2)))
        / ((a.n_episodes + b.n_episodes - 2) as f64);
    let pooled_std = pooled.sqrt();
    if pooled_std <= 0.0 {
        return None;
    }
    Some((b.value - a.value) / pooled_std)
}

fn ci_to_std(r: &EvalResult) -> Option<f64> {
    let lo = r.ci_low?;
    let hi = r.ci_high?;
    Some(((hi - lo) / (2.0 * 1.96)).abs())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_suite(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvalSuite> {
    Ok(EvalSuite {
        suite_id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        config_yaml: r.get(3)?,
        created_at: r.get(4)?,
    })
}

fn row_to_result(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvalResult> {
    let extra_str: Option<String> = r.get(7)?;
    let extra = extra_str
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null);
    Ok(EvalResult {
        run_id: r.get(0)?,
        suite_id: r.get(1)?,
        metric_name: r.get(2)?,
        value: r.get(3)?,
        ci_low: r.get(4)?,
        ci_high: r.get(5)?,
        n_episodes: r.get(6)?,
        extra,
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
    use crate::rl_runs::{CreateRunRequest, RunKind, RunStore};
    use tempfile::TempDir;

    fn open_stores() -> (TempDir, RunStore, EvalStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        let runs = RunStore::open_with(&db_path).unwrap();
        let evals = EvalStore::open_with(&db_path).unwrap();
        (tmp, runs, evals)
    }

    fn fake_run(store: &RunStore, ws: &Path, name: &str) -> String {
        store
            .create(CreateRunRequest {
                name: name.into(),
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
            .unwrap()
            .run_id
    }

    #[test]
    fn create_list_get_delete_suite_round_trip() {
        let (_tmp, _runs, evals) = open_stores();
        let suite = evals
            .create_suite(CreateSuiteRequest {
                name: "cartpole-robustness".into(),
                description: Some("perturbed dynamics".into()),
                config_yaml: "rollouts_per_env: 50\n".into(),
            })
            .unwrap();
        assert!(suite.suite_id.starts_with("suite-"));
        assert_eq!(evals.list_suites().unwrap().len(), 1);
        let fetched = evals.get_suite(&suite.suite_id).unwrap().unwrap();
        assert_eq!(fetched.name, "cartpole-robustness");
        evals.delete_suite(&suite.suite_id).unwrap();
        assert!(evals.list_suites().unwrap().is_empty());
    }

    #[test]
    fn create_suite_rejects_invalid_yaml() {
        let (_tmp, _runs, evals) = open_stores();
        let err = evals
            .create_suite(CreateSuiteRequest {
                name: "x".into(),
                description: None,
                config_yaml: "{ this is :::: not valid yaml".into(),
            })
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn create_suite_rejects_duplicate_name() {
        let (_tmp, _runs, evals) = open_stores();
        evals
            .create_suite(CreateSuiteRequest {
                name: "dup".into(),
                description: None,
                config_yaml: "k: 1\n".into(),
            })
            .unwrap();
        let err = evals
            .create_suite(CreateSuiteRequest {
                name: "dup".into(),
                description: None,
                config_yaml: "k: 2\n".into(),
            })
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(msg) if msg.contains("already exists")));
    }

    #[test]
    fn upsert_results_replaces_on_conflict() {
        let (tmp, runs, evals) = open_stores();
        let run_id = fake_run(&runs, tmp.path(), "r");
        let suite = evals
            .create_suite(CreateSuiteRequest {
                name: "s".into(),
                description: None,
                config_yaml: "k: 1\n".into(),
            })
            .unwrap();

        evals
            .upsert_results(
                &run_id,
                &suite.suite_id,
                &[ResultUpsert {
                    metric_name: "mean_return".into(),
                    value: 100.0,
                    ci_low: Some(95.0),
                    ci_high: Some(105.0),
                    n_episodes: 50,
                    extra: None,
                }],
            )
            .unwrap();
        evals
            .upsert_results(
                &run_id,
                &suite.suite_id,
                &[ResultUpsert {
                    metric_name: "mean_return".into(),
                    value: 120.0,
                    ci_low: Some(115.0),
                    ci_high: Some(125.0),
                    n_episodes: 100,
                    extra: None,
                }],
            )
            .unwrap();

        let rows = evals.list_results_for_run(&run_id).unwrap();
        assert_eq!(rows.len(), 1, "upsert must collapse to a single row");
        assert_eq!(rows[0].value, 120.0);
        assert_eq!(rows[0].n_episodes, 100);
    }

    #[test]
    fn compare_pivots_metrics_and_marks_improvement() {
        let (tmp, runs, evals) = open_stores();
        let a = fake_run(&runs, tmp.path(), "a");
        let b = fake_run(&runs, tmp.path(), "b");
        let suite = evals
            .create_suite(CreateSuiteRequest {
                name: "s".into(),
                description: None,
                config_yaml: "k: 1\n".into(),
            })
            .unwrap();

        evals
            .upsert_results(
                &a,
                &suite.suite_id,
                &[
                    ResultUpsert {
                        metric_name: "mean_return".into(),
                        value: 100.0,
                        ci_low: Some(95.0),
                        ci_high: Some(105.0),
                        n_episodes: 100,
                        extra: None,
                    },
                    ResultUpsert {
                        metric_name: "success_rate".into(),
                        value: 0.5,
                        ci_low: None,
                        ci_high: None,
                        n_episodes: 100,
                        extra: None,
                    },
                ],
            )
            .unwrap();
        evals
            .upsert_results(
                &b,
                &suite.suite_id,
                &[
                    ResultUpsert {
                        metric_name: "mean_return".into(),
                        value: 130.0,
                        ci_low: Some(125.0),
                        ci_high: Some(135.0),
                        n_episodes: 100,
                        extra: None,
                    },
                    ResultUpsert {
                        metric_name: "success_rate".into(),
                        value: 0.7,
                        ci_low: None,
                        ci_high: None,
                        n_episodes: 100,
                        extra: None,
                    },
                ],
            )
            .unwrap();

        let report = evals.compare(&a, &b, Some(&suite.suite_id)).unwrap();
        assert_eq!(report.rows.len(), 2);
        let mr = report.rows.iter().find(|r| r.metric_name == "mean_return").unwrap();
        assert_eq!(mr.value_a, Some(100.0));
        assert_eq!(mr.value_b, Some(130.0));
        assert_eq!(mr.difference, Some(30.0));
        assert!(mr.improved);
        // Cohen's d should be positive (B > A) and reasonably sized given the CIs.
        assert!(mr.effect_size.unwrap() > 1.0);

        let sr = report.rows.iter().find(|r| r.metric_name == "success_rate").unwrap();
        // 0.7 - 0.5 in fp64 is 0.19999…6, so compare with tolerance.
        assert!((sr.difference.unwrap() - 0.2).abs() < 1e-9);
        assert!(sr.improved);
        // No CIs given → effect_size None.
        assert!(sr.effect_size.is_none());
    }

    #[test]
    fn delete_suite_blocked_when_results_exist() {
        // The slice-1 schema defines `rl_eval_results.suite_id` as a
        // foreign key to `rl_eval_suites(suite_id)` without ON DELETE
        // CASCADE. With `PRAGMA foreign_keys=ON` (set by RunStore at
        // open), attempting to delete a suite that still has result rows
        // raises a FOREIGN KEY constraint error. This test pins the
        // current behavior; if we want cascade later, the migration goes
        // in slice 4.5.
        let (tmp, runs, evals) = open_stores();
        let run_id = fake_run(&runs, tmp.path(), "r");
        let suite = evals
            .create_suite(CreateSuiteRequest {
                name: "s".into(),
                description: None,
                config_yaml: "k: 1\n".into(),
            })
            .unwrap();
        evals
            .upsert_results(
                &run_id,
                &suite.suite_id,
                &[ResultUpsert {
                    metric_name: "mean_return".into(),
                    value: 1.0,
                    ci_low: None,
                    ci_high: None,
                    n_episodes: 1,
                    extra: None,
                }],
            )
            .unwrap();
        let err = evals.delete_suite(&suite.suite_id).unwrap_err();
        match err {
            RunError::Storage(msg) => assert!(msg.contains("FOREIGN KEY")),
            other => panic!("expected FK storage error, got {other:?}"),
        }
        // Results are still queryable.
        let rows = evals.list_results_for_run(&run_id).unwrap();
        assert_eq!(rows.len(), 1);
    }
}
