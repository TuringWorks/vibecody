# RL-OS Quickstart

End-to-end runnable pipeline covering every shipped workflow:
**train → register → deploy → /act**, plus optimization, multi-agent, and
RLHF. Every command in this doc was verified against the slice-by-slice
smoke runs during productionization (see commit messages on `main`).

For the per-slice design rationale, see [README.md](./README.md). For
the original 7-slice plan that this implements, see slice docs
[01-persistence.md](./01-persistence.md) through
[07-advanced.md](./07-advanced.md).

---

## Prerequisites

```bash
# 1. Workspace
cd /path/to/your/workspace          # any directory; .vibecli/ goes here
mkdir -p .vibecli                   # daemon will create the rest

# 2. Sidecar venv (one-time)
cd /path/to/vibecody/vibe-rl-py
uv sync                             # base: gymnasium + torch + pyyaml + numpy

# Opt-in extras (install only what you need)
uv sync --extra opt                 # quantize: onnx + onnxruntime
uv sync --extra marl                # multi-agent: pettingzoo + supersuit + mpe2
uv sync --extra rlhf                # alignment: transformers + accelerate

# 3. Daemon (Rust)
cd /path/to/vibecody
cargo build --release -p vibecli
./target/release/vibecli serve      # runs on http://localhost:7878 by default
```

The first daemon launch creates `<workspace>/.vibecli/workspace.db` (the
encrypted SQLite that holds runs, episodes, metrics, environments, eval
suites, policies, deployments, lineage edges, preferences, and alignment
scores — all 11 RL-OS tables introduced across slices 1, 5, 7).

---

## Workflow 1 — Single-agent PPO on CartPole

The slice-2 baseline. PyTorch state-dict checkpoint + JSON metadata
sidecar.

```yaml
# /tmp/cartpole.yaml
algorithmId: PPO
environmentName: CartPole-v1
total_timesteps: 100000
num_envs: 4
num_steps: 128
seed: 42
checkpoint_every_steps: 50000
workspace_path: /your/workspace
artifact_dir: /your/workspace/.vibecli/rl-artifacts/cartpole-baseline
```

```bash
cd vibe-rl-py
uv run python -m vibe_rl train --run-id cartpole-baseline --config /tmp/cartpole.yaml

# JSON-Lines on stdout:
# {"t":"started","run_id":"cartpole-baseline","device":"mps", ...}
# {"t":"episode","idx":1, "reward":195.0, ...}
# {"t":"tick","tick":1,"timestep":2048,"payload":{"policy_loss":0.02,"value_loss":1.2,"entropy":0.69,"approx_kl":0.012, ...}}
# ...
# {"t":"checkpoint","timestep":50000,"rel_path":".vibecli/rl-artifacts/cartpole-baseline/ckpt-50000.pt","sha256":"..."}
# {"t":"finished","reason":"done","final_reward_mean":487.3}
```

Or via the daemon's HTTP route (recommended — gets the run row in
SQLite + lifecycle management):

```bash
curl -X POST http://localhost:7878/v1/rl/runs \
  -H 'content-type: application/json' \
  -d '{"name":"cartpole-baseline","kind":"train","algorithm":"PPO",
       "environment_id":"gym:CartPole-v1:gym-bundled",
       "config_yaml":"...","seed":42,"total_timesteps":100000,
       "workspace_path":"/your/workspace"}'

curl -X POST http://localhost:7878/v1/rl/runs/<run_id>/start
curl http://localhost:7878/v1/rl/runs/<run_id>/metrics
```

---

## Workflow 2 — Optimize: Quantize to INT8

The slice-7a quantize pipeline. Takes a PyTorch checkpoint, exports to
ONNX, applies dynamic INT8 quantization. Opt-in: `uv sync --extra opt`.

```yaml
# /tmp/quantize.yaml
kind: quantize
source_checkpoint: /your/workspace/.vibecli/rl-artifacts/cartpole-baseline/final.pt
scheme: int8_dynamic
workspace_path: /your/workspace
artifact_dir: /your/workspace/.vibecli/rl-artifacts/cartpole-int8
```

```bash
uv run python -m vibe_rl train --run-id cartpole-int8 --config /tmp/quantize.yaml
# Smoke result on slice-2 PPO output:
#   FP32: 19,329 bytes  →  INT8: 9,099 bytes  (2.12× compression)
#   Outputs: final.onnx (FP32) + final-int8.onnx + final-int8.json
```

Pruning (`kind: prune`, `target_sparsity: 0.5`) and distillation
(`kind: distill`, `teacher_checkpoint: ...`) work the same way.

---

## Workflow 3 — Register a Policy + Deploy

Slice 5 (model hub) + slice 6 (deployment lifecycle) + slice 6.5 / ONNX
runtime path. Promotes a checkpoint to a first-class `Policy` with
semver, lineage, and a model card.

