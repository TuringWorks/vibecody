---
layout: page
title: Agent Panel
permalink: /agent-panel/
---

The Agent Panel is the core "describe a task, the agent plans and executes it" surface in VibeUI/VibeApp. It owns the agent loop UX — prompt, plan, tool calls, approval gates, partial-completion checkpoints, and parallel-chunk execution via git-worktree isolation.

This page covers the desktop panel. The chat-tab side of agent invocation (per-tab agent loop toggle) is documented in [`docs/chat-tabs`](./chat-tabs.md). The cross-cutting agent runtime lives in `crates/vibe-ai/src/agent_loop.rs`.

---

## What you can do

1. **Type a task.** Free-form natural language — "Add a /health endpoint to src/server.ts", "Refactor the auth module to use the new token store", etc.
2. **Pick an approval policy:**
   - **Suggest** — every tool call requires explicit user approval.
   - **Auto-Edit** — file writes auto-approve; bash and other side-effecting tools still need approval.
   - **Full Auto** — all tools auto-approve. The Turbo toggle is a one-click shortcut to this mode.
3. **Optionally enable Parallel mode** — splits the task into independent chunks (up to 4) and runs each on its own git worktree branch, then merges back. Falls back to sequential execution on non-git workspaces.
4. **Press Run** (⌘/Ctrl+Enter) — the agent starts streaming.

---

## Streaming UI

While the agent is running, the panel shows:

- **Step feed** (role=log) — one card per tool call with the summary line, expandable output (collapses past 600 chars), and a copy button. Auto-scrolls.
- **Streaming card** with a TTFT metric (milliseconds to first token) and a live tokens-per-second counter — useful for diagnosing slow providers or local model setups.
- **Approval prompt** (role=alertdialog) when a tool needs explicit consent. Non-destructive approvals auto-focus the **Approve** button so keyboard users can hit Enter; destructive approvals require an explicit pointer click.

---

## Status states

| State | UI signal |
|---|---|
| `idle` | Run button enabled, empty state visible |
| `running` | Streaming card + step feed populated; Stop button visible |
| `complete` | Reset button visible |
| `partial` | Resume button visible (resumes from last checkpoint) + Reset |
| `error` | Retry button (preserves completed steps) + Reset |

Status changes are announced via a screen-reader-only `aria-live="polite"` region — AT users hear "Agent is running" / "Agent task complete" without visual cues.

---

## Parallel mode + worktree isolation

When **Parallel** is on, the daemon splits the task plan into up to 4 independent chunks. Each chunk runs in its own ephemeral git worktree on a temporary branch; on completion, the branches are squash-merged back into the source branch. This is the only way to avoid clobbering the working tree when multiple agents are editing the same workspace concurrently.

The panel shows the active isolation mode as soon as the daemon decides:

| Badge | Meaning |
|---|---|
| 🛡 Worktree Isolated | Each chunk has its own branch. Conflicts surface as merge errors per chunk. |
| 📋 Sequential Mode | Non-git workspace — chunks run one at a time on the same tree. |

If neither shows, the daemon hasn't reported yet (very early in startup).

---

## Resume from checkpoint

When the agent partially completes a multi-step plan and exits early (timeout, max-steps reached, or stop-by-user), it writes a checkpoint to `<workspace>/.vibe/agent-runs/<task-id>.json`. The Resume button reads the most recent checkpoint and continues from where it left off — completed steps are preserved, only the remaining plan steps are re-issued.

Retry (after `error`) is similar but re-runs the original task without consulting a checkpoint — useful when the failure was transient (network blip, rate limit) rather than a partial-progress situation.

---

## /health declaration

`features.agent_panel`:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "requires": "providers.configured_count > 0",
  "approval_policies": ["suggest", "auto-edit", "full-auto"],
  "parallel_isolation": "git-worktree"
}
```

`available` is `false` if no providers are configured — the panel still renders but shows "Select an AI provider in the header first." Cross-client gating reads `available`; clients that want to know which approval policies the daemon honors read the array (the daemon validates `approval_policy` strings against this list).

---

## Observability

Backend operations emit structured tracing events under `vibecody::agent`:

```bash
RUST_LOG=vibecody::agent=info vibecli serve
```

Events:

```
INFO vibecody::agent: agent.task.start
  provider=ollama approval_policy=auto-edit task_len=124 tab_id=(global)

