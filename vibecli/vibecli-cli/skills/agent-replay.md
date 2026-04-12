# Agent Replay Debugger

Records agent execution traces and replays them step-by-step for debugging. Extends `repro_agent` with time-travel and assertion support.

## Step Kinds
- `user_msg` — user input
- `asst_msg` — agent response
- `tool_call` — tool invocation with args + result
- `tool_result` — raw tool return
- `state_transition` — FSM event
- `thought` — planning/reasoning step
- `error` — execution error

## Key Types
- **TraceRecorder** — records steps into an `ExecutionTrace`
- **TraceReplayer** — step/seek/run_all with position tracking
- **ExecutionTrace** — session_id, agent_name, steps, duration

## Replay Operations
- `step()` — advance to next step
- `seek(index)` — time-travel to any step
- `run_all()` — fast-forward to end
- `assert_step_output(index, expected)` — regression assertion

## Commands
- `/replay load <session-id>` — load a saved trace
- `/replay step` — advance one step
- `/replay seek <N>` — jump to step N
- `/replay run` — fast-forward to end
- `/replay assert <N> <expected>` — assert step output

## Examples
```
/replay load sess-abc123
# Trace: sess-abc123 (coder-agent), 8 steps, 340ms

/replay step
# [00] user_msg: "Fix the bug in main.rs"

/replay seek 4
# [04] tool_call: read_file(main.rs) → "fn main() {...}"
```
