# VibeCody UI v2 — Product Specification

> Project-first, Sandbox-Chat-Centric Redesign
> Version: 2.0-draft
> Date: 2026-05-28
> Owner: Product & Engineering
> Status: Specification Draft

---

## 0. Vision Statement

**VibeCody UI v2 is project-first, not coding-first.** The project — its goals, context, agents, and sandbox — is the center of the experience. Coding is one of the actions the project takes through its agents. The sandbox chat is the primary surface where intent meets execution.

---

## 1. Design Principles

### P1. Project is the center, not the editor

The landing surface is the **Project Hub** — a project health dashboard showing goals, agents, context, and recent activity. The editor is one panel among many, not the gravitational center.

### P2. Sandbox Chat is the primary interaction surface

Every AI interaction starts as a sandbox chat. The user describes intent, the agent executes in a sandbox, and the result is visible as a task card with diff, tests, and branch.

### P3. Progressive disclosure

New users see 3 surfaces: Project Hub, Sandbox Chat, Editor. Power users see 11 tab groups and 308 panels. The ramp is gradual and earned through usage.

### P4. Task > Conversation

A chat session that creates code changes is framed as a **task**, not a conversation. Tasks have status, branches, diffs, costs, and completion actions. Conversations without code changes are notes.

### P5. Every action is transparent

Agent actions appear as **structured tool-use blocks** inside the chat: named, collapsible, timestamped, with cost. No invisible work.

### P6. Cost is always visible

Per-session cost is shown in the chat header. Per-task cost is shown on the task card. Per-day cost is shown on the Project Hub.

---

## 2. Layout Architecture

### 2.1 New Primary Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  VibeCody — [Project Name]                      [Provider ▾]     │
├──────────┬──────────────────────────────┬───────────────────────┤
│          │                              │                         │
│ Project  │  Sandbox Chat (primary)      │  Editor (secondary)    │
│ Hub      │                              │                         │
│          │  ┌─ Session Cards ─────────┐│  ┌─ Tabs ───────────┐ │
│ ○ Goals  │  │ ● Auth bug fix  │running ││  │ auth/mod.rs      │ │
│ ○ Agents │  │ ✓ Add tests     │done    ││  │                  │ │
│ ○ Context│  │ ⏸ Refactor API │paused  ││  │  (inline diff    │ │
│ ○ Health │  └─────────────────────────┘│  │   overlay when   │ │
│ ○ Cost   │                              │  │   agent edits)   │ │
│          │  ┌─ Active Session ─────────┐│  │                  │ │
│          │  │ > fix the auth timeout    ││  └──────────────────┘ │
│          │  │                           ││                         │
│          │  │ ┌─ 📖 Read ────────────┐ ││                         │
│          │  │ │ src/auth/mod.rs (2.3K) │ ││                         │
│          │  │ └────────────────────────┘ ││                         │
│          │  │                              ││                         │
│          │  │ ┌─ ✏️ Edit ───────────────┐ ││                         │
│          │  │ │ src/auth/mod.rs (+12/-3)  │ ││                         │
│          │  │ │ [Accept] [Reject]         │ ││                         │
│          │  │ └───────────────────────────┘ ││                         │
│          │  │                               ││                         │
│          │  │ ┌─ ▶ Run ─────────────────┐ ││                         │
│          │  │ │ cargo test -- auth        │ ││                         │
│          │  │ │ ✓ 3 tests passing        │ ││                         │
│          │  │ └───────────────────────────┘ ││                         │
│          │  │                               ││                         │
│          │  │ > _                            ││                         │
│          │  └───────────────────────────────┘│                         │
│          │                              │  ┌─ Terminal ────────────┐ │
│          │                              │  │ $                      │ │
│          │                              │  └────────────────────────┘ │
├──────────┴──────────────────────────────┴───────────────────────────┤
│  Status Bar: Agent: 2 running │ Sandbox: docker │ Cost: $0.14      │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.2 Layout Zones

