# Parallel Tool Executor

Concurrent tool dispatch with sequential preflight hooks and ordered result emission. Bridges the pi-mono gap (Phase A2): VibeCody now matches Claude Code 1.x default-parallel execution behaviour.

## When Parallel Mode Applies

Use `ExecutionMode::Parallel` (the default) whenever a single assistant turn produces multiple independent tool calls — for example, reading several files simultaneously, running a formatter alongside a linter, or fetching multiple URLs in one step. Parallel mode spawns one OS thread per allowed call up to `max_concurrency` (default 8), so wall-clock time collapses to the duration of the slowest single call rather than the sum of all.

Parallel mode is **not** appropriate when calls have data dependencies (e.g., read-then-edit the same file). In those cases rely on `parallel_tool_scheduler` (dependency-tracked scheduler) to sequence jobs, or use `ExecutionMode::Sequential` as a blanket fallback.

## How Preflight Blocking Works

Before any tool executes, every `ToolCall` is passed to the `preflight` closure **sequentially in submission order**. This mirrors `beforeToolCall` hooks and ensures hook side-effects (logging, rate-limit counters, audit records) are deterministic.

If `preflight` returns `PreflightDecision::Block { reason }`, the call is converted to a stub `ToolResult` with `blocked = true`, an empty `output`, and the reason string in `error`. The tool executor is **never invoked** for blocked calls — no network request, no filesystem write, no subprocess.

Allowed calls proceed to the parallel (or sequential) execution stage.

## Result Order Guarantee

`dispatch()` always returns results in the **original submission order** of the `calls` Vec regardless of which threads finish first. Internally, each thread writes into a pre-allocated slot indexed by the call's original position; results are assembled only after all threads have joined. This means downstream code — streaming the results back to the model, serialising to JSONL — never needs to re-sort.

## Sequential Fallback Use Cases

Create a dispatcher with `ExecutionMode::Sequential` when:

- Debugging and you need deterministic, single-threaded execution.
- A downstream system cannot handle concurrent side-effects (e.g., a serial external API, a non-thread-safe mock).
- A workspace hook mandates sequential tool execution (e.g., policy enforcement that tracks inter-call state).
- Running inside a test that asserts on the order of side-effects.

Sequential mode calls the executor closure in submission order on the **calling thread** — no threads are spawned.

## Concurrency Limits

`ParallelToolDispatcher::with_concurrency(mode, max)` caps the number of simultaneously running threads. Calls are dispatched in **batches** of `max` threads; the next batch starts only after the current batch completes. This limits peak thread count and prevents thread exhaustion on large call lists.

Choose `max_concurrency` based on:
- I/O-bound tools (file reads, HTTP): high concurrency (8–32) is safe.
- CPU-bound tools (syntax analysis, code compilation): match the number of logical CPU cores.
- External APIs with rate limits: set `max_concurrency = rate_limit_per_second`.

The minimum enforced value is 1 (clamped internally), which effectively serialises execution while still using the parallel code path.

## Integrating with `parallel_tool_scheduler`

`ParallelToolDispatcher` is the **execution layer**; `ParallelToolScheduler` is the **scheduling layer**. Typical integration:

1. Use `ParallelToolScheduler` to build a dependency graph of `ToolJob`s.
2. Call `scheduler.tick()` to get the set of jobs ready to run.
3. Convert each ready `ToolJob` to a `ToolCall` and submit them as a batch to `ParallelToolDispatcher::dispatch()`.
4. Feed results back to `scheduler.mark_completed()` / `scheduler.mark_failed()`.
5. Repeat until `TickResult::Done`.

This two-layer design means dependency resolution is always correct while actual I/O benefits from true parallelism.
