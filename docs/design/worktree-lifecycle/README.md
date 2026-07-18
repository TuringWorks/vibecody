# Worktree Lifecycle — Design Index

**Status:** Draft · 2026-06-06
**Scope:** vibecli daemon (Rust) — the task/worktree subsystem behind VibeDesk's `/api/tasks`; surfaced in VibeUI, VibeApp, VibeMobile, VibeWatch
**Owner:** TBD

---

## What this is

A lifecycle and garbage-collection policy for the per-task git worktrees the daemon
creates under `.vibecli/worktrees/<id>`. Today a worktree is created when a task is
created, and is only ever torn down on the **merge** path. Every other exit — deleting
a chat, archiving it, deleting a project, or the daemon crashing mid-task — **orphans
the worktree on disk with no back-reference**, because the delete path hard-deletes the
`tasks` row (which is the only thing that knows the `worktree_path`). The default delete
even ships a footgun: `DELETE /api/tasks/:id?remove_worktree=true` runs
`git worktree remove --force`, silently discarding uncommitted work.

This design resolves the two requirements that pull against each other:

1. **Never orphan a worktree** when a chat/task is deleted or archived, or a project is deleted.
2. **Never lose work** when a chat is *accidentally* deleted.

## The core principle

> **Separate _intent_ from _reclaim_.** "The user deleted the chat" is an instant,
> reversible, daemon-side **state change**. "The worktree's disk + branch are physically
> reclaimed" is a deferred **GC action** that runs only when the work is provably safe.
> Destructive git never runs on a delete click.

Two corollaries make this safe and cheap:

- **The reaper lives inside the daemon.** This repo is driven by a live daemon that
  auto-commits/merges/PRs the working tree; a manual `git worktree remove` races it and
  leaves the task DB pointing at a gone path. All physical removal goes through one
  in-daemon reaper.
- **Directory removal and branch deletion are different risks.** A worktree *directory*
  is always safe to remove — it is fully reconstructable from its branch
  (`git worktree add` from the ref). A *branch* is only safe to delete if it has zero
  unique commits. So: reclaim directories aggressively, reclaim branches conservatively.

## Lifecycle states

A task row gains three nullable timestamps; the derived state is computed from them
(no enum migration, fully additive):

| State | Set by | Worktree dir | Branch | Recoverable? |
|---|---|---|---|---|
| **Active** | create | present | present | n/a — live |
| **Archived** | `POST /api/tasks/:id/archive` | reclaimed (disk freed) | **kept forever** | yes — restore re-creates the worktree from the branch |
| **Trashed** | `DELETE /api/tasks/:id` (default) | kept during grace, then reclaimed | kept if unmerged (preserved ref), `-d` only if merged | yes — full restore during grace window |
| **Reaped** | reaper, post-grace | gone | merged → deleted; unmerged → preserved at `refs/trash/<id>` | commits still recoverable via preserved ref / reflog |
| **Merged** | `POST /api/tasks/:id/merge` | gone | merged into HEAD | the work *is* on the target branch |

- **Archive** = "done, might revisit." Costs ~0 disk (directory removed), loses nothing
  (branch kept). Restore = `git worktree add` from the kept branch.
- **Trash** = soft-delete with a grace window (default **14 days**). Restorable from a
  Trash view the whole window. After grace, the reaper reclaims the directory and deletes
  the branch **only if it is merged**; an unmerged branch's tip is moved to
  `refs/trash/<id>` (hidden from normal branch listing, recoverable indefinitely) and only
  the directory is removed.

## Goals

1. **No orphans on any exit.** Delete/archive/project-delete only change state; the
   back-reference (`branch` + `worktree_path`) survives until the reaper removes it.
2. **No lost work on accidental delete.** Delete is reversible during the grace window;
   directory removal is reconstructable from the branch; unmerged branches are preserved,
   never `-D`'d; dirty trees are committed before any removal.
3. **One owner of destructive git.** A single in-daemon reaper (startup sweep + periodic
   loop) is the only code that runs `worktree remove` / `branch -d`.
4. **Self-healing.** The reaper reconciles the filesystem against the DB, so pre-existing
   orphans (and any future bug) get cleaned up without a manual `git worktree prune`.
5. **Cross-surface cascade through one funnel.** Project-delete and chat-delete mark child
   tasks trashed; they never iterate-and-nuke.

## Non-goals

- A general trash bin for *sessions/memory*. Scope is task worktrees; session/memory
  soft-delete can reuse the pattern later but is out of scope here.
- Replacing the **merge** path. Merge already reclaims correctly; we only make its
  worktree teardown commit-safe (no `--force` discard of dirty trees).
- A new DB. Lifecycle timestamps are additive columns on the existing `tasks` table in
  `~/.vibecli/sessions.db`.

## What exists today (grounded in `91ff4b63`)

