# VibeCody UI Current-State Assessment

> Evaluation date: 2026-05-28
> Codebase: VibeUI (Tauri 2 + React/TS + Monaco)
> Panels: 308 components across 41 composites
> AI panel: 11 tab groups, 40+ sub-tabs

---

## 1. Architecture Overview

VibeUI is structured as a **classic IDE layout** with an AI panel bolted on the right:

```
┌──────────────────────────────────────────────────────────────────┐
│  Menu Bar (File, Edit, View, AI, Project, Help)                  │
├──────┬──────────────────────────────┬──────────────┬────────────┤
│      │                              │              │            │
│ Act. │  Editor (Monaco)             │  Resizer     │  AI Panel  │
│ Bar  │  ┌─ Tabs ──────────────────┐  │              │            │
│      │  │ file1.rs │ file2.ts │   │  │              │  ┌─ Group ┐│
│ ☐    │  └─────────────────────────┘  │              │  │ Chat   ││
│ 🔍   │                               │              │  │ Agent  ││
│ 󰊢    │  (code editing area)          │              │  │ Sandbox││
│ 🐛   │                               │              │  │ ...    ││
│      │                               │              │  └────────┘│
│      │                               │              │            │
│      ├───────────────────────────────┤              │            │
│      │  Terminal / Browser            │              │            │
└──────┴───────────────────────────────┴──────────────┴────────────┘
```

The AI panel uses `GroupedTabBar` with 11 tab groups:
- AI, Project, Code Quality, Source Control, Infrastructure, Data & APIs, Developer Tools, Toolkit, Settings, Company, Agent Intelligence

Each group contains 3-5 sub-tabs, each of which loads a "composite" panel made of 2-7 lazy-loaded sub-panels.

**Total surface area:** 308 panel components, 41 composites, ~9 visible at any given time (editor, terminal, AI chat, sidebar).

---

## 2. Strengths (What VibeCody UI Gets Right)

### 2.1 Breadth of capability

No other tool offers 23 AI providers, triple-protocol support (MCP + ACP + A2A), 5 device surfaces (desktop + mobile + watch), sandbox isolation, and 308 panels. The breadth is unmatched.

### 2.2 Project awareness

`ProjectContextPanel` auto-scans the workspace, builds a project profile (languages, frameworks, build commands, key files), and injects it into AI conversations. This is better than Cursor's auto-context (which is invisible) and better than Claude Code's `CLAUDE.md` (which is manual).

### 2.3 Sandbox infrastructure

`SandboxChatPanel`, `CloudSandboxPanel`, `GatewaySandboxPanel` — three sandbox backends. No other IDE offers local Docker + cloud gateway sandbox options. Codex has cloud-only. Others have none.

### 2.4 Multi-agent visibility

`AgentOSDashboard`, `AgentTeamsPanel`, `BranchAgentPanel`, `WorktreePoolPanel` — the infrastructure for parallel agents exists and is wired. Cursor only shipped its Agents Window in 3.0; VibeCody had this first.

### 2.5 Design system discipline

`vibeui/design-system/` with tokens, component specs, and 10 hard rules. This is more disciplined than any competitor's UI codebase.

### 2.6 Cost tracking

`CostPanel` tracks token usage and cost per provider. Only Claude Code (`/cost`) and Cline match this.

### 2.7 Device matrix

5-device surface (desktop + mobile + 2 watches + web) is unmatched. No competitor has a watch client at all.

---

## 3. Weaknesses (What VibeCody UI Gets Wrong)

### 3.1 The Editor-First Problem

**Current layout:** Editor (center) + Sidebar (left) + AI Panel (right)

The editor is the gravitational center. The AI panel is a sidebar. This communicates: "you are editing code, and AI is an accessory."

For a project-first, sandbox-chat-centric product, the layout should communicate: "you are having a conversation with an AI about your project, and the editor is one of the tools the AI uses."

**The fix is not just rearranging panels.** It's a fundamental reordering of what the primary surface is.

### 3.2 The 308-Panel Discoverability Crisis

308 panel components across 41 composites across 11 tab groups. This is:
- 4x more panels than Cursor
- 10x more panels than Claude Code
- 20x more panels than Codex App

The result: most panels are invisible to new users. The discoverability curve is a cliff, not a ramp.

**Cursor's progressive disclosure is the model:** Start with 2 surfaces (Cmd+K + chat), graduate to Agent Mode, then reveal the full panel set. VibeCody should adopt the same ramp.

### 3.3 Chat Is a Tab, Not the Center

`AIChat.tsx` is loaded as a tab inside `ChatComposite`, which is loaded as the first tab inside the AI panel, which is a sidebar on the right.

