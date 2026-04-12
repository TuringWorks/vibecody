# Multi-File Symbol Rename

Workspace-wide symbol rename with whole-word matching, reference classification, and safe diff generation. Matches Cursor 4.0's rename refactor.

## Reference Kinds
- **Definition** — `fn foo()`, `struct Foo`, `trait Foo`
- **Call** — `foo(args)`
- **TypeAnnotation** — `let x: Foo = ...`
- **Import** — `use crate::foo;`
- **DocComment** — `/// foo is ...`

## Key Types
- **ReferenceScanner** — whole-word scan of a single file
- **RenameEngine** — workspace-wide collect + apply edits
- **RenameResult** — `files_affected`, `total_edits`, `updated_files`

## Commands
- `/rename <old> <new>` — rename symbol across workspace
- `/rename preview <old> <new>` — show edits without applying
- `/rename refs <symbol>` — list all references to a symbol

## Examples
```
/rename preview process handle
# 3 edits in 2 files:
#   src/a.rs:1 — pub fn process → pub fn handle
#   src/b.rs:1 — use crate::process → use crate::handle
#   src/b.rs:2 — process() → handle()
```