| Surface | File | State |
|---|---|---|
| Worktree create | `vibecli/vibecli-cli/src/serve.rs:1602` (`create_task`) | `POST /api/tasks?create_worktree=true` → branch `task/<id>-<slug>`, dir `.vibecli/worktrees/<id>`; persisted via `TaskStore::set_worktree` |
| Task↔worktree mapping | `vibecli/vibecli-cli/src/task_store.rs:59` (`TaskRow`) | `branch` + `worktree_path` columns in `tasks` (sessions.db) — **lost on row delete** |
| Merge (clean teardown) | `serve.rs:1810` (`merge_task`) | merge → `remove_worktree` → `delete` row; conflict → abort + keep task |
| Delete (footgun) | `serve.rs:1758` (`delete_task`) | default `?remove_worktree=false` → **row gone, worktree orphaned**; `=true` → `--force` discard |
| Worktree git ops | `vibeui/crates/vibe-core/src/git.rs:386–520` | `create_worktree`, `remove_worktree` (**`--force`**), `list_worktrees`, `merge_worktree_branch` |
| Status enum | `task_store.rs:19` (`TaskStatus`) | Draft/Queued/Running/Reviewing/Completed/Failed — **no Archived/Trashed**; deletes are hard |
| GC / orphan detection | — | **none** — no prune, TTL, periodic loop, or startup sweep |
| Trash / recovery | — | **none** — all deletes permanent |
| Project delete | — | **no endpoint** touches worktrees |
| Startup recovery (prior art) | `serve.rs:6632` (`recover_interrupted`) | sweeps queued/running jobs → failed on boot — the pattern the startup worktree sweep mirrors |

So the *parts* (per-task mapping, git helpers, a startup-sweep precedent, a 24h periodic
loop at `serve.rs:6796`) exist. The work is adding the state columns, the reaper, and
routing every exit through it.

## The reaper

A new module `vibecli/vibecli-cli/src/worktree_reaper.rs` exposing one entry point the
daemon calls on boot and on a timer:

```
sweep(store, repos, policy, now) -> SweepReport
  ├── reap_trashed:    rows trashed before (now - grace) and not Running/Reviewing
  │     for each:  commit-WIP → (merged ? delete branch : preserve ref) → remove dir → mark reaped
  ├── reclaim_archived_dirs:  archived rows whose dir still exists → remove dir, keep branch
  └── reconcile_orphans:  for each repo, scan .vibecli/worktrees/* not referenced by any row
        for each:  clean ? remove dir : (preserve ref + remove dir + log) ; then `git worktree prune`
```

Safety invariants the reaper enforces, in order, before removing any directory:

1. **Never touch a live task.** Skip rows whose `status` is `Running`/`Reviewing` (or
   whose worktree has a held lock).
2. **Commit before remove.** If the worktree is dirty, `git -C <wt> add -A && commit`
   into its branch first (consistent with the daemon's auto-commit behavior). WIP is
   never discarded.
3. **Preserve before delete.** Delete a branch only when
   `git merge-base --is-ancestor <branch> HEAD` (fully merged). Otherwise
   `git update-ref refs/trash/<id> refs/heads/<branch>` and keep the ref; remove only the
   directory.

## API surface

| Route | Before | After |
|---|---|---|
| `DELETE /api/tasks/:id` | hard-delete row (+ optional `--force` worktree) | **soft-delete → Trashed** (sets `trashed_at`); worktree untouched |
| `POST /api/tasks/:id/archive` | — | **new** — set `archived_at`; reaper reclaims dir, keeps branch |
| `POST /api/tasks/:id/restore` | — | **new** — clear `trashed_at`/`archived_at`; re-create worktree from branch if dir was reclaimed |
| `DELETE /api/tasks/:id?purge=true` | — | **new** — explicit permanent delete (routes through reaper's safe teardown, not raw `--force`) |
| `GET /api/tasks` | lists all rows | excludes Trashed by default; `?state=trashed\|archived\|all` to filter |
| `GET /api/tasks/:id/merge` | unchanged | unchanged (already clean) — only the `--force` discard is replaced by commit-then-remove |

`remove_worktree=true` is retained as a back-compat alias for `purge=true` but now goes
through the safe path (commit + preserve), so it can no longer silently discard work.

## Migration

Additive, idempotent (guarded by `PRAGMA table_info`):

```sql
ALTER TABLE tasks ADD COLUMN archived_at INTEGER;  -- nullable
ALTER TABLE tasks ADD COLUMN trashed_at  INTEGER;  -- nullable
ALTER TABLE tasks ADD COLUMN reaped_at   INTEGER;  -- nullable
CREATE INDEX IF NOT EXISTS idx_tasks_trashed  ON tasks(trashed_at);
CREATE INDEX IF NOT EXISTS idx_tasks_archived ON tasks(archived_at);
```

No backfill: existing rows have all three `NULL` → Active, which is correct.

## Rollout slices

1. **Stop the bleeding (this slice).** Store columns + lifecycle methods; `delete_task`
   → soft-delete; `list`/`get` exclude trashed; the reaper module
   (`reconcile_orphans` + `reap_trashed` + startup sweep + periodic loop). This alone
   prevents the orphan accumulation that produced the 20 stray worktrees on `91ff4b63`,
   and makes accidental deletes recoverable.
2. **Recovery UX.** `restore` / `archive` / `purge` routes + a Trash list in VibeDesk, wired
   through VibeUI → VibeMobile.
3. **Cross-surface cascade.** Project-delete and chat-delete cascade-trash child tasks
   through the same funnel; document in the AGENTS.md change-surface cookbook.

## Policy defaults (knobs)

| Knob | Default | Rationale |
|---|---|---|
| Trash grace window | 14 days | Long enough to notice an accidental delete; bounded disk |
| Reaper interval | 6 h | Cheap; orphans are not urgent |
| Startup sweep | on | Heals crashes + pre-existing orphans |
| Archive keeps branch | forever | Archive must never lose work; branches are cheap |
| Preserve-ref namespace | `refs/trash/<id>` | Hidden from `git branch`, recoverable, prunable separately |
