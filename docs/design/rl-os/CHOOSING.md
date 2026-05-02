# Choosing the right RL-OS algorithm

A decision guide for the four areas where RL-OS gives you more than one
option. If you don't have a strong reason yet, follow the **default**
column — it's what the worked examples in `examples/rl-os/` use.

---

## 1. Single-agent training (slice 2)

Only PPO ships in slice 2. SAC / DQN / TD3 are reserved (the sidecar
emits a structured "not yet implemented" error if you ask for them).
Use **PPO** for everything that fits in a single-agent Gymnasium env.

| Algorithm | Status | When |
|---|---|---|
| **PPO** | ✅ Default | Discrete or continuous, on-policy, easy to tune |
| SAC / DQN / TD3 | ⏳ Reserved | Off-policy / replay-buffer needs — slice TBD |

---

## 2. Multi-agent training (slice 7b + 7b-extras + 7b-extras+1)

Four algorithms. The split is **discrete vs. continuous actions** and
**centralized vs. decentralized critic**.

| Algorithm | Action space | Critic | When | Default? |
|---|---|---|---|---|
| **MAPPO** | discrete or continuous | centralized (state) | Cooperative tasks with rich global state. The strongest baseline. | ✅ |
| **VDN** | discrete | summed Q | Cooperative tasks where credit assignment is purely additive. | |
| **QMIX** | discrete | monotonic mixer | Cooperative tasks where individual agents can't independently lower team Q. | |
| **MADDPG** | continuous | centralized (per-agent) with all-agent obs/actions | Mixed cooperative-competitive, deterministic policies, replay-buffer affordable. | |

### Quick triage

```
        ┌─────────────────────────────────┐
        │  Action space?                  │
        └────────┬────────────┬───────────┘
                 │ continuous │ discrete
                 ▼            ▼
              MADDPG    ┌──────────────────────┐
                        │  Cooperative?        │
                        └──┬─────────────────┬─┘
                       yes │             no  │
                           ▼                 ▼
                    ┌─────────────┐       MAPPO
                    │ Strong      │
                    │ global      │
                    │ state?      │
                    └──┬───────┬──┘
                   yes │   no  │
                       ▼       ▼
                    MAPPO   VDN/QMIX
                            (QMIX if non-additive,
                             VDN if additive)
```

### Worked example

`examples/rl-os/marl-mappo/` — MAPPO on `mpe2:simple_spread_v3`. To try
QMIX or VDN, copy that example and change `algorithm:` to `QMIX` or
`VDN`. To try MADDPG, change `environment_id:` to a continuous-action
PettingZoo env (e.g. `mpe2:simple_spread_v3` with continuous actions
enabled, or a continuous task) and `algorithm:` to `MADDPG`.

---

## 3. Alignment / RLHF (slice 7c + 7c-extras + 7c-extras+1)

Six algorithms in two families:

- **Single-stage** — train directly from preferences:
  - **DPO** (paired prefs, frozen reference)
  - **ORPO** (paired prefs, NO reference model)
  - **KTO** (unpaired desirable/undesirable, frozen reference)
- **Two-stage** — train a reward model first, then RL against it:
  - **REWARD_MODEL** → **PPO RLHF** (classical InstructGPT-style)
  - **REWARD_MODEL** → **GRPO** (group-relative, no value head)

### Triage

| Question | If yes → use |
|---|---|
| You only have unpaired thumbs-up / thumbs-down feedback | **KTO** |
| You can't afford a second model in memory at training time | **ORPO** |
| You want the simplest "preferences in, aligned model out" path | **DPO** |
| You want to score arbitrary completions with a separate model later | **REWARD_MODEL** (then DPO is moot — go to PPO RLHF or GRPO) |
| You need on-policy generation + reward shaping (not just preferences) | **PPO RLHF** |
| You're aligning a reasoning model and want stable updates without a value head | **GRPO** |

### Practical defaults

| Goal | Default | Why |
|---|---|---|
| Quick prototype on paired preferences | **DPO** | One stage, well-studied, stable |
| Memory-constrained training | **ORPO** | Drops the reference model |
| Production RLHF over a real RM | **REWARD_MODEL → PPO RLHF** | Industry baseline; what InstructGPT shipped |
| Reasoning-model alignment | **REWARD_MODEL → GRPO** | Group-relative scoring removes the value head's variance |
| Asymmetric feedback signal | **KTO** | Doesn't require pairing |

### Memory & speed

