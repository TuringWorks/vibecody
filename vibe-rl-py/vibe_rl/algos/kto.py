"""Slice 7c-extras — KTO (Kahneman-Tversky Optimization).

Ethayarajh, Xu, Chaudhary, Kiela 2024, "Model Alignment as Prospect
Theoretic Optimization". KTO uses **unpaired** desirable / undesirable
labels — one example at a time, not pairs — which matches real-world
feedback patterns (thumbs-up / thumbs-down) better than DPO's pair
requirement.

Loss (per example):

    KL    = E_x[ KL(π_θ(· | x) || π_ref(· | x)) ]                  (estimated batch-wise)
    r(x,y) = β · (logp_θ(y|x) - logp_ref(y|x))

    L_KTO = E_desirable  [ 1 - σ(   r(x,y) - z_0 ) ]
          + E_undesirable[ 1 - σ( - r(x,y) + z_0 ) ]
    z_0   = max(0, KL)

We have paired preferences from the daemon, but KTO trains on the
unpaired form: each pair → one desirable example (the chosen completion)
+ one undesirable example (the rejected completion). The loss treats
them independently, no in-batch pair coupling required.

`λ_d`/`λ_u` are typically equal but the paper recommends λ_d = 1.0,
λ_u = (n_undesirable / n_desirable) (rebalance for skewed feedback).
For our paired data they're 1:1 so default = 1.0 each.

Reference model: same-id frozen copy of the policy at startup, just
like DPO.
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
import torch.nn.functional as F

from vibe_rl import runtime
from vibe_rl.algos.dpo import _logp_completions, _read_preferences
from vibe_rl.checkpoint import _sha256_file  # type: ignore[attr-defined]


@dataclass
class KTOConfig:
    run_id: str
    base_model_id: str = "distilgpt2"
    reference_model_id: str | None = None
    beta: float = 0.1
    lambda_d: float = 1.0  # weight on desirable examples
    lambda_u: float = 1.0  # weight on undesirable examples
    max_seq_len: int = 256
    batch_size: int = 4
    learning_rate: float = 5e-6
    num_epochs: int = 1
    grad_accum_steps: int = 1
    seed: int = 42
    suite_id: str | None = None
    workspace_path: str = "."
    artifact_dir: str = ""


def train(cfg: KTOConfig, streamer) -> dict[str, Any]:  # type: ignore[no-untyped-def]
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
            error="transformers missing — install [rlhf] extra",
        )
        return {"error": "transformers missing"}

    workspace_db = Path(cfg.workspace_path) / ".vibecli" / "workspace.db"
    if not workspace_db.is_file():
        streamer.finished(reason="error", error=f"workspace.db not found at {workspace_db}")
        return {"error": "no workspace db"}

    prefs = _read_preferences(workspace_db, cfg.suite_id)
    if not prefs:
        streamer.finished(reason="error", error="no judged preferences")
        return {"error": "no preferences"}

    # Unfold pairs into per-example desirable/undesirable rows.
    examples: list[tuple[str, str, bool]] = []  # (prompt, completion, is_desirable)
    for prompt, chosen, rejected in prefs:
        examples.append((prompt, chosen, True))
        examples.append((prompt, rejected, False))

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

    n_examples = len(examples)
    t0 = time.monotonic()
    global_step = 0
    tick_idx = 0
    best_acc = 0.0

    for epoch in range(cfg.num_epochs):
        if runtime.should_stop():
            break
        order = list(range(n_examples))
        random.shuffle(order)

        for start in range(0, n_examples, cfg.batch_size):
            if runtime.should_stop():
                break
            batch_idx = order[start : start + cfg.batch_size]
            batch = [examples[i] for i in batch_idx]
            prompts = [p for p, _, _ in batch]
            completions = [c for _, c, _ in batch]
            is_desirable = torch.tensor(
                [d for _, _, d in batch], dtype=torch.bool, device=device
            )

            policy_logp = _logp_completions(policy, tokenizer, prompts, completions, cfg.max_seq_len, device)
            with torch.no_grad():
                ref_logp = _logp_completions(reference, tokenizer, prompts, completions, cfg.max_seq_len, device)

            r = cfg.beta * (policy_logp - ref_logp)
            # KL estimate over the batch (matches Eq. 7 of the paper).
            with torch.no_grad():
                kl_estimate = (policy_logp - ref_logp).mean().clamp(min=0.0)
            z_0 = kl_estimate

            # Desirable examples want r > z_0 (loss = 1 - σ(r - z_0)).
            # Undesirable examples want r < z_0 (loss = 1 - σ(z_0 - r)).
            # Combined: loss = 1 - σ(sign · (r - z_0)) where sign = +1 for
            # desirable, -1 for undesirable.
            sign = torch.where(is_desirable, torch.ones_like(r), -torch.ones_like(r))
            margin = sign * (r - z_0)
            per_example_loss = 1.0 - torch.sigmoid(margin)
            weights = torch.where(
                is_desirable,
                torch.full_like(r, cfg.lambda_d),
                torch.full_like(r, cfg.lambda_u),
            )
            loss = (weights * per_example_loss).mean()
            loss_to_backward = loss / max(cfg.grad_accum_steps, 1)

            loss_to_backward.backward()
            global_step += 1
            if global_step % max(cfg.grad_accum_steps, 1) == 0:
                torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)

            with torch.no_grad():
                # "Accuracy": for desirable examples r > z_0, for undesirable r < z_0.
                correct = torch.where(is_desirable, r > z_0, r < z_0)
                accuracy = float(correct.float().mean().item())
                desirable_reward = float(r[is_desirable].mean().item()) if is_desirable.any() else 0.0
                undesirable_reward = float(r[~is_desirable].mean().item()) if (~is_desirable).any() else 0.0

            tick_idx += 1
            sps = global_step * cfg.batch_size / max(time.monotonic() - t0, 1e-6)
            best_acc = max(best_acc, accuracy)
            streamer.tick(
                tick=tick_idx,
                timestep=global_step * cfg.batch_size,
                payload={
                    "kto_loss": float(loss.item()),
                    "desirable_reward": desirable_reward,
                    "undesirable_reward": undesirable_reward,
                    "kl_estimate": float(kl_estimate.item()),
                    "z_0": float(z_0.item()),
                    "accuracy": accuracy,
                    "beta": cfg.beta,
                    "lambda_d": cfg.lambda_d,
                    "lambda_u": cfg.lambda_u,
                    "learning_rate": cfg.learning_rate,
                    "sps": sps,
                    "n_examples": n_examples,
                    "epoch": epoch,
                },
            )

    if global_step % max(cfg.grad_accum_steps, 1) != 0:
        torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
        optimizer.step()
        optimizer.zero_grad(set_to_none=True)

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
        timestep=global_step * cfg.batch_size,
        rel_path=rel_str,
        sha256=sha,
        size_bytes=size_bytes,
    )
    streamer.finished(reason="done", final_reward_mean=best_acc)
    return {
        "n_examples": n_examples,
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
