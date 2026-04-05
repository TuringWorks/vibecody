---
triggers: ["email", "gmail", "outlook", "inbox", "send email", "read email", "email triage", "unread emails", "email search"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Email Automation (Gmail & Outlook)

VibeCLI provides direct Gmail and Outlook integration via `/email` commands.

## Setup

**Gmail** — requires OAuth2 access token:
```toml
# ~/.vibecli/config.toml
[email]
provider = "gmail"
access_token = "ya29.xxxx"
```
Or set `GMAIL_ACCESS_TOKEN` environment variable.

**Outlook** — requires Microsoft Graph access token:
```toml
[email]
provider = "outlook"
access_token = "eyJ0..."
```
Or set `OUTLOOK_ACCESS_TOKEN` environment variable.

## REPL Commands

| Command | Description |
|---------|-------------|
| `/email inbox` | Show last 20 inbox messages |
| `/email unread` | List unread messages with sender/subject |
| `/email read <id>` | Read full message body |
| `/email send <to> <subject> <body>` | Send an email |
| `/email search <query>` | Search messages by keyword |
| `/email labels` | List Gmail labels or Outlook folders |
| `/email archive <id>` | Archive a message |
| `/email triage` | AI-assisted triage: auto-label and prioritize unread |

## Effective Usage Patterns

1. **Morning triage**: Run `/email triage` to have the AI scan unread messages, flag urgent items, and suggest responses — same pattern as Superhuman's AI triage.
2. **Inbox zero workflow**: Combine `/email unread` → read each with `/email read <id>` → action with archive or reply → repeat until count is 0.
3. **Search before sending**: Use `/email search` to find prior context in a thread before composing a reply to avoid duplicating information.
4. **Batch archiving**: Pass multiple IDs to `/email archive` in a loop — pipe the output of `/email unread` into a script that archives messages matching certain criteria.
5. **Smart send**: When using `/email send`, the AI composes the body if you describe the intent in natural language, e.g. `/email send alice@co.com "Project update" "let alice know the deployment is done and ask for sign-off"`.
6. **OAuth token refresh**: Access tokens expire. Store refresh tokens in the config and use `vibecli setup --email` to re-authorize. The system will warn 5 minutes before expiry.
7. **Multi-account**: Configure `[email.accounts]` array to switch between personal Gmail and work Outlook with `/email switch work`.
8. **Privacy**: Tokens are stored at `~/.vibecli/config.toml` (chmod 600). Never commit this file. Use `vibecli config --show` to verify permissions.
9. **Rate limits**: Gmail REST API allows 250 quota units/second per user. Bulk operations (triage over large inboxes) implement exponential backoff automatically.
10. **Attachments**: Use `/email read <id> --attachments` to list and download attachments to the current working directory.
