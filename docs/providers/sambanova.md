---
layout: page
title: "Provider: SambaNova"
permalink: /providers/sambanova/
---

# SambaNova Provider

[SambaNova](https://sambanova.ai) runs AI models on custom RDU (Reconfigurable Dataflow Unit) hardware, delivering fast inference for open-source models.


## Get an API Key

1. Go to [cloud.sambanova.ai](https://cloud.sambanova.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

SambaNova offers a free tier with limited usage.


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export SAMBANOVA_API_KEY="..."
vibecli --provider sambanova
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[sambanova]
enabled = true
api_key = "..."
model = "Meta-Llama-3.1-70B-Instruct"
```


## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `Meta-Llama-3.1-70B-Instruct` | Strong general coding | Daily tasks (default) |
| `Meta-Llama-3.1-8B-Instruct` | Ultra-fast | Quick completions |

**Default:** `Meta-Llama-3.1-70B-Instruct`

Override from the CLI:

```bash
vibecli --provider sambanova --model Meta-Llama-3.1-8B-Instruct
```


## Best For

- **Fast inference** -- custom RDU hardware optimized for AI workloads
- **Open-source models** -- access Llama models with high throughput
- **Free tier** -- test fast inference without upfront costs


## Verify Connection

```bash
vibecli --provider sambanova -c "Write a Python async web scraper"
```


## Troubleshooting

### Rate limited

```
Error: 429 Too Many Requests
```

- Free tier has usage limits
- Wait and retry, or upgrade for higher limits

### Model not available

- Check [cloud.sambanova.ai](https://cloud.sambanova.ai) for current model availability
