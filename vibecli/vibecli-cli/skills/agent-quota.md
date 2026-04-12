# Agent Quota

Per-agent token, cost, time, and task quotas with soft-warn + hard-block enforcement.
Matches Claude Code 1.x cost controls and Devin 2.0's resource budgets.

## Resources
| Kind | Description |
|---|---|
| `tokens` | LLM input + output tokens |
| `cost_cents` | USD cost (stored as cents) |
| `wall_time_secs` | Wall-clock execution time |
| `tool_calls` | Number of tool invocations |
| `tasks` | Total tasks executed |

## Decisions
- **Allow** — within limits
- **Warn** — above soft limit (default 80%), still allowed
- **Deny** — would exceed hard limit

## Commands
- `/agent quota set <agent> <resource> <limit>` — set a quota
- `/agent quota usage <agent>` — show current usage
- `/agent quota reset <agent> <resource>` — reset counter
