"""Slice 7a — Magnitude pruning + optional short fine-tune.

Pipeline:
  1. Load source PyTorch checkpoint + metadata.
  2. Apply `torch.nn.utils.prune.l1_unstructured` (or
     `ln_structured` when `structured=True`) to each Linear's weight,
     targeting `target_sparsity` (e.g. 0.5 = 50% of weights zero'd).
  3. Optionally fine-tune for `finetune_steps` env steps to recover
     reward; the fine-tune reuses slice-2's PPO loop on a reward-only
     loss (no distillation) so the pruned policy can adjust.
  4. Make the pruning permanent (`prune.remove`), save the pruned
     checkpoint, emit metrics.

`torch.nn.utils.prune` is in-core torch (no extra dep), so this works
out of the box with the slice-2 install.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn as nn
import torch.nn.utils.prune as torch_prune

from vibe_rl import runtime
from vibe_rl.algos.ppo import PPOConfig
from vibe_rl.algos.ppo import train as ppo_train
from vibe_rl.algos.ppo import _ContinuousAgent, _DiscreteAgent
from vibe_rl.checkpoint import save_checkpoint


@dataclass
class PruneConfig:
    run_id: str
    source_checkpoint: str
    env_id: str = "CartPole-v1"
    target_sparsity: float = 0.5
    structured: bool = False
    finetune_steps: int = 0
    workspace_path: str = "."
    artifact_dir: str = ""
    seed: int = 42


def _load_source(checkpoint_path: Path) -> tuple[nn.Module, dict[str, Any]]:
    metadata_path = checkpoint_path.with_suffix(".json")
    if not metadata_path.is_file():
        raise FileNotFoundError(
            f"source checkpoint metadata sidecar missing: {metadata_path}"
        )
    metadata = json.loads(metadata_path.read_text())
    obs_dim = int(metadata["obs_dim"])
    action_kind = metadata.get("action_kind", "discrete")
    if action_kind == "discrete":
        n_actions = int(metadata.get("n_actions") or metadata.get("action_n") or 2)
        agent: nn.Module = _DiscreteAgent(obs_dim, n_actions)
    else:
        action_dim = int(metadata.get("action_dim") or 1)
        agent = _ContinuousAgent(obs_dim, action_dim)
    state = torch.load(checkpoint_path, map_location="cpu", weights_only=True)
    agent.load_state_dict(state.get("policy", state), strict=False)
    return agent, metadata


def _apply_pruning(agent: nn.Module, target_sparsity: float, structured: bool) -> tuple[int, int]:
    """Apply magnitude pruning to every Linear layer. Returns (zeroed, total)."""
    zeroed = 0
    total = 0
    for module in agent.modules():
        if isinstance(module, nn.Linear):
            if structured:
                # n=2 means structured prune along output dim with L2 norm
                torch_prune.ln_structured(
                    module, name="weight", amount=target_sparsity, n=2, dim=0
                )
            else:
                torch_prune.l1_unstructured(module, name="weight", amount=target_sparsity)
            # Count zeros after the prune mask is applied.
            with torch.no_grad():
                w = module.weight
                zeroed += int((w == 0).sum().item())
                total += int(w.numel())
    return zeroed, total


def _make_permanent(agent: nn.Module) -> None:
    """Bake the pruning mask into the weight tensor so the saved
    checkpoint reflects the zeros directly."""
    for module in agent.modules():
        if isinstance(module, nn.Linear):
            try:
                torch_prune.remove(module, "weight")
            except ValueError:
                # Module wasn't pruned (e.g. critic-only path); ignore.
                pass


def run(cfg: PruneConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    streamer.started(
        sidecar_version=_sidecar_version(),
        seed=cfg.seed,
        device="cpu",
    )

    source_path = Path(cfg.source_checkpoint)
    if not source_path.is_file():
        streamer.finished(
            reason="error",
            error=f"source checkpoint not found: {cfg.source_checkpoint}",
        )
        return {"error": "source missing"}

    if not (0.0 < cfg.target_sparsity < 1.0):
        streamer.finished(
            reason="error",
            error=f"target_sparsity must be in (0, 1) — got {cfg.target_sparsity}",
        )
        return {"error": "bad sparsity"}

    agent, metadata = _load_source(source_path)
    zeroed, total = _apply_pruning(agent, cfg.target_sparsity, cfg.structured)

    # Emit a tick describing the pre-finetune state.
    streamer.tick(
        tick=1,
        timestep=0,
        payload={
            "target_sparsity": cfg.target_sparsity,
            "structured": cfg.structured,
            "weights_zeroed": zeroed,
            "weights_total": total,
            "actual_sparsity": zeroed / max(total, 1),
        },
    )

    # Optional fine-tune. We share PPO's training loop by writing a
    # snapshot of the pruned agent to disk, then calling ppo_train with
    # the snapshot as a "warm start" via the env-init seed. For slice
    # 7a we skip the warm-start plumbing and just count this as
    # one-shot pruning when finetune_steps == 0; the user can do a
    # follow-on Train run with `parent_run_id` set to the prune run if
    # they want to continue from the pruned weights.
    if cfg.finetune_steps > 0:
        streamer.tick(
            tick=2,
            timestep=0,
            payload={
                "note": (
                    "finetune_steps > 0 not yet plumbed in slice 7a — "
                    "skipping fine-tune. Run a follow-on Train with "
                    "parent_run_id set to this run for warm-start."
                ),
                "skipped_finetune_steps": cfg.finetune_steps,
            },
        )

    _make_permanent(agent)

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    )
    info = save_checkpoint(
        artifact_dir=artifact_dir,
        timestep=0,
        state={"policy": agent.state_dict()},
        metadata={
            "algorithm": "MagnitudePrune",
            "kind": "prune",
            "env_id": cfg.env_id,
            "obs_dim": int(metadata["obs_dim"]),
            "action_kind": metadata.get("action_kind", "discrete"),
            "source_checkpoint": str(source_path),
            "target_sparsity": cfg.target_sparsity,
            "structured": cfg.structured,
            "weights_zeroed": zeroed,
            "weights_total": total,
            "actual_sparsity": zeroed / max(total, 1),
            "sidecar_version": _sidecar_version(),
            "final": True,
        },
        workspace_path=Path(cfg.workspace_path),
    )
    streamer.checkpoint(
        timestep=0,
        rel_path=info.rel_path,
        sha256=info.sha256,
        size_bytes=info.size_bytes,
    )
    streamer.finished(reason="done")
    return {
        "weights_zeroed": zeroed,
        "weights_total": total,
        "actual_sparsity": zeroed / max(total, 1),
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
