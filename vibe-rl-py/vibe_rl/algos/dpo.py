"""Slice 7c — DPO (Direct Preference Optimization).

Rafailov et al. 2023, "Direct Preference Optimization: Your Language
Model Is Secretly a Reward Model". Single-stage RLHF alignment that
sidesteps the reward-model + PPO loop:

    L_DPO = -E[ log σ( β · ( log π_θ(y_w | x) - log π_ref(y_w | x)
                            - log π_θ(y_l | x) + log π_ref(y_l | x) ) ) ]

where (y_w, y_l) is a (winning, losing) preference pair, π_θ is the
trainable policy, π_ref is a frozen reference (typically the base
model), and β is a temperature (~0.1).

Implementation choices for slice 7c:

- Reads preferences from `<workspace>/.vibecli/workspace.db` directly
  (stdlib sqlite3) — the daemon's `PreferenceStore` writes them; we
  only consume the rows where `chosen` is set.
- Uses HuggingFace transformers (no TRL dep — DPO loss is 30 lines).
- Loads base + frozen reference from the same Hugging Face model id
  (you can override `reference_model_id` in the config to use a
  different snapshot).
- Default base: `distilgpt2` (~80 MB) — small enough for a smoke run
  on CPU/MPS in minutes.

Wire format mirrors PPO. Per-tick payload:
- `dpo_loss`, `chosen_reward`, `rejected_reward` (= β · log-ratio),
  `accuracy` (fraction of preferences where chosen > rejected
  reward), `kl_to_reference` (estimate),
- `learning_rate`, `sps` (samples-per-second over preference pairs).

Reward-model training (RM) for PPO-style alignment is deferred to
7c-extras — this slice ships the DPO end-to-end path which is simpler
and arguably more production-proven than PPO RLHF as of 2024.
"""

from __future__ import annotations

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
class DPOConfig:
    run_id: str
    base_model_id: str = "distilgpt2"
    reference_model_id: str | None = None  # default: same as base_model_id
    beta: float = 0.1
    max_seq_len: int = 256
    batch_size: int = 4
    learning_rate: float = 5e-6
    num_epochs: int = 1
    grad_accum_steps: int = 1
    seed: int = 42
    suite_id: str | None = None  # filter rl_preferences by suite if given
    workspace_path: str = "."
    artifact_dir: str = ""


def _read_preferences(workspace_db: Path, suite_id: str | None) -> list[tuple[str, str, str]]:
    """Pull (prompt, chosen_completion, rejected_completion) tuples from
    rl_preferences. Only rows with chosen ∈ {a, b} are eligible — `tie`
    and `reject_both` are filtered out.
    """
    conn = sqlite3.connect(str(workspace_db))
    try:
        if suite_id is None:
            rows = conn.execute(
                "SELECT prompt, completion_a, completion_b, chosen "
                "FROM rl_preferences WHERE chosen IN ('a', 'b')"
            ).fetchall()
        else:
            rows = conn.execute(
                "SELECT prompt, completion_a, completion_b, chosen "
                "FROM rl_preferences WHERE suite_id = ? AND chosen IN ('a', 'b')",
                (suite_id,),
            ).fetchall()
    finally:
        conn.close()
    out: list[tuple[str, str, str]] = []
    for prompt, a, b, chosen in rows:
        if chosen == "a":
            out.append((prompt, a, b))
        else:
            out.append((prompt, b, a))
    return out


