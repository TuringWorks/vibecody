"""Slice 7a — INT8 dynamic quantization via ONNX Runtime.

Pipeline:
  1. Load source PyTorch checkpoint + metadata sidecar.
  2. Reconstruct the actor network (so we have a clean inference graph
     without optimizer state / value head).
  3. Export the actor to ONNX (`opset >= 13` for dynamic-quant ops).
  4. Run `onnxruntime.quantization.quantize_dynamic` to produce an INT8
     graph.
  5. Write both `final.onnx` and `final-int8.onnx` to the artifact dir,
     emit a checkpoint event for each, plus a `tick` carrying the
     compression metrics.

This is one-shot — no rollouts, no training. The streamer protocol
matches the others so the panel can render uniform results.
"""

from __future__ import annotations

import json
import math
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn as nn

from vibe_rl.algos.ppo import _ContinuousAgent, _DiscreteAgent
from vibe_rl.checkpoint import _sha256_file, save_checkpoint  # type: ignore[attr-defined]


@dataclass
class QuantizeConfig:
    run_id: str
    source_checkpoint: str
    scheme: str = "int8_dynamic"  # 'int8_dynamic' is the only one wired today
    workspace_path: str = "."
    artifact_dir: str = ""


def _build_actor_only(metadata: dict[str, Any]) -> tuple[nn.Module, str, int]:
    """Reconstruct the actor network only (no critic head) for export."""
    obs_dim = int(metadata["obs_dim"])
    action_kind = metadata.get("action_kind", "discrete")

    def linear(in_dim: int, out_dim: int, std: float = math.sqrt(2)) -> nn.Linear:
        layer = nn.Linear(in_dim, out_dim)
        nn.init.orthogonal_(layer.weight, std)
        nn.init.constant_(layer.bias, 0.0)
        return layer

    if action_kind == "discrete":
        n_actions = int(metadata.get("n_actions") or metadata.get("action_n") or 2)

        class ActorOnly(nn.Module):
            def __init__(self) -> None:
                super().__init__()
                self.actor = nn.Sequential(
                    linear(obs_dim, 64),
                    nn.Tanh(),
                    linear(64, 64),
                    nn.Tanh(),
                    linear(64, n_actions, std=0.01),
                )

            def forward(self, x: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
                return self.actor(x)

        return ActorOnly(), action_kind, obs_dim

    action_dim = int(metadata.get("action_dim") or 1)

    class ActorOnly(nn.Module):
        def __init__(self) -> None:
            super().__init__()
            self.actor_mean = nn.Sequential(
                linear(obs_dim, 64),
                nn.Tanh(),
                linear(64, 64),
                nn.Tanh(),
                linear(64, action_dim, std=0.01),
            )

        def forward(self, x: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
            return self.actor_mean(x)

    return ActorOnly(), action_kind, obs_dim


def _load_source(checkpoint_path: Path) -> tuple[nn.Module, dict[str, Any], str, int]:
    metadata_path = checkpoint_path.with_suffix(".json")
    if not metadata_path.is_file():
        raise FileNotFoundError(
            f"source checkpoint metadata sidecar missing: {metadata_path}"
        )
    metadata = json.loads(metadata_path.read_text())
    actor, action_kind, obs_dim = _build_actor_only(metadata)
    state = torch.load(checkpoint_path, map_location="cpu", weights_only=True)
    full_state = state.get("policy", state)
    # Filter to actor-related keys only — the source has actor + critic;
    # we want just the actor for inference.
    actor_state = {k: v for k, v in full_state.items() if k.startswith(("actor", "actor_mean"))}
    if not actor_state:
        actor_state = full_state  # fall through; ActorOnly will load_strict=False
    actor.load_state_dict(actor_state, strict=False)
    actor.eval()
    return actor, metadata, action_kind, obs_dim


def run(cfg: QuantizeConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    streamer.started(
        sidecar_version=_sidecar_version(),
        seed=0,
        device="cpu",  # ONNX dynamic quantize runs on CPU
    )

    if cfg.scheme != "int8_dynamic":
        streamer.finished(
            reason="error",
            error=f"scheme '{cfg.scheme}' is not yet wired — slice 7a ships int8_dynamic only",
        )
        return {"error": cfg.scheme}

    try:
        from onnxruntime.quantization import QuantType, quantize_dynamic
    except ImportError:
        streamer.finished(
            reason="error",
            error=(
                "onnxruntime not installed — slice 7a quantize requires the "
                "[opt] extra: `cd vibe-rl-py && uv sync --extra opt`"
            ),
        )
        return {"error": "onnxruntime missing"}

    source_path = Path(cfg.source_checkpoint)
    if not source_path.is_file():
        streamer.finished(
            reason="error",
            error=f"source checkpoint not found: {cfg.source_checkpoint}",
        )
        return {"error": "source missing"}

    actor, metadata, action_kind, obs_dim = _load_source(source_path)

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    )
    artifact_dir.mkdir(parents=True, exist_ok=True)

    # Step 1 — export FP32 ONNX.
    fp32_path = artifact_dir / "final.onnx"
    dummy = torch.zeros((1, obs_dim), dtype=torch.float32)
    t0 = time.monotonic()
    # `dynamo=False` forces the legacy ONNX exporter so we don't need
    # onnxscript. The legacy path is well-tested for the small MLPs slice
    # 2's PPO produces; the new dynamo path is overkill.
    torch.onnx.export(
        actor,
        dummy,
        fp32_path,
        export_params=True,
        opset_version=13,
        input_names=["obs"],
        output_names=["logits" if action_kind == "discrete" else "action_mean"],
        dynamic_axes={"obs": {0: "batch"}},
        do_constant_folding=True,
        dynamo=False,
    )
    fp32_size = fp32_path.stat().st_size
    fp32_sha = _sha256_file(fp32_path)
    fp32_rel = str(fp32_path.relative_to(Path(cfg.workspace_path).resolve())) if Path(cfg.workspace_path).resolve() in fp32_path.parents else str(fp32_path)
    streamer.checkpoint(
        timestep=0,
        rel_path=fp32_rel.replace("\\", "/"),
        sha256=fp32_sha,
        size_bytes=fp32_size,
    )

    # Step 2 — INT8 dynamic quantize.
    int8_path = artifact_dir / "final-int8.onnx"
    quantize_dynamic(
        model_input=str(fp32_path),
        model_output=str(int8_path),
        weight_type=QuantType.QInt8,
    )
    int8_size = int8_path.stat().st_size
    int8_sha = _sha256_file(int8_path)
    int8_rel = str(int8_path.relative_to(Path(cfg.workspace_path).resolve())) if Path(cfg.workspace_path).resolve() in int8_path.parents else str(int8_path)
    streamer.checkpoint(
        timestep=1,
        rel_path=int8_rel.replace("\\", "/"),
        sha256=int8_sha,
        size_bytes=int8_size,
    )

    # Sidecar metadata for the int8 artifact (mirrors slice-2's sidecar shape).
    int8_meta_path = int8_path.with_suffix(".json")
    int8_meta_path.write_text(
        json.dumps(
            {
                "kind": "quantize",
                "algorithm": "ONNX-DynamicINT8",
                "source_checkpoint": str(source_path),
                "obs_dim": obs_dim,
                "action_kind": action_kind,
                "scheme": "int8_dynamic",
                "fp32_size_bytes": fp32_size,
                "int8_size_bytes": int8_size,
                "compression_ratio": fp32_size / max(int8_size, 1),
                "sidecar_version": _sidecar_version(),
                "final": True,
            },
            indent=2,
        )
    )

    elapsed = time.monotonic() - t0
    compression_ratio = fp32_size / max(int8_size, 1)
    streamer.tick(
        tick=1,
        timestep=1,
        payload={
            "fp32_size_bytes": fp32_size,
            "int8_size_bytes": int8_size,
            "compression_ratio": compression_ratio,
            "elapsed_seconds": elapsed,
            "scheme": cfg.scheme,
        },
    )

    streamer.finished(reason="done")
    return {
        "fp32_size_bytes": fp32_size,
        "int8_size_bytes": int8_size,
        "compression_ratio": compression_ratio,
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
