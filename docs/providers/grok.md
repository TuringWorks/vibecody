---
layout: page
title: "Provider: Grok"
permalink: /providers/grok/
---

# Grok Provider

[Grok](https://x.ai) by xAI is a frontier AI model with strong reasoning and coding abilities, available through the xAI API.


## Get an API Key

1. Go to [console.x.ai](https://console.x.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export GROK_API_KEY="xai-..."
vibecli --provider grok
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[grok]
enabled = true
api_key = "xai-..."
model = "grok-3-mini"
```


## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `grok-2` | Strong reasoning and coding | General-purpose coding, debugging |
| `grok-3-mini` | Faster, lighter | Quick tasks, completions |

**Default:** `grok-3-mini`

Override from the CLI:

```bash
vibecli --provider grok --model grok-3-mini
```


## Verify Connection

```bash
vibecli --provider grok -c "Write a Python function to check if a string is a palindrome"
```


## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Verify the key starts with `xai-`
- Check that the key has not been revoked in [console.x.ai](https://console.x.ai)
- Confirm the env var is set: `echo $GROK_API_KEY`

### Rate limited

```
Error: 429 Too Many Requests
```

- xAI applies per-key rate limits
- Wait briefly and retry
