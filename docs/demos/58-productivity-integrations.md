---
layout: page
title: "Demo 58: Productivity Integrations"
permalink: /demos/58-productivity-integrations/
nav_order: 58
parent: Demos
---

## Overview

VibeCody is not just a coding assistant — it integrates directly with the tools you use every day: email (Gmail/Outlook), calendars (Google/Outlook), task management (Todoist), knowledge bases (Notion), project tracking (Jira), and smart home control (Home Assistant). This demo shows how to connect all six integrations and use them from the terminal alongside your development workflow.

**Time to complete:** 10–20 minutes

## Prerequisites

- VibeCLI installed (`vibecli --version`)
- At least one account to connect: Gmail, Outlook, Google Calendar, Todoist, Notion, Jira, or Home Assistant
- API keys / tokens for the services you want to connect (details below)

---

## Step 1: Configure Integrations

All integrations are configured in `~/.vibecli/config.toml`. Add only the sections for services you use.

```toml
# Email — Gmail OAuth2 token
[email]
provider     = "gmail"
access_token = "ya29.xxxx"

# Calendar — Google Calendar OAuth2 token
[calendar]
provider     = "google"
access_token = "ya29.xxxx"

# Home Assistant
[home_assistant]
url   = "http://homeassistant.local:8123"
token = "eyJ0..."

# Jira
[jira]
url   = "https://yourorg.atlassian.net"
email = "you@yourorg.com"
token = "ATATT3xxx"

# Notion + Todoist (top-level keys)
notion_api_key = "secret_xxxx"
todoist_api_key = "xxxx"
```

Alternatively, set environment variables instead of config file entries:

```bash
export GMAIL_ACCESS_TOKEN="ya29.xxxx"
export GOOGLE_CALENDAR_TOKEN="ya29.xxxx"
export HA_URL="http://homeassistant.local:8123"
export HA_TOKEN="eyJ0..."
export JIRA_URL="https://yourorg.atlassian.net"
export JIRA_EMAIL="you@yourorg.com"
export JIRA_API_TOKEN="ATATT3xxx"
export NOTION_API_KEY="secret_xxxx"
export TODOIST_API_KEY="xxxx"
```

---

## Step 2: Morning Briefing in 30 Seconds

Open a VibeCLI REPL session and run the morning briefing sequence:

```
$ vibecli chat

vibecli> /email unread
 📬 14 unread messages
  1. [alice@company.com] Re: Deploy plan for v2.1 — 9:02 AM
  2. [bot@github.com]     PR #892 review requested — 8:47 AM
  3. [jira@yourorg.com]   PROJ-234 assigned to you — 8:31 AM
  ...

vibecli> /cal today
 📅 Tuesday, April 4, 2026
  09:30  Engineering standup (30 min) — Google Meet
  11:00  1:1 with Alice (60 min)
  14:00  Architecture review — free until 14:00 (3.5h gap)
  16:00  Sprint planning (90 min)

vibecli> /todo today
 ✅  3 tasks due today
  [p1] PROJ-234: investigate slow login endpoint
  [p2] Write release notes for v2.1
  [p3] Review Alice's PR #891

vibecli> /jira mine
  PROJ-234  In Progress  Investigate slow login endpoint
  PROJ-228  To Do        Update API documentation
  PROJ-221  In Review    Refactor auth middleware
```

In four commands you have a complete picture of your day without opening a single browser tab.

---

## Step 3: Email Triage

```
vibecli> /email triage

 🤖 AI Triage Results (14 messages):
  🔴 URGENT (2):
    • alice@company.com: "Deploy plan for v2.1" — needs your approval before 11am
    • ops@company.com:   "Production alert: p99 latency spike" — requires investigation

  🟡 ACTION NEEDED (5):
    • PR #892 review requested — can review after standup
    • 3 Jira assignment notifications — already in your Jira queue

  🟢 FYI / ARCHIVE (7):
    • GitHub notifications (merged PRs, CI results)
    • Newsletter: "This Week in Rust"

  Suggested: archive 7 FYI messages? [y/N] y
  ✓ Archived 7 messages
```

