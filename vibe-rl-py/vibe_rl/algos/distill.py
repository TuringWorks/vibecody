"""Slice 7a — Policy distillation (single-teacher).

Implementation strategy: reuse slice 2's PPO infrastructure (rollout +
GAE + clipped surrogate + value loss + entropy bonus), and add one extra
term to the loss: a KL-divergence between the student's action
distribution and the teacher's. The teacher's checkpoint loads at startup,
its forward pass runs in `torch.no_grad()` on every batch, and the
distill_coef weights the KL term in the combined loss.

This shares 95% of the structure with `vibe_rl.algos.ppo` — by design.
The student trains on its OWN rollouts (it acts in the env) but is
nudged toward the teacher's action distribution via the KL term. That's
the "policy distillation" formulation from Rusu et al. 2016, generalized
for PPO.

Wire format / streamer protocol matches PPO 1:1 — extra payload fields
(`distill_kl`, `distill_coef`) appear in the per-tick dict so the panel
can show how aggressively the student is being pulled.
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
from vibe_rl.algos.ppo import _ContinuousAgent, _DiscreteAgent
from vibe_rl.checkpoint import save_checkpoint
from vibe_rl.envs.registry import make_env
from vibe_rl.envs.wrappers import MonitorWrapper


@dataclass
class DistillConfig:
    run_id: str
    env_id: str
    teacher_checkpoint: str
    total_timesteps: int = 100_000
    learning_rate: float = 3e-4
    num_envs: int = 4
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
    distill_coef: float = 1.0
    seed: int = 42

    workspace_path: str = "."
    artifact_dir: str = ""
    checkpoint_every_steps: int = 50_000

    @property
    def batch_size(self) -> int:
        return self.num_envs * self.num_steps

    @property
    def minibatch_size(self) -> int:
        return self.batch_size // self.num_minibatches

    @property
    def num_iterations(self) -> int:
        return self.total_timesteps // self.batch_size


def _layer_init(layer, std=math.sqrt(2), bias_const=0.0):  # type: ignore[no-untyped-def]
    nn.init.orthogonal_(layer.weight, std)
    nn.init.constant_(layer.bias, bias_const)
    return layer


def _load_teacher(checkpoint_path: Path, device: torch.device, obs_dim: int, action_kind: str, n_actions: int, action_dim: int) -> nn.Module:
    """Load the teacher policy state_dict and freeze it."""
    metadata_path = checkpoint_path.with_suffix(".json")
    if not metadata_path.is_file():
        raise FileNotFoundError(
            f"teacher checkpoint metadata sidecar missing: {metadata_path}"
        )
    if action_kind == "discrete":
        teacher: nn.Module = _DiscreteAgent(obs_dim, n_actions).to(device)
    else:
        teacher = _ContinuousAgent(obs_dim, action_dim).to(device)
    state = torch.load(checkpoint_path, map_location=device, weights_only=True)
    if "policy" in state:
        teacher.load_state_dict(state["policy"], strict=False)
    else:
        teacher.load_state_dict(state, strict=False)
    teacher.eval()
    for p in teacher.parameters():
        p.requires_grad = False
    return teacher


def _make_vector_env(cfg: DistillConfig, streamer) -> tuple[Any, str, bool]:  # type: ignore[no-untyped-def]
    thunks = [make_env(cfg.env_id, seed=cfg.seed + i) for i in range(cfg.num_envs)]
    envs = gym.vector.SyncVectorEnv(thunks)
    envs.envs[0] = MonitorWrapper(envs.envs[0], streamer)  # type: ignore[attr-defined]
    if isinstance(envs.single_action_space, gym.spaces.Discrete):
        return envs, "discrete", False
    if isinstance(envs.single_action_space, gym.spaces.Box):
        return envs, "continuous", True
    raise ValueError(
        f"Distill supports Discrete or Box action spaces — got {type(envs.single_action_space).__name__}"
    )


def train(cfg: DistillConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
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
        student: nn.Module = _ContinuousAgent(obs_dim, action_dim).to(device)
    else:
        n_actions = int(envs.single_action_space.n)
        action_dim = 1
        student = _DiscreteAgent(obs_dim, n_actions).to(device)

    teacher = _load_teacher(
        Path(cfg.teacher_checkpoint),
        device,
        obs_dim,
        action_kind,
        n_actions if not continuous else 0,
        action_dim,
    )

    optimizer = optim.Adam(student.parameters(), lr=cfg.learning_rate, eps=1e-5)

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
    monitor = envs.envs[0]  # type: ignore[attr-defined]
    last_seen_episode_idx = 0

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    workspace_path_obj = Path(cfg.workspace_path)

    t0 = time.monotonic()

    for iteration in range(1, cfg.num_iterations + 1):
        if runtime.should_stop():
            break

        if cfg.anneal_lr:
            frac = 1.0 - (iteration - 1.0) / max(cfg.num_iterations, 1)
            optimizer.param_groups[0]["lr"] = frac * cfg.learning_rate

        # Rollout — student acts in the env.
        for step in range(cfg.num_steps):
            global_step += cfg.num_envs
            obs_buf[step] = next_obs
            done_buf[step] = next_done

            with torch.no_grad():
                action, logprob, _, value = student.get_action_and_value(next_obs)
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

        # GAE
        with torch.no_grad():
            next_value = student.get_value(next_obs).reshape(1, -1)
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

        b_obs = obs_buf.reshape((-1, obs_dim))
        b_logprobs = logprob_buf.reshape(-1)
        if continuous:
            b_actions = act_buf.reshape((-1,) + envs.single_action_space.shape)
        else:
            b_actions = act_buf.reshape(-1)
        b_advantages = advantages.reshape(-1)
        b_returns = returns.reshape(-1)
        b_values = val_buf.reshape(-1)

        b_inds = np.arange(cfg.batch_size)
        pg_loss_v = 0.0
        v_loss_v = 0.0
        entropy_v = 0.0
        distill_kl_v = 0.0
        approx_kl_v: float | None = None

        for _epoch in range(cfg.update_epochs):
            np.random.shuffle(b_inds)
            for start in range(0, cfg.batch_size, cfg.minibatch_size):
                end = start + cfg.minibatch_size
                mb_inds = b_inds[start:end]

                _, newlogprob, entropy, newvalue = student.get_action_and_value(
                    b_obs[mb_inds], b_actions[mb_inds]
                )
                logratio = newlogprob - b_logprobs[mb_inds]
                ratio = logratio.exp()

                with torch.no_grad():
                    approx_kl_v = float(((ratio - 1) - logratio).mean().item())

                mb_advantages = b_advantages[mb_inds]
                mb_advantages = (mb_advantages - mb_advantages.mean()) / (mb_advantages.std() + 1e-8)

                pg_loss1 = -mb_advantages * ratio
                pg_loss2 = -mb_advantages * torch.clamp(ratio, 1 - cfg.clip_coef, 1 + cfg.clip_coef)
                pg_loss = torch.max(pg_loss1, pg_loss2).mean()
                pg_loss_v = float(pg_loss.item())

                newvalue = newvalue.view(-1)
                v_loss = 0.5 * ((newvalue - b_returns[mb_inds]) ** 2).mean()
                v_loss_v = float(v_loss.item())

                entropy_loss = entropy.mean()
                entropy_v = float(entropy_loss.item())

                # Distillation KL — KL(student || teacher) over the full
                # action distribution. For Discrete spaces we use the
                # softmax over logits; for Continuous we use the Normal
                # mean/logstd directly.
                if continuous:
                    mb_obs = b_obs[mb_inds]
                    student_mean = student.actor_mean(mb_obs)  # type: ignore[attr-defined]
                    student_std = student.actor_logstd.exp().expand_as(student_mean)  # type: ignore[attr-defined]
                    student_dist = Normal(student_mean, student_std)
                    with torch.no_grad():
                        teacher_mean = teacher.actor_mean(mb_obs)  # type: ignore[attr-defined]
                        teacher_std = teacher.actor_logstd.exp().expand_as(teacher_mean)  # type: ignore[attr-defined]
                        teacher_dist = Normal(teacher_mean, teacher_std)
                    distill_kl = torch.distributions.kl_divergence(student_dist, teacher_dist).sum(-1).mean()
                else:
                    mb_obs = b_obs[mb_inds]
                    student_logits = student.actor(mb_obs)  # type: ignore[attr-defined]
                    student_dist = Categorical(logits=student_logits)
                    with torch.no_grad():
                        teacher_logits = teacher.actor(mb_obs)  # type: ignore[attr-defined]
                        teacher_dist = Categorical(logits=teacher_logits)
                    distill_kl = torch.distributions.kl_divergence(student_dist, teacher_dist).mean()

                distill_kl_v = float(distill_kl.item())

                loss = (
                    pg_loss
                    - cfg.ent_coef * entropy_loss
                    + v_loss * cfg.vf_coef
                    + cfg.distill_coef * distill_kl
                )

                optimizer.zero_grad()
                loss.backward()
                nn.utils.clip_grad_norm_(student.parameters(), cfg.max_grad_norm)
                optimizer.step()

        sps = int(global_step / max(time.monotonic() - t0, 1e-6))
        current_idx = getattr(monitor, "_episode_idx", last_seen_episode_idx)
        if current_idx > last_seen_episode_idx:
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
                "distill_kl": distill_kl_v,
                "distill_coef": cfg.distill_coef,
                "learning_rate": optimizer.param_groups[0]["lr"],
                "sps": sps,
                "recent_reward_mean_100": recent_mean,
            },
        )

        if global_step - last_checkpoint_step >= cfg.checkpoint_every_steps:
            info = save_checkpoint(
                artifact_dir=artifact_dir,
                timestep=global_step,
                state={"policy": student.state_dict(), "optimizer": optimizer.state_dict()},
                metadata={
                    "algorithm": "PPO-Distill",
                    "kind": "distill",
                    "env_id": cfg.env_id,
                    "obs_dim": obs_dim,
                    "action_kind": action_kind,
                    "teacher_checkpoint": cfg.teacher_checkpoint,
                    "distill_coef": cfg.distill_coef,
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
        state={"policy": student.state_dict(), "optimizer": optimizer.state_dict()},
        metadata={
            "algorithm": "PPO-Distill",
            "kind": "distill",
            "env_id": cfg.env_id,
            "obs_dim": obs_dim,
            "action_kind": action_kind,
            "teacher_checkpoint": cfg.teacher_checkpoint,
            "distill_coef": cfg.distill_coef,
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
        "teacher_checkpoint": cfg.teacher_checkpoint,
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
