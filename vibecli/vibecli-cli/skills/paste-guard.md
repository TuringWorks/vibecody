# Paste Guard

Protect the TUI input handler from large or malicious pastes by collapsing
bracketed paste events to compact markers and storing the full content in a
ring buffer.

## Why Bracketed Paste Mode Matters

Modern terminals support *bracketed paste mode* (BPM): when enabled the
terminal wraps any pasted text in `ESC[200~` … `ESC[201~` escape sequences.
Without BPM, a paste is indistinguishable from typed input — every newline
triggers an immediate submission, and any AI prompt embedded in the pasted
text is executed verbatim. Enabling BPM lets the application intercept the
entire paste before it reaches the command parser, preventing *prompt bombing*
and accidental multi-command execution.

Enable BPM on session start by writing `ENABLE_BRACKETED_PASTE` to stdout and
disable it on exit with `DISABLE_BRACKETED_PASTE`:

```rust
use vibecli_cli::paste_guard::{ENABLE_BRACKETED_PASTE, DISABLE_BRACKETED_PASTE};
print!("{}", ENABLE_BRACKETED_PASTE);   // on startup
// ... run TUI ...
print!("{}", DISABLE_BRACKETED_PASTE);  // on clean exit / panic handler
```

## Threshold Configuration

The `line_threshold` field in `PasteGuardConfig` sets the boundary between
*inline* (small) and *collapsed* (large) pastes. The default of **10 lines**
works well for REPL sessions. Raise it for editors that frequently paste
larger snippets; lower it for security-sensitive environments.

```rust
use vibecli_cli::paste_guard::PasteGuardConfig;

let config = PasteGuardConfig {
    line_threshold: 20,          // collapse pastes longer than 20 lines
    max_stored_pastes: 50,       // keep last 50 pastes in memory
    show_preview_lines: 5,       // show first 5 lines before the marker
    auto_expand_under_threshold: true,
};
```

## Marker Format Convention

Collapsed pastes are represented by a fixed-format marker:

```
[paste #<id> +<line_count> lines]
```

Examples:
- `[paste #1 +47 lines]` — first paste, 47 lines
- `[paste #12 +3 lines]` — twelfth paste, 3 lines (collapsed if threshold < 3)

The format is intentionally terse so it fits on a single terminal line. Parse
the id back with `PasteGuard::marker_to_id(marker)`.

## Expansion Workflow

When the user wants to inspect or reuse a collapsed paste (e.g. Ctrl+E on the
marker):

1. Detect the marker string under the cursor or at the end of the input buffer.
2. Call `guard.expand_marker(marker)` — returns `Some(&str)` with the full
   content if the paste is still in the ring buffer, or `None` if it has been
   evicted.
3. Replace the marker in the input buffer with the returned content.
4. Re-render the input widget.

```rust
if let Some(full) = guard.expand_marker(&marker_under_cursor) {
    input_buffer.replace(&marker_under_cursor, full);
}
```

## max_stored_pastes Sizing

The ring buffer caps memory usage. Each stored paste retains its full string
content. Choose `max_stored_pastes` based on expected paste size and available
RAM:

| Session type | Recommended `max_stored_pastes` |
|---|---|
| Lightweight REPL | 10–20 |
| Code editor | 50–100 |
| High-throughput pipeline | 200+ (monitor heap) |

When the buffer is full the oldest entry is silently evicted. If `expand_marker`
returns `None`, inform the user that the paste is no longer available and offer
to re-paste.

## Integration with the TUI Input Handler

Feed every raw crossterm `Event::Paste` (or manually assembled byte stream)
through `PasteGuard::process`:

```rust
use vibecli_cli::paste_guard::{PasteGuard, PasteGuardConfig};
use crossterm::event::Event;

let mut guard = PasteGuard::new(PasteGuardConfig::default());

// Inside the crossterm event loop:
if let Event::Key(key_event) = event {
    // Build raw_input from key codes including escape sequences.
    let raw = encode_key_event(key_event);
    let result = guard.process(&raw);

    if result.was_collapsed {
        // Display the marker in the input widget.
        input.set_text(&result.processed_input);
        status_bar.show(&format!(
            "Paste collapsed — {} lines stored as {}",
            result.paste_events[0].line_count,
            result.paste_events[0].marker()
        ));
    } else {
        input.set_text(&result.processed_input);
    }
}
```

## Commands

- `/paste list` — List all paste ids currently in the ring buffer with line
  counts and char counts.
- `/paste show <id>` — Print the full content of paste `#<id>`.
- `/paste expand` — Expand the most recent paste marker in the current input.
- `/paste clear` — Discard all stored pastes and reset the id counter.
- `/paste threshold <n>` — Change the line threshold for the current session.
