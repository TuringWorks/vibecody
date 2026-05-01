"""Slice 7b — MAPPO (Multi-Agent PPO) for PettingZoo ParallelEnv.

Architecture (Yu et al., NeurIPS 2022 — "MAPPO is the unsung baseline"):
  - One actor network per agent role, decentralized execution.
  - One shared centralized critic that takes the *joint* observation
    (concatenation of all agents' obs) and returns one scalar value per
    agent (or one shared value when fully cooperative). We use the
    cooperative variant by default — single value head over the joint.
  - PPO update: standard clipped surrogate on each agent's actor,
    joint MSE update on the centralized critic.

Wire format matches PPO 1:1 — extra payload fields:
  - `agents` (list of agent role names)
  - `per_agent_reward` (mean reward per agent for the last batch)
  - `centralized_value_loss` (joint critic loss)

Smoke test target: PettingZoo MPE `simple_spread_v3` (3 agents,
continuous obs of dim ~18 each, discrete actions of cardinality 5,
cooperative reward). MAPPO converges to ~-15 to ~-25 mean episode
return on this env in a few hundred thousand steps.
"""

from __future__ import annotations

import math
import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.distributions.categorical import Categorical
from torch.distributions.normal import Normal

from vibe_rl import runtime
from vibe_rl.checkpoint import save_checkpoint


@dataclass
class MAPPOConfig:
    run_id: str
    env_id: str  # PettingZoo env spec, e.g. "pettingzoo:simple_spread_v3"
    total_timesteps: int = 200_000
    learning_rate: float = 3e-4
    num_steps: int = 128
    anneal_lr: bool = True
    gamma: float = 0.99
    gae_lambda: float = 0.95
    num_minibatches: int = 4
    update_epochs: int = 4
    clip_coef: float = 0.2
    ent_coef: float = 0.01
    vf_coef: float = 0.5
    max_grad_norm: float = 0.5
    seed: int = 42
    # MAPPO-specific
    share_actor: bool = False  # single shared actor across agents (parameter sharing)

    workspace_path: str = "."
    artifact_dir: str = ""
    checkpoint_every_steps: int = 50_000

    # Per-iteration these are derived from agent count + obs dims at
    # construction time (we don't know them here; the algorithm
    # discovers them from the env's first reset).
    @property
    def num_iterations(self) -> int:
        return self.total_timesteps // self.num_steps


def _layer_init(layer, std=math.sqrt(2), bias_const=0.0):  # type: ignore[no-untyped-def]
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


class _DiscreteActor(nn.Module):
    def __init__(self, obs_dim: int, n_actions: int) -> None:
        super().__init__()
        self.net = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, n_actions), std=0.01),
        )

    def forward(self, obs):  # type: ignore[no-untyped-def]
        return self.net(obs)


class _ContinuousActor(nn.Module):
    def __init__(self, obs_dim: int, action_dim: int) -> None:
        super().__init__()
        self.mean = nn.Sequential(
            _layer_init(nn.Linear(obs_dim, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, 64)),
            nn.Tanh(),
            _layer_init(nn.Linear(64, action_dim), std=0.01),
        )
        self.logstd = nn.Parameter(torch.zeros(1, action_dim))


class _CentralizedCritic(nn.Module):
    """Joint-observation value head. Cooperative MARL → single scalar."""

    def __init__(self, joint_obs_dim: int) -> None:
        super().__init__()
        self.net = nn.Sequential(
            _layer_init(nn.Linear(joint_obs_dim, 128)),
            nn.Tanh(),
            _layer_init(nn.Linear(128, 128)),
            nn.Tanh(),
            _layer_init(nn.Linear(128, 1), std=1.0),
        )

    def forward(self, joint_obs):  # type: ignore[no-untyped-def]
        return self.net(joint_obs)


