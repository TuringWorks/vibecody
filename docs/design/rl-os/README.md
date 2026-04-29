# RL-OS Productionization — Design Index

**Status:** Draft · 2026-04-29
**Scope:** vibecli daemon (Rust) + new `vibe-rl-py/` sidecar + vibeui RL-OS panels (10) + Tauri bridge (20 commands)
**Owner:** TBD
**Cross-references:** [AGENTS.md](../../../AGENTS.md) (storage rules, Product Matrix), [vibeui/design-system/README.md](../../../vibeui/design-system/README.md) (panel UX), [docs/RL-OS-ARCHITECTURE.md](../../RL-OS-ARCHITECTURE.md) (the original 1.0 spec these docs operationalize)

---

## What this is

The RL-OS surface in VibeCody — a 10-panel composite tab covering Training, Environments, Evaluation, Optimization, Deployment, Policy Comparison, Model Lineage, Multi-Agent, Reward Decomposition, and RLHF Alignment — currently renders **deterministic synthetic data**. The disclaimer banner at `vibeui/src/components/composite/RLOSComposite.tsx:19` is honest: numbers are illustrative and do not reflect production runs.

This doc set is the plan to remove that disclaimer panel-by-panel by building a real RL lifecycle stack: persistence, a training executor, real environments, real evaluation, a real model hub, real deployment, and the advanced surfaces (RLHF, MARL, optimization). Each slice ends with one or more panels showing real numbers and the disclaimer text being trimmed to cover only the still-illustrative panels.

## The 7-slice plan

| # | Slice | Doc | First panel(s) wired | Critical path |
|---|---|---|---|---|
| 1 | **Persistence + run lifecycle** | [01-persistence.md](./01-persistence.md) | RLTrainingDashboard (run list only) | SQLite schema in `workspace.db` (encrypted) |
| 2 | **Training executor (Python sidecar)** | [02-training-executor.md](./02-training-executor.md) | RLTrainingDashboard (live metrics) | `vibe-rl-py/` + CleanRL + streaming JSONL |
| 3 | **Environment registry** | [03-environments.md](./03-environments.md) | RLEnvironmentViewer | Gymnasium probe + custom env registration |
| 4 | **Evaluation suites** | [04-evaluation.md](./04-evaluation.md) | RLEvalResults, RLPolicyComparison | Eval rollouts + off-policy estimators |
| 5 | **Model hub + lineage** | [05-model-hub.md](./05-model-hub.md) | RLModelLineage, RLRewardDecomposition | Artifact store + DAG |
| 6 | **Deployment + serving** | [06-deployment.md](./06-deployment.md) | RLDeploymentMonitor | ONNX runtime path + A/B + health |
| 7 | **Advanced (RLHF / MARL / Opt)** | [07-advanced.md](./07-advanced.md) | RLHFAlignmentDashboard, RLMultiAgentView, RLOptimizationReport | TRL + PettingZoo + distill/quantize |

Slices are sequenced because each depends on the previous: no executor without persistence, no eval without an executor, no hub without artifacts to register, no deployment without registered models, no advanced workflows without the basic loop closed.

## Backend choice — Path C (hybrid)

After looking at the three credible compute backends, the chosen path is:

