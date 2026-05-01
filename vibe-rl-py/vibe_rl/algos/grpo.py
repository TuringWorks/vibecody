"""Slice 7c-extras+1 — GRPO (Group Relative Policy Optimization).

DeepSeek's approach (Shao et al. 2024, "DeepSeekMath", refined in
DeepSeek-R1). The key idea: skip the learned value head entirely. For
each prompt sample G completions, score them with the reward model,
and use the group statistics (mean, std) to normalize the rewards into
advantages:

    A(x, y_i) = (r(x, y_i) - mean_j r(x, y_j)) / (std_j r(x, y_j) + ε)

This advantage is *terminal* — applied to every response token of
generation i. The PPO-style clipped surrogate then optimizes:

    L = -E[ min( ρ · A, clip(ρ, 1-ε, 1+ε) · A ) ] + β · KL(π || π_ref)

where ρ is the per-token policy ratio. KL is added directly to the
loss (rather than baked into per-token rewards) because we don't have
a value baseline to soak it up.

Why GRPO over PPO RLHF:
  - **No value head** → fewer params, no value-loss to balance.
  - **Group-relative** → automatically rescales rewards across batches
    of varying difficulty (a hard prompt may have all-low rewards but
    the relative ranking still produces signal).
  - **Simpler tuning** — fewer hyperparameters (no GAE λ, no value
    coefficient).

Wire payload mirrors PPO's:
  - `group_size` (G), `group_mean_reward`, `group_std_reward`,
  - `policy_loss`, `kl_to_reference`, `clip_fraction`, `entropy`,
  - `terminal_reward_mean`, `kl_coef`.
"""

from __future__ import annotations

import random
import sqlite3
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

from vibe_rl import runtime
from vibe_rl.algos.ppo_rlhf import _logp_only, _read_prompts, _score_with_rm
from vibe_rl.checkpoint import _sha256_file  # type: ignore[attr-defined]


@dataclass
class GRPOConfig:
    run_id: str
    base_model_id: str = "distilgpt2"
    reference_model_id: str | None = None
    reward_model_id: str = ""
    group_size: int = 4  # G — number of completions sampled per prompt
    kl_coef: float = 0.05
    clip_coef: float = 0.2
    entropy_coef: float = 1e-3
    learning_rate: float = 5e-6
    max_new_tokens: int = 32
    max_prompt_len: int = 128
    n_prompts_per_iter: int = 1  # smaller because we sample group_size completions per
    update_epochs: int = 2
    num_iterations: int = 4
    grad_accum_steps: int = 1
    seed: int = 42
    suite_id: str | None = None
    workspace_path: str = "."
    artifact_dir: str = ""