def _make_pettingzoo_env(env_id: str, seed: int):  # type: ignore[no-untyped-def]
    """Resolve a PettingZoo ParallelEnv from an env-id spec.

    PettingZoo 1.26 deprecated the MPE suite; the maintained replacement
    is the standalone `mpe2` package. We try mpe2 first when the user
    asks for a `simple_*` env, then fall back to the in-tree suites
    (sisl, butterfly, magent, atari).

    Examples accepted:
      pettingzoo:simple_spread_v3        → tries mpe2.simple_spread_v3, then pettingzoo.mpe.simple_spread_v3
      pettingzoo:sisl.pursuit_v4         → pettingzoo.sisl.pursuit_v4
      mpe2:simple_spread_v3              → mpe2.simple_spread_v3
      simple_spread_v3                   → tries mpe2 first
    """
    import importlib

    name = env_id
    for prefix in ("pettingzoo:", "mpe2:"):
        if name.startswith(prefix):
            name = name[len(prefix) :]
            break

    candidates: list[str] = []
    if name.startswith("simple_"):
        # MPE-style envs live in mpe2 in modern installs.
        candidates.append(f"mpe2.{name}")
        candidates.append(f"pettingzoo.mpe.{name}")
    elif "." in name:
        # Explicit suite: `sisl.pursuit_v4` → pettingzoo.sisl.pursuit_v4.
        candidates.append(f"pettingzoo.{name}")
        candidates.append(f"mpe2.{name}")
    else:
        # Bare name: try mpe2, then each in-tree suite.
        candidates.append(f"mpe2.{name}")
        for suite in ("sisl", "butterfly", "atari", "magent"):
            candidates.append(f"pettingzoo.{suite}.{name}")

    last_err: Exception | None = None
    for candidate in candidates:
        try:
            mod = importlib.import_module(candidate)
        except ImportError as e:
            last_err = e
            continue
        if not hasattr(mod, "parallel_env"):
            last_err = RuntimeError(
                f"module '{candidate}' does not expose parallel_env()"
            )
            continue
        env = mod.parallel_env()
        env.reset(seed=seed)
        return env

    raise RuntimeError(
        f"could not resolve PettingZoo env {env_id!r}. Tried: {candidates}. "
        f"Last error: {last_err}. Install with `uv sync --extra marl`; for MPE "
        f"envs also `uv pip install mpe2`."
    )


