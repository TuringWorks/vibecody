//! Slice 2 — RL training executor.
//!
//! Spawns the `vibe-rl-py` sidecar (`python -m vibe_rl train …`) as a managed
//! child process, reads JSON-Lines from its stdout, persists ticks /
//! episodes / artifacts via `RunStore`, and drives the run-lifecycle
//! state machine.
//!
//! See `docs/design/rl-os/02-training-executor.md` for the protocol +
//! lifecycle. The protocol is the same shape the sidecar emits today
//! (one JSON object per line, dispatched by the `t` field):
//!     {"t":"started",  "device":"cpu",  "seed":42, ...}
//!     {"t":"tick",     "tick":1, "timestep":2048, "payload": {...}}
//!     {"t":"episode",  "idx":17,"timestep":2100,"reward":195.0, ...}
//!     {"t":"checkpoint","timestep":50000,"rel_path":".vibecli/.../ckpt-50k.pt", ...}
//!     {"t":"gpu",      "util":[78.0],"mem_mb":[18432]}
//!     {"t":"finished", "reason":"done","final_reward_mean":487.3}
//!     {"t":"finished", "reason":"error","error":"..."}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::rl_runs::{
    ArtifactRecord, EpisodeRow, MetricTick, RunError, RunStatus, RunStore,
};

// ── Configuration ─────────────────────────────────────────────────────────────

/// How the daemon locates the sidecar Python interpreter and module root.
///
/// Production deploys ship a vendored interpreter alongside the daemon
/// binary and a `vibe-rl-py` venv materialized at first launch. For
/// development the daemon falls back to the host `python3` and the
/// `vibe-rl-py/` directory in the repo root. Both modes go through the
/// same `Command::spawn` path.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Absolute path to the Python interpreter to launch the sidecar with.
    pub interpreter: PathBuf,
    /// Absolute path to the directory containing the `vibe_rl/` package
    /// (i.e. the root of `vibe-rl-py/`). Added to PYTHONPATH.
    pub sidecar_root: PathBuf,
    /// Per-run timeout. None = unbounded; the user stops via the dashboard.
    pub run_timeout: Option<Duration>,
    /// How many ticks/episodes to batch before each persistence flush.
    pub batch_flush_size: usize,
    /// Maximum interval before forcing a flush even when the batch is
    /// short of `batch_flush_size`.
    pub batch_flush_interval: Duration,
}

impl ExecutorConfig {
    /// Best-effort discovery for development: assume the daemon is run
    /// from the repo root, fall back to `python3`. Production startup
    /// will replace these via explicit configuration.
    pub fn from_env() -> Self {
        // `python3` is resolved by `Command::spawn` against PATH; no
        // explicit which-resolution needed. Override via VIBE_RL_PYTHON
        // for venv interpreters.
        let interpreter = std::env::var_os("VIBE_RL_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("python3"));
        let sidecar_root = std::env::var_os("VIBE_RL_SIDECAR_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // Walk up from CWD until we find a `vibe-rl-py/` sibling.
                let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                for _ in 0..6 {
                    let candidate = p.join("vibe-rl-py");
                    if candidate.is_dir() {
                        return candidate;
                    }
                    if !p.pop() {
                        break;
                    }
                }
                PathBuf::from("vibe-rl-py")
            });
        Self {
            interpreter,
            sidecar_root,
            run_timeout: None,
            batch_flush_size: 50,
            batch_flush_interval: Duration::from_millis(250),
        }
    }
}

// ── Trait ────────────────────────────────────────────────────────────────────

/// The executor abstraction the HTTP layer talks to. Slice 2 ships
/// `PythonExecutor`; slice 7d adds a native-Rust impl that swaps in for
/// inference-only loads, and the long-arc Phase C replaces this for hot
/// training loops too.
#[async_trait]
pub trait TrainingExecutor: Send + Sync {
    /// Start a previously-created run. The run row must exist in
    /// `RunStore` and be in `Created` status. Returns immediately after
    /// the child process has been spawned (and transitioned the row to
    /// `Queued` → `Running`); the actual training runs in a background
    /// task that owns the metric stream.
    async fn start(&self, run_id: &str) -> Result<(), RunError>;

