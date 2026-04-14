---
layout: page
title: "Provider: GitHub Copilot"
permalink: /providers/copilot/
---

# GitHub Copilot Provider

[GitHub Copilot](https://github.com/features/copilot) provides AI-powered coding assistance through GitHub's infrastructure. VibeCody can use your existing Copilot subscription as an AI provider.

## Prerequisites

1. A GitHub account with an active Copilot subscription (Individual, Business, or Enterprise)
2. A GitHub personal access token (PAT) with the `copilot` scope

## Get a Token

1. Go to [github.com/settings/tokens](https://github.com/settings/tokens)
2. Click **Generate new token** (classic) or **Fine-grained token**
3. For classic tokens: select the `copilot` scope
4. Copy the token

VibeCody exchanges this GitHub token for a short-lived Copilot API token automatically (refreshed every ~30 minutes).

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export GITHUB_TOKEN="ghp_..."
vibecli --provider copilot
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[copilot]
enabled = true
api_key = "ghp_..."
model = "gpt-4o"
```

## Model Selection

GitHub Copilot provides access to models through its API:

| Model | Best for |
|-------|----------|
| `gpt-4o` | General-purpose coding (default) |

**Default:** `gpt-4o`

## Best For

- **Existing Copilot subscribers** -- use your subscription for VibeCody without additional API costs
- **GitHub-integrated workflows** -- works with your existing GitHub authentication
- **No separate billing** -- included in your Copilot subscription

## Verify Connection

```bash
vibecli --provider copilot -c "Write a JavaScript function to debounce events"
```

## Troubleshooting

### Token exchange failed

```
Error: Failed to exchange GitHub token
```

- Verify your GitHub token is valid and has the `copilot` scope
- Confirm you have an active Copilot subscription
- Check: `curl -H "Authorization: token ghp_..." https://api.github.com/user`

### Expired token

VibeCody caches the Copilot API token and refreshes it automatically. If you see authentication errors, try restarting VibeCody to force a token refresh.

### No Copilot subscription

The Copilot provider requires an active GitHub Copilot subscription. Sign up at [github.com/features/copilot](https://github.com/features/copilot).
