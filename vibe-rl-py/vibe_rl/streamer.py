"""JSON-Lines stream emitter.

Two file descriptors:
- fd 3 → metric ticks (`{"t":"tick", ...}`, `{"t":"started",...}`, `{"t":"finished",...}`)
- fd 4 → episode rows (`{"t":"episode", ...}`)

If either fd is not open (e.g. running standalone for debugging), we fall
back to writing to stdout. The daemon-spawned variant always opens both.

Lines are flushed immediately so the daemon's metric reader sees them
without waiting on stdio buffering. This is the per-line cost; ticks are
already batched on the producer side (algorithm-level) so we are not
emitting thousands of lines per second.
"""

from __future__ import annotations

import json
import os
import sys
import time
from typing import IO, Any


def _open_fd(fd: int) -> IO[str] | None:
    try:
        # `closefd=False` so closing this wrapper doesn't close the underlying fd.
        return os.fdopen(fd, "w", buffering=1, closefd=False)
    except OSError:
        return None


class Streamer:
    def __init__(self, run_id: str) -> None:
        self.run_id = run_id
        self._tick_fd: IO[str] | None = _open_fd(3) or sys.stdout
        self._episode_fd: IO[str] | None = _open_fd(4) or sys.stdout

    # ── Tick stream (fd 3) ────────────────────────────────────────────────

    def started(self, *, sidecar_version: str, seed: int, device: str) -> None:
        self._emit_tick(
            {
                "t": "started",
                "run_id": self.run_id,
                "wall": _now_ms(),
                "sidecar_version": sidecar_version,
                "seed": seed,
                "device": device,
            }
        )

    def tick(self, *, tick: int, timestep: int, payload: dict[str, Any]) -> None:
        self._emit_tick(
            {
                "t": "tick",
                "run_id": self.run_id,
                "tick": tick,
                "timestep": timestep,
                "wall": _now_ms(),
                "payload": payload,
            }
        )

    def gpu(self, *, util: list[float], mem_mb: list[int]) -> None:
        self._emit_tick(
            {
                "t": "gpu",
                "run_id": self.run_id,
                "wall": _now_ms(),
                "util": util,
                "mem_mb": mem_mb,
            }
        )

    def checkpoint(self, *, timestep: int, rel_path: str, sha256: str, size_bytes: int) -> None:
        self._emit_tick(
            {
                "t": "checkpoint",
                "run_id": self.run_id,
                "timestep": timestep,
                "rel_path": rel_path,
                "sha256": sha256,
                "size_bytes": size_bytes,
            }
        )

    def finished(
        self,
        *,
        reason: str,
        final_reward_mean: float | None = None,
        error: str | None = None,
    ) -> None:
        out: dict[str, Any] = {
            "t": "finished",
            "run_id": self.run_id,
            "wall": _now_ms(),
            "reason": reason,
        }
        if final_reward_mean is not None:
            out["final_reward_mean"] = final_reward_mean
        if error is not None:
            out["error"] = error
        self._emit_tick(out)

    # ── Episode stream (fd 4) ─────────────────────────────────────────────

    def episode(
        self,
        *,
        idx: int,
        timestep: int,
        reward: float,
        length: int,
        success: bool | None,
        duration_ms: int,
    ) -> None:
        self._emit_episode(
            {
                "t": "episode",
                "run_id": self.run_id,
                "idx": idx,
                "timestep": timestep,
                "reward": reward,
                "length": length,
                "success": success,
                "duration_ms": duration_ms,
            }
        )

    # ── Internal ──────────────────────────────────────────────────────────

    def _emit_tick(self, obj: dict[str, Any]) -> None:
        if self._tick_fd is None:
            return
        try:
            self._tick_fd.write(json.dumps(obj, separators=(",", ":")) + "\n")
            self._tick_fd.flush()
        except (OSError, ValueError):
            self._tick_fd = None

    def _emit_episode(self, obj: dict[str, Any]) -> None:
        if self._episode_fd is None:
            return
        try:
            self._episode_fd.write(json.dumps(obj, separators=(",", ":")) + "\n")
            self._episode_fd.flush()
        except (OSError, ValueError):
            self._episode_fd = None


def _now_ms() -> int:
    return int(time.time() * 1000)
