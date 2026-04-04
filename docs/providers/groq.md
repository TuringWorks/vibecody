---
layout: page
title: "Provider: Groq"
permalink: /providers/groq/
---

# Groq Provider

[Groq](https://groq.com) runs open-source models on custom LPU (Language Processing Unit) hardware, delivering extremely low latency inference -- often 10-20x faster than cloud GPU providers.


## Get an API Key

1. Go to [console.groq.com](https://console.groq.com)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

Groq offers a free tier with rate limits.


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export GROQ_API_KEY="gsk_..."
vibecli --provider groq
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[groq]
enabled = true
api_key = "gsk_..."
model = "llama-3.3-70b-versatile"
```


## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `llama-3.3-70b-versatile` | Strong general coding | Daily coding tasks |
| `llama-3.1-8b-instant` | Ultra-fast responses | Quick completions, simple edits |
| `mixtral-8x7b-32768` | Good balance, 32K context | Longer code analysis |

**Default:** `llama-3.3-70b-versatile`

Override from the CLI:

```bash
vibecli --provider groq --model llama-3.1-8b-instant
```


## Pricing

Groq offers a generous free tier with rate limits. Paid plans remove rate limits and add priority access.


## Best For

- **Ultra-fast iteration** -- responses arrive in under a second
- **Interactive coding sessions** -- low latency makes back-and-forth feel instant
- **Running open-source models** -- access Llama, Mixtral without hosting them yourself


## Verify Connection

```bash
vibecli --provider groq -c "Write a Go function to reverse a linked list"
```


## Troubleshooting

### Rate limited on free tier

```
Error: 429 Too Many Requests
```

- Free tier has per-minute and per-day token limits
- Wait 60 seconds and retry, or upgrade to a paid plan

### Model not available

- Groq's model catalog changes; check [console.groq.com/docs/models](https://console.groq.com/docs/models) for current availability
