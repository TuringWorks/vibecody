# vibe-rl-py — VibeCody RL Sidecar

This is the Python side of VibeCody's RL-OS productionization (slice 2 of
`docs/design/rl-os/`). The vibecli daemon spawns one of these processes per
training run; the sidecar runs CleanRL-style algorithms against Gymnasium
environments and streams metrics back over file descriptors:

- **fd-3** → JSON-Lines metric ticks (per-update aggregates)
- **fd-4** → JSON-Lines episode rows (per-episode summaries)
- **stdout/stderr** → run log, tee'd by the daemon to
  `<workspace>/.vibecli/rl-logs/<run_id>.jsonl`

## Layout

```
vibe-rl-py/
├── VERSION              # version pin Rust verifies against
├── pyproject.toml       # uv-managed
├── vibe_rl/
│   ├── __main__.py      # entry: python -m vibe_rl …
│   ├── cli.py           # subcommands (train · probe-envs · probe-gpu · …)
│   ├── runtime.py       # signal handling, fd setup, stop/cancel semantics
│   ├── streamer.py      # JSON-Lines emitter
│   ├── checkpoint.py    # PyTorch + sidecar metadata save/load
│   ├── envs/
│   │   ├── registry.py  # Gymnasium / PettingZoo / custom-Python probe
│   │   └── wrappers.py  # MonitorWrapper that emits per-step + per-episode
│   └── algos/
│       └── ppo.py       # vendored CleanRL ppo.py + ppo_continuous_action.py
└── tests/
    └── …
```

## Lifecycle

1. **Daemon startup** — daemon materializes the venv at
   `~/.vibecli/python-envs/vibe-rl-py-<VERSION>/` if missing (cached against
   the lockfile hash).
2. **Run start** — daemon writes a temporary YAML config, spawns
   `python -m vibe_rl train --run-id <id> --config <path>` with fd-3 and
   fd-4 piped, and reads JSON-Lines as the run progresses.
3. **Run stop** — daemon sends `SIGTERM`. `runtime.py` catches it, writes a
   final checkpoint, transitions to `Stopped`, exits.
4. **Run cancel** — daemon sends `SIGKILL`. No checkpoint.

See `docs/design/rl-os/02-training-executor.md` for the full design.
