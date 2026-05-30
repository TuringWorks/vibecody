# Deep Analysis: Cursor / Antigravity 2.0

> Category: AI-native IDE (VS Code fork) + Agent framework
> Last evaluated: 2026-05-28
> Product versions: Cursor 4.0, Antigravity 1.22+

---

## 1. UI Paradigm: Editor-Native AI With Progressive Disclosure

Cursor is the most commercially successful AI-native IDE. It started as a VS Code fork with Cmd+K inline edits, evolved through a "chat sidebar" era, and has now arrived at what they call **Agent Mode** — a deeply integrated multi-agent system that operates within the editor context. Antigravity 2.0 is Google's answer in the same category: an agent framework layered on top of an IDE.

The defining pattern: **the editor is the center of gravity, and AI is a persistent companion that progressively reveals its capabilities.**

### Core layout (Cursor 4.0)

```
┌──────────┬────────────────────────┬──────────────────┐
│ Activity │  Editor (primary)      │  Agent Panel     │
│ Bar      │                        │                  │
│          │  ┌─ Tab ──────────┐   │  ┌─ Chat ──────┐│
│ ☐ Files  │  │ src/auth/mod.rs│   │  │ > fix the    ││
│ 🔍 Search│  └────────────────┘   │  │   auth bug   ││
│ 󰊢 Git   │                        │  │              ││
│ 🐛 Debug │  Inline diff overlay   │  │ 🔍 Reading.. ││
│          │  (accept/reject per     │  │ ✏️ Editing.. ││
│          │   hunk, in-place)       │  │ ✓ Done       ││
│          │                        │  │              ││
│          │                        │  │ [Context: 5   ││
│          │                        │  │  files]       ││
│          │                        │  └──────────────┘│
│          │                        │                  │
│          │                        │  ┌─ Agent Tabs ─┐│
│          │                        │  │ Chat│Agents│  ││
│          │                        │  └──────────────┘│
└──────────┴────────────────────────┴──────────────────┘
```

### Key UI innovations in Cursor 4.0

1. **Agents Window (new in 3.0+)**. A grid of agent cards, each showing:
   - Agent name and status (idle/running/needs-input)
   - Current task description
   - File being edited (with live diff)
   - Terminal output (streaming)
   - Cost counter

   This is Cursor's answer to "how do you visualize multiple agents?" — and it's the first commercially shipped multi-agent UI in an IDE.

2. **Design Mode (new in 3.0)**. Live DOM annotation: point at a UI element, describe what you want, and the agent edits the code. The preview updates in real-time. This is a direct attack on the "AI generates code that doesn't look right" problem.

3. **Interactive Canvases (new in 3.2)**. Agent responses can render as dashboards, forms, and charts inside the chat panel. Not just text — structured, interactive UIs embedded in the conversation.

4. **Inline diff with Cmd+K**. The original Cursor pattern: select code, press Cmd+K, type what you want, see the diff inline. Accept/reject per hunk. Still the fastest micro-edit flow in any AI IDE.

5. **Multi-root workspaces (3.2)**. Agent can operate across multiple repos simultaneously.

---

## 2. Interaction Model Deep Dive

### Cmd+K inline edit flow

1. User selects code (or places cursor)
2. Presses Cmd+K
3. Types natural language instruction
4. Cursor streams a diff overlay on the selected code
5. Accept (Tab) or reject (Esc) per hunk
6. If rejected, iterate with another instruction

**This is the gold standard for inline AI editing.** No chat panel needed. No context switching. The edit happens where the code is.

### Agent Mode flow

1. Open Agent Panel (Cmd+L → switch to Agent)
2. Describe task
3. Agent reads files, runs commands, edits files — all shown as tool-use cards in the chat
4. Each file edit shows as an inline diff in the editor (in addition to the chat card)
5. User can accept/reject each edit independently
6. Agent continues to next step
7. Multiple agents can run in parallel (visible in Agents Window)

### Context management

Cursor's context strategy is **semi-automatic**:
- `@file` to include a specific file
- `@folder` to include a folder
- `@web` to search the web
- `@docs` to include documentation
- `@git` to include recent git changes
- Auto-include: recent edits, open files, terminal output

The `@` mention system is the most discoverable context management UI in any AI IDE. It's the same pattern as Slack mentions — developers already know how to use it.

### Agent tabs and grid view

