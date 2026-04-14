# Event Bus

Typed in-process lifecycle event bus for extensions and internal observability.
Subscribers react to 30+ typed `BusEvent` variants — session, agent, tool, provider,
streaming, memory, file, cost, and extension lifecycles — without spawning shell
subprocesses.

## When to Use

- **Use `event_bus`** when you need in-process, synchronous delivery of typed events to
  multiple subscribers with zero IPC overhead. Ideal for WASM/native extensions that
  must react to events in the same address space.
- **Use hooks (stdin/stdout JSON)** when you need out-of-process notification, e.g. to
  shell scripts or external services that cannot be loaded as Rust/WASM modules.
- Both can coexist: hooks fire first at the CLI boundary; the event bus fires internally
  after the hook allow/block decision has been made.

## Blocking vs Observational Subscriptions

Two kinds of subscriptions exist:

| Kind | When to use | `is_blocking_candidate` |
|---|---|---|
| **Observational** | Logging, metrics, UI updates, cost tracking | All non-blocking events |
| **Blocking** | Security policy, quota enforcement, argument validation | `ToolCall`, `BeforeProviderRequest` |

Only `ToolCall` and `BeforeProviderRequest` honour a `HandlerDecision::Block` return.
On all other event types `Block` is silently treated as `Continue` to prevent accidental
deadlocks. Always return `Continue` in observational handlers.

## Priority Guidelines

| Priority range | Use case |
|---|---|
| `100+` | Security policy enforcers (run first, can block) |
| `10–99` | Quota / rate-limit checks |
| `1–9` | Argument mutation or enrichment |
| `0` | Ordinary observers (logging, metrics) |
| `< 0` | Low-priority telemetry, best-effort replay |

Higher priority handlers execute first. If a blocking handler at priority 100 returns
`Block`, handlers at lower priorities are never called, saving CPU.

## ByPrefix vs ByType Filters

- **`ByPrefix("tool_")`** — Broad category subscription. Catches every `tool_call`,
  `tool_result`, and `tool_blocked` variant with a single filter. Useful for general
  tool observability or security scanners that inspect all tool activity.
- **`ByType(vec!["tool_call".into()])`** — Precise subscription. Use when you only care
  about one or a small explicit set of event types and want to avoid noise from related
  variants.
- `ByPrefix` is a prefix match on `type_name()` (e.g. `"session_"` matches
  `session_init`, `session_end`, `session_before_compact`, `session_after_compact`).
- Prefer `ByType` for blocking handlers; prefer `ByPrefix` for broad observers.

## Custom Events for Extension-Defined Types

Extensions emit `BusEvent::Custom { event_type, payload }` to broadcast their own
domain events without adding new enum variants:

```rust
bus.emit(BusEvent::Custom {
    event_type: "my_ext.analysis_complete".into(),
    payload: serde_json::to_string(&result).unwrap(),
});
```

Subscribe with `ByType(vec!["custom".into()])` to receive all custom events, then
dispatch on `event_type` inside the handler. Use a namespace prefix (e.g.
`"my_ext."`) to avoid collisions with other extensions.

## History Replay

The bus retains a bounded ring buffer of recent events (`max_history` defaults to 256).
Use this for:

- **Debugging**: `bus.history()` returns a snapshot of the last N events for inspection.
- **Catch-up**: When a new subscriber registers mid-session, replay history to bring it
  up to date.
- **Audit trails**: Persist `bus.history()` to JSONL at session end.

Call `bus.clear_history()` before replaying a recorded session to avoid mixing live and
replayed events.

## Thread Safety

`EventBus` is `Send + Sync` via `Arc<Mutex<_>>` internals. All handler closures must
satisfy `Send + Sync + 'static`. Wrap captured mutable state in `Arc<Mutex<T>>`:

```rust
let count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
let c = Arc::clone(&count);
bus.subscribe(EventFilter::All, 0, move |_| {
    *c.lock().unwrap() += 1;
    HandlerDecision::Continue
});
```

For process-wide shared access use `global_bus()` which returns an `Arc<EventBus>`
backed by a `OnceLock` singleton. In unit tests, always construct a local
`EventBus::with_history(N)` to avoid cross-test state leakage.

## REPL Commands

```
/events emit <type> [payload]   — Emit a custom event on the global bus
/events history [--n <count>]   — Show the last N events from the global bus
/events subscribers             — List active subscription IDs and filters
/events clear                   — Clear global bus history
```
