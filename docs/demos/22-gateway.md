---
layout: page
title: "Demo 22: Gateway Messaging"
permalink: /demos/gateway/
nav_order: 22
parent: Demos
---


## Overview

The VibeCody Gateway connects your AI assistant to 18 messaging platforms simultaneously. You can chat with your AI agent from Telegram, Discord, Slack, or any supported platform, and it responds with full context awareness -- reading your project files, running commands, and applying code changes just as if you were using the REPL directly. The gateway supports platform-specific features like markdown rendering, emoji reactions, threaded conversations, and file attachments.

**Time to complete:** ~15 minutes

## Supported Platforms (18)

| Platform | Auth Method | Key Features |
|----------|-------------|--------------|
| Telegram | Bot token | Markdown, inline keyboards, file upload |
| Discord | Bot token + OAuth2 | Embeds, reactions, threads, slash commands |
| Slack | App token + Bot token | Blocks, threads, reactions, slash commands |
| Signal | Signal CLI | End-to-end encrypted |
| Matrix | Access token | Federated, E2EE rooms |
| Twilio SMS | Account SID + Auth token | SMS/MMS, phone number routing |
| iMessage | AppleScript bridge | macOS only, blue bubble |
| WhatsApp | Business API | Templates, media, read receipts |
| Teams | App registration | Adaptive cards, tabs |
| IRC | Server + nick | Standard IRC protocol |
| Twitch | OAuth token | Chat commands, whispers |
| WebChat | Built-in HTTP server | Embeddable widget |
| Nostr | Private key (nsec) | Decentralized, relays |
| QQ | Bot appid + secret | Groups, rich messages |
| Tlon | Ship name | Urbit-native messaging |

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- API credentials for the platforms you want to connect
- (Optional) VibeUI installed for the Gateway panel

## Step-by-Step Walkthrough

### Step 1: Configure a Telegram bot

First, create a bot via Telegram's BotFather and obtain a token.

Add the token to your VibeCLI config:

```bash
vibecli --config set gateway.telegram.token "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
```

Or edit `~/.vibecli/config.toml` directly:

```toml
[gateway.telegram]
token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
allowed_users = [12345678, 87654321]   # Telegram user IDs (security)
project_dir = "/path/to/your/project"   # Working directory for AI
```

### Step 2: Configure a Discord bot

Create an application at the Discord Developer Portal, add a bot, and copy the token.

```bash
vibecli --config set gateway.discord.token "MTIzNDU2Nzg5MDEyMzQ1Njc4OQ.XXXXXX.XXXXX"
vibecli --config set gateway.discord.guild_id "987654321012345678"
```

Or in `config.toml`:

```toml
[gateway.discord]
token = "MTIzNDU2Nzg5MDEyMzQ1Njc4OQ.XXXXXX.XXXXX"
guild_id = "987654321012345678"
allowed_channels = ["dev-chat", "ai-assistant"]
prefix = "!vibe"
```

### Step 3: Configure a Slack app

Create a Slack app at api.slack.com, install it to your workspace, and copy the tokens.

```toml
[gateway.slack]
app_token = "xapp-1-A0123456789-..."
bot_token = "xoxb-..."
allowed_channels = ["C0123456789"]
```

### Step 4: Start the gateway

Launch the gateway with one or more platforms:

```bash
vibecli
```

```
/gateway start --platforms telegram,discord,slack
```

```
Gateway starting...
  [telegram] Connected as @VibeCodyBot (polling)
  [discord]  Connected to guild "My Server" (websocket)
  [slack]    Connected to workspace "My Team" (socket mode)

Gateway active on 3 platforms. Waiting for messages...
```

To start all configured platforms at once:

```
/gateway start --all
```

### Step 5: Interact from Telegram

Send a message to your Telegram bot:

```
You: Explain the auth middleware in src/middleware/auth.rs

Bot: The auth middleware in `src/middleware/auth.rs` implements JWT
     token validation. Here's how it works:

     1. Extracts the `Authorization` header
     2. Validates the JWT signature using the configured secret
     3. Decodes the claims and attaches the user to the request context
     4. Returns 401 if the token is expired or invalid

     Key function: `validate_token()` at line 45
```

The bot reads the actual file from your project directory and responds with context-aware analysis.

### Step 6: Use platform-specific features

**Discord embeds:**

The bot formats code blocks, uses embeds for long responses, and adds reaction-based confirmations:

```
!vibe fix the failing test in tests/api_test.rs

[Embed: Code Fix Applied]
File: tests/api_test.rs
Lines changed: 23-27
React with Yes to apply, No to reject
```

**Slack threads:**

Conversations in Slack are threaded automatically. Each new question starts a thread; follow-ups stay in the same thread for context continuity.

**Telegram inline keyboards:**

For multi-step operations, the bot presents inline keyboard buttons:

```
I found 3 potential fixes for this issue:
[Option A: Add null check]  [Option B: Use default value]  [Option C: Refactor method]
```

### Step 7: Check gateway status

```
/gateway status
```

```
Gateway Status:
  Platform    Status     Messages    Uptime
  telegram    ACTIVE     47          2h 15m
  discord     ACTIVE     23          2h 15m
  slack       ACTIVE     31          2h 15m

  Total messages processed: 101
  AI provider: claude (claude-sonnet-4-20250514)
  Token usage: 45,230 / 100,000 budget
```