Cursor 4.0 introduced a grid view for agents:
```
┌─────────────┬─────────────┐
│  Agent 1    │  Agent 2    │
│  "Fix auth" │  "Add test" │
│  ● Running  │  ● Running  │
│  +12/-3     │  +47 lines   │
│  $0.04      │  $0.02       │
├─────────────┼─────────────┤
│  Agent 3    │  Agent 4    │
│  "Refactor" │  ⏸ Paused   │
│  ✓ Done     │  Needs input │
│  +8/-2      │             │
│  $0.01      │             │
└─────────────┴─────────────┘
```

This is a significant UX advance — it makes parallel agent work visible and manageable.

---

## 3. Antigravity 2.0 Analysis

Antigravity is Google's agent framework. Its UI layer (when used with an IDE) looks different from Cursor:

### Core layout (Antigravity + IDE)

```
┌──────────────────────────────────────────────────┐
│  Agent Status Bar (top)                           │
│  ● Agent running  │  Step 3/7  │  +23/-8  │ $0.12│
├──────────┬───────────────────────┬───────────────┤
│          │  Editor               │  Agent Log     │
│ Files    │                       │                │
│          │  (edits appear as     │  ▸ Read: auth  │
│          │   inline diffs)       │  ▸ Edit: auth  │
│          │                       │  ▸ Run: tests  │
│          │                       │  ▸ Read: test   │
│          │                       │  ✓ Complete    │
│          │                       │                │
├──────────┴───────────────────────┴───────────────┤
│  Terminal (agent commands stream here)           │
└──────────────────────────────────────────────────┘
```

**Antigravity's differentiators from Cursor:**
- **AGENTS.md fallback**: Reads `.cursor/rules`, `CLAUDE.md`, or its own `AGENTS.md`. Multi-format project instruction discovery.
- **Linux sandboxing**: Optional sandbox for agent operations (seatbelt-style). More isolation than Cursor, less than Codex.
- **MCP auth**: Native support for Model Context Protocol with authentication. Can connect to external tool servers.
- **Design focus on verification**: Agent runs tests after edits and self-corrects. The "verify then report" loop is more explicit than Cursor's.

---

## 4. Design Philosophy Extraction

### What Cursor/Antigravity gets right

| Pattern | Why it works |
|---------|-------------|
| **Editor-native AI** | The code IS the context. No copy-paste, no "describe your project." The agent sees what you see. |
| **Cmd+K inline edit** | Fastest micro-edit flow in any AI IDE. Zero context switch. |
| **`@` mention context** | Most discoverable context management. Slack-like pattern that developers already know. |
| **Agent grid view** | Makes parallel agents visible and manageable. First commercial multi-agent UI. |
| **Interactive Canvases** | Chat responses render as dashboards/forms/charts, not just text. Rich output inside the conversation. |
| **Progressive disclosure** | Start with Cmd+K, graduate to Agent Mode. The user never feels overwhelmed. |
| **Inline diff overlay** | Edits appear where the code is, not in a separate panel. Spatial memory preserved. |

### What Cursor/Antigravity gets wrong

| Gap | Impact |
|-----|--------|
| **VS Code fork dependency** | Cursor is a fork of VS Code. It cannot ship features faster than VS Code changes its internal APIs. This creates a perpetual merge burden. |
| **Closed source** | No community extension ecosystem for AI capabilities. You get what Cursor ships. |
| **Single-app focus** | No mobile, no watch, no CLI. You're in Cursor or you're not. VibeCody's multi-surface strategy is unmatched here. |
| **No project dashboard** | Cursor has no "project health" view. No test results, no dependency audit, no architecture map. The file tree is the project. |
| **No sandbox isolation** | Like Claude Code, Cursor agents run on the host. Docker/seatbelt isolation is not the default. |
| **Chat panel is second-class** | Despite Agent Mode, the chat panel still feels like a "sidebar" — not the primary surface. The editor is primary. |
| **Cost opacity** | Token usage is shown but not prominent. No per-session cost breakdown. |
| **No session persistence** | Agent sessions don't persist across restarts. Close Cursor, lose context. |

---

## 5. Transferable Patterns for VibeCody

### Pattern: Cmd+K inline edit (highest priority)

Cursor's Cmd+K is the most-copied pattern in AI IDEs for a reason. VibeCody should implement:

1. Select code in Monaco editor
2. Press Cmd+K (or Ctrl+K)
3. Inline prompt bar appears over the selection
4. Type instruction
5. Diff streams in-place over the selected code
6. Tab to accept, Esc to reject, iterate

