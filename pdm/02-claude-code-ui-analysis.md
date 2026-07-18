# Deep Analysis: Claude Code UI

> Category: Terminal-first AI coding agent with IDE extensions
> Last evaluated: 2026-05-28
> Product version: Claude Code 2.x (CLI + VS Code + JetBrains)

---

## 1. UI Paradigm: Conversation-Native, Terminal-First

Claude Code's UI philosophy is the inverse of Codex's: the **conversation IS the interface**. There is no separate task card, no sandbox panel, no branch viewer. The REPL is the product. Every action — file edit, shell command, web search, git operation — is mediated through natural language inside a terminal session.

### Core layout (CLI)

```
┌─────────────────────────────────────────────────┐
│  $ claude                                        │
│                                                  │
│  > add error handling to the auth module         │
│                                                  │
│  I'll read the auth module first.                │
│                                                  │
│  ● Reading src/auth/mod.rs                       │
│  ● Reading src/auth/middleware.rs                 │
│  ● Editing src/auth/mod.rs (+12/-3)              │
│  ● Editing src/auth/middleware.rs (+8/-1)         │
│                                                  │
│  Done. Here's what changed:                      │
│  - Added Result<> return types                   │
│  - Added custom AuthError enum                   │
│  - Wrapped fallible operations in map_err        │
│                                                  │
│  > now add tests for it                          │
│  ● Reading src/auth/mod.rs                       │
│  ● Creating tests/auth_test.rs (+47 lines)       │
│  ● Running cargo test -- auth                    │
│  ✓ 3 tests passing                               │
│                                                  │
│  >                                               │
└─────────────────────────────────────────────────┘
```

### Core layout (VS Code extension)

```
┌──────────┬────────────────────────┬──────────────┐
│ Explorer │  Editor (code)         │ Claude Chat   │
│          │                        │              │
│          │  ● diff inline         │ > add error  │
│          │  ● accept/reject       │   handling   │
│          │  ● suggestions in-line │              │
│          │                        │ ● Reading..  │
│          │                        │ ● Editing..  │
│          │                        │ ✓ Done       │
│          │                        │              │
│          │                        │ > _          │
└──────────┴────────────────────────┴──────────────┘
```

---

## 2. Interaction Model Deep Dive

### Conversation as the only surface

Claude Code has one surface: the chat input. Everything flows through it:
- File edits are shown as streaming diffs inside the conversation
- Shell commands appear as tool-use blocks
- Git operations are tool calls, not separate UI
- Web searches are inline citations
- Computer use (screenshots, clicks) are embedded image blocks

**This is radically simple.** There is no toolbar, no panel picker, no sidebar with 11 tab groups. The cost: discoverability. You can only do what you know how to ask for.

### Approval model

Three tiers: Suggest → Auto-edit → Full-auto. In Suggest mode, every edit requires explicit confirmation. In Full-auto, the agent runs to completion. The UI for approval is inline — a diff hunk with `y/n/a` keys.

### Context management

Claude Code's context strategy is **aggressive auto-inclusion**:
- Automatically reads `CLAUDE.md` (project instructions)
- Scans recent git changes
- Reads files mentioned in the prompt
- Uses `/compact` to compress context when the conversation gets long
- `/resume` to pick up where you left off

The user never manually selects context — Claude decides what to read. This is the polar opposite of VibeCody's `ContextPicker` approach.

### Session persistence

- `/resume` picks up a previous session from SQLite
- `/fork` creates a branch of the conversation
- Sessions are named and searchable
- This is one of Claude Code's strongest features — long-running agent sessions that survive restarts

### Slash commands

`/compact`, `/resume`, `/fork`, `/review`, `/cost`, `/memory`, `/model`, `/clear`, `/help`. These are discoverable through `/help` but invisible until you learn them. No GUI affordance for any of them.

---

## 3. Design Philosophy Extraction

### What Claude Code gets right

| Pattern | Why it works |
|---------|-------------|
| **Conversation as unified surface** | Zero cognitive switching cost. Everything happens in one flow. No panel hunting. |
| **Streaming tool-use blocks** | Each action (read, edit, run) is a named, timestamped block in the chat. Auditable, skimmable, copyable. |
| **Session persistence + fork** | `/resume` and `/fork` make long agent sessions practical. You can branch a conversation like you branch code. |
| **Auto-context injection** | `CLAUDE.md` + git-aware context means the agent already knows your project without you telling it. |
| **Minimal approval UI** | Inline `y/n/a` is fast. No modal, no separate panel. The conversation keeps flowing. |
| **Cost visibility** | `/cost` shows token usage in real-time. Developers care about spend — making it visible builds trust. |

### What Claude Code gets wrong

