"""Slice 7c-extras+1 — Token-level PPO RLHF using a trained reward model.

The classical InstructGPT recipe (Ouyang et al. 2022). Pairs with the
reward model produced by `vibe_rl.algos.reward_model`:

  Stage 1 (slice 7c-extras): train an RM on preference pairs.
  Stage 2 (this file)        : PPO over the LM, scoring generations
                               with the RM, with a KL penalty against
                               a frozen reference policy.

Per-batch loop:

  1. Read prompts from rl_preferences (re-using the suite the RM was
     trained on; no preferences needed at this stage — only prompts).
  2. Generate responses from the policy (sampling).
  3. Compute old log-probs (from policy at gen time, detached).
  4. Compute ref log-probs (frozen reference).
  5. Score (prompt + response) with the RM → terminal scalar reward.
  6. Build per-token reward: r_t = -β · (logp_pol − logp_ref); the last
     response token's reward also adds the RM score.
  7. Compute values via a learned linear head over LM hidden states.
  8. GAE → advantages, returns.
  9. K PPO epochs of clipped-surrogate + value MSE + small entropy bonus.

Key configurables:
  - `kl_coef`       — β in the per-token KL penalty (typical 0.02–0.10).
  - `value_coef`    — weight on the value MSE term in the combined loss.
  - `entropy_coef`  — encourages exploration; small (1e-3) for LMs.
  - `max_new_tokens`— cap on generation length per response.

Wire format mirrors PPO's tick payload, with extra fields:
  - `terminal_reward_mean` (avg RM score across the batch),
  - `kl_to_reference`, `kl_coef`,
  - `value_loss`, `policy_loss`, `entropy`, `clip_fraction`.

The RM is loaded from `--reward_model_id` (path to a HuggingFace folder
saved by `reward_model.py`). The base policy + reference default to
the same `base_model_id`.
"""

from __future__ import annotations

import json
import math
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
from vibe_rl.checkpoint import _sha256_file  # type: ignore[attr-defined]


@dataclass
class PPORLHFConfig:
    run_id: str
    base_model_id: str = "distilgpt2"
    reference_model_id: str | None = None
    reward_model_id: str = ""  # path/HF id of the trained RM (slice 7c-extras output)
    kl_coef: float = 0.05
    clip_coef: float = 0.2
    value_coef: float = 0.5
    entropy_coef: float = 1e-3
    learning_rate: float = 5e-6
    max_new_tokens: int = 32
    max_prompt_len: int = 128
    batch_size: int = 2
    update_epochs: int = 2
    num_iterations: int = 4
    grad_accum_steps: int = 1
    seed: int = 42
    suite_id: str | None = None
    workspace_path: str = "."
    artifact_dir: str = ""


def _read_prompts(workspace_db: Path, suite_id: str | None) -> list[str]:
    conn = sqlite3.connect(str(workspace_db))
    try:
        if suite_id is None:
            rows = conn.execute("SELECT DISTINCT prompt FROM rl_preferences").fetchall()
        else:
            rows = conn.execute(
                "SELECT DISTINCT prompt FROM rl_preferences WHERE suite_id = ?",
                (suite_id,),
            ).fetchall()
    finally:
        conn.close()
    return [r[0] for r in rows]


class _ValueHead(nn.Module):
    """Linear value head over the LM's last hidden state (per-position)."""

    def __init__(self, hidden_dim: int) -> None:
        super().__init__()
        self.linear = nn.Linear(hidden_dim, 1)
        nn.init.zeros_(self.linear.bias)
        nn.init.normal_(self.linear.weight, std=0.01)

    def forward(self, hidden: torch.Tensor) -> torch.Tensor:  # type: ignore[override]
        return self.linear(hidden).squeeze(-1)  # (batch, seq)


def _logp_and_value(
    model,
    value_head: _ValueHead,
    tokenizer,
    input_ids: torch.Tensor,
    attention_mask: torch.Tensor,
    response_mask: torch.Tensor,
):  # type: ignore[no-untyped-def]
    """Forward pass returning per-token log-probs and per-token values
    over the response positions only.

    `response_mask` has 1 at positions that are response tokens (we
    train on these), 0 elsewhere. Shape matches input_ids.
    """
    outputs = model(
        input_ids=input_ids,
        attention_mask=attention_mask,
        output_hidden_states=True,
    )
    logits = outputs.logits[:, :-1, :]  # next-token prediction
    targets = input_ids[:, 1:]
    log_probs = F.log_softmax(logits, dim=-1)
    gathered = log_probs.gather(2, targets.unsqueeze(-1)).squeeze(-1)
    # Mask: only response positions count.
    mask = response_mask[:, 1:]

    last_hidden = outputs.hidden_states[-1][:, :-1, :]
    values = value_head(last_hidden)
    return gathered, values, mask


