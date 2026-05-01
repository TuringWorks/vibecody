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

    p_onnx = sub.add_parser(
        "onnx-inference",
        help="long-lived ONNX inference sidecar — same protocol as `inference` but via onnxruntime",
    )
    p_onnx.add_argument("--model", required=True)

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
    if args.cmd == "onnx-inference":
        return _cmd_onnx_inference(args)

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
        return _dispatch_rlhf(args.run_id, cfg, streamer)

    # kind == "train" (or unset) → dispatch by algorithm.
    algorithm = str(cfg.get("algorithmId") or cfg.get("algorithm") or "PPO").upper()
    if algorithm == "MAPPO":
        return _dispatch_mappo(args.run_id, cfg, streamer)
    if algorithm in {"QMIX", "VDN", "MADDPG"}:
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                f"algorithm '{algorithm}' is not yet implemented. Slice 7b ships MAPPO; "
                f"QMIX / VDN / MADDPG follow in 7b-extras (open an issue or implement on top)."
            ),
        )
        return 2
    if algorithm not in {"PPO"}:
        streamer.started(sidecar_version=__version__, seed=int(cfg.get("seed", 42)), device="cpu")
        streamer.finished(
            reason="error",
            error=(
                f"algorithm '{algorithm}' is not yet implemented in the slice-2 sidecar. "
                f"Slice 2 ships PPO; SAC/DQN/TD3 follow. Multi-agent (slice 7b) ships MAPPO."
            ),
        )
        return 2

    from vibe_rl.algos.ppo import PPOConfig, train

    ppo_cfg = _ppo_config_from_yaml(args.run_id, cfg)
    with report_errors(streamer):
        result = train(ppo_cfg, streamer)
        streamer.finished(reason="done", final_reward_mean=float(result.get("final_reward_mean", 0.0)))
    return 0


