# Hook Abort Protocol

Implement and manage pre/post-tool-use hooks with structured exit-code signaling, JSON decision payloads, abort signals, and progress event channels. Provides claw-code parity for the Claude Code hook protocol.

## When to Use
- Enforcing allow/block decisions before tool execution
- Emitting structured JSON decisions from hooks (action: block/allow/modify)
- Coordinating abort signals across concurrent hook invocations
- Streaming hook progress events to an observer channel
- Aggregating multiple hook outputs into a single gate decision

## Commands
- `/hooks list` — Show all configured PreToolUse / PostToolUse hooks
- `/hooks test <tool>` — Dry-run hooks for a tool call without executing
- `/hooks parse <exit-code> <stdout>` — Parse a hook invocation result
- `/hooks aggregate` — Aggregate outputs from all hooks for a tool
- `/hooks context <session> <tool> <input>` — Build hook JSON context payload

## Protocol
Exit codes: **0** = allow, **2** = block, **1** = non-blocking warning

JSON stdout (preferred over plain text):
```json
{ "action": "block", "reason": "rm -rf detected", "message": "Blocked for safety", "suggest_retry": false }
```

## Abort Signal
`AbortSignal` propagates cancellation across cloned handles — aborting the original immediately marks all clones as aborted.

## Progress Events
`HookAbortController` owns an mpsc channel. Emit `Started / Running / Completed` events; observers receive them via `take_receiver()`.

## Examples
```
/hooks test Bash
# Runs all PreToolUse hooks with {"command": "..."} — reports allow/block

/hooks parse 2 "dangerous rm -rf detected"
# → blocking: true, message: "dangerous rm -rf detected"

/hooks parse 0 '{"action":"block","message":"policy violation"}'
# → blocking: true (JSON overrides exit 0)
```
