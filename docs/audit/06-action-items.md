# 06 — Prioritized Action Items

> Remediation plan organized by severity. Each item references the audit file where it was discovered.

---

## P0 — Critical (fix immediately)

### P0-1: Quickstart instructs users to store API keys in plaintext
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `docs/quickstart.md` lines 304-307
- **Action**: Remove `api_key = "sk-ant-..."` config.toml example. Replace with instructions to use `vibecli` key setup flow or ProfileStore.
- **Verify**: Grep codebase for any code that reads API keys from config.toml

### P0-2: Verify `api_keys.json` migration code is write-only
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **Files**: `commands.rs`, `SettingsPanel.tsx`
- **Action**: Confirm these references are migration/cleanup code, not active reads. Remove if dead code.

---

## P1 — High (fix this sprint)

### P1-1: Remove or implement 4 documented CLI commands
- **Source**: [02-missing-features.md](02-missing-features.md)
- **Commands**: `vibecli service`, `vibecli setup`, `vibecli doctor`, `vibecli config set tier`
- **Action**: Either implement them or remove from docs/setup.md and docs/glossary.md
- **Recommendation**: `vibecli doctor` is high-value; implement it. Remove `service`, `setup wizard`, and `tier` from docs.

### P1-2: Remove or implement `--api-token` CLI flag
- **Source**: [02-missing-features.md](02-missing-features.md)
- **File**: `docs/api-reference.md` lines 51-54
- **Action**: Either add the flag to serve.rs CLI args or remove from docs

### P1-3: Add honest status labels to FIT-GAP stub modules
- **Source**: [05-fitgap-overstatements.md](05-fitgap-overstatements.md)
- **Modules**: 12+ modules (web_grounding, mcp_streamable, a2a_protocol, worktree_pool, proactive_agent, issue_triage, native_connectors, langgraph_bridge, voice_local, mcts_repair, sketch_canvas, cost_router)
- **Action**: In each FIT-GAP doc, change status from "Implemented" to "Typed/Designed — awaiting I/O layer". Or add a status column: `Designed | Partial | Functional`
- **Also**: Update ROADMAP-v5.md to distinguish `[x] designed` from `[x] functional`

### P1-4: Fix RL-OS documentation to reflect actual capabilities
- **Source**: [05-fitgap-overstatements.md](05-fitgap-overstatements.md)
- **File**: `docs/FIT-GAP-RL-OS.md`
- **Action**: Add prominent disclaimer that RL-OS is a data modeling / type system layer. Remove claims about GPU/TPU, Python bindings, and neural network training.

### P1-5: Fix broken SVG reference in chat-workflow.md
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `docs/chat-workflow.md` line 9
- **Action**: Change `![Chat Workflow](/chat-workflow.svg)` to `![Chat Workflow]({{ '/chat-workflow.svg' | relative_url }})`

---

## P2 — Medium (fix this month)

### P2-1: Update all stale counts across docs
- **Source**: [01-stale-counts.md](01-stale-counts.md)
- **Files**: CLAUDE.md, MEMORY.md, docs/architecture.md, docs/vibeui.md, docs/development.md, docs/PLUGIN-DEVELOPMENT.md
- **Action**: Replace hardcoded counts with current values. Consider using a script to auto-generate counts.

### P2-2: Fix Tauri macro name in CLAUDE.md
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: CLAUDE.md line 29
- **Action**: Change `invoke_handler!` to `generate_handler!` and clarify `invoke_handler` is the builder method

### P2-3: Add missing crate to repo layout
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: CLAUDE.md "Repo Layout"
- **Action**: Add `vibe-collab` to crates list. Add `vibeapp/src-tauri` and `vibe-indexer` as workspace members.

### P2-4: Update PANEL-AUDIT.md
- **Source**: [02-missing-features.md](02-missing-features.md), [03-undocumented-code.md](03-undocumented-code.md)
- **File**: `docs/PANEL-AUDIT.md`
- **Action**: Remove 4 phantom panel entries (ComparePanel, FlowPanel, KeysPanel, ModelManagerPanel). Fix 5 naming mismatches. Add 48 undocumented panels.

### P2-5: Document undocumented REPL commands
- **Source**: [03-undocumented-code.md](03-undocumented-code.md)
- **File**: `docs/vibecli.md`
- **Action**: Add entries for the 11 undocumented commands and update the remaining ~90 commands not covered in the doc.

### P2-6: Fix design system token source reference
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `vibeui/design-system/README.md` line 3
- **Action**: Change "defined in `src/App.css`" to "defined in `design-system/tokens.css`"

### P2-7: Fix design system README rule numbering
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `vibeui/design-system/README.md` lines 42-53
- **Action**: Renumber rules sequentially (1-10)

### P2-8: Fix provider count inconsistencies
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **Files**: docs/architecture.md, MEMORY.md
- **Action**: Standardize on 22 providers across all docs

### P2-9: Remove ghost Tauri command registrations
- **Source**: [03-undocumented-code.md](03-undocumented-code.md)
- **File**: `vibeui/src-tauri/src/lib.rs`
- **Action**: Remove `inline_edit` and `record_purple_team_simulation` from generate_handler if functions don't exist, or implement them

### P2-10: Fix semantic_index.rs claim about AST parsing
- **Source**: [05-fitgap-overstatements.md](05-fitgap-overstatements.md)
- **File**: `docs/FIT-GAP-ANALYSIS-v7.md` Gap 5
- **Action**: Change "AST-level" to "regex-based declaration scanning" until tree-sitter integration is added

---

## P3 — Low (backlog)

### P3-1: Document all 11 React hooks
- **Source**: [03-undocumented-code.md](03-undocumented-code.md)
- **Action**: Add hook API docs to design system or a dedicated hooks reference page

### P3-2: Document 5 utility modules
- **Source**: [03-undocumented-code.md](03-undocumented-code.md)
- **Action**: Add docs for DocsResolver, fileUtils, FlowContext, LinterIntegration, SupercompleteEngine

### P3-3: Fix Windows installer URL
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `docs/setup.md` line 71
- **Action**: Parameterize the GitHub URL or verify it matches actual repo path

### P3-4: Reconcile skill file counts
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **Action**: Pick one number (599) and update architecture.md, PLUGIN-DEVELOPMENT.md, development.md

### P3-5: Fix deploy script tier references
- **Source**: [04-doc-inconsistencies.md](04-doc-inconsistencies.md)
- **File**: `docs/setup.md` lines 84-86, 153-155
- **Action**: Remove `--tier` references or implement tier concept

### P3-6: ~~Add missing Gemini Native provider or remove from docs~~ RESOLVED
- **Source**: [02-missing-features.md](02-missing-features.md)
- **Action**: Removed "Gemini native" references — `gemini.rs` is the single Gemini provider (no separate `gemini_native.rs` exists)

---

## Summary

| Priority | Count | Effort Estimate |
|----------|-------|-----------------|
| P0 (Critical) | 2 | < 1 hour |
| P1 (High) | 5 | 2-4 hours |
| P2 (Medium) | 10 | 4-8 hours |
| P3 (Low) | 6 | 2-4 hours |
| **Total** | **23** | **8-17 hours** |