def _dispatch_rlhf(run_id: str, cfg: dict[str, Any], streamer) -> int:  # type: ignore[no-untyped-def]
    """Slice 7c + 7c-extras — RLHF dispatch.

    Single-stage methods (no separate reward-model stage):
      - DPO    — paired preferences, frozen reference (slice 7c)
      - ORPO   — paired preferences, NO reference model (extras)
      - KTO    — unpaired desirable/undesirable, frozen reference (extras)

    Reward-modeling stage:
      - REWARD_MODEL — trains a scalar BCE-Bradley-Terry head over an LM
                       backbone; produces a model usable for ranking or
                       PPO RLHF (extras+1).

    Reserved (require a separately-trained reward model — not yet wired):
      - PPO    — classical InstructGPT-style RLHF over the RM
      - GRPO   — GRPO from DeepSeek's R1 line of work
    """
    algorithm = str(cfg.get("algorithm") or cfg.get("algorithmId") or "DPO").upper()

    def pick(*keys: str, default: Any = None) -> Any:
        for k in keys:
            if k in cfg and cfg[k] is not None:
                return cfg[k]
        return default

    workspace = pick("workspace_path", "workspacePath", default=".")
    artifact_dir = pick("artifact_dir", "artifactDir", default="")
    base_model = str(pick("base_model_id", "baseModelId", default="distilgpt2"))
    seed = int(pick("seed", default=42))
    batch_size = int(pick("batch_size", "batchSize", default=4))
    lr = float(pick("learning_rate", "learningRate", default=5e-6))
    num_epochs = int(pick("num_epochs", "numEpochs", default=1))
    grad_accum = int(pick("grad_accum_steps", "gradAccumSteps", default=1))
    max_seq = int(pick("max_seq_len", "maxSeqLen", default=256))
    suite_id = pick("suite_id", "suiteId", default=None)

    if algorithm == "DPO":
        from vibe_rl.algos.dpo import DPOConfig, train

        dpo_cfg = DPOConfig(
            run_id=run_id,
            base_model_id=base_model,
            reference_model_id=pick("reference_model_id", "referenceModelId", default=None),
            beta=float(pick("beta", default=0.1)),
            max_seq_len=max_seq,
            batch_size=batch_size,
            learning_rate=lr,
            num_epochs=num_epochs,
            grad_accum_steps=grad_accum,
            seed=seed,
            suite_id=suite_id,
            workspace_path=str(workspace),
            artifact_dir=str(artifact_dir),
        )
        with report_errors(streamer):
            result = train(dpo_cfg, streamer)
            streamer.finished(reason="done", final_reward_mean=float(result.get("best_accuracy", 0.0)))
        return 0

    if algorithm == "ORPO":
        from vibe_rl.algos.orpo import ORPOConfig, train

        orpo_cfg = ORPOConfig(
            run_id=run_id,
            base_model_id=base_model,
            lambda_=float(pick("lambda", "lambda_", default=0.1)),
            max_seq_len=max_seq,
            batch_size=batch_size,
            learning_rate=lr,
            num_epochs=num_epochs,
            grad_accum_steps=grad_accum,
            seed=seed,
            suite_id=suite_id,
            workspace_path=str(workspace),
            artifact_dir=str(artifact_dir),
        )
        with report_errors(streamer):
            result = train(orpo_cfg, streamer)
            streamer.finished(reason="done", final_reward_mean=float(result.get("best_accuracy", 0.0)))
        return 0

    if algorithm == "KTO":
        from vibe_rl.algos.kto import KTOConfig, train

        kto_cfg = KTOConfig(
            run_id=run_id,
            base_model_id=base_model,
            reference_model_id=pick("reference_model_id", "referenceModelId", default=None),
            beta=float(pick("beta", default=0.1)),
            lambda_d=float(pick("lambda_d", "lambdaD", default=1.0)),
            lambda_u=float(pick("lambda_u", "lambdaU", default=1.0)),
            max_seq_len=max_seq,
            batch_size=batch_size,
            learning_rate=lr,
            num_epochs=num_epochs,
            grad_accum_steps=grad_accum,
            seed=seed,
            suite_id=suite_id,
            workspace_path=str(workspace),
            artifact_dir=str(artifact_dir),
        )
        with report_errors(streamer):
            result = train(kto_cfg, streamer)
            streamer.finished(reason="done", final_reward_mean=float(result.get("best_accuracy", 0.0)))
        return 0

    if algorithm in {"REWARD_MODEL", "RM"}:
        from vibe_rl.algos.reward_model import RewardModelConfig, train

        rm_cfg = RewardModelConfig(
            run_id=run_id,
            base_model_id=base_model,
            max_seq_len=max_seq,
            batch_size=batch_size,
            learning_rate=float(pick("learning_rate", "learningRate", default=1e-5)),
            num_epochs=num_epochs,
            grad_accum_steps=grad_accum,
            seed=seed,
            suite_id=suite_id,
            workspace_path=str(workspace),
            artifact_dir=str(artifact_dir),
        )
        with report_errors(streamer):
            result = train(rm_cfg, streamer)
            streamer.finished(reason="done", final_reward_mean=float(result.get("best_accuracy", 0.0)))
        return 0

    if algorithm in {"PPO", "GRPO"}:
        streamer.started(sidecar_version=__version__, seed=seed, device="cpu")
        streamer.finished(
            reason="error",
            error=(
                f"RLHF algorithm '{algorithm}' is not yet wired. It requires "
                f"a separately-trained reward model (set algorithm: REWARD_MODEL "
                f"first), then a token-level PPO loop with KL penalty. Use "
                f"algorithm: DPO / ORPO / KTO for single-stage alignment instead."
            ),
        )
        return 2

    streamer.started(sidecar_version=__version__, seed=seed, device="cpu")
    streamer.finished(
        reason="error",
        error=(
            f"unknown RLHF algorithm '{algorithm}'. "
            f"Available: DPO, ORPO, KTO, REWARD_MODEL."
        ),
    )
    return 2


def _dispatch_mappo(run_id: str, cfg: dict[str, Any], streamer) -> int:  # type: ignore[no-untyped-def]
    from vibe_rl.algos.mappo import MAPPOConfig, train

    def pick(*keys: str, default: Any = None) -> Any:
        for k in keys:
            if k in cfg and cfg[k] is not None:
                return cfg[k]
        return default

    env_id = pick("environment_id", "environmentId", "environment", "environmentName", default="pettingzoo:simple_spread_v3")
    workspace = pick("workspace_path", "workspacePath", default=".")
    artifact_dir = pick("artifact_dir", "artifactDir", default="")

    mappo_cfg = MAPPOConfig(
        run_id=run_id,
        env_id=str(env_id),
        total_timesteps=int(pick("total_timesteps", "totalTimesteps", default=200_000)),
        learning_rate=float(pick("learning_rate", "learningRate", default=3e-4)),
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
        share_actor=bool(pick("share_actor", "shareActor", default=False)),
        seed=int(pick("seed", default=42)),
        workspace_path=str(workspace),
        artifact_dir=str(artifact_dir),
        checkpoint_every_steps=int(pick("checkpoint_every_steps", "checkpointEverySteps", default=50_000)),
    )
    with report_errors(streamer):
        result = train(mappo_cfg, streamer)
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


def _cmd_onnx_inference(args: argparse.Namespace) -> int:
    from vibe_rl.onnx_inference import main as onnx_main

    return onnx_main(["--model", args.model])


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
