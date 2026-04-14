# TUI IME — Input Method Editor & CJK Width Support
Zero-width APC CURSOR_MARKER embedding for IME candidate window positioning,
plus ANSI-safe Unicode East Asian Width calculations for CJK terminal layout.

## When to Use
- Any TUI input widget that must support Chinese / Japanese / Korean text entry
- Positioning the hardware cursor so that the OS IME candidate window appears
  directly below the insertion point (not at a wrong column)
- Computing display column widths of strings that may contain ANSI colour codes
  and/or CJK wide characters (2 columns per glyph)
- Truncating or wrapping rendered text without splitting wide characters

## Rules

### Rule 1 — Embed CURSOR_MARKER, never rely on byte offsets
Always call `insert_cursor_marker(line, col)` to embed the APC marker at the
correct visible column in rendered TUI output.  Never use raw byte or char
offsets to position the hardware cursor — ANSI codes and wide chars invalidate
such calculations.

### Rule 2 — Use `visible_width` for all column arithmetic
`visible_width(s)` correctly accounts for ANSI escape sequences (zero columns),
CJK wide characters (two columns), and the CURSOR_MARKER itself (zero columns).
Do NOT use `s.len()` or `s.chars().count()` to measure display width.

### Rule 3 — Strip CURSOR_MARKER before storing or hashing
The CURSOR_MARKER is a presentation-layer artefact.  Remove it with
`strip_cursor_marker(s)` before storing text in any buffer, computing diffs,
or feeding content to the AI backend.

### Rule 4 — Never split wide characters during truncation or wrapping
Use `truncate_to_width(s, max_cols)` and `wrap_to_width(s, max_cols)` instead
of slicing strings by byte or char index.  Both functions stop cleanly before
a wide character that would overflow, leaving a potentially smaller-than-max
column budget rather than a corrupted glyph.

### Rule 5 — Follow the IME composition lifecycle strictly
```
Idle  --on_composition_start()-->  Composing
Composing  --on_composition_update(preedit)-->  Composing   (repeat)
Composing  --on_composition_end(final)-->  Committed
Committed  --reset()-->  Idle
```
Call `on_composition_update` only while in the `Composing` state — it is a
no-op in other states.  Always call `reset()` after consuming `committed()` to
return the handler to `Idle`.

### Rule 6 — Check IME capability before activating composition UI
Call `is_ime_capable_terminal()` at startup.  Only show the IME preedit overlay
when it returns `true` (UTF-8 CJK locale detected).  Fall back to direct ASCII
input on non-CJK terminals to avoid spurious UI elements.

## API Reference

| Function / Type | Description |
|---|---|
| `CURSOR_MARKER` | `"\x1b_pi-cursor\x1b\\"` — APC zero-width cursor position marker |
| `insert_cursor_marker(line, col)` | Embed marker at visible column `col` in `line` |
| `find_cursor_marker(rendered)` | Return `Some(col)` where marker sits, or `None` |
| `strip_cursor_marker(s)` | Remove all occurrences of CURSOR_MARKER |
| `EawCategory::for_char(c)` | Wide / Narrow / Fullwidth / Halfwidth / Ambiguous / Neutral |
| `EawCategory::display_width()` | 2 for Wide/Fullwidth, 1 otherwise |
| `visible_width(s)` | Column count excluding ANSI escapes and CURSOR_MARKER |
| `truncate_to_width(s, max)` | Cut at `max` cols; appends `\x1b[0m` if ANSI was open |
| `wrap_to_width(s, max)` | Word-wrap preserving ANSI SGR state across lines |
| `is_ime_capable_terminal()` | `true` when `$LANG`/`$LC_ALL`/`$LC_CTYPE` is UTF-8 CJK |
| `cursor_position_sequence(row, col)` | CSI CUP sequence `"\x1b[row;colH"` (1-based) |
| `ImeHandler::new()` | Create handler in `Idle` state |
| `ImeHandler::on_composition_start()` | Idle → Composing |
| `ImeHandler::on_composition_update(text)` | Update preedit while Composing |
| `ImeHandler::on_composition_end(final)` | Composing → Committed |
| `ImeHandler::reset()` | Committed → Idle; clear both buffers |

## Examples

```rust
use vibecli_cli::tui_ime::{
    insert_cursor_marker, find_cursor_marker, strip_cursor_marker,
    visible_width, truncate_to_width, wrap_to_width,
    cursor_position_sequence, is_ime_capable_terminal,
    ImeHandler,
};

// ── CURSOR_MARKER embedding ──────────────────────────────────────────────────
let rendered = insert_cursor_marker("hello world", 5);
let col = find_cursor_marker(&rendered).unwrap(); // 5
let csi = cursor_position_sequence(3, col as u16 + 1); // "\x1b[3;6H"

// ── CJK-aware width ──────────────────────────────────────────────────────────
assert_eq!(visible_width("\x1b[1m中文\x1b[0m"), 4); // 2 wide chars

// ── Safe truncation ──────────────────────────────────────────────────────────
let t = truncate_to_width("你好世界", 5);
assert!(visible_width(&t) <= 5); // never splits 好

// ── IME composition ──────────────────────────────────────────────────────────
let mut ime = ImeHandler::new();
if is_ime_capable_terminal() {
    ime.on_composition_start();
    ime.on_composition_update("にほ");
    ime.on_composition_end("日本");
    let committed = ime.committed().to_owned();
    ime.reset();
    // Insert `committed` into the editor buffer.
    println!("committed: {committed}");
}
```
