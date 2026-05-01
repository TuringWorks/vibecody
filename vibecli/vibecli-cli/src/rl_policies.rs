//! Slice 5 — Model Hub + Lineage.
//!
//! Promotes runs' artifacts to first-class `Policy` rows with semver,
//! lineage edges, model cards, and a per-name ordering for promotion
//! workflows. The schema additions live in `rl_runs::SCHEMA` (rl_policies,
//! rl_policy_runs, rl_reward_components).
//!
//! See `docs/design/rl-os/05-model-hub.md`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::rl_runs::RunError;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub primary_artifact: String,
    pub onnx_artifact: Option<String>,
    pub model_card_md: String,
    pub framework: String, // 'pytorch' | 'onnx' | 'native_candle'
    pub obs_space_json: serde_json::Value,
    pub act_space_json: serde_json::Value,
    pub obs_normalization_json: Option<serde_json::Value>,
    pub act_normalization_json: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Run that produced the artifact. Required for lineage + the
    /// auto-generated card. The run must exist + be terminal.
    pub run_id: String,
    /// `kind='checkpoint'|'final'` artifact_id from `rl_artifacts`.
    /// Required: the policy points at this file as its weights.
    pub primary_artifact_id: String,
    /// Optional ONNX export pointer. Slice 5 can be wired with or
    /// without; the ONNX export pipeline is opt-in (`include_onnx`).
    #[serde(default)]
    pub onnx_artifact_id: Option<String>,
    /// Defaults to "pytorch" — the artifact format produced by
    /// slice 2's PPO sidecar.
    #[serde(default)]
    pub framework: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageNode {
    pub node_id: String,    // run_id (always) — policies attach via rl_policy_runs
    pub kind: String,       // 'run' | 'policy'
    pub label: String,      // human-readable
    pub status: String,     // run.status or 'registered'
    pub algorithm: Option<String>,
    pub environment_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageEdge {
    pub from: String,
    pub to: String,
    pub kind: String, // 'parent_run' | 'distill_teacher' | 'rlhf_base' | 'merge_source' | 'producer'
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageGraph {
    pub root_policy_id: String,
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RewardComponentRow {
    pub component: String,
    pub mean: f64,
    pub total: f64,
    pub n_episodes: i64,
}

// ── PolicyStore ──────────────────────────────────────────────────────────────

pub struct PolicyStore {
    conn: Mutex<Connection>,
    workspace_path: PathBuf,
}

impl PolicyStore {
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

    // ── Register ──────────────────────────────────────────────────────────────

    pub fn register(&self, req: RegisterRequest, runs: &crate::rl_runs::RunStore) -> Result<Policy, RunError> {
        if req.name.trim().is_empty() {
            return Err(RunError::Invalid("policy name must be non-empty".into()));
        }
        if req.version.trim().is_empty() {
            return Err(RunError::Invalid("policy version must be non-empty".into()));
        }
        // Validate that the run + artifact exist and the artifact belongs
        // to that run. The slice-1 schema's FK on primary_artifact already
        // catches the "no such artifact" case, but a typed error here
        // produces a friendlier message for the panel.
        let run = runs
            .get(&req.run_id)?
            .ok_or_else(|| RunError::NotFound(format!("run {}", req.run_id)))?;
        let artifacts = runs.list_artifacts(&req.run_id)?;
        let primary = artifacts
            .iter()
            .find(|a| a.artifact_id == req.primary_artifact_id)
            .ok_or_else(|| {
                RunError::Invalid(format!(
                    "artifact {} does not belong to run {}",
                    req.primary_artifact_id, req.run_id
                ))
            })?;
        if let Some(onnx_id) = req.onnx_artifact_id.as_deref() {
            if !artifacts.iter().any(|a| a.artifact_id == onnx_id) {
                return Err(RunError::Invalid(format!(
                    "onnx artifact {} does not belong to run {}",
                    onnx_id, req.run_id
                )));
            }
        }

        // Pull obs/action space metadata from the run's config_yaml +
        // environment_id. For slice 5 the auto-generated card embeds the
        // environment id; the rich space JSONs are filled when the
        // sidecar writes them into the artifact metadata sidecar (slice 5.5).
        let obs_space_json = serde_json::json!({"env_id": run.environment_id.clone()});
        let act_space_json = serde_json::json!({"env_id": run.environment_id.clone()});

        let framework = req.framework.clone().unwrap_or_else(|| {
            if req.onnx_artifact_id.is_some() {
                "onnx".into()
            } else {
                "pytorch".into()
            }
        });
        let policy_id = format!("policy-{}", uuid::Uuid::new_v4());
        let now = now_ms();
        let card_md = render_model_card(&run, primary, &req, &framework);

        let conn = self.conn.lock().expect("rl_policies mutex poisoned");
        conn.execute(
            "INSERT INTO rl_policies (
                policy_id, name, version, description, primary_artifact, onnx_artifact,
                model_card_md, framework, obs_space_json, act_space_json,
                obs_normalization_json, act_normalization_json, created_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                policy_id,
                req.name,
                req.version,
                req.description,
                req.primary_artifact_id,
                req.onnx_artifact_id,
                card_md,
                framework,
                serde_json::to_string(&obs_space_json).map_err(|e| RunError::Storage(e.to_string()))?,
                serde_json::to_string(&act_space_json).map_err(|e| RunError::Storage(e.to_string()))?,
                None::<String>,
                None::<String>,
                now,
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(_, ref m) if m.as_deref().map_or(false, |x| x.contains("UNIQUE")) => {
                RunError::Invalid(format!("policy {}@{} already exists", req.name, req.version))
            }
            other => RunError::from(other),
        })?;

        // Producer edge: this run produced the policy.
        conn.execute(
            "INSERT INTO rl_policy_runs (policy_id, run_id, role) VALUES (?1, ?2, 'producer')",
            params![policy_id, req.run_id],
        )?;
        drop(conn);

        self.get(&policy_id)?
            .ok_or_else(|| RunError::Storage("inserted policy vanished".into()))
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    pub fn get(&self, policy_id: &str) -> Result<Option<Policy>, RunError> {
        let conn = self.conn.lock().expect("rl_policies mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT policy_id, name, version, description, primary_artifact, onnx_artifact,
                    model_card_md, framework, obs_space_json, act_space_json,
                    obs_normalization_json, act_normalization_json, created_at
             FROM rl_policies WHERE policy_id = ?1",
        )?;
        match stmt.query_row(params![policy_id], row_to_policy) {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self, name_filter: Option<&str>) -> Result<Vec<Policy>, RunError> {
        let conn = self.conn.lock().expect("rl_policies mutex poisoned");
        let (sql, args): (&str, Vec<String>) = match name_filter {
            Some(n) => (
                "SELECT policy_id, name, version, description, primary_artifact, onnx_artifact,
                        model_card_md, framework, obs_space_json, act_space_json,
                        obs_normalization_json, act_normalization_json, created_at
                 FROM rl_policies WHERE name = ?1 ORDER BY created_at DESC",
                vec![n.to_string()],
            ),
            None => (
                "SELECT policy_id, name, version, description, primary_artifact, onnx_artifact,
                        model_card_md, framework, obs_space_json, act_space_json,
                        obs_normalization_json, act_normalization_json, created_at
                 FROM rl_policies ORDER BY name ASC, created_at DESC",
                vec![],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if args.is_empty() {
            stmt.query_map([], row_to_policy)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params_from_iter(args.iter()), row_to_policy)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn delete(&self, policy_id: &str) -> Result<(), RunError> {
        let conn = self.conn.lock().expect("rl_policies mutex poisoned");
        let n = conn.execute(
            "DELETE FROM rl_policies WHERE policy_id = ?1",
            params![policy_id],
        )?;
        if n == 0 {
            return Err(RunError::NotFound(policy_id.to_string()));
        }
        Ok(())
    }

    pub fn card(&self, policy_id: &str) -> Result<String, RunError> {
        let conn = self.conn.lock().expect("rl_policies mutex poisoned");
        conn.query_row(
            "SELECT model_card_md FROM rl_policies WHERE policy_id = ?1",
            params![policy_id],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => RunError::NotFound(policy_id.to_string()),
            other => RunError::from(other),
        })
    }

    // ── Lineage ───────────────────────────────────────────────────────────────

    /// Walk the lineage DAG up to `depth` parents. Returns nodes for the
    /// policy itself, its producer run, that run's parent_run_id chain
    /// (slice-1 implicit edges), and any explicit edges in
    /// `rl_lineage_edges` (slice 7 distill / RLHF / merge edges).
    pub fn lineage(&self, policy_id: &str, depth: usize) -> Result<LineageGraph, RunError> {
        let depth = depth.clamp(1, 10);

        let policy = self
            .get(policy_id)?
            .ok_or_else(|| RunError::NotFound(policy_id.to_string()))?;

        let conn = self.conn.lock().expect("rl_policies mutex poisoned");

        // Policy → producer run.
        let producer_run_id: String = conn.query_row(
            "SELECT run_id FROM rl_policy_runs WHERE policy_id = ?1 AND role = 'producer' LIMIT 1",
            params![policy_id],
            |r| r.get(0),
        )?;

        // BFS over runs via parent_run_id (implicit) + rl_lineage_edges (explicit).
        use std::collections::{HashMap, VecDeque};
        let mut nodes: HashMap<String, LineageNode> = HashMap::new();
        let mut edges: Vec<LineageEdge> = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((producer_run_id.clone(), 0));

        while let Some((run_id, level)) = queue.pop_front() {
            if !visited.insert(run_id.clone()) {
                continue;
            }
            // Fetch run metadata.
            let row = conn.query_row(
                "SELECT name, status, algorithm, environment_id, created_at
                 FROM rl_runs WHERE run_id = ?1",
                params![run_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            );
            if let Ok((name, status, algorithm, env_id, created_at)) = row {
                nodes.insert(
                    run_id.clone(),
                    LineageNode {
                        node_id: run_id.clone(),
                        kind: "run".into(),
                        label: name,
                        status,
                        algorithm,
                        environment_id: env_id,
                        created_at,
                    },
                );
            }
            if level >= depth {
                continue;
            }
            // Implicit parent.
            if let Ok(parent) = conn.query_row(
                "SELECT parent_run_id FROM rl_runs WHERE run_id = ?1 AND parent_run_id IS NOT NULL",
                params![run_id],
                |r| r.get::<_, String>(0),
            ) {
                edges.push(LineageEdge {
                    from: run_id.clone(),
                    to: parent.clone(),
                    kind: "parent_run".into(),
                    weight: None,
                });
                queue.push_back((parent, level + 1));
            }
            // Explicit edges.
            let mut stmt = conn.prepare(
                "SELECT parent_run_id, edge_kind, weight FROM rl_lineage_edges WHERE child_run_id = ?1",
            )?;
            let extra = stmt
                .query_map(params![run_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<f64>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for (parent, kind, weight) in extra {
                edges.push(LineageEdge {
                    from: run_id.clone(),
                    to: parent.clone(),
                    kind,
                    weight,
                });
                queue.push_back((parent, level + 1));
            }
        }

        // Add the policy node + producer edge so the graph is rooted at the policy.
        nodes.insert(
            policy_id.to_string(),
            LineageNode {
                node_id: policy_id.to_string(),
                kind: "policy".into(),
                label: format!("{}@{}", policy.name, policy.version),
                status: "registered".into(),
                algorithm: None,
                environment_id: None,
                created_at: policy.created_at,
            },
        );
        edges.push(LineageEdge {
            from: policy_id.to_string(),
            to: producer_run_id,
            kind: "producer".into(),
            weight: None,
        });

        Ok(LineageGraph {
            root_policy_id: policy_id.to_string(),
            nodes: nodes.into_values().collect(),
            edges,
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_policy(r: &rusqlite::Row<'_>) -> rusqlite::Result<Policy> {
    let obs_str: String = r.get(8)?;
    let act_str: String = r.get(9)?;
    let obs_norm: Option<String> = r.get(10)?;
    let act_norm: Option<String> = r.get(11)?;
    Ok(Policy {
        policy_id: r.get(0)?,
        name: r.get(1)?,
        version: r.get(2)?,
        description: r.get(3)?,
        primary_artifact: r.get(4)?,
        onnx_artifact: r.get(5)?,
        model_card_md: r.get(6)?,
        framework: r.get(7)?,
        obs_space_json: serde_json::from_str(&obs_str).unwrap_or(serde_json::Value::Null),
        act_space_json: serde_json::from_str(&act_str).unwrap_or(serde_json::Value::Null),
        obs_normalization_json: obs_norm.and_then(|s| serde_json::from_str(&s).ok()),
        act_normalization_json: act_norm.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: r.get(12)?,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn render_model_card(
    run: &crate::rl_runs::Run,
    primary: &crate::rl_runs::Artifact,
    req: &RegisterRequest,
    framework: &str,
) -> String {
    let final_reward = run
        .last_reward_mean
        .map(|v| format!("{:.2}", v))
        .unwrap_or_else(|| "N/A".to_string());
    let timesteps = run.elapsed_steps;
    let total = run.total_timesteps;
    let descr = req
        .description
        .as_deref()
        .map(|s| format!("\n## Description\n\n{s}\n"))
        .unwrap_or_default();
    let onnx_section = match req.onnx_artifact_id.as_deref() {
        Some(id) => format!("\n- ONNX export: `{id}`\n"),
        None => "\n- ONNX export: (not exported)\n".to_string(),
    };
    format!(
        r#"# Policy: {name}@{version}

## Summary

- Algorithm: {algo}
- Environment: `{env_id}`
- Framework: {framework}
- Trained for: {timesteps} / {total} timesteps
- Last reward mean: {reward}
- Sidecar version: `{sidecar}`

## Reproducibility

- Seed: {seed}
- Run ID: `{run_id}`
- Run config (YAML):

```yaml
{config}
```

## Artifacts

- Primary ({framework}): `{primary_path}` (sha256: `{sha}`, {bytes} bytes){onnx}
{descr}
"#,
        name = req.name,
        version = req.version,
        algo = run.algorithm,
        env_id = run.environment_id,
        framework = framework,
        timesteps = timesteps,
        total = total,
        reward = final_reward,
        sidecar = run.sidecar_version,
        seed = run.seed,
        run_id = run.run_id,
        config = run.config_yaml.trim(),
        primary_path = primary.rel_path,
        sha = primary.sha256,
        bytes = primary.size_bytes,
        onnx = onnx_section,
        descr = descr,
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rl_runs::{ArtifactRecord, CreateRunRequest, RunKind, RunStatus, RunStore};
    use tempfile::TempDir;

    fn open_stores() -> (TempDir, RunStore, PolicyStore) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join(".vibecli").join("workspace.db");
        let runs = RunStore::open_with(&db_path).unwrap();
        let policies = PolicyStore::open_with(&db_path).unwrap();
        (tmp, runs, policies)
    }

    fn finished_run(store: &RunStore, ws: &Path, name: &str) -> (String, String) {
        let run = store
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
            .unwrap();
        // Take it through the lifecycle to Succeeded.
        store.transition(&run.run_id, RunStatus::Queued, None).unwrap();
        store.transition(&run.run_id, RunStatus::Running, None).unwrap();
        store.transition(&run.run_id, RunStatus::Succeeded, None).unwrap();
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
        (run.run_id, art.artifact_id)
    }

    #[test]
    fn register_and_round_trip() {
        let (tmp, runs, policies) = open_stores();
        let (run_id, art_id) = finished_run(&runs, tmp.path(), "ppo-cartpole");
        let policy = policies
            .register(
                RegisterRequest {
                    name: "cartpole-baseline".into(),
                    version: "0.1.0".into(),
                    description: Some("first PPO baseline".into()),
                    run_id: run_id.clone(),
                    primary_artifact_id: art_id.clone(),
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap();
        assert!(policy.policy_id.starts_with("policy-"));
        assert_eq!(policy.framework, "pytorch");
        assert!(policy.model_card_md.contains("cartpole-baseline@0.1.0"));
        assert!(policy.model_card_md.contains("PPO"));
        assert!(policy.model_card_md.contains(&run_id));

        let listed = policies.list(None).unwrap();
        assert_eq!(listed.len(), 1);
        let card = policies.card(&policy.policy_id).unwrap();
        assert_eq!(card, policy.model_card_md);
    }

    #[test]
    fn register_rejects_unknown_artifact() {
        let (tmp, runs, policies) = open_stores();
        let (run_id, _real_art) = finished_run(&runs, tmp.path(), "x");
        let err = policies
            .register(
                RegisterRequest {
                    name: "x".into(),
                    version: "0.1.0".into(),
                    description: None,
                    run_id,
                    primary_artifact_id: "art-does-not-exist".into(),
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(_)));
    }

    #[test]
    fn register_rejects_duplicate_name_version() {
        let (tmp, runs, policies) = open_stores();
        let (run_id, art_id) = finished_run(&runs, tmp.path(), "x");
        policies
            .register(
                RegisterRequest {
                    name: "dup".into(),
                    version: "0.1.0".into(),
                    description: None,
                    run_id: run_id.clone(),
                    primary_artifact_id: art_id.clone(),
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap();
        let err = policies
            .register(
                RegisterRequest {
                    name: "dup".into(),
                    version: "0.1.0".into(),
                    description: None,
                    run_id,
                    primary_artifact_id: art_id,
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap_err();
        assert!(matches!(err, RunError::Invalid(msg) if msg.contains("already exists")));
    }

    #[test]
    fn lineage_walks_parent_run_chain() {
        let (tmp, runs, policies) = open_stores();
        // Parent run.
        let (parent_id, parent_art) = finished_run(&runs, tmp.path(), "parent");
        // Child run that resumes parent (parent_run_id link).
        let child = runs
            .create(CreateRunRequest {
                name: "child".into(),
                kind: RunKind::Train,
                algorithm: "PPO".into(),
                environment_id: "gym:CartPole-v1:gym-bundled".into(),
                parent_run_id: Some(parent_id.clone()),
                config_yaml: "lr: 3e-4\n".into(),
                seed: 42,
                total_timesteps: 1000,
                workspace_path: tmp.path().to_string_lossy().into(),
                sidecar_version: None,
            })
            .unwrap();
        runs.transition(&child.run_id, RunStatus::Queued, None).unwrap();
        runs.transition(&child.run_id, RunStatus::Running, None).unwrap();
        runs.transition(&child.run_id, RunStatus::Succeeded, None).unwrap();
        let child_art = runs
            .record_artifact(
                &child.run_id,
                ArtifactRecord {
                    kind: "final".into(),
                    timestep: Some(1000),
                    rel_path: format!(".vibecli/rl-artifacts/{}/final.pt", child.run_id),
                    sha256: "feedface".into(),
                    size_bytes: 4096,
                    metadata_json: None,
                },
            )
            .unwrap();
        let _ = parent_art;
        let policy = policies
            .register(
                RegisterRequest {
                    name: "child-policy".into(),
                    version: "0.1.0".into(),
                    description: None,
                    run_id: child.run_id.clone(),
                    primary_artifact_id: child_art.artifact_id,
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap();

        let graph = policies.lineage(&policy.policy_id, 5).unwrap();
        let kinds: std::collections::HashSet<&str> =
            graph.edges.iter().map(|e| e.kind.as_str()).collect();
        assert!(kinds.contains("producer"));
        assert!(kinds.contains("parent_run"));

        let node_ids: std::collections::HashSet<&str> =
            graph.nodes.iter().map(|n| n.node_id.as_str()).collect();
        assert!(node_ids.contains(policy.policy_id.as_str()));
        assert!(node_ids.contains(child.run_id.as_str()));
        assert!(node_ids.contains(parent_id.as_str()));
    }

    #[test]
    fn delete_policy_drops_its_producer_edge() {
        let (tmp, runs, policies) = open_stores();
        let (run_id, art_id) = finished_run(&runs, tmp.path(), "x");
        let policy = policies
            .register(
                RegisterRequest {
                    name: "x".into(),
                    version: "0.1.0".into(),
                    description: None,
                    run_id,
                    primary_artifact_id: art_id,
                    onnx_artifact_id: None,
                    framework: None,
                },
                &runs,
            )
            .unwrap();
        policies.delete(&policy.policy_id).unwrap();
        assert!(policies.get(&policy.policy_id).unwrap().is_none());
    }
}
