"""PPO — vendored from CleanRL (https://github.com/vwxyzjn/cleanrl, MIT)
and modified to:

1. Stream metric ticks + episode rows to a `Streamer` instance instead of
   tensorboard.
2. Honor `runtime.should_stop()` between updates so SIGTERM cleanly stops
   the run with a final checkpoint.
3. Persist checkpoints into the workspace artifact tree on a configurable
   cadence and at completion.
4. Support both discrete and continuous action spaces in one entry point
   so the daemon doesn't need to dispatch by env type.

The original CleanRL files were `ppo.py` and `ppo_continuous_action.py`;
they share ~95% of the structure. We unify them here.
"""

from __future__ import annotations

import math
import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import gymnasium as gym
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.distributions.categorical import Categorical
from torch.distributions.normal import Normal

from vibe_rl import runtime
from vibe_rl.checkpoint import save_checkpoint
from vibe_rl.envs.registry import make_env
from vibe_rl.envs.wrappers import MonitorWrapper


# ── Config ──────────────────────────────────────────────────────────────────


@dataclass
class PPOConfig:
    run_id: str
    env_id: str
    total_timesteps: int = 100_000
    learning_rate: float = 3e-4
    num_envs: int = 4
    num_steps: int = 128                  # rollout length per env
    anneal_lr: bool = True
    gamma: float = 0.99
    gae_lambda: float = 0.95
    num_minibatches: int = 4
    update_epochs: int = 4
    norm_adv: bool = True
    clip_coef: float = 0.2
    clip_vloss: bool = True
    ent_coef: float = 0.01
    vf_coef: float = 0.5
    max_grad_norm: float = 0.5
    target_kl: float | None = None
    seed: int = 42

    # Sidecar bookkeeping
    workspace_path: str = "."
    artifact_dir: str = ""                # absolute path to write checkpoints
    checkpoint_every_steps: int = 50_000

    # Inferred
    @property
    def batch_size(self) -> int:
        return self.num_envs * self.num_steps

    @property
    def minibatch_size(self) -> int:
        return self.batch_size // self.num_minibatches

    @property
    def num_iterations(self) -> int:
        return self.total_timesteps // self.batch_size


# ── Networks ────────────────────────────────────────────────────────────────


def _layer_init(layer, std=math.sqrt(2), bias_const=0.0):  # type: ignore[no-untyped-def]
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


class _DiscreteAgent(nn.Module):
    def __init__(self, obs_dim: int, n_actions: int) -> None:
        super().__init__()
        self.critic = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 1), std=1.0),
        )
        self.actor = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, n_actions), std=0.01),
        )

    def get_value(self, x):  # type: ignore[no-untyped-def]
        return self.critic(x)

    def get_action_and_value(self, x, action=None):  # type: ignore[no-untyped-def]
        logits = self.actor(x)
        probs = Categorical(logits=logits)
        if action is None:
            action = probs.sample()
        return action, probs.log_prob(action), probs.entropy(), self.critic(x)


class _ContinuousAgent(nn.Module):
    def __init__(self, obs_dim: int, action_dim: int) -> None:
        super().__init__()
        self.critic = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 1), std=1.0),
        )
        self.actor_mean = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, action_dim), std=0.01),
        )
        self.actor_logstd = nn.Parameter(torch.zeros(1, action_dim))

    def get_value(self, x):  # type: ignore[no-untyped-def]
        return self.critic(x)

    def get_action_and_value(self, x, action=None):  # type: ignore[no-untyped-def]
        action_mean = self.actor_mean(x)
        action_logstd = self.actor_logstd.expand_as(action_mean)
        action_std = torch.exp(action_logstd)
        probs = Normal(action_mean, action_std)
        if action is None:
            action = probs.sample()
        return action, probs.log_prob(action).sum(1), probs.entropy().sum(1), self.critic(x)


# ── Vector env construction ─────────────────────────────────────────────────


