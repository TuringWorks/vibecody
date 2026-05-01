"""Slice 7b-extras — VDN + QMIX value-decomposition cooperative MARL.

Both algorithms share the same skeleton (off-policy DQN-style with
per-agent Q-networks, target nets, replay buffer, joint Bellman target
on the team reward) and differ only in the *mixer* that combines
per-agent action-values Q_i(o_i, a_i) into a joint Q_tot(s, a).

  - **VDN** (Sunehag et al. 2017): Q_tot = Σ_i Q_i(o_i, a_i).
    Linear sum, cheap, baseline.
  - **QMIX** (Rashid et al. 2018): Q_tot = mixer(Q_1, …, Q_N | s)
    where the mixer is a monotonic neural network whose weights are
    produced by a hypernetwork conditioned on the global state. The
    monotonicity (non-negative weights) preserves the IGM property
    (argmax_{a} Q_tot = (argmax_{a_i} Q_i)_i), so decentralized greedy
    execution remains optimal w.r.t. Q_tot.

Selection is via the config flag `mixer ∈ {sum, monotonic}` — `sum`
gives VDN, `monotonic` gives QMIX. Default: monotonic.

Targets: cooperative discrete-action MARL with a team reward summed
over agents. Smoke env: PettingZoo MPE `simple_spread_v3` via the
slice 7b `_make_pettingzoo_env` resolver.

Wire format mirrors MAPPO's tick payload, with extra fields:
- `q_loss` (Bellman MSE on Q_tot)
- `q_tot_mean` (current avg joint Q)
- `target_q_mean` (avg target Q after the Bellman backup)
- `epsilon` (current epsilon-greedy exploration rate)
- `mixer` (sum / monotonic)
- `replay_size` (current replay buffer occupancy)
"""

from __future__ import annotations

import math
import random
import time
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

from vibe_rl import runtime
from vibe_rl.algos.mappo import _make_pettingzoo_env  # reuse env resolver
from vibe_rl.checkpoint import save_checkpoint


@dataclass
class QMIXConfig:
    run_id: str
    env_id: str
    mixer: str = "monotonic"  # "sum" → VDN, "monotonic" → QMIX
    total_timesteps: int = 200_000
    learning_rate: float = 5e-4
    gamma: float = 0.99
    epsilon_start: float = 1.0
    epsilon_end: float = 0.05
    epsilon_decay_steps: int = 50_000
    replay_capacity: int = 50_000
    batch_size: int = 128
    learn_starts: int = 1_000
    target_update_interval: int = 200  # in env steps
    train_interval: int = 1            # update per env step
    grad_norm_clip: float = 10.0
    seed: int = 42

    workspace_path: str = "."
    artifact_dir: str = ""
    checkpoint_every_steps: int = 50_000


def _layer_init(layer, std=math.sqrt(2), bias_const=0.0):  # type: ignore[no-untyped-def]
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


