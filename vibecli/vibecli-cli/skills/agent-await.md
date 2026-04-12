---
triggers: ["agent await", "conditional pause", "wait for build", "await condition", "agent dependency"]
tools_allowed: ["read_file", "write_file", "bash"]
category: agent
---

# Agent Wait Conditions

When an agent must pause and wait for an external condition before continuing:

1. **ProcessExit Condition** — Use `ProcessExit` when the agent spawned the process itself and holds its PID (e.g., a compilation step, a test runner, a migration). Await the exit code; treat non-zero as a failure and surface the stderr output. Do not use ProcessExit to wait for processes the agent did not spawn — use LogPattern or PortOpen instead.
2. **LogPattern Condition** — Use `LogPattern` when success is signaled by a specific string appearing in a log file or stream (e.g., "Server started on port 3000", "BUILD SUCCESS"). Define the pattern as a regex, set a max scan window (default: last 10,000 lines), and specify whether the match is case-sensitive. Pair with a timeout to avoid infinite tailing. Useful for services where the PID is not available.
3. **PortOpen Condition** — Use `PortOpen` when the condition is that a TCP/UDP port becomes reachable (e.g., a database, an HTTP server, a gRPC endpoint). Poll with exponential backoff starting at 200ms. Confirm reachability with a lightweight application-level probe (HTTP 200, TCP handshake, or ping query) rather than just a TCP connection, which can succeed before the service is ready to handle requests.
4. **Timeout Setting Rule** — Set the wait timeout to 3x the expected duration of the awaited operation. If the expected duration is unknown, use 5 minutes as a default and surface a configuration prompt to the user. Never use an infinite timeout in production agents — always bound wait conditions. Log the expected vs actual duration on completion to calibrate future timeouts.
5. **Timeout Handling Strategies** — On timeout, choose one of three strategies based on task criticality: (1) fail-fast — mark the task as failed, surface the timeout with the last observed state, and stop; (2) partial-continue — proceed with degraded behavior and annotate the output as provisional; (3) escalate — pause the agent and notify the user for a manual decision. Declare the strategy at wait-condition definition time, not at timeout occurrence.
6. **Difference from Polling** — Wait conditions are event-driven or process-bound; polling is interval-based. Prefer wait conditions when the event source can be observed directly (PID, log stream, port). Use polling only as a last resort for conditions with no observable signal (e.g., an external API status page). When polling is unavoidable, use exponential backoff with jitter and a maximum interval of 30 seconds.
7. **Integration with Thought Stream** — When an agent enters a wait condition, emit a thought-stream event: `{type: "waiting", condition: "PortOpen", target: "localhost:5432", expected_duration_s: 10, timeout_s: 30}`. This allows the UI to display a meaningful waiting state rather than appearing frozen. On condition resolution, emit `{type: "wait_resolved", elapsed_s: N, outcome: "success"|"timeout"}`.
8. **Chaining Multiple Conditions** — When a task requires multiple sequential conditions (e.g., build completes → port opens → health check passes), define a condition chain rather than nesting agent steps. Each condition in the chain inherits the remaining timeout budget from the previous condition. If any condition in the chain fails, the chain halts immediately and reports which condition failed, without attempting subsequent conditions.
