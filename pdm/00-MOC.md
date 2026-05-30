# Product Decision Matrix — UI Competitive Analysis

> This is the Map of Content (MOC) for the VibeCody PDM vault.
> All documents are in `pdm/` at the repository root.
> Last updated: 2026-05-28

---

## Purpose

This PDM vault contains deep competitive UI analyses and a product specification for VibeCody UI v2. The central thesis: **move VibeCody from a coding-first IDE with an AI sidebar to a project-first platform with sandbox chat as the primary surface.**

---

## Documents

| # | Document | Focus |
|---|----------|-------|
| 01 | [Codex UI Analysis](01-codex-ui-analysis.md) | OpenAI Codex App & CLI — task-first, sandbox-native paradigm |
| 02 | [Claude Code UI Analysis](02-claude-code-ui-analysis.md) | Claude Code — conversation-native, terminal-first paradigm |
| 03 | [Cursor / Antigravity Analysis](03-cursor-antigravity-analysis.md) | Cursor 4.0 & Antigravity 2.0 — editor-native, progressive disclosure |
| 04 | [Additional Tools Analysis](04-additional-tools-analysis.md) | Windsurf, Augment, Aider, Cline, Devin — cross-pattern synthesis |
| 05 | [VibeCody UI Assessment](05-vibecody-ui-assessment.md) | Current-state assessment of VibeUI — strengths, weaknesses, gaps |
| 06 | [VibeUI v2 Specification](06-vibecody-ui-v2-specification.md) | Product specification for project-first, sandbox-chat-centric redesign |

---

## Cross-Cutting Insights

### The Paradigm Shift

Every tool in this space is converging on the same insight from different angles:

- **Codex**: Task is the unit of work, sandbox is the environment, branch is the output
- **Claude Code**: Conversation IS the interface, session persistence matters, context should be automatic
- **Cursor**: Editor-native AI with progressive disclosure — start simple, graduate to power
- **Devin**: Full VM, self-verifying, PR as deliverable — the autonomous extreme
- **Augment**: Intent-driven, plan-then-execute — the thoughtful extreme
- **Aider**: Git-native, terminal-simple — the minimalist extreme
- **Cline**: Action-by-action approval, cost transparency — the cautious extreme

**No tool is project-first.** They are all either coding-first (Cursor, VibeCody current) or task-first (Codex, Devin). None center the project's goals, context, agents, and health as the primary surface.

**That's VibeCody's opportunity.**

### The Sandbox Chat Opportunity

Sandbox Chat is VibeCody's most unique and underutilized surface. No other tool combines:
- Project context awareness (`ProjectContextPanel`, `AGENTS.md`)
- Sandbox isolation (Docker + cloud gateway)
- Multi-agent visibility (`AgentOSDashboard`, `AgentTeamsPanel`)
- 5-device surface (desktop + mobile + watch)
- 23-provider model selection

The v2 spec reframes Sandbox Chat from "a tab in a sidebar" to "the center of the experience." This is the single highest-leverage change.

### Key Transferable Patterns

| Pattern | Source | VibeCody Implementation |
|---------|--------|------------------------|
| Task card as primary artifact | Codex | Convert `ChatTabManager` tabs to `TaskCard` with status, branch, diff, cost |
| Structured tool-use blocks | Claude Code | Add named, collapsible, timestamped blocks to `AIChat` message stream |
| Cmd+K inline edit | Cursor | Wire `InlineChat.tsx` with Cmd+K trigger and streaming diff overlay |
| @ mention context | Cursor | Add inline `@` dropdown to chat input (files, folders, docs, web, symbols) |
| Progressive disclosure | Cursor | 3 surfaces → 7 surfaces → full panel set over 5 sessions |
| Agent grid view | Cursor | Show running agents at a glance in Project Hub |
| Intent bar mode | Augment | Separate "Task" submission from "Chat" conversation in the input bar |
| Auto-commit per task | Aider | Auto-commit in FullAuto sandbox mode; commit message from agent summary |
| Repo map context | Aider | Compact codebase summary that fits in context, auto-generated from `ProjectContextPanel` |
| Per-action approval | Cline | Offer per-action approval as a mode (not the default) |
| Live sandbox view | Devin | VNC-like view of cloud sandbox in `CloudSandboxPanel` |
| Self-verifying loop | Devin | Wire `visual_verify.rs` output back into agent loop |
| Context pins | Windsurf | Let users pin files/folders that persist across chat sessions |
| Linter feedback loop | Windsurf | Wire linter output into agent loop for immediate validation |
| Interactive Canvases | Cursor | Render diffs, test results, charts as structured UI inside chat |
| Session cards with metadata | Claude Code | Auto-title, branch, cost, status, last-activity on each session tab |

---

## Priority Matrix

The v2 specification is organized in 4 phases. Here's the impact/effort prioritization:

```
                    High Impact
                        │
     Cmd+K inline ─────┼───── Structured tool-use blocks
     @ mentions        │       Task cards with metadata
     Layout reorder    │       Context badge
     Progressive       │       Interactive Canvases
     disclosure        │       Auto-follow in editor
                        │
   Low Effort ─────────┼──────── High Effort
                        │
     Cost in header    │       Live sandbox VNC
     Quick-approve     │       Self-verifying loop
     Auto-scan project │       Intent bar mode
                        │
                    Low Impact
```

**Phase 1 (v2.0) focuses on the left column:** high impact, reasonable effort.
**Phase 2 (v2.1) adds task lifecycle:** the structural change from "chat tabs" to "task cards."
**Phase 3 (v2.2) adds progressive disclosure:** the discoverability improvement.
**Phase 4 (v2.3) adds power features:** the differentiation from competitors.

---

## Competitive Positioning After v2

| Dimension | Codex | Claude Code | Cursor | Devin | VibeCody v1 | VibeCody v2 |
|-----------|-------|-------------|--------|-------|-------------|-------------|
| Primary surface | Task prompt | Terminal chat | Editor + sidebar | Task dashboard | Editor + sidebar | **Project Hub + Sandbox Chat** |
| Project awareness | None | CLAUDE.md | Auto-context | None | Full (but buried) | **Full (primary surface)** |
| Sandbox isolation | Cloud VM | None | None | Cloud VM | Docker + cloud | **Docker + cloud (primary)** |
| Session persistence | No | Yes (SQLite) | No | Yes | Tabs (limited) | **Task cards (full)** |
| Cost visibility | No | /cost command | Partial | Per-task | CostPanel (buried) | **Header + task cards** |
| Multi-agent | Experimental | Limited | Agent Window | No | Full (buried) | **Project Hub (primary)** |
| Mobile/watch | No | No | No | Web only | Full | **Full (unchanged)** |
| Progressive disclosure | High | Low | High | High | Low | **High (new)** |
| Model selection | OpenAI only | Anthropic only | 3-4 providers | OpenAI only | 23 providers | **23 providers (unchanged)** |

**VibeCody v2's unique position:** Project-first, sandbox-chat-centric, multi-agent, multi-provider, multi-device. No other tool occupies this position.
