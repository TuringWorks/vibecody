# 06 — Prioritized Action Items

> Remediation plan organized by severity. Each item references the audit file where it was discovered.
> 
> **Last updated**: 2026-04-09 — ALL 23 items resolved (20 fixed, 3 no-action-needed).

---

## P0 — Critical (fix immediately)

### P0-1: ~~Quickstart instructs users to store API keys in plaintext~~ RESOLVED
- **Commit**: `dd86237f` — removed plaintext key instructions, added env var + Settings panel guidance + security warning

### P0-2: ~~Verify `api_keys.json` migration code is write-only~~ RESOLVED
- **Commit**: `d031da24` — 3 stale references fixed (2 comments in commands.rs, 1 UI string in SettingsPanel.tsx). No code was actively reading from the deleted file.

---

## P1 — High (fix this sprint)

### P1-1: ~~Remove or implement 4 documented CLI commands~~ RESOLVED
- **Commit**: `638dabc7` — removed `vibecli service`, `vibecli setup` wizard, `vibecli config set tier` from docs. Marked `vibecli doctor` as planned.

### P1-2: ~~Remove or implement `--api-token` CLI flag~~ RESOLVED
- **Commit**: `638dabc7` — removed phantom `--api-token` flag from api-reference.md

### P1-3: ~~Add honest status labels to FIT-GAP stub modules~~ RESOLVED
- **Commit**: `27534118` — added `> **Status**:` notes to 12 gaps in FIT-GAP-v7.md clarifying I/O integration pending

### P1-4: ~~Fix RL-OS documentation to reflect actual capabilities~~ RESOLVED
- **Commit**: `27534118` — added prominent disclaimer to FIT-GAP-RL-OS.md about data modeling vs GPU/Python reality

### P1-5: ~~Fix broken SVG reference in chat-workflow.md~~ RESOLVED
- **Commit**: `2c1f7387` — applied `relative_url` filter to SVG and drawio links

---

## P2 — Medium (fix this month)

### P2-1: ~~Update all stale counts across docs~~ RESOLVED
- **Commit**: `d22dfb0e` — updated Tauri (1,045+), modules (222), panels (235+/39), skills (599), REPL (126), tests (~10,535), providers (22) across CLAUDE.md, architecture.md, vibeui.md, development.md

### P2-2: ~~Fix Tauri macro name in CLAUDE.md~~ RESOLVED
- **Commit**: `d22dfb0e` — changed `invoke_handler!` to `tauri::generate_handler!`

### P2-3: ~~Add missing crate to repo layout~~ RESOLVED
- **Commit**: `d22dfb0e` — added vibe-collab, vibeapp/src-tauri, vibe-indexer to CLAUDE.md

### P2-4: ~~Update PANEL-AUDIT.md~~ RESOLVED
- **Commit**: `f333e29a` — removed 4 phantom panels, fixed 5 naming mismatches, added 48 undocumented panels

### P2-5: ~~Document undocumented REPL commands~~ RESOLVED
- **Commit**: `6b7de1ed` — added ~75 undocumented commands to vibecli.md in 18 logical groups

### P2-6: ~~Fix design system token source reference~~ RESOLVED
- **Commit**: `2c1f7387` — changed `src/App.css` to `design-system/tokens.css`

### P2-7: ~~Fix design system README rule numbering~~ RESOLVED
- **Commit**: `2c1f7387` — renumbered rules 1-11 sequentially

### P2-8: ~~Fix provider count inconsistencies~~ RESOLVED
- **Commit**: `d22dfb0e` — standardized on 22 providers, added 5 missing to architecture table

### P2-9: ~~Remove ghost Tauri command registrations~~ RESOLVED
- `inline_edit` was removed entirely on 2026-04-26 along with the rest of the Path A / Path C inline-completion stack (patent-distance work; see `notes/PATENT_AUDIT_INLINE.md`).
- `record_purple_team_simulation` exists in commands.rs (not a ghost).

### P2-10: ~~Fix semantic_index.rs claim about AST parsing~~ RESOLVED
- **Commit**: `27534118` — changed "AST-level" to "regex-based declaration scanning" in FIT-GAP-v7 and ROADMAP-v5

---

## P3 — Low (backlog)

### P3-1: ~~Document all 11 React hooks~~ RESOLVED
- **Commit**: `fcdec08d` — created docs/hooks-reference.md with signatures, params, returns, and examples for all 11 hooks

### P3-2: ~~Document 5 utility modules~~ RESOLVED
- **Commit**: `0ae906e0` — created docs/utils-reference.md covering all 5 utilities with exports and consumer panels

### P3-3: ~~Fix Windows installer URL~~ RESOLVED (no action needed)
- Verified: git remote is `git@github.com:TuringWorks/vibecody.git` — matches the documented URL

### P3-4: ~~Reconcile skill file counts~~ RESOLVED
- **Commit**: `d6c0a631` — updated PLUGIN-DEVELOPMENT.md (526->599). architecture.md and development.md already updated in P2-1.

### P3-5: ~~Fix deploy script tier references~~ RESOLVED
- **Commit**: `638dabc7` — removed `--tier` references from setup.md

### P3-6: ~~Add missing Gemini Native provider or remove from docs~~ RESOLVED
- **Commit**: `d6c0a631` — renamed "Gemini native" to "Gemini" in CHANGELOG and FIT-GAP-v6

---

## Summary

| Priority | Total | Resolved | Open |
|----------|-------|----------|------|
| P0 (Critical) | 2 | 2 | 0 |
| P1 (High) | 5 | 5 | 0 |
| P2 (Medium) | 10 | 10 | 0 |
| P3 (Low) | 6 | 6 | 0 |
| **Total** | **23** | **23** | **0** |

All items resolved.
