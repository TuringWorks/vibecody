---
triggers: ["Elixir", "elixir lang", "GenServer", "OTP", "supervisor", "elixir pattern matching", "elixir pipe operator", "BEAM"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["elixir"]
category: elixir
---

# Elixir Language Patterns

When working with Elixir:

1. Use pattern matching in function heads for control flow instead of conditionals — define multiple function clauses like `def handle(:ok, data)` and `def handle(:error, reason)` to make code declarative and self-documenting.
2. Compose data transformations with the pipe operator `|>` — structure pipelines as `input |> step1() |> step2() |> step3()` where each function takes the previous result as its first argument; keep each step focused on one transformation.
3. Implement GenServer for stateful processes — define `init/1`, `handle_call/3` (synchronous), and `handle_cast/2` (asynchronous) callbacks; keep the state minimal and let the client API module wrap `GenServer.call/cast` for a clean interface.
4. Build fault-tolerant systems with Supervisors — use `one_for_one` strategy when children are independent, `one_for_all` when they depend on each other, and `rest_for_one` for ordered dependencies; define supervision trees in your `Application.start/2`.
5. Prefer immutable data and pure functions — Elixir data structures are immutable by default; use `Map.put/3`, `Keyword.merge/2`, and `Enum` functions to transform data rather than accumulating mutations in variables.
6. Use `with` expressions for chaining operations that may fail — `with {:ok, user} <- fetch_user(id), {:ok, token} <- generate_token(user)` provides clean error handling without deeply nested case statements; add an `else` clause for error matching.
7. Leverage protocols for polymorphism — define a protocol with `defprotocol` and implement it for different types with `defimpl`; use this instead of type-checking with `is_map/1` or `is_list/1` for extensible, clean dispatch.
8. Write comprehensive tests with ExUnit — use `describe` blocks for grouping, `setup` callbacks for fixtures, `assert` and `assert_receive` for message-passing tests; tag tests with `@tag :integration` and filter with `mix test --only integration`.
9. Use `Task` and `Task.async_stream/3` for concurrent work — `Task.async` spawns a process and `Task.await` collects the result; use `Task.Supervisor` for production code to handle failures gracefully instead of bare `Task.async`.
10. Structure applications with the `Application` behaviour — start top-level supervisors, ETS tables, and persistent connections in `start/2`; use `Application.get_env/3` for runtime configuration and config providers for release-time config.
11. Use ETS (Erlang Term Storage) for fast in-memory key-value storage shared across processes — create tables with `:ets.new/2`, prefer `read_concurrency: true` for read-heavy workloads, and wrap access in a GenServer for coordinated writes.
12. Profile and debug with built-in tools — use `:observer.start()` for system monitoring, `:recon` for production introspection, `IEx.pry()` for interactive debugging, and `mix profile.eprof` or `mix profile.fprof` to identify performance bottlenecks.