---

## Step 4: Creating and Updating Jira Tickets from the Terminal

While investigating the production latency spike:

```
vibecli> /jira create PROJ "p99 latency spike in login endpoint — 2026-04-04" \
         "Observed in prod monitoring. p99 went from 80ms to 840ms at 08:15 UTC. \
          Affects /api/auth/login. Need to profile DB queries."

 ✓ Created PROJ-235 (Task)
   URL: https://yourorg.atlassian.net/browse/PROJ-235

vibecli> /jira transition PROJ-235 "In Progress"
 ✓ PROJ-235 moved to In Progress

vibecli> /jira comment PROJ-235 "Found root cause: missing index on users.last_login_at. \
         Adding migration now."
 ✓ Comment added
```

---

## Step 5: Smart Home During Focus Time

Before a deep work session, set the right environment:

```
vibecli> /ha scene focus
 ✓ Activated scene: focus
   • Office lights: warm white, 60% brightness
   • Thermostat: 70°F
   • Do Not Disturb: enabled

vibecli> /ha status
 🏠 Home Status
   Lights:     office (on, 60%), kitchen (off), bedroom (off)
   Climate:    70.0°F (target: 70°F) — cooling
   Doors:      front (locked), back (locked)
   Security:   armed (home mode)
```

---

## Step 6: Cross-Tool Workflows

### Log meeting notes to Notion automatically

```
vibecli> /notion search "Engineering standup notes"
 Found: "Engineering Standups 2026" (page-id: abc123)

vibecli> /notion append abc123 \
         "2026-04-04: Discussed latency spike (PROJ-235). Alice to review auth PR today."
 ✓ Appended to Engineering Standups 2026
```

### End-of-day task completion

```
vibecli> /todo close 1234567
 ✓ Completed: "PROJ-234: investigate slow login endpoint"

vibecli> /ha scene evening
 ✓ Activated scene: evening
   • All lights: warm white, 80% brightness
   • Thermostat: 72°F
```

---

## Available Commands Reference

| Category | Commands |
|----------|----------|
| **Email** | `/email inbox`, `unread`, `read <id>`, `send`, `search`, `triage`, `archive` |
| **Calendar** | `/cal today`, `week`, `list`, `create`, `delete`, `free`, `move`, `next`, `remind` |
| **Tasks** | `/todo list`, `today`, `add`, `close`, `delete`, `project`, `search`, `postpone` |
| **Notion** | `/notion search`, `get`, `create`, `databases`, `query`, `append` |
| **Jira** | `/jira list`, `create`, `get`, `comment`, `transition`, `assign`, `search`, `sprint`, `mine` |
| **Home** | `/ha status`, `lights`, `on`, `off`, `toggle`, `set`, `scene`, `climate`, `history`, `automation` |

---

## Skill Files

Each integration has a dedicated skill file with setup guides and usage patterns:

- `skills/email-automation.md` — Gmail & Outlook
- `skills/calendar-management.md` — Google Calendar & Outlook Calendar
- `skills/home-assistant.md` — Home Assistant smart home
- `skills/notion.md` — Notion workspace
- `skills/todoist.md` — Todoist task management
- `skills/jira.md` — Jira issue tracking

---

## What's Next

- **MCP exposure**: All six integrations are exposed as MCP tools, so Claude Desktop and other MCP clients can use your email, calendar, and tasks directly.
- **Agent workflows**: Use VibeCLI's agent loop to build cross-tool automations — e.g. "when a Jira ticket is assigned, add it to Todoist and send a Slack confirmation".
- **Voice control**: Pair with the voice interface (Demo 55) to control smart home and manage tasks hands-free.

See also: [Easy Setup](/demos/57-easy-setup/) · [Agent Loop](/demos/04-agent-loop/) · [Voice Pairing](/demos/55-voice-pairing-tailscale/)
