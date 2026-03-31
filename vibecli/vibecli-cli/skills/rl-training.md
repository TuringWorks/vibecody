# RL Training

Orchestrate reinforcement learning training with 30+ algorithms, distributed training, AutoRL hyperparameter search, curriculum learning, and multi-agent support.

## When to Use
- Training RL agents with PPO, SAC, DQN, TD3, CQL, IQL, MAPPO, QMIX
- Setting up distributed training across multiple GPUs/nodes
- Running AutoRL hyperparameter optimization (Bayesian, PBT, grid, random)
- Configuring curriculum learning with stage-based progression
- Training multi-agent systems with cooperative/competitive modes
- Managing replay buffers (uniform, prioritized, hindsight)

## Commands
- `/rlos train run <config.yaml>` — Start a training run from config
- `/rlos train status` — Show active training runs with metrics
- `/rlos train stop <run_id>` — Stop a training run
- `/rlos train resume <checkpoint>` — Resume training from checkpoint
- `/rlos train autotune <config.yaml>` — Run AutoRL hyperparameter search
- `/rlos train curriculum` — Show curriculum learning progress
- `/rlos train agents` — List multi-agent training configurations
