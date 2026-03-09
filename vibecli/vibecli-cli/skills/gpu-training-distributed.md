---
triggers: ["distributed training", "model training", "fine-tuning", "LoRA", "DeepSpeed", "FSDP", "data parallel", "model parallel"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# GPU Training & Distributed Training

When training or fine-tuning models across GPUs:

1. **Data parallelism (DDP)** — Use PyTorch DistributedDataParallel as the baseline for multi-GPU training. Each GPU holds a full model copy and processes a shard of the batch. Gradients are all-reduced across GPUs. Launch with `torchrun --nproc_per_node=N`. Ensure the DataLoader uses `DistributedSampler` and set `find_unused_parameters=False` when possible for performance.

2. **Model parallelism (tensor/pipeline)** — When a model exceeds single-GPU memory, split it across GPUs. Tensor parallelism shards individual layers (e.g., attention heads) across GPUs — use Megatron-LM or DeepSpeed for this. Pipeline parallelism splits layers into stages across GPUs with micro-batching to hide bubble overhead. Combine both for large-scale training (e.g., 3D parallelism = data + tensor + pipeline).

3. **DeepSpeed ZeRO stages** — ZeRO-1 partitions optimizer states across GPUs (1.5x memory savings). ZeRO-2 also partitions gradients (4x savings). ZeRO-3 partitions parameters too (linear scaling with GPU count). Use ZeRO-Offload to offload optimizer states/parameters to CPU RAM or NVMe for even larger models. Configure via a JSON config file and integrate with HuggingFace Trainer using `deepspeed` argument.

4. **FSDP (Fully Sharded Data Parallel)** — PyTorch-native alternative to DeepSpeed ZeRO-3. Shards model parameters, gradients, and optimizer states. Configure sharding strategy (FULL_SHARD, SHARD_GRAD_OP, NO_SHARD), auto_wrap_policy (transformer layers), and mixed precision. Use `torch.distributed.fsdp.FullyShardedDataParallel`. Works well with PyTorch 2.x and `torch.compile`.

5. **Mixed precision training** — Use bf16 on Ampere+ GPUs (A100, H100) for stable training without loss scaling. Use fp16 on older GPUs with dynamic loss scaling (`torch.cuda.amp.GradScaler`). Mixed precision halves memory usage and doubles throughput on tensor cores. Set `bf16=True` in HuggingFace TrainingArguments or configure in DeepSpeed config.

6. **Gradient accumulation** — Simulate larger batch sizes without more GPU memory by accumulating gradients over N micro-batches before stepping the optimizer. Set `gradient_accumulation_steps` in training config. Effective batch size = micro_batch_size x num_gpus x accumulation_steps. Essential when per-GPU batch size is memory-limited.

7. **Gradient checkpointing** — Trade compute for memory by recomputing activations during backward pass instead of storing them. Reduces activation memory by ~60-70% at ~30% compute overhead. Enable with `model.gradient_checkpointing_enable()` in HuggingFace or `torch.utils.checkpoint.checkpoint` for custom models.

8. **LoRA/QLoRA fine-tuning** — LoRA freezes base weights and trains low-rank adapter matrices (rank 8-64 typical), reducing trainable parameters by 90%+. QLoRA quantizes the base model to 4-bit (NF4) and trains LoRA adapters in bf16, enabling fine-tuning 65B+ models on a single 48GB GPU. Use the `peft` library. Target attention layers (`q_proj`, `v_proj`) at minimum; include `k_proj`, `o_proj`, `gate_proj`, `up_proj`, `down_proj` for higher quality.

9. **Learning rate scheduling** — Use warmup (5-10% of total steps) followed by cosine decay to zero. For fine-tuning, use a lower peak LR (1e-5 to 5e-5) than pre-training (1e-4 to 3e-4). With LoRA, higher LRs (1e-4 to 3e-4) often work well. Use `get_cosine_schedule_with_warmup` from transformers. Consider WSD (warmup-stable-decay) for pre-training runs.

10. **Dataset preparation** — Tokenize data offline and save as Arrow/parquet files for fast loading. Use packing (concatenate sequences with separator tokens) to minimize padding waste. For instruction tuning, mask loss on prompt tokens. Shuffle data thoroughly and use streaming datasets for corpora that exceed RAM.

11. **Checkpointing and experiment tracking** — Save checkpoints every N steps (not just epochs) to networked storage. For DeepSpeed/FSDP, use their native checkpoint methods which handle sharded state. Track experiments with WandB or MLflow — log loss curves, learning rate, gradient norms, GPU utilization, and evaluation metrics. Tag runs with hyperparameters for reproducibility.

12. **Evaluation metrics** — Monitor training loss, validation loss, and gradient norm. For language models, track perplexity. For fine-tuning, evaluate on held-out task-specific benchmarks periodically. Watch for loss spikes (reduce LR or increase warmup) and divergence (gradient norm explosion).
