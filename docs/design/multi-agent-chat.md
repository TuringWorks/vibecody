# Multi-Agent Chat — Migration Design

**Status:** Proposed
**Date:** 2026-04-21
**Owner:** VibeUI / vibe-ai
**Related:** `docs/chat-workflow.md`, `docs/AGENT-FRAMEWORK-BLUEPRINT.md`

---

## 0. The bug we are fixing

`stream_chat_message` (`vibeui/src-tauri/src/commands.rs:2411`) is single-turn:

1. It streams one model completion to the frontend (`chat:chunk`).
2. After the stream ends it runs `process_tool_calls` **once** (`vibeui/src-tauri/src/commands.rs:3600`), which scans the assistant text for ad-hoc XML tags like `<read_file path="…" />`, `<write_file …>`, `<list_dir …>`.
3. The tool output is shipped back as `chat:complete`.

There is **no second model turn**. So when the model emits `<list_dir path="src" />`, the user sees the directory listing — but the model never observes it, never decides what to do next, and the user has to type "continue" to push the conversation forward. A short-term frontend auto-continue loop is shipping now to mask this; the long-term fix is a real agentic loop.

The good news: the proper agent loop already exists. We do not need to invent it — we need to wire Chat into it.

---

## 1. Inventory — what's already built

### 1.1 The agent loop (Rust, `vibeui/crates/vibe-ai`)

| Concern | File:line | Status |
|---|---|---|
| `AgentLoop` struct + `run()` cycle | `vibeui/crates/vibe-ai/src/agent.rs:525`, `:622` | Built. Plan→act→observe with circuit breaker, retry, hooks, policy, context pruning. |
| `ApprovalPolicy` enum (`ChatOnly`, `Suggest`, `AutoEdit`, `FullAuto`) | `vibeui/crates/vibe-ai/src/agent.rs:320` | Built. Goose-equivalent labels at `:343`. |
| `AgentEvent` enum (StreamChunk, ToolCallPending, ToolCallExecuted, Complete, Partial, Error, RetryableError, CircuitBreak) | `vibeui/crates/vibe-ai/src/agent.rs:367` | Built. |
| `AgentContext` (workspace, git, depth, parent_session_id, active_agent_counter, **team_bus**, **team_agent_id**, memory, plan, project_summary) | `vibeui/crates/vibe-ai/src/agent.rs:470` | Built. Note the team-related fields already exist but are not wired in Chat. |
| `ToolExecutorTrait` (decouples loop from executor) | `vibeui/crates/vibe-ai/src/agent.rs:518` | Built. |
| `CircuitBreaker` (Stalled/Spinning/Degraded/Blocked, with antifragile half-open recovery) | `vibeui/crates/vibe-ai/src/agent.rs:88`, `:138` | Built. |
| Prompt-injection sanitizer for tool outputs | `vibeui/crates/vibe-ai/src/agent.rs:25`, `:46` | Built. |
| `ToolCall::SpawnAgent { task, max_steps, max_depth }` (the subagent primitive) | `vibeui/crates/vibe-ai/src/tools.rs:375` | Built. |
| `ToolCall::TaskComplete`, `Think`, `PlanTask`, `RecordMemory`, `Diffstat` | `vibeui/crates/vibe-ai/src/tools.rs:369`, `:385`, `:389`, `:397`, `:393` | Built. |
| `MultiAgentOrchestrator` (parallel agents in git worktrees) | `vibeui/crates/vibe-ai/src/multi_agent.rs:103` | Built. Currently used by `start_parallel_agent_task` only. |
| `AgentTeam` + `TeamMessageBus` (peer-to-peer broadcast/directed messages) | `vibeui/crates/vibe-ai/src/agent_team.rs:70`, `:158` | Built but **not wired to Chat or Sandbox** — `team_bus`/`team_agent_id` on `AgentContext` are set to `None` in `start_agent_task`. |
| `HookRunner` + `HookEvent` (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, TaskCompleted, **SubagentStart**, FileSaved/Created/Deleted) | `vibeui/crates/vibe-ai/src/hooks.rs:24`, `:107` | Built. Three handler types: shell command, LLM eval, HTTP webhook. |
| `AdminPolicy` from `.vibecli/policy.toml` | `vibeui/crates/vibe-ai/src/policy.rs` (loaded at `agent.rs:607`) | Built. Can clamp `max_steps`. |

### 1.2 Tauri executor + commands (`vibeui/src-tauri/src`)