def train(cfg: MAPPOConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
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

    try:
        env = _make_pettingzoo_env(cfg.env_id, cfg.seed)
    except Exception as e:  # noqa: BLE001
        streamer.finished(
            reason="error",
            error=(
                f"failed to load PettingZoo env {cfg.env_id!r}: {type(e).__name__}: {e}. "
                f"Install with `cd vibe-rl-py && uv sync --extra marl`."
            ),
        )
        return {"error": str(e)}

    agents = list(env.possible_agents)
    if not agents:
        streamer.finished(reason="error", error="env has no agents")
        return {"error": "no agents"}

    # Discover per-agent obs / action shapes. Most MPE envs have
    # homogeneous agents; we still handle heterogeneous by keeping
    # per-agent actor instances.
    obs_dims: dict[str, int] = {}
    is_discrete: dict[str, bool] = {}
    action_sizes: dict[str, int] = {}
    for agent in agents:
        obs_space = env.observation_space(agent)
        act_space = env.action_space(agent)
        obs_dim = int(np.prod(obs_space.shape)) if hasattr(obs_space, "shape") else 0
        obs_dims[agent] = obs_dim
        # PettingZoo Discrete vs Box detection.
        space_class = type(act_space).__name__
        if space_class == "Discrete":
            is_discrete[agent] = True
            action_sizes[agent] = int(act_space.n)
        elif space_class == "Box":
            is_discrete[agent] = False
            action_sizes[agent] = int(np.prod(act_space.shape))
        else:
            streamer.finished(
                reason="error",
                error=f"unsupported action space {space_class} for agent {agent!r}",
            )
            return {"error": "unsupported space"}

    joint_obs_dim = sum(obs_dims.values())

    # Build actors. share_actor=True collapses to a single shared net
    # across agents (parameter sharing — common for fully cooperative).
    if cfg.share_actor and len(set(obs_dims.values())) == 1 and len(set(action_sizes.values())) == 1:
        first = agents[0]
        if is_discrete[first]:
            shared_actor: nn.Module = _DiscreteActor(obs_dims[first], action_sizes[first]).to(device)
        else:
            shared_actor = _ContinuousActor(obs_dims[first], action_sizes[first]).to(device)
        actors: dict[str, nn.Module] = {a: shared_actor for a in agents}
    else:
        actors = {}
        for agent in agents:
            if is_discrete[agent]:
                actors[agent] = _DiscreteActor(obs_dims[agent], action_sizes[agent]).to(device)
            else:
                actors[agent] = _ContinuousActor(obs_dims[agent], action_sizes[agent]).to(device)

    critic = _CentralizedCritic(joint_obs_dim).to(device)

    # Optimize all unique parameters jointly. Use a set so a shared
    # actor's parameters aren't doubled.
    seen_ids: set[int] = set()
    params: list[nn.Parameter] = []
    for actor in actors.values():
        for p in actor.parameters():
            if id(p) not in seen_ids:
                seen_ids.add(id(p))
                params.append(p)
    for p in critic.parameters():
        params.append(p)
    optimizer = optim.Adam(params, lr=cfg.learning_rate, eps=1e-5)

    # Rollout buffers (per-agent observations + actions + logprobs +
    # rewards, plus joint values + dones at the env level).
    def _new_buffers() -> dict[str, Any]:
        return {
            "obs": {a: np.zeros((cfg.num_steps, obs_dims[a]), dtype=np.float32) for a in agents},
            "act": {a: (
                np.zeros((cfg.num_steps,), dtype=np.int64) if is_discrete[a]
                else np.zeros((cfg.num_steps, action_sizes[a]), dtype=np.float32)
            ) for a in agents},
            "logp": {a: np.zeros((cfg.num_steps,), dtype=np.float32) for a in agents},
            "rew": {a: np.zeros((cfg.num_steps,), dtype=np.float32) for a in agents},
            "done": np.zeros((cfg.num_steps,), dtype=np.float32),
            "joint_obs": np.zeros((cfg.num_steps, joint_obs_dim), dtype=np.float32),
            "value": np.zeros((cfg.num_steps,), dtype=np.float32),
        }

    global_step = 0
    next_obs_dict, _ = env.reset(seed=cfg.seed)
    last_episode_returns: list[float] = []
    cumulative_episode_return = 0.0
    last_seen_episode_idx = 0
    episode_step_in_run = 0
    final_reward_mean: float | None = None

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    )
    workspace_path_obj = Path(cfg.workspace_path)

    t0 = time.monotonic()
    last_checkpoint_step = 0

    for iteration in range(1, cfg.num_iterations + 1):
        if runtime.should_stop():
            break

        if cfg.anneal_lr:
            frac = 1.0 - (iteration - 1.0) / max(cfg.num_iterations, 1)
            optimizer.param_groups[0]["lr"] = frac * cfg.learning_rate

        buf = _new_buffers()

        for step in range(cfg.num_steps):
            global_step += len(agents)

            # Build joint obs (concat in agent order) and per-agent obs.
            joint_obs = []
            actions_to_send: dict[str, Any] = {}
            for agent in agents:
                obs_np = np.asarray(next_obs_dict[agent], dtype=np.float32).reshape(-1)
                buf["obs"][agent][step] = obs_np
                joint_obs.append(obs_np)
                obs_t = torch.tensor(obs_np, dtype=torch.float32, device=device).unsqueeze(0)

                with torch.no_grad():
                    if is_discrete[agent]:
                        logits = actors[agent](obs_t)
                        dist = Categorical(logits=logits)
                        action = dist.sample()
                        logp = dist.log_prob(action)
                        a_np = int(action.item())
                        buf["act"][agent][step] = a_np
                        buf["logp"][agent][step] = float(logp.item())
                        actions_to_send[agent] = a_np
                    else:
                        actor: _ContinuousActor = actors[agent]  # type: ignore[assignment]
                        mean = actor.mean(obs_t)
                        std = actor.logstd.exp().expand_as(mean)
                        dist = Normal(mean, std)
                        action = dist.sample()
                        logp = dist.log_prob(action).sum(-1)
                        a_np = action.squeeze(0).cpu().numpy().astype(np.float32)
                        buf["act"][agent][step] = a_np
                        buf["logp"][agent][step] = float(logp.item())
                        actions_to_send[agent] = a_np

            joint_obs_arr = np.concatenate(joint_obs, axis=0)
            buf["joint_obs"][step] = joint_obs_arr
            with torch.no_grad():
                v = critic(torch.tensor(joint_obs_arr, dtype=torch.float32, device=device).unsqueeze(0))
                buf["value"][step] = float(v.item())

            next_obs_dict, rewards, terminations, truncations, _ = env.step(actions_to_send)
            done = any(terminations.values()) or any(truncations.values())
            buf["done"][step] = 1.0 if done else 0.0
            for agent in agents:
                buf["rew"][agent][step] = float(rewards.get(agent, 0.0))
                cumulative_episode_return += float(rewards.get(agent, 0.0))
            episode_step_in_run += 1

            if done:
                last_seen_episode_idx += 1
                # Joint-return aggregate (cooperative reward summed over agents).
                last_episode_returns.append(cumulative_episode_return)
                streamer.episode(
                    idx=last_seen_episode_idx,
                    timestep=global_step,
                    reward=cumulative_episode_return,
                    length=episode_step_in_run,
                    success=None,
                    duration_ms=0,
                )
                cumulative_episode_return = 0.0
                episode_step_in_run = 0
                next_obs_dict, _ = env.reset(seed=cfg.seed + global_step)

        # Bootstrap value for the final state.
        joint_next = np.concatenate(
            [np.asarray(next_obs_dict[a], dtype=np.float32).reshape(-1) for a in agents]
        )
        with torch.no_grad():
            next_value = float(
                critic(
                    torch.tensor(joint_next, dtype=torch.float32, device=device).unsqueeze(0)
                ).item()
            )

        # GAE on the cooperative joint reward (sum over agents per step).
        joint_rewards = np.zeros((cfg.num_steps,), dtype=np.float32)
        for agent in agents:
            joint_rewards += buf["rew"][agent]
        advantages = np.zeros_like(joint_rewards)
        lastgaelam = 0.0
        for t in reversed(range(cfg.num_steps)):
            if t == cfg.num_steps - 1:
                nextnonterminal = 1.0 - buf["done"][t]
                nextvalue = next_value
            else:
                nextnonterminal = 1.0 - buf["done"][t + 1]
                nextvalue = buf["value"][t + 1]
            delta = joint_rewards[t] + cfg.gamma * nextvalue * nextnonterminal - buf["value"][t]
            advantages[t] = lastgaelam = (
                delta + cfg.gamma * cfg.gae_lambda * nextnonterminal * lastgaelam
            )
        returns = advantages + buf["value"]

        # Flatten + tensorize for the update.
        b_joint_obs = torch.tensor(buf["joint_obs"], dtype=torch.float32, device=device)
        b_advantages = torch.tensor(advantages, dtype=torch.float32, device=device)
        b_returns = torch.tensor(returns, dtype=torch.float32, device=device)
        b_values = torch.tensor(buf["value"], dtype=torch.float32, device=device)

        b_obs_per_agent = {
            a: torch.tensor(buf["obs"][a], dtype=torch.float32, device=device) for a in agents
        }
        b_act_per_agent = {
            a: (
                torch.tensor(buf["act"][a], dtype=torch.long, device=device) if is_discrete[a]
                else torch.tensor(buf["act"][a], dtype=torch.float32, device=device)
            ) for a in agents
        }
        b_logp_per_agent = {
            a: torch.tensor(buf["logp"][a], dtype=torch.float32, device=device) for a in agents
        }

        b_inds = np.arange(cfg.num_steps)
        minibatch_size = max(1, cfg.num_steps // cfg.num_minibatches)

        pg_loss_v = 0.0
        v_loss_v = 0.0
        entropy_v = 0.0
        approx_kl_v: float = 0.0

        for _epoch in range(cfg.update_epochs):
            np.random.shuffle(b_inds)
            for start in range(0, cfg.num_steps, minibatch_size):
                end = start + minibatch_size
                mb_inds = b_inds[start:end]

                # Per-agent actor loss (sum); shared scalar advantage for
                # cooperative reward.
                mb_adv = b_advantages[mb_inds]
                mb_adv = (mb_adv - mb_adv.mean()) / (mb_adv.std() + 1e-8)
                pg_total = torch.tensor(0.0, device=device)
                ent_total = torch.tensor(0.0, device=device)
                approx_kl_total = 0.0
                for agent in agents:
                    obs_mb = b_obs_per_agent[agent][mb_inds]
                    act_mb = b_act_per_agent[agent][mb_inds]
                    old_logp_mb = b_logp_per_agent[agent][mb_inds]
                    if is_discrete[agent]:
                        logits = actors[agent](obs_mb)
                        dist = Categorical(logits=logits)
                        new_logp = dist.log_prob(act_mb)
                        entropy = dist.entropy()
                    else:
                        actor = actors[agent]
                        mean = actor.mean(obs_mb)
                        std = actor.logstd.exp().expand_as(mean)
                        dist = Normal(mean, std)
                        new_logp = dist.log_prob(act_mb).sum(-1)
                        entropy = dist.entropy().sum(-1)
                    logratio = new_logp - old_logp_mb
                    ratio = logratio.exp()
                    with torch.no_grad():
                        approx_kl_total += float(((ratio - 1) - logratio).mean().item())
                    pg1 = -mb_adv * ratio
                    pg2 = -mb_adv * torch.clamp(ratio, 1 - cfg.clip_coef, 1 + cfg.clip_coef)
                    pg_total = pg_total + torch.max(pg1, pg2).mean()
                    ent_total = ent_total + entropy.mean()

                approx_kl_v = approx_kl_total / max(len(agents), 1)

                # Centralized critic loss.
                new_value = critic(b_joint_obs[mb_inds]).view(-1)
                v_loss = 0.5 * ((new_value - b_returns[mb_inds]) ** 2).mean()
                v_loss_v = float(v_loss.item())

                pg_loss_v = float((pg_total / max(len(agents), 1)).item())
                entropy_v = float((ent_total / max(len(agents), 1)).item())

                loss = (pg_total / max(len(agents), 1)) - cfg.ent_coef * (
                    ent_total / max(len(agents), 1)
                ) + v_loss * cfg.vf_coef

                optimizer.zero_grad()
                loss.backward()
                nn.utils.clip_grad_norm_(params, cfg.max_grad_norm)
                optimizer.step()

        sps = int(global_step / max(time.monotonic() - t0, 1e-6))
        recent_mean = float(np.mean(last_episode_returns[-100:])) if last_episode_returns else 0.0
        final_reward_mean = recent_mean

        # Per-agent mean reward over the rollout.
        per_agent_reward = {a: float(np.sum(buf["rew"][a])) for a in agents}

        streamer.tick(
            tick=iteration,
            timestep=global_step,
            payload={
                "policy_loss": pg_loss_v,
                "value_loss": v_loss_v,
                "centralized_value_loss": v_loss_v,
                "entropy": entropy_v,
                "approx_kl": approx_kl_v,
                "learning_rate": optimizer.param_groups[0]["lr"],
                "sps": sps,
                "recent_reward_mean_100": recent_mean,
                "agents": agents,
                "per_agent_reward": per_agent_reward,
                "share_actor": cfg.share_actor,
            },
        )

        if global_step - last_checkpoint_step >= cfg.checkpoint_every_steps:
            actor_state = {a: actors[a].state_dict() for a in agents} if not cfg.share_actor else {agents[0]: actors[agents[0]].state_dict()}
            info = save_checkpoint(
                artifact_dir=artifact_dir,
                timestep=global_step,
                state={
                    "actors": actor_state,
                    "critic": critic.state_dict(),
                    "optimizer": optimizer.state_dict(),
                },
                metadata={
                    "algorithm": "MAPPO",
                    "kind": "train",
                    "env_id": cfg.env_id,
                    "agents": agents,
                    "obs_dims": obs_dims,
                    "action_sizes": action_sizes,
                    "is_discrete": is_discrete,
                    "share_actor": cfg.share_actor,
                    "joint_obs_dim": joint_obs_dim,
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

    # Final checkpoint.
    actor_state = (
        {a: actors[a].state_dict() for a in agents}
        if not cfg.share_actor
        else {agents[0]: actors[agents[0]].state_dict()}
    )
    info = save_checkpoint(
        artifact_dir=artifact_dir,
        timestep=global_step,
        state={
            "actors": actor_state,
            "critic": critic.state_dict(),
            "optimizer": optimizer.state_dict(),
        },
        metadata={
            "algorithm": "MAPPO",
            "kind": "train",
            "env_id": cfg.env_id,
            "agents": agents,
            "obs_dims": obs_dims,
            "action_sizes": action_sizes,
            "is_discrete": is_discrete,
            "share_actor": cfg.share_actor,
            "joint_obs_dim": joint_obs_dim,
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
        "iterations_completed": iteration,
        "agents": agents,
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
