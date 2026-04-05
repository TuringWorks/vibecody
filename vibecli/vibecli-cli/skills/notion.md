---
triggers: ["notion", "notion page", "notion database", "notion search", "knowledge base", "notion workspace", "notion blocks"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Notion Integration

VibeCLI connects to Notion via the official API for search, reading, and creating pages.

## Setup

```toml
# ~/.vibecli/config.toml
notion_api_key = "secret_xxxx"
```
Or set `NOTION_API_KEY` environment variable.

Create an integration at https://www.notion.so/my-integrations, then share the pages/databases you want VibeCLI to access with that integration.

## REPL Commands

| Command | Description |
|---------|-------------|
| `/notion search <query>` | Full-text search across workspace |
| `/notion get <page-id>` | Read a page's content as plain text |
| `/notion create <parent-id> <title>` | Create a new page under a parent |
| `/notion databases` | List accessible databases |
| `/notion query <db-id> [filter]` | Query a database with optional filter |
| `/notion append <page-id> <text>` | Append a paragraph block to a page |

## Effective Usage Patterns

1. **Knowledge retrieval**: `/notion search "sprint retrospective"` finds all pages matching the query — use it like a CLI grep over your entire Notion workspace.
2. **Meeting notes**: After a meeting, use `/notion append <meeting-notes-page-id> "key decisions: ..."` to log outcomes without opening the browser.
3. **Project status**: Query a project database `/notion query <db-id> "status=In Progress"` to get a terminal-friendly list of active work items.
4. **AI-powered summarization**: Pipe `/notion get <page-id>` output into the AI to summarize a long design doc: "summarize this for a 2-minute standup".
5. **Cross-tool linking**: Combine Jira and Notion — find the Jira ticket with `/jira list`, then log the ticket URL in the corresponding Notion project page with `/notion append`.
6. **Template pages**: Create a template page ID in config. Use `/notion create <template-id> "2026-04-04 Daily Notes"` to spin up a new page from the template layout.
7. **Offline-first**: Notion pages are fetched fresh each time. For frequently accessed reference pages, use `/notion get <id> > ~/notes/reference.md` to cache locally.
8. **Permissions**: If a search returns 0 results for known pages, verify the integration was shared with those pages. Notion's API only returns pages explicitly shared with the integration.
9. **Block types**: The `/notion get` command renders headings, paragraphs, bullets, to-dos (with ✓/☐), and code blocks. Toggle blocks are expanded inline.
10. **Rate limits**: Notion API allows 3 requests/second. Bulk operations (querying large databases) are automatically throttled with a 400ms delay between requests.
