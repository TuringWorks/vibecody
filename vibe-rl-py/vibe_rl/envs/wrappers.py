"""Gymnasium wrapper that emits per-episode events to the streamer.

The wrapper is intentionally narrow — slice 2 only needs to know:
- how many steps the episode lasted
- what the cumulative reward was
- whether the env declared a "success" terminal (some envs do via `info["is_success"]`)
- how long the episode took in wall time

Slice 5 will extend this to capture per-component reward decomposition
when the env exposes `info["reward_components"]`.
"""

from __future__ import annotations

import time
from typing import Any

import gymnasium as gym


class MonitorWrapper(gym.Wrapper):
    def __init__(self, env: gym.Env, streamer, *, env_idx: int = 0) -> None:  # type: ignore[no-untyped-def]
        super().__init__(env)
        self._streamer = streamer
        self._env_idx = env_idx
        self._episode_idx = 0
        self._reward_sum = 0.0
        self._length = 0
        self._t_start: float = time.monotonic()
        self._global_timestep = 0

    def reset(self, **kwargs: Any):  # type: ignore[no-untyped-def]
        self._reward_sum = 0.0
        self._length = 0
        self._t_start = time.monotonic()
        return self.env.reset(**kwargs)

    def step(self, action):  # type: ignore[no-untyped-def]
        obs, reward, terminated, truncated, info = self.env.step(action)
        self._reward_sum += float(reward)
        self._length += 1
        self._global_timestep += 1
        if terminated or truncated:
            self._episode_idx += 1
            success = None
            if isinstance(info, dict) and "is_success" in info:
                success = bool(info["is_success"])
            duration_ms = int((time.monotonic() - self._t_start) * 1000)
            self._streamer.episode(
                idx=self._episode_idx,
                timestep=self._global_timestep,
                reward=self._reward_sum,
                length=self._length,
                success=success,
                duration_ms=duration_ms,
            )
        return obs, reward, terminated, truncated, info
