# `/goal` — Durable Execution Intent

**Status:** Shipped · 2026-05-18 (G1.1–G1.7 baseline, G3 hardening, G4 hierarchy + current-pin, G5 fan-out)
**Scope:** vibecli (daemon + REPL + TUI), vibecoder (Tauri desktop), vibemobile (Flutter), vibewatch (watchOS + Wear OS), vscode-extension, agent-sdk
**Owner:** TBD

---

## What this is

`/goal` is a durable, cross-session record of **what the user is working toward**. It is the forward-looking sibling of [Recap & Resume](../recap-resume/README.md):

- Recap answers *what just happened?* (backward-looking summary of a session/job).
- Resume answers *where do I pick up?* (cursor + seed instruction).
- **Goal** answers *what are we working toward, and what's the next pull?* (intent + plan + link graph).

A goal is not a chat prompt or a task description. It is a stable surface that:

- Survives across sessions and restarts.
- Decomposes into an `ExecutionPlan` on demand (one explicit LLM call, never auto on create).
- Gathers a link graph of sessions, jobs, recaps, and freeform notes that contributed.
- Has a lifecycle (Active → Paused / Done / Abandoned).

## Data shape

```rust
pub struct Goal {
    pub id: String,                  // UUIDv4 hex
    pub workspace: Option<PathBuf>,  // None = global, visible from mobile/watch
    pub title: String,               // ≤120 chars
    pub statement: String,           // free-form body
    pub status: GoalStatus,          // Active | Paused | Done | Abandoned
    pub success_criteria: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parent_goal_id: Option<String>,  // schema reserved; tree ops deferred
    pub current_plan: Option<ExecutionPlan>,  // from vibe_ai::planner
    pub schema_version: u16,
}

pub struct GoalLink {
    pub id: String,
    pub goal_id: String,
    pub kind: GoalLinkKind,   // Session | Job | Recap | Note
    pub target_id: String,
    pub linked_at: DateTime<Utc>,
    pub note: Option<String>,
}
```

Plan invalidation rule: any `PATCH` that mutates `statement` or `success_criteria` clears `current_plan`. A stale plan against a re-stated goal is worse than no plan.

## Storage

- `goals` and `goal_links` tables in **`~/.vibecli/sessions.db`** (unencrypted) — same store as sessions and recaps so all three can JOIN cheaply.
- Workspace is **nullable**. `None` = global goal visible from anywhere (including mobile + watch). `Some(path)` = workspace-bound; the unique index `(IFNULL(workspace, ''), title)` enforces per-workspace dedup.
- `goal_links.target_id` is **not** foreign-keyed to `sessions(id)` because targets can live in other stores (jobs in `jobs.db`, freeform notes with no row at all). Validation is application-layer.

## HTTP surface

All routes require bearer auth (mounted under the `authed_routes` block in `serve.rs`).

| Method | Path | Body | Purpose |
|---|---|---|---|
| `POST`   | `/v1/goals`                 | `{title, statement?, workspace?, success_criteria?, tags?, parent_goal_id?}` | Create |
| `GET`    | `/v1/goals`                 | (query: `status?`, `workspace?`, `tag?`, `limit?`) | List |
| `GET`    | `/v1/goals/:id`             | — | Detail (goal + links) |
| `PATCH`  | `/v1/goals/:id`             | partial `GoalPatch` (incl. double-`Option` `parent_goal_id`) | Update (auto-clears plan on statement/criteria edit) |
| `DELETE` | `/v1/goals/:id`             | — | Hard delete; links + `pinned_goals` rows cascade |
| `POST`   | `/v1/goals/:id/plan`        | `{provider?, model?}` | Generate `ExecutionPlan`; per-request override honored when key resolves (env or `profile_settings.db`); response carries `plan_provider_override_applied` |
| `POST`   | `/v1/goals/:id/start`       | `{task?, provider?, model?}` | Create a session linked to this goal |
| `POST`   | `/v1/goals/:id/link`        | `{kind, target_id, note?}` | Attach an existing session/job/recap/note |
| `POST`   | `/v1/goals/:id/recap`       | `{provider?, model?}` | Aggregate recap across linked targets. LLM synthesis when both fields are supplied and the provider resolves, otherwise heuristic fold. Response carries `recap_synthesizer: "llm" \| "heuristic"` |
| `GET`    | `/v1/goals/:id/children`    | — | One-level tree query. Returns `{parent_goal_id, children, count}` |
| `GET`    | `/v1/goals/:id/tree`        | (query: `depth?` clamped 1..10, default 3) | Recursive subtree walk. Cycle-safe (`cycle: true` flag), truncation flag at the depth boundary |
| `GET`    | `/v1/goals/current`         | (query: `workspace?`) | Look up the pinned goal for a workspace (empty/absent = global slot) |
| `PUT`    | `/v1/goals/current`         | `{goal_id, workspace?}` | Pin or replace the current goal |
| `DELETE` | `/v1/goals/current`         | (query: `workspace?`) | Clear the pin |

## Watch surface

