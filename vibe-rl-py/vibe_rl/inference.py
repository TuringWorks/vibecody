"""Slice 6.5 — long-lived inference sidecar.

Started by the daemon's `PythonRuntime` (one process per deployed Policy).
Reads JSON-Lines observation requests on stdin, writes JSON-Lines action
responses on stdout. The wire format is intentionally minimal:

    request:  {"obs": [<floats>]}                  per line
    response: {"action": [<scalars>]}              per line
              {"action": <int>}                    for discrete spaces
              {"error": "<msg>"}                   for failures

`ready` is emitted once on startup so the daemon knows the policy is loaded
and the loop is listening:

    ready:    {"t":"ready","framework":"pytorch","action_kind":"discrete"}

The checkpoint format is the one slice 2's PPO writes via
`vibe_rl/checkpoint.py` — a PyTorch state_dict + a JSON sidecar with
`algorithm`, `obs_dim`, `action_kind`, etc.

Slice 6.5 supports PyTorch checkpoints (slice 2's PPO output) only. ONNX
support lands when the daemon picks the `ort` runtime path.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import numpy as np
import torch


def _build_agent_from_metadata(meta: dict[str, Any]) -> torch.nn.Module:
    """Reconstruct the small PPO networks from the metadata sidecar.

    Mirrors `vibe_rl.algos.ppo._DiscreteAgent` / `_ContinuousAgent`. Lives
    here (not imported) so the inference sidecar's startup time isn't
    dominated by `gymnasium` imports.
    """
    import math

    obs_dim = int(meta["obs_dim"])
    action_kind = meta.get("action_kind", "discrete")

    def linear(in_dim: int, out_dim: int, std: float = math.sqrt(2)) -> torch.nn.Linear:
        layer = torch.nn.Linear(in_dim, out_dim)
        torch.nn.init.orthogonal_(layer.weight, std)
        torch.nn.init.constant_(layer.bias, 0.0)
        return layer

    if action_kind == "discrete":
        n_actions = int(meta.get("n_actions") or meta.get("action_n") or 2)

        class DiscreteAgent(torch.nn.Module):
            def __init__(self) -> None:
                super().__init__()
                self.actor = torch.nn.Sequential(
                    linear(obs_dim, 64),
                    torch.nn.Tanh(),
                    linear(64, 64),
                    torch.nn.Tanh(),
                    linear(64, n_actions, std=0.01),
                )
                self.critic = torch.nn.Sequential(
                    linear(obs_dim, 64),
                    torch.nn.Tanh(),
                    linear(64, 64),
                    torch.nn.Tanh(),
                    linear(64, 1, std=1.0),
                )

            def act(self, x: torch.Tensor) -> torch.Tensor:
                logits = self.actor(x)
                return torch.argmax(logits, dim=-1)  # deterministic at serve time

        return DiscreteAgent()

    # Continuous (Box action space)
    action_dim = int(meta.get("action_dim") or 1)

    class ContinuousAgent(torch.nn.Module):
        def __init__(self) -> None:
            super().__init__()
            self.actor_mean = torch.nn.Sequential(
                linear(obs_dim, 64),
                torch.nn.Tanh(),
                linear(64, 64),
                torch.nn.Tanh(),
                linear(64, action_dim, std=0.01),
            )
            self.actor_logstd = torch.nn.Parameter(torch.zeros(1, action_dim))
            self.critic = torch.nn.Sequential(
                linear(obs_dim, 64),
                torch.nn.Tanh(),
                linear(64, 64),
                torch.nn.Tanh(),
                linear(64, 1, std=1.0),
            )

        def act(self, x: torch.Tensor) -> torch.Tensor:
            return self.actor_mean(x)  # deterministic at serve time

    return ContinuousAgent()


def _load_policy(checkpoint_path: Path) -> tuple[torch.nn.Module, dict[str, Any], torch.device]:
    metadata_path = checkpoint_path.with_suffix(".json")
    if not metadata_path.is_file():
        raise FileNotFoundError(
            f"checkpoint metadata sidecar missing: {metadata_path} "
            f"(slice 2's PPO writes both .pt and .json — point at the .pt)"
        )
    metadata = json.loads(metadata_path.read_text())

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    agent = _build_agent_from_metadata(metadata).to(device)
    state = torch.load(checkpoint_path, map_location=device, weights_only=True)
    # CleanRL-style PPO saves under "policy".
    if "policy" in state:
        agent.load_state_dict(state["policy"], strict=False)
    else:
        agent.load_state_dict(state, strict=False)
    agent.eval()
    return agent, metadata, device


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m vibe_rl inference")
    parser.add_argument(
        "--checkpoint",
        required=True,
        help="path to a PyTorch checkpoint .pt — must have a sibling .json metadata file",
    )
    args = parser.parse_args(argv)

    try:
        agent, metadata, device = _load_policy(Path(args.checkpoint))
    except Exception as e:  # noqa: BLE001
        sys.stdout.write(json.dumps({"t": "error", "error": f"{type(e).__name__}: {e}"}) + "\n")
        sys.stdout.flush()
        return 1

    sys.stdout.write(
        json.dumps(
            {
                "t": "ready",
                "framework": "pytorch",
                "action_kind": metadata.get("action_kind", "discrete"),
                "device": str(device),
                "checkpoint": args.checkpoint,
            }
        )
        + "\n"
    )
    sys.stdout.flush()

    # Request loop. One JSON request per stdin line, one JSON response per stdout line.
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as e:
            sys.stdout.write(json.dumps({"error": f"invalid JSON: {e}"}) + "\n")
            sys.stdout.flush()
            continue

        obs_raw = req.get("obs")
        if obs_raw is None:
            sys.stdout.write(json.dumps({"error": "obs field required"}) + "\n")
            sys.stdout.flush()
            continue

        try:
            obs = np.asarray(obs_raw, dtype=np.float32)
            if obs.ndim == 1:
                obs = obs.reshape(1, -1)
            obs_t = torch.tensor(obs, dtype=torch.float32, device=device)
            with torch.no_grad():
                action = agent.act(obs_t)
            action_np = action.cpu().numpy()
            if action_np.ndim > 0 and action_np.size == 1:
                action_out: int | float | list[float] = action_np.item()
            else:
                action_out = action_np.flatten().tolist()
            sys.stdout.write(json.dumps({"action": action_out}) + "\n")
            sys.stdout.flush()
        except Exception as e:  # noqa: BLE001
            sys.stdout.write(json.dumps({"error": f"{type(e).__name__}: {e}"}) + "\n")
            sys.stdout.flush()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