def _logp_completions(model, tokenizer, prompts: list[str], completions: list[str], max_seq_len: int, device: torch.device) -> torch.Tensor:  # type: ignore[no-untyped-def]
    """Per-example log P(completion | prompt) under `model`.

    Implementation:
    - Concatenate prompt + completion (BOS-prepended), tokenize.
    - Build per-token labels: -100 on prompt tokens, real token ids on
      completion tokens.
    - Forward, gather log-probs at the labelled positions, sum per row.

    Returns a 1-D tensor of shape (batch,).
    """
    assert len(prompts) == len(completions)
    full_texts = [p + c for p, c in zip(prompts, completions)]
    full_enc = tokenizer(
        full_texts,
        padding=True,
        truncation=True,
        max_length=max_seq_len,
        return_tensors="pt",
    ).to(device)
    prompt_enc = tokenizer(
        prompts,
        padding=True,
        truncation=True,
        max_length=max_seq_len,
        return_tensors="pt",
    ).to(device)

    input_ids = full_enc["input_ids"]
    attention_mask = full_enc["attention_mask"]

    # Build label tensor: -100 for prompt + padding, real ids for the
    # completion tokens.
    labels = input_ids.clone()
    labels[attention_mask == 0] = -100
    prompt_lengths = prompt_enc["attention_mask"].sum(dim=1)  # per-row
    for i, pl in enumerate(prompt_lengths.tolist()):
        labels[i, :pl] = -100

    outputs = model(input_ids=input_ids, attention_mask=attention_mask)
    logits = outputs.logits[:, :-1, :]  # predict next-token at each position
    target = labels[:, 1:]
    log_probs = F.log_softmax(logits, dim=-1)

    # Gather log-prob of the target token at each position (mask -100 → 0).
    safe_target = target.clone()
    safe_target[safe_target == -100] = 0
    gathered = log_probs.gather(2, safe_target.unsqueeze(-1)).squeeze(-1)
    mask = (target != -100).float()
    per_example = (gathered * mask).sum(dim=1)
    return per_example


