# Agent Registry

Discovery and capability advertisement for the Agent-OS pool. Matches Devin 2.0's agent registry.

## Key Types
- **AgentRegistry** — register/deregister/heartbeat/query agents
- **AgentRegistration** — id, capabilities, health, load, max_concurrent_tasks
- **AgentHealth** — Healthy | Degraded | Unhealthy | Unknown
- **Capability** — named string constants (CODE_EDIT, GIT_OPS, DEPLOY, TEST_RUN, …)

## Queries
- `find_capable(cap)` — all available agents with a capability
- `find_all_capable(caps)` — agents matching ALL required capabilities
- `least_loaded(cap)` — best agent for a task (min load)

## Commands
- `/agent registry list` — show all agents + health
- `/agent registry find <capability>` — find capable agents
- `/agent registry status` — pool utilization summary

## Examples
```
/agent registry list
# a1: coder-agent v1.0 — healthy, load 0.2, 1/4 tasks
# a2: review-agent v1.0 — healthy, load 0.0, 0/2 tasks

/agent registry find code_edit
# a1 (load 0.2) — best match
```
