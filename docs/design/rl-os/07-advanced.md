# Slice 7 — Advanced (RLHF · Multi-Agent · Reward · Optimization)

**Status:** Draft · 2026-04-29
**Depends on:** all prior slices
**Disclaimer banner after this slice:** removed entirely. The four panels still under it (`Optimization`, `MultiAgent`, `RLHF`) all light up here. (`Rewards` was lit up in slice 5 against env-supplied components; this slice extends it to learned reward models for the RLHF flow.)

---

## Goal

Close out the four remaining illustrative panels — `RLOptimizationReport`, `RLMultiAgentView`, `RLHFAlignmentDashboard`, and the RLHF half of `RLRewardDecomposition` — and ship the complete end-to-end RL-OS productionization. This slice is wide rather than deep; each sub-area is a self-contained mini-project.

Order within the slice (sub-slice grain):

7a. Optimization (distill / quantize / prune)
7b. Multi-agent (MARL)
7c. RLHF + Constitutional / process rewards
7d. Native serving runtimes (Burn / CubeCL) for distilled outputs from 7a

---

## 7a — Optimization

### Goal

Take a registered teacher policy and produce a smaller / faster / lower-precision student. `rl_opti_os.rs` (3,387 LOC) has the type system; we wire three concrete pipelines.

### Pipelines

| Pipeline | Input | Output | Implementation |
|---|---|---|---|
| **Distillation** | Teacher Policy (slice 5), env, student-architecture spec | Student Policy with parent edge `distill_teacher` | Sidecar runs PPO/SAC against the env, but the loss includes a KL term against teacher actions — single-teacher first; multi-teacher later |
| **Quantization** | Teacher Policy (must have ONNX artifact) | Quantized ONNX policy (INT8 dynamic or static) + calibration report | `onnxruntime.quantization` from Python; output is registered as a new Policy with `framework='onnx'` and a metadata note `quantization: int8_dynamic` |
| **Pruning** | Teacher Policy (PyTorch artifact) | Sparse PyTorch + sparse ONNX export | `torch.nn.utils.prune` structured pruning; magnitude-based with retraining pass |

Each pipeline is a special run kind: `kind='distill' | 'quantize' | 'prune'`. They reuse the executor, sidecar, persistence, and registration paths from slices 1–5. The diff vs a training run:

- **Distill** has two policies in scope: teacher (read) + student (write). Lineage edge `distill_teacher` from student → teacher.
- **Quantize** has zero training; it's a transform with a calibration set (a small batch of recorded observations).
- **Prune** is a transform + a fine-tune; lineage edge `pruned_from`.

### HTTP routes

| Method | Path | Body |
|---|---|---|
| `POST` | `/v1/rl/optimization/distill` | `{ teacher_policy_id, student_arch, env_id, training_config }` |
| `POST` | `/v1/rl/optimization/quantize` | `{ source_policy_id, scheme: "int8_dynamic" \| "int8_static", calibration_run_id }` |
| `POST` | `/v1/rl/optimization/prune` | `{ source_policy_id, target_sparsity, structured: bool, finetune_steps }` |
| `GET` | `/v1/rl/optimization/reports/{run_id}` | — |

The `OptimizationReport` shape comes from `rl_opti_os.rs::OptimizationReport`; the sidecar emits one as the run's final artifact.

### Tauri command rewrites

| Command | After slice 7a |
|---|---|
| `rl_get_optimization_report` (mock at `commands.rs:41946`) | `GET /v1/rl/optimization/reports/{run_id}` |
| `rl_run_optimization` (mock at `commands.rs:41958`) | dispatches to one of the three POSTs above based on the body |

### Frontend (`RLOptimizationReport.tsx`)

- Pipeline picker (Distill / Quantize / Prune) with required-fields validation.
- Real before/after metrics: parameter count, model size MB, p95 latency on the target runtime, eval-suite delta.
- Side-by-side reward curves when distill includes an eval rollout.

---

## 7b — Multi-Agent

### Goal

Wire MARL training (PettingZoo envs + MAPPO / QMIX / VDN) and the cooperation-quality metrics rendered in `RLMultiAgentView.tsx`.

### Algorithm + library

- **MAPPO** (multi-agent PPO): we ship a CleanRL-style single-file implementation in `vibe_rl/algos/mappo.py`. Centralized critic, decentralized actors, parameter sharing optional.
- **QMIX, VDN**: implemented similarly (single-file).
- **Envs**: PettingZoo (parallel API). Slice 3's environment registry already covers `pettingzoo` source; slice 7 just consumes it.

### Metrics

The multi-agent panel today shows four mock metrics: coordination index, role specialization, equilibrium gap, communication entropy. Real implementations:

