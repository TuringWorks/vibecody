---
triggers: ["daily prep", "task prep", "morning prep", "prepare tasks", "prepare today", "seed today", "daily task prep", "start of day", "morning tasks", "task list prep"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Daily Task Prep

Use `clawchief/tasks.md` as the canonical live task file and `clawchief/tasks-completed.md` as the completed-task archive. Typically triggered by a cron or direct request before the day starts.

## Core rules

- Read `clawchief/priority-map.md` before regrouping or inserting active tasks.
- Preserve existing manually added open tasks in `## Today` unless they are obviously stale.
- On weekdays, treat `## Every weekday` as the recurring seed list.
- On weekends, do not auto-add `## Every weekday` items unless explicitly asked.
- Promote due-today items from `## Backlog with due date` into `## Today`, removing the backlog copy in the same edit.
- Scan `## Recurring reminders` and add any due today into `## Today` without deleting the source item.
- Add principal-owned meetings and calls for today to `## Today`.
- Exclude personal / family calendar blocks that are only conflict sources.
- Keep assistant tasks clearly separate from principal tasks.
- Archive tasks completed yesterday out of `clawchief/tasks.md` into `clawchief/tasks-completed.md`.
- Keep tasks completed today in `clawchief/tasks.md` until the next morning's prep run.
- Update the file's `Last updated` timestamp.
- Stay silent unless something needs human attention.

## Preparation workflow

1. Read `clawchief/tasks.md`.
2. Read `clawchief/priority-map.md`.
3. Read `clawchief/tasks-completed.md` if it exists.
4. Determine whether today is a weekday.
5. Archive tasks completed yesterday into `clawchief/tasks-completed.md`.
6. Build the candidate `## Today` list from:
   - current open tasks
   - weekday recurring items (Mon–Fri)
   - due-today backlog items
   - due-today recurring reminders
   - today's principal-owned calendar events
7. Remove duplicates by normalized task text.
8. Preserve or assign each task to the best matching owner section + program/person grouping header.
9. Reorder open tasks in priority-first order within each owner section.
10. Write back only the minimal necessary edits.

## Calendar workflow

```bash
gog calendar events --all -a {{ASSISTANT_EMAIL}} --days=1 --max=100 --json --results-only
```

Only add calendar items the principal is actually expected to attend.

## Safety rules

- Do not wipe `## Today` just to rebuild it.
- Do not archive recurring source items from `## Recurring reminders`.
- Do not archive tasks completed today during the same day's prep run.
- If calendar access fails, still do file-based prep and only notify if the failure matters.
- If nothing needs to change, do nothing.
