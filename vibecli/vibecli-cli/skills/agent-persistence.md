# Agent Persistence

Serialize and restore agent state across restarts. Matches Claude Code 1.x background agent persistence.

## Key Types
- **SnapshotStore** — in-memory checkpoint store (max N per session, auto-evict oldest)
- **AgentSnapshot** — full state: phase, step count, pending tool calls, context summary
- **StateValue** — typed: Str | Int | Float | Bool | List | Null
- **SnapshotBuilder** — fluent API for constructing snapshots

## Operations
- `save(snapshot)` — auto-increments checkpoint_id, evicts oldest if over limit
- `load_latest(agent, session)` — most recent checkpoint
- `load_checkpoint(agent, session, id)` — specific checkpoint by ID

## Commands
- `/agent checkpoint save` — save current state
- `/agent checkpoint restore [id]` — restore to a checkpoint
- `/agent checkpoint list` — show available checkpoints