- **Phase A (slices 1–6 default):** Python sidecar to **CleanRL** (single-file PyTorch implementations of PPO, SAC, DQN, DDPG, TD3) with **Gymnasium** envs, **TRL** for RLHF (slice 7), **PettingZoo + MAPPO** for multi-agent (slice 7). The daemon spawns `vibe-rl-py` subprocesses, streams metrics over JSON-Lines, persists to SQLite.
- **Phase B (slice 6 + slice 7 optimization, opt-in):** Native Rust path for **inference-only** workloads — distilled / quantized policies served via Candle, **Burn**, or **CubeCL** kernels. This is where the cross-platform GPU story matters; CubeCL/Burn are not excluded for RL-OS (memory note: TurboQuant's CubeCL/Burn ban is scoped to TurboQuant kernels only).
- **Phase C (long arc, post-doc):** Native Rust *training* path for the hot inner loop of PPO/SAC/DQN where Python overhead dominates. Replaces sidecar incrementally; sidecar stays as a fallback and as the home for algorithms that aren't worth porting.

**Why hybrid not pure-Python or pure-Rust:** pure-Python ships in weeks but never gets the inference latency or single-binary deploy story we want for edge serving. Pure-Rust is months before any panel shows real data and gambles ecosystem reach (DreamerV3, MuZero, TRL won't be reimplemented). Hybrid lets each panel light up at the natural pace of the underlying capability.

## Architecture (target state)

```text
┌────────────────────── vibeui (Tauri 2 + React) ──────────────────────┐
│  RLOSComposite.tsx → 10 panels                                        │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ Tauri commands (20)
                                ▼
┌────────────── vibeui/src-tauri/src/commands.rs ───────────────────────┐
│  rl_* command handlers — thin wrappers → daemon HTTP /v1/rl/*         │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ HTTP / SSE
                                ▼
┌──── vibecli daemon (Rust) — vibecli/vibecli-cli/src/ ─────────────────┐
│  serve.rs: /v1/rl/runs · /metrics · /envs · /eval · /models · /deploy │
│  rl_train_os.rs · rl_eval_os.rs · rl_env_os.rs · rl_serve_os.rs       │
│  rl_opti_os.rs · rl_model_hub.rs · rl_rlhf.rs · rl_observe.rs         │
│  Persistence: WorkspaceStore (workspace.db, encrypted)                 │
└──────────┬─────────────────────────────────────────┬──────────────────┘
           │ spawn + JSON-Lines (Phase A)            │ in-process (Phase C)
           ▼                                          ▼
┌──── vibe-rl-py/ (Python sidecar) ─────┐  ┌── candle / burn / cubecl ──┐
│  CleanRL · Gymnasium · TRL · PettingZ │  │  Phase B/C native kernels  │
│  vec envs · checkpointing · ONNX exp  │  │  inference + hot training  │
└────────────────────────────────────────┘  └────────────────────────────┘
                                                         │
                                                         ▼
                                          ┌── Inference / serving ──────┐
                                          │  ONNX Runtime · WASI        │
                                          │  edge deploy (no Python)    │
                                          └──────────────────────────────┘
```

The daemon stays the single source of truth (per CLAUDE.md cross-cutting invariants). Tauri commands and panels never bypass it. The sidecar is a managed child process; it never speaks directly to the UI.

## Goals

1. **Disclaimer banner falls panel-by-panel.** Each slice ends with one or more `disclaimerCovers` entries removed; never ship a slice that leaves the affected panel still labelled "illustrative".
2. **Persistence is encrypted and per-workspace.** All RL state lives in `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`) per AGENTS.md storage rules. No plaintext run logs.
3. **One executor abstraction across algorithms.** A `TrainingExecutor` trait with one Python-sidecar impl in Phase A; a Rust impl swaps in for Phase C without touching commands or panels.
4. **Reproducibility.** Every run records: seed, code hash (sidecar version), env version, config YAML, dependency lock. The recap system already speaks "session" and "job" — RL runs are jobs.
5. **No panel knows whether the backend is Python or Rust.** The Tauri command surface is identical in Phase A vs Phase C.

## Non-goals

- A new tensor library. We use what works (PyTorch in sidecar, Candle/Burn/CubeCL in native).
- Distributed training across machines in slice 2. Vector envs on a single host first; multi-host later.
- Replacing `rl_*_os.rs` modules wholesale. They're 31k lines of well-typed configs/registries; we **wire them**, fill in the implementation gaps where there are no real `step`/`forward`/`rollout` functions, and let the executor produce the data they describe.
- A custom env runtime that competes with Gymnasium. We embrace it.

## What exists today (grounded as of `8118771f`, 2026-04-29)

| Surface | File | State |
|---|---|---|
| Disclaimer banner | `vibeui/src/components/composite/RLOSComposite.tsx:19` | Renders today; gates removal of panel-specific labels |
| 10 panel components | `vibeui/src/components/RL*.tsx` | Render synthetic data fed by Tauri commands |
| 20 Tauri commands | `vibeui/src-tauri/src/commands.rs:41688-42029` | All return mock data from `OnceLock<Mutex<Vec<Value>>>` |
| Tauri registration | `vibeui/src-tauri/src/lib.rs:1526-1545` | All 20 wired in `generate_handler!` |
| `rl_train_os.rs` | `vibecli/vibecli-cli/src/rl_train_os.rs` (3,780 LOC) | Algorithm/config/registry types + ELO + replay-buffer priority math. **No `step` / `forward` / `rollout` functions.** Module declared in `lib.rs:362` but **no callers** |
| `rl_eval_os.rs` | (3,819 LOC) | Eval suite types, off-policy estimator types, regression-test types. **No execution.** No callers |
| `rl_env_os.rs` | (4,202 LOC) | Space DSL, env-registry types, version DAG, domain-randomization config. **No env step.** No callers |
| `rl_serve_os.rs` | (4,490 LOC) | Session, A/B, canary types. **No real serving.** No callers |
| `rl_opti_os.rs` | (3,387 LOC) | Distill/quantize/prune config + report types. **No real optimization.** No callers |
| `rl_model_hub.rs` | (3,726 LOC) | Registry + lineage + promotion types. **No artifact storage.** No callers |
| `rl_rlhf.rs` | (3,690 LOC) | RLHF algorithm config types + 3 `TODO`s. No callers |
| `rl_observe.rs` | (4,401 LOC) | Drift/anomaly metric types. No callers |
| `rlcef_loop.rs` | (LOC unknown — Gap 18) | Code execution feedback loop — separately wired into vibecli, partially real |
| Daemon HTTP routes | `vibecli/vibecli-cli/src/serve.rs` | Zero `/v1/rl/*` routes today |
| Skill docs | `vibecli/vibecli-cli/skills/rl-*.md` | 9 skill files describing capabilities (aspirational) |

So the parts we have are: types, configs, registries, panels, command names, registration. The parts we are missing: every **execution** layer, every **persistence** layer, every **HTTP route** the daemon exposes, every **Python integration**, every **artifact** on disk.

This is a build, not a refactor. The slice docs scope the build.

## Cross-cutting impact (per CLAUDE.md change-surface checklist)

Per AGENTS.md → Product Matrix, RL-OS is currently a **VibeUI-only** feature. Mobile/watch/VS Code/JetBrains/Neovim plugins do not surface RL-OS panels and do not need to in v1. If we ever expose run-status notifications to mobile, that's a separate slice not in this doc set.

| Change type (as we ship slices) | Surfaces touched |
|---|---|
| New daemon HTTP route (`/v1/rl/*`) | `serve.rs` → Tauri thin wrapper in `commands.rs` → optional doc in `docs/api/` |
| New Tauri command | `commands.rs` → `generate_handler!` in `vibeui/src-tauri/src/lib.rs` (and `vibeapp/src-tauri/src/lib.rs` if RL is ever surfaced there — currently not) |
| New SQL migration | `WorkspaceStore` schema bump + migration test |
| New Python dependency | `vibe-rl-py/pyproject.toml` + lockfile + `Makefile` install target + CI matrix |
| Sidecar version bump | `vibe-rl-py/VERSION` + Rust `vibe-rl-sidecar` crate constant + CI integration test |

No mobile / watch / IDE-plugin surfaces are touched by any slice in this doc set.

## Open questions

1. **Python runtime distribution.** Bundle a portable interpreter (PyOxidizer, python-build-standalone) with the daemon, or require a host Python? See [02-training-executor.md](./02-training-executor.md#python-runtime-distribution).
2. **GPU detection.** The sidecar needs to know whether to ask for CUDA / MPS / CPU. Probe at sidecar startup; surface to UI in run-config wizard.
3. **Artifact storage location.** Workspace-relative (`.vibecli/rl-artifacts/`) vs profile-wide (`~/.vibecli/rl-artifacts/`) vs configurable. See [05-model-hub.md](./05-model-hub.md#artifact-storage).
4. **Multi-tenant runs.** Single user / single workspace assumed throughout. If RL-OS ever runs in a shared daemon (vibe-collab), authz becomes load-bearing.
5. **Telemetry of training runs.** Out of scope for this doc set; opt-in usage analytics live elsewhere.
6. **OS coverage of the sidecar.** macOS + Linux first-class; Windows targeted but Gymnasium/MuJoCo on Windows is rougher. CI pin: macOS-arm64 + ubuntu-22.04 + windows-2022.

## Migration of the disclaimer banner

`SimulationModeBadge` in `RLOSComposite.tsx:19` currently labels the entire RL-OS tab. Slice deliverable: the badge becomes parameterized — `<SimulationModeBadge covers={[...stillIllustrativePanels]} />` — and each slice removes the panels it productionizes from that list. Final slice (#7) deletes the badge entirely.

```tsx
// Target shape after slice 1+2:
<SimulationModeBadge covers={["Environments","Evaluation","Optimization","Deployment","Compare","Lineage","MultiAgent","Rewards","RLHF"]} />
// After slice 7:
// (badge removed)
```

This gives the user honest ground-truth on which panels are real at any commit. No big-bang flip.

## Risks

- **Sidecar deployment friction.** If users have to install Python + nvidia drivers + MuJoCo licenses to use RL-OS, adoption stalls. Mitigation: bundle python-build-standalone, default to CPU + CartPole, document the GPU upgrade path. See slice 2.
- **31k lines of orphaned scaffolding rot.** The `rl_*_os.rs` modules will diverge from reality unless we either wire them or delete them. Slice 1 begins the wiring; if a module isn't wired by slice 7, that's a signal to delete.
- **Python ↔ Rust serialization cost.** Per-step IPC is fatal for fast envs (CartPole at 10k steps/s). Mitigation: env loop runs entirely in Python; only metric snapshots cross the boundary. See slice 2.
- **License surface.** Gymnasium (MIT), CleanRL (MIT), TRL (Apache 2.0) — clean. MuJoCo (Apache 2.0 since 2021) — clean. PettingZoo (MIT) — clean. No GPL surface introduced.
- **Patent surface.** RL training is unencumbered. The diffcomplete patent-distance work (per CLAUDE.md memory) is unrelated to RL-OS — RL training surfaces don't need patent re-audit.

## Glossary (project-specific)

- **rl_*_os.rs**: 8 modules in `vibecli/vibecli-cli/src/` enumerating RL types/configs. Currently orphaned.
- **Sidecar (`vibe-rl-py`)**: managed Python child process owned by the daemon, lifecycle = run.
- **WorkspaceStore**: encrypted SQLite at `<workspace>/.vibecli/workspace.db` per AGENTS.md.
- **Run**: one training execution, identified by a ULID (`run_id`), persisted across restarts.
- **Artifact**: an on-disk file (model weights, ONNX export, replay buffer snapshot) referenced by SQL row.
- **Suite**: a named collection of evaluation episodes against one or more environments.
- **Disclaimer banner**: `SimulationModeBadge` at `RLOSComposite.tsx:19` — the visible signal of un-productionized panels.
