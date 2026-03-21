---
layout: page
title: "Provider: OpenAI"
permalink: /providers/openai/
---

# OpenAI Provider

[OpenAI](https://openai.com) provides GPT-4o and the o-series reasoning models. Widely used, well-documented, and available in most regions.

---

## Get an API Key

1. Go to [platform.openai.com](https://platform.openai.com)
2. Sign in or create an account
3. Navigate to **API Keys** in the left sidebar
4. Click **Create new secret key** and copy it

Your key will look like: `sk-proj-...` or `sk-...`

---

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export OPENAI_API_KEY="sk-proj-..."
vibecli --provider openai
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[openai]
enabled = true
api_key = "sk-proj-..."
model = "gpt-4o"
```

**Option 3: API key helper** (for key rotation / vault integration)

```toml
[openai]
enabled = true
api_key_helper = "~/.vibecli/get-key.sh openai"
model = "gpt-4o"
```

---

## Model Selection

| Model | Strengths | Context | Best for |
|-------|-----------|---------|----------|
| `gpt-4o` | Best overall quality, multimodal | 128K tokens | Daily coding, code review, generation |
| `gpt-4o-mini` | Fast and affordable | 128K tokens | Quick tasks, completions, simple edits |
| `o3` | Advanced reasoning | 200K tokens | Complex debugging, architecture, math |
| `o3-mini` | Fast reasoning | 200K tokens | Moderate reasoning at lower cost |
| `o4-mini` | Latest compact reasoning | 200K tokens | Balanced reasoning and speed |

**Default:** `gpt-4o`

Override from the CLI:

```bash
vibecli --provider openai --model o3
```

---

## Pricing

Pricing as of early 2026 (check [openai.com/pricing](https://openai.com/pricing) for current rates):

| Model | Input (per 1M tokens) | Output (per 1M tokens) |
|-------|----------------------|------------------------|
| GPT-4o | $2.50 | $10.00 |
| GPT-4o mini | $0.15 | $0.60 |
| o3 | $10.00 | $40.00 |
| o3-mini | $1.10 | $4.40 |

**Tip:** Use GPT-4o mini for quick questions and simple tasks. Use GPT-4o for complex coding. Reserve o3 for hard reasoning problems.

---

## Custom API URL

For Azure OpenAI, proxies, or compatible APIs (e.g., local LLM servers with OpenAI-compatible endpoints):

```toml
[openai]
enabled = true
api_url = "https://my-proxy.example.com/v1"
model = "gpt-4o"
```

For Azure OpenAI specifically, use the dedicated Azure provider instead:

```toml
[azure_openai]
enabled = true
api_key = "..."
api_url = "https://<resource>.openai.azure.com"
model = "gpt-4o"
```

---

## Verify Connection

```bash
vibecli --provider openai -c "Say hello and identify yourself"
```

---

## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Verify the key starts with `sk-` and has not been revoked
- Check at [platform.openai.com/api-keys](https://platform.openai.com/api-keys)
- If using an env var, confirm it is exported: `echo $OPENAI_API_KEY`

### Insufficient quota

```
Error: 429 - You exceeded your current quota
```

- Add a payment method at [platform.openai.com/account/billing](https://platform.openai.com/account/billing)
- Check your usage limits and increase them if needed
- New accounts may have a low spending cap

### Rate limited

```
Error: 429 Too Many Requests
```

- OpenAI applies per-key rate limits (tokens per minute and requests per minute)
- Wait a moment and retry, or upgrade your usage tier
- Consider using GPT-4o mini for high-volume tasks

### Model not available

```
Error: 404 - model not found
```

- Some models require specific access (e.g., o3 may require tier 5)
- Check your available models at [platform.openai.com/docs/models](https://platform.openai.com/docs/models)
- Verify the model name is spelled correctly

### Timeout

```
Error: request timed out
```

- o3 reasoning requests can take 30-120 seconds for complex tasks
- Check your network connection
- If behind a corporate proxy, configure `api_url`
