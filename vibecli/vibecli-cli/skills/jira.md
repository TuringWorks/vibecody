---
triggers: ["jira", "jira ticket", "jira issue", "sprint", "backlog", "jira comment", "create ticket", "jira board", "story points", "epic", "bug ticket"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Jira Integration

VibeCLI connects to Jira Cloud and Server via the REST API using `/jira` commands.

## Setup

```toml
# ~/.vibecli/config.toml
[jira]
url   = "https://yourorg.atlassian.net"
email = "you@example.com"
token = "ATATT3xxx"          # API token (not password)
```

Or set environment variables:
- `JIRA_URL` — base URL of your Jira instance
- `JIRA_EMAIL` — your Atlassian account email
- `JIRA_API_TOKEN` — API token from https://id.atlassian.com/manage-profile/security/api-tokens

## REPL Commands

| Command | Description |
|---------|-------------|
| `/jira list [project]` | List open issues (optional project key filter) |
| `/jira create <project> <summary> [desc]` | Create an issue (default type: Task) |
| `/jira get <issue-key>` | Read full issue details |
| `/jira comment <issue-key> <text>` | Add a comment to an issue |
| `/jira transition <issue-key> <status>` | Move issue to a status (e.g. "In Progress") |
| `/jira assign <issue-key> <email>` | Assign issue to a user |
| `/jira search <jql>` | Run a JQL query |
| `/jira sprint [board-id]` | List issues in the active sprint |
| `/jira mine` | Issues assigned to me |

## JQL Examples

```
/jira search "project = PROJ AND status = 'In Progress' AND assignee = currentUser()"
/jira search "created >= -7d AND type = Bug ORDER BY priority DESC"
/jira search "sprint in openSprints() AND status != Done"
```

## Effective Usage Patterns

1. **Sprint standup**: Run `/jira mine` to see your assigned issues, then `/jira sprint` for team context — no browser needed for the daily standup.
2. **Bug triage from logs**: When an error surfaces in the terminal, create a ticket immediately: `/jira create PROJ "NullPointerException in UserService.login" "Stack trace: ..."` — maintains context without context-switching.
3. **AI-generated descriptions**: Paste an error trace and ask the AI to write a Jira description — it includes steps to reproduce, expected vs actual behavior, and environment details.
4. **Status workflow**: Move tickets through workflow with `/jira transition PROJ-123 "In Review"` — equivalent to dragging on the board but scriptable.
5. **Comment from terminal**: During code review, add inline Jira comments: `/jira comment PROJ-456 "Approved PR #78 — merged to main"` keeps the ticket updated without switching to the browser.
6. **Cross-reference with Git**: Include the Jira issue key in commit messages (`git commit -m "PROJ-123: fix login null check"`). The Jira/GitHub integration then links commits to tickets automatically.
7. **Bulk updates**: Script `/jira transition` across a set of issue keys (e.g. move all "Done" issues to "Closed") by combining `/jira search` output with a shell loop.
8. **Epic grouping**: `/jira search "Epic Link = PROJ-10"` lists all stories under an epic — useful for release planning without opening Roadmaps.
9. **Server vs Cloud**: The same commands work for Jira Server (on-prem) by setting `url` to your internal URL. Basic auth with email+token works for both.
10. **Custom fields**: `/jira get <key>` returns all fields including custom ones. Reference custom field IDs in JQL: `cf[10014] = "High Value Customer"`.
