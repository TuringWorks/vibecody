# Slice 2 — Training Executor (Python Sidecar)

**Status:** Draft · 2026-04-29
**Depends on:** [01-persistence.md](./01-persistence.md)
**Unblocks:** slices 3–7
**Disclaimer banner after this slice:** drops "Training" from the covered list. `RLTrainingDashboard` shows real reward / loss / KL / entropy / GPU curves on real Gymnasium environments.

---

## Goal

Make `POST /v1/rl/runs/{id}/start` actually train. Spawn a managed Python subprocess that runs CleanRL on the user's chosen Gymnasium env, streams metrics back over JSON-Lines, persists them via `RunStore` (slice 1), checkpoints to disk, and exits cleanly on stop.

## Why CleanRL

CleanRL ([Huang et al., JMLR 2022](https://github.com/vwxyzjn/cleanrl)) is single-file PyTorch implementations of PPO, SAC, DQN, DDPG, TD3, C51, QR-DQN, and a few more. Each algorithm is one self-contained `.py` file with no inheritance hierarchy. We vendor it (under MIT) into `vibe-rl-py/cleanrl/` and patch only the metric-emission layer.

**Why not Stable-Baselines3:** SB3 is more mature but its abstraction layers (BaseAlgorithm, OffPolicyAlgorithm, etc.) make it hard to splice in our metric stream and our checkpoint format without monkey-patching. CleanRL is closer to "the algorithm in a single readable file"; that maps cleanly to "spawn one file per run."

**Why not RLlib:** RLlib is for cluster-scale Ray training. Out of scope for slice 2; the abstraction tax isn't worth it for single-host work. We can revisit for the multi-host slice (not in this doc set).

**Why not Tianshou / ACME / SaLina / Garage:** all credible; CleanRL wins on code-readability and PR-mergeability for the patches we need.

## Sidecar layout

New top-level directory `vibe-rl-py/`:

```text
vibe-rl-py/
├── pyproject.toml           # uses uv (already in CI per recent commits)
├── uv.lock
├── VERSION                  # version string Rust pins against
├── vibe_rl/
│   ├── __init__.py
│   ├── __main__.py          # entry: python -m vibe_rl <subcommand>
│   ├── cli.py               # subcommands: train, eval, distill, rlhf, probe-envs, probe-gpu
│   ├── runtime.py           # subprocess lifecycle, signal handling, stop/cancel via SIGTERM
│   ├── streamer.py          # JSON-Lines emitter, fd-3 (or stdout if fd-3 unavailable)
│   ├── checkpoint.py        # save/load — PyTorch state_dict + metadata sidecar
│   ├── envs/
│   │   ├── registry.py      # discover Gymnasium envs, register custom .py files
│   │   └── wrappers.py      # MonitorWrapper that emits per-step metrics
│   ├── algos/
│   │   ├── ppo.py           # vendored CleanRL ppo_continuous_action.py + ppo.py merged
│   │   ├── sac.py
│   │   ├── dqn.py
│   │   ├── ddpg.py
│   │   └── td3.py
│   ├── eval.py              # rollout loop, off-policy estimators (slice 4)
│   └── export.py            # ONNX export (slice 5/6)
├── tests/
│   └── ...
└── README.md
```

Vendored algorithm files keep their CleanRL provenance comment block (per CleanRL MIT license). Patches we add are clearly marked `# vibecody:` so future CleanRL upstream syncs stay tractable.

## Python runtime distribution

Three options, ranked by preference:

1. **Bundle [python-build-standalone](https://github.com/astral-sh/python-build-standalone)** — single tarball, no host Python required, ~30 MB compressed. Ships with the daemon installer per OS. **Picked.**
2. **Require host Python ≥ 3.11.** Documented requirement, friction at first run.
3. **PyOxidizer.** Embeds Python in the daemon itself. Heavier integration; revisit if (1) hits snags.

The bundled interpreter lives at `<install_root>/vendor/python/{darwin-arm64,linux-x64,linux-arm64,windows-x64}/`. CI matrix (`.github/workflows/release.yml`) downloads + caches per-OS tarballs and unpacks into the per-OS bundle alongside the existing daemon binary.

`uv` is the package manager (already in this repo's CI per recent build-fix commit `8118771f`). The daemon, on first run, materializes `vibe-rl-py`'s venv into `~/.vibecli/python-envs/vibe-rl-py-<version>/` (per-profile, not per-workspace — wheels are big and shouldn't pollute every workspace tree). Materialization is idempotent and cached against the lockfile hash.

GPU detection: at sidecar startup, `python -m vibe_rl probe-gpu` runs and emits one JSON line:

```json
{"cuda": true, "cuda_devices": [{"name": "RTX 4090", "memory_mb": 24576}], "mps": false, "rocm": false}
```

The Rust side caches this per-profile. The training-run wizard surfaces it: "GPU: NVIDIA RTX 4090, 24 GB" or "GPU: none — running on CPU."

## Sidecar version pinning

Rust pins the sidecar version in a constant:

```rust
// vibecli/vibecli-cli/src/rl_sidecar.rs
pub const VIBE_RL_PY_VERSION: &str = env!("VIBE_RL_PY_VERSION");  // wired in build.rs from VERSION file
```

`build.rs` reads `vibe-rl-py/VERSION` and emits a compile-time check. Mismatch between Rust constant and on-disk sidecar = startup error with a clear message ("rl sidecar version mismatch — expected X, found Y").

## Subprocess lifecycle

A new module `vibecli/vibecli-cli/src/rl_executor.rs` owns the executor:

```rust
pub struct PythonExecutor {
    interpreter: PathBuf,             // bundled python
    sidecar_root: PathBuf,            // .../vibe-rl-py/
    venv_python: PathBuf,             // ~/.vibecli/python-envs/.../bin/python
    runs: Mutex<HashMap<RunId, Child>>, // tokio::process::Child handles
    store: Arc<RunStore>,
}

#[async_trait]
impl TrainingExecutor for PythonExecutor {
    async fn start(&self, run_id: RunId) -> Result<()>;
    async fn stop(&self, run_id: RunId) -> Result<()>;
    async fn cancel(&self, run_id: RunId) -> Result<()>;
    async fn status(&self, run_id: RunId) -> Result<RunStatus>;
}
```

`start` does:

1. Read the run row + config from `RunStore`.
2. Spawn `<venv_python> -m vibe_rl train --run-id <id> --config <path-to-temp-yaml> --workspace <path>`.
3. Open `fd 3` on the child for the metric stream (Python writes JSON-Lines there).
4. Spawn three async tasks:
   - **Metric reader**: parses fd-3 lines, batches by 100 ticks or 250 ms (whichever first), calls `store.append_metrics(...)`.
   - **Episode reader**: parses a separate fd-4 for episode-end events.
   - **Stdout/stderr drain**: tees to `<workspace>/.vibecli/rl-logs/<run_id>.jsonl` and structured tracing.
5. Transition the run from `Queued` → `Running` once the child emits its `started` heartbeat.

`stop` sends `SIGTERM`; the sidecar's `runtime.py` catches it, finishes the current update step, writes a final checkpoint, transitions to `Stopped`. Hard timeout 30 s before `SIGKILL`.

`cancel` is `SIGKILL` immediately + transition to `Cancelled`. No checkpoint.

Crash recovery: on daemon startup, any run with status=`Running` that has no live child gets transitioned to `Failed` with `error_message="daemon restart while running"`. (Auto-resume from last checkpoint is a separate feature; for slice 2, restarts are user-initiated.)

## Metric stream protocol

`fd 3` lines, one JSON object per line:

```json
{"t":"started","run_id":"01HXY...","wall":1714368000123,"sidecar_version":"0.1.0","seed":42,"device":"cuda:0"}
{"t":"tick","run_id":"01HXY...","tick":1,"timestep":2048,"wall":1714368003012,"payload":{"policy_loss":0.0234,"value_loss":1.234,"entropy":0.78,"approx_kl":0.012,"clip_fraction":0.18,"learning_rate":0.0003,"grad_norm":0.41,"sps":1820}}
{"t":"episode","run_id":"01HXY...","idx":17,"timestep":2100,"reward":195.0,"length":195,"success":true,"duration_ms":1820}
{"t":"checkpoint","run_id":"01HXY...","timestep":50000,"rel_path":".vibecli/rl-artifacts/01HXY.../ckpt-50k.pt","sha256":"..."}
{"t":"gpu","run_id":"01HXY...","wall":1714368010123,"util":[78.0],"mem_mb":[18432]}
{"t":"finished","run_id":"01HXY...","wall":1714370000123,"reason":"done","final_reward_mean":487.3}
```

`tick`s are aggregated metrics from N gradient updates. `episode` rows go to `rl_episodes`. Why two streams: episodes are sparse and per-row; ticks are dense and per-window — separating them makes the SQL more sensible and lets the UI subscribe to one without the other.

The sidecar batches lines internally with a 50 ms flush window so we don't context-switch the daemon on every step.

## Live UI updates

The Tauri command `rl_get_training_metrics` flips from "synthesize noise" to "read persisted batch + tail." For live charts:

- New Tauri command `rl_subscribe_metrics(run_id)` opens an SSE connection to `GET /v1/rl/runs/{id}/metrics/stream`, emits Tauri events `rl-metrics:<run_id>` to the frontend.
- `RLTrainingDashboard.tsx` subscribes via `listen('rl-metrics:<run_id>', ...)` while the run is `Running`, falls back to the polling `rl_get_training_metrics` for completed runs.

Why SSE over WebSocket: half-duplex is enough, SSE survives proxies better, the daemon already speaks SSE for recap streams (per the recap-resume design docs).

## Checkpoint format

PyTorch state-dict + a metadata sidecar:

```text
.vibecli/rl-artifacts/<run_id>/
├── ckpt-50000.pt           # torch.save({"policy": ..., "value": ..., "optimizer": ...})
├── ckpt-50000.json         # {"timestep": 50000, "algorithm": "PPO", "env_id": "CartPole-v1", "obs_space": ..., "act_space": ..., "sidecar_version": "0.1.0", "git_sha": "..."}
├── ckpt-100000.pt
├── ckpt-100000.json
├── final.pt                # last checkpoint, symlinked or copied
├── final.json
└── replay/
    └── buffer-100000.npz   # off-policy only
```

Every checkpoint is also recorded in `rl_artifacts` with `kind='checkpoint'`, plus `final` rows for the run-end snapshot. SHA-256 is computed by the sidecar before emit.

## Algorithm coverage in slice 2

Ship four algorithms, not all 30+:

- **PPO** — discrete + continuous action spaces (covers CartPole, LunarLander, Pendulum, MuJoCo locomotion)
- **SAC** — continuous off-policy (covers Pendulum, MuJoCo locomotion)
- **DQN** — discrete off-policy (covers CartPole, Atari subset)
- **TD3** — continuous off-policy alt (covers MuJoCo locomotion)

These four cover ~95% of "I want to try a thing" RL workflows. The other 26 algorithms enumerated in `rl_train_os.rs::AlgorithmId` get marked "coming soon" in the wizard step. Shipping them is per-algorithm grunt work, not architecture work — they slot into the same executor.

## Hyperparameter validation

The wizard in `RLTrainingDashboard.tsx` already collects hyperparameters. Slice 2 adds server-side validation in `rl_runs::CreateRunRequest::validate()`:

- learning_rate ∈ (0, 1)
- gamma ∈ [0, 1]
- batch_size > 0, ≤ 65536
- algorithm-specific: PPO clip_coef ∈ (0, 1), SAC alpha > 0, etc.

Invalid configs return `400 Bad Request` with a structured error. The wizard surfaces these inline.

## Out of scope for slice 2

- **Distributed training** (AllReduce, parameter server). Single-host vector envs only. Multi-host is a later slice not in this doc set.
- **Population-based training, Bayesian HPO, NAS.** AutoRL features. Later.
- **Curriculum learning.** Later.
- **Multi-agent.** Slice 7.
- **Real-environment connectors** (REST/gRPC/MQTT envs). Slice 3.
- **MuJoCo / DeepMind Control / Atari ROMs.** Documented as optional installs. Default install ships classic-control envs only.

## Tests

- `pytest vibe-rl-py/tests/` — unit tests for streamer, checkpoint, registry.
- Integration: spawn `python -m vibe_rl train` against `CartPole-v1` for 10k steps, assert reward > 100 by end. Runs in CI on every PR.
- Rust integration: spawn the executor against a fixture sidecar that emits scripted JSON-Lines; assert metrics persist correctly.
- Stop signal: 5 ms after `start`, send `stop`; assert sidecar exits within 30 s and writes a checkpoint.
- Crash recovery: kill the daemon mid-run; assert the row transitions to `Failed` on next startup.

## Definition of done

1. `RLTrainingDashboard` shows real reward / loss / KL / entropy / GPU curves on `CartPole-v1` PPO and `Pendulum-v1` SAC.
2. Stop button stops within 30 s and leaves a final checkpoint.
3. Metrics persist across daemon restart; reopening a completed run shows the historical curves.
4. Disclaimer banner in `RLOSComposite.tsx:19` updates to drop "Training" from `covers={[...]}`.
5. CI green on macOS-arm64 + ubuntu-22.04 + windows-2022 (Windows may skip MuJoCo deps).