| Concern | File:line | Status |
|---|---|---|
| `TauriToolExecutor` impl `ToolExecutorTrait` (read/write/patch/bash/search/list/web/fetch/diffstat/memory) | `vibeui/src-tauri/src/agent_executor.rs:58`, dispatch at `:538` | Built. |
| Path-traversal protection (`resolve()`) | `vibeui/src-tauri/src/agent_executor.rs:91` | Built. |
| Bash blocklist (rm -rf, fork bombs, exfil, reverse shells) | `vibeui/src-tauri/src/agent_executor.rs:172` | Built. |
| SSRF guard (`validate_url_for_ssrf`) | `vibeui/src-tauri/src/agent_executor.rs:21` | Built. |
| Bash 120s timeout | `vibeui/src-tauri/src/agent_executor.rs:17`, `:225` | Built. |
| `spawn_sub_agent` — depth ≤ 5 hard cap, global ≤ 20 active | `vibeui/src-tauri/src/agent_executor.rs:388`, `:406`, `:424` | Built. Inherits parent context, runs sub-loop, streams steps back. |
| `start_agent_task` Tauri command (events: `agent:chunk`, `agent:pending`, `agent:step`, `agent:complete`, `agent:partial`, `agent:error`, `agent:retry`, `agent:circuit_break`) | `vibeui/src-tauri/src/commands.rs:4115` | Built. Bridges `AgentEvent` → frontend, persists checkpoints to `.vibe/agent-runs/<id>.json`. |
| `stop_agent_task` (abort + clear pending) | `vibeui/src-tauri/src/commands.rs:4417` | Built. |
| `resume_agent_task` (replays remaining plan from checkpoint) | `vibeui/src-tauri/src/commands.rs:4481` | Built. |
| `respond_to_agent_approval(approved: bool)` (resolves the `oneshot` from `ToolCallPending`) | `vibeui/src-tauri/src/commands.rs:4852` | Built. |
| `start_parallel_agent_task` (LLM-decomposes + runs N agents) | `vibeui/src-tauri/src/commands.rs:4539` | Built. |
| Pre-baked `PostToolUse` hooks (cargo check on `.rs`, tsc on `.ts`/`.tsx`) | `.claude/settings.json:6-18` | Built — but these are Claude Code's harness hooks, not vibe-ai HookRunner hooks. See §5. |

### 1.3 Frontend (`vibeui/src`)

| Concern | File:line | Status |
|---|---|---|
| Single-turn `stream_chat_message` invocation | `vibeui/src/components/AIChat.tsx:1494` | Built — this is what we are migrating away from. |
| `chat:chunk` / `chat:complete` / `chat:error` / `chat:status` / `chat:metrics` / `file:written` listeners | `vibeui/src/components/AIChat.tsx:1235`, `:1257`, `:1316`, `:1347`, `:1369`, `:1385` | Built. |
| Ad-hoc tool-call XML parser (`parseToolCalls`) | `vibeui/src/components/AIChat.tsx:1260` | Built — temporary; deprecate when migration completes. |
| `start_agent_task` reference impl in Sandbox panel — listens to `agent:chunk/step/pending/complete/error`, calls `respond_to_agent_approval` | `vibeui/src/components/SandboxChatPanel.tsx:77`, `:113`, `:150`, `:157`, `:188`, `:233`, `:259`, `:267` | Built. **This is the template for AIChat.tsx.** |
| `AgentPanel`, `SubAgentPanel`, `AgentOSDashboard`, `TraceDashboard` — visualize agent runs | `vibeui/src/components/AgentPanel.tsx`, `SubAgentPanel.tsx`, `AgentOSDashboard.tsx`, `TraceDashboard.tsx` | Built. |
| `ChatTabManager` (per-tab provider, history persistence) | `vibeui/src/components/ChatTabManager.tsx` (709 lines) | Built. Owns `messages` and forwards to `AIChat` — lifts state across tabs. Does **not** know about agent events today. |

### 1.4 Skills relevant to a verifier subagent (`vibecli/vibecli-cli/skills/`)

711 skill files total. The ones most directly applicable to a verifier role:

- `self-review-gate.md` — **closest existing analog**: build, lint, test, security, format, typecheck, diff-review. Already structured around "before mark complete." (`vibecli/vibecli-cli/skills/self-review-gate.md:1-30`)
- `ai-code-review.md`
- `review-code-review.md`, `review-pr-best-practices.md`, `review-refactor-patterns.md`, `review-tech-debt.md`
- `critical-thinking.md`
- `formal-verification.md`, `security-application-verification.md`, `security-review.md`
- `collaborative-review-protocol.md`
- `test-impact.md`, `visual-verify.md`
- `rust-safety-critical.md`, `misra-c-safety-critical.md`

Skills are loaded by `SkillLoader` (`vibeui/crates/vibe-ai/src/skills.rs`) and can be dropped into the system prompt for a spawned agent via `AgentContext.extra_skill_dirs`.

### 1.5 What's NOT built

