# Smart Diff

Syntax-aware diff renderer — splits unified diff hunks by semantic blocks (fn, struct, impl, class, def), and renders side-by-side or inline colour views. Matches Cursor 4.0's diff renderer.

## When to Use
- Reviewing large refactors with better hunk context than line numbers
- Displaying side-by-side before/after views for code changes
- Annotating hunks with the function/class they belong to
- Computing diff statistics (added/removed/semantic hunk counts)

## Render Modes
- **Unified** — classic `--- / +++` diff text
- **SideBySide { col_width }** — two-column old | new view
- **InlineColour** — ANSI coloured terminal output

## Semantic Block Detection
Walks backwards from each hunk's `old_start` to find the nearest enclosing:
`fn`, `struct`, `enum`, `impl`, `trait`, `class`, `def`, `function`, `const`, `type`

## Commands
- `/diff show` — show unified diff of current changes
- `/diff side-by-side` — two-column diff view
- `/diff semantic` — annotate hunks with block names
- `/diff stats` — show files/hunks/added/removed counts

## Examples
```
/diff show
# --- src/lib.rs
# +++ src/lib.rs
# @@ -10,3 +10,4 @@ fn greet
#  fn greet(name: &str) {
# -    format!("hello {}", name)
# +    format!("Hello, {}!", name)

/diff stats
# 1 file, 1 hunk, +2 -1, 1 semantic hunk
```
