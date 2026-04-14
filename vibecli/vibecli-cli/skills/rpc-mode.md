# RPC Mode

Bidirectional stdin/stdout JSONL protocol for embedding VibeCLI in non-Rust
processes — Python scripts, Node.js tools, CI pipelines, and editor plugins.

## When to Use RPC Mode vs HTTP Daemon

| Situation | Recommendation |
|-----------|----------------|
| Scripting VibeCLI from Python/Node.js in the same process group | **RPC mode** — no network port required |
| CI pipeline step that launches VibeCLI as a subprocess | **RPC mode** — simple pipe I/O, no firewall rules |
| Multiple independent clients on the same machine | **HTTP daemon** (`--serve`) — one long-lived process, many callers |
| Remote callers over a network | **HTTP daemon** — exposes REST/SSE endpoints |
| Editor plugin that wants low-latency token streaming | **RPC mode** — lower overhead than HTTP round-trips |
| Shared agent pool shared across services | **HTTP daemon** — centralised queue and rate limiting |

## Frame Type Conventions

Every line (inbound and outbound) is a JSON object with a mandatory `"type"` field.

### Inbound (caller → VibeCLI)
| type | Required fields | Purpose |
|------|-----------------|---------|
| `send_message` | `content`, `role` | Submit a user (or system) turn |
| `interrupt` | — | Cancel the current generation |
| `set_config` | `key`, `value` | Update a runtime config key |
| `shutdown` | — | Gracefully stop the RPC session |
| `ping` | `id` | Health-check; echoed back as `pong` |

### Outbound (VibeCLI → caller)
| type | Key fields | Purpose |
|------|------------|---------|
| `session_start` | `session_id` | Emitted once when the session opens |
| `agent_start` | — | Agent begins processing a turn |
| `token_delta` | `text` | Incremental token (stream chunk) |
| `tool_call` | `name`, `args` | Agent is invoking a tool |
| `tool_result` | `name`, `output`, `exit_code` | Tool execution result |
| `token_usage` | `input`, `output`, `cost_usd` | Token accounting at turn end |
| `agent_end` | — | Agent finished processing |
| `error` | `message` | Non-fatal error; session continues |
| `session_end` | `session_id` | Emitted once when the session closes |
| `pong` | `id` | Response to `ping` (same `id`) |

## LF-Only Framing Requirement

Always terminate each JSONL line with `\n` (0x0A). **Never** use `\r\n`
(0x0D 0x0A / CRLF). Rationale:

- Some JSON values contain Unicode paragraph separator (U+2029) or line
  separator (U+2028), which readline implementations on Windows may treat as
  line boundaries, breaking the frame if CRLF is also present.
- Python's `sys.stdin` and Node.js `readline` both split on `\n` by default;
  CRLF causes the `\r` to appear inside the parsed `"type"` value and breaks
  dispatch.
- `RpcFrame::to_jsonl()` enforces this — never bypass it by writing raw bytes.

## Error Frame on Panic

When VibeCLI catches a recoverable error inside RPC mode it emits an `error`
frame (`{ "type": "error", "message": "..." }`) and continues the session.
If the process panics (unrecoverable), the stdout pipe closes — the caller
should detect EOF and treat it as a fatal session failure. Always wrap the
spawned subprocess in a respawn loop for long-lived integrations.

## Ping / Pong Health Check

Send a `ping` frame at any time to verify the subprocess is alive:

```json
{"type":"ping","id":"hc-1"}
```

VibeCLI responds immediately (even during generation) with:

```json
{"type":"pong","id":"hc-1"}
```

The `id` is reflected verbatim. Use a monotonically increasing counter or a
UUID as the `id` to correlate responses when pipelining multiple pings.

## Session Lifecycle

```
caller                      VibeCLI
  |                            |
  |  (open pipe)               |
  |                            |-- {"type":"session_start","session_id":"..."}
  |-- {"type":"send_message"}  |
  |                            |-- {"type":"agent_start"}
  |                            |-- {"type":"token_delta","text":"..."}  (0..N)
  |                            |-- {"type":"tool_call","name":"..."}    (0..N)
  |                            |-- {"type":"tool_result","name":"..."}  (0..N)
  |                            |-- {"type":"token_usage",...}
  |                            |-- {"type":"agent_end"}
  |-- {"type":"shutdown"}      |
  |                            |-- {"type":"session_end","session_id":"..."}
  |  (pipe closes)             |
```

One `session_start` / `session_end` pair wraps the entire lifetime of the
subprocess — not individual turns.

## Token Delta Batching

When `emit_token_deltas = true`, VibeCLI emits one `token_delta` frame per
token (or small token group). Callers that prefer lower overhead can buffer
deltas client-side and render them on a timer (e.g., every 16 ms for 60 fps
display). The `agent_end` frame signals that all deltas for a turn have been
sent — flush any buffered text at that point.

If the caller does not need streaming output (e.g., a CI script that only
cares about tool results), set `emit_token_deltas = false` in `RpcModeConfig`
to suppress individual delta frames and receive only the final assembled
output via `tool_result` or a custom completion frame.

## Testing RPC Mode with MemoryTransport

`MemoryTransport` provides an in-process alternative to real pipes:

```rust
let transport = MemoryTransport::new();

// Simulate inbound message from caller.
transport.push_inbound(&RpcFrame::new("ping").with("id", json!("t1")));

// Run your handler with a reader built from the transport.
let mut reader = transport.reader();
let frame = reader.next_frame().unwrap().unwrap();

// Emit outbound frames through the writer.
let mut writer = transport.writer();
writer.send(&RpcFrame::pong(frame.get_str("id").unwrap())).unwrap();
writer.flush().unwrap();

// Assert what was sent.
let sent = transport.pop_outbound();
assert_eq!(sent[0].msg_type, "pong");
```

Use `MemoryTransport` in unit tests to avoid spawning real subprocesses and
to keep tests deterministic and fast.

## Commands

- `/rpc start [--session-id ID]` — Launch VibeCLI in RPC mode (connects to stdin/stdout of the calling process)
- `/rpc send "<message>"` — Send a `send_message` frame and wait for `agent_end`
- `/rpc ping` — Send a `ping` and print the round-trip latency
- `/rpc shutdown` — Send a `shutdown` frame and wait for `session_end`
- `/rpc config set <key> <value>` — Send a `set_config` frame at runtime
