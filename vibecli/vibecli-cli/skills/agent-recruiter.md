# Agent Recruiter

Dynamic task-to-agent assignment with scoring heuristic. Matches Devin 2.0's recruitment system.

## Scoring (higher = better)
- Load: `(1 - load) × 0.5`
- Preferred capability match: `match_ratio × 0.3`
- Priority bonus (Critical/High): `× 0.2`

## Outcomes
- **Assigned(id)** — task matched to an agent
- **Queued** — capable agents exist but all busy; task waits
- **NoCapableAgent** — no agent has the required capabilities
- **TimedOut** — task timed out before assignment

## Commands
- `/agent recruit <task>` — assign a task to best available agent
- `/agent queue` — show queued tasks
- `/agent release <task-id>` — mark task complete, free agent slot
