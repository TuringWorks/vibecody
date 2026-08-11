---
name: "Parallel Tool Scheduler"
description: "Parallel Tool Scheduler: Dependency-tracked concurrent tool execution — up to N tools run in parallel when declared dependencies have completed. Use when the task involves parallel tool scheduler, concurrent tools, tool dependency DAG, sequence write conflicts."
category: performance
triggers: ["parallel tool scheduler", "concurrent tools", "tool dependency DAG", "sequence write conflicts"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Parallel Tool Scheduler

Dependency-tracked concurrent tool execution — up to N tools run in parallel when declared dependencies have completed. Tools sharing write targets are automatically sequenced. Matches Claude Code 1.x behaviour.

## When to Use
- Running multiple independent tool calls concurrently (e.g. reading several files at once)
- Expressing tool dependency DAGs to prevent race conditions on shared files
- Monitoring the execution status of in-flight tool batches
- Cancelling in-flight tool batches on error or user request

## Key Concepts
- **ToolJobId** — unique identifier per tool invocation
- **JobState** — Pending / Running / Completed / Failed / Skipped / Cancelled
- **write_targets** / **read_targets** — auto-infers ordering when `auto_sequence_writes` is on
- **tick()** — promotes eligible pending jobs to Running (call repeatedly)
- **max_concurrency** — default 10, matches Claude Code 1.x

## TickResult
- `Dispatched(ids)` — these jobs are now Running; caller should execute them
- `Blocked` — no jobs can start yet; wait for running jobs to finish
- `Done` — all jobs in terminal state

## Commands
- `/tools parallel N` — set max concurrency to N
- `/tools status` — show running/pending/completed counts
- `/tools cancel` — cancel all in-flight jobs

## Examples
```
/tools parallel 5
# max_concurrency set to 5

/tools status
# pending: 3  running: 2  completed: 7  failed: 0
```