def train(cfg: GRPOConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    if not cfg.reward_model_id:
        streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device="cpu")
        streamer.finished(
            reason="error",
            error="reward_model_id is required. Train one first with algorithm: REWARD_MODEL.",
        )
        return {"error": "no rm"}
    if cfg.group_size < 2:
        streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device="cpu")
        streamer.finished(
            reason="error",
            error=f"group_size must be ≥ 2 (got {cfg.group_size}) — GRPO needs ≥ 2 samples to compute std",
        )
        return {"error": "bad group"}

    random.seed(cfg.seed)
    np.random.seed(cfg.seed)
    torch.manual_seed(cfg.seed)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device=str(device))

    try:
        from transformers import (  # type: ignore[import-not-found]
            AutoModelForCausalLM,
            AutoModelForSequenceClassification,
            AutoTokenizer,
        )
    except ImportError:
        streamer.finished(reason="error", error="transformers missing — install [rlhf] extra")
        return {"error": "transformers missing"}

    workspace_db = Path(cfg.workspace_path) / ".vibecli" / "workspace.db"
    if not workspace_db.is_file():
        streamer.finished(reason="error", error=f"workspace.db not found at {workspace_db}")
        return {"error": "no workspace db"}
    prompts_all = _read_prompts(workspace_db, cfg.suite_id)
    if not prompts_all:
        streamer.finished(reason="error", error="no prompts in rl_preferences")
        return {"error": "no prompts"}

    tokenizer = AutoTokenizer.from_pretrained(cfg.base_model_id)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token
    tokenizer.padding_side = "left"

    policy = AutoModelForCausalLM.from_pretrained(cfg.base_model_id).to(device)
    policy.config.pad_token_id = tokenizer.pad_token_id
    policy.train()
    reference_id = cfg.reference_model_id or cfg.base_model_id
    reference = AutoModelForCausalLM.from_pretrained(reference_id).to(device)
    reference.eval()
    for p in reference.parameters():
        p.requires_grad = False

    rm = AutoModelForSequenceClassification.from_pretrained(
        cfg.reward_model_id, num_labels=1
    ).to(device)
    if rm.config.pad_token_id is None:
        rm.config.pad_token_id = tokenizer.pad_token_id
    rm.eval()
    for p in rm.parameters():
        p.requires_grad = False

    optimizer = torch.optim.AdamW(policy.parameters(), lr=cfg.learning_rate)

    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    )
    artifact_dir.mkdir(parents=True, exist_ok=True)

    t0 = time.monotonic()
    tick_idx = 0
    best_terminal: float = -1e9

    for iteration in range(1, cfg.num_iterations + 1):
        if runtime.should_stop():
            break

        # Sample one prompt and tile it group_size times.
        prompts_iter = [random.choice(prompts_all) for _ in range(cfg.n_prompts_per_iter)]
        # Each prompt → group_size copies, generate group_size completions per prompt.
        tiled_prompts = [p for p in prompts_iter for _ in range(cfg.group_size)]
        prompt_enc = tokenizer(
            tiled_prompts,
            padding=True,
            truncation=True,
            max_length=cfg.max_prompt_len,
            return_tensors="pt",
        ).to(device)
        prompt_len = prompt_enc["input_ids"].shape[1]

        with torch.no_grad():
            gen = policy.generate(
                input_ids=prompt_enc["input_ids"],
                attention_mask=prompt_enc["attention_mask"],
                max_new_tokens=cfg.max_new_tokens,
                do_sample=True,
                top_k=50,
                top_p=0.95,
                pad_token_id=tokenizer.pad_token_id,
            )
        full_ids = gen
        full_mask = torch.ones_like(full_ids)
        full_mask[full_ids == tokenizer.pad_token_id] = 0
        response_mask = torch.zeros_like(full_ids)
        response_mask[:, prompt_len:] = full_mask[:, prompt_len:]

        with torch.no_grad():
            old_logp_full, _ = _logp_only(
                policy, tokenizer, full_ids, full_mask, response_mask
            )
            ref_logp_full, _ = _logp_only(
                reference, tokenizer, full_ids, full_mask, response_mask
            )
            response_mask_shifted = response_mask[:, 1:]

        # ── Score each generation with the RM ─────────────────────────────
        decoded = tokenizer.batch_decode(full_ids, skip_special_tokens=True)
        with torch.no_grad():
            terminal_rewards = _score_with_rm(
                rm, tokenizer, decoded, cfg.max_prompt_len + cfg.max_new_tokens, device
            )
        terminal_reward_mean = float(terminal_rewards.mean().item())
        best_terminal = max(best_terminal, terminal_reward_mean)

        # ── Group-relative advantage ─────────────────────────────────────
        # Reshape to (n_prompts_per_iter, group_size) and z-score within
        # each group. Broadcast back to (batch,).
        rewards_grouped = terminal_rewards.view(cfg.n_prompts_per_iter, cfg.group_size)
        group_mean = rewards_grouped.mean(dim=-1, keepdim=True)
        group_std = rewards_grouped.std(dim=-1, keepdim=True).clamp(min=1e-6)
        advantages_per_group = (rewards_grouped - group_mean) / group_std
        # Same scalar advantage for every response token of that generation.
        per_token_advantage = advantages_per_group.view(-1, 1).expand(
            -1, response_mask_shifted.shape[1]
        )

        # ── PPO-style clipped surrogate over the per-token policy ratio ──
        for epoch in range(cfg.update_epochs):
            new_logp_full, _ = _logp_only(
                policy, tokenizer, full_ids, full_mask, response_mask
            )
            valid = response_mask_shifted.float()
            n_valid = valid.sum().clamp(min=1)

            log_ratio = (new_logp_full - old_logp_full) * valid
            ratio = log_ratio.exp()

            pg1 = -per_token_advantage * ratio
            pg2 = -per_token_advantage * torch.clamp(
                ratio, 1 - cfg.clip_coef, 1 + cfg.clip_coef
            )
            pg_loss = (torch.max(pg1, pg2) * valid).sum() / n_valid

            # KL penalty added directly to the loss (no value baseline).
            with torch.no_grad():
                kl_per_token = (old_logp_full - ref_logp_full) * valid
            kl_loss = ((new_logp_full - ref_logp_full) * valid).sum() / n_valid
            entropy = (-new_logp_full * valid).sum() / n_valid

            with torch.no_grad():
                clip_frac = (
                    ((ratio - 1.0).abs() > cfg.clip_coef).float() * valid
                ).sum() / n_valid

            loss = pg_loss + cfg.kl_coef * kl_loss - cfg.entropy_coef * entropy
            loss_to_backward = loss / max(cfg.grad_accum_steps, 1)
            loss_to_backward.backward()
            if (epoch + 1) % max(cfg.grad_accum_steps, 1) == 0:
                torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)

            tick_idx += 1
            sps = (iteration * cfg.n_prompts_per_iter * cfg.group_size) / max(
                time.monotonic() - t0, 1e-6
            )
            streamer.tick(
                tick=tick_idx,
                timestep=iteration * cfg.n_prompts_per_iter * cfg.group_size,
                payload={
                    "policy_loss": float(pg_loss.item()),
                    "kl_loss": float(kl_loss.item()),
                    "entropy": float(entropy.item()),
                    "clip_fraction": float(clip_frac.item()),
                    "kl_coef": cfg.kl_coef,
                    "terminal_reward_mean": terminal_reward_mean,
                    "group_size": cfg.group_size,
                    "group_mean_reward": float(group_mean.mean().item()),
                    "group_std_reward": float(group_std.mean().item()),
                    "ratio_mean": float(((ratio * valid).sum() / n_valid).item()),
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "iteration": iteration,
                    "epoch": epoch,
                    "n_response_tokens": int(n_valid.item()),
                },
            )

    final_dir = artifact_dir / "aligned-model"
    policy.save_pretrained(str(final_dir), safe_serialization=True)
    tokenizer.save_pretrained(str(final_dir))
    weights_path = final_dir / "model.safetensors"
    if not weights_path.is_file():
        for cand in final_dir.glob("pytorch_model*.bin"):
            weights_path = cand
            break
    size_bytes = weights_path.stat().st_size if weights_path.is_file() else 0
    sha = _sha256_file(weights_path) if weights_path.is_file() else ""
    workspace = Path(cfg.workspace_path).resolve()
    try:
        rel_str = str(weights_path.resolve().relative_to(workspace)).replace("\\", "/")
    except ValueError:
        rel_str = str(weights_path)
    streamer.checkpoint(
        timestep=cfg.num_iterations * cfg.n_prompts_per_iter * cfg.group_size,
        rel_path=rel_str,
        sha256=sha,
        size_bytes=size_bytes,
    )
    streamer.finished(reason="done", final_reward_mean=best_terminal)
    return {
        "n_prompts": len(prompts_all),
        "best_terminal_reward": best_terminal,
        "aligned_model_dir": str(final_dir),
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
