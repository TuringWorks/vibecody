# `/goal` — Durable Execution Intent

**Status:** Draft · 2026-05-15
**Scope:** vibecli (daemon + REPL), vibeui (Tauri desktop), vibemobile (Flutter), vibewatch (watchOS + Wear OS), vscode-extension, agent-sdk
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
| `POST`   | `/v1/goals`             | `{title, statement?, workspace?, success_criteria?, tags?, parent_goal_id?}` | Create |
| `GET`    | `/v1/goals`             | (query: `status?`, `workspace?`, `tag?`, `limit?`) | List |
| `GET`    | `/v1/goals/:id`         | — | Detail (goal + links) |
| `PATCH`  | `/v1/goals/:id`         | partial `GoalPatch` | Update (auto-clears plan on statement/criteria edit) |
| `DELETE` | `/v1/goals/:id`         | — | Hard delete; links cascade |
| `POST`   | `/v1/goals/:id/plan`    | `{provider?, model?}` | Generate `ExecutionPlan` via `PlannerAgent` |
| `POST`   | `/v1/goals/:id/start`   | `{task?, provider?, model?}` | Create a session linked to this goal |
| `POST`   | `/v1/goals/:id/link`    | `{kind, target_id, note?}` | Attach an existing session/job/recap/note |
| `POST`   | `/v1/goals/:id/recap`   | `{provider?, model?}` | (G1.6+) aggregate recap across linked targets |

## Watch surface

Watch never talks to `/v1/*` directly. The curated `/watch/goals` routes return a slim payload:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/watch/goals`      | Active goals only, 25 max, `WatchGoalSummary` shape |
| `GET` | `/watch/goals/:id`  | Full goal + links (same shape as `/v1/goals/:id`) |

## Client surface

| Client | Surface |
|---|---|
| **VibeCLI REPL** (`/goal`) | `new`, `list`, `show`, `status`, `link`, `start`, `delete`; `plan` points to daemon |
| **VibeUI** (Goals panel, tab `goals`) | List + detail + status switcher + Generate Plan + Start session + Linked sessions; New Goal modal |
| **VibeUI** slash palette | `/goal` opens the Goals panel; `/goal <text>` opens it and seeds the New Goal modal |
| **VibeMobile** (Flutter) | `listGoals`, `getGoal`, `startGoal` — read-mostly remote control |
| **Apple Watch** | `loadGoals`, `fetchGoal` — read-only via `/watch/goals` |
| **Wear OS** | `listGoals`, `getGoal` — read-only via `/watch/goals` |
| **VS Code extension** | `listGoals`, `createGoal`, `startGoal` |
| **Agent SDK** (TypeScript) | `agent.goals.{list,get,create,update,delete,plan,start,link}` |

## Naming — why `exec_goal_*`?

VibeUI already has `CompanyGoalsPanel` (company strategy goals via `company_cmd "goal …"`) and `AgilePanel` (sprint goals). The HTTP path stays friendly (`/v1/goals`) but the Rust module is `exec_goal.rs` and the Tauri commands are `exec_goal_*` so future maintainers reading `commands.rs` see no ambiguity.

## Slice history

- **G1.1** — Types + schema migration + CRUD helpers in `SessionStore`. Unit-tested.
- **G1.2** — Daemon HTTP CRUD.
- **G1.3** — `/plan`, `/link`, `/start` routes; PlannerAgent wiring.
- **G1.4** — REPL `/goal` subcommands.
- **G1.5** — VibeUI panel + Tauri commands + slash palette action.
- **G1.6** — Mobile + Watch curated routes.
- **G1.7** — VS Code + Agent SDK + docs.

## Deferred / future

- Aggregate-recap helper (`POST /v1/goals/:id/recap`) — endpoint is registered and the SDK + Tauri commands proxy through, but the cross-store fan-out (sessions + jobs + recaps) is intentionally a stub until a concrete UX needs it.
- TUI Goals screen — no `CurrentScreen` variant in v1; deferred.
- Hierarchy operations — `parent_goal_id` column is reserved but no `/v1/goals/:id/children` route yet.
- Per-request provider override for `/plan` — currently uses the daemon's configured `ServeState::provider`. Body's `provider`/`model` are accepted but not yet honored.
