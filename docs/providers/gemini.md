---
layout: page
title: "Provider: Gemini"
permalink: /providers/gemini/
---

# Gemini Provider

[Google Gemini](https://ai.google.dev) is Google's multimodal AI model family. Gemini offers a generous free tier, making it an excellent choice for getting started without any cost.


## Get an API Key

1. Go to [Google AI Studio](https://aistudio.google.com)
2. Sign in with your Google account
3. Click **Get API Key** in the top navigation
4. Click **Create API key** and select a Google Cloud project (one is created automatically)
5. Copy the key

Your key will look like: `AIza...`


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export GEMINI_API_KEY="AIza..."
vibecli --provider gemini
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[gemini]
enabled = true
api_key = "AIza..."
model = "gemini-2.0-flash"
```

**Option 3: API key helper**

```toml
[gemini]
enabled = true
api_key_helper = "~/.vibecli/get-key.sh gemini"
model = "gemini-2.5-pro"
```


## Model Selection

| Model | Strengths | Context | Best for |
|-------|-----------|---------|----------|
| `gemini-2.5-pro` | Highest quality, deep reasoning | 1M tokens | Complex coding, architecture, analysis |
| `gemini-2.5-flash` | Fast with thinking capabilities | 1M tokens | Balanced quality and speed |
| `gemini-2.0-flash` | Very fast, cost-effective | 1M tokens | Quick tasks, high-volume use |

**Default:** `gemini-2.0-flash`

Override from the CLI:

```bash
vibecli --provider gemini --model gemini-2.5-pro
```


## Free Tier

Gemini offers one of the most generous free tiers among cloud providers:

| Model | Free rate limit | Free context |
|-------|-----------------|-------------|
| Gemini 2.0 Flash | 15 RPM, 1M TPM | 1M tokens |
| Gemini 2.5 Flash | 5 RPM, 250K TPM | 1M tokens |
| Gemini 2.5 Pro | 2 RPM, 250K TPM | 1M tokens |

RPM = requests per minute, TPM = tokens per minute.

The free tier is sufficient for personal coding use. For higher throughput, enable billing in Google Cloud Console.


## Paid Pricing

For usage beyond the free tier (check [ai.google.dev/pricing](https://ai.google.dev/pricing) for current rates):

| Model | Input (per 1M tokens) | Output (per 1M tokens) |
|-------|----------------------|------------------------|
| Gemini 2.5 Pro | $1.25 - $2.50 | $10.00 - $15.00 |
| Gemini 2.5 Flash | $0.15 - $0.30 | $0.60 - $3.50 |
| Gemini 2.0 Flash | $0.10 | $0.40 |

Pricing varies by context length (under/over 200K tokens).


## Multimodal Capabilities

Gemini natively supports image understanding. In VibeUI, you can:

- Paste screenshots of UI mockups and ask Gemini to generate code
- Share error screenshots for debugging
- Analyze architecture diagrams

This makes Gemini particularly useful for front-end development workflows.


## 1M Token Context Window

All Gemini models support a 1 million token context window, which means you can:

- Load entire codebases into context
- Analyze very large files without truncation
- Maintain long conversation histories

VibeCody's infinite context manager works especially well with Gemini's large context window.


## Verify Connection

```bash
vibecli --provider gemini -c "Say hello and identify yourself"
```


## Troubleshooting

### Invalid API key

```
Error: 400 - API key not valid
```

- Verify the key at [Google AI Studio](https://aistudio.google.com)
- Ensure the key starts with `AIza`
- Check that the associated Google Cloud project has the Generative Language API enabled

### Rate limited (free tier)

```
Error: 429 - Resource has been exhausted
```

- You have exceeded the free tier rate limit
- Wait 60 seconds for the rate limit to reset
- Consider enabling billing for higher limits
- Use `gemini-2.0-flash` for the most generous free-tier limits

### Region restrictions

Some regions may not have access to all Gemini models. If you encounter availability issues:

- Check [Google AI availability](https://ai.google.dev/available_regions) for your region
- Try a different model (Flash models have broader availability)
- Use a VPN if permitted by your organization

### API key vs. OAuth

VibeCody uses API key authentication (not OAuth). Ensure you are generating an **API key** from Google AI Studio, not an OAuth client credential from Google Cloud Console.

### Billing not enabled

```
Error: 403 - Billing account not found
```

If you exceed free-tier limits, you need to enable billing:

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Select the project linked to your API key
3. Navigate to **Billing** and link a billing account
