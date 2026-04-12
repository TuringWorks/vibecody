# File Watcher

Debounced file-system change detection for live index refresh. Batches rapid changes within a configurable debounce window (default 50ms) and emits `ChangeBatch` events. Matches Cursor 4.0 and Cody 6.0's sub-50ms reindex latency.

## When to Use
- Triggering live symbol reindex when source files change
- Debouncing bursts of save events (e.g. auto-format on save)
- Watching for new files created by scaffolding tools
- Pausing/resuming watching during bulk operations

## Key Types
- **ChangeKind** — Created / Modified / Deleted / Renamed
- **ChangeBatch** — debounced batch of events with window_start/end
- **WatcherStatus** — Idle / Watching / Paused / Error

## Ignore Patterns (default)
- `target/**` — Rust build artifacts
- `.git/**` — Git internals
- `node_modules/**` — npm packages

## Commands
- `/watch start <path>` — begin watching a directory
- `/watch stop <path>` — stop watching a directory
- `/watch status` — show current watcher state and stats
- `/watch debounce <ms>` — change the debounce window

## Examples
```
/watch start src/
# Watching: src/ (debounce: 50ms)

/watch status
# status: watching  paths: 1  events: 142  batches: 38
```