### Step 8: Stop a specific platform or all

```
/gateway stop --platform telegram
```

```
[telegram] Disconnected. 47 messages processed.
Gateway still active on: discord, slack
```

To stop everything:

```
/gateway stop --all
```

### Step 9: Multi-platform simultaneous operation

The gateway multiplexes across all active platforms. A message from any platform triggers the same AI pipeline. Session context is isolated per user per platform, so conversations do not bleed across platforms.

```
[telegram] alice: "What's in the latest commit?"
  -> AI reads git log, responds in Telegram

[discord]  bob: "Run the test suite"
  -> AI runs cargo test, responds in Discord with embed

[slack]    carol: "Review PR #42"
  -> AI fetches PR diff, responds in Slack thread
```

### Step 10: Using the Gateway panel in VibeUI

Open the **Gateway** panel from the sidebar.

1. **Platforms Tab** -- Toggle platforms on/off, view connection status, configure credentials.
2. **Messages Tab** -- Live feed of messages across all platforms with platform icons.
3. **Settings Tab** -- Set global AI provider, token budget, allowed users, and project directory.

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/gateway start --platforms <list>` | Start gateway on specified platforms |
| `/gateway start --all` | Start all configured platforms |
| `/gateway stop --platform <name>` | Stop a specific platform |
| `/gateway stop --all` | Stop all platforms |
| `/gateway status` | Show status of all platforms |
| `vibecli --config set gateway.<platform>.<key> <value>` | Configure platform credentials |

## Demo Recording

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "Gateway Messaging Demo",
    "description": "Connect VibeCody to Telegram, Discord, and Slack simultaneously and interact with your AI assistant from any platform",
    "duration_seconds": 270,
    "steps": [
      {
        "timestamp": 0,
        "action": "config_edit",
        "file": "~/.vibecli/config.toml",
        "section": "gateway.telegram",
        "content": "token = \"123456:ABC-DEF...\"",
        "narration": "Configure Telegram bot credentials"
      },
      {
        "timestamp": 15,
        "action": "config_edit",
        "file": "~/.vibecli/config.toml",
        "section": "gateway.discord",
        "content": "token = \"MTIzNDU2...\"",
        "narration": "Configure Discord bot credentials"
      },
      {
        "timestamp": 30,
        "action": "config_edit",
        "file": "~/.vibecli/config.toml",
        "section": "gateway.slack",
        "content": "app_token = \"xapp-...\"",
        "narration": "Configure Slack app credentials"
      },
      {
        "timestamp": 45,
        "action": "repl_command",
        "command": "/gateway start --platforms telegram,discord,slack",
        "output": "Gateway starting...\n  [telegram] Connected as @VibeCodyBot\n  [discord]  Connected to guild \"My Server\"\n  [slack]    Connected to workspace \"My Team\"\n\nGateway active on 3 platforms.",
        "narration": "Start the gateway on all three platforms"
      },
      {
        "timestamp": 75,
        "action": "platform_message",
        "platform": "telegram",
        "user": "alice",
        "message": "Explain the auth middleware in src/middleware/auth.rs",
        "response": "The auth middleware implements JWT validation...",
        "narration": "A user sends a question from Telegram -- AI reads the file and responds"
      },
      {
        "timestamp": 110,
        "action": "platform_message",
        "platform": "discord",
        "user": "bob",
        "message": "!vibe run the test suite",
        "response": "[Embed] Test Results: 142 passed, 0 failed",
        "narration": "A Discord user triggers a test run via the bot prefix"
      },
      {
        "timestamp": 145,
        "action": "platform_message",
        "platform": "slack",
        "user": "carol",
        "message": "Review PR #42",
        "response": "[Thread] PR #42 Review: 3 files changed...",
        "narration": "A Slack user requests a PR review -- response is threaded"
      },
      {
        "timestamp": 180,
        "action": "repl_command",
        "command": "/gateway status",
        "output": "Gateway Status:\n  telegram  ACTIVE  47 msgs  2h 15m\n  discord   ACTIVE  23 msgs  2h 15m\n  slack     ACTIVE  31 msgs  2h 15m\n\nTotal: 101 messages",
        "narration": "Check gateway status across all platforms"
      },
      {
        "timestamp": 210,
        "action": "ui_interaction",
        "panel": "Gateway",
        "tab": "Messages",
        "action_detail": "view_live_feed",
        "narration": "View the live cross-platform message feed in VibeUI"
      },
      {
        "timestamp": 240,
        "action": "repl_command",
        "command": "/gateway stop --all",
        "output": "Gateway stopped.\n  telegram: 47 messages\n  discord: 23 messages\n  slack: 31 messages",
        "narration": "Stop all gateway platforms"
      }
    ]
  }
}
```

## What's Next

- [Demo 23: Test Runner & Coverage](../test-coverage/) -- AI-powered test generation with coverage tracking
- [Demo 24: Red Team Security](../red-team/) -- Automated security scanning with OWASP checks
- Use the gateway with agent teams to let team agents report progress to your Slack channel
