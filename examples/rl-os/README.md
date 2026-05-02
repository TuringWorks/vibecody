# RL-OS Worked Examples

Five copy-and-run examples covering the full pipeline. Each example is
self-contained: a `run.sh`, a `config.yaml`, and a per-example README
explaining what to look for.

| # | Example | Demonstrates | Time | Extras needed |
|---|---|---|---|---|
| 1 | [cartpole-baseline](./cartpole-baseline/) | Slice 2 PPO + slice 1 persistence | ~30 s | none (base install) |
| 2 | [quantize-pipeline](./quantize-pipeline/) | Slice 7a INT8 ONNX quantization on the cartpole checkpoint | ~10 s | `--extra opt` |
| 3 | [marl-mappo](./marl-mappo/) | Slice 7b MAPPO on PettingZoo MPE simple_spread | ~60 s | `--extra marl` |
| 4 | [rlhf-dpo](./rlhf-dpo/) | Slice 7c DPO over distilgpt2 + 6 toy preferences | ~30 s (after model download) | `--extra rlhf` |
| 5 | [rlhf-full-stack](./rlhf-full-stack/) | Slice 7c-extras: REWARD_MODEL → PPO RLHF chain | ~3 min | `--extra rlhf` |

## Prerequisites

```bash
# One-time: install the sidecar venv.
cd /path/to/vibecody/vibe-rl-py
uv sync

# Install the extras you need (per the table above).
uv sync --extra opt --extra marl --extra rlhf
```

## How the examples work

Each `run.sh` is self-locating — it figures out the repo root from its
own path and points the sidecar at a workspace under the example
directory. Re-running the same example overwrites the previous
artifacts. To wipe state:

```bash
rm -rf examples/rl-os/<example-name>/.workspace
```

## Reading the output

Each script tails the JSON-Lines stream the sidecar emits. The schema
is consistent across slices:

- `{"t":"started","run_id":...,"sidecar_version":...,"seed":...,"device":...}` — sidecar launched, device detected
- `{"t":"tick","run_id":...,"tick":...,"timestep":...,"wall":...,"payload":{...}}` — periodic training metrics, nested under `payload`
- `{"t":"episode","run_id":...,"idx":...,"timestep":...,"reward":...,"length":...,"duration_ms":...}` — single-episode summary (single-agent or first agent in MARL)
- `{"t":"checkpoint","run_id":...,"rel_path":...,"sha256":...,"size_bytes":...}` — artifact persisted with sha256
- `{"t":"finished","run_id":...,"reason":"done","final_reward_mean":...}` — clean termination

Each example's per-run README calls out the specific fields to watch
(e.g. `policy_loss` decreasing, `compression_ratio` for quantize,
`accuracy → 1.0` for DPO).

## Picking the right algorithm

If you don't know which alignment / MARL algorithm to use, see
[CHOOSING.md](../../docs/design/rl-os/CHOOSING.md).
