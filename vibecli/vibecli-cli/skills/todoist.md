---
triggers: ["todoist", "todo", "task list", "tasks today", "add task", "complete task", "task management", "inbox tasks", "due today"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Todoist Integration

VibeCLI connects to Todoist for task management via `/todo` (alias `/todoist`).

## Setup

```toml
# ~/.vibecli/config.toml
todoist_api_key = "xxxx"
```
Or set `TODOIST_API_KEY` environment variable.

Get your API token at https://todoist.com/prefs/integrations → API token.

## REPL Commands

| Command | Description |
|---------|-------------|
| `/todo list` | Show all active tasks |
| `/todo today` | Tasks due today or overdue |
| `/todo add <task> [due:<date>] [p:<1-4>]` | Add a task with optional due date and priority |
| `/todo close <task-id>` | Mark task complete |
| `/todo delete <task-id>` | Delete a task |
| `/todo project <name>` | List tasks in a project |
| `/todo search <query>` | Search tasks by keyword |
| `/todo postpone <task-id> <date>` | Reschedule a task |

## Priority Levels

- `p1` — Priority 1 (red, urgent)
- `p2` — Priority 2 (orange, high)
- `p3` — Priority 3 (blue, medium)
- `p4` — Priority 4 (no color, default)

## Due Date Syntax

Todoist's natural language due dates work directly:
- `due:today`, `due:tomorrow`, `due:friday`
- `due:next week`, `due:every day`, `due:every monday`
- `due:2026-04-10`

## Effective Usage Patterns

1. **Morning briefing**: Combine `/todo today` with `/cal today` and `/email unread` for a complete start-of-day view in one terminal session.
2. **Quick capture**: `/todo add "review PR #123" due:today p2` — faster than opening a browser tab, especially when deep in a terminal workflow.
3. **AI task generation**: Describe a project in natural language ("I need to deploy the new service") and ask the AI to generate a Todoist task list — it issues multiple `/todo add` calls automatically.
4. **Done tracking**: Use `/todo close` immediately when finishing work rather than batching — the timestamp is logged in Todoist's activity feed for productivity metrics.
5. **Project organization**: Use `/todo project "Work"` to scope views to a specific project instead of the noisy global inbox.
6. **Overdue triage**: `/todo today` shows overdue items first. Decide quickly: close (done), postpone (later), or delete (no longer relevant).
7. **Recurrence for habits**: `/todo add "Review pull requests" due:"every weekday" p2` creates a recurring habit task.
8. **Integration with Jira**: Add Jira ticket IDs in task names (`[PROJ-123]`) so `/todo list` gives you a cross-referenced view of what you're tracking across both systems.
9. **Filter syntax**: `/todo search @work` filters by label; `/todo search #Inbox` by project. Todoist filter syntax is passed through to the API.
10. **Sync latency**: Todoist API changes are typically reflected within 1 second. If a task doesn't appear after adding, wait briefly and run `/todo list` again.
