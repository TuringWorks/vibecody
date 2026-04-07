---
triggers: ["task list", "todo", "add task", "complete task", "remove task", "defer task", "reprioritize", "task manager", "task summary", "tasks for today", "what's left", "open tasks", "task status"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Daily Task Manager

Use `clawchief/tasks.md` as the canonical live task list and `clawchief/tasks-completed.md` as the completed-task archive.

## Core rules

1. Read `clawchief/tasks.md` before answering any questions about current tasks.
2. Treat `clawchief/tasks.md` as the source of truth across all sessions.
3. When task state changes, update `clawchief/tasks.md` in the same turn whenever practical.
4. When given a task with a due date, add an assistant-owned task with canonical due-date format.
5. When a task depends on an outside reply or future check-in, add a separate follow-up task with its own due date.
6. Scan for overdue and due-today assistant tasks before deciding what needs attention.
7. Keep long-term preferences in memory files, live operational state in `clawchief/tasks.md`, prior-day completed history in `clawchief/tasks-completed.md`.
8. If a task change materially affects heartbeat behavior, update the heartbeat instructions.
9. Use `YYYY-MM-DD` for all-day due dates and `YYYY-MM-DD HH:MM TZ` for timed due dates.

## File structure

Maintain these sections in `clawchief/tasks.md`:

- `## Today`
- `## Every weekday`
- `## Backlog with due date`
- `## Recurring reminders`
- `## Backlog`
- `## Rules`

Within `## Today`, use owner sections `### Principal` and `### Assistant`, grouped under `#### <program or person>` or `#### Other / uncategorized`.

## Update workflow

- **Add task**: add to `## Today` or `## Backlog with due date`; remove older backlog copy when promoting; add follow-up task if a later nudge is needed.
- **Complete task**: change to `- [x]`; preserve completion timestamp; leave same-day completions in place until next daily prep.
- **Priority change**: reorder open tasks so highest-priority work is first within each section.
- **User asks what's left**: report only open tasks unless they ask for completed work too.

## Heartbeat behavior

When a heartbeat includes task follow-up: read `clawchief/tasks.md`, ask about open tasks only, do not ask about tasks already marked done, keep the message short and direct.