| Metric | Definition | When it works |
|---|---|---|
| **Per-agent return** | Already covered — episodes table per agent | Always |
| **Joint return** | Sum across agents per episode | Always |
| **Coordination index** | Correlation of per-agent rewards across episodes | When agents are coupled |
| **Role specialization** | Cluster purity of per-agent action distributions | When action spaces overlap |
| **Equilibrium gap** | Best-response improvement when one agent retrains while others freeze (expensive — opt-in) | Manual trigger |
| **Communication entropy** | Entropy of inter-agent message channel (only meaningful if env exposes one) | When env exposes a comm channel |

### HTTP routes

| Method | Path | Body |
|---|---|---|
| `GET` | `/v1/rl/runs/{run_id}/multi-agent-metrics` | — |
| `POST` | `/v1/rl/runs/{run_id}/equilibrium-probe` | `{ agent_idx, retrain_steps }` |

### Tauri command rewrites

| Command | After slice 7b |
|---|---|
| `rl_get_multi_agent_metrics` (mock at `commands.rs:41998`) | `GET /v1/rl/runs/{run_id}/multi-agent-metrics` |

---

## 7c — RLHF + Constitutional + Process Rewards

### Goal

The `RLHFAlignmentDashboard.tsx` panel rendering preference collection, reward-model training, PPO/DPO over an LLM, and alignment scores. `rl_rlhf.rs` (3,690 LOC, with 3 `TODO`s) has the types; we wire the executor to TRL.

### Library

[TRL (Transformer Reinforcement Learning)](https://github.com/huggingface/trl) — Apache 2.0, the de-facto standard for RLHF on top of HuggingFace transformers. Ships PPO, DPO, KTO, ORPO, GRPO. Vendored or pip-installed into `vibe-rl-py` extras.

This is the slice that brings RLHF into VibeCody's broader concerns. Specifically:

- The base model for RLHF can be any HF-format LLM the user has registered. Re-uses the model-hub schema (slice 5) with `framework='hf_transformers'`.
- Reward models are themselves Policies in the hub (with `framework='hf_classifier'`) — same registration / lineage path.
- Preference collection has its own panel UI: pairs of completions side-by-side, A/B feedback recorded into a new SQL table.

### Schema additions

```sql
CREATE TABLE rl_preferences (
    pref_id        TEXT PRIMARY KEY,
    suite_id       TEXT,                          -- optional grouping
    prompt         TEXT NOT NULL,
    completion_a   TEXT NOT NULL,
    completion_b   TEXT NOT NULL,
    chosen         TEXT NOT NULL,                 -- 'a' | 'b' | 'tie' | 'reject_both'
    rationale      TEXT,
    reviewer       TEXT,                          -- the user identity
    created_at     INTEGER NOT NULL
);

CREATE TABLE rl_alignment_scores (
    run_id         TEXT NOT NULL REFERENCES rl_runs(run_id) ON DELETE CASCADE,
    metric         TEXT NOT NULL,                 -- 'reward_score' | 'kl_to_base' | 'win_rate' | 'safety_violation_rate' | 'ConstitutionalAI_score'
    value          REAL NOT NULL,
    timestep       INTEGER NOT NULL,
    PRIMARY KEY (run_id, metric, timestep)
);
```

### RLHF flow (end-to-end)

1. **Collect preferences** — UI presents pairs, user clicks. (Or import from a JSONL.)
2. **Train reward model** — kind=`rlhf` run with sub-kind=`reward_model`. TRL's `RewardTrainer`. Output is a Policy registered in the hub.
3. **PPO/DPO/KTO/ORPO/GRPO over base LLM** — kind=`rlhf` run with sub-kind=`alignment`. Lineage edge `rlhf_base` to base LLM, `rlhf_reward_model` to the reward model.
4. **Constitutional self-critique** (opt-in) — sub-kind=`constitutional`. Uses a critique prompt list to filter outputs before reward scoring.
5. **Process rewards** (opt-in) — instead of scoring final output, score reasoning steps. PRM-style. Plugs in as a different reward-emission strategy in the trainer.

### Reward decomposition extension (the "Rewards" panel back-link)

Slice 5 wired env-supplied reward components. RLHF adds *learned* reward components: when the reward model is decomposable (multi-head: helpfulness + harmlessness + factuality), the contribution per head is logged per generation — same `rl_reward_components` table, different `component` strings.

### HTTP routes

| Method | Path | Body |
|---|---|---|
| `POST` | `/v1/rl/rlhf/preferences` | `{ suite_id, items: [{prompt, completion_a, completion_b}] }` |
| `POST` | `/v1/rl/rlhf/preferences/{pref_id}/judge` | `{ chosen, rationale }` |
| `GET` | `/v1/rl/rlhf/preferences` | `?suite_id=` |
| `POST` | `/v1/rl/rlhf/reward-model/train` | `{ base_model_id, preference_suite_id, training_config }` |
| `POST` | `/v1/rl/rlhf/align` | `{ base_policy_id, reward_model_id, algorithm: "ppo" \| "dpo" \| ..., training_config }` |
| `GET` | `/v1/rl/runs/{run_id}/alignment-metrics` | — |

### Tauri command rewrites

| Command | After slice 7c |
|---|---|
| `rl_get_alignment_metrics` (mock at `commands.rs:42029`) | `GET /v1/rl/runs/{run_id}/alignment-metrics` |
| (new) `rl_collect_preference`, `rl_train_reward_model`, `rl_run_alignment` | corresponding routes |

### Constraints

- **The base LLM is HF-format only** in slice 7c. Native-Rust inference for RLHF is post-slice (touches `vibe-infer` / TurboQuant — a separate workstream).
- **Single-GPU for the alignment step** in slice 7c. Multi-GPU sharding reuses TRL's `Accelerate` integration but is not on the critical path for disclaimer removal.
- **No reward-hacking auto-detection** in 7c. `rl_rlhf.rs` enumerates the metrics; we surface them but don't auto-flag.

---

## 7d — Native serving runtimes (Burn / CubeCL)

### Goal

Stand up the `burn` and `cubecl` runtime kinds in the deployment trait from slice 6. This is the payoff for slice 7a's quantization / distillation: a small student policy can serve from a single Rust binary, no Python, GPU-accelerated on macOS Metal and Linux CUDA via a single source.

### Why CubeCL/Burn here, not earlier

The TurboQuant memo bans CubeCL/Burn from the kv-cache codec path because that path is hand-tuned around Candle/mistralrs-quant. RL-OS serving is a *different problem*: the per-request inference is a fixed-graph forward pass through a small policy network, exactly the shape CubeCL is good at. Burn's portability across Metal + CUDA + WGPU is the value prop. The user's clarification on 2026-04-29 explicitly opens this door.

### Scope

- `BurnRuntime` impl of `PolicyRuntime` (slice 6 trait) loading from Burn-format weights.
- `CubeclRuntime` impl wrapping Burn's CubeCL backend specifically (so the policy's matmul kernels run on the GPU regardless of host vendor).
- Conversion pipeline in slice 7a: `Distill` and `Prune` outputs optionally export Burn-format alongside ONNX. Quantization stays ONNX-only (INT8 path is mature in onnxruntime, less so in Burn at this writing).
- A new policy framework value: `framework='burn_native'` with the manifest format documented in this doc.

