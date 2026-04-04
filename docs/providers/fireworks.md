---
layout: page
title: "Provider: Fireworks AI"
permalink: /providers/fireworks/
---

# Fireworks AI Provider

[Fireworks AI](https://fireworks.ai) provides fast inference for open-source models with optimized serving infrastructure and competitive pricing.


## Get an API Key

1. Go to [fireworks.ai](https://fireworks.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

Fireworks AI offers a free tier with limited usage.


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export FIREWORKS_API_KEY="..."
vibecli --provider fireworks
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[fireworks]
enabled = true
api_key = "..."
model = "accounts/fireworks/models/llama-v3p1-70b-instruct"
```


## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `accounts/fireworks/models/llama-v3p1-70b-instruct` | Strong general coding | Daily tasks (default) |
| `accounts/fireworks/models/llama-v3p1-8b-instruct` | Ultra-fast | Quick completions |
| `accounts/fireworks/models/mixtral-8x22b-instruct` | Large context | Longer analysis |

**Default:** `accounts/fireworks/models/llama-v3p1-70b-instruct`

Override from the CLI:

```bash
vibecli --provider fireworks --model "accounts/fireworks/models/llama-v3p1-8b-instruct"
```


## Best For

- **Fast inference** -- optimized serving for low-latency responses
- **Open-source models** -- access Llama, Mixtral, and other open models
- **Free tier** -- experiment before committing


## Verify Connection

```bash
vibecli --provider fireworks -c "Write a Go HTTP handler with middleware"
```


## Troubleshooting

### Invalid API key

- Check your key at [fireworks.ai](https://fireworks.ai)
- Confirm the env var is set: `echo $FIREWORKS_API_KEY`

### Model not found

- Use the full model path including `accounts/fireworks/models/` prefix
- Check [fireworks.ai/models](https://fireworks.ai/models) for current model names