def _make_vector_env(cfg: PPOConfig, streamer) -> tuple[Any, str, bool]:  # type: ignore[no-untyped-def]
    """Return (vec_env, action_kind, continuous_flag).

    SyncVectorEnv (single-process) is intentional for slice 2 — async vec
    envs add fork/spawn complexity we don't need before profiling says we
    do. CartPole at 1k steps/s on CPU is plenty for slice 2's smoke test.
    """
    thunks = [make_env(cfg.env_id, seed=cfg.seed + i) for i in range(cfg.num_envs)]
    envs = gym.vector.SyncVectorEnv(thunks)
    # Wrap each underlying env with our monitor — we attach to the first
    # env only, since SyncVectorEnv resets each env independently and we
    # otherwise multi-count episodes. The first env's stats are
    # representative for the smoke test; slice 2.5 will track per-env.
    envs.envs[0] = MonitorWrapper(envs.envs[0], streamer)  # type: ignore[attr-defined]
    if isinstance(envs.single_action_space, gym.spaces.Discrete):
        return envs, "discrete", False
    if isinstance(envs.single_action_space, gym.spaces.Box):
        return envs, "continuous", True
    raise ValueError(
        f"PPO supports Discrete or Box action spaces — got {type(envs.single_action_space).__name__}"
    )


# ── Training loop ───────────────────────────────────────────────────────────


