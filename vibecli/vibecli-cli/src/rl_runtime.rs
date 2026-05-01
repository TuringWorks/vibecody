//! Slice 6.5 — PolicyRuntime trait + PythonRuntime impl.
//!
//! Owns the long-lived `python -m vibe_rl inference` subprocess that
//! holds a deployed Policy in memory. The daemon's `/v1/rl/serve/:name/act`
//! handler routes obs → runtime → action through this trait.
//!
//! Wire format (one JSON object per line on each pipe):
//!
//!   stdin  ←  {"obs": [<floats>]}
//!   stdout →  {"t":"ready", ...}                    (once, at startup)
//!   stdout →  {"action": <int | [<scalars>]>}
//!   stdout →  {"error": "<msg>"}
//!
//! Slice 6.5 ships a single backend (`PythonRuntime`). Slice 7d adds
//! `BurnRuntime` / `CubeclRuntime` impls of the same trait for
//! native-Rust serving — no daemon-side code change required.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use crate::rl_runs::RunError;

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActRequest {
    pub obs: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActResponse {
    pub action: serde_json::Value,
    pub deployment: String,
    pub policy_id: Option<String>,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeHealth {
    pub deployment_id: String,
    pub framework: String,
    pub action_kind: String,
    pub device: String,
    pub checkpoint: String,
    pub uptime_seconds: u64,
    pub requests_total: u64,
    pub error_total: u64,
    pub last_latency_ms: Option<f64>,
}

// ── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait PolicyRuntime: Send + Sync {
    async fn act(&self, obs: serde_json::Value) -> Result<serde_json::Value, RunError>;
    async fn health(&self) -> RuntimeHealth;
    async fn shutdown(&self) -> Result<(), RunError>;
}

// ── PythonRuntime ────────────────────────────────────────────────────────────

struct PythonRuntimeInner {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    framework: String,
    action_kind: String,
    device: String,
    checkpoint: String,
}

pub struct PythonRuntime {
    deployment_id: String,
    started_at: Instant,
    inner: Mutex<PythonRuntimeInner>,
    requests_total: std::sync::atomic::AtomicU64,
    error_total: std::sync::atomic::AtomicU64,
    last_latency_ms: Mutex<Option<f64>>,
}

impl PythonRuntime {
    /// Spawn `python -m vibe_rl <inference|onnx-inference>` and read
    /// the `ready` heartbeat. The runtime kind selects:
    ///
    /// - `"python"` → `inference --checkpoint <path>` (PyTorch checkpoint)
    /// - `"onnx"`   → `onnx-inference --model <path>` (ONNX file from
    ///                slice 7a's quantize, or any FP32 .onnx export)
    ///
    /// Returns once the sidecar emits its `{"t":"ready", ...}` line.
    pub async fn spawn(
        cfg: &crate::rl_executor::ExecutorConfig,
        deployment_id: String,
        checkpoint_path: PathBuf,
        runtime_kind: &str,
    ) -> Result<Self, RunError> {
        let (subcommand, flag) = match runtime_kind {
            "python" => ("inference", "--checkpoint"),
            "onnx" => ("onnx-inference", "--model"),
            other => {
                return Err(RunError::Invalid(format!(
                    "PythonRuntime::spawn doesn't handle runtime '{other}' — supports python | onnx"
                )))
            }
        };

        let mut cmd = Command::new(&cfg.interpreter);
        cmd.arg("-m")
            .arg("vibe_rl")
            .arg(subcommand)
            .arg(flag)
            .arg(&checkpoint_path)
            .env("PYTHONPATH", &cfg.sidecar_root)
            .env("PYTHONUNBUFFERED", "1")
            .env("MPLBACKEND", "Agg")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| RunError::Storage(format!("spawn vibe-rl {subcommand}: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RunError::Storage("inference child stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RunError::Storage("inference child stdout missing".into()))?;
        let mut reader = BufReader::new(stdout);

        let mut ready_line = String::new();
        match timeout(Duration::from_secs(60), reader.read_line(&mut ready_line)).await {
            Ok(Ok(0)) => {
                return Err(RunError::Storage(
                    "inference sidecar exited before emitting ready".into(),
                ))
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(RunError::Storage(format!("read ready: {e}"))),
            Err(_) => {
                return Err(RunError::Storage(
                    "inference sidecar did not emit ready within 60s".into(),
                ))
            }
        }

        #[derive(Deserialize)]
        #[serde(tag = "t", rename_all = "snake_case")]
        enum Hello {
            Ready {
                framework: String,
                action_kind: String,
                device: String,
                checkpoint: String,
            },
            Error {
                error: String,
            },
        }
        let hello: Hello = serde_json::from_str(ready_line.trim())
            .map_err(|e| RunError::Storage(format!("parse ready: {e}; got: {ready_line:?}")))?;
        let (framework, action_kind, device, checkpoint) = match hello {
            Hello::Ready { framework, action_kind, device, checkpoint } => {
                (framework, action_kind, device, checkpoint)
            }
            Hello::Error { error } => {
                return Err(RunError::Invalid(format!(
                    "inference sidecar failed to load checkpoint: {error}"
                )))
            }
        };

        Ok(Self {
            deployment_id,
            started_at: Instant::now(),
            inner: Mutex::new(PythonRuntimeInner {
                child,
                stdin,
                stdout: reader,
                framework,
                action_kind,
                device,
                checkpoint,
            }),
            requests_total: 0.into(),
            error_total: 0.into(),
            last_latency_ms: Mutex::new(None),
        })
    }
}

#[async_trait]
impl PolicyRuntime for PythonRuntime {
    async fn act(&self, obs: serde_json::Value) -> Result<serde_json::Value, RunError> {
        let req = serde_json::json!({"obs": obs});
        let line = serde_json::to_string(&req)
            .map_err(|e| RunError::Storage(format!("serialize act: {e}")))?;

        let started = Instant::now();
        let mut inner = self.inner.lock().await;
        // Write request.
        inner
            .stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| RunError::Storage(format!("write act: {e}")))?;
        inner
            .stdin
            .write_all(b"\n")
            .await
            .map_err(|e| RunError::Storage(format!("write act: {e}")))?;
        inner
            .stdin
            .flush()
            .await
            .map_err(|e| RunError::Storage(format!("flush act: {e}")))?;

        let mut response = String::new();
        match timeout(Duration::from_secs(30), inner.stdout.read_line(&mut response)).await {
            Ok(Ok(0)) => {
                self.error_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(RunError::Storage(
                    "inference sidecar closed stdout".into(),
                ));
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                self.error_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(RunError::Storage(format!("read act: {e}")));
            }
            Err(_) => {
                self.error_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Err(RunError::Storage(
                    "inference sidecar timed out (30s)".into(),
                ));
            }
        }

        self.requests_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        *self.last_latency_ms.lock().await = Some(latency_ms);

        let parsed: serde_json::Value = serde_json::from_str(response.trim())
            .map_err(|e| RunError::Storage(format!("parse act response: {e}")))?;
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            self.error_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(RunError::Invalid(format!("sidecar error: {err}")));
        }
        let action = parsed
            .get("action")
            .cloned()
            .ok_or_else(|| RunError::Storage(format!("act response missing 'action': {response:?}")))?;
        Ok(action)
    }

    async fn health(&self) -> RuntimeHealth {
        let inner = self.inner.lock().await;
        let last_latency = *self.last_latency_ms.lock().await;
        RuntimeHealth {
            deployment_id: self.deployment_id.clone(),
            framework: inner.framework.clone(),
            action_kind: inner.action_kind.clone(),
            device: inner.device.clone(),
            checkpoint: inner.checkpoint.clone(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            requests_total: self
                .requests_total
                .load(std::sync::atomic::Ordering::Relaxed),
            error_total: self
                .error_total
                .load(std::sync::atomic::Ordering::Relaxed),
            last_latency_ms: last_latency,
        }
    }

    async fn shutdown(&self) -> Result<(), RunError> {
        let mut inner = self.inner.lock().await;
        // Closing stdin signals the sidecar's request loop to exit cleanly.
        let _ = inner.stdin.shutdown().await;
        // Give the sidecar a beat to drain, then SIGKILL on Drop.
        match timeout(Duration::from_secs(5), inner.child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                let _ = inner.child.start_kill();
            }
        }
        Ok(())
    }
}

// ── OnnxRustRuntime (slice 7d, feature-gated) ────────────────────────────────
//
// Native Rust ONNX inference via the `ort` crate. When the `rl-ort`
// feature is enabled at build time, the routing logic in
// `RuntimePool::get_or_spawn` directs `runtime: onnx` deployments here
// instead of spawning the Python `onnx_inference` sidecar.
//
// Status: the routing path + struct + trait impl are in place. The
// actual `ort` dep is *not yet declared* in `Cargo.toml` because
// `ort 2.0-rc.10` pins `smallvec = =2.0.0-alpha.10` which collides
// with mistralrs's smallvec ≥ alpha.12 requirement. Resolving wants
// either:
//   (a) a newer `ort` rc that doesn't pin smallvec exactly, or
//   (b) a workspace-level [patch.crates-io] for smallvec that
//       satisfies both constraints, or
//   (c) bump ort to a version with the rc.12 API and adapt the
//       `commit_from_file` / `inputs` field rename.
//
// Until then this struct returns a structured "dep deferred" error so
// the gap is visible and the panel can surface a clear hint. Default
// build (rl-ort off) is unaffected — `runtime: onnx` falls through to
// the Python sidecar which works today.

#[cfg(feature = "rl-ort")]
pub struct OnnxRustRuntime {
    deployment_id: String,
    started_at: Instant,
    model_path: PathBuf,
    action_kind: String,
}

#[cfg(feature = "rl-ort")]
impl OnnxRustRuntime {
    pub async fn load(deployment_id: String, model_path: PathBuf) -> Result<Self, RunError> {
        let metadata_path = model_path.with_extension("json");
        let action_kind = if metadata_path.is_file() {
            let raw = std::fs::read_to_string(&metadata_path)
                .map_err(|e| RunError::Storage(format!("read onnx metadata: {e}")))?;
            serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| {
                    v.get("action_kind")
                        .and_then(|x| x.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "discrete".to_string())
        } else {
            "discrete".to_string()
        };
        Ok(Self {
            deployment_id,
            started_at: Instant::now(),
            model_path,
            action_kind,
        })
    }
}

#[cfg(feature = "rl-ort")]
#[async_trait]
impl PolicyRuntime for OnnxRustRuntime {
    async fn act(&self, _obs: serde_json::Value) -> Result<serde_json::Value, RunError> {
        Err(RunError::Invalid(format!(
            "rl-ort feature compiled in but the `ort` dep itself is deferred — \
             needs a smallvec patch to coexist with mistralrs (see Cargo.toml \
             comment near `rl-ort = []`). Falling back: rebuild without \
             --features rl-ort to use the Python `onnx_inference` sidecar, \
             which serves {model_path} via Python onnxruntime.",
            model_path = self.model_path.display()
        )))
    }

    async fn health(&self) -> RuntimeHealth {
        RuntimeHealth {
            deployment_id: self.deployment_id.clone(),
            framework: "onnx-rust (deferred)".into(),
            action_kind: self.action_kind.clone(),
            device: "n/a".into(),
            checkpoint: self.model_path.display().to_string(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            requests_total: 0,
            error_total: 0,
            last_latency_ms: None,
        }
    }

    async fn shutdown(&self) -> Result<(), RunError> {
        Ok(())
    }
}

// ── RuntimePool (one PythonRuntime per active deployment) ────────────────────

#[derive(Default)]
pub struct RuntimePool {
    runtimes: Mutex<HashMap<String, Arc<dyn PolicyRuntime>>>,
}

impl RuntimePool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a live runtime by deployment id, or create one by spawning
    /// a sidecar against the deployment's primary artifact path. The
    /// `runtime_kind` selects which backend to use:
    ///
    /// - `"python"` → spawn `python -m vibe_rl inference` (PyTorch).
    /// - `"onnx"`   → with `--features rl-ort`, load via `ort` in-process;
    ///                without the feature, falls back to spawning
    ///                `python -m vibe_rl onnx-inference`.
    pub async fn get_or_spawn(
        &self,
        cfg: &crate::rl_executor::ExecutorConfig,
        deployment_id: &str,
        checkpoint_rel_path: &str,
        workspace_root: &std::path::Path,
        runtime_kind: &str,
    ) -> Result<Arc<dyn PolicyRuntime>, RunError> {
        // Fast path: already loaded.
        {
            let map = self.runtimes.lock().await;
            if let Some(rt) = map.get(deployment_id) {
                return Ok(rt.clone());
            }
        }
        // Slow path. Resolve relative path against workspace root.
        let abs = workspace_root.join(checkpoint_rel_path);
        if !abs.is_file() {
            return Err(RunError::Invalid(format!(
                "checkpoint file not found at {abs:?} — deploy from a finished run + final artifact"
            )));
        }

        // Slice 7d: native Rust ONNX path when `rl-ort` is enabled.
        #[cfg(feature = "rl-ort")]
        let arc: Arc<dyn PolicyRuntime> = if runtime_kind == "onnx" {
            let runtime = OnnxRustRuntime::load(deployment_id.to_string(), abs).await?;
            Arc::new(runtime)
        } else {
            let runtime =
                PythonRuntime::spawn(cfg, deployment_id.to_string(), abs, runtime_kind).await?;
            Arc::new(runtime)
        };
        #[cfg(not(feature = "rl-ort"))]
        let arc: Arc<dyn PolicyRuntime> = {
            let runtime =
                PythonRuntime::spawn(cfg, deployment_id.to_string(), abs, runtime_kind).await?;
            Arc::new(runtime)
        };
        let mut map = self.runtimes.lock().await;
        // Re-check in case a concurrent caller beat us to the spawn.
        if let Some(existing) = map.get(deployment_id) {
            // Discard ours; the existing sidecar is good. Best-effort
            // shutdown of the duplicate.
            tokio::spawn({
                let arc = arc.clone();
                async move {
                    let _ = arc.shutdown().await;
                }
            });
            return Ok(existing.clone());
        }
        map.insert(deployment_id.to_string(), arc.clone());
        Ok(arc)
    }

    pub async fn drop_runtime(&self, deployment_id: &str) -> Result<(), RunError> {
        let mut map = self.runtimes.lock().await;
        if let Some(rt) = map.remove(deployment_id) {
            rt.shutdown().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    /// A stand-in runtime for tests that doesn't require Python.
    pub struct FakeRuntime {
        pub deployment_id: String,
        started_at: Instant,
        requests: AtomicU64,
    }

    impl FakeRuntime {
        pub fn new(deployment_id: &str) -> Self {
            Self {
                deployment_id: deployment_id.to_string(),
                started_at: Instant::now(),
                requests: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl PolicyRuntime for FakeRuntime {
        async fn act(&self, obs: serde_json::Value) -> Result<serde_json::Value, RunError> {
            self.requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // Echo the obs's first scalar as the action — enough to test
            // the wire shape without any real model.
            let action = obs
                .as_array()
                .and_then(|arr| arr.first())
                .cloned()
                .unwrap_or(serde_json::Value::Number(0.into()));
            Ok(action)
        }

        async fn health(&self) -> RuntimeHealth {
            RuntimeHealth {
                deployment_id: self.deployment_id.clone(),
                framework: "fake".into(),
                action_kind: "discrete".into(),
                device: "cpu".into(),
                checkpoint: "<fake>".into(),
                uptime_seconds: self.started_at.elapsed().as_secs(),
                requests_total: self.requests.load(std::sync::atomic::Ordering::Relaxed),
                error_total: 0,
                last_latency_ms: None,
            }
        }

        async fn shutdown(&self) -> Result<(), RunError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn fake_runtime_round_trip() {
        let rt = FakeRuntime::new("dep-1");
        let action = rt.act(serde_json::json!([42, 1, 2])).await.unwrap();
        assert_eq!(action, serde_json::json!(42));
        let h = rt.health().await;
        assert_eq!(h.deployment_id, "dep-1");
        assert_eq!(h.requests_total, 1);
    }

    #[tokio::test]
    async fn pool_caches_runtime_by_deployment_id() {
        let pool = RuntimePool::new();
        // Direct insertion path — exercise the cache without spawning Python.
        let rt: Arc<dyn PolicyRuntime> = Arc::new(FakeRuntime::new("dep-1"));
        pool.runtimes
            .lock()
            .await
            .insert("dep-1".to_string(), rt.clone());

        let cfg = crate::rl_executor::ExecutorConfig::from_env();
        // Use a non-existent checkpoint path; the cache hit should
        // short-circuit before the file is checked.
        let got = pool
            .get_or_spawn(&cfg, "dep-1", "nope.pt", std::path::Path::new("/tmp"), "python")
            .await
            .unwrap();
        // Same allocation reached via the cache.
        assert!(Arc::ptr_eq(&got, &rt));

        pool.drop_runtime("dep-1").await.unwrap();
        assert!(pool.runtimes.lock().await.is_empty());
    }
}