For sandbox-chat-centric UI, the chat should be the **primary surface** — not a tab inside a panel inside a sidebar.

### 3.4 No Cmd+K Inline Edit

VibeCody has `InlineChat.tsx` but it's not triggered by Cmd+K with the Cursor-style overlay pattern. This is the single most-requested UX pattern in AI IDEs and VibeCody doesn't have it wired.

### 3.5 No @ Mention Context System

`ContextPicker` is a separate panel. Cursor's `@` mention system is inline in the chat input. The friction difference is significant.

### 3.6 Sandbox Chat Is Not Task-Oriented

`SandboxChatPanel` has a system prompt that says "you are an autonomous coding agent" — but the UI treats it as a regular chat with a sandbox file tree. There's no task card, no status, no branch, no completion action. It's "chat with sandbox access" instead of "submit a task to a sandbox."

### 3.7 Session Cards Don't Exist

`ChatTabManager` manages chat tabs like browser tabs. There's no auto-title, no branch association, no cost counter, no status badge, no last-activity timestamp. Every competitor with session persistence (Claude Code, Codex, Devin) shows richer session metadata.

### 3.8 Tool-Use Blocks Are Unstructured

AIChat streams messages but doesn't structure tool-use into named, collapsible blocks. Claude Code's tool-use blocks (Read file, Edit file, Run command) are the gold standard for agent transparency.

### 3.9 No Interactive Canvases in Chat

McpAppEmbed supports MCP Apps, but the chat itself doesn't render structured output (diffs, test results, charts) as interactive UI. It's all plain text or code blocks.

### 3.10 Cost Is Buried

`CostPanel` is a separate panel. Per-session cost should be visible in the chat header, not a click away.

---

## 4. Layout Architecture Assessment

### Current layout (editor-first)

```
Activity Bar → Sidebar → Editor → Terminal → AI Panel (right)
     ↑            ↑         ↑         ↑            ↑
   files       context   code      shell      11 tab groups
               search    editing              308 panels
               git
```

### Proposed layout (project-first, sandbox-chat-centric)

```
Project Hub → Sandbox Chat (center) → Editor (right) → Tool Panels (collapsible)
     ↑              ↑                      ↑                ↑
   projects    task cards              code editing     agents, cost,
   context      streaming log           diff review      git, deploy
   steering     diff review
```

The key change: **Sandbox Chat moves from a tab inside a sidebar to the center of the layout.** The editor stays but becomes secondary — it's where you see the code the AI is working on, not where you manually type.

---

## 5. Component Inventory (What We Have)

| Component | Status | What It Needs |
|-----------|--------|---------------|
| `AIChat.tsx` | Exists | Restructure into project-first layout; add tool-use blocks, @ mentions |
| `SandboxChatPanel.tsx` | Exists | Reframe as task cards with status, branch, cost, completion |
| `ChatTabManager.tsx` | Exists | Upgrade to session cards with metadata |
| `ChatComposite.tsx` | Exists | Make it the primary surface, not a tab inside a sidebar |
| `ProjectContextPanel.tsx` | Exists | Auto-run on workspace open; show context badge in chat |
| `ProjectHubComposite.tsx` | Exists | Make it the landing page (new tab default) |
| `InlineChat.tsx` | Exists | Wire Cmd+K trigger; add streaming diff overlay |
| `DiffReviewPanel.tsx` | Exists | Add inline quick-approve for small diffs |
| `AgentOSDashboard.tsx` | Exists | Reframe as agent grid with live cards |
| `CostPanel.tsx` | Exists | Extract per-session cost into chat header |
| `ContextPicker.tsx` | Exists | Add @ mention mode for inline context selection |
| `DashboardPanel.tsx` | Exists | Elevate to project health landing |
| `CloudSandboxPanel.tsx` | Exists | Add live sandbox view (VNC-like) |
| `BrowserAgentPanel.tsx` | Exists | Integrate browser screenshots into chat flow |

---

## 6. The Core Thesis

VibeCody's UI is **coding-first**: you open an editor, and AI is an accessory on the side. Every competitor with a simpler UI (Codex, Claude Code, Cursor) is also coding-first, but they hide the complexity better.

The opportunity is to be **project-first**: the project (its goals, its context, its agents, its sandbox) is the center, and coding is one of the actions the project takes through its agents.

**Sandbox Chat is the mechanism.** It's the surface where:
- You describe what you want (intent)
- The agent takes action (sandbox execution)
- You see the result (diff, test, deployment)
- You iterate (branch, refine, approve)

Making Sandbox Chat the center of VibeUI — not a tab in a sidebar — is the single most impactful change in the next version.
