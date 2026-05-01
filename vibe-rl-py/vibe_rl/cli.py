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

    p_inf = sub.add_parser(
        "inference",
        help="long-lived inference sidecar — reads obs on stdin, writes actions on stdout (slice 6.5)",
    )
    p_inf.add_argument("--checkpoint", required=True)

    args = parser.parse_args(argv)

    if args.cmd == "train":
        return _cmd_train(args)
    if args.cmd == "eval":
        return _cmd_eval(args)
    if args.cmd == "probe-gpu":
        return _cmd_probe_gpu()
    if args.cmd == "probe-envs":
        return _cmd_probe_envs()
    if args.cmd == "inference":
        return _cmd_inference(args)

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

    # Slice 7a — dispatch by run kind. The daemon writes the kind into
    # the config_yaml at executor-spawn time; older configs (slice 2)
    # without a kind field fall through to PPO.
    kind = str(cfg.get("kind") or "train").lower()

    if kind == "distill":
        return _dispatch_distill(args.run_id, cfg, streamer)
    if kind == "quantize":
        return _dispatch_quantize(args.run_id, cfg, streamer)
    if kind == "prune":
        return _dispatch_prune(args.run_id, cfg, streamer)
    if kind == "rlhf":
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                "RLHF is not yet wired in the slice-7c sidecar. Install the [rlhf] "
                "extra with `cd vibe-rl-py && uv sync --extra rlhf` once it ships."
            ),
        )
        return 2

    # kind == "train" (or unset) → PPO baseline.
    algorithm = str(cfg.get("algorithmId") or cfg.get("algorithm") or "PPO").upper()
    if algorithm not in {"PPO"}:
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


def _resolve_parent_checkpoint(cfg: dict[str, Any]) -> str | None:
    """Resolve the source checkpoint for a distill / quantize / prune run.

    Priority:
      1. Explicit `teacher_checkpoint` / `source_checkpoint` field in cfg.
      2. `parent_run_id` (written by the daemon's executor) →
         `<workspace>/.vibecli/rl-artifacts/<parent_run_id>/final.pt`.

    Returns the absolute path if resolvable, else None (caller surfaces
    the error to the streamer).
    """
    explicit = cfg.get("teacher_checkpoint") or cfg.get("source_checkpoint")
    if explicit:
        return str(explicit)
    parent_id = cfg.get("parent_run_id")
    workspace = cfg.get("workspace_path") or "."
    if parent_id:
        candidate = Path(workspace) / ".vibecli" / "rl-artifacts" / str(parent_id) / "final.pt"
        if candidate.is_file():
            return str(candidate)
    return None


def _dispatch_distill(run_id: str, cfg: dict[str, Any], streamer) -> int:  # type: ignore[no-untyped-def]
    teacher = _resolve_parent_checkpoint(cfg)
    if not teacher:
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                "distill requires a teacher checkpoint. Set `teacher_checkpoint` in "
                "the config or `parent_run_id` to a finished training run with a "
                "final.pt artifact."
            ),
        )
        return 2

    from vibe_rl.algos.distill import DistillConfig, train

    def pick(*keys: str, default: Any = None) -> Any:
        for k in keys:
            if k in cfg and cfg[k] is not None:
                return cfg[k]
        return default

    env_id = pick("environment_id", "environmentId", "environment", "environmentName", default="CartPole-v1")
    workspace = pick("workspace_path", "workspacePath", default=".")
    artifact_dir = pick("artifact_dir", "artifactDir", default="")

    distill_cfg = DistillConfig(
        run_id=run_id,
        env_id=str(env_id),
        teacher_checkpoint=teacher,
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
        distill_coef=float(pick("distill_coef", "distillCoef", default=1.0)),
        seed=int(pick("seed", default=42)),
        workspace_path=str(workspace),
        artifact_dir=str(artifact_dir),
        checkpoint_every_steps=int(pick("checkpoint_every_steps", "checkpointEverySteps", default=50_000)),
    )
    with report_errors(streamer):
        result = train(distill_cfg, streamer)
        streamer.finished(reason="done", final_reward_mean=float(result.get("final_reward_mean", 0.0)))
    return 0


def _dispatch_quantize(run_id: str, cfg: dict[str, Any], streamer) -> int:  # type: ignore[no-untyped-def]
    source = _resolve_parent_checkpoint(cfg)
    if not source:
        streamer.started(sidecar_version=__version__, seed=0, device="cpu")
        streamer.finished(
            reason="error",
            error=(
                "quantize requires a source checkpoint. Set `source_checkpoint` "
                "in the config or `parent_run_id` to a finished training run."
            ),
        )
        return 2

    from vibe_rl.algos.quantize import QuantizeConfig, run as quantize_run

    workspace = cfg.get("workspace_path") or cfg.get("workspacePath") or "."
    artifact_dir = cfg.get("artifact_dir") or cfg.get("artifactDir") or ""
    scheme = cfg.get("scheme") or cfg.get("quantization_scheme") or "int8_dynamic"
    quant_cfg = QuantizeConfig(
        run_id=run_id,
        source_checkpoint=source,
        scheme=str(scheme),
        workspace_path=str(workspace),
        artifact_dir=str(artifact_dir),
    )
    with report_errors(streamer):
        quantize_run(quant_cfg, streamer)
    return 0


def _dispatch_prune(run_id: str, cfg: dict[str, Any], streamer) -> int:  # type: ignore[no-untyped-def]
    source = _resolve_parent_checkpoint(cfg)
    if not source:
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                "prune requires a source checkpoint. Set `source_checkpoint` "
                "in the config or `parent_run_id` to a finished training run."
            ),
        )
        return 2

    from vibe_rl.algos.prune import PruneConfig, run as prune_run

    workspace = cfg.get("workspace_path") or cfg.get("workspacePath") or "."
    artifact_dir = cfg.get("artifact_dir") or cfg.get("artifactDir") or ""
    env_id = cfg.get("environment_id") or cfg.get("environmentId") or cfg.get("environment") or cfg.get("environmentName") or "CartPole-v1"
    prune_cfg = PruneConfig(
        run_id=run_id,
        source_checkpoint=source,
        env_id=str(env_id),
        target_sparsity=float(cfg.get("target_sparsity") or cfg.get("targetSparsity") or 0.5),
        structured=bool(cfg.get("structured") or False),
        finetune_steps=int(cfg.get("finetune_steps") or cfg.get("finetuneSteps") or 0),
        workspace_path=str(workspace),
        artifact_dir=str(artifact_dir),
        seed=int(cfg.get("seed") or 42),
    )
    with report_errors(streamer):
        prune_run(prune_cfg, streamer)
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


def _cmd_inference(args: argparse.Namespace) -> int:
    from vibe_rl.inference import main as inference_main

    return inference_main(["--checkpoint", args.checkpoint])


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
