# cartpole-baseline

PPO on `CartPole-v1` — the simplest worked example. Demonstrates slice 2
(PPO training loop) writing to slice 1 (run/episode/metric persistence)
under an example-local workspace.

## Run

```bash
./run.sh
```

Tails the JSON-Lines event stream from the sidecar.

## Expected output

```
{"t":"started","run_id":"cartpole-baseline-001","sidecar_version":"...","seed":42,"device":"cpu"}
{"t":"tick","run_id":"...","tick":1,"timestep":1024,"wall":...,"payload":{"policy_loss":..., "value_loss":..., "entropy":..., "approx_kl":..., "sps":..., "recent_reward_mean_100":...}}
{"t":"episode","run_id":"...","idx":17,"timestep":...,"reward":42.0,"length":42,"success":null,"duration_ms":...}
...
{"t":"checkpoint","run_id":"...","timestep":48128,"rel_path":".vibecli/rl-artifacts/cartpole-baseline-001/ckpt-48128.pt","sha256":"...","size_bytes":124341}
{"t":"finished","run_id":"...","reason":"done","final_reward_mean":54.0}
```

## What to look for

- **`payload.policy_loss`** small and oscillating (clip-ratio is doing
  its job). Magnitudes in the 1e-4 to 1e-3 band are healthy for CartPole.
- **`reward` per episode trending up** toward 200 (CartPole's max
  episode length). 50k steps gets you partway; bump
  `total_timesteps` in `config.yaml` if you want full convergence.
- **`payload.recent_reward_mean_100`** is the smoothed signal — easier
  to read than per-episode `reward`. Should climb monotonically.
- **`payload.approx_kl`** staying < 0.05 — clip-ratio kicks in correctly.
- **`final.pt` artifact** at
  `.workspace/.vibecli/rl-artifacts/cartpole-baseline-001/final.pt` —
  consumed by `quantize-pipeline` next.

## Re-run

```bash
rm -rf .workspace
./run.sh
```

## Time

~30 s on a 2023-era laptop CPU (M-series, recent x86, etc).
