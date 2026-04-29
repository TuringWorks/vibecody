# Slice 6 — Deployment + Serving

**Status:** Draft · 2026-04-29
**Depends on:** [01-persistence.md](./01-persistence.md), [04-evaluation.md](./04-evaluation.md) (gates), [05-model-hub.md](./05-model-hub.md) (deploys a Policy, not a run)
**Unblocks:** slice 7 RLHF (RLHF policies need a serving target for human-feedback collection)
**Disclaimer banner after this slice:** drops "Deployment" from `covers={[...]}`. `RLDeploymentMonitor` shows real serving health, A/B traffic split, and latency.

---

## Goal

Take a registered `Policy` and stand up a real inference endpoint inside the daemon (or as a sibling process) that the user — or other VibeCody surfaces — can hit with observations and get back actions. Track health (latency, throughput, error rate), support A/B traffic splits between policy versions, and auto-rollback on regression.

This is where the **Phase B native path** becomes valuable: serving doesn't need PyTorch; ONNX Runtime or a Candle/Burn loader gets us a single-process, fast, no-GIL inference path with no Python in the hot loop.

## Runtime choice

Three runtime options, registered in `rl_deployments.runtime`:

| Runtime | When to use | What it loads |
|---|---|---|
| `onnx` | **Default** when `Policy.onnx_artifact` is present. Cross-platform, fast, no Python at serve time. Backed by [`ort`](https://crates.io/crates/ort) (Rust ONNX Runtime bindings). | `final.onnx` |
| `python` | Fallback when ONNX export failed or the policy uses a custom module that doesn't trace. Spawns a long-lived sidecar holding the policy in memory. | `final.pt` |
| `native_candle` | Opt-in for distilled / quantized policies (slice 7). Routes through `vibe-infer` style loaders. | Candle-format weights + manifest |

`burn` and `cubecl` paths are valid runtime kinds but **not in slice 6's scope** — they enter when slice 7's quantization/distillation produces a Burn-native artifact. The trait is shaped to accept them:

```rust
// vibecli/vibecli-cli/src/rl_serving.rs
pub trait PolicyRuntime: Send + Sync {
    fn load(&mut self, policy: &Policy, artifact_path: &Path) -> Result<()>;
    fn act(&self, obs: &ObsTensor) -> Result<ActionTensor>;
    fn unload(&mut self) -> Result<()>;
    fn health(&self) -> RuntimeHealth;
}
```

`OnnxRuntime`, `PythonRuntime`, `CandleRuntime` are the slice-6 implementations. `BurnRuntime`, `CubeclRuntime` are slice-7 additions.

## Deployment lifecycle

```text
created ──▶ staging ──promote──▶ canary (X% traffic) ──promote──▶ production
                                       │
                                       └─ regression detected ──▶ rolled_back
```

Promotion requires:

- All `quality_gates` from the policy's most recent eval pass.
- A signed-off "promoter" identity (slice 6 supports a single-user daemon: any local user can promote; multi-user gating is future).

Auto-rollback triggers (configurable per-deployment):

- p99 latency > threshold for N minutes
- Error rate > threshold for N minutes
- Reward proxy (slice 6 doesn't have ground-truth reward at serve time; use a configured "reward proxy" — e.g. the user's downstream loop reports back)

## Serving HTTP surface

A new mounted router under `/v1/rl/serve/`:

| Method | Path | Body | Returns |
|---|---|---|---|
| `POST` | `/v1/rl/serve/deployments` | `{ name, policy_id, runtime, traffic_pct, ... }` | `Deployment` |
| `GET` | `/v1/rl/serve/deployments` | — | `Vec<Deployment>` |
| `POST` | `/v1/rl/serve/deployments/{id}/promote` | `{ to: "canary" \| "production", traffic_pct }` | `Deployment` |
| `POST` | `/v1/rl/serve/deployments/{id}/rollback` | `{ reason }` | `Deployment` |
| `POST` | `/v1/rl/serve/deployments/{id}/stop` | — | `Deployment` |
| `GET` | `/v1/rl/serve/deployments/{id}/health` | — | `HealthSnapshot` (latency p50/p95/p99, throughput, error rate, last N minutes) |
| `POST` | `/v1/rl/serve/{deployment_name}/act` | `{ observation: ... }` | `{ action: ..., policy_id, latency_ms }` |
| `POST` | `/v1/rl/serve/{deployment_name}/feedback` | `{ trajectory_id, reward_proxy, ... }` | `204` |

`/serve/{name}/act` is the hot path. Internally it consults the deployment's traffic-split table (canary X% vs production 100−X%), routes the request to the chosen runtime, records the latency, and emits a structured trace.

## A/B + canary

Canary deployment: `production` deployment exists at 100% traffic, `canary` deployment exists at e.g. 10% traffic. The router picks canary with probability 0.10 per request, deterministically by hash of `request_id` if provided (so a single trajectory stays on one policy).

Auto-promote: configurable rule "if canary p95 latency ≤ production p95 latency × 1.1 AND canary error rate ≤ production error rate over 1h, promote to 100%."

A/B (split, no winner): traffic_pct configurable; both stay live until manual decision. Used for human-feedback collection in slice 7.

## Health metrics

Every `act` call records to an in-memory rolling histogram (`tdigest` for percentiles) and to a SQL ring buffer for the persistent view. `RuntimeHealth` exposes:

```rust
pub struct RuntimeHealth {
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_qps: f64,
    pub error_rate: f64,
    pub uptime_seconds: u64,
    pub memory_mb: u64,
    pub last_n_errors: Vec<ErrorTrace>,
}
```

## Tauri command rewrites

| Command | After slice 6 |
|---|---|
| `rl_list_deployments` (currently mock at `commands.rs:41965`) | `GET /v1/rl/serve/deployments` |
| `rl_get_deployment_health` | `GET /v1/rl/serve/deployments/{id}/health` |
| (plus new) `rl_create_deployment`, `rl_promote_deployment`, `rl_rollback_deployment`, `rl_stop_deployment` | corresponding routes |

## Frontend changes

`RLDeploymentMonitor.tsx`:
- Live health charts (latency p50/p95/p99 over time, throughput, error rate).
- Traffic split gauge with promote/rollback affordances.
- Auto-rollback rule editor.
- Per-deployment runtime badge (`onnx` / `python` / `native_candle`).

The "Register policy" flow from slice 5 gets an optional next step: "Deploy as staging."

## Resource isolation

Per CLAUDE.md sandbox-tiers context, serving runtimes that load arbitrary user models should run inside the egress broker's policy. For slice 6:

- `onnx` runtime runs in-process (it's just `ort` reading a file we control).
- `python` runtime runs as a subprocess sandboxed by the existing tool-executor's `with_no_network()` — same path used by other Python-spawning surfaces.
- `native_candle` is in-process, no external execution.

The serving HTTP endpoint (`/serve/{name}/act`) inherits the daemon's auth bearer-token. We do **not** expose serving endpoints unauthenticated, even to localhost — too easy to footgun.

## Out of scope for slice 6

- **Cluster / multi-host deployments.** Single daemon = single host. Cluster deployment is a future product surface.
- **gRPC / WebSocket surface.** REST first; protocol expansion when there's a customer ask.
- **Batched inference.** Slice 6 ships per-request; batch is an obvious next.
- **GPU pinning across deployments.** Whichever GPU the runtime picks is what it gets.
- **Circuit breaking, retry policies, timeouts on the act path.** A reasonable default (5 s timeout, fail-fast) is the floor; richer policies are future.
- **`rl_run_optimization`** (currently mock at `commands.rs:41958`) — that's a slice-7 concern; slice 6 just consumes the artifacts optimization produces.

## Tests

- ONNX round-trip: train PPO on CartPole, export ONNX, load via `OnnxRuntime`, assert action distribution matches PyTorch within 1e-3.
- Python fallback: register a policy with no ONNX, deploy, hit `/act`, assert real action shape.
- Canary split: 10% canary across 10k requests gives 950–1050 canary hits (binomial check).
- Auto-rollback: inject latency above threshold for 60 s, assert rollback fires and traffic returns to production.

## Definition of done

1. User registers a policy (slice 5) → clicks deploy → sees a live deployment with real `/act` latency.
2. A/B between two policy versions works; the panel shows real traffic-split metrics.
3. Auto-rollback fires on latency or error-rate breach.
4. Disclaimer banner drops "Deployment".