This is VibeCody's biggest single UX gap versus Cursor. The `InlineChat` component exists in the component list but needs the Cmd+K trigger pattern.

### Pattern: @-mention context system

VibeCody has `ContextPicker` but it's a separate UI panel. Cursor's `@` mention system is inline — type `@` in the chat and get a dropdown of files, folders, docs, web results.

**Recommendation:** Add `@` mention support to `AIChat.tsx` input:
- `@file` → search files in workspace
- `@folder` → include entire folder
- `@git` → include recent git changes
- `@docs` → include project docs
- `@web` → web search
- `@symbol` → include a specific function/class

### Pattern: Agent grid view

VibeCody's `AgentOSDashboard` and `AgentTeamsPanel` exist but are separate panels. Cursor's grid view puts all running agents in a single glanceable surface.

**Recommendation:** Create an `AgentGrid` view that shows:
- Agent name + status (idle/running/paused/needs-input)
- Current task (one-line description)
- Files changed (+/- lines)
- Cost counter
- One-click: View, Pause, Resume, Kill

This should be the landing page of the "AI" tab group, not buried in a sub-panel.

### Pattern: Interactive Canvases in chat

Cursor's Interactive Canvases render structured UI inside the chat. VibeCody's `McpAppEmbed` component already does this for MCP Apps. The pattern should extend to:

- Diff summaries (rendered as a mini diff viewer, not raw text)
- Test results (rendered as a pass/fail grid, not JSON)
- Architecture diagrams (rendered as a visual, not ASCII)
- Cost breakdowns (rendered as a bar chart, not numbers)

### Pattern: Progressive disclosure

Cursor's most subtle strength: you start with Cmd+K (simple), discover Agent Mode (medium), then find Agents Window (advanced). Each level adds capability without overwhelming.

VibeCody currently shows all 11 tab groups and 40+ sub-tabs on first launch. This is the opposite of progressive disclosure.

**Recommendation:** Implement a disclosure ramp:
1. First launch: sandbox chat + file tree (2 surfaces)
2. After 3 sessions: show Agent Mode prompt (3 surfaces)
3. After 5 sessions: reveal full panel tab groups
4. Power users: full 11-group, 40+ tab interface

---

## 6. Cursor/Antigravity vs VibeCody UI: Structural Comparison

| Dimension | Cursor/Antigravity | VibeCody UI |
|-----------|---------------------|-------------|
| Entry point | Cmd+K or Agent Panel | File editor + AI panel |
| Primary artifact | Inline diff in editor | Chat message + DiffReviewPanel |
| Project visibility | File tree (standard VS Code) | File tree + ProjectContext + Dashboard |
| Sandbox model | None (host-native) | Local Docker + cloud gateway |
| Model selection | Anthropic + OpenAI + Google | 23 providers |
| Chat paradigm | Sidebar chat with agent cards | Right-panel chat with tabs |
| Context control | `@` mentions (inline) | ContextPicker (separate panel) |
| Completion format | Inline diff overlay | DiffReviewPanel (modal) + inline |
| Multi-agent UI | Agents Window (grid) | AgentOSDashboard (separate panel) |
| Settings surface | Settings.json (VS Code style) | 308 panels across 11 tab groups |
| Mobile/watch | None | Full 5-device matrix |
| Inline edit | Cmd+K (gold standard) | InlineChat (needs Cmd+K trigger) |

**The fundamental divergence:** Cursor optimizes for **editor-native, progressively-disclosed AI**. VibeCody optimizes for **panel-rich, project-aware, multi-surface AI**. Cursor's progressive disclosure is a better first-time experience; VibeCody's breadth is a better power-user experience.

---

## 7. Summary: What VibeCody Should Take From Cursor/Antigravity

1. **Cmd+K inline edit.** The single highest-impact UX pattern to adopt. Inline prompt over selection, streaming diff overlay, Tab/Esc to accept/reject.
2. **`@` mention context system.** Inline context selection in the chat input. Replace the separate ContextPicker for common use cases.
3. **Agent grid as landing surface.** Show running agents at a glance, not buried in a sub-panel.
4. **Interactive Canvases in chat.** Render diffs, test results, and charts as structured UI inside the chat stream.
5. **Progressive disclosure ramp.** Start new users with 2 surfaces, graduate to full power. The 11-tab-group, 308-panel interface is overwhelming on day 1.
6. **Inline diff overlay for small edits.** Don't force every edit through the full DiffReviewPanel. Small edits should stream inline in the editor.