    /// Request a graceful stop. Sidecar gets SIGTERM; the run finishes
    /// the current update, writes a final checkpoint, transitions to
    /// `Stopped`. Hard timeout falls back to SIGKILL after `kill_after`.
    async fn stop(&self, run_id: &str) -> Result<(), RunError>;

    /// Hard cancel — SIGKILL immediately, transition to `Cancelled`.
    /// No final checkpoint is written.
    async fn cancel(&self, run_id: &str) -> Result<(), RunError>;
}

// ── Python sidecar implementation ────────────────────────────────────────────

pub struct PythonExecutor {
    cfg: ExecutorConfig,
    store: Arc<RunStore>,
    /// Live `Child` handles keyed by run_id. Held under a Mutex so
    /// `stop`/`cancel` from the HTTP path can find them. Stale entries
    /// are removed by the background reader task on process exit.
    children: Arc<Mutex<HashMap<String, Arc<Mutex<Child>>>>>,
}

impl PythonExecutor {
    pub fn new(cfg: ExecutorConfig, store: Arc<RunStore>) -> Self {
        Self {
            cfg,
            store,
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Sweep any rows left in `Running` from a prior daemon process into
    /// `Failed`. Mirrors `JobManager`'s startup behaviour and is the
    /// honest answer to "the daemon crashed mid-run" — slice 2 does not
    /// implement auto-resume from checkpoint; the user re-creates the run
    /// (and slice 5's lineage will eventually link the resumes together).
    pub async fn recover_stale_runs(&self) -> Result<(), RunError> {
        use crate::rl_runs::RunFilter;
        let stale = self.store.list(RunFilter {
            status: Some(RunStatus::Running),
            ..RunFilter::default()
        })?;
        for run in stale {
            // Best-effort transition; ignore "illegal" failures (the row
            // may have raced into another terminal state already).
            let _ = self.store.transition(
                &run.run_id,
                RunStatus::Failed,
                Some("daemon restart while running — re-create the run to retry".into()),
            );
        }
        let stopping = self.store.list(RunFilter {
            status: Some(RunStatus::Stopping),
            ..RunFilter::default()
        })?;
        for run in stopping {
            let _ = self
                .store
                .transition(&run.run_id, RunStatus::Stopped, None);
        }
        Ok(())
    }

    /// Build the YAML config the sidecar consumes. The sidecar parses
    /// the same `config_yaml` blob the daemon already has on the run
    /// row (the dashboard's `TrainRunConfig` serialized verbatim) plus
    /// a few daemon-supplied overrides.
    fn write_config_yaml(
        &self,
        run: &crate::rl_runs::Run,
    ) -> Result<PathBuf, std::io::Error> {
        // Parse the run's existing config_yaml so we can patch overrides
        // in (workspace_path, artifact_dir, total_timesteps when they
        // got changed at create time, …).
        let mut doc: serde_yaml::Value = serde_yaml::from_str(&run.config_yaml)
            .unwrap_or(serde_yaml::Value::Mapping(Default::default()));
        if let serde_yaml::Value::Mapping(ref mut m) = doc {
            m.insert(
                serde_yaml::Value::String("environment_id".into()),
                serde_yaml::Value::String(run.environment_id.clone()),
            );
            m.insert(
                serde_yaml::Value::String("workspace_path".into()),
                serde_yaml::Value::String(run.workspace_path.clone()),
            );
            m.insert(
                serde_yaml::Value::String("total_timesteps".into()),
                serde_yaml::Value::Number((run.total_timesteps as i64).into()),
            );
            m.insert(
                serde_yaml::Value::String("seed".into()),
                serde_yaml::Value::Number(run.seed.into()),
            );
            let artifact_dir = PathBuf::from(&run.workspace_path)
                .join(".vibecli")
                .join("rl-artifacts")
                .join(&run.run_id);
            m.insert(
                serde_yaml::Value::String("artifact_dir".into()),
                serde_yaml::Value::String(artifact_dir.to_string_lossy().to_string()),
            );
            m.insert(
                serde_yaml::Value::String("algorithm".into()),
                serde_yaml::Value::String(run.algorithm.clone()),
            );
            // Slice 7a — sidecar dispatches by run kind. We always write
            // it explicitly so the sidecar's `train` subcommand can route
            // distill / quantize / prune / rlhf into the right algorithm
            // module without re-deriving it from the algorithm field.
            m.insert(
                serde_yaml::Value::String("kind".into()),
                serde_yaml::Value::String(run.kind.as_str().to_string()),
            );
            // Surface the parent_run_id (used by distill to find the
            // teacher's checkpoint).
            if let Some(parent) = run.parent_run_id.as_ref() {
                m.insert(
                    serde_yaml::Value::String("parent_run_id".into()),
                    serde_yaml::Value::String(parent.clone()),
                );
            }
        }
        let cfg_dir = PathBuf::from(&run.workspace_path)
            .join(".vibecli")
            .join("rl-runs");
        std::fs::create_dir_all(&cfg_dir)?;
        let cfg_path = cfg_dir.join(format!("{}.yaml", run.run_id));
        std::fs::write(&cfg_path, serde_yaml::to_string(&doc).unwrap_or_default())?;
        Ok(cfg_path)
    }

    fn build_command(&self, run_id: &str, config_path: &Path) -> Command {
        let mut cmd = Command::new(&self.cfg.interpreter);
        cmd.arg("-m")
            .arg("vibe_rl")
            .arg("train")
            .arg("--run-id")
            .arg(run_id)
            .arg("--config")
            .arg(config_path);
        cmd.env("PYTHONPATH", &self.cfg.sidecar_root);
        // Force unbuffered stdio so JSON-Lines arrive promptly.
        cmd.env("PYTHONUNBUFFERED", "1");
        // Disable matplotlib backends et al. that some envs hit.
        cmd.env("MPLBACKEND", "Agg");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(false); // We manage shutdown explicitly.
        cmd
    }
}

#[async_trait]
impl TrainingExecutor for PythonExecutor {
    async fn start(&self, run_id: &str) -> Result<(), RunError> {
        let run = self
            .store
            .get(run_id)?
            .ok_or_else(|| RunError::NotFound(run_id.to_string()))?;
        if run.status != RunStatus::Created {
            return Err(RunError::IllegalTransition {
                from: run.status.as_str(),
                to: "queued",
            });
        }

        let config_path = self
            .write_config_yaml(&run)
            .map_err(|e| RunError::Storage(format!("write run config: {e}")))?;

        // Created → Queued. We promote to Running once the child emits
        // its `started` heartbeat, so a failure to spawn doesn't leave
        // the row claiming Running.
        self.store.transition(run_id, RunStatus::Queued, None)?;

        let mut cmd = self.build_command(run_id, &config_path);
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.store.transition(
                    run_id,
                    RunStatus::Failed,
                    Some(format!(
                        "could not spawn vibe-rl-py sidecar: {e}. Hint: install deps with \
                         `cd vibe-rl-py && uv sync`, or set VIBE_RL_PYTHON to a venv interpreter."
                    )),
                );
                return Err(RunError::Storage(e.to_string()));
            }
        };

        let child_arc = Arc::new(Mutex::new(child));
        self.children
            .lock()
            .await
            .insert(run_id.to_string(), child_arc.clone());

        // Spawn the reader. It owns the child handle for stdout reading +
        // exit-status drain; `stop`/`cancel` reach in via the children map
        // to send signals.
        let store = self.store.clone();
        let children = self.children.clone();
        let run_id_owned = run_id.to_string();
        let cfg = self.cfg.clone();
        tokio::spawn(async move {
            let _ = run_reader_loop(child_arc, store.clone(), &run_id_owned, &cfg).await;
            // Drop the entry regardless of how the loop exited.
            children.lock().await.remove(&run_id_owned);
        });

        Ok(())
    }