WARN vibecody::agent: agent.task.provider_unavailable provider=anthropic
INFO vibecody::agent: agent.approval.respond approved=true tool=write_file
INFO vibecody::agent: agent.task.stop was_running=true
```

**Task contents are NEVER logged** — only the length, provider, and approval policy. Tool arguments and outputs are never logged. Operator dashboards aggregate these to spot abuse patterns (long-running tasks, rejection-heavy users) without seeing user prompts.

---

## Accessibility

- Status announcements via `sr-only` `aria-live="polite"` region (e.g., "Agent is running" → "Agent task complete").
- Step feed uses `role="log"` `aria-live="polite"` — AT users hear each step incrementally.
- **Turbo** and **Parallel** toggles expose `aria-pressed` reflecting state, plus verbose `aria-label` ("Turbo Mode on — full-auto approvals enabled").
- Approval prompt is `role="alertdialog"` `aria-live="assertive"` with `aria-labelledby` / `aria-describedby` wiring — AT users get the same urgency signal as a sighted user seeing the bordered card.
- Auto-focus on Approve button (non-destructive only) — destructive actions force an explicit pointer click to defend against muscle-memory Enters.
- All status badges (worktree isolation, error, completion) are static, visible, and color-contrast-checked.

---

## Cross-client behaviour

| Client | Agent UI |
|---|---|
| **VibeUI / VibeApp** | Full panel |
| **VibeMobile** | Single-task agent screen, no parallel mode |
| **VibeWatch** | Active-agent indicator; can stop but not start |
| **IDE plugins** | Implicit — extensions invoke the agent via `start_agent_task` directly |
| **Agent SDK** | Programmatic — same `start_agent_task` Tauri command as the panel uses |

The daemon is the single source of truth for the agent loop. All clients send `start_agent_task` and subscribe to the per-tab event stream (`agent:{tab_id}:{base}`) or the global stream (`agent:{base}`).

---

## Troubleshooting

### "Select an AI provider in the header first"

No provider configured. Open Settings → Providers (or the toolbar dropdown) and add at least one API key. The panel inherits provider availability from the global selection.

### "Agent stopped by user"

Emitted when the Stop button is clicked. The current tool call (if any) is aborted; pending approvals are dropped. The agent task id is marked `cancelled` in the dashboard. To resume from here, click Retry — but note Retry restarts the task from scratch, not from where it stopped.

### "Worktree isolation failed: not a git repo"

Parallel mode requires a git workspace. The daemon falls back to sequential mode — the badge changes from 🛡 to 📋. To re-enable worktree isolation, run `git init` (or open a directory that's already a git repo).

### Partial completions don't show a Resume button

The checkpoint write happens at end-of-task. If the agent crashes hard (panic, OOM) before writing the checkpoint, there's nothing to resume. Use Retry instead.

### "Linter ERRORS - agent must fix before proceeding"

After a `write_file` step, the panel auto-runs a linter against the changed file. If the linter reports errors, a synthetic step appears in the feed (step number `N.5`) with the diagnostic. The agent reads this in its next turn and is expected to fix the errors before claiming completion.

---

## Related

- **Agent runtime:** `crates/vibe-ai/src/agent_loop.rs` — the core loop (Plan → Act → Verify → Done)
- **Source:** `vibeui/src/components/AgentPanel.tsx` (~750 LOC) · backend `vibeui/src-tauri/src/commands.rs` (`start_agent_task`, `stop_agent_task`, `respond_to_agent_approval`, `resume_agent_task`)
- **Tests:** `vibeui/src/components/__tests__/AgentPanel.bdd.test.tsx` (25 BDD scenarios)
- **Sandbox:** [`docs/sandbox`](./sandbox.md) — every agent shell tool call goes through the Tier-0 sandbox