def train(cfg: PPOConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    """Run PPO. Returns a dict the caller can include in the `finished`
    JSON-Line (final reward mean, total updates, etc.).
    """
    random.seed(cfg.seed)
    np.random.seed(cfg.seed)
    torch.manual_seed(cfg.seed)
    torch.backends.cudnn.deterministic = True

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    streamer.started(
        sidecar_version=_sidecar_version(),
        seed=cfg.seed,
        device=str(device),
    )

    envs, action_kind, continuous = _make_vector_env(cfg, streamer)
    obs_dim = int(np.prod(envs.single_observation_space.shape))
    if continuous:
        action_dim = int(np.prod(envs.single_action_space.shape))
        agent: nn.Module = _ContinuousAgent(obs_dim, action_dim).to(device)
    else:
        n_actions = int(envs.single_action_space.n)
        agent = _DiscreteAgent(obs_dim, n_actions).to(device)

    optimizer = optim.Adam(agent.parameters(), lr=cfg.learning_rate, eps=1e-5)

    # Storage
    obs_buf = torch.zeros((cfg.num_steps, cfg.num_envs, obs_dim)).to(device)
    if continuous:
        act_buf = torch.zeros((cfg.num_steps, cfg.num_envs) + envs.single_action_space.shape).to(device)
    else:
        act_buf = torch.zeros((cfg.num_steps, cfg.num_envs)).to(device)
    logprob_buf = torch.zeros((cfg.num_steps, cfg.num_envs)).to(device)
    rew_buf = torch.zeros((cfg.num_steps, cfg.num_envs)).to(device)
    done_buf = torch.zeros((cfg.num_steps, cfg.num_envs)).to(device)
    val_buf = torch.zeros((cfg.num_steps, cfg.num_envs)).to(device)

    global_step = 0
    next_obs_np, _ = envs.reset(seed=cfg.seed)
    next_obs = torch.tensor(next_obs_np, dtype=torch.float32).to(device).reshape(cfg.num_envs, -1)
    next_done = torch.zeros(cfg.num_envs).to(device)

    last_checkpoint_step = 0
    last_episode_returns: list[float] = []
    final_reward_mean: float | None = None
    # The MonitorWrapper on env 0 captures full-episode returns by tracking
    # `_reward_sum` between resets. We snapshot whatever it has flushed by
    # episode terminal, building a sliding window for the per-tick reward
    # aggregate. The wrapper itself emits the canonical episode JSON-Lines
    # via the streamer; this list is purely for the convenience aggregate.
    monitor = envs.envs[0]  # type: ignore[attr-defined]
    last_seen_episode_idx = 0

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    workspace_path_obj = Path(cfg.workspace_path)

    t0 = time.monotonic()

    for iteration in range(1, cfg.num_iterations + 1):
        if runtime.should_stop():
            break

        # LR anneal
        if cfg.anneal_lr:
            frac = 1.0 - (iteration - 1.0) / max(cfg.num_iterations, 1)
            optimizer.param_groups[0]["lr"] = frac * cfg.learning_rate

        # Rollout
        for step in range(cfg.num_steps):
            global_step += cfg.num_envs
            obs_buf[step] = next_obs
            done_buf[step] = next_done

            with torch.no_grad():
                action, logprob, _, value = agent.get_action_and_value(next_obs)
                val_buf[step] = value.flatten()

            act_buf[step] = action
            logprob_buf[step] = logprob

            if continuous:
                action_np = action.cpu().numpy()
            else:
                action_np = action.cpu().numpy().astype(np.int64)

            next_obs_np, reward, terminations, truncations, _ = envs.step(action_np)
            next_done_np = np.logical_or(terminations, truncations)
            rew_buf[step] = torch.tensor(reward, dtype=torch.float32).to(device).view(-1)
            next_obs = torch.tensor(next_obs_np, dtype=torch.float32).to(device).reshape(cfg.num_envs, -1)
            next_done = torch.tensor(next_done_np, dtype=torch.float32).to(device)

        # Returns + advantages (GAE)
        with torch.no_grad():
            next_value = agent.get_value(next_obs).reshape(1, -1)
            advantages = torch.zeros_like(rew_buf).to(device)
            lastgaelam = 0.0
            for t in reversed(range(cfg.num_steps)):
                if t == cfg.num_steps - 1:
                    nextnonterminal = 1.0 - next_done
                    nextvalues = next_value
                else:
                    nextnonterminal = 1.0 - done_buf[t + 1]
                    nextvalues = val_buf[t + 1]
                delta = rew_buf[t] + cfg.gamma * nextvalues * nextnonterminal - val_buf[t]
                advantages[t] = lastgaelam = delta + cfg.gamma * cfg.gae_lambda * nextnonterminal * lastgaelam
            returns = advantages + val_buf

        # Flatten the batch
        b_obs = obs_buf.reshape((-1, obs_dim))
        b_logprobs = logprob_buf.reshape(-1)
        if continuous:
            b_actions = act_buf.reshape((-1,) + envs.single_action_space.shape)
        else:
            b_actions = act_buf.reshape(-1)
        b_advantages = advantages.reshape(-1)
        b_returns = returns.reshape(-1)
        b_values = val_buf.reshape(-1)

        # Optimize policy + value
        b_inds = np.arange(cfg.batch_size)
        clipfracs: list[float] = []
        approx_kl_v: float | None = None
        pg_loss_v: float = 0.0
        v_loss_v: float = 0.0
        entropy_v: float = 0.0
        for _epoch in range(cfg.update_epochs):
            np.random.shuffle(b_inds)
            for start in range(0, cfg.batch_size, cfg.minibatch_size):
                end = start + cfg.minibatch_size
                mb_inds = b_inds[start:end]

                _, newlogprob, entropy, newvalue = agent.get_action_and_value(
                    b_obs[mb_inds], b_actions[mb_inds] if not continuous else b_actions[mb_inds]
                )
                logratio = newlogprob - b_logprobs[mb_inds]
                ratio = logratio.exp()

                with torch.no_grad():
                    approx_kl = ((ratio - 1) - logratio).mean()
                    approx_kl_v = float(approx_kl.item())
                    clipfracs.append(((ratio - 1.0).abs() > cfg.clip_coef).float().mean().item())

                mb_advantages = b_advantages[mb_inds]
                if cfg.norm_adv:
                    mb_advantages = (mb_advantages - mb_advantages.mean()) / (mb_advantages.std() + 1e-8)

                pg_loss1 = -mb_advantages * ratio
                pg_loss2 = -mb_advantages * torch.clamp(ratio, 1 - cfg.clip_coef, 1 + cfg.clip_coef)
                pg_loss = torch.max(pg_loss1, pg_loss2).mean()
                pg_loss_v = float(pg_loss.item())

                newvalue = newvalue.view(-1)
                if cfg.clip_vloss:
                    v_loss_unclipped = (newvalue - b_returns[mb_inds]) ** 2
                    v_clipped = b_values[mb_inds] + torch.clamp(
                        newvalue - b_values[mb_inds], -cfg.clip_coef, cfg.clip_coef
                    )
                    v_loss_clipped = (v_clipped - b_returns[mb_inds]) ** 2
                    v_loss_max = torch.max(v_loss_unclipped, v_loss_clipped)
                    v_loss = 0.5 * v_loss_max.mean()
                else:
                    v_loss = 0.5 * ((newvalue - b_returns[mb_inds]) ** 2).mean()
                v_loss_v = float(v_loss.item())

                entropy_loss = entropy.mean()
                entropy_v = float(entropy_loss.item())
                loss = pg_loss - cfg.ent_coef * entropy_loss + v_loss * cfg.vf_coef

                optimizer.zero_grad()
                loss.backward()
                nn.utils.clip_grad_norm_(agent.parameters(), cfg.max_grad_norm)
                optimizer.step()

            if cfg.target_kl is not None and approx_kl_v is not None and approx_kl_v > cfg.target_kl:
                break

        # Aggregate per-iteration metrics + emit a tick.
        sps = int(global_step / max(time.monotonic() - t0, 1e-6))
        # Pick up any episodes that completed since last tick. The monitor
        # wrapper bumps `_episode_idx` and snapshots `_reward_sum` *just
        # before* resetting, so reading after step() is safe.
        current_idx = getattr(monitor, "_episode_idx", last_seen_episode_idx)
        if current_idx > last_seen_episode_idx:
            # We don't preserve a per-episode history on the wrapper, but
            # we *do* see the rolling current return immediately before a
            # done flips. The streamer-side episode rows are the canonical
            # record; here we fall back to the wrapper's most recent
            # observed reward sum as a one-sample approximation, which is
            # still better than the pure-stub 0.0.
            last_episode_returns.append(float(getattr(monitor, "_reward_sum", 0.0)))
            last_seen_episode_idx = current_idx
        recent_mean = float(np.mean(last_episode_returns[-100:])) if last_episode_returns else 0.0
        final_reward_mean = recent_mean

        streamer.tick(
            tick=iteration,
            timestep=global_step,
            payload={
                "policy_loss": pg_loss_v,
                "value_loss": v_loss_v,
                "entropy": entropy_v,
                "approx_kl": approx_kl_v if approx_kl_v is not None else 0.0,
                "clip_fraction": float(np.mean(clipfracs)) if clipfracs else 0.0,
                "learning_rate": optimizer.param_groups[0]["lr"],
                "sps": sps,
                "recent_reward_mean_100": recent_mean,
            },
        )

        # Periodic checkpoints
        if global_step - last_checkpoint_step >= cfg.checkpoint_every_steps:
            info = save_checkpoint(
                artifact_dir=artifact_dir,
                timestep=global_step,
                state={
                    "policy": agent.state_dict(),
                    "optimizer": optimizer.state_dict(),
                },
                metadata={
                    "algorithm": "PPO",
                    "env_id": cfg.env_id,
                    "obs_dim": obs_dim,
                    "action_kind": action_kind,
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

    # Final checkpoint regardless of whether we hit total_timesteps or stopped early.
    info = save_checkpoint(
        artifact_dir=artifact_dir,
        timestep=global_step,
        state={
            "policy": agent.state_dict(),
            "optimizer": optimizer.state_dict(),
        },
        metadata={
            "algorithm": "PPO",
            "env_id": cfg.env_id,
            "obs_dim": obs_dim,
            "action_kind": action_kind,
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

    envs.close()
    return {
        "final_reward_mean": final_reward_mean if final_reward_mean is not None else 0.0,
        "total_steps": global_step,
        "iterations_completed": iteration,
    }


def _recent_returns(envs, prev: list[float]) -> list[float]:  # type: ignore[no-untyped-def]
    """Pull any recently-finished episode returns from the wrapped first env.

    SyncVectorEnv exposes `envs[i]` as the wrapped env. Our MonitorWrapper
    on env 0 already emitted episode rows to the streamer; here we just
    keep an in-memory list for `recent_reward_mean_100` in the metric
    payload. The wrapper doesn't expose a queue, so we approximate by
    re-reading its `_reward_sum` whenever a done was observed — but for
    slice 2 the streamer-side episode emissions are the ground truth and
    this is a convenience aggregate only. We return `prev` unchanged when
    we can't read; the panel falls back to per-tick policy/value losses
    which are always live.
    """
    return prev


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
