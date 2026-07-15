# Fluxo — a durable workflow engine in Rust

Fluxo is a Rust, Conductor-compatible durable workflow orchestration engine. It is the
platform layer that VibeCody's orchestration surfaces build on. It is intentionally a
standalone set of crates with no dependency on the rest of VibeCody, so it can be reused,
tested, and reasoned about in isolation.

## Why it exists

VibeCody already owns a durable job store, a DAG tool scheduler, RBAC, OTel, and rich
multi-agent orchestration — but it was never assembled into the one loop that Orkes /
Netflix Conductor is built around:

```
define (DSL) → register (versioned) → trigger (cron/webhook/event)
    → schedule tasks → workers execute by type → persist each transition
    → fork / switch / loop / wait / human / sub-workflow → complete / compensate
    → observe (timeline + metrics)
```

Fluxo is that loop, as a clean, high-performance Rust core.

## Crates

| Crate | Responsibility | Status |
|---|---|---|
| `fluxo-core` | Domain model, **Conductor-compatible JSON DSL**, validation, `${…}` expression resolution, and the **pure decider** (the workflow state machine). No I/O. | ✅ implemented + tested |
| `fluxo-store` | `Store` trait + backends: `MemoryStore` (always on), `SqliteStore` (feature `sqlite`, default), `PostgresStore` (feature `postgres`). | ✅ implemented + tested |
| `fluxo-engine` | Ties core + store into a runnable engine: `register` → `start` → `decide` (to fixed point) → `poll` / `complete_task` → `signal` / `pause` / `resume` / `terminate` / `reap`. Per-task **retries + timeouts**. | ✅ implemented + tested |
| `fluxo-server` | Axum HTTP API: workflow CRUD, execute, run control, task poll/complete, SSE timeline. Ships a `fluxo-server` binary. | ✅ implemented + tested |
| `fluxo-worker` | Poll-by-task-type worker client + task handler registry; stateless & scalable. | ✅ implemented + tested |
| `fluxo-cli` | `fluxo` binary — thin HTTP client: register/run/tail(SSE)/ls/runs/signal. | ✅ implemented + tested |

## Design principles

- **Pure core.** `fluxo-core::decide` is a pure function `(def, run, now) → Decision`. No
  clock, no I/O, no randomness — all effects live at the edges (`fluxo-engine`). This is
  what makes the state machine exhaustively testable.
- **Conductor-compatible DSL.** Workflow definitions deserialize from the same JSON shape
  Netflix/Orkes Conductor uses (`tasks[]` with `type`, `taskReferenceName`,
  `inputParameters`, `decisionCases`, `forkTasks`, `joinOn`, …). Import path from Conductor
  is a feature, not an accident.
- **Pluggable storage.** One `Store` trait, three backends. SQLite for zero-config
  local-first; Postgres for scale; memory for tests.
- **Functional Rust.** Iterator combinators, `let` over `let mut`, exhaustive `match`, no
  `unwrap`/`expect`/`panic` in library paths.

## Supported task types (v1)

`SIMPLE` (worker), `SWITCH` (decision), `FORK_JOIN` + `JOIN`, `FORK_JOIN_DYNAMIC`,
`DO_WHILE` (loop), `SET_VARIABLE`, `INLINE`, `WAIT`, `HUMAN`, `SUB_WORKFLOW`, `TERMINATE`.
Every external task honors `retryCount` / `retryDelaySeconds` / `retryLogic`
(FIXED | EXPONENTIAL_BACKOFF), `timeoutSeconds`, and `optional`.
Deferred: `JSON_JQ_TRANSFORM`, `HTTP`, `EVENT`, `START_WORKFLOW`, and the `LLM_*` AI task
family (next milestones).

## Build & test

```bash
cargo test -p fluxo-core -p fluxo-store -p fluxo-engine -p fluxo-server   # default (sqlite)
cargo test -p fluxo-store --features postgres                            # needs a reachable Postgres
cargo run  -p fluxo-server                                               # FLUXO_ADDR / FLUXO_DB
```
