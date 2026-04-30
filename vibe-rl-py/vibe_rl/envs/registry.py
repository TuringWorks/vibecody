"""Environment registry probing — slice 3 leans on this; slice 2 only needs
`make_env` so the trainer can construct an env from a string id.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from typing import Any, Callable

import gymnasium as gym


def make_env(env_id: str, *, seed: int = 0, file_path: str | None = None) -> Callable[[], gym.Env]:
    """Return a thunk that constructs the env. Used by Gymnasium's vector-env API.

    `env_id` follows the storage scheme used by the daemon:
        gym:<name>:<gym-version>          → standard Gymnasium env
        custom:<name>                     → custom env, optional `file_path`
        custom_python:<name>              → user-registered Python env
    """

    def _thunk() -> gym.Env:
        env: gym.Env
        if env_id.startswith("gym:"):
            # gym:CartPole-v1:gym-0.29  → env name is the middle segment
            parts = env_id.split(":")
            name = parts[1] if len(parts) > 1 else env_id[len("gym:") :]
            env = gym.make(name)
        elif env_id.startswith("custom_python:") and file_path:
            env = _load_custom_python(file_path)
        elif env_id.startswith("custom:"):
            # Slice 3 will validate these against the workspace registry.
            # For slice 2, fall back to Gymnasium with the suffix as the id.
            name = env_id.split(":", 1)[1]
            env = gym.make(name)
        else:
            env = gym.make(env_id)
        env.reset(seed=seed)
        return env

    return _thunk


def _load_custom_python(file_path: str) -> gym.Env:
    """Import a `.py` file and instantiate the first `gym.Env` subclass it defines.

    Slice 3 will tighten this to require the file declare a single
    `make_env()` callable, so we don't have to guess. For slice 2 it's
    used only when a workspace already has a custom env registered.
    """
    p = Path(file_path).resolve()
    if not p.is_file():
        raise FileNotFoundError(file_path)
    spec = importlib.util.spec_from_file_location(f"vibe_rl_user_env_{p.stem}", str(p))
    if spec is None or spec.loader is None:
        raise ImportError(f"could not load {file_path}")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    for attr in dir(mod):
        obj = getattr(mod, attr)
        if isinstance(obj, type) and issubclass(obj, gym.Env) and obj is not gym.Env:
            return obj()  # type: ignore[no-any-return]
    raise ImportError(f"no gym.Env subclass found in {file_path}")


def probe_gymnasium() -> dict[str, Any]:
    """Walk Gymnasium's registry and return a JSON-friendly summary."""
    envs: list[dict[str, Any]] = []
    for spec in gym.registry.values():
        try:
            envs.append(
                {
                    "id": spec.id,
                    "entry_point": str(spec.entry_point),
                    "max_episode_steps": spec.max_episode_steps,
                    "reward_threshold": spec.reward_threshold,
                    "nondeterministic": spec.nondeterministic,
                }
            )
        except Exception:  # noqa: BLE001 — registry entries can raise on inspect
            continue
    return {"source": "gymnasium", "sdk_version": gym.__version__, "envs": envs}
