# Stream Tool Args

Real-time partial argument streaming for LLM tool calls via `toolcall_delta` events. Pi-mono gap bridge: Phase B3.

## Rules

### 1. When to emit toolcall_delta events
Emit a `ToolCallDelta` (with `is_complete: false`) for every chunk of tool-call argument JSON
received during streaming. Emit a final delta (with `is_complete: true`) or call `on_complete`
when the LLM finishes generating the argument block. Never buffer the full payload before
notifying the UI layer — the whole point is real-time display.

### 2. Using PartialHint for UI status bars
After each `push`, inspect the returned `PartialParseResult::hint`:

| Variant | Meaning | Suggested TUI label |
|---|---|---|
| `FilePath(p)` | A `"path"` or `"file_path"` key is complete | `"write_file: editing src/main.rs…"` |
| `CommandFragment(cmd)` | `"command"` key value has arrived | `"bash: running cargo build…"` |
| `ContentLength(n)` | `"content"` key is streaming; `n` chars so far | `"write_file: receiving content (1 024 chars)…"` |
| `UnknownKey(k)` | First key found; semantics unclear | `"tool: reading <k> arg…"` |
| `None` | No keys parsed yet | `"tool: reading args…"` |

Call `render_partial_hint(&result, tool_name)` to obtain a ready-made status string.

### 3. Accumulator lifecycle
1. Create one `ToolArgAccumulator` per unique `call_id` at the start of the tool call stream.
2. Call `push(fragment)` for every delta.  The returned `PartialParseResult` is safe to read
   immediately on the UI thread.
3. On the final delta call `finalize()` to obtain the complete `serde_json::Value`.
4. Discard the accumulator after finalization.

Use `StreamingToolCallManager` when multiple tool calls may arrive interleaved in the same
generation (e.g. parallel tool use).  It creates and routes to the correct accumulator
automatically via `on_delta`.

### 4. Finalize error handling
`finalize()` returns `Err(String)` when the buffer is not valid JSON.  This can happen if:
- The LLM was interrupted mid-stream (e.g. user cancellation or network error).
- The provider sent malformed partial JSON that was never completed.

On error, log the raw `buffer()` for diagnostics, surface a user-facing "tool call failed"
message, and skip execution of the tool.  Do **not** panic or silently ignore the error.

### 5. Sequence numbers for deduplication and ordering
Every `push` increments `ToolArgAccumulator::sequence()` by 1.  When receiving deltas from a
network source that may deliver them out of order, buffer deltas by `sequence` before pushing to
ensure deterministic reconstruction.  The `ToolCallDelta::sequence` field carries the
provider-assigned counter; the accumulator's `sequence()` accessor reflects how many fragments
have been pushed locally.

### 6. Concurrency and thread safety
`ToolArgAccumulator` and `StreamingToolCallManager` are not `Send + Sync` — they are designed
for use on a single async task or the UI thread.  If you need to share updates across threads,
clone the `PartialParseResult` (it is `Clone`) or pass it over a channel.  Never share a mutable
reference to the accumulator across thread boundaries.

## Commands
- `/stream status` — show active streaming tool calls and their current partial hints
- `/stream clear` — discard all in-progress accumulators (use after cancelling a generation)