| Zone | Purpose | Default Visibility | Toggle |
|------|---------|-------------------|--------|
| Project Hub (left) | Project goals, agents, context, health, cost | Always visible | Cmd+B |
| Sandbox Chat (center) | Primary AI interaction surface | Always visible | Cmd+J |
| Editor (right) | Code viewing and inline editing | Always visible | Cmd+E |
| Terminal (bottom) | Shell access | Collapsed | Cmd+` |
| Tool Panels (overlay) | 308 panels for power users | Hidden until invoked | Cmd+Shift+P |

### 2.3 Resizing

All zones are resizable. Default split: Project Hub 220px | Sandbox Chat flex | Editor 40% | Terminal collapsed. The user can drag to resize any zone. Zones remember their sizes.

---

## 3. Project Hub (Left Sidebar)

### 3.1 Purpose

Replace the current 8-item Activity Bar (Explorer, Search, Git, Testing, Project, Infra, AI, Security) with a unified **Project Hub** that serves as the project's command center.

### 3.2 Sections

```
┌─ Project Hub ────────────────┐
│ 📁 my-project                │
│                               │
│ ── Goals ─────────────────── │
│ ○ Fix auth timeout (active)   │
│ ✓ Add payment tests (done)    │
│ ○ Refactor API (planned)     │
│ [+ New Goal]                  │
│                               │
│ ── Agents ─────────────────── │
│ ● auth-fixer    running  $0.04│
│ ● test-adder    running  $0.02│
│ ⏸ api-refactor  paused   $0.01│
│                               │
│ ── Context ────────────────── │
│ 📄 14 files loaded            │
│ 📋 AGENTS.md detected         │
│ 🔍 Project profile ready      │
│ [Manage Context]              │
│                               │
│ ── Health ─────────────────── │
│ ✅ 847 tests passing           │
│ ⚠️ 3 lint warnings             │
│ 🔄 main → 2 ahead             │
│                               │
│ ── Cost ───────────────────── │
│ Today: $0.47 │ This week: $3.12│
│                               │
│ ── Sessions ───────────────── │
│ ○ Fix auth timeout (2m ago)   │
│ ○ Add tests (completed)       │
│ ○ API discussion (1h ago)      │
│                               │
│ ── Quick Actions ──────────── │
│ [📋 New Task] [🎯 New Goal]   │
│ [🔍 Search Code] [📊 Dashboard]│
│                               │
│ ── Power User ─────────────── │
│ [All Panels ▾]                │
└───────────────────────────────┘
```

### 3.3 Section Details

**Goals** — Pinned goals from `SteeringPanel` + `PinnedGoalBanner`. Click a goal to open its sandbox session. Click "New Goal" to create one (or type `/goal` in chat).

**Agents** — Live view of `AgentOSDashboard`. Shows agent name, status (running/paused/needs-input/done), and cost. Click to jump to that agent's session. This replaces the need to navigate to the AI > Agent OS tab.

**Context** — Summary of `ProjectContextPanel`. Shows file count, AGENTS.md status, and project profile. Click "Manage Context" to open the full context picker.

**Health** — Test results, lint warnings, git status. Pulls from `DashboardPanel` data. Click to open full dashboard.

**Cost** — Per-day and per-week cost from `CostPanel`. Always visible.

**Sessions** — Recent chat sessions from `ChatTabManager`. Click to resume.

**Quick Actions** — One-click entry points: New Task, New Goal, Search Code, Dashboard.

**Power User** — The "All Panels" button opens the full 11-tab-group panel set. This is the progressive disclosure gate. New users don't see it. After 5 sessions, it appears with a "Want more tools?" tooltip.

---

## 4. Sandbox Chat (Center — Primary Surface)

### 4.1 Purpose

The sandbox chat is where intent meets execution. It replaces the current `AIChat` + `SandboxChatPanel` + `ChatTabManager` combination with a unified task-oriented surface.

### 4.2 Session Cards (Top)

Each sandbox session is a **task card** displayed in a horizontal strip at the top of the chat:

```
┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
│ ● Fix auth timeout  │  │ ✓ Add payment tests │  │ ⏸ Refactor API      │
│   running           │  │   done               │  │   paused             │
│   +12/-3 lines      │  │   +47 lines          │  │   needs input        │
│   $0.04  │  docker  │  │   $0.02  │  local    │  │   $0.01  │  cloud  │
│   [View] [Kill]     │  │   [Diff] [PR]        │  │   [Resume] [Cancel] │
└─────────────────────┘  └─────────────────────┘  └─────────────────────┘
```

Card metadata:
- **Title**: Auto-generated from first message (editable)
- **Status**: running / paused / needs-input / done / failed
- **Diff summary**: +N/-M lines changed
- **Cost**: Per-session dollar amount
- **Sandbox type**: docker / cloud / local
- **Branch**: Git branch name (auto-generated)
- **Actions**: View, Diff, Create PR, Pause, Resume, Kill, Fork

### 4.3 Chat Input

The chat input bar is the primary interaction point:

```
┌────────────────────────────────────────────────────────────────┐
│  @ 📎 🎤 │  Fix the auth timeout bug                    │ ▶ Go │
└────────────────────────────────────────────────────────────────┘
```

- **@** triggers context mention dropdown (files, folders, docs, web, symbols)
- **📎** attaches files or images
- **🎤** voice input (existing `useVoiceInput`)
- **▶ Go** submits as a task (starts sandbox execution)
- Plain Enter submits as a chat message (non-sandbox conversation)

The distinction between **Task** (▶ Go) and **Chat** (Enter) is the key innovation. Tasks execute in a sandbox. Chat is conversation-only.

### 4.4 Tool-Use Blocks (In Chat)

Every agent action appears as a **structured block** inside the chat:

```
┌─ 🔍 Read file ──────────────────────────── [12:34:56] ─────────┐
│ src/auth/mod.rs (2.3 KB)                                       │
│ ▸ Shows first 12 lines of context                               │
│                                                                  │
│ [Expand] [Open in Editor]                                        │
└──────────────────────────────────────────────────────────────────┘