1. **AIChat.tsx → `start_agent_task` wiring.** Sandbox uses it; Chat does not.
2. **Verifier subagent role.** `SpawnAgent` is a plain task spawner — there is no canonical verifier prompt, and no convention for "the verifier said X, the main agent should react." The `self-review-gate` skill is content-only; nothing automatically loads it.
3. **Subagent panel rendering inside Chat.** `SubAgentPanel.tsx` exists but is its own surface; Chat needs an inline collapsible card per spawned agent.
4. **Per-message routing (Q&A vs. task).** No classifier today decides whether to use `stream_chat_message` (cheap) or `start_agent_task` (full loop). Today Chat always picks the former.
5. **Bridging Claude Code's `PostToolUse` hooks (`.claude/settings.json`) into the runtime agent.** Those hooks fire only inside the Claude Code harness on `Edit`/`Write` of files we author for the codebase — they do not fire when *the running vibe-ai agent* writes a file. To get the same behavior at runtime we need vibe-ai `HookRunner` configs (`hooks.rs:107`) registered on `PostToolUse` for `write_file`/`apply_patch`. Conflating these two is the most common reasoning trap in this design.
6. **Plan-mode → agent-loop bridge.** `AgentContext.approved_plan` exists (`agent.rs:478`); the Plan panel doesn't push into it for the Chat surface.
7. **`team_bus` activation.** The peer-to-peer message bus is built but every Chat-spawned agent gets `team_bus: None`.
8. **Cost telemetry.** `record_agent_run_stats` exists for run-level token estimation (`commands.rs:4301`) but per-subagent cost is not aggregated, and there is no "this run will spawn ~3 agents at ~$0.40" preflight estimate.

---

## 2. Target architecture

### 2.1 Picture

```text
┌──────────────────────────────────────────────────────────────────────┐
│                           AIChat panel                               │
│                                                                      │
│  Q&A path                  Task path                                 │
│  ────────                  ─────────                                 │
│  stream_chat_message  ──   start_agent_task(approval, provider)      │
│  (single turn)             │                                         │
│                            ▼                                         │
│                       AgentLoop  (root)                              │
│                            │                                         │
│                            │ may call SpawnAgent for…                │
│                            │                                         │
│         ┌──────────────────┼──────────────────┬──────────────┐       │
│         ▼                  ▼                  ▼              ▼       │
│    Explore subagent   Plan subagent     Worker subagent   Verifier   │
│    (read-only,        (no tools,        (full tools)      (PostTool  │
│     ChatOnly+search)  produces plan)                      gated)     │
│                                                                      │
│   each child has:                                                    │
│     - own context window (own message[])                             │
│     - own `AgentContext` (depth=parent.depth+1)                      │
│     - own approval policy (usually FullAuto for short tasks)         │
│     - inherits `active_agent_counter` (≤20 across the tree)          │
│                                                                      │
│   children stream their own AgentEvents back through the parent's    │
│   `spawn_sub_agent` (agent_executor.rs:388) which collapses them     │
│   into a single ToolResult for the parent's message[].               │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
            Tauri events (agent:chunk/step/pending/complete/error)
                            │
                            ▼
                  ChatTabManager forwards
```

### 2.2 Per-agent context isolation

The existing `spawn_sub_agent` (`agent_executor.rs:388-506`) already provides isolation: the child gets a fresh `messages: Vec<Message>` inside its own `AgentLoop::run`, with a freshly built system prompt from `build_system_prompt(&child_context, …)`. No parent message history leaks. The parent receives **only the child's final summary + a per-step trace** (`agent_executor.rs:467-502`), formatted as a single `ToolResult` it can reason about. This is the Claude-Code-style "context isolation by subagent" pattern. **No new code needed for this property.**

### 2.3 Where each surface fits

| Surface | Today | Target |
|---|---|---|
| **Chat** (`AIChat.tsx`) | `stream_chat_message` always | Router: short Q&A → `stream_chat_message`; tasks → `start_agent_task`. See §3. |
| **Sandbox** (`SandboxChatPanel.tsx`) | `start_agent_task` already | No change to mechanism. Default policy = `FullAuto` (sandbox is the "go ahead" surface). |
| **Plan** panel | Standalone | Push approved plan into `AgentContext.approved_plan` and call `start_agent_task` with `--use-plan` semantics. |
| **AgentOS Dashboard** | Read-only view of `state.sub_agents` | Add per-subagent token/cost roll-up (TBD §6). |
| **MultiModel / Arena** | Independent | Out of scope. |

### 2.4 Verifier loop placement

The verifier is a **subagent the main agent spawns**, not a wrapper around it. Reasons:

1. The main agent has the plan; only it knows whether the verifier's complaints are spurious.
2. Subagents already have isolated context windows — reusing the same primitive avoids inventing a parallel "post-processor" mechanism.
3. Spawn is depth-counted (`agent_executor.rs:405-415`) and globally bounded (`:424`), so verifier spawns can't snowball.