class _QNet(nn.Module):
    """Per-agent action-value head (discrete actions)."""

    def __init__(self, obs_dim: int, n_actions: int) -> None:
        super().__init__()
        self.net = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.ReLU(),
            _layer_init(nn.Linear(64, 64)),
            nn.ReLU(),
            _layer_init(nn.Linear(64, n_actions), std=0.01),
        )

    def forward(self, obs: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        return self.net(obs)


class _SumMixer(nn.Module):
    """VDN — joint Q is the elementwise sum of per-agent Qs."""

    def forward(self, agent_qs: torch.Tensor, _state: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        # agent_qs: (B, N) of selected per-agent Q-values; state ignored.
        return agent_qs.sum(dim=-1, keepdim=True)


class _MonotonicMixer(nn.Module):
    """QMIX — monotonic mixer with hypernet weights conditioned on state.

    Hypernet:
      h1: state → flat(N × hidden) weights for layer 1
      h2: state → flat(hidden × 1) weights for layer 2
      b1, b2: per-state biases.

    Monotonicity: take abs() of hypernet weights so the mixer's gradient
    w.r.t. each Q_i is non-negative, which is the IGM-preserving
    constraint of Rashid et al.
    """

    def __init__(self, n_agents: int, state_dim: int, hidden: int = 32) -> None:
        super().__init__()
        self.n_agents = n_agents
        self.hidden = hidden
        self.hyper_w1 = nn.Linear(state_dim, n_agents * hidden)
        self.hyper_w2 = nn.Linear(state_dim, hidden * 1)
        self.hyper_b1 = nn.Linear(state_dim, hidden)
        self.hyper_b2 = nn.Sequential(
            nn.Linear(state_dim, hidden),
            nn.ReLU(),
            nn.Linear(hidden, 1),
        )

    def forward(self, agent_qs: torch.Tensor, state: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        # agent_qs: (B, N), state: (B, state_dim) → (B, 1)
        bs = agent_qs.shape[0]
        agent_qs = agent_qs.view(bs, 1, self.n_agents)

        w1 = torch.abs(self.hyper_w1(state)).view(bs, self.n_agents, self.hidden)
        b1 = self.hyper_b1(state).view(bs, 1, self.hidden)
        hidden = F.elu(torch.bmm(agent_qs, w1) + b1)

        w2 = torch.abs(self.hyper_w2(state)).view(bs, self.hidden, 1)
        b2 = self.hyper_b2(state).view(bs, 1, 1)
        out = torch.bmm(hidden, w2) + b2
        return out.view(bs, 1)


class _ReplayBuffer:
    """FIFO buffer of (per-agent obs, per-agent action, joint reward, next-obs, done)."""

    def __init__(self, capacity: int, agents: list[str], obs_dims: dict[str, int]) -> None:
        self.capacity = capacity
        self.agents = agents
        self.obs_dims = obs_dims
        self._obs: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._next_obs: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._act: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._rew: deque = deque(maxlen=capacity)  # joint reward (sum over agents)
        self._done: deque = deque(maxlen=capacity)
        self._size = 0

    def push(
        self,
        obs: dict[str, np.ndarray],
        actions: dict[str, int],
        joint_reward: float,
        next_obs: dict[str, np.ndarray],
        done: bool,
    ) -> None:
        for a in self.agents:
            self._obs[a].append(obs[a].copy())
            self._next_obs[a].append(next_obs[a].copy())
            self._act[a].append(int(actions[a]))
        self._rew.append(joint_reward)
        self._done.append(1.0 if done else 0.0)
        self._size = min(self._size + 1, self.capacity)

    def __len__(self) -> int:
        return self._size

    def sample(self, batch_size: int, device: torch.device):  # type: ignore[no-untyped-def]
        idx = np.random.randint(0, self._size, size=batch_size)
        out: dict[str, Any] = {"obs": {}, "next_obs": {}, "act": {}}
        for a in self.agents:
            obs_arr = np.stack([self._obs[a][i] for i in idx]).astype(np.float32)
            next_arr = np.stack([self._next_obs[a][i] for i in idx]).astype(np.float32)
            act_arr = np.array([self._act[a][i] for i in idx], dtype=np.int64)
            out["obs"][a] = torch.tensor(obs_arr, device=device)
            out["next_obs"][a] = torch.tensor(next_arr, device=device)
            out["act"][a] = torch.tensor(act_arr, device=device)
        rew_arr = np.array([self._rew[i] for i in idx], dtype=np.float32)
        done_arr = np.array([self._done[i] for i in idx], dtype=np.float32)
        out["reward"] = torch.tensor(rew_arr, device=device)
        out["done"] = torch.tensor(done_arr, device=device)
        return out


def _epsilon_at(step: int, cfg: QMIXConfig) -> float:
    if step >= cfg.epsilon_decay_steps:
        return cfg.epsilon_end
    frac = step / max(cfg.epsilon_decay_steps, 1)
    return cfg.epsilon_start + frac * (cfg.epsilon_end - cfg.epsilon_start)


def train(cfg: QMIXConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    if cfg.mixer not in {"sum", "monotonic"}:
        streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device="cpu")
        streamer.finished(
            reason="error",
            error=f"mixer must be 'sum' (VDN) or 'monotonic' (QMIX) — got {cfg.mixer!r}",
        )
        return {"error": "bad mixer"}

    random.seed(cfg.seed)
    np.random.seed(cfg.seed)
    torch.manual_seed(cfg.seed)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device=str(device))

    try:
        env = _make_pettingzoo_env(cfg.env_id, cfg.seed)
    except Exception as e:  # noqa: BLE001
        streamer.finished(
            reason="error",
            error=(
                f"failed to load PettingZoo env {cfg.env_id!r}: {type(e).__name__}: {e}. "
                f"Install with `cd vibe-rl-py && uv sync --extra marl` and ensure mpe2 for MPE envs."
            ),
        )
        return {"error": str(e)}

    agents = list(env.possible_agents)
    if not agents:
        streamer.finished(reason="error", error="env has no agents")
        return {"error": "no agents"}

    obs_dims: dict[str, int] = {}
    action_sizes: dict[str, int] = {}
    for a in agents:
        os_ = env.observation_space(a)
        as_ = env.action_space(a)
        if type(as_).__name__ != "Discrete":
            streamer.finished(
                reason="error",
                error=f"VDN/QMIX require Discrete action spaces — agent {a!r} has {type(as_).__name__}",
            )
            return {"error": "non-discrete"}
        obs_dims[a] = int(np.prod(os_.shape)) if hasattr(os_, "shape") else 0
        action_sizes[a] = int(as_.n)

    state_dim = sum(obs_dims.values())

    # Per-agent online + target Q-nets.
    online: dict[str, nn.Module] = {a: _QNet(obs_dims[a], action_sizes[a]).to(device) for a in agents}
    target: dict[str, nn.Module] = {a: _QNet(obs_dims[a], action_sizes[a]).to(device) for a in agents}
    for a in agents:
        target[a].load_state_dict(online[a].state_dict())
        for p in target[a].parameters():
            p.requires_grad = False

    online_mixer: nn.Module = (
        _MonotonicMixer(len(agents), state_dim) if cfg.mixer == "monotonic" else _SumMixer()
    ).to(device)
    target_mixer: nn.Module = (
        _MonotonicMixer(len(agents), state_dim) if cfg.mixer == "monotonic" else _SumMixer()
    ).to(device)
    target_mixer.load_state_dict(online_mixer.state_dict())
    for p in target_mixer.parameters():
        p.requires_grad = False

    params: list[nn.Parameter] = []
    for a in agents:
        params.extend(online[a].parameters())
    params.extend(online_mixer.parameters())
    optimizer = torch.optim.Adam(params, lr=cfg.learning_rate)

    replay = _ReplayBuffer(cfg.replay_capacity, agents, obs_dims)

    obs_dict, _ = env.reset(seed=cfg.seed)
    # Convert to per-agent np arrays of consistent shape.
    obs_np: dict[str, np.ndarray] = {a: np.asarray(obs_dict[a], dtype=np.float32).reshape(-1) for a in agents}

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli/rl-artifacts" / cfg.run_id
    )
    workspace_path_obj = Path(cfg.workspace_path)

    global_step = 0
    last_checkpoint_step = 0
    cumulative_episode_return = 0.0
    episode_step_in_run = 0
    last_episode_idx = 0
    last_returns: list[float] = []
    final_reward_mean: float | None = None
    last_q_loss: float = 0.0
    last_q_tot: float = 0.0
    last_target_q: float = 0.0
    t0 = time.monotonic()
    tick_idx = 0

    while global_step < cfg.total_timesteps:
        if runtime.should_stop():
            break

        epsilon = _epsilon_at(global_step, cfg)
        actions: dict[str, int] = {}
        for a in agents:
            if random.random() < epsilon:
                actions[a] = random.randrange(action_sizes[a])
            else:
                with torch.no_grad():
                    obs_t = torch.tensor(obs_np[a], dtype=torch.float32, device=device).unsqueeze(0)
                    q = online[a](obs_t)
                    actions[a] = int(q.argmax(dim=-1).item())

        next_obs_dict, rewards, terminations, truncations, _ = env.step(actions)
        joint_reward = float(sum(float(rewards.get(a, 0.0)) for a in agents))
        cumulative_episode_return += joint_reward
        episode_step_in_run += 1
        next_obs_np: dict[str, np.ndarray] = {
            a: np.asarray(next_obs_dict[a], dtype=np.float32).reshape(-1) for a in agents
        }
        done = any(terminations.values()) or any(truncations.values())

        replay.push(obs_np, actions, joint_reward, next_obs_np, done)
        global_step += 1

        if done:
            last_episode_idx += 1
            last_returns.append(cumulative_episode_return)
            streamer.episode(
                idx=last_episode_idx,
                timestep=global_step,
                reward=cumulative_episode_return,
                length=episode_step_in_run,
                success=None,
                duration_ms=0,
            )
            cumulative_episode_return = 0.0
            episode_step_in_run = 0
            obs_dict, _ = env.reset(seed=cfg.seed + global_step)
            obs_np = {a: np.asarray(obs_dict[a], dtype=np.float32).reshape(-1) for a in agents}
        else:
            obs_np = next_obs_np

        # Learn.
        if (
            len(replay) >= cfg.learn_starts
            and global_step % cfg.train_interval == 0
            and len(replay) >= cfg.batch_size
        ):
            batch = replay.sample(cfg.batch_size, device)
            # Per-agent online Q for the action that was taken.
            agent_qs = []
            for a in agents:
                q_all = online[a](batch["obs"][a])
                q_taken = q_all.gather(1, batch["act"][a].view(-1, 1)).squeeze(-1)
                agent_qs.append(q_taken)
            agent_qs_t = torch.stack(agent_qs, dim=-1)  # (B, N)

            # Target: per-agent argmax under online net (Double-DQN-style),
            # evaluated by target net for stability.
            with torch.no_grad():
                next_agent_qs = []
                for a in agents:
                    next_online = online[a](batch["next_obs"][a])
                    next_actions = next_online.argmax(dim=-1)
                    next_target = target[a](batch["next_obs"][a])
                    next_q = next_target.gather(1, next_actions.view(-1, 1)).squeeze(-1)
                    next_agent_qs.append(next_q)
                next_agent_qs_t = torch.stack(next_agent_qs, dim=-1)

            # State for the mixer = concat of all agents' observations.
            state = torch.cat([batch["obs"][a] for a in agents], dim=-1)
            next_state = torch.cat([batch["next_obs"][a] for a in agents], dim=-1)

            q_tot = online_mixer(agent_qs_t, state).squeeze(-1)
            with torch.no_grad():
                next_q_tot = target_mixer(next_agent_qs_t, next_state).squeeze(-1)
                target_q = batch["reward"] + cfg.gamma * (1.0 - batch["done"]) * next_q_tot

            loss = F.mse_loss(q_tot, target_q)
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(params, cfg.grad_norm_clip)
            optimizer.step()

            last_q_loss = float(loss.item())
            last_q_tot = float(q_tot.mean().item())
            last_target_q = float(target_q.mean().item())

            if global_step % cfg.target_update_interval == 0:
                for a in agents:
                    target[a].load_state_dict(online[a].state_dict())
                target_mixer.load_state_dict(online_mixer.state_dict())

            tick_idx += 1
            sps = global_step / max(time.monotonic() - t0, 1e-6)
            recent_mean = float(np.mean(last_returns[-100:])) if last_returns else 0.0
            final_reward_mean = recent_mean
            streamer.tick(
                tick=tick_idx,
                timestep=global_step,
                payload={
                    "q_loss": last_q_loss,
                    "q_tot_mean": last_q_tot,
                    "target_q_mean": last_target_q,
                    "epsilon": epsilon,
                    "mixer": cfg.mixer,
                    "replay_size": len(replay),
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "recent_reward_mean_100": recent_mean,
                    "agents": agents,
                },
            )

        if global_step - last_checkpoint_step >= cfg.checkpoint_every_steps:
            info = save_checkpoint(
                artifact_dir=artifact_dir,
                timestep=global_step,
                state={
                    "online": {a: online[a].state_dict() for a in agents},
                    "target": {a: target[a].state_dict() for a in agents},
                    "online_mixer": online_mixer.state_dict(),
                    "target_mixer": target_mixer.state_dict(),
                    "optimizer": optimizer.state_dict(),
                },
                metadata={
                    "algorithm": "QMIX" if cfg.mixer == "monotonic" else "VDN",
                    "kind": "train",
                    "env_id": cfg.env_id,
                    "agents": agents,
                    "obs_dims": obs_dims,
                    "action_sizes": action_sizes,
                    "mixer": cfg.mixer,
                    "state_dim": state_dim,
                    "sidecar_version": _sidecar_version(),
                },
                workspace_path=workspace_path_obj,
            )
            streamer.checkpoint(
                timestep=global_step,
                rel_path=info.rel_path,
                sha256=info.sha256,
                size_bytes=info.size_bytes,
            )
            last_checkpoint_step = global_step

    info = save_checkpoint(
        artifact_dir=artifact_dir,
        timestep=global_step,
        state={
            "online": {a: online[a].state_dict() for a in agents},
            "target": {a: target[a].state_dict() for a in agents},
            "online_mixer": online_mixer.state_dict(),
            "target_mixer": target_mixer.state_dict(),
            "optimizer": optimizer.state_dict(),
        },
        metadata={
            "algorithm": "QMIX" if cfg.mixer == "monotonic" else "VDN",
            "kind": "train",
            "env_id": cfg.env_id,
            "agents": agents,
            "obs_dims": obs_dims,
            "action_sizes": action_sizes,
            "mixer": cfg.mixer,
            "state_dim": state_dim,
            "sidecar_version": _sidecar_version(),
            "final": True,
        },
        workspace_path=workspace_path_obj,
    )
    streamer.checkpoint(
        timestep=global_step,
        rel_path=info.rel_path,
        sha256=info.sha256,
        size_bytes=info.size_bytes,
    )

    env.close()
    return {
        "final_reward_mean": final_reward_mean if final_reward_mean is not None else 0.0,
        "total_steps": global_step,
        "agents": agents,
        "mixer": cfg.mixer,
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
