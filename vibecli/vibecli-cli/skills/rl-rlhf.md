# RLHF & LLM Alignment

Align language models with PPO, DPO, KTO, ORPO, GRPO, reward model training, RLEF (execution feedback), and Constitutional AI.

## When to Use
- Training reward models from human preference data
- Running RLHF with PPO for LLM alignment
- Using DPO/KTO/ORPO/GRPO preference optimization
- Setting up RL from Execution Feedback (RLEF) for code generation
- Building Constitutional AI / RLAIF pipelines
- Detecting reward hacking and alignment drift
- Training process reward models for reasoning chains

## Commands
- `/rlos rlhf train <config.yaml>` — Run RLHF training pipeline
- `/rlos rlhf reward-model <data.jsonl>` — Train reward model
- `/rlos rlhf dpo <config.yaml>` — Run DPO alignment
- `/rlos rlhf eval <policy>` — Run alignment evaluation benchmarks
- `/rlos rlhf detect-hacking <deployment>` — Check for reward hacking
- `/rlos rlhf constitutional <rules.yaml>` — Run Constitutional AI pipeline
