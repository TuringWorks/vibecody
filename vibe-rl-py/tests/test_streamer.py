"""Streamer + protocol tests — no torch/gym needed.

The Rust-side fixture for the executor reads the same JSON-Lines wire
format we emit here, so these tests double as the protocol contract.
"""

import io
import json

from vibe_rl.streamer import Streamer


class _CaptureStreamer(Streamer):
    def __init__(self, run_id: str) -> None:  # type: ignore[no-untyped-def]
        super().__init__(run_id)
        self.tick_buf = io.StringIO()
        self.episode_buf = io.StringIO()
        self._tick_fd = self.tick_buf
        self._episode_fd = self.episode_buf


def _lines(buf: io.StringIO) -> list[dict]:
    buf.seek(0)
    return [json.loads(line) for line in buf.read().splitlines() if line.strip()]


def test_started_then_tick_then_finished_round_trips() -> None:
    s = _CaptureStreamer("run-x")
    s.started(sidecar_version="0.1.0", seed=7, device="cpu")
    s.tick(tick=1, timestep=2048, payload={"policy_loss": 0.1})
    s.tick(tick=2, timestep=4096, payload={"policy_loss": 0.05})
    s.finished(reason="done", final_reward_mean=487.3)

    rows = _lines(s.tick_buf)
    assert [r["t"] for r in rows] == ["started", "tick", "tick", "finished"]
    assert rows[1]["tick"] == 1
    assert rows[2]["payload"]["policy_loss"] == 0.05
    assert rows[3]["final_reward_mean"] == 487.3


def test_episodes_go_to_separate_stream() -> None:
    s = _CaptureStreamer("run-y")
    s.episode(idx=1, timestep=200, reward=100.0, length=200, success=True, duration_ms=500)
    s.episode(idx=2, timestep=400, reward=150.0, length=200, success=False, duration_ms=510)

    tick_rows = _lines(s.tick_buf)
    ep_rows = _lines(s.episode_buf)
    assert tick_rows == []
    assert [r["idx"] for r in ep_rows] == [1, 2]
    assert ep_rows[1]["reward"] == 150.0


def test_finished_with_error_includes_message() -> None:
    s = _CaptureStreamer("run-z")
    s.finished(reason="error", error="boom")
    rows = _lines(s.tick_buf)
    assert rows[-1]["reason"] == "error"
    assert rows[-1]["error"] == "boom"