def train(cfg: DPOConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
    random.seed(cfg.seed)
    np.random.seed(cfg.seed)
    torch.manual_seed(cfg.seed)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    if device.type == "cpu" and getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        device = torch.device("mps")

    streamer.started(
        sidecar_version=_sidecar_version(),
        seed=cfg.seed,
        device=str(device),
    )

    try:
        from transformers import AutoModelForCausalLM, AutoTokenizer
    except ImportError:
        streamer.finished(
            reason="error",
            error=(
                "transformers not installed — slice 7c DPO requires the [rlhf] "
                "extra: `cd vibe-rl-py && uv sync --extra rlhf`"
            ),
        )
        return {"error": "transformers missing"}

    workspace_db = Path(cfg.workspace_path) / ".vibecli" / "workspace.db"
    if not workspace_db.is_file():
        streamer.finished(
            reason="error",
            error=f"workspace.db not found at {workspace_db} — collect preferences first",
        )
        return {"error": "no workspace db"}

    prefs = _read_preferences(workspace_db, cfg.suite_id)
    if not prefs:
        streamer.finished(
            reason="error",
            error=(
                "no judged preferences found in rl_preferences. "
                "Collect + judge preferences (chosen ∈ {a, b}) before training."
            ),
        )
        return {"error": "no preferences"}

    tokenizer = AutoTokenizer.from_pretrained(cfg.base_model_id)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    policy = AutoModelForCausalLM.from_pretrained(cfg.base_model_id).to(device)
    policy.train()
    reference_id = cfg.reference_model_id or cfg.base_model_id
    reference = AutoModelForCausalLM.from_pretrained(reference_id).to(device)
    reference.eval()
    for p in reference.parameters():
        p.requires_grad = False

    optimizer = torch.optim.AdamW(policy.parameters(), lr=cfg.learning_rate)
    artifact_dir = Path(cfg.artifact_dir) if cfg.artifact_dir else (
        Path(cfg.workspace_path) / ".vibecli" / "rl-artifacts" / cfg.run_id
    )
    artifact_dir.mkdir(parents=True, exist_ok=True)

    n_pairs = len(prefs)
    t0 = time.monotonic()
    global_step = 0
    tick_idx = 0
    best_acc = 0.0

    for epoch in range(cfg.num_epochs):
        if runtime.should_stop():
            break
        order = list(range(n_pairs))
        random.shuffle(order)

        for start in range(0, n_pairs, cfg.batch_size):
            if runtime.should_stop():
                break
            batch_idx = order[start : start + cfg.batch_size]
            batch = [prefs[i] for i in batch_idx]
            prompts = [p for p, _, _ in batch]
            chosen = [c for _, c, _ in batch]
            rejected = [r for _, _, r in batch]

            # Policy log-probs (with grad).
            policy_chosen_logp = _logp_completions(policy, tokenizer, prompts, chosen, cfg.max_seq_len, device)
            policy_rejected_logp = _logp_completions(policy, tokenizer, prompts, rejected, cfg.max_seq_len, device)

            # Reference log-probs (frozen, no grad).
            with torch.no_grad():
                ref_chosen_logp = _logp_completions(reference, tokenizer, prompts, chosen, cfg.max_seq_len, device)
                ref_rejected_logp = _logp_completions(reference, tokenizer, prompts, rejected, cfg.max_seq_len, device)

            chosen_logratio = policy_chosen_logp - ref_chosen_logp
            rejected_logratio = policy_rejected_logp - ref_rejected_logp
            chosen_reward = cfg.beta * chosen_logratio
            rejected_reward = cfg.beta * rejected_logratio

            losses = -F.logsigmoid(chosen_reward - rejected_reward)
            loss = losses.mean() / max(cfg.grad_accum_steps, 1)

            loss.backward()
            global_step += 1
            if global_step % max(cfg.grad_accum_steps, 1) == 0:
                torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)

            with torch.no_grad():
                accuracy = float((chosen_reward > rejected_reward).float().mean().item())
                kl_to_ref = float((-(policy_chosen_logp - ref_chosen_logp)).mean().item())

            tick_idx += 1
            sps = global_step * cfg.batch_size / max(time.monotonic() - t0, 1e-6)
            best_acc = max(best_acc, accuracy)
            streamer.tick(
                tick=tick_idx,
                timestep=global_step * cfg.batch_size,
                payload={
                    "dpo_loss": float(loss.item() * max(cfg.grad_accum_steps, 1)),
                    "chosen_reward": float(chosen_reward.mean().item()),
                    "rejected_reward": float(rejected_reward.mean().item()),
                    "accuracy": accuracy,
                    "kl_to_reference": kl_to_ref,
                    "beta": cfg.beta,
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "n_pairs": n_pairs,
                    "epoch": epoch,
                },
            )

    # Final flush of any remaining grads.
    if global_step % max(cfg.grad_accum_steps, 1) != 0:
        torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
        optimizer.step()
        optimizer.zero_grad(set_to_none=True)

    # Save the aligned model (HF format) — write under artifact_dir so
    # the daemon's record_artifact is happy with a workspace-relative
    # path. We persist both the safetensors weights and a JSON sidecar
    # following slice 2's checkpoint shape.
    final_dir = artifact_dir / "aligned-model"
    policy.save_pretrained(str(final_dir), safe_serialization=True)
    tokenizer.save_pretrained(str(final_dir))

    # Aggregate size + sha for the streamer's checkpoint event. We hash
    # the safetensors file specifically so the artifact has a stable
    # content-addressable identity.
    weights_path = final_dir / "model.safetensors"
    if not weights_path.is_file():
        # Fallback for older transformers that wrote pytorch_model.bin.
        for cand in final_dir.glob("pytorch_model*.bin"):
            weights_path = cand
            break
    size_bytes = weights_path.stat().st_size if weights_path.is_file() else 0
    sha = _sha256_file(weights_path) if weights_path.is_file() else ""
    workspace = Path(cfg.workspace_path).resolve()
    try:
        rel = weights_path.resolve().relative_to(workspace)
        rel_str = str(rel).replace("\\", "/")
    except ValueError:
        rel_str = str(weights_path)

    streamer.checkpoint(
        timestep=global_step * cfg.batch_size,
        rel_path=rel_str,
        sha256=sha,
        size_bytes=size_bytes,
    )
    streamer.finished(
        reason="done",
        final_reward_mean=best_acc,
    )

    return {
        "n_pairs": n_pairs,
        "global_step": global_step,
        "best_accuracy": best_acc,
        "aligned_model_dir": str(final_dir),
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
