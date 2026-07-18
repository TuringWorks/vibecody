# Deep Analysis: OpenAI Codex UI

> Category: Cloud-hosted AI coding agent + CLI
> Last evaluated: 2026-05-28
> Product version: Codex App (web) + Codex CLI v0.116+

---

## 1. UI Paradigm: Task-First, Sandbox-Native

OpenAI Codex represents the most aggressive bet on **task-centric UI** in the AI coding space. The entire interface is organized around submitting a task, watching it execute in a sandbox, and reviewing the result. This is not a chat with a coding assistant — it is a **dispatch-and-monitor** paradigm.

### Core layout

```
┌─────────────────────────────────────────────────┐
│  Codex                                          │
│  ┌─────────────────────────────────────────────┐│
│  │  [Task Input — full-width prompt bar]       ││
│  └─────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────┐│
│  │  Task History (sidebar or stacked cards)    ││
│  │  ┌─ Task #3 ────────────────────────────┐  ││
│  │  │  "Add error handling to auth module" │  ││
│  │  │  ● Running  [sandbox] [branch]       │  ││
│  │  └──────────────────────────────────────┘  ││
│  │  ┌─ Task #2 ────────────────────────────┐  ││
│  │  │  "Write unit tests for payment flow"  │  ││
│  │  │  ✓ Completed  [diff] [apply]         │  ││
│  │  └──────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────┘│
└─────────────────────────────────────────────────┘
```

### Key structural decisions

1. **No file tree.** Codex does not present a persistent file explorer. The sandbox IS the project. You interact with the project through the agent, not by browsing files.

2. **Sandbox-as-project.** Each task spawns an isolated cloud sandbox (Firecracker microVM or Docker). The sandbox IS the working copy — there is no local ↔ remote split.

3. **Git-native completion.** Tasks complete by producing a git branch and PR. The diff view is the primary deliverable, not a chat transcript.

4. **Approval tiers.** Suggest → Auto-edit → Full-auto. The UI makes the trust level explicit and persistent.

5. **Linear task history.** Tasks stack vertically. Each is an independent unit of work with its own sandbox, branch, and outcome. There is no persistent "conversation" in the chat sense.

---

## 2. Interaction Model Deep Dive

### Task submission

- Single prompt bar at top (always visible, never obscured)
- Slash commands: `/review`, `/test`, `/fix`, `/deploy`
- Context is injected automatically (repo, recent files, task history)
- No manual file selection — the agent infers scope

### Task execution monitoring

- Live streaming of agent actions (file reads, writes, shell commands)
- Action log is append-only, not conversational
- Diff appears inline as files are modified
- Branch name auto-generated from task description

### Task completion

- Diff review with accept/reject per hunk
- One-click "Create PR" from the task card
- Option to "Continue in CLI" for tasks that need local follow-up

### Context window management

- Codex App (web) auto-selects relevant files from the repo
- No explicit context picker UI — the agent decides what to read
- Users can @-mention files or paste URLs for manual scoping

---

## 3. Design Philosophy Extraction

### What Codex gets right

| Pattern | Why it works |
|---------|-------------|
| **Task as unit of work** | Matches how developers actually think: "fix this bug", "add this feature". Not "have a conversation about code". |
| **Sandbox isolation** | Eliminates fear of agent damage. Every task is safe to experiment with. |
| **Git branch as deliverable** | The output is a PR, not a chat log. Directly composable into existing workflows. |
| **Minimal chrome** | No sidebar, no file tree, no settings panels. The prompt bar + task cards IS the entire UI. |
| **Streaming action log** | Shows what the agent is doing in real-time without requiring the user to babysit. Trust through transparency. |

### What Codex gets wrong (or leaves incomplete)

| Gap | Impact |
|-----|--------|
| **No project awareness surface** | You cannot see project structure, dependencies, or health without starting a task. There's no "landing page" for the project itself. |
| **No persistent context across tasks** | Each task starts fresh. Learning from task #1 does not carry to task #2 unless you manually paste the output. |
| **Chat is not first-class** | If you want to "think out loud" with the AI before committing to a task, you must use a separate product (ChatGPT). Codex treats exploration as a task. |
| **Single-provider lock-in** | The UI is built around OpenAI models. No model selector, no provider comparison. |
| **No local development bridge** | The sandbox is cloud-only. There is no path from "I want to work on my local machine" to "I want a sandbox" — it's one or the other. |
| **Terminal-like in web** | The streaming log format feels like a terminal transcript pasted into a web page. No rich rendering of code structure, call graphs, or architecture. |

