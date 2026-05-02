# rlhf-dpo

Direct Preference Optimization on `distilgpt2` with 6 hand-crafted toy
preferences (polite vs. rude completions). Demonstrates slice 7c — the
DPO loss + reference-model KL term against a frozen copy of the base
model.

## Prerequisites

```bash
cd vibe-rl-py
uv sync --extra rlhf   # transformers, accelerate, peft (optional)
```

First run downloads `distilgpt2` (~330 MB) from Hugging Face Hub.
Cached afterwards.

## Run

```bash
./run.sh
```

The script first seeds 6 preferences into
`.workspace/.vibecli/workspace.db` under `suite_id=dpo-demo`, then
trains.

## Expected output

```
seeded 6 preferences into .../workspace.db (suite=dpo-demo)
{"t":"started","run_id":"dpo-001","sidecar_version":"...","seed":42,"device":"cpu"}
{"t":"tick","run_id":"...","payload":{"epoch":0,"loss":0.71,"accuracy":0.50,"kl":0.00, ...}}
{"t":"tick","run_id":"...","payload":{"epoch":1,"loss":0.45,"accuracy":0.83,"kl":0.04, ...}}
{"t":"tick","run_id":"...","payload":{"epoch":2,"loss":0.18,"accuracy":1.00,"kl":0.12, ...}}
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/dpo-001/final.pt","sha256":"...", ...}
{"t":"finished","run_id":"...","reason":"done","final_reward_mean":1.0}
```

## What to look for

- **`payload.accuracy → 1.0`** — the fraction of preferences where
  log-π(chosen) > log-π(rejected). Toy prefs are obvious enough to hit
  1.0 in 1–3 epochs.
- **`payload.loss` decreasing** monotonically. DPO loss is
  `-log σ(β · (Δ_chosen - Δ_rejected))` where Δ is log-π under the
  policy minus log-π under the reference.
- **`payload.kl` ≪ 1.0** — reference model anchor keeps the policy
  from drifting too far. Tunable via `beta` in `config.yaml`.

## Re-run

```bash
rm -rf .workspace
./run.sh
```

## Time

~30 s after the model is cached. First run downloads distilgpt2 — add
60–90 s for that.
