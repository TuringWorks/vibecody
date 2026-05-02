# marl-mappo

Multi-Agent PPO (MAPPO) on PettingZoo MPE `simple_spread_v3`.
Demonstrates slice 7b — centralized-critic, decentralized-actor MARL
with a shared global state observation.

## Prerequisites

```bash
cd vibe-rl-py
uv sync --extra marl   # PettingZoo + mpe2
```

Note: PettingZoo 1.26 dropped the bundled MPE; `mpe2` is the
maintained standalone package and ships in our `[marl]` extra.

## Run

```bash
./run.sh
```

## Expected output

```
{"t":"started","run_id":"mappo-001","sidecar_version":"...","seed":42,"device":"cpu"}
{"t":"tick","run_id":"...","tick":...,"timestep":2048,"payload":{"policy_loss":..., "value_loss":..., "approx_kl":..., ...}}
{"t":"episode","run_id":"...","idx":7,"timestep":...,"reward":-32.4,"length":25,...}
...
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/mappo-001/...pt","sha256":"...", ...}
{"t":"finished","run_id":"...","reason":"done","final_reward_mean":-12.7}
```

## What to look for

- **`reward` increasing from ~-40 toward ~-10** — `simple_spread`
  rewards landmark coverage with a negative distance penalty, so closer
  to zero is better. Hitting -10 to -15 within 60k steps is healthy.
- **`payload.approx_kl` staying < 0.05** — clip-ratio kicks in correctly.
- **`payload.policy_loss`** oscillating without drift — MAPPO's
  centralized critic gives the actor a steady signal in this env.
- **Per-agent metrics** — the `episode` event reports the first agent's
  reward for legibility; the centralized critic pools observations
  internally.

## Re-run

```bash
rm -rf .workspace
./run.sh
```

## Time

~60 s on a 2023-era laptop CPU. simple_spread_v3 has 3 agents, small
observation/action spaces — most wall-clock is in the rollout loop.
