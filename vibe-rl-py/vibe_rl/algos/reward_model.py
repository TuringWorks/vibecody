"""Slice 7c-extras — Reward Model (RM) training.

Trains a scalar reward head over a frozen-or-trainable LM backbone using
binary cross-entropy on preference pairs. The classical RLHF recipe
(InstructGPT, Llama-Chat) uses the resulting RM to score generations
during PPO; ORPO/DPO/KTO sidestep the separate RM stage but having one
on hand is useful for:

- Standalone "is this completion good?" scoring (rank candidates from
  an LM, gate generations, etc.).
- Future PPO RLHF (slice 7c-extras+1 — not in this commit).
- Sanity checks on the preference dataset itself.

Architecture:
- Hugging Face `AutoModelForSequenceClassification` with `num_labels=1`
  gives us an LM body + a scalar regression head.
- For each pair (prompt, chosen, rejected) we compute scores
  r_chosen, r_rejected and minimize:

    L_RM = -log σ( r_chosen - r_rejected )

This is the Bradley-Terry preference loss; it makes the RM agree with
human pairwise rankings. We log:

- `rm_loss`: the BCE-style loss above.
- `accuracy`: fraction of pairs where r_chosen > r_rejected.
- `reward_margin`: mean (r_chosen - r_rejected); should grow as
  training proceeds.

The trained RM is saved as a HuggingFace folder
(`<artifact_dir>/reward-model/`); it can be loaded later with
`AutoModelForSequenceClassification.from_pretrained(...)` to score any
(prompt, completion) pair.
"""

from __future__ import annotations

import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
import torch.nn.functional as F

from vibe_rl import runtime
from vibe_rl.algos.dpo import _read_preferences
from vibe_rl.checkpoint import _sha256_file  # type: ignore[attr-defined]


@dataclass
class RewardModelConfig:
    run_id: str
    base_model_id: str = "distilgpt2"
    max_seq_len: int = 256
    batch_size: int = 4
    learning_rate: float = 1e-5
    num_epochs: int = 1
    grad_accum_steps: int = 1
    seed: int = 42
    suite_id: str | None = None
    workspace_path: str = "."
    artifact_dir: str = ""


def _score_completions(model, tokenizer, prompts: list[str], completions: list[str], max_seq_len: int, device: torch.device) -> torch.Tensor:  # type: ignore[no-untyped-def]
    """Score (prompt + completion) pairs through the regression head.

    Returns a 1-D tensor of shape (batch,). The score is the
    classification head's output at the final non-pad position — same
    convention as TRL's RewardTrainer.
    """
    full_texts = [p + c for p, c in zip(prompts, completions)]
    enc = tokenizer(
        full_texts,
        padding=True,
        truncation=True,
        max_length=max_seq_len,
        return_tensors="pt",
    ).to(device)
    outputs = model(input_ids=enc["input_ids"], attention_mask=enc["attention_mask"])
    # `AutoModelForSequenceClassification` returns logits of shape
    # (batch, num_labels). We use num_labels=1, so squeeze.
    return outputs.logits.squeeze(-1)


def train(cfg: RewardModelConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
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
        from transformers import AutoModelForSequenceClassification, AutoTokenizer
    except ImportError:
        streamer.finished(reason="error", error="transformers missing — install [rlhf] extra")
        return {"error": "transformers missing"}

    workspace_db = Path(cfg.workspace_path) / ".vibecli" / "workspace.db"
    if not workspace_db.is_file():
        streamer.finished(reason="error", error=f"workspace.db not found at {workspace_db}")
        return {"error": "no workspace db"}

    prefs = _read_preferences(workspace_db, cfg.suite_id)
    if not prefs:
        streamer.finished(reason="error", error="no judged preferences")
        return {"error": "no preferences"}

    tokenizer = AutoTokenizer.from_pretrained(cfg.base_model_id)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token
    model = AutoModelForSequenceClassification.from_pretrained(
        cfg.base_model_id, num_labels=1
    ).to(device)
    if model.config.pad_token_id is None:
        model.config.pad_token_id = tokenizer.pad_token_id
    model.train()

    optimizer = torch.optim.AdamW(model.parameters(), lr=cfg.learning_rate)
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

            r_chosen = _score_completions(model, tokenizer, prompts, chosen, cfg.max_seq_len, device)
            r_rejected = _score_completions(model, tokenizer, prompts, rejected, cfg.max_seq_len, device)

            # Bradley-Terry preference loss.
            loss = -F.logsigmoid(r_chosen - r_rejected).mean()
            loss_to_backward = loss / max(cfg.grad_accum_steps, 1)

            loss_to_backward.backward()
            global_step += 1
            if global_step % max(cfg.grad_accum_steps, 1) == 0:
                torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)

            with torch.no_grad():
                accuracy = float((r_chosen > r_rejected).float().mean().item())
                margin = float((r_chosen - r_rejected).mean().item())

            tick_idx += 1
            sps = global_step * cfg.batch_size / max(time.monotonic() - t0, 1e-6)
            best_acc = max(best_acc, accuracy)
            streamer.tick(
                tick=tick_idx,
                timestep=global_step * cfg.batch_size,
                payload={
                    "rm_loss": float(loss.item()),
                    "accuracy": accuracy,
                    "reward_margin": margin,
                    "r_chosen_mean": float(r_chosen.mean().item()),
                    "r_rejected_mean": float(r_rejected.mean().item()),
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "n_pairs": n_pairs,
                    "epoch": epoch,
                },
            )

    if global_step % max(cfg.grad_accum_steps, 1) != 0:
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        optimizer.step()
        optimizer.zero_grad(set_to_none=True)

    rm_dir = artifact_dir / "reward-model"
    model.save_pretrained(str(rm_dir), safe_serialization=True)
    tokenizer.save_pretrained(str(rm_dir))
    weights_path = rm_dir / "model.safetensors"
    if not weights_path.is_file():
        for cand in rm_dir.glob("pytorch_model*.bin"):
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
        timestep=global_step * cfg.batch_size,
        rel_path=rel_str,
        sha256=sha,
        size_bytes=size_bytes,
    )
    streamer.finished(reason="done", final_reward_mean=best_acc)
    return {
        "n_pairs": n_pairs,
        "global_step": global_step,
        "best_accuracy": best_acc,
        "reward_model_dir": str(rm_dir),
    }


def _sidecar_version() -> str:
    try:
        from vibe_rl import __version__ as v

        return v
    except Exception:  # noqa: BLE001
        return "0.0.0-unknown"
