"""Checkpoint save/load — PyTorch state_dict + sidecar metadata.

Layout (per run):
    <workspace>/.vibecli/rl-artifacts/<run_id>/
        ckpt-<timestep>.pt        # torch.save({"policy": ..., "value": ..., "optimizer": ...})
        ckpt-<timestep>.json      # {"timestep", "algorithm", "env_id", ...}
        final.pt                  # symlink or copy of the last checkpoint
        final.json                # symlink or copy of the last metadata

Slice 5 (model hub) registers these as `Policy` rows; slice 6 (deployment)
loads them via the runtime trait. For slice 2 we just write them.
"""

from __future__ import annotations

import hashlib
import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class CheckpointInfo:
    rel_path: str
    sha256: str
    size_bytes: int


def save_checkpoint(
    *,
    artifact_dir: Path,
    timestep: int,
    state: dict[str, Any],
    metadata: dict[str, Any],
    workspace_path: Path,
) -> CheckpointInfo:
    """Save a checkpoint and its metadata; return the workspace-relative path + sha256."""

    artifact_dir.mkdir(parents=True, exist_ok=True)
    pt_path = artifact_dir / f"ckpt-{timestep}.pt"
    json_path = artifact_dir / f"ckpt-{timestep}.json"

    # torch import is deferred so unit tests that don't need torch can run.
    import torch

    torch.save(state, pt_path)
    metadata_with_step = {**metadata, "timestep": timestep}
    json_path.write_text(json.dumps(metadata_with_step, indent=2))

    # Update final.pt / final.json to point at the latest snapshot. We use
    # copies on Windows (no symlinks) and on macOS/Linux too, because the
    # daemon might pack the artifact tree later and symlinks complicate
    # that. The duplicate cost is small relative to checkpoint sizes.
    final_pt = artifact_dir / "final.pt"
    final_json = artifact_dir / "final.json"
    shutil.copyfile(pt_path, final_pt)
    shutil.copyfile(json_path, final_json)

    sha = _sha256_file(pt_path)
    size = pt_path.stat().st_size

    rel = pt_path.resolve().relative_to(workspace_path.resolve())
    return CheckpointInfo(rel_path=str(rel).replace("\\", "/"), sha256=sha, size_bytes=size)


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(64 * 1024):
            h.update(chunk)
    return h.hexdigest()