```bash
# Register the trained PPO checkpoint as a Policy.
curl -X POST http://localhost:7878/v1/rl/policies \
  -H 'content-type: application/json' \
  -d '{"name":"cartpole","version":"0.1.0",
       "run_id":"<train-run-id>",
       "primary_artifact_id":"<artifact-id-of-final.pt>",
       "framework":"pytorch"}'
# → {"policy_id":"policy-uuid", "model_card_md":"...", ...}

# Get the auto-generated model card (Markdown).
curl http://localhost:7878/v1/rl/policies/<policy_id>/card

# Walk the lineage DAG (parent_run_id chain + slice-7 distill / RLHF edges).
curl http://localhost:7878/v1/rl/policies/<policy_id>/lineage?depth=3

# Create a deployment pointing at the INT8 ONNX artifact.
curl -X POST http://localhost:7878/v1/rl/serve/deployments \
  -H 'content-type: application/json' \
  -d '{"name":"cartpole-prod","artifact_id":"<artifact-id-of-final-int8.onnx>",
       "runtime":"onnx","traffic_pct":0}'

# Promote staging → canary → production.
curl -X POST http://localhost:7878/v1/rl/serve/deployments/<dep>/promote \
  -H 'content-type: application/json' \
  -d '{"to":"canary","traffic_pct":10}'
curl -X POST http://localhost:7878/v1/rl/serve/deployments/<dep>/promote \
  -H 'content-type: application/json' \
  -d '{"to":"production","traffic_pct":100}'

# Hit /act — daemon lazy-spawns `python -m vibe_rl onnx-inference`,
# routes obs through onnxruntime, returns the action.
curl -X POST http://localhost:7878/v1/rl/serve/cartpole-prod/act \
  -H 'content-type: application/json' \
  -d '{"obs":[0.0, 0.0, 0.0, 0.0]}'
# → {"action":1, "deployment":"cartpole-prod", "latency_ms":42.3}
```

On macOS, onnxruntime auto-picks the **Core ML execution provider** (Apple
Neural Engine accelerated). On Linux with CUDA installed, it picks
**CUDAExecutionProvider**. Otherwise CPU.

