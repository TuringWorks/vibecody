# Session Health Probe

Monitor active agent sessions for health status (Healthy/Degraded/Stalled/Dead) based on token utilization, idle time, and error rates. Also runs post-compaction probes to verify tool responsiveness before resuming the agent loop.

## When to Use
- Detecting stalled sessions that stopped responding
- Flagging degraded sessions before they consume full token budgets
- Validating tool executor responsiveness after context compaction
- Prioritizing which unhealthy sessions to recover vs. terminate
- Monitoring multi-session agent swarms

## Health States
| State | Triggers |
|---|---|
| Healthy | Recent activity, normal token use |
| Degraded | Slow responses, elevated error rate |
| Stalled | No activity for > 30s |
| Dead | No activity for > 120s |

## Post-Compaction Probe
After auto-compaction, runs a lightweight responsiveness check before resuming. If the tool executor returns `Degraded` or `Failed`, the session is flagged rather than blindly continuing.

## Commands
- `/health sessions` — Show health status of all active sessions
- `/health check <session-id>` — Probe a specific session
- `/health probe-compaction` — Run post-compaction health check
- `/health stalled` — List stalled/dead sessions
- `/health recover <session-id>` — Attempt to revive a stalled session
- `/health metrics <session-id>` — Show token utilization and error rate

## Examples
```
/health sessions
# s-abc123: Healthy (tokens: 42%, idle: 3s)
# s-def456: Stalled (tokens: 78%, idle: 45s)
# s-ghi789: Dead (tokens: 95%, idle: 180s)

/health probe-compaction
# PostCompactionProbe: tool executor responsive (ProbeResult::Healthy)

/health metrics s-abc123
# Tokens: 21,000/50,000 (42%)  Errors: 0  Last activity: 3s ago
```
