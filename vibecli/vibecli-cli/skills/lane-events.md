# Lane Events

Structured agent event bus with typed lanes for observability, audit, and cost tracking. Events are bucketed into Tool, Plan, Memory, User, System, Error, and Cost lanes.

## When to Use
- Auditing all tool invocations and their outputs in a session
- Tracking cumulative cost across an agent run
- Filtering events by lane for targeted analysis
- Correlating plan steps with tool executions
- Debugging agent behavior by replaying the event timeline

## Lanes
| Lane | Contents |
|---|---|
| Tool | ToolCall / ToolResult events |
| Plan | PlanCreated / PlanStepStarted / PlanStepCompleted |
| Memory | MemoryStored / MemoryRetrieved |
| User | UserMessage / UserApproval |
| System | SessionStart / SessionEnd / Compaction |
| Error | ToolError / AgentError |
| Cost | TokenUsage events with model + input/output counts |

## Commands
- `/events list [--lane <lane>]` — Show events for current session
- `/events cost` — Summarize total token cost by model
- `/events replay <session-id>` — Replay all events for a session
- `/events export` — Export session event log as JSONL
- `/events filter --from <ts> --to <ts>` — Time-bounded event query
- `/events lanes` — Show event count per lane

## Examples
```
/events cost
# total: 18,432 tokens | models: claude-opus-4: 12k, claude-sonnet-4: 6k

/events list --lane error
# [14:32:01] ToolError: Bash timed out after 30s
# [14:33:15] ToolError: Edit — file not found: src/foo.rs

/events lanes
# tool: 42  plan: 8  memory: 15  cost: 20  error: 2
```