The native-Rust ONNX path (`--features rl-ort`) is wired but the `ort` crate dep
is currently deferred — see slice 7d in [README.md](./README.md#slice-7d). The
Python path serves real predictions today.

---

## Workflow 4 — Eval suites + paired comparison

Slice 4. Runs a trained policy across a YAML-defined suite, persists
metrics with bootstrap CIs, supports paired comparison with Cohen's d.

```bash
curl -X POST http://localhost:7878/v1/rl/eval/suites \
  -H 'content-type: application/json' \
  -d '{"name":"cartpole-robustness-v1",
       "config_yaml":"rollouts_per_env: 100\nseed_strategy: deterministic\n..."}'

curl -X POST http://localhost:7878/v1/rl/eval/compare \
  -H 'content-type: application/json' \
  -d '{"run_a":"<run-id-baseline>","run_b":"<run-id-distilled>"}'
# → {"rows":[
#     {"metric_name":"mean_return","value_a":487.3,"value_b":471.0,
#      "difference":-16.3,"effect_size":-1.2,"improved":false}, ...]}
```

---

## Workflow 5 — Multi-agent (MAPPO + VDN/QMIX + MADDPG)

Slice 7b + extras. Cooperative discrete (VDN, QMIX, MAPPO with discrete
acts) and continuous (MAPPO, MADDPG). Opt-in: `uv sync --extra marl`.

```yaml
# /tmp/mappo.yaml — cooperative on-policy
algorithm: MAPPO
environment_id: mpe2:simple_spread_v3
total_timesteps: 100000
num_steps: 128
share_actor: true
seed: 42
workspace_path: /your/workspace
```

```yaml
# /tmp/qmix.yaml — cooperative off-policy with monotonic mixer
algorithm: QMIX             # use VDN for the linear-sum mixer
environment_id: mpe2:simple_spread_v3
total_timesteps: 100000
batch_size: 128
target_update_interval: 200
epsilon_decay_steps: 50000
seed: 42
workspace_path: /your/workspace
```

```yaml
# /tmp/maddpg.yaml — continuous-action multi-agent
algorithm: MADDPG
environment_id: mpe2:simple_spread_v3
total_timesteps: 100000
actor_lr: 1.0e-4
critic_lr: 1.0e-3
exploration_noise_std: 0.1
seed: 42
workspace_path: /your/workspace
```

All four route through the daemon's standard run lifecycle. The
`RLMultiAgentView` panel reads `agents[]` + `per_agent_reward` from the
tick stream.

---

## Workflow 6 — Full RLHF (collect → RM → align)

Slice 7c + extras. Six algorithms across three patterns:

| Algorithm | Stage | Reference model? | Pairs or unpaired? | Best for |
|---|---|---|---|---|
| **REWARD_MODEL** | RM training | n/a | Pairs | Standalone reward scoring; feeds PPO/GRPO |
| **DPO** | 1-stage align | Yes (frozen) | Pairs | Production default — well-understood |
| **ORPO** | 1-stage align | **No** | Pairs | Memory-constrained (no reference) |
| **KTO** | 1-stage align | Yes (frozen) | **Unpaired** desirable/undesirable | Thumbs-up/down feedback at scale |
| **PPO** | 2-stage align | Yes + RM | Prompts (RM scores at runtime) | Classical InstructGPT recipe |
| **GRPO** | 2-stage align | Yes + RM | Prompts, group sampling | DeepSeek's value-free variant |

```bash
# 1. Collect preferences (manual or scripted).
curl -X POST http://localhost:7878/v1/rl/rlhf/preferences \
  -H 'content-type: application/json' \
  -d '{"suite_id":"helpfulness","prompt":"Q: What is 2+2?\nA:",
       "completion_a":" 4.","completion_b":" I do not know."}'

# 2. Judge them.
curl -X POST http://localhost:7878/v1/rl/rlhf/preferences/<pref_id>/judge \
  -H 'content-type: application/json' \
  -d '{"chosen":"a","rationale":"correct","reviewer":"alice"}'

# 3. Train a reward model (uses transformers — uv sync --extra rlhf).
cat > /tmp/rm.yaml <<EOF
kind: rlhf
algorithm: REWARD_MODEL
base_model_id: distilgpt2
suite_id: helpfulness
batch_size: 4
num_epochs: 3
workspace_path: /your/workspace
artifact_dir: /your/workspace/.vibecli/rl-artifacts/helpfulness-rm
EOF
uv run python -m vibe_rl train --run-id helpfulness-rm --config /tmp/rm.yaml

# 4a. Single-stage alignment (no RM needed). Pick one of DPO/ORPO/KTO.
cat > /tmp/dpo.yaml <<EOF
kind: rlhf
algorithm: DPO              # or ORPO or KTO
base_model_id: distilgpt2
beta: 0.1
batch_size: 4
num_epochs: 1
suite_id: helpfulness
workspace_path: /your/workspace
artifact_dir: /your/workspace/.vibecli/rl-artifacts/helpful-dpo
EOF
uv run python -m vibe_rl train --run-id helpful-dpo --config /tmp/dpo.yaml

# 4b. OR — 2-stage RLHF: PPO over the RM with KL penalty.
cat > /tmp/ppo-rlhf.yaml <<EOF
kind: rlhf
algorithm: PPO              # or GRPO for group-relative advantages
base_model_id: distilgpt2
reward_model_id: /your/workspace/.vibecli/rl-artifacts/helpfulness-rm/reward-model
kl_coef: 0.05
batch_size: 4
num_iterations: 100
update_epochs: 4
max_new_tokens: 64
suite_id: helpfulness
workspace_path: /your/workspace
artifact_dir: /your/workspace/.vibecli/rl-artifacts/helpful-ppo
EOF
uv run python -m vibe_rl train --run-id helpful-ppo --config /tmp/ppo-rlhf.yaml
```

Smoke results on 6 toy preferences against `distilgpt2`:

- **DPO**: `dpo_loss` 0.669 → 0.575 (decreasing), accuracy 0.5 → 1.0
- **ORPO**: `nll_loss` + `or_loss` decreasing, accuracy → 1.0
- **KTO**: `desirable_reward` positive, `undesirable_reward` negative
- **REWARD_MODEL**: `rm_loss` 1.28 → 0.63
- **PPO RLHF**: `policy_loss` 76.7 → 0.16 (clipped surrogate converging)
- **GRPO**: `policy_loss` 0.11–0.19 stable, `group_std_reward` flowing

Each persists a 313 MB safetensors aligned model under
`<artifact_dir>/aligned-model/`.

---

## Where the durable state lives

Per AGENTS.md storage rules:

```
<workspace>/
├── .vibecli/
│   ├── workspace.db                    # encrypted SQLite — all RL-OS tables
│   └── rl-artifacts/
│       └── <run_id>/
│           ├── ckpt-N.pt               # PyTorch checkpoints
│           ├── ckpt-N.json             # metadata sidecars
│           ├── final.pt
│           ├── final.onnx              # (slice 7a quantize)
│           ├── final-int8.onnx         # (slice 7a quantize)
│           ├── aligned-model/          # (slice 7c RLHF)
│           │   └── model.safetensors
│           └── reward-model/           # (slice 7c-extras RM)
```

Workspace move = full state move. Multiple workspaces = multiple
isolated RL-OS instances on one machine.

---

## What's deferred

- **Native Rust ONNX** (`--features rl-ort`): infrastructure shipped,
  `ort` dep declaration deferred pending smallvec collision resolution
  with mistralrs. Three documented paths in `vibecli/vibecli-cli/Cargo.toml`.
  Default Python ONNX path covers production today.
- **Burn/CubeCL native model architectures**: explicitly out of scope.
  ONNX (via `ort` once unblocked) is the canonical native path; Burn is
  a longer arc not justified by current goals.

Everything else from `docs/design/rl-os/` is shipped, smoke-tested, and
on `origin/main`.