def _logp_only(
    model,
    tokenizer,
    input_ids: torch.Tensor,
    attention_mask: torch.Tensor,
    response_mask: torch.Tensor,
):  # type: ignore[no-untyped-def]
    outputs = model(input_ids=input_ids, attention_mask=attention_mask)
    logits = outputs.logits[:, :-1, :]
    targets = input_ids[:, 1:]
    log_probs = F.log_softmax(logits, dim=-1)
    gathered = log_probs.gather(2, targets.unsqueeze(-1)).squeeze(-1)
    mask = response_mask[:, 1:]
    return gathered, mask


def _score_with_rm(rm, tokenizer, full_texts: list[str], max_len: int, device: torch.device) -> torch.Tensor:  # type: ignore[no-untyped-def]
    enc = tokenizer(
        full_texts,
        padding=True,
        truncation=True,
        max_length=max_len,
        return_tensors="pt",
    ).to(device)
    out = rm(input_ids=enc["input_ids"], attention_mask=enc["attention_mask"])
    return out.logits.squeeze(-1)


def train(cfg: PPORLHFConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    if not cfg.reward_model_id:
        streamer.started(sidecar_version=_sidecar_version(), seed=cfg.seed, device="cpu")
        streamer.finished(
            reason="error",
            error=(
                "reward_model_id is required. Train one first with "
                "algorithm: REWARD_MODEL, then point at its artifact "
                "(reward-model/) here."
            ),
        )
        return {"error": "no rm"}

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
    tokenizer.padding_side = "left"  # generation expects left padding

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

    # Discover hidden dim from the policy's config.
    hidden_dim = getattr(policy.config, "hidden_size", None) or getattr(
        policy.config, "n_embd", None
    )
    if hidden_dim is None:
        streamer.finished(
            reason="error",
            error=f"could not infer hidden_size from {cfg.base_model_id} config",
        )
        return {"error": "no hidden dim"}
    value_head = _ValueHead(int(hidden_dim)).to(device)

    optimizer = torch.optim.AdamW(
        list(policy.parameters()) + list(value_head.parameters()),
        lr=cfg.learning_rate,
    )

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

        # ── Sample a batch of prompts ──────────────────────────────────────
        batch_prompts = [random.choice(prompts_all) for _ in range(cfg.batch_size)]
        prompt_enc = tokenizer(
            batch_prompts,
            padding=True,
            truncation=True,
            max_length=cfg.max_prompt_len,
            return_tensors="pt",
        ).to(device)
        prompt_len = prompt_enc["input_ids"].shape[1]

        # ── Generate ──────────────────────────────────────────────────────
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
        # gen shape: (batch, prompt_len + new_tokens)
        full_ids = gen
        full_mask = torch.ones_like(full_ids)
        full_mask[full_ids == tokenizer.pad_token_id] = 0
        # response_mask: 1 only on the new (response) tokens.
        response_mask = torch.zeros_like(full_ids)
        response_mask[:, prompt_len:] = full_mask[:, prompt_len:]

        # ── Compute old (current-policy, detached) and reference logprobs ─
        with torch.no_grad():
            old_logp_full, _ = _logp_only(
                policy, tokenizer, full_ids, full_mask, response_mask
            )
            ref_logp_full, _ = _logp_only(
                reference, tokenizer, full_ids, full_mask, response_mask
            )
            response_mask_shifted = response_mask[:, 1:]  # align to per-token positions

        # ── Score with RM ────────────────────────────────────────────────
        decoded = tokenizer.batch_decode(full_ids, skip_special_tokens=True)
        with torch.no_grad():
            terminal_rewards = _score_with_rm(
                rm, tokenizer, decoded, cfg.max_prompt_len + cfg.max_new_tokens, device
            )
        terminal_reward_mean = float(terminal_rewards.mean().item())
        best_terminal = max(best_terminal, terminal_reward_mean)

        # ── Build per-token rewards: -β · KL on each response position; ──
        #    the last response position also adds the terminal RM score.
        per_token_kl = old_logp_full - ref_logp_full
        per_token_rewards = -cfg.kl_coef * per_token_kl * response_mask_shifted.float()
        # Add terminal reward at the last response token of each row.
        last_response_idx = response_mask_shifted.sum(dim=1).long() - 1  # 0-indexed
        last_response_idx = torch.clamp(last_response_idx, min=0)
        for i in range(cfg.batch_size):
            if response_mask_shifted[i].sum() > 0:
                # Find the actual position of the last response token in the
                # shifted indexing: it's the rightmost 1 in row i.
                idx = (response_mask_shifted[i].nonzero(as_tuple=True)[0])
                if len(idx) > 0:
                    per_token_rewards[i, idx[-1].item()] += terminal_rewards[i]

        # ── Compute values + GAE ─────────────────────────────────────────
        with torch.no_grad():
            _, values_full, _ = _logp_and_value(
                policy, value_head, tokenizer, full_ids, full_mask, response_mask
            )
        # GAE iteratively from the right.
        advantages = torch.zeros_like(per_token_rewards)
        last_gae = torch.zeros(cfg.batch_size, device=device)
        gamma = 1.0  # token-level RLHF typically uses gamma=1
        gae_lambda = 0.95
        seq_len = per_token_rewards.shape[1]
        for t in reversed(range(seq_len)):
            mask_t = response_mask_shifted[:, t].float()
            if t == seq_len - 1:
                next_value = torch.zeros(cfg.batch_size, device=device)
                next_mask = torch.zeros(cfg.batch_size, device=device)
            else:
                next_value = values_full[:, t + 1]
                next_mask = response_mask_shifted[:, t + 1].float()
            delta = per_token_rewards[:, t] + gamma * next_value * next_mask - values_full[:, t]
            last_gae = delta + gamma * gae_lambda * next_mask * last_gae
            advantages[:, t] = last_gae * mask_t
        returns = advantages + values_full

        # ── PPO epochs ───────────────────────────────────────────────────
        for epoch in range(cfg.update_epochs):
            new_logp_full, new_values_full, _ = _logp_and_value(
                policy, value_head, tokenizer, full_ids, full_mask, response_mask
            )
            log_ratio = (new_logp_full - old_logp_full) * response_mask_shifted.float()
            ratio = log_ratio.exp()

            # Normalize advantages over response tokens only.
            adv = advantages
            valid = response_mask_shifted.float()
            n_valid = valid.sum().clamp(min=1)
            adv_mean = (adv * valid).sum() / n_valid
            adv_var = ((adv - adv_mean) ** 2 * valid).sum() / n_valid
            adv_std = adv_var.sqrt() + 1e-8
            norm_adv = ((adv - adv_mean) / adv_std) * valid

            pg1 = -norm_adv * ratio
            pg2 = -norm_adv * torch.clamp(ratio, 1 - cfg.clip_coef, 1 + cfg.clip_coef)
            pg_loss = (torch.max(pg1, pg2) * valid).sum() / n_valid

            v_loss = ((new_values_full - returns) ** 2 * valid).sum() / n_valid

            # Entropy estimate from -mean(log_p) on response positions
            entropy = (-new_logp_full * valid).sum() / n_valid

            with torch.no_grad():
                clip_frac = (
                    ((ratio - 1.0).abs() > cfg.clip_coef).float() * valid
                ).sum() / n_valid

            loss = pg_loss + cfg.value_coef * v_loss - cfg.entropy_coef * entropy
            loss_to_backward = loss / max(cfg.grad_accum_steps, 1)
            loss_to_backward.backward()
            if (epoch + 1) % max(cfg.grad_accum_steps, 1) == 0:
                torch.nn.utils.clip_grad_norm_(
                    list(policy.parameters()) + list(value_head.parameters()), 1.0
                )
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)

            with torch.no_grad():
                kl_to_ref = ((old_logp_full - ref_logp_full) * valid).sum() / n_valid

            tick_idx += 1
            sps = (iteration * cfg.batch_size) / max(time.monotonic() - t0, 1e-6)
            streamer.tick(
                tick=tick_idx,
                timestep=iteration * cfg.batch_size * cfg.update_epochs,
                payload={
                    "policy_loss": float(pg_loss.item()),
                    "value_loss": float(v_loss.item()),
                    "entropy": float(entropy.item()),
                    "clip_fraction": float(clip_frac.item()),
                    "kl_to_reference": float(kl_to_ref.item()),
                    "kl_coef": cfg.kl_coef,
                    "terminal_reward_mean": terminal_reward_mean,
                    "advantage_mean": float(adv_mean.item()),
                    "ratio_mean": float(((ratio * valid).sum() / n_valid).item()),
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "iteration": iteration,
                    "epoch": epoch,
                    "n_response_tokens": int(n_valid.item()),
                },
            )

    # ── Save the aligned policy ─────────────────────────────────────────
    final_dir = artifact_dir / "aligned-model"
    policy.save_pretrained(str(final_dir), safe_serialization=True)
    tokenizer.save_pretrained(str(final_dir))
    # Persist the value head as a sidecar (not consumed at inference time;
    # useful for resuming a PPO run).
    torch.save(value_head.state_dict(), str(final_dir / "value_head.pt"))
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
        timestep=cfg.num_iterations * cfg.batch_size,
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