---

## 4. Sandbox Model Analysis

Codex's sandbox is its defining architectural choice:

```
User prompt
    │
    ▼
┌──────────────────┐
│  Codex Scheduler  │
│  (task queue,     │
│   quota, routing) │
└──────┬───────────┘
       │
       ▼
┌──────────────────┐     ┌──────────────────┐
│  Firecracker VM   │────▶│  Git remote       │
│  (isolated FS,    │     │  (branch + PR)    │
│   network policy) │     └──────────────────┘
└──────────────────┘
```

**Key properties:**
- Ephemeral: sandbox dies when task completes
- Network-restricted by default (opt-in for package install)
- Full Linux environment (not a container slice)
- Git repo is cloned at task start; changes are pushed as a branch

**Implications for VibeCody:**
- VibeCody already has `CloudSandboxPanel` + `GatewaySandboxPanel` + `SandboxChatPanel` — the infra exists
- What's missing is the **task-as-unit-of-work** framing. Currently VibeCody's sandbox chat is "chat inside a sandbox" — it should be "submit a task to a sandbox"

---

## 5. Transferable Patterns for VibeCody

### Pattern: Task Card as primary artifact

Codex's task card is the analog of VibeCody's chat tab — but it carries richer metadata:

- Status (queued / running / completed / failed)
- Branch name
- Diff summary (+N/-M lines)
- Duration
- Approval tier used
- One-click actions: View Diff, Apply, Create PR, Continue

VibeCody's `ChatTabManager` tabs could evolve to carry this metadata, transforming "chat tabs" into "task cards with conversation history."

### Pattern: Sandbox-first onboarding

When you open Codex App for the first time, you see:
1. A prompt bar
2. A "Create task" affordance
3. Nothing else

No settings, no API key configuration, no workspace setup. The sandbox is pre-provisioned.

VibeCody's onboarding currently requires: (1) start daemon, (2) set API key, (3) open workspace, (4) find AI chat. That's 4 steps before value. A sandbox-first flow could be: (1) open VibeCoder, (2) type a task. Done.

### Pattern: Git branch as output

Every Codex task produces a branch. This is the correct mental model for agentic coding. VibeCody's `BranchAgentPanel` and `WorktreePoolPanel` have the infra, but the UI doesn't make "branch per task" the default visible output. Making this the primary completion format would align the product with how developers already think about code review.

---

## 6. Codex UI vs VibeCody UI: Structural Comparison

| Dimension | Codex UI | VibeCody UI |
|-----------|----------|-------------|
| Entry point | Task prompt (top) | File editor (center) + AI panel (right) |
| Primary artifact | Task card with branch | Chat tab with messages |
| Project visibility | None (sandbox IS project) | Full file tree + editor |
| Sandbox model | Cloud Firecracker VM | Local Docker + cloud gateway |
| Model selection | None (OpenAI only) | Full provider dropdown |
| Chat paradigm | Linear task log | Multi-tab conversational chat |
| Context control | Auto-inferred | Manual context picker + auto-profile |
| Completion format | Git branch + PR | Diff review + apply |
| Settings surface | Minimal (approval tier) | 308 panels across 11 tab groups |
| Mobile/watch | None | Full mobile + watch clients |

**The fundamental divergence:** Codex optimizes for the **single-task, sandbox-native, cloud-first** developer. VibeCody optimizes for the **multi-task, project-aware, local-first** developer. Both are valid — but VibeCody can absorb Codex's strengths (task cards, sandbox-first flow, branch-as-output) without losing its project-awareness advantage.

---

## 7. Summary: What VibeCody Should Take From Codex

1. **Task card > chat tab.** Reframe the sandbox chat session as a "task" with status, branch, diff summary, and one-click completion actions.
2. **Sandbox-first onboarding.** The first thing a new user sees should be a sandbox-ready prompt, not a file tree.
3. **Branch as deliverable.** Auto-create a branch per sandbox session. Make "Create PR" a first-class completion action.
4. **Minimal chrome for the sandbox flow.** When in sandbox mode, hide the file tree and editor. Show the task card + streaming log + diff. Reduce visual noise.
5. **Auto-context without hiding the manual option.** Codex auto-infers context. VibeCody has a better manual context picker. Merge both: auto-inject context, but show what was included and let the user adjust.