    async fn stop(&self, run_id: &str) -> Result<(), RunError> {
        // Move the row into Stopping first so the reader knows what
        // terminal state to land on if the child exits cleanly.
        self.store
            .transition(run_id, RunStatus::Stopping, None)?;

        let child_arc = {
            let map = self.children.lock().await;
            map.get(run_id).cloned()
        };
        if let Some(child) = child_arc {
            send_term(&child).await;
        } else {
            // No live child — likely already terminal. Ensure the row
            // doesn't stay in Stopping forever.
            let _ = self.store.transition(run_id, RunStatus::Stopped, None);
        }
        Ok(())
    }

    async fn cancel(&self, run_id: &str) -> Result<(), RunError> {
        let child_arc = {
            let map = self.children.lock().await;
            map.get(run_id).cloned()
        };
        if let Some(child) = child_arc {
            let mut c = child.lock().await;
            let _ = c.start_kill();
        }
        // Allow Created → Cancelled OR (via Stopping) Running → Cancelled
        // depending on where the row currently is. We do a best-effort
        // direct transition; if it's illegal, surface the error.
        self.store
            .transition(run_id, RunStatus::Cancelled, None)?;
        Ok(())
    }
}

// ── Reader loop ──────────────────────────────────────────────────────────────

async fn run_reader_loop(
    child_arc: Arc<Mutex<Child>>,
    store: Arc<RunStore>,
    run_id: &str,
    cfg: &ExecutorConfig,
) -> Result<(), RunError> {
    // Take stdout once; the rest of the reader doesn't need to hold the
    // child mutex (which `stop`/`cancel` need).
    let stdout = {
        let mut c = child_arc.lock().await;
        c.stdout.take()
    };
    let stdout = match stdout {
        Some(s) => s,
        None => {
            return Err(RunError::Storage("child stdout missing".into()));
        }
    };

    let mut reader = BufReader::new(stdout).lines();
    let mut tick_batch: Vec<MetricTick> = Vec::with_capacity(cfg.batch_flush_size);
    let mut episode_batch: Vec<EpisodeRow> = Vec::with_capacity(cfg.batch_flush_size);
    let mut started = false;
    let mut finished_reason: Option<String> = None;
    let mut finished_error: Option<String> = None;

    let flush_interval = cfg.batch_flush_interval;
    let mut flush_at = tokio::time::Instant::now() + flush_interval;

    loop {
        tokio::select! {
            line = reader.next_line() => match line {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    handle_line(
                        &line,
                        run_id,
                        &store,
                        &mut tick_batch,
                        &mut episode_batch,
                        &mut started,
                        &mut finished_reason,
                        &mut finished_error,
                    );
                    if tick_batch.len() >= cfg.batch_flush_size {
                        let _ = store.append_metrics(run_id, &tick_batch);
                        tick_batch.clear();
                    }
                    if episode_batch.len() >= cfg.batch_flush_size {
                        let _ = store.append_episodes(run_id, &episode_batch);
                        episode_batch.clear();
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    finished_reason.get_or_insert("error".into());
                    finished_error.get_or_insert_with(|| format!("stdout read error: {e}"));
                    break;
                }
            },
            _ = tokio::time::sleep_until(flush_at) => {
                if !tick_batch.is_empty() {
                    let _ = store.append_metrics(run_id, &tick_batch);
                    tick_batch.clear();
                }
                if !episode_batch.is_empty() {
                    let _ = store.append_episodes(run_id, &episode_batch);
                    episode_batch.clear();
                }
                flush_at = tokio::time::Instant::now() + flush_interval;
            }
        }
    }

    // Final flush.
    if !tick_batch.is_empty() {
        let _ = store.append_metrics(run_id, &tick_batch);
    }
    if !episode_batch.is_empty() {
        let _ = store.append_episodes(run_id, &episode_batch);
    }

    // Wait for child to actually exit so we collect status.
    let exit_status = {
        let mut c = child_arc.lock().await;
        c.wait().await.ok()
    };

    let target = match (finished_reason.as_deref(), exit_status) {
        (Some("done"), _) => RunStatus::Succeeded,
        (Some("cancelled"), _) => RunStatus::Cancelled,
        (Some("error"), _) => RunStatus::Failed,
        (None, Some(es)) if es.success() => {
            // Sidecar exited 0 without a finished line — treat as success.
            RunStatus::Succeeded
        }
        _ => RunStatus::Failed,
    };

    let err_msg = if target == RunStatus::Failed {
        Some(
            finished_error
                .clone()
                .unwrap_or_else(|| "sidecar exited without a 'done' marker".into()),
        )
    } else {
        None
    };

    // If we never saw `started`, the row is still Queued. Push it to
    // Running first so the failure transition is legal.
    if !started {
        let _ = store.transition(run_id, RunStatus::Running, None);
    }
    let _ = store.transition(run_id, target, err_msg);
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum SidecarLine {
    Started {
        #[serde(default)]
        device: Option<String>,
        #[serde(default)]
        seed: Option<i64>,
        #[serde(default)]
        sidecar_version: Option<String>,
        #[serde(default)]
        wall: Option<i64>,
    },
    Tick {
        tick: i64,
        timestep: i64,
        #[serde(default)]
        wall: Option<i64>,
        payload: serde_json::Value,
    },
    Episode {
        idx: i64,
        timestep: i64,
        reward: f64,
        length: i64,
        #[serde(default)]
        success: Option<bool>,
        duration_ms: i64,
    },
    Checkpoint {
        timestep: i64,
        rel_path: String,
        sha256: String,
        size_bytes: i64,
    },
    Gpu {
        #[serde(default)]
        util: Vec<f64>,
        #[serde(default)]
        mem_mb: Vec<i64>,
    },
    Finished {
        reason: String,
        #[serde(default)]
        final_reward_mean: Option<f64>,
        #[serde(default)]
        error: Option<String>,
    },
}

#[allow(clippy::too_many_arguments)]
fn handle_line(
    line: &str,
    run_id: &str,
    store: &Arc<RunStore>,
    tick_batch: &mut Vec<MetricTick>,
    episode_batch: &mut Vec<EpisodeRow>,
    started: &mut bool,
    finished_reason: &mut Option<String>,
    finished_error: &mut Option<String>,
) {
    let parsed: SidecarLine = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            // Non-JSON lines (Python tracebacks, library prints) are
            // captured by the daemon's stdout drain elsewhere and tee'd
            // to the run log. Here we just ignore.
            return;
        }
    };

    match parsed {
        SidecarLine::Started { .. } => {
            *started = true;
            // Promote Queued → Running on first heartbeat.
            let _ = store.transition(run_id, RunStatus::Running, None);
        }
        SidecarLine::Tick {
            tick,
            timestep,
            wall,
            payload,
        } => {
            tick_batch.push(MetricTick {
                tick,
                timestep,
                wall_time: wall.unwrap_or_else(now_ms),
                payload,
            });
        }
        SidecarLine::Episode {
            idx,
            timestep,
            reward,
            length,
            success,
            duration_ms,
        } => {
            episode_batch.push(EpisodeRow {
                episode_idx: idx,
                timestep,
                reward_sum: reward,
                length,
                success,
                duration_ms,
            });
        }
        SidecarLine::Checkpoint {
            timestep,
            rel_path,
            sha256,
            size_bytes,
        } => {
            // Checkpoint paths arrive workspace-relative; the
            // RunStore::record_artifact validator keeps them inside the
            // workspace tree.
            let _ = store.record_artifact(
                run_id,
                ArtifactRecord {
                    kind: "checkpoint".to_string(),
                    timestep: Some(timestep),
                    rel_path,
                    sha256,
                    size_bytes,
                    metadata_json: None,
                },
            );
        }
        SidecarLine::Gpu { util, mem_mb } => {
            // GPU snapshots are surfaced as a synthetic tick so the
            // dashboard's GPU card has a recent value to read. We don't
            // bother with a separate table for slice 2.
            tick_batch.push(MetricTick {
                tick: i64::MAX,
                timestep: 0,
                wall_time: now_ms(),
                payload: serde_json::json!({"gpu_util": util, "gpu_mem_mb": mem_mb}),
            });
        }
        SidecarLine::Finished {
            reason,
            error,
            ..
        } => {
            *finished_reason = Some(reason);
            *finished_error = error;
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── Signal helpers (Unix-flavored; Windows uses kill on stop too) ────────────

#[cfg(unix)]
async fn send_term(child: &Arc<Mutex<Child>>) {
    let pid_opt = {
        let c = child.lock().await;
        c.id().map(|p| p as i32)
    };
    if let Some(pid) = pid_opt {
        // SAFETY: `kill` with SIGTERM is safe to call against an existing
        // PID; no memory is touched. ESRCH (process already gone) is a
        // tolerable no-op.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
}

#[cfg(not(unix))]
async fn send_term(child: &Arc<Mutex<Child>>) {
    // Windows: no SIGTERM. Best we can do without an extra crate is a
    // hard kill; the sidecar's atexit hook still gets a chance to flush
    // a final-checkpoint line via Python's interpreter shutdown.
    let mut c = child.lock().await;
    let _ = c.start_kill();
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rl_runs::{CreateRunRequest, RunKind};
    use tempfile::TempDir;

    fn fake_run(store: &RunStore, ws: &Path) -> crate::rl_runs::Run {
        store
            .create(CreateRunRequest {
                name: "fake".into(),
                kind: RunKind::Train,
                algorithm: "PPO".into(),
                environment_id: "gym:CartPole-v1:gym-0.29".into(),
                parent_run_id: None,
                config_yaml: "learning_rate: 0.0003\n".into(),
                seed: 7,
                total_timesteps: 1_000,
                workspace_path: ws.to_string_lossy().into(),
                sidecar_version: None,
            })
            .unwrap()
    }

    #[test]
    fn handle_line_dispatches_each_event_kind() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join(".vibecli").join("workspace.db");
        let store = Arc::new(RunStore::open_with(&db).unwrap());
        let run = fake_run(&store, tmp.path());
        // Move to Queued so `started` can promote to Running.
        store
            .transition(&run.run_id, RunStatus::Queued, None)
            .unwrap();

        let mut ticks = Vec::new();
        let mut eps = Vec::new();
        let mut started = false;
        let mut reason = None;
        let mut err = None;

        for line in [
            r#"{"t":"started","seed":7,"device":"cpu","sidecar_version":"0.1.0"}"#,
            r#"{"t":"tick","tick":1,"timestep":2048,"payload":{"policy_loss":0.1}}"#,
            r#"{"t":"episode","idx":1,"timestep":200,"reward":150.0,"length":200,"success":true,"duration_ms":500}"#,
            r#"{"t":"checkpoint","timestep":50000,"rel_path":".vibecli/rl-artifacts/run-x/ckpt-50k.pt","sha256":"abc","size_bytes":2048}"#,
            r#"{"t":"finished","reason":"done","final_reward_mean":487.3}"#,
            r#"not even json"#,
        ] {
            handle_line(
                line,
                &run.run_id,
                &store,
                &mut ticks,
                &mut eps,
                &mut started,
                &mut reason,
                &mut err,
            );
        }

        assert!(started, "started flag should flip on the started line");
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].tick, 1);
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].reward_sum, 150.0);
        assert_eq!(reason.as_deref(), Some("done"));
        // Started promoted Queued → Running:
        let updated = store.get(&run.run_id).unwrap().unwrap();
        assert_eq!(updated.status, RunStatus::Running);

        // Checkpoint registered as an artifact:
        let arts = store.list_artifacts(&run.run_id).unwrap();
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].kind, "checkpoint");
        assert_eq!(arts[0].timestep, Some(50000));
    }

    #[test]
    fn recover_stale_runs_marks_running_as_failed() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join(".vibecli").join("workspace.db");
        let store = Arc::new(RunStore::open_with(&db).unwrap());
        let run = fake_run(&store, tmp.path());
        store
            .transition(&run.run_id, RunStatus::Queued, None)
            .unwrap();
        store
            .transition(&run.run_id, RunStatus::Running, None)
            .unwrap();

        let exec = PythonExecutor::new(
            ExecutorConfig {
                interpreter: "python3".into(),
                sidecar_root: "vibe-rl-py".into(),
                run_timeout: None,
                batch_flush_size: 10,
                batch_flush_interval: Duration::from_millis(100),
            },
            store.clone(),
        );
        // recover_stale_runs is async — block on it via a single-thread
        // tokio runtime.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(exec.recover_stale_runs()).unwrap();

        let updated = store.get(&run.run_id).unwrap().unwrap();
        assert_eq!(updated.status, RunStatus::Failed);
        assert!(updated
            .error_message
            .unwrap()
            .contains("daemon restart"));
    }
}
