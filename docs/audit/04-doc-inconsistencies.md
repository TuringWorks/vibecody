# 04 — Documentation Inconsistencies

> Contradictions, broken references, and internal conflicts across documentation files.

## P0 — Security Policy Contradiction

**docs/quickstart.md lines 304-307** instructs users to add `api_key = "sk-ant-..."` to `~/.vibecli/config.toml`.

**CLAUDE.md** and **AGENTS.md** explicitly state:
- "API keys -> ProfileStore (`~/.vibecli/profile_settings.db`)"
- "Never write keys to `*.toml`, `*.json`, or any plaintext file"

The quickstart actively contradicts the project's own security policy. Users following the quickstart will store secrets in plaintext.

**Legacy code note**: `api_keys.json` is still referenced in 2 files (`commands.rs`, `SettingsPanel.tsx`) — possibly migration code, but should be verified it's not actively reading from it.

## P1 — Broken Image/Asset References

| File | Line | Reference | Issue |
|------|------|-----------|-------|
| `docs/chat-workflow.md` | 9 | `![Chat Workflow](/chat-workflow.svg)` | Uses absolute path without `relative_url` Jekyll filter. Will 404 at `/vibecody/` base path |
| `docs/chat-workflow.md` | 11 | `[Open in draw.io](chat-workflow.drawio)` | Relative link breaks when Jekyll renders to `/chat-workflow/index.html` |

## P2 — Tauri Macro Name Mismatch

**CLAUDE.md line 29** and **MEMORY.md** reference `invoke_handler!` macro.

**Actual code** (`lib.rs` line 272): `.invoke_handler(tauri::generate_handler![...])`. The macro is `generate_handler!`; `invoke_handler` is the builder method. This will confuse anyone trying to add a new command by following the docs.

## P2 — Repo Layout Incomplete

**CLAUDE.md "Repo Layout"** lists 4 crates under `vibeui/crates/`:
```
vibeui/crates/ ← vibe-core, vibe-ai, vibe-lsp, vibe-extensions
```

**Actual**: 5 crates (missing `vibe-collab`). Also missing from the layout: `vibeapp/src-tauri` and `vibe-indexer` which are workspace members.

## P2 — Provider Count Varies Across Docs

| Document | Claimed Count |
|----------|---------------|
| MEMORY.md | 18 |
| docs/architecture.md line 221 | 23 |
| docs/architecture.md table (lines 228-240) | 17 (only lists 17) |
| docs/providers/index.md | 22 |
| docs/faq.md | 22 |
| Actual provider .rs files | 23 (but `openai_compat.rs` is shared utility, so 22 independent providers) |

## P2 — Module Declaration Pattern Mismatch

**CLAUDE.md line 20**: "Both `lib.rs` and `main.rs` must declare new modules. Add `pub mod foo;` to both files."

**Actual**: `lib.rs` has 54 `pub mod`, `main.rs` has 14. There are 222 `.rs` files total. The pattern is not followed consistently — 154 files exist that are not declared in either entry point.

## P2 — Design System Token Source Mismatch

**design-system/README.md line 3**: "All tokens are CSS custom properties defined in `src/App.css`"

**Actual**: Tokens are defined in `vibeui/design-system/tokens.css` (which has the header "Single source of truth for all CSS custom properties"). `App.css` consumes the tokens but does not define them.

## P3 — Design System README Misnumbered Rules

Rules section (lines 42-53) numbering: 1, 2, 3, 4, **7**, 5, 6, 7, 8, 9, 10. Rule 7 appears twice; rules 5-6 come after the first rule 7.

## P3 — Skill File Counts Disagree Across 3 Docs

| Document | Count |
|----------|-------|
| docs/architecture.md | 568 |
| docs/PLUGIN-DEVELOPMENT.md | 526 |
| docs/development.md | 500+ |
| Actual | 599 |

## P3 — Windows Installer URL Assumes Repo Path

`docs/setup.md line 71` references:
```
https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/windows/setup.ps1
```
This URL will 404 if the GitHub repo org/name differs from `TuringWorks/vibecody`.

## P3 — Deploy Scripts Reference Tier Flag

`docs/setup.md` lines 84-86, 153-155 reference `./deploy/aws/setup.sh --tier lite` and `./deploy/gcp/setup.sh --tier max`. The `--tier` concept is not implemented in VibeCLI itself.
