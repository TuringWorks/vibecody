# Message Queue
Thread-safe agent message queues for mid-turn steering and post-turn follow-up injection. Pi-mono gap bridge (Phase A4): mirrors `agent.steer(msg)` / `agent.followUp(msg)`.

## When to Use Steering vs Follow-up

**Steering queue (`AgentMessageQueues::steer`)** — use when the message must reach the agent *before* the current turn concludes, i.e. while tool calls are still in flight. Examples: correcting the agent's approach mid-search, injecting a constraint discovered at runtime, redirecting focus when an early tool result signals a wrong path.

**Follow-up queue (`AgentMessageQueues::follow_up_with`)** — use when the message is only meaningful *after* the agent has finished all tool work for the turn. Examples: "now summarise what you found", "format the result as JSON", secondary tasks that depend on the completed primary result.

## DrainMode Selection

| Situation | Recommended mode |
|---|---|
| Real-time human guidance where each correction should be processed individually | `DrainMode::OneAtATime` |
| Batch follow-up tasks that should all start together (e.g. parallel post-processing) | `DrainMode::All` |
| Slow human typers where you want to debounce several quick edits | `DrainMode::All` on steering |
| Production agent loops where you want predictable one-message-at-a-time pacing | `DrainMode::OneAtATime` |

Default for both queues in `AgentMessageQueues::new()` is `OneAtATime`.

## Max Queue Size Guidelines

- Default cap is **1,000 messages** — sufficient for virtually all interactive sessions.
- For ephemeral per-request queues, set `max_size` to a small value (e.g. `5–20`) to catch runaway callers early.
- For long-running daemon agents that accumulate user steering over many minutes, keep the default or raise it.
- When `enqueue` returns `Err`, the caller must decide whether to drop the message, block until space is available, or surface an error to the user — the queue itself never blocks.

## Thread Safety Guarantees

- All methods on `MessageQueue` and `AgentMessageQueues` take `&self` (shared reference) and are safe to call from any thread.
- The internal `VecDeque` is protected by a `Mutex`; contention is minimal because each lock is held only for the duration of a single push or pop.
- `MessageQueue` implements `Clone` by cloning the `Arc` — clones share the same underlying queue, enabling producer/consumer patterns without extra synchronisation.

## Integrating with an Agent Loop

```rust
use vibecli_cli::message_queue::{AgentMessageQueues, DrainMode};

// 1. Create queues when starting a turn.
let queues = AgentMessageQueues::with_modes(DrainMode::OneAtATime, DrainMode::All);

// 2. In the UI / controller thread — push guidance any time:
queues.steer("focus on security vulnerabilities").unwrap();
queues.follow_up_with("provide a remediation checklist").unwrap();

// 3. In the agent loop — drain steering between each tool call:
loop {
    // ... execute next tool call ...

    for msg in queues.drain_steering() {
        // inject msg into the running context window
        context.push(msg);
    }

    if no_more_tool_calls { break; }
}

// 4. After the turn ends — flush follow-up queue:
for msg in queues.drain_follow_up() {
    // start a new turn with this follow-up
    agent.start_turn(msg.content);
}

// 5. Check idleness before deciding to park the agent:
if queues.is_idle() {
    agent.park();
}
```

## Metadata Conventions

Messages enqueued via `steer()` automatically carry `injected_at = "between_tools"`. Messages from `follow_up_with()` carry `injected_at = "after_turn"`. You can also use `QueuedMessage::user(...).with_metadata("priority", "high")` to attach arbitrary hints that the agent loop can inspect before injecting.

## Peeking Without Consuming

`MessageQueue::peek()` returns a clone of the front message without removing it. Use this to inspect what the next message would be before deciding whether to drain (e.g. priority routing, content-based filtering).

## Clearing Stale Guidance

Call `MessageQueue::clear()` (or `AgentMessageQueues::steering.clear()`) when a turn is aborted or the user cancels a request. This prevents stale guidance from leaking into a subsequent turn.