| Algorithm | Models in VRAM | Per-step cost | Notes |
|---|---|---|---|
| DPO | policy + frozen reference | 2× forward + 1× backward | The reference can be the same checkpoint as the policy at step 0; no separate download. |
| ORPO | policy only | 1× forward + 1× backward | Half the memory. Tradeoff: quality is comparable on small datasets, often slightly worse on large ones vs. DPO. |
| KTO | policy + frozen reference | 2× forward + 1× backward | Same memory as DPO. Loss is asymmetric (desirable + undesirable terms). |
| REWARD_MODEL | policy backbone + scalar head | 1× forward + 1× backward | Cheap. Run once, reuse. |
| PPO RLHF | policy + reference + reward model + value head | generate + 4× forward + 1× backward | Heaviest. Largest gains on hard tasks. |
| GRPO | policy + reference + reward model | generate + 3× forward + 1× backward | No value head. Slightly lighter than PPO RLHF; often better on reasoning. |

### Worked examples

- `examples/rl-os/rlhf-dpo/` — DPO end-to-end with toy preferences.
- `examples/rl-os/rlhf-full-stack/` — REWARD_MODEL → PPO RLHF.

To try the others, copy `rlhf-dpo` and change `algorithm:` in the
config:

| Variant | Edit |
|---|---|
| ORPO | `algorithm: ORPO` (drop `reference_model_id`); add `lambda: 0.1` |
| KTO | `algorithm: KTO`; add `lambda_d: 1.0`, `lambda_u: 1.0` |
| GRPO | Use `rlhf-full-stack` and change Stage 2 `algorithm:` to `GRPO`; add `group_size: 4` |

---

## 4. Optimization (slice 7a)

Three techniques. They compose — run them in order if you want all
three benefits.

| Technique | What it changes | Typical wins | When |
|---|---|---|---|
| **distill** | Trains a student policy to imitate a teacher policy's actions/logits | 2–10× smaller policy, similar reward | Inference cost matters more than training cost; teacher is too big to ship. |
| **quantize** | Exports to ONNX, applies INT8 dynamic quantization to weights | 3–4× smaller artifact, minor latency win | Disk / network footprint matters; CPU inference target. |
| **prune** | Zeros out weights with smallest magnitude (or whole channels) | 2–3× sparsity with minor reward drop | Inference framework can exploit sparsity; or you're chaining into quantize. |

### Order of operations

```
   teacher policy
        │
        ▼
   ┌──────────┐    optional   ┌──────────┐    optional   ┌──────────┐
   │ distill  │──────────────▶│  prune   │──────────────▶│ quantize │
   └──────────┘               └──────────┘               └──────────┘
                                                              │
                                                              ▼
                                                    final.onnx (servable)
```

- **Distill first** — pruning a too-large model wastes compute; train
  a smaller student instead.
- **Prune before quantize** — quantization re-encodes weights; pruning
  before lets the quantizer see the actual sparsity pattern and pack
  zero-weights efficiently.
- **Quantize last** — ONNX export is one-way; you can't fine-tune an
  INT8 ONNX model.

### Worked example

`examples/rl-os/quantize-pipeline/` — INT8 dynamic quantization of the
cartpole-baseline checkpoint. Distill and prune chain the same way:
set `kind: distill` (with a `teacher_checkpoint`) or `kind: prune`
(with a `source_checkpoint`).

---

## 5. Deployment runtime (slice 6 + 6.5)

Three options. Pick by **artifact format** and **performance**.

| Runtime | Artifact | Latency (cartpole-class MLP, M-series CPU) | When | Status |
|---|---|---|---|---|
| **python** | `final.pt` (PyTorch) | ~3–6 ms / step | You haven't quantized yet, or you need exotic PyTorch ops at inference time. | ✅ Ships |
| **onnx** (Python sidecar) | `final.onnx` | ~0.5–1.5 ms / step | You ran quantize-pipeline and want the smaller artifact + faster inference. | ✅ Ships |
| **onnx-rust** (in-process) | `final.onnx` | < 0.2 ms / step (no IPC) | Latency-critical hot loop in the daemon; no Python startup cost. | ⏳ Feature-flagged (`rl-ort`); deferred until the smallvec / mistralrs version pin clears |

### Triage

```
Is your artifact final.pt or final.onnx?
  final.pt   → runtime: python
  final.onnx → runtime: onnx (always works)
                 │
                 ▼
              Latency-critical (sub-ms inside the daemon)?
                 │
              ┌──┴──┐
              │ yes │ → runtime: onnx-rust (when feature lands)
              └─────┘
              ┌──┐
              │ no │ → runtime: onnx
              └────┘
```

### Wiring it up

When you `rl-deploy register`, set the `runtime` field on the
deployment row:

```yaml
runtime: python   # or "onnx" or "onnx-rust"
```

The daemon's `RuntimePool` routes `/v1/rl/deployments/<id>/act` calls
to the matching runtime. See
[06-deployment.md](./06-deployment.md) for the full wiring.

---

## See also

- [QUICKSTART.md](./QUICKSTART.md) — six concrete workflows from PPO to RLHF
- [README.md](./README.md) — slice-by-slice index of the design docs
- `examples/rl-os/` — five copy-and-run examples covering this matrix
