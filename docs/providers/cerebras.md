---
layout: page
title: "Provider: Cerebras"
permalink: /providers/cerebras/
---

# Cerebras Provider

[Cerebras](https://cerebras.ai) runs AI models on wafer-scale custom chips, delivering extremely fast inference for open-source models. Their hardware is purpose-built for AI workloads.

## Get an API Key

1. Go to [cloud.cerebras.ai](https://cloud.cerebras.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

Cerebras offers a free tier with limited usage.

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export CEREBRAS_API_KEY="..."
vibecli --provider cerebras
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[cerebras]
enabled = true
api_key = "..."
model = "llama3.1-70b"
```

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `llama3.1-70b` | Strong general coding | Daily coding tasks |
| `llama3.1-8b` | Ultra-fast | Quick completions, simple edits |

**Default:** `llama3.1-70b`

Override from the CLI:

```bash
vibecli --provider cerebras --model llama3.1-8b
```

## Best For

- **Ultra-fast inference** -- custom hardware delivers very low latency
- **Open-source models** -- access Llama models with blazing speed
- **Free tier** -- test fast inference without upfront costs

## Verify Connection

```bash
vibecli --provider cerebras -c "Write a Python class for a binary search tree"
```

## Troubleshooting

### Rate limited

```
Error: 429 Too Many Requests
```

- Free tier has usage limits
- Wait and retry, or upgrade for higher limits

### Model not available

- Check [cloud.cerebras.ai](https://cloud.cerebras.ai) for current model availability
