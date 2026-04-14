---
layout: page
title: "Provider: MiniMax"
permalink: /providers/minimax/
---

# MiniMax Provider

[MiniMax](https://www.minimaxi.com) is a Chinese AI company offering large language models through their API platform.

## Get an API Key

1. Go to [api.minimax.chat](https://api.minimax.chat)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export MINIMAX_API_KEY="..."
vibecli --provider minimax
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[minimax]
enabled = true
api_key = "..."
model = "abab6.5s-chat"
```

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `abab6.5s-chat` | General-purpose | Daily tasks (default) |
| `abab6.5-chat` | Stronger reasoning | Complex tasks |

**Default:** `abab6.5s-chat`

## Best For

- **Chinese language tasks** -- strong Chinese language understanding
- **Alternative provider** -- diversify across model families

## Verify Connection

```bash
vibecli --provider minimax -c "Write a Python function to validate email addresses"
```

## Troubleshooting

### Authentication error

- Verify your key at [api.minimax.chat](https://api.minimax.chat)
- Confirm the env var is set: `echo $MINIMAX_API_KEY`