| Gap | Impact |
|-----|--------|
| **No project-level visibility** | You cannot see project health, test status, or architecture without asking Claude. No dashboard, no file-tree integration with AI annotations. |
| **Discoverability crisis** | 308 VibeCody panels vs. ~12 slash commands. Claude Code's features are invisible until you learn the incantation. |
| **No sandbox isolation** | Claude Code runs on your local machine with full access. No Firecracker, no Docker, no seatbelt-by-default. Risk is mitigated by approval tiers, not isolation. |
| **Single-provider lock-in** | Claude Code is Anthropic-only. No model selector. No provider comparison. This is by design (deep integration with Claude's capabilities) but limits flexibility. |
| **No multi-agent visualization** | When Claude Code spawns sub-agents (it can), there's no UI for monitoring them. You see the final result, not the process. |
| **No rich diff review** | Diffs are inline text in the terminal. No side-by-side, no syntax highlighting, no hunk-by-hunk accept/reject with a proper diff viewer. |
| **Mobile/watch gap** | No mobile or watch client. VibeCody's 5-device matrix has no analog. |

---

## 4. The CLAUDE.md Pattern

Claude Code popularized the `CLAUDE.md` / `AGENTS.md` convention that VibeCody adopted:

```markdown
# Project Rules
- Use TypeScript strict mode
- Test with vitest
- Never commit directly to main
```

This file is read automatically at session start. It's a low-friction way to inject project conventions without a settings UI.

**VibeCody already has this** (`AGENTS.md` at repo root). The difference: Claude Code treats it as the primary configuration surface. VibeCody treats it as one input among many (ProfileStore, workspace settings, panel settings, config.toml).

**Implication for VibeCody:** Consider making `AGENTS.md` more prominent in the VibeCoder onboarding flow. When a user opens a project for the first time, the "project steering" panel should highlight the steering file and offer to scaffold one.

---

## 5. Transferable Patterns for VibeCody

### Pattern: Streaming tool-use blocks inside chat

Claude Code's most transferable UI pattern is the **tool-use block**: a named, collapsible section within the chat that shows exactly what the agent did.

```
┌─ 🔍 Read file ─────────────────────┐
│ src/auth/mod.rs (2.3 KB)            │
│ ▸ 12 lines of context                │
└──────────────────────────────────────┘

┌─ ✏️ Edit file ───────────────────────┐
│ src/auth/mod.rs (+12/-3)             │
│ ▸ diff preview                       │
│   [Accept] [Reject] [Accept All]     │
└──────────────────────────────────────┘

┌─ ▶ Run command ─────────────────────┐
│ cargo test -- auth                   │
│ ✓ 3 tests passing                    │
└──────────────────────────────────────┘
```

VibeCody's `AIChat.tsx` already streams messages but doesn't have the structured tool-use block pattern. This would be the single highest-impact UI upgrade for the chat experience.

### Pattern: Session as first-class object

Claude Code's `/resume` and `/fork` make sessions persistent, named, and branchable. VibeCody has `ChatTabManager` with tab-based sessions, but they feel like browser tabs rather than git branches.

**Evolution path:** Make each chat tab carry a "session card" with:
- Auto-generated title (from first message)
- Branch name (if sandbox session)
- Token count and cost
- Status (active / paused / completed)
- Last activity timestamp

### Pattern: Zero-setup auto-context

Claude Code's `CLAUDE.md` + git-scan + auto-read context means the user never has to configure context. VibeCody's `ProjectContextPanel` and `scan_project_profile` do this, but they're hidden in the "Project" sidebar tab.

**Fix:** Auto-run `scan_project_profile` on workspace open. Inject the summary into every new chat tab's system prompt. Show a small "Context: 14 files loaded" badge at the top of the chat.

### Pattern: Inline approval, not modal approval

Claude Code's `y/n/a` inline approval is faster than VibeCody's `DiffReviewPanel` modal. For quick edits, inline is better. For large multi-file changes, the full diff panel is better.

**Recommendation:** Add a "quick approve" inline affordance for single-file edits (< 20 lines changed), and reserve the full `DiffReviewPanel` for multi-file changes or diffs > 50 lines.

---

## 6. Claude Code vs VibeCody UI: Structural Comparison

| Dimension | Claude Code | VibeCody UI |
|-----------|-------------|-------------|
| Entry point | Terminal prompt | File editor + AI panel |
| Primary artifact | Conversation turn | Chat tab + messages |
| Project visibility | None (ask agent) | Full file tree + context panel |
| Sandbox model | None (runs on host) | Local Docker + cloud gateway |
| Model selection | None (Anthropic only) | Full provider dropdown (23 providers) |
| Chat paradigm | Single linear thread | Multi-tab with sandbox tabs |
| Context control | Auto-inferred (CLAUDE.md) | Manual + auto (ProjectContextPanel) |
| Completion format | Inline diff in chat | DiffReviewPanel (modal) |
| Settings surface | Slash commands (~12) | 308 panels across 11 groups |
| Session management | /resume, /fork | ChatTabManager |
| Approval model | y/n/a inline | DiffReviewPanel modal |
| Mobile/watch | None | Full 5-device matrix |

**The fundamental divergence:** Claude Code optimizes for the **power user who lives in the terminal**. VibeCody optimizes for the **developer who wants a full IDE with AI deeply integrated**. The conversation-only model is elegant but limited — VibeCody should adopt Claude Code's streaming tool-use blocks and session cards while keeping its project-awareness advantage.

---

## 7. Summary: What VibeCody Should Take From Claude Code

1. **Structured tool-use blocks in chat.** Each agent action (read, edit, run, search) should be a named, collapsible, timestamped block inside the chat stream. This is the #1 UX upgrade for the AI chat experience.
2. **Session as persistent object.** Upgrade `ChatTabManager` tabs to "session cards" with auto-title, branch, cost, and status.
3. **Auto-context injection visible to user.** Show "Context: N files loaded" badge. Let the user click to see what was included. Keep the manual picker for power users.
4. **Inline quick-approve for small diffs.** Single-file edits under 20 lines should get inline accept/reject without opening the full DiffReviewPanel.
5. **Slash commands as discoverable actions.** Add a `/` command palette inside the chat input (like GitHub Copilot Chat does). Don't hide features behind keyboard shortcuts.
6. **Cost visibility in the chat header.** Show token count and estimated cost for each session. Not buried in a settings panel — right where the user is working.