┌─ ✏️ Edit file ──────────────────────────── [12:35:02] ─────────┐
│ src/auth/mod.rs (+12/-3)                                       │
│ ▸ Diff preview (collapsible)                                    │
│                                                                  │
│ [Accept] [Reject] [Accept All]  [View Full Diff]                 │
└──────────────────────────────────────────────────────────────────┘

┌─ ▶ Run command ─────────────────────────── [12:35:18] ─────────┐
│ cargo test -- auth                                               │
│ ✓ 3 tests passing                                               │
│                                                                  │
│ [Copy Output] [Open in Terminal]                                 │
└──────────────────────────────────────────────────────────────────┘

┌─ 🔍 Web search ─────────────────────────── [12:35:30] ─────────┐
│ "Rust tokio timeout pattern"                                    │
│ ▸ 3 results                                                      │
│                                                                  │
│ [View Results]                                                   │
└──────────────────────────────────────────────────────────────────┘
```

Each block is:
- **Named**: Read file, Edit file, Run command, Web search, etc.
- **Collapsible**: Click to expand/collapse
- **Timestamped**: When the action occurred
- **Actionable**: Accept/Reject for edits, Open in Editor for reads, Copy for output
- **Cost-annotated**: Token count for each action (optional, shown in settings)

### 4.5 Interactive Canvases

In addition to tool-use blocks, the chat can render structured content:

| Content Type | Rendering |
|-------------|-----------|
| Diff summary | Mini diff viewer with accept/reject per hunk |
| Test results | Pass/fail grid with expandable failures |
| Architecture | Visual diagram (rendered from code or ASCII) |
| Cost breakdown | Bar chart per provider |
| File tree | Tree view of sandbox contents |

### 4.6 Context Badge

At the top of every chat, a context badge shows what's included:

```
┌─ Context: 14 files │ AGENTS.md │ git diff (3 files) │ Project profile ──┐
│ [Manage] [Refresh]                                                                │
└────────────────────────────────────────────────────────────────────────────────────┘
```

Click "Manage" to open the full `ContextPicker`. Click any item to see details.

### 4.7 Cmd+K Inline Edit

In the editor zone, selecting code and pressing Cmd+K opens an inline prompt:

```
┌─ auth/mod.rs ──────────────────────────────────────────────┐
│                                                              │
│  pub async fn authenticate(                                 │
│      credentials: Credentials,                              │
│  ) -> Result<Token, AuthError> {                             │
│      ┌─ Make this function timeout-safe ────────────────┐   │
│      │ [Accept] [Reject]                                 │   │
│      └──────────────────────────────────────────────────┘   │
│  }                                                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

The diff streams in-place over the selected code. Tab accepts, Esc rejects. This is the Cursor Cmd+K pattern, integrated into VibeUI.

---

## 5. Editor Zone (Right — Secondary Surface)

### 5.1 Purpose

The editor zone shows the code that the AI agent is working on. It's not the primary surface — the sandbox chat is. But it's always visible and automatically navigates to files the agent is editing.

### 5.2 Auto-follow Mode

When an agent edits a file, the editor auto-navigates to that file and shows the diff as an inline overlay. The user can:
- Accept all changes (Tab)
- Reject all changes (Esc)
- Review hunk-by-hunk (click each hunk)
- Switch to manual mode (stop auto-follow, browse freely)

### 5.3 Diff Rendering

Small diffs (< 20 lines): inline overlay in the editor, no DiffReviewPanel needed.
Large diffs (> 20 lines): auto-open DiffReviewPanel with hunk-by-hunk review.

