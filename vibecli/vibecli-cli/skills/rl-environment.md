# RL Environment Management

Manage reinforcement learning environments with versioning, declarative YAML definitions, simulation backends, real-world connectors, and hybrid sim+real training pipelines.

## When to Use
- Defining RL environments with observation/action spaces and reward functions
- Versioning environment definitions (commit, diff, rollback)
- Connecting to simulation backends (MuJoCo, PhysX, Brax, Unity)
- Setting up real-world data connectors (REST API, gRPC, MQTT, WebSocket)
- Configuring domain randomization for sim-to-real transfer
- Recording and replaying trajectories with time-travel replay

## Commands
- `/rlos env init <name>` — Create a new environment definition
- `/rlos env deploy <file.yaml>` — Deploy environment from YAML spec
- `/rlos env list` — List all registered environments
- `/rlos env diff <v1>..<v2>` — Diff two environment versions
- `/rlos env replay <episode_id>` — Replay a recorded episode
- `/rlos env rollback <version>` — Rollback to a previous version
- `/rlos env connectors` — List available real-world connectors
