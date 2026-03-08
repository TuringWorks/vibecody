---
triggers: ["fine-tune", "fine tuning", "finetune", "model training", "LoRA", "SWE-bench", "training data", "dataset preparation", "custom model", "coding model", "adapter", "training job"]
tools_allowed: ["read_file", "write_file", "bash", "search_files"]
category: ai-models
---

# Model Fine-Tuning for Code

When preparing datasets, launching fine-tuning jobs, or evaluating custom coding models:

1. **Choose a Dataset Source** — Three primary sources for training data: **Codebase extraction** (`from_codebase(path)`) mines function signatures + docstrings + implementations as instruction-completion pairs. **Git history** (`from_git_history(repo, max_commits)`) extracts commit message + diff pairs — the model learns to write code given a natural language description of the change. **Saved conversations** (`from_conversations(path)`) uses previous agent sessions as multi-turn training examples. Each source produces a `Dataset` with `TrainingExample` entries.

2. **Validate and Clean Datasets** — Always run `dataset.validate()` before training. Common issues: empty messages, examples exceeding `max_seq_length` (default: 8192 tokens), imbalanced language distribution, near-duplicates inflating the dataset. Use `deduplicate()` to remove examples with >90% token overlap. Use `filter_by_language("rust")` to create language-specific fine-tunes. Check `stats()` for token count distribution, average example length, and language breakdown.

3. **Export in the Right Format** — Use `to_jsonl(path)` for OpenAI-compatible fine-tuning format. Supported formats: `ChatML` (OpenAI chat models), `Alpaca` (instruction/input/output triplets), `ShareGPT` (multi-turn conversations), `Completion` (simple prompt-completion), `SWEBench` (repo + problem statement + patch). The JSONL export handles proper escaping and token counting.

4. **Configure Training Parameters** — Key settings in `FineTuneConfig`: `base_model` (e.g., "gpt-4o-mini-2024-07-18"), `n_epochs` (1-4, start with 1), `batch_size` (default: 4), `learning_rate` (default: 2e-5, lower for larger models). For LoRA fine-tuning: set `lora_rank` (8-64, higher = more capacity), `lora_alpha` (typically 2x rank). Use `warmup_steps` (default: 100) to stabilize early training.

5. **Estimate Costs Before Training** — `estimate_cost(config, dataset)` returns estimated USD cost, training duration in minutes, and total tokens. OpenAI fine-tuning costs ~$0.008/1k training tokens for GPT-4o-mini. A 10k-example dataset with 3 epochs ≈ $50-200 depending on example length. Together AI and Fireworks offer cheaper alternatives for open-weight models.

6. **Select a Fine-Tuning Provider** — `OpenAI` for GPT-4o/4o-mini fine-tuning (best quality, highest cost). `TogetherAI` for Llama/Mistral/CodeLlama (good balance of cost and quality). `Fireworks` for fast iteration with open models. `Local` for LoRA fine-tuning via llama.cpp (free, requires GPU, best for experimentation).

7. **Monitor Training Jobs** — Jobs progress through states: Pending → Validating → Running(epoch, loss) → Completed/Failed. Track `TrainingMetrics`: training loss (should decrease), validation loss (watch for overfitting — val loss increasing while train loss decreases), accuracy, tokens processed. Cancel diverging jobs early with `cancel_job(id)` to save costs.

8. **Evaluate with SWE-Bench** — Load SWE-bench tasks with `load_tasks(path)` — each task has a repo, problem statement, expected patch, and test patch. Run `evaluate_model(model, provider)` to measure resolution rate (% of tasks where the model produces a correct patch). Use `compare_models(results)` to benchmark your fine-tuned model against the base model and competitors. Target: 20%+ resolution rate is competitive.

9. **Manage LoRA Adapters** — LoRA adapters are small weight deltas (~10-100 MB) that modify a base model's behavior without changing its weights. `list_adapters(dir)` shows available adapters with their base model and rank. `merge_adapter(adapter, output)` bakes the adapter into the base model weights for faster inference (no runtime overhead). Keep adapters versioned — they're small enough to commit to git.

10. **Iterate on Dataset Quality** — Fine-tuning results depend more on dataset quality than size. Start with 100-500 high-quality examples, evaluate, then scale up. Remove low-quality examples (vague instructions, incorrect code, trivial changes). Add examples for failure modes you observe during evaluation. Use `split(0.9)` for a 90/10 train/validation split to catch overfitting early.