### Manifest format

```toml
# .vibecli/rl-policies/<name>/<version>/burn-manifest.toml
framework = "burn"
backend = "cubecl"        # or "metal", "cuda", "wgpu", "ndarray"
weights_file = "weights.bin"
arch = "mlp_64_64"        # one of a small set we ship implementations for
obs_dim = 4
act_dim = 2
act_kind = "discrete"
```

We ship a small zoo of `arch` values (`mlp_64_64`, `mlp_256_256`, `cnn_atari_v1`, `lstm_128_x2`, `transformer_small_v1`) — enough to cover what the slice-2 algorithms produce. Custom architectures fall back to the ONNX runtime.

### Non-goals (even at end of slice 7)

- Burn / CubeCL at *training* time. That's the long-arc Phase C; not in this doc set.
- Auto-conversion of arbitrary PyTorch models to Burn. Architecture allowlist only.

---

## Cross-cutting deliverables for slice 7

1. **Disclaimer banner removed entirely** from `RLOSComposite.tsx:19`. The component itself can stay as an empty `Fragment` for now and be deleted on the same PR (or in a chase-up cleanup).
2. **Status doc** at `docs/RL-OS-ARCHITECTURE.md` updated to reflect "implemented" rather than "specified."
3. **Skill files** under `vibecli/vibecli-cli/skills/rl-*.md` updated with concrete invocation examples that actually work (currently aspirational).
4. **Release notes** entry — RL-OS productionization gets its own line in `docs/CHANGELOG.md` and the next release-notes YAML.
5. **AGENTS.md Product Matrix update** — RL-OS becomes a real entry in the cross-cutting checklist.

## Definition of done

1. The four formerly-illustrative panels (Optimization, MultiAgent, RLHF, post-7a Rewards) all show real numbers.
2. A user can run a complete pipeline: train → eval → register → distill → quantize → deploy → A/B → RLHF → re-eval. Every step persists, every step has real metrics, every step is in the lineage DAG.
3. The disclaimer banner is gone.
4. The 31k lines of `rl_*_os.rs` are wired (via the executor and HTTP routes) — anything still orphaned at this point is a candidate for deletion.