Watch never talks to `/v1/*` directly. The curated `/watch/goals` routes return a slim payload:

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/watch/goals`            | Active goals only, 25 max, `WatchGoalSummary` shape |
| `GET`  | `/watch/goals/:id`        | Full goal + links (same shape as `/v1/goals/:id`) |
| `POST` | `/watch/goals/:id/start`  | Curated wrapper for `do_v1_exec_goal_start`. Body: `{task?}`. Returns `{session_id, link_id, goal_id}` |

## Client surface

| Client | Surface |
|---|---|
| **VibeCLI REPL** (`/goal`) | `new`, `list`, `show`, `status`, `link`, `start`, `children`, `reparent`, `pin`, `unpin`, `current`, `delete`; `plan` points to daemon |
| **VibeCLI TUI** | Read-only `Goals` screen — `/goal` from chat opens it; `f` cycles status filter, `j/k` scroll, `r` refresh |
| **VibeCoder** (Goals panel, tab `goals`) | List + detail + status switcher + Generate Plan + Start session + Linked sessions; New Goal modal; tree-view toggle (children indented under parents); Aggregate recap routed through toolbar `selectedProvider` + `selectedModel` |
| **VibeCoder** slash palette | `/goal` opens the Goals panel; `/goal <text>` opens it and seeds the New Goal modal; AIChat input hybrid mirrors the palette action |
| **VibeMobile** (Flutter) | `listGoals`, `getGoal`, `startGoal`, `getGoalTree`, `getCurrentGoal`, `pinGoal`, `unpinGoal` |
| **Apple Watch** | `loadGoals`, `fetchGoal`, `startGoal` — read-mostly via `/watch/goals` plus the curated `/start` wrapper |
| **Wear OS** | `listGoals`, `getGoal`, `startGoal` plus `GoalDetailScreen` (row tap → detail) and `GoalsTileService` (freshest active goal as a Tile) |
| **VS Code extension** | `listGoals`, `createGoal`, `startGoal`; `vibecli.goalsView` sidebar tree-view (`goals-tree.ts`) with refresh and per-row context-menu actions |
| **Agent SDK** (TypeScript) | `agent.goals.{list,get,create,update,delete,plan,start,link,tree,pin,unpin,current,recap}` |
| **`/agent`** | New sessions auto-link to the pinned goal for the daemon's workspace (or the global slot) — silent best-effort, never blocks session creation |

## Naming — why `exec_goal_*`?

VibeCoder already has `CompanyGoalsPanel` (company strategy goals via `company_cmd "goal …"`) and `AgilePanel` (sprint goals). The HTTP path stays friendly (`/v1/goals`) but the Rust module is `exec_goal.rs` and the Tauri commands are `exec_goal_*` so future maintainers reading `commands.rs` see no ambiguity.

## Slice history

### G1 — baseline (commit `55cf91ea`)

- **G1.1** — Types + schema migration + CRUD helpers in `SessionStore`. Unit-tested.
- **G1.2** — Daemon HTTP CRUD.
- **G1.3** — `/plan`, `/link`, `/start` routes; PlannerAgent wiring.
- **G1.4** — REPL `/goal` subcommands.
- **G1.5** — VibeCoder panel + Tauri commands + slash palette action.
- **G1.6** — Mobile + Watch curated routes.
- **G1.7** — VS Code + Agent SDK + design docs.

### G3 — follow-up (commit `0ef69c24`)

- **G3.1** — Read-only TUI `Goals` screen (`CurrentScreen::Goals` + `GoalsComponent` + `draw_goals`).
- **G3.2** — Goal trees: `parent_goal_id` column + `idx_goals_parent`, `GoalPatch.parent_goal_id` as double-`Option`, `/v1/goals/:id/children` one-level query, REPL `/goal children` + `/goal reparent`.
- **G3.3** — AIChat `/goal <text>` hybrid (typing the command in the chat input opens the Goals panel, seeded when text is present).
- **G3.4** — Per-request planner provider override: body `provider`/`model` honored when key resolves (env or `profile_settings.db`); response carries `plan_provider_override_applied`, `plan_provider_requested`, `plan_model_requested`.
- **G3.6** — Wear OS `GoalDetailScreen` behind a row tap; `GoalsTileService` for the freshest active goal; `AndroidManifest` service registration.

### G4 — hardening (commit `4c0294fc`)

- **G4.1** — `GET /v1/goals/:id/tree?depth=N` recursive subtree walk (depth clamped 1..10, default 3, cycle-safe, truncation flag).
- **G4.2** — `GET/PUT/DELETE /v1/goals/current` per-workspace "current pin" with `pinned_goals` table cascading on goal delete; empty/absent workspace = global slot.
- **G4.3** — `/v1/goals/:id/recap` LLM synthesis (heuristic fallback retained); response carries `recap_synthesizer`.
- **G4.4** — REPL `/goal pin`, `/goal unpin`, `/goal current`.
- **G4.5** — Apple Watch "Start session" routes through the new curated `POST /watch/goals/:id/start` wrapper.
- **G4.6** — VS Code `vibecli.goalsView` sidebar tree-view (`goals-tree.ts`) with refresh + per-row context-menu actions.

### G5 — fan-out parity (commit `4c0294fc`)

- **G5.1** — Wear OS `GoalDetailScreen` "Start session" chip backed by `WearNetworkManager.startGoal()`.
- **G5.2** — `/agent` auto-link of new sessions to the pinned goal for the daemon's workspace (or the global slot) — silent best-effort, never blocks session creation.
- **G5.3** — Agent SDK `goals.*` adds `tree(id, depth?)`, `pin(id, ws?)`, `unpin(ws?)`, `current(ws?)`, `recap(id, {provider, model})`.
- **G5.4** — Flutter `ApiClient` gains parallel `getGoalTree`, `getCurrentGoal`, `pinGoal`, `unpinGoal`.
- **G5.5** — VibeCoder `GoalPanel` tree-view toggle (children indented under parents) + Aggregate recap section routed through `selectedProvider` + `selectedModel`.

## Deferred / future

All four items originally listed here (aggregate-recap helper, TUI Goals screen, hierarchy operations, per-request provider override for `/plan`) shipped in G3–G5. The list is intentionally empty today; new items will be added as concrete UX needs arrive.
