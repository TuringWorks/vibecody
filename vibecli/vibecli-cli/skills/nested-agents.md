---
triggers: ["nested agents", "recursive subagents", "agent tree", "child agent", "subagent spawning"]
tools_allowed: ["read_file", "write_file", "bash"]
category: agent
---

# Nested Agent Architectures

When designing recursive or hierarchical agent systems:

1. **Max Depth Limit** — Enforce a hard maximum recursion depth of 4 by default (root = depth 0, leaf = depth 4). Deeper trees create exponential cost growth and make debugging intractable. If a task genuinely requires more depth, flatten the intermediate levels into a single broader agent before spawning children.
2. **Context Inheritance Policy** — Child agents inherit a reduced context slice from their parent: task goal, relevant tool subset, and output schema — never the full parent conversation history. Passing full history to children doubles token cost per level and pollutes child focus. Summarize parent context to 512 tokens maximum before passing down.
3. **Cycle Detection** — Before spawning any child agent, check whether the same task signature (goal hash + tool set hash) already exists anywhere in the current agent call stack. If a cycle is detected, return the partial result from the existing node instead of spawning a new child. Log cycle detections as warnings for review.
4. **Result Aggregation Strategies** — Choose an aggregation strategy appropriate to the task: concatenate (independent subtasks), reduce (parallel identical tasks producing numeric results), merge-by-key (structured data), or delegate-back (child produces a partial plan the parent must finalize). Define the strategy before spawning to avoid N result formats arriving at the parent.
5. **Cancellation Propagation** — When a parent agent is cancelled or times out, immediately broadcast a cancellation signal to all active child agents. Children must respect cancellation within one iteration and return a partial result or an explicit cancellation acknowledgment. Orphaned child processes must not continue consuming resources after parent cancellation.
6. **Flat vs Nested Decision Rule** — Prefer flat (single orchestrator + parallel workers) when subtasks are independent and homogeneous. Use nested trees only when subtasks themselves require further decomposition that cannot be known in advance. If the nesting structure is known at spawn time, always flatten it upfront into a static DAG rather than dynamic recursion.
7. **Cost Attribution** — Track and attribute token costs by depth level. Surface a cost estimate before allowing the tree to grow past depth 2. Allow users to configure a budget cap per tree; when exceeded, halt spawning at the current node and synthesize a partial answer from available child results.
8. **Debugging and Tracing** — Assign a stable trace ID to each agent tree and a node ID to each agent in the tree. Emit structured trace events (spawn, complete, cancel, error) keyed by tree ID and node ID. Store traces in append-only logs to enable full tree replay and subtask isolation when debugging unexpected behavior.
