---
layout: page
title: "Tutorial: Always-On Channel Daemon"
permalink: /tutorials/channel-daemon/
---

# Always-On Channel Daemon

Run VibeCody as a persistent bot that listens on Slack, Discord, or GitHub webhooks and autonomously handles tasks — like Claude Code Channels or Cursor Automations.

**Prerequisites:** VibeCody installed, a provider configured, and a platform bot token.

## What Is the Channel Daemon?

The channel daemon is an always-on process that:

1. **Listens** on messaging platforms (Slack, Discord, Telegram, GitHub webhooks)
2. **Routes** incoming messages through automation rules
3. **Spawns** agent tasks for matching messages
4. **Falls back** to conversational chat when no rules match
5. **Maintains** session affinity (multi-turn conversations per user)

Unlike the basic `--gateway` mode (which does simple chat), the channel daemon supports concurrent agent execution, automation rules, and session management.

## Quick Start

### Step 1: Get a Bot Token

**Slack:**

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → Create New App
2. Under "OAuth & Permissions", add scopes: `chat:write`, `app_mentions:read`, `channels:history`
3. Install to workspace → copy Bot User OAuth Token

**Discord:**

1. Go to [discord.com/developers](https://discord.com/developers/applications) → New Application
2. Under "Bot", click "Add Bot" → copy token
3. Under "OAuth2", generate invite URL with `bot` scope

**Telegram:**

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot`, follow prompts → copy token

### Step 2: Set Environment Variable

```bash
# Choose one:
export SLACK_BOT_TOKEN="xoxb-your-token"
export DISCORD_BOT_TOKEN="your-token"
export TELEGRAM_BOT_TOKEN="123456:ABC-your-token"
```

### Step 3: Start the Daemon

```bash
# Start with Slack
vibecli --channel-daemon slack

# Or Discord
vibecli --channel-daemon discord

# Or Telegram
vibecli --channel-daemon telegram
```

Output:

```
[channel-daemon] Starting enhanced daemon on slack
[channel-daemon] Automation rules: .vibecli/automations/
[channel-daemon] Max concurrent tasks: 4
[daemon] Starting enhanced channel daemon on slack
[daemon] Max concurrent tasks: 4
```

The daemon runs indefinitely, polling for messages every 2 seconds.

## Automation Rules

Create `.vibecli/automations/` with TOML rule files to auto-trigger agent tasks:

### Example: Auto-Review PRs

```toml
# .vibecli/automations/pr-review.toml
[rule]
id = "auto-pr-review"
name = "Auto-Review Pull Requests"
enabled = true

[trigger]
source = "GitHub"
events = ["pull_request"]

[filter]
required_fields = ["pr_number", "pr_title"]

[action]
prompt_template = "Review PR #{{pr_number}}: {{pr_title}}. Read the diff, check for bugs, security issues, and style problems. Provide a concise review."
approval_policy = "full-auto"
max_steps = 15
```

### Example: Bug Reports → Auto-Fix

```toml
# .vibecli/automations/bug-fix.toml
[rule]
id = "auto-bug-fix"
name = "Auto-Fix Bug Reports"
enabled = true

[trigger]
source = "Slack"
events = ["message"]

[filter]
keywords = ["bug", "error", "crash", "broken"]

[action]
prompt_template = "A user reported: {{content}}. Investigate and fix the issue."
approval_policy = "auto-edit"
max_steps = 20
```

### Example: Incident Response

```toml
# .vibecli/automations/incident.toml
[rule]
id = "incident-response"
name = "PagerDuty Incident Response"
enabled = true

[trigger]
source = "PagerDuty"
events = ["incident.trigger"]

[filter]
severity = ["critical", "high"]

[action]
prompt_template = "INCIDENT: {{content}}. Check logs, identify root cause, and prepare a fix."
approval_policy = "suggest"
max_steps = 30
```

## Session Management

The daemon maintains per-user sessions with conversation history:

- Each `channel:user` pair gets its own session
- Sessions persist across multiple messages (multi-turn)
- Sessions timeout after 30 minutes of inactivity (configurable)
- History is capped at 50 messages to prevent unbounded growth

## Managing the Daemon from REPL

You can also manage the daemon from the interactive REPL:

```bash
vibecli
> /daemon status          # Show daemon state
> /daemon start           # Start listening
> /daemon stop            # Stop the daemon
> /daemon channels        # List supported platforms
> /daemon logs            # Show recent event log
```

## Configuration

Full daemon configuration in `~/.vibecli/config.toml`:

```toml
[channel_daemon]
port = 7879                      # Webhook server port
max_concurrent_sessions = 8      # Max parallel agent tasks
rate_limit_per_channel = 60      # Messages per minute per channel
health_check_interval_secs = 30  # Health log interval
auto_restart = true              # Restart on crash

[[channel_daemon.channels]]
platform = "slack"
auto_respond = true              # Respond to all messages (not just @mentions)
max_concurrent = 3               # Per-channel concurrency limit
approval_policy = "auto-edit"    # Agent approval mode
session_timeout_secs = 1800      # 30 minute session timeout
```

## Security

- **Allowed users** — whitelist specific users via `allowed_users` in gateway config
- **Command blocklist** — dangerous commands are blocked (same as interactive mode)
- **SSRF protection** — agent URL fetching blocks internal IPs
- **Rate limiting** — configurable per-channel rate limits prevent abuse
- **Session isolation** — each user's session is independent