---

## 6. Progressive Disclosure Ramp

### 6.1 Level 1: New User (Sessions 0-2)

Surfaces visible:
1. Project Hub (left) — Goals, Agents summary, Cost
2. Sandbox Chat (center) — Task cards + chat input
3. Editor (right) — Auto-follows agent edits

Hidden: All 11 tab groups, terminal, tool panels.

First-run experience:
1. "What are you working on?" prompt in Sandbox Chat
2. Auto-scan project context (visible in context badge)
3. Agent starts with Suggest mode (requires approval for every edit)

### 6.2 Level 2: Explorer (Sessions 3-5)

Surfaces unlocked:
4. Cmd+K inline edit
5. Terminal (Cmd+`)
6. Quick Actions in Project Hub
7. @ mention context system

Prompt: "You've completed a few tasks. Want to see more tools?" with a one-click reveal.

### 6.3 Level 3: Power User (Sessions 6+)

All 11 tab groups available via Cmd+Shift+P (command palette) or "All Panels" button in Project Hub.

But they open as overlays on the sandbox chat, not replacing it. The sandbox chat is always the center.

### 6.4 Level 4: Expert (Manual trigger)

Custom layout modes: classic IDE (current layout), project-first (new default), zen (chat only, no editor).

Available via Settings > Layout.

---

## 7. Sandbox Task Lifecycle

### 7.1 Task States

```
Draft → Queued → Running → Reviewing → Completed
                                    ↘ Failed
                                    ↘ Paused (needs input)
```

- **Draft**: Intent described, not yet submitted
- **Queued**: Submitted, waiting for sandbox allocation
- **Running**: Agent executing in sandbox
- **Reviewing**: Agent completed, waiting for user review
- **Completed**: User approved changes
- **Failed**: Agent could not complete (error shown)
- **Paused**: Agent needs user input (question, approval, clarification)

### 7.2 Task Card Metadata

```typescript
interface TaskCard {
  id: string;
  title: string;           // auto-generated from first message
  status: TaskStatus;
  branch: string;          // auto-generated git branch
  diffSummary: string;    // "+12/-3 in 2 files"
  cost: number;            // in USD
  sandboxType: 'docker' | 'cloud' | 'local';
  provider: string;        // e.g. "anthropic"
  model: string;           // e.g. "claude-sonnet-4-20250514"
  createdAt: Date;
  lastActivity: Date;
  actions: TaskAction[];
}

type TaskAction = 'view' | 'diff' | 'pr' | 'pause' | 'resume' | 'kill' | 'fork';
```

### 7.3 Task Completion Flow

When a task completes:

1. Task card transitions to "Reviewing" state
2. Diff summary appears in the chat
3. Editor auto-navigates to the changed files with inline diff overlay
4. User can: Accept All, Reject All, Review Hunk-by-Hunk, or Continue Iterating
5. If "Accept All": changes are applied, branch is pushed, "Create PR" button appears
6. If "Continue Iterating": user types follow-up, agent continues in same sandbox

---

## 8. Implementation Plan

### Phase 1: Foundation (v2.0)

| # | Item | Component | Effort |
|---|------|-----------|--------|
| 1 | Reframe layout: Project Hub left, Sandbox Chat center, Editor right | `App.tsx` | S |
| 2 | Convert `ChatTabManager` tabs to Task Cards with metadata | `ChatTabManager.tsx`, new `TaskCard.tsx` | M |
| 3 | Add @ mention system to chat input | `AIChat.tsx`, new `MentionDropdown.tsx` | M |
| 4 | Add structured tool-use blocks to chat messages | `AIChat.tsx`, new `ToolUseBlock.tsx` | L |
| 5 | Add context badge at top of chat | `ChatComposite.tsx` | S |
| 6 | Wire Cmd+K inline edit trigger | `App.tsx`, `InlineChat.tsx` | M |
| 7 | Add per-session cost to chat header | `ChatComposite.tsx`, `CostPanel.tsx` | S |
| 8 | Add inline quick-approve for small diffs | `DiffReviewPanel.tsx` | S |
| 9 | Auto-run `scan_project_profile` on workspace open | `App.tsx` | S |
| 10 | Add "New Task" / "New Goal" buttons to Project Hub | `ProjectHubComposite.tsx` | S |

### Phase 2: Task Lifecycle (v2.1)

| # | Item | Component | Effort |
|---|------|-----------|--------|
| 11 | Task state machine (Draft → Queued → Running → Reviewing → Completed) | New `TaskLifecycle.ts` | M |
| 12 | Auto-generate git branch per task | `SandboxChatPanel.tsx`, daemon | M |
| 13 | "Create PR" button on completed tasks | `TaskCard.tsx`, daemon | M |
| 14 | Auto-follow mode in editor (navigate to agent-edited files) | `App.tsx` | M |
| 15 | Agent grid view in Project Hub (replace AgentOSDashboard navigation) | `ProjectHubComposite.tsx` | S |
| 16 | Interactive Canvases in chat (diffs, test results, charts) | `AIChat.tsx`, new `CanvasRenderers.tsx` | L |

### Phase 3: Progressive Disclosure (v2.2)

| # | Item | Component | Effort |
|---|------|-----------|--------|
| 17 | Implement disclosure ramp (3 surfaces → 7 surfaces → full) | `App.tsx`, new `DisclosureManager.ts` | M |
| 18 | First-run experience ("What are you working on?") | New `FirstRun.tsx` | M |
| 19 | "Want more tools?" prompt after 5 sessions | `App.tsx` | S |
| 20 | Layout modes: project-first, classic IDE, zen | New `LayoutMode.tsx` | L |

### Phase 4: Power Features (v2.3)

| # | Item | Component | Effort |
|---|------|-----------|--------|
| 21 | Fork/resume session cards | `ChatTabManager.tsx`, `TaskCard.tsx` | M |
| 22 | Live sandbox view (VNC-like) for cloud sandboxes | `CloudSandboxPanel.tsx` | L |
| 23 | Self-verifying agent loop (wire visual_verify back) | `SandboxChatPanel.tsx`, daemon | L |
| 24 | Browser automation screenshots in chat | `BrowserAgentPanel.tsx`, `AIChat.tsx` | M |
| 25 | Intent bar mode (separate from chat mode) | `ChatComposite.tsx` | M |

---

## 9. Success Metrics

| Metric | Current Baseline | Target (v2.0) | Target (v2.2) |
|--------|-----------------|---------------|---------------|
| Time to first value (new user) | ~4 min (set API key, open workspace, find AI panel) | < 30 sec (open app, type task) | < 15 sec (auto-sandbox) |
| Discoverable features (new user) | ~5 of 308 | 15+ (progressive disclosure) | 25+ (disclosure ramp) |
| Chat-to-code latency | ~8 sec (context switch from editor to chat) | < 2 sec (Cmd+K inline) | < 1 sec (auto-follow) |
| Task completion rate | ~60% (estimated, no tracking) | 75% (sandbox isolation) | 85% (self-verifying loop) |
| Panel navigation clicks (common task) | 3-5 clicks | 1-2 clicks (Project Hub) | 0-1 clicks (auto-follow) |
| Per-session cost visibility | Hidden in CostPanel | In chat header | In task card + header |

---

## 10. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Layout change alienates existing users | High | Classic IDE layout preserved as Level 4 option |
| Progressive disclosure hides powerful features | Medium | "Want more tools?" prompt after 5 sessions; Cmd+Shift+P always available |
| Task card abstraction overloads ChatTabManager | Medium | TaskCard extends ChatTab, doesn't replace it |
| Sandbox-first onboarding requires cloud sandbox | High | Offer local Docker sandbox as default; cloud sandbox as opt-in |
| Cmd+K conflicts with Monaco keybinding | Low | Check existing keybindings; use Cmd+Shift+K as fallback |

---

## 11. Dependencies on Daemon Changes

| Change | Daemon Component | Priority |
|--------|-----------------|----------|
| Auto-branch-per-task | `serve.rs` | Phase 2 |
| Task state machine API | New `/api/tasks` endpoint | Phase 2 |
| Self-verifying agent loop | `visual_verify.rs` → agent loop | Phase 4 |
| Live sandbox VNC | `CloudSandboxPanel` → sandbox API | Phase 4 |
| Auto-scan project profile on workspace open | `scan_project_profile` Tauri command | Phase 1 |

---

## 12. Naming

The next version of VibeUI should be positioned as:

- **Internal**: VibeUI v2
- **Marketing**: VibeCody 2.0 — Project-First AI
- **Key differentiator**: "Your project, your agents, your sandbox — not just a chat sidebar."

The sandbox chat is the hero feature. It's what makes VibeCody different from every other AI IDE. The others have chat sidebars. VibeCody has **task-oriented sandbox sessions** that create branches, run tests, and produce PRs — with the project context always visible.