The trigger is a `PostToolUse` hook (`hooks.rs:29`) on `task_complete`: the hook intercepts the would-be terminal call, runs the verifier, and either lets `task_complete` through or injects context demanding fixes. See §5.

---

## 3. UI surface changes

### 3.1 `AIChat.tsx` — listen for agent events

Today: listens to `chat:chunk`/`chat:complete`/`chat:error`/`chat:status`/`chat:metrics`/`file:written` (`AIChat.tsx:1235-1389`).

Add (mirroring `SandboxChatPanel.tsx:77-188`):

| Event | Handler behavior |
|---|---|
| `agent:chunk` | Append to current streaming bubble (same UX as `chat:chunk`). |
| `agent:step` | Render an inline collapsible step card under the assistant message: tool name, summary, success/fail badge, output (truncated, expand on click). Reuse `parseToolCalls` rendering style. |
| `agent:pending` | Show modal/inline approval banner: tool summary, destructive flag (`AgentPendingPayload.is_destructive`), Approve/Reject buttons → `respond_to_agent_approval(true|false)`. **In Chat the default approval policy is `Suggest` so this fires often** (see §4). |
| `agent:complete` | Append final assistant message with the summary. Clear streaming state. |
| `agent:partial` | Show "stopped early" banner with `remaining_plan` and a Resume button → `resume_agent_task(checkpoint_id)`. |
| `agent:error` | Same as `chat:error`. |
| `agent:retry` | Update existing retry indicator UI. |
| `agent:circuit_break` | Show "Agent appears stuck" toast with the `state` and `reason`. |
| **NEW** `agent:subagent` (proposed) | Render an inline subagent card. See §3.3. |

`stream_chat_message` listeners stay — short-Q&A path keeps using them.

### 3.2 Routing — Q&A vs. task

We propose a heuristic on the **frontend** at `AIChat.tsx:1417` (`sendMessage`), gated by a setting (`Settings → Chat → Always use agent loop`):

| Heuristic | Route |
|---|---|
| `messageText.length < 200` AND no `@file` references AND no imperative verbs (`fix`, `add`, `write`, `create`, `refactor`, `build`, `run`, `test`) | `stream_chat_message` (cheap). |
| Anything else | `start_agent_task`. |
| Setting `agent.always_on = true` | `start_agent_task` always. |
| Setting `agent.always_on = false` AND user prefixes with `/agent` slash command | `start_agent_task`. |

Why frontend: the heuristic is cheap, doesn't require shipping a request just to be told to retry as an agent, and is easy for the user to override with a toggle.

**TBD experiment:** measure on a sample of 100 real Chat messages whether the heuristic correctly classifies. If precision < 80%, move classification to a tiny LLM call on the backend (Haiku-equivalent, ~50 tokens). Resolution: log every routing decision for one week behind a `chat.routing_telemetry` flag.

### 3.3 Subagent visualization — proposed `agent:subagent` event

Today, when the parent agent calls `SpawnAgent`, the child runs to completion and the parent gets a single collapsed step (`agent:step` with `tool_name = "spawn_agent"`). The user sees only the parent's perspective.

**Proposal:** add a new event family. In `agent_executor.rs:469-489`, while the child's events stream in, also forward:

