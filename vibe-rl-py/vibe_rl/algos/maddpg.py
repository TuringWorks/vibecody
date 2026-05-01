"""Slice 7b-extras+1 — MADDPG (Multi-Agent Deep Deterministic Policy Gradient).

Lowe et al. NIPS 2017, "Multi-Agent Actor-Critic for Mixed Cooperative-
Competitive Environments". Continuous-action multi-agent RL with the
**centralized training, decentralized execution** (CTDE) pattern:

- Each agent has its own deterministic actor μ_i(o_i) → a_i (no
  stochasticity at training time; exploration is via additive Gaussian
  noise on the action).
- Each agent has its own *centralized* critic Q_i(x, a_1, …, a_N) that
  takes the joint observation x = concat(o_1, …, o_N) and the joint
  action vector. Centralization lets the critic learn from the *true*
  team dynamics; decentralized execution falls back to actors that only
  see their own observation.
- Off-policy: shared replay buffer over transitions (o_i, a_i, r_i,
  o_i', d) for every agent. Soft (Polyak) target updates with rate τ.

Update step per learn-call:

  1. Sample batch from replay.
  2. For each agent i:
       - Compute target action a'_j = μ_target_j(o_j') for all j.
       - Compute target value y_i = r_i + γ (1 - d) Q_target_i(x', a'_*).
       - Critic loss = MSE(Q_i(x, a_*), y_i).
       - Actor loss  = -mean Q_i(x, μ_i(o_i), a_-i_from_replay)
         (deterministic policy gradient through the critic; other
         agents' actions are taken from the replay batch as in the
         original paper, not through their actor heads).
  3. Polyak target updates:
       θ_target ← τ θ_online + (1 - τ) θ_target.

Selection: continuous discrete-vs-continuous action determined by the
PettingZoo env. We force `continuous_actions=True` when constructing
MPE envs so simple_spread / simple_speaker_listener / etc. expose Box
spaces.
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
from vibe_rl.algos.mappo import _make_pettingzoo_env
from vibe_rl.checkpoint import save_checkpoint


@dataclass
class MADDPGConfig:
    run_id: str
    env_id: str
    total_timesteps: int = 200_000
    actor_lr: float = 1e-4
    critic_lr: float = 1e-3
    gamma: float = 0.95           # MADDPG paper uses 0.95 (typical for MPE).
    tau: float = 0.01             # Polyak rate for target updates.
    replay_capacity: int = 100_000
    batch_size: int = 128
    learn_starts: int = 1_000
    train_interval: int = 1
    grad_norm_clip: float = 0.5
    exploration_noise_std: float = 0.1   # Gaussian noise std on action.
    seed: int = 42

    workspace_path: str = "."
    artifact_dir: str = ""
    checkpoint_every_steps: int = 50_000


def _layer_init(layer, std=math.sqrt(2), bias_const=0.0):  # type: ignore[no-untyped-def]
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


class _Actor(nn.Module):
    """Deterministic continuous actor with tanh output bounded to [-1, 1].

    Action ranges from each PettingZoo env are typically [-1, 1] for the
    Box action space exposed when continuous_actions=True; tanh keeps us
    in-range without per-env rescaling.
    """

    def __init__(self, obs_dim: int, action_dim: int) -> None:
        super().__init__()
        self.net = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.ReLU(),
            _layer_init(nn.Linear(64, 64)),
            nn.ReLU(),
            _layer_init(nn.Linear(64, action_dim), std=0.01),
        )

    def forward(self, obs: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        return torch.tanh(self.net(obs))


class _CentralizedCritic(nn.Module):
    """Q_i(x, a_1, …, a_N) — joint obs + joint actions → scalar."""

    def __init__(self, joint_obs_dim: int, joint_action_dim: int) -> None:
        super().__init__()
        self.net = nn.Sequential(
            _layer_init(nn.Linear(joint_obs_dim + joint_action_dim, 128)),
            nn.ReLU(),
            _layer_init(nn.Linear(128, 128)),
            nn.ReLU(),
            _layer_init(nn.Linear(128, 1), std=1.0),
        )

    def forward(self, joint_obs: torch.Tensor, joint_actions: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        x = torch.cat([joint_obs, joint_actions], dim=-1)
        return self.net(x).squeeze(-1)


class _ReplayBuffer:
    """FIFO buffer storing per-agent (obs, action, reward, next-obs) + done."""

    def __init__(self, capacity: int, agents: list[str]) -> None:
        self.capacity = capacity
        self.agents = agents
        self._obs: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._next_obs: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._act: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._rew: dict[str, deque] = {a: deque(maxlen=capacity) for a in agents}
        self._done: deque = deque(maxlen=capacity)
        self._size = 0

    def push(
        self,
        obs: dict[str, np.ndarray],
        actions: dict[str, np.ndarray],
        rewards: dict[str, float],
        next_obs: dict[str, np.ndarray],
        done: bool,
    ) -> None:
        for a in self.agents:
            self._obs[a].append(obs[a].copy())
            self._next_obs[a].append(next_obs[a].copy())
            self._act[a].append(actions[a].copy())
            self._rew[a].append(float(rewards.get(a, 0.0)))
        self._done.append(1.0 if done else 0.0)
        self._size = min(self._size + 1, self.capacity)

    def __len__(self) -> int:
        return self._size

    def sample(self, batch_size: int, device: torch.device) -> dict[str, Any]:  # type: ignore[no-untyped-def]
        idx = np.random.randint(0, self._size, size=batch_size)
        out: dict[str, Any] = {"obs": {}, "next_obs": {}, "act": {}, "rew": {}}
        for a in self.agents:
            out["obs"][a] = torch.tensor(
                np.stack([self._obs[a][i] for i in idx]).astype(np.float32),
                device=device,
            )
            out["next_obs"][a] = torch.tensor(
                np.stack([self._next_obs[a][i] for i in idx]).astype(np.float32),
                device=device,
            )
            out["act"][a] = torch.tensor(
                np.stack([self._act[a][i] for i in idx]).astype(np.float32),
                device=device,
            )
            out["rew"][a] = torch.tensor(
                np.array([self._rew[a][i] for i in idx], dtype=np.float32),
                device=device,
            )
        out["done"] = torch.tensor(
            np.array([self._done[i] for i in idx], dtype=np.float32),
            device=device,
        )
        return out


def _polyak_update(target: nn.Module, online: nn.Module, tau: float) -> None:
    with torch.no_grad():
        for tp, op in zip(target.parameters(), online.parameters()):
            tp.data.mul_(1.0 - tau).add_(tau * op.data)


def train(cfg: MADDPGConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    random.seed(cfg.seed)
    np.random.seed(cfg.seed)
    torch.manual_seed(cfg.seed)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device=str(device))

    try:
        env = _make_pettingzoo_env(cfg.env_id, cfg.seed, continuous_actions=True)
    except Exception as e:  # noqa: BLE001
        streamer.finished(
            reason="error",
            error=(
                f"failed to load PettingZoo env {cfg.env_id!r} with continuous_actions=True: "
                f"{type(e).__name__}: {e}. Try mpe2:simple_spread_v3 (set "
                f"`uv sync --extra marl` first)."
            ),
        )
        return {"error": str(e)}

    agents = list(env.possible_agents)
    if not agents:
        streamer.finished(reason="error", error="env has no agents")
        return {"error": "no agents"}

    obs_dims: dict[str, int] = {}
    action_dims: dict[str, int] = {}
    for a in agents:
        os_ = env.observation_space(a)
        as_ = env.action_space(a)
        if type(as_).__name__ != "Box":
            streamer.finished(
                reason="error",
                error=(
                    f"MADDPG requires Box (continuous) action spaces — agent {a!r} "
                    f"has {type(as_).__name__}. Try a different env or pass "
                    f"continuous_actions=True."
                ),
            )
            return {"error": "non-continuous"}
        obs_dims[a] = int(np.prod(os_.shape)) if hasattr(os_, "shape") else 0
        action_dims[a] = int(np.prod(as_.shape))

    joint_obs_dim = sum(obs_dims.values())
    joint_action_dim = sum(action_dims.values())

    online_actors: dict[str, nn.Module] = {a: _Actor(obs_dims[a], action_dims[a]).to(device) for a in agents}
    target_actors: dict[str, nn.Module] = {a: _Actor(obs_dims[a], action_dims[a]).to(device) for a in agents}
    online_critics: dict[str, nn.Module] = {a: _CentralizedCritic(joint_obs_dim, joint_action_dim).to(device) for a in agents}
    target_critics: dict[str, nn.Module] = {a: _CentralizedCritic(joint_obs_dim, joint_action_dim).to(device) for a in agents}
    for a in agents:
        target_actors[a].load_state_dict(online_actors[a].state_dict())
        for p in target_actors[a].parameters():
            p.requires_grad = False
        target_critics[a].load_state_dict(online_critics[a].state_dict())
        for p in target_critics[a].parameters():
            p.requires_grad = False

    actor_optimizers = {
        a: torch.optim.Adam(online_actors[a].parameters(), lr=cfg.actor_lr) for a in agents
    }
    critic_optimizers = {
        a: torch.optim.Adam(online_critics[a].parameters(), lr=cfg.critic_lr) for a in agents
    }

    replay = _ReplayBuffer(cfg.replay_capacity, agents)

    obs_dict, _ = env.reset(seed=cfg.seed)
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
    last_critic_loss: float = 0.0
    last_actor_loss: float = 0.0
    last_q_mean: float = 0.0
    t0 = time.monotonic()
    tick_idx = 0

    while global_step < cfg.total_timesteps:
        if runtime.should_stop():
            break

        actions: dict[str, np.ndarray] = {}
        for a in agents:
            with torch.no_grad():
                obs_t = torch.tensor(obs_np[a], dtype=torch.float32, device=device).unsqueeze(0)
                action = online_actors[a](obs_t).cpu().numpy().reshape(-1)
            # Add Gaussian exploration noise, then clip to [-1, 1].
            noise = np.random.normal(0.0, cfg.exploration_noise_std, size=action.shape).astype(np.float32)
            actions[a] = np.clip(action + noise, -1.0, 1.0)

        next_obs_dict, rewards, terminations, truncations, _ = env.step(actions)
        joint_reward = float(sum(float(rewards.get(a, 0.0)) for a in agents))
        cumulative_episode_return += joint_reward
        episode_step_in_run += 1
        next_obs_np: dict[str, np.ndarray] = {
            a: np.asarray(next_obs_dict[a], dtype=np.float32).reshape(-1) for a in agents
        }
        done = any(terminations.values()) or any(truncations.values())

        replay.push(obs_np, actions, rewards, next_obs_np, done)
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

        # Learn — per-agent critic + actor updates on a shared batch.
        if (
            len(replay) >= cfg.learn_starts
            and global_step % cfg.train_interval == 0
            and len(replay) >= cfg.batch_size
        ):
            batch = replay.sample(cfg.batch_size, device)

            # Build joint obs + joint actions for the batch (used by every
            # agent's critic). Joint = concat in fixed agent order.
            joint_obs = torch.cat([batch["obs"][a] for a in agents], dim=-1)
            joint_next_obs = torch.cat([batch["next_obs"][a] for a in agents], dim=-1)
            joint_actions = torch.cat([batch["act"][a] for a in agents], dim=-1)
            with torch.no_grad():
                joint_target_actions = torch.cat(
                    [target_actors[a](batch["next_obs"][a]) for a in agents], dim=-1
                )

            # Per-agent critic update.
            agent_critic_losses: list[float] = []
            agent_q_means: list[float] = []
            for a in agents:
                with torch.no_grad():
                    q_target_next = target_critics[a](joint_next_obs, joint_target_actions)
                    target_q = batch["rew"][a] + cfg.gamma * (1.0 - batch["done"]) * q_target_next
                q_online = online_critics[a](joint_obs, joint_actions)
                critic_loss = F.mse_loss(q_online, target_q)
                critic_optimizers[a].zero_grad()
                critic_loss.backward()
                nn.utils.clip_grad_norm_(online_critics[a].parameters(), cfg.grad_norm_clip)
                critic_optimizers[a].step()
                agent_critic_losses.append(float(critic_loss.item()))
                agent_q_means.append(float(q_online.mean().item()))

            # Per-agent actor update — gradient ascent on Q_i wrt agent i's
            # action only (other agents' actions held fixed at the replay
            # values, per the original MADDPG paper).
            agent_actor_losses: list[float] = []
            for a in agents:
                # Replace agent a's action in the joint action with the
                # online actor's output (with grad), keep others fixed.
                live_actions = []
                for j in agents:
                    if j == a:
                        live_actions.append(online_actors[a](batch["obs"][a]))
                    else:
                        live_actions.append(batch["act"][j].detach())
                joint_live_actions = torch.cat(live_actions, dim=-1)
                actor_loss = -online_critics[a](joint_obs, joint_live_actions).mean()
                actor_optimizers[a].zero_grad()
                actor_loss.backward()
                nn.utils.clip_grad_norm_(online_actors[a].parameters(), cfg.grad_norm_clip)
                actor_optimizers[a].step()
                agent_actor_losses.append(float(actor_loss.item()))

            # Soft target updates.
            for a in agents:
                _polyak_update(target_actors[a], online_actors[a], cfg.tau)
                _polyak_update(target_critics[a], online_critics[a], cfg.tau)

            last_critic_loss = float(np.mean(agent_critic_losses))
            last_actor_loss = float(np.mean(agent_actor_losses))
            last_q_mean = float(np.mean(agent_q_means))

            tick_idx += 1
            sps = global_step / max(time.monotonic() - t0, 1e-6)
            recent_mean = float(np.mean(last_returns[-100:])) if last_returns else 0.0
            final_reward_mean = recent_mean
            streamer.tick(
                tick=tick_idx,
                timestep=global_step,
                payload={
                    "critic_loss": last_critic_loss,
                    "actor_loss": last_actor_loss,
                    "q_mean": last_q_mean,
                    "exploration_noise_std": cfg.exploration_noise_std,
                    "replay_size": len(replay),
                    "tau": cfg.tau,
                    "actor_lr": cfg.actor_lr,
                    "critic_lr": cfg.critic_lr,
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
                    "online_actors": {a: online_actors[a].state_dict() for a in agents},
                    "target_actors": {a: target_actors[a].state_dict() for a in agents},
                    "online_critics": {a: online_critics[a].state_dict() for a in agents},
                    "target_critics": {a: target_critics[a].state_dict() for a in agents},
                },
                metadata={
                    "algorithm": "MADDPG",
                    "kind": "train",
                    "env_id": cfg.env_id,
                    "agents": agents,
                    "obs_dims": obs_dims,
                    "action_dims": action_dims,
                    "joint_obs_dim": joint_obs_dim,
                    "joint_action_dim": joint_action_dim,
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
            "online_actors": {a: online_actors[a].state_dict() for a in agents},
            "target_actors": {a: target_actors[a].state_dict() for a in agents},
            "online_critics": {a: online_critics[a].state_dict() for a in agents},
            "target_critics": {a: target_critics[a].state_dict() for a in agents},
        },
        metadata={
            "algorithm": "MADDPG",
            "kind": "train",
            "env_id": cfg.env_id,
            "agents": agents,
            "obs_dims": obs_dims,
            "action_dims": action_dims,
            "joint_obs_dim": joint_obs_dim,
            "joint_action_dim": joint_action_dim,
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
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
