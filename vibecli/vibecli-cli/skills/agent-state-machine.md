# Agent State Machine

Formal FSM for the agent execution loop — exposes state (Idle/Planning/Executing/Reviewing/Blocked/Complete/Aborted) as a first-class API. Matches Cody 6.0's agent FSM and Claude Code SDK state transitions.

## When to Use
- Displaying an accurate status badge in the UI (● idle / ▶ executing / ⏸ blocked)
- Subscribing to state transitions in hooks or MCP tools
- Detecting when an agent is blocked and why (AwaitingApproval / RateLimited / BudgetExceeded)
- Restarting a completed/aborted session with TaskReceived

## States
| State | Meaning |
|---|---|
| Idle | No active task |
| Planning | Breaking down the request |
| Executing | Running tool calls |
| Reviewing | Waiting for user approval |
| Blocked(reason) | Paused on external condition |
| Complete | Task done successfully |
| Aborted(error) | Failed or cancelled |

## Events
`TaskReceived`, `PlanStarted`, `PlanReady`, `ExecutionStarted`, `ToolsDispatched`, `OutputReady`, `UserApproved`, `ApprovalRequired`, `WaitingForTool(name)`, `Unblocked`, `TaskComplete`, `Abort(reason)`, `BudgetExceeded`, `RateLimited`

## Commands
- `/agent state` — show current FSM state and badge
- `/agent transitions` — list valid next events
- `/agent history` — show recent state transitions

## Examples
```
/agent state
# ▶ executing (Planning → Executing 3s ago)

/agent history
# Idle → Planning  (TaskReceived)
# Planning → Executing  (PlanReady)
```