- `agent:subagent_started { id, parent_id, task, depth }`
- `agent:subagent_step { id, step }` (forward the child's `ToolCallExecuted`)
- `agent:subagent_complete { id, summary }`

`AIChat.tsx` renders each subagent as a nested collapsible panel keyed by `id`, indented by `depth`. This reuses `SubAgentPanel.tsx` styling. **TBD:** event volume — at depth 3 with 10 steps each, a single user turn could fire ~30 subagent events; ensure listener throttling. Resolution: prototype with one task that explicitly calls 3 spawn_agents.

### 3.4 `ChatTabManager.tsx`

Minimal change. Today it owns `messages` per tab and forwards to `AIChat` (`ChatTabManager.tsx:1-80`). It does **not** subscribe to chat events itself. After the migration:

- Pass per-tab `agentSessionId` so that `agent:*` events emitted with `session_id` (TBD, requires adding session_id to `AgentEvent` payloads — see §8) can be routed to the correct tab.
- Persist `messages` including agent steps so the tab snapshot is replayable from `localStorage`.

**TBD:** The `agent:*` events today are global, not per-tab. If two tabs run agents simultaneously, they cross-contaminate. Either (a) restrict to one active agent task at a time across all tabs (simplest, matches `state.agent_abort_handle: Option<AbortHandle>` at `commands.rs:4179` — there's only one slot), or (b) refactor the backend to keep a `HashMap<TabId, AbortHandle>` and emit events scoped to a tab. **Phase 1 picks (a); phase 3 reconsiders (b).**

---

## 4. Approval policy UX

### 4.1 The four policies

From `agent.rs:320-340`:

| Variant | display_name (`agent.rs:343`) | Behavior |
|---|---|---|
| `ChatOnly` | "Chat Only" | All tool calls blocked. |
| `Suggest` | "Manual Approval" | Every tool call → `agent:pending` → user approves/rejects. |
| `AutoEdit` | "Smart Approval" | File edits auto-applied; bash needs approval. |
| `FullAuto` | "Completely Autonomous" | Everything runs without prompting. |

### 4.2 Per-surface defaults (proposed)

| Surface | Default | Rationale |
|---|---|---|
| **Chat** | `Suggest` | Chat is conversational; user is at the keyboard and expects to confirm destructive actions. Don't surprise people. |
| **Sandbox** | `FullAuto` | The whole point of Sandbox is "let it rip in an isolated workspace." Already the implicit default. |
| **Plan-then-execute flow** | `AutoEdit` (after plan approval) | The plan was pre-approved; let edits flow but still gate bash. |
| **`/agent --auto`** explicit slash | `FullAuto` | User opted in by typing it. |

### 4.3 Toggle UX

Mirror `SandboxChatPanel.tsx:398-399` — a `<select>` near the message input with the four `display_name` labels. Persist per-tab in `ChatTabManager.tsx`. Highlight in red when set to `FullAuto`. When the user changes a tab from `Suggest` → `FullAuto` mid-conversation, show a one-time confirmation toast.

### 4.4 What "Reject" means

Today `respond_to_agent_approval(false)` sends `None` through the oneshot (`commands.rs:4896`). Inside `AgentLoop::run` this is recorded as a "rejected by user" `ToolResult` (search `agent.rs` for `oneshot`). The model sees that and (in practice) tries a different approach. We do not need to change this — but we should surface in the UI a quick "Tell the agent why" textarea so the rejection injects useful context, not just "rejected." **Phase 2 work.**

---

## 5. Verifier subagent design

### 5.1 When does it run

The cleanest trigger is a **`PostToolUse` hook on `task_complete`** registered via the existing `HookRunner`:

- `HookEvent::PostToolUse { call: ToolCall::TaskComplete { .. }, .. }` (`hooks.rs:29`)
- Handler: `HookHandler::Llm { prompt }` (`hooks.rs:117`) running a verifier prompt, OR `HookHandler::Command` invoking a script that itself calls `start_agent_task` for a verifier child. The Llm path is simpler and lives entirely in-process.

Hook decisions (`hooks.rs:97`):
- `Allow` → `task_complete` proceeds, agent finishes.
- `Block { reason }` → `task_complete` is rejected; the reason is fed back as a tool result; the main agent must address it before it can mark complete again.
- `InjectContext { text }` → `task_complete` proceeds **but** the verifier's notes are appended to the next turn (used when the verifier finds non-blocking nits, useful for a follow-up commit message).

Why `task_complete` (not after every code change): running a verifier on every `write_file` makes a 10-step task spawn 10 verifiers — a 10× token blow-up for marginal value. The codebase already has `PostToolUse` shell hooks for type-checking each `write_file` (`.claude/settings.json:6-18`) — those run cheap `cargo check`/`tsc` per save. Reserve the LLM verifier for the moment the agent claims to be done.

**Alternative trigger considered & rejected:** "verify after every write_file." Cost-prohibitive (see §6) and the PostToolUse shell hooks already cover correctness at that granularity.

### 5.2 What the verifier checks

Reuse `vibecli/vibecli-cli/skills/self-review-gate.md:1-30`:

1. **Build clean** — language-detected (`cargo check --workspace --exclude vibe-collab`, `npm run build`, `go build ./...`). Reuses `.claude/settings.json` patterns.
2. **Lint clean** — clippy / eslint / ruff.
3. **Tests pass** — but only the impacted ones (use the `test-impact.md` skill heuristic if available, else skip).
4. **Diff review** — verifier reads its own `git diff HEAD`, looks for: secrets, dead branches, unrelated changes, missing error handling at boundaries (per `vibe-ai`'s `policy.toml`).
5. **Acceptance against the original user request** — does the diff actually do what the user asked? This is the part type-checkers can't catch.

### 5.3 How it reports back

The verifier is a `SpawnAgent` child running a **`ChatOnly` + read-only tool subset** (no `write_file`, no `bash`). Implementation note: today `ApprovalPolicy::ChatOnly` blocks all tools (`agent.rs:321`), which would prevent even `read_file` and `diffstat`. Either:

(a) Add a new policy variant `ReadOnly` that allows non-destructive tools; or
(b) Use `Suggest` with a hook that auto-approves only the read-set.

**Recommend (a)** — it's a 30-line change in `agent.rs:320`, and "read-only audit" is a pattern useful beyond verifier (e.g. Explore subagent, Plan subagent).

The verifier returns a structured summary:

```
PASS  — all checks green
FAIL  — checks: [build|lint|test|diff|acceptance], details: …
NITS  — checks pass but: …
```

The main agent sees this in its message[] as the `task_complete` tool result and either retries (FAIL), commits with the nits noted (NITS), or finishes (PASS).

### 5.4 Override

The user can:

- Reject the verifier's `Block` decision via the `agent:pending` flow (verifier hook decisions surface as a special pending event — **TBD: payload shape**, see §8).
- Disable the verifier per-conversation via a Chat setting (`agent.verifier.enabled = false`), persisted per tab.
- Disable globally in `.vibecli/policy.toml` (extends the existing `AdminPolicy`).

---

## 6. Cost / latency analysis

### 6.1 Token math (rough order of magnitude)

Numbers below assume Sonnet-class pricing (~$3/M input, ~$15/M output) and a typical "fix this bug" task. **All numbers are estimates — confirm with a 10-task benchmark before defaulting any policy on.**

| Mode | Calls | Avg input tokens / call | Avg output tokens / call | Total cost (one task) |
|---|---|---|---|---|
| Today (`stream_chat_message`, single turn) | 1 | 4 K | 1 K | ~$0.027 |
| Today + manual "continue" loop (~3 turns) | 3 | 6 K, 8 K, 10 K | 1 K each | ~$0.117 |
| `start_agent_task`, 8 steps | 8 | growing 4 K → 14 K | 800 each | ~$0.45 |
| Same + 1 verifier subagent | 9 | +1 verifier @ 6 K in / 400 out | | ~$0.49 |
| Same + Explore + Plan + 2 Workers + Verifier | ~20 | mixed | | ~$1.20 |

Implications:

- A "Q&A" routed through the agent loop costs ~17× a single-turn chat.
- A multi-subagent run costs ~40–50× a single-turn chat.
- The verifier itself is cheap (one extra call); the multiplier comes from **letting the agent loop run instead of stopping after one turn**.

This is the right tradeoff for *tasks* but the wrong tradeoff for *Q&A*. Hence the routing in §3.2.

### 6.2 When the main agent should NOT spawn a subagent

System-prompt guidance to add to the spawned-agent doc (`tools.rs:317`):

- Q&A about the codebase (use parent's `read_file` directly).
- Single-file edits where parent already has the file in context.
- Anything < 3 expected steps.
- Anything where parallelism gains nothing (sequential dependencies).

When TO spawn:

- Parallel work streams ("write the tests AND the docs").
- Read-heavy explore over a large unfamiliar area (Explore subagent, ChatOnly + read tools).
- Plan generation when the parent's context is already large.
- Verification (the canonical case).

### 6.3 Latency

`start_agent_task` adds wall-clock latency proportional to step count. A 10-step task at ~5s/step = 50s before the user sees `agent:complete`. Mitigations already in place: streaming `agent:chunk` keeps the UI feeling alive; `agent:step` events show progress. **No additional work needed in phase 1.**

### 6.4 Subagent budget cap

Add a per-task token budget. `AgentContext` doesn't have this field today (`agent.rs:470`). Proposed: add `task_token_budget: Option<u64>` and abort when exceeded with `AgentEvent::Partial`. **TBD:** what's the right default? Suggest 100 K tokens / task. Resolution: ship without the cap in phase 1, instrument `record_agent_run_stats` to measure typical usage, set the cap in phase 3.

---

## 7. Migration plan

### Phase 1 — wire AIChat to `start_agent_task` behind a flag

Goal: existing single-turn behavior is unchanged by default; opt-in users get the agent loop.

Work items:

1. Add `Settings → Chat → Use agent loop` toggle (defaults off). Persist in `~/.vibeui/settings.json`.
2. In `AIChat.tsx:1417`, branch on the toggle: if on, call `start_agent_task` with `approval_policy = "suggest"`. If off, current code path.
3. Add agent-event listeners next to existing chat-event listeners (`AIChat.tsx:1228-1394`). Reuse `SandboxChatPanel.tsx` patterns.
4. Add inline `agent:step` rendering (collapsed by default; reuse existing tool-call card style).
5. Add the `agent:pending` approval banner with Approve/Reject buttons → `respond_to_agent_approval`.
6. Document that only one agent task runs at a time across all tabs (single `agent_abort_handle` slot at `lib.rs:101`).

**Acceptance criteria:**
- A user with the flag on can ask "list the files in src/" and the agent does `<list_directory>` → sees output → composes a final answer in one turn from their perspective. No "type continue."
- A user with the flag off sees zero behavior change.
- All existing AIChat tests pass.
- New BDD: "agent loop completes a multi-step task," "approval rejection halts the tool call," "stop button aborts mid-run."

### Phase 2 — verifier subagent

Goal: when the main agent calls `task_complete`, a verifier runs and can block/inject.

Work items:

1. Add `ApprovalPolicy::ReadOnly` variant in `agent.rs:320` (allows `read_file`, `list_directory`, `search_files`, `diffstat`, `think`; blocks all writes/bash/spawn).
2. Add a default verifier `HookConfig` (`hooks.rs:138`) loaded automatically when `agent.verifier.enabled = true` (default true in phase 2 only for Chat — Sandbox stays off).
3. Verifier prompt template: a condensed version of `self-review-gate.md` plus the user's original task. Emitted as `HookHandler::Llm`.
4. Plumb the verifier's decision into the agent loop: `Allow` lets `task_complete` through, `Block` injects a synthetic tool result and continues the loop.
5. UI: render verifier output as a distinct kind of step card (e.g., a checkmark badge on PASS, warning on NITS, red on FAIL).
6. Add `agent:verifier` event for clarity (separate from `agent:step` — the user shouldn't see this as "the agent did 9 steps.").

**Acceptance criteria:**
- 10-task benchmark: verifier catches at least 3 cases per 10 where `task_complete` was premature (build broken, missing test, unrelated diff). Numbers approximate; calibrate after running.
- Verifier never spawns its own children (depth-bound enforced).
- Verifier costs ≤ 15% of total task tokens on average.
- User can disable per-conversation and globally.

### Phase 3 — default-on + subagent visualization + per-tab routing

Goal: Chat defaults to the agent loop. Subagent panels render inline. Multiple tabs can run agents simultaneously.

Work items:

1. Flip `agent.always_on` default to true.
2. Implement the routing heuristic in §3.2 with telemetry; tune.
3. Implement `agent:subagent_started/step/complete` events (§3.3).
4. Refactor `AppState.agent_abort_handle: Option<AbortHandle>` → `HashMap<TabId, AbortHandle>` (`lib.rs:101`, `commands.rs:4180`, `commands.rs:4422`).
5. Plumb `tab_id` through `start_agent_task` and into all `agent:*` event payloads.
6. Add token budget cap (`AgentContext.task_token_budget`).
7. Wire `team_bus` for parent + verifier so they can exchange directed messages (`agent_team.rs:70`) — useful for verifier asking "should I check X?" rather than guessing.

**Acceptance criteria:**
- Routing classifier is at ≥ 80% precision on 100 real messages.
- Two tabs can run agents in parallel with no event cross-contamination.
- Subagent panels render with depth-indented nesting.
- Avg cost per Chat message ≤ 3× phase-1 baseline (the rest is Q&A staying cheap).

---

## 8. Open questions

These need a human decision before implementation begins:

1. **Plan-mode source of truth.** `AgentContext.approved_plan` exists (`agent.rs:478`) and `tools.rs:168` documents `plan_task`. The Plan panel (`docs/PlanDocumentPanel`-related code at `vibeui/src/components/PlanDocumentPanel.tsx`) does not push into either today. Do we (a) keep Plan panel as a separate flow that calls `start_agent_task` with a pre-filled `approved_plan`, or (b) make the agent loop always emit a `plan_task` first via a system-prompt nudge and treat the panel as a viewer? **Recommendation: (a). Cleaner separation; reuses the existing field.**

2. **Verifier provider.** Should the verifier subagent use the same provider as the main agent, or always a small fast model (e.g., Haiku-equivalent) regardless? Same provider is simpler; small model is cheaper/faster but adds a config dependency. **Decision needed before phase 2.**

3. **Per-tab vs single-active-agent.** Phase 3 proposes refactoring to per-tab handles. If that's too invasive, we could keep single-active and just disable other tabs' Send button while one agent runs. **Cost: per-tab refactor touches `AppState`, two Tauri commands, three event listeners. UX cost of single-active: bad — users open multiple tabs.** Recommendation: do the refactor in phase 3.

4. **Hook event for verifier decisions.** When the verifier `Block`s `task_complete`, do we emit `agent:pending` (treating the block like a regular approval) or a new `agent:verifier_block`? The former reuses UI; the latter is clearer semantically. **TBD.**

5. **`session_id` propagation in `AgentEvent`.** Today the events emitted by `AgentLoop::run` (`agent.rs:367`) do not carry `session_id` even though the loop has it (`agent.rs:628`). Adding it requires touching every variant of `AgentEvent` and every event emit in `commands.rs:4243-4393`. Phase-3 work, but worth deciding now whether session-scoping is the right model or whether we want tab-scoping (a different identifier).

6. **Skills-as-system-prompt for verifier.** Should the verifier auto-load `self-review-gate.md`, or should we extract a Rust constant? Loading via `SkillLoader` is consistent with the rest of the codebase but adds I/O on every spawn. **Recommendation: extract once into a `const VERIFIER_PROMPT: &str` in vibe-ai; keep the skill file as the source canon updated periodically.**

7. **What to do with `process_tool_calls` (`commands.rs:3600`).** Once Chat is on the agent loop, the legacy XML tag handler in `stream_chat_message` is only exercised by the Q&A path. The model still emits `<read_file …>` etc. for short queries. Two options: (a) delete `process_tool_calls` and let the Q&A path be truly chat-only (no tool execution from a single turn), or (b) keep it as a "lite tools" path. **Recommendation: (a) after phase 3 stabilizes — Q&A doesn't need tools; if the model wants tools the message routes to the agent.**

8. **Mobile / VS Code clients.** Per CLAUDE.md, AIChat is one of 13 clients. Do mobile / VS Code chat surfaces (`vibemobile/`, `vscode-extension/`) follow the same migration? They each have their own chat UIs against the `/api/*` daemon routes. **Out of scope for this doc** — but flag for a follow-up: VS Code in particular benefits from the same "auto-continue" fix.

---

## Appendix A — file:line references in one place

| Subsystem | File:line |
|---|---|
| Bug source: single-turn chat | `vibeui/src-tauri/src/commands.rs:2411` |
| Bug source: one-shot tool processing | `vibeui/src-tauri/src/commands.rs:3600` |
| `AgentLoop` struct | `vibeui/crates/vibe-ai/src/agent.rs:525` |
| `AgentLoop::run` | `vibeui/crates/vibe-ai/src/agent.rs:622` |
| `AgentEvent` enum | `vibeui/crates/vibe-ai/src/agent.rs:367` |
| `AgentContext` | `vibeui/crates/vibe-ai/src/agent.rs:470` |
| `ApprovalPolicy` | `vibeui/crates/vibe-ai/src/agent.rs:320` |
| `ToolExecutorTrait` | `vibeui/crates/vibe-ai/src/agent.rs:518` |
| `CircuitBreaker` | `vibeui/crates/vibe-ai/src/agent.rs:88` |
| Prompt-injection sanitizer | `vibeui/crates/vibe-ai/src/agent.rs:25` |
| `ToolCall` enum (incl. `SpawnAgent`) | `vibeui/crates/vibe-ai/src/tools.rs:338`, `:375` |
| `TaskComplete` | `vibeui/crates/vibe-ai/src/tools.rs:369` |
| `TauriToolExecutor` | `vibeui/src-tauri/src/agent_executor.rs:58` |
| Path resolution / traversal guard | `vibeui/src-tauri/src/agent_executor.rs:91` |
| Bash blocklist | `vibeui/src-tauri/src/agent_executor.rs:172` |
| SSRF guard | `vibeui/src-tauri/src/agent_executor.rs:21` |
| `spawn_sub_agent` (depth/global caps) | `vibeui/src-tauri/src/agent_executor.rs:388` |
| `execute_call` dispatch | `vibeui/src-tauri/src/agent_executor.rs:538` |
| `start_agent_task` Tauri command | `vibeui/src-tauri/src/commands.rs:4115` |
| `stop_agent_task` | `vibeui/src-tauri/src/commands.rs:4417` |
| `resume_agent_task` | `vibeui/src-tauri/src/commands.rs:4481` |
| `respond_to_agent_approval` | `vibeui/src-tauri/src/commands.rs:4852` |
| `start_parallel_agent_task` | `vibeui/src-tauri/src/commands.rs:4539` |
| `PendingAgentCall` slot in AppState | `vibeui/src-tauri/src/commands.rs:99`, `:112` |
| `agent_abort_handle` (single slot) | `vibeui/src-tauri/src/lib.rs:101`, `:181` |
| `HookEvent` / `HookDecision` / `HookHandler` | `vibeui/crates/vibe-ai/src/hooks.rs:24`, `:97`, `:110` |
| `MultiAgentOrchestrator` | `vibeui/crates/vibe-ai/src/multi_agent.rs:103` |
| `AgentTeam` / `TeamMessageBus` | `vibeui/crates/vibe-ai/src/agent_team.rs:70`, `:158` |
| Claude Code harness PostToolUse hooks (NOT vibe-ai hooks) | `.claude/settings.json:6-18` |
| `AIChat.tsx` — `stream_chat_message` invoke | `vibeui/src/components/AIChat.tsx:1494` |
| `AIChat.tsx` — chat event listeners | `vibeui/src/components/AIChat.tsx:1235-1389` |
| `AIChat.tsx` — `parseToolCalls` (legacy XML) | `vibeui/src/components/AIChat.tsx:1260` |
| `SandboxChatPanel.tsx` — agent listeners + approval | `vibeui/src/components/SandboxChatPanel.tsx:77-267` |
| `ChatTabManager.tsx` | `vibeui/src/components/ChatTabManager.tsx` |
| Verifier-shaped skill | `vibecli/vibecli-cli/skills/self-review-gate.md` |
