# Stream Patcher

Streaming unified diff applicator — applies patch hunks as they arrive, with per-hunk rollback and conflict detection. Matches Claude Code 1.x and Devin 2.0's streaming patch application.

## When to Use
- Applying AI-generated file edits as a token stream rather than a whole-file rewrite
- Showing incremental file changes in the UI as they're applied
- Rolling back a bad hunk without reverting the whole patch
- Detecting merge conflicts before writing to disk

## Key Operations
- `apply_hunk(hunk)` — apply one hunk; returns Applied / Conflict / Skipped
- `rollback_last()` — undo the most recently applied hunk
- `rollback_all()` — revert to original content
- `preview_hunk(hunk)` — show what the file would look like without modifying state
- `summary()` — count applied/skipped/conflicted hunks

## HunkResult
- **Applied** — hunk applied successfully
- **Conflict { expected, got }** — context lines didn't match
- **Skipped { reason }** — hunk skipped (e.g. already applied)

## Commands
- `/patch apply <file>` — apply a unified diff to a file
- `/patch rollback` — undo the last applied hunk
- `/patch preview` — show what the patch would produce
- `/patch status` — show current patch session stats

## Examples
```
/patch apply src/lib.rs < changes.diff
# Applied: 3 hunks, +12 -7 lines

/patch rollback
# Rolled back last hunk (was: fn greet, -1 +2)
```
