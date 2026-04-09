# 01 — Stale Counts in Documentation

> Numbers in docs, CLAUDE.md, AGENTS.md, and MEMORY.md that no longer match reality.

| What | Where Claimed | Claimed Value | Actual Value | Delta |
|------|---------------|---------------|--------------|-------|
| Tauri commands | CLAUDE.md line 29, MEMORY.md | 360+ | **1,045** | +685 (2.9x) |
| Tauri commands | docs/vibeui.md line 24 | 200+ | **1,045** | +845 (5.2x) |
| Rust modules | CLAUDE.md line 76, MEMORY.md | ~196 | **222** | +26 |
| VibeUI panels | MEMORY.md | 196+ | **235** standalone + 39 composites = **274** | +78 |
| VibeUI panels | docs/architecture.md line 17 | 187 | **274** | +87 |
| Skill files | MEMORY.md | ~550 | **599** | +49 |
| Skill files | docs/architecture.md line 17 | 568 | **599** | +31 |
| Skill files | docs/PLUGIN-DEVELOPMENT.md line 56 | 526 | **599** | +73 |
| Skill files | docs/development.md line 42 | 500+ | **599** | +99 |
| REPL commands | MEMORY.md | 106+ | **126** | +20 |
| REPL commands | docs/development.md line 34 | 80+ | **126** | +46 |
| Unit tests | docs/architecture.md line 524 | 9,570 | **~10,535** | +965 |
| CSS custom properties | MEMORY.md | 85+ | **133** | +48 |
| Workspace members | MEMORY.md | 6 | **9** | +3 |
| AI providers | MEMORY.md | 18 | **22** | +4 |

## Additional Notes

- The Tauri macro is `tauri::generate_handler!`, not `invoke_handler!` as stated in CLAUDE.md and MEMORY.md. `invoke_handler` is the builder method.
- CLAUDE.md repo layout lists 4 crates under `vibeui/crates/` but actual count is 5 (missing `vibe-collab`).
- docs/vibecli.md only documents 25 out of 126 REPL slash commands.
- `lib.rs` has 54 `pub mod` declarations, `main.rs` has 14, but 222 `.rs` files exist — the "declare in both files" pattern from CLAUDE.md is not consistently followed.
