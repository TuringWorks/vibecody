# rlhf-full-stack

The full classical RLHF pipeline:

1. **REWARD_MODEL** — train a scalar Bradley-Terry head on top of
   `distilgpt2` using 6 toy preferences (slice 7c-extras+1).
2. **PPO RLHF** — generate completions from the policy, score them with
   the trained RM, optimize via PPO with a frozen-reference KL penalty
   (slice 7c-extras+1).

## Prerequisites

```bash
cd vibe-rl-py
uv sync --extra rlhf
```

First run downloads `distilgpt2`. Cached afterwards.

## Run

```bash
./run.sh
```

The script seeds preferences, runs Stage 1 (RM), then Stage 2 (PPO
RLHF) using the RM's `final.pt` as the reward function.

## Expected output (Stage 1 — RM)

```
{"t":"started","run_id":"rm-001",...,"device":"cpu"}
{"t":"tick","run_id":"...","payload":{"stage":"reward_model","epoch":0,"loss":0.69,"accuracy":0.50,"sps":...}}
{"t":"tick","run_id":"...","payload":{"epoch":2,"loss":0.20,"accuracy":1.00, ...}}
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/rm-001/final.pt", ...}
{"t":"finished","run_id":"...","reason":"done","final_reward_mean":1.0}
```

## Expected output (Stage 2 — PPO RLHF)

```
{"t":"started","run_id":"ppo-rlhf-001",...,"device":"cpu"}
{"t":"tick","run_id":"...","payload":{"stage":"ppo_rlhf","iteration":0,"kl":0.0,"policy_loss":..., "value_loss":..., "reward_mean":-0.34, ...}}
{"t":"tick","run_id":"...","payload":{"iteration":1,"kl":0.05,"reward_mean":0.12, ...}}
{"t":"tick","run_id":"...","payload":{"iteration":2,"kl":0.09,"reward_mean":0.41, ...}}
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/ppo-rlhf-001/final.pt", ...}
{"t":"finished","run_id":"...","reason":"done","final_reward_mean":0.41}
```

## What to look for

### RM stage

- **`payload.accuracy → 1.0`** — RM ranks the preferred completion
  above the rejected one for every pair.
- **`payload.loss` drops** monotonically (BCE-Bradley-Terry is convex
  in the separation, easy to fit on a tiny dataset).

### PPO RLHF stage

- **`payload.reward_mean` increasing** — the RM scores the policy's
  generated completions higher each iteration as PPO pushes the
  policy toward preferred outputs.
- **`payload.kl` rising slowly** but bounded — the KL penalty against
  the frozen reference model prevents catastrophic drift; tune via
  `kl_coef`.
- **`payload.policy_loss`** & **`payload.value_loss`** behave like
  classical PPO over language-model logits.

## Re-run

```bash
rm -rf .workspace
./run.sh
```

## Time

~3 min on CPU after the model is cached:
- RM stage: ~30 s (3 epochs × 6 prefs)
- PPO RLHF stage: ~2–3 min (3 iterations × generation + RM scoring + PPO updates)

## Variants

This pipeline is the baseline; once it works you can swap the second
stage for **GRPO** (group-relative — no value head, no per-token KL).
See `docs/design/rl-os/CHOOSING.md` for guidance on which alignment
algorithm fits your setup.
