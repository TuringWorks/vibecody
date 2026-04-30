"""Subcommand dispatcher.

Subcommands:
    train        — run a training algorithm against a YAML config
    probe-envs   — list installed Gymnasium / PettingZoo envs (slice 3)
    probe-gpu    — report CUDA / MPS / CPU detection (slice 2)
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import yaml  # type: ignore[import-untyped]

from vibe_rl import __version__
from vibe_rl.runtime import install_signal_handlers, report_errors
from vibe_rl.streamer import Streamer


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m vibe_rl")
    parser.add_argument("--version", action="version", version=__version__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_train = sub.add_parser("train", help="run a training run")
    p_train.add_argument("--run-id", required=True)
    p_train.add_argument(
        "--config",
        required=True,
        help="path to a YAML file matching the schema below; produced by the daemon per run",
    )

    p_eval = sub.add_parser("eval", help="run an evaluation rollout (slice 4)")
    p_eval.add_argument("--run-id", required=True)
    p_eval.add_argument("--config", required=True)

    sub.add_parser("probe-gpu", help="emit one JSON line describing local accelerators")
    sub.add_parser("probe-envs", help="emit JSON describing installed Gymnasium envs")

    args = parser.parse_args(argv)

    if args.cmd == "train":
        return _cmd_train(args)
    if args.cmd == "eval":
        return _cmd_eval(args)
    if args.cmd == "probe-gpu":
        return _cmd_probe_gpu()
    if args.cmd == "probe-envs":
        return _cmd_probe_envs()

    parser.print_help()
    return 2


def _load_yaml(path: str) -> dict[str, Any]:
    p = Path(path)
    if not p.is_file():
        raise FileNotFoundError(path)
    return yaml.safe_load(p.read_text()) or {}


def _cmd_train(args: argparse.Namespace) -> int:
    install_signal_handlers()
    streamer = Streamer(args.run_id)
    cfg = _load_yaml(args.config)

    algorithm = str(cfg.get("algorithmId") or cfg.get("algorithm") or "PPO").upper()
    if algorithm not in {"PPO"}:
        # Slice 2 ships PPO only — SAC / DQN / TD3 land alongside in
        # follow-on patches. The daemon already records the algorithm
        # choice on the run row; we surface the gap here so the dashboard
        # shows an honest message instead of training the wrong thing.
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                f"algorithm '{algorithm}' is not yet implemented in the slice-2 sidecar. "
                f"Slice 2 ships PPO; SAC/DQN/TD3 follow."
            ),
        )
        return 2

    from vibe_rl.algos.ppo import PPOConfig, train

    ppo_cfg = _ppo_config_from_yaml(args.run_id, cfg)
    with report_errors(streamer):
        result = train(ppo_cfg, streamer)
        streamer.finished(reason="done", final_reward_mean=float(result.get("final_reward_mean", 0.0)))
    return 0


def _cmd_eval(args: argparse.Namespace) -> int:
    """Slice-4 placeholder. Emits a structured 'not yet implemented' message."""
    install_signal_handlers()
    streamer = Streamer(args.run_id)
    streamer.started(sidecar_version=__version__, seed=0, device="cpu")
    streamer.finished(
        reason="error",
        error="eval rollouts ship in slice 4 (docs/design/rl-os/04-evaluation.md)",
    )
    return 2


def _cmd_probe_gpu() -> int:
    info: dict[str, Any] = {"cuda": False, "cuda_devices": [], "mps": False, "rocm": False}
    try:
        import torch

        info["cuda"] = torch.cuda.is_available()
        if info["cuda"]:
            info["cuda_devices"] = [
                {
                    "name": torch.cuda.get_device_name(i),
                    "memory_mb": int(torch.cuda.get_device_properties(i).total_memory / 1024 / 1024),
                }
                for i in range(torch.cuda.device_count())
            ]
        if hasattr(torch.backends, "mps"):
            info["mps"] = bool(torch.backends.mps.is_available())
        if hasattr(torch.version, "hip") and torch.version.hip:
            info["rocm"] = True
    except Exception as e:  # noqa: BLE001 — torch may be unimportable in dev shells
        info["error"] = f"{type(e).__name__}: {e}"
    print(json.dumps(info, separators=(",", ":")))
    return 0


def _cmd_probe_envs() -> int:
    from vibe_rl.envs.registry import probe_gymnasium

    print(json.dumps(probe_gymnasium(), separators=(",", ":")))
    return 0


def _ppo_config_from_yaml(run_id: str, cfg: dict[str, Any]):  # type: ignore[no-untyped-def]
    """Translate the daemon's `config_yaml` blob into PPOConfig fields.

    The daemon serializes the dashboard's `TrainRunConfig` (camelCase
    keys) to YAML verbatim, so we map both camelCase (dashboard origin)
    and snake_case (CLI/CI origin).
    """
    from vibe_rl.algos.ppo import PPOConfig

    def pick(*keys: str, default=None):  # type: ignore[no-untyped-def]
        for k in keys:
            if k in cfg and cfg[k] is not None:
                return cfg[k]
        return default

    env_id = pick("environment_id", "environmentId", "environment", "environmentName", default="CartPole-v1")
    workspace = pick("workspace_path", "workspacePath", default=".")
    artifact_dir = pick("artifact_dir", "artifactDir", default="")
    return PPOConfig(
        run_id=run_id,
        env_id=str(env_id),
        total_timesteps=int(pick("total_timesteps", "totalTimesteps", default=100_000)),
        learning_rate=float(pick("learning_rate", "learningRate", default=3e-4)),
        num_envs=int(pick("num_envs", "numEnvs", default=4)),
        num_steps=int(pick("num_steps", "numSteps", default=128)),
        anneal_lr=bool(pick("anneal_lr", "annealLr", default=True)),
        gamma=float(pick("gamma", default=0.99)),
        gae_lambda=float(pick("gae_lambda", "gaeLambda", default=0.95)),
        num_minibatches=int(pick("num_minibatches", "numMinibatches", default=4)),
        update_epochs=int(pick("update_epochs", "updateEpochs", default=4)),
        clip_coef=float(pick("clip_coef", "clipCoef", default=0.2)),
        ent_coef=float(pick("ent_coef", "entCoef", default=0.01)),
        vf_coef=float(pick("vf_coef", "vfCoef", default=0.5)),
        max_grad_norm=float(pick("max_grad_norm", "maxGradNorm", default=0.5)),
        target_kl=pick("target_kl", "targetKl", default=None),
        seed=int(pick("seed", default=42)),
        workspace_path=str(workspace),
        artifact_dir=str(artifact_dir),
        checkpoint_every_steps=int(pick("checkpoint_every_steps", "checkpointEverySteps", default=50_000)),
    )
