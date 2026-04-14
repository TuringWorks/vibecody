---
layout: page
title: "Provider: OpenRouter"
permalink: /providers/openrouter/
---

# OpenRouter Provider

[OpenRouter](https://openrouter.ai) is a unified API gateway that provides access to 300+ models from multiple providers (OpenAI, Anthropic, Google, Meta, Mistral, and more) through a single API key and billing account.

## Get an API Key

1. Go to [openrouter.ai/keys](https://openrouter.ai/keys)
2. Create an account or sign in
3. Create a new key and copy it

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
vibecli --provider openrouter
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[openrouter]
enabled = true
api_key = "sk-or-v1-..."
model = "anthropic/claude-3.5-sonnet"
```

## Model Selection

OpenRouter gives you access to 300+ models. Use the `provider/model` naming convention:

| Model | Provider | Best for |
|-------|----------|----------|
| `anthropic/claude-3.5-sonnet` | Anthropic | Strong coding, default |
| `openai/gpt-4o` | OpenAI | General-purpose |
| `google/gemini-2.0-flash-exp` | Google | Fast, multimodal |
| `meta-llama/llama-3.1-70b-instruct` | Meta (hosted) | Open model, good quality |
| `mistralai/mistral-large-latest` | Mistral | European alternative |
| `deepseek/deepseek-chat` | DeepSeek | Budget-friendly coding |

**Default:** `anthropic/claude-3.5-sonnet`

Override from the CLI:

```bash
vibecli --provider openrouter --model openai/gpt-4o
```

Browse all available models at [openrouter.ai/models](https://openrouter.ai/models).

## Pricing

OpenRouter uses pay-per-token pricing that varies by model. Prices are transparently listed on each model's page. OpenRouter adds a small markup over the provider's native price for the convenience of unified billing.

## Best For

- **Model comparison** -- quickly switch between models to compare quality
- **Single billing** -- one API key and invoice for all providers
- **Access to restricted models** -- some models are easier to access through OpenRouter
- **Fallback routing** -- OpenRouter can automatically route to alternative models

## Verify Connection

```bash
vibecli --provider openrouter -c "Explain the difference between async and threads in Rust"
```

## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Verify the key starts with `sk-or-`
- Check your key at [openrouter.ai/keys](https://openrouter.ai/keys)

### Model not found

```
Error: 404 - model not found
```

- Use the full `provider/model` format (e.g., `anthropic/claude-3.5-sonnet`)
- Check [openrouter.ai/models](https://openrouter.ai/models) for current model names

### Insufficient credits

- Add credits at [openrouter.ai/credits](https://openrouter.ai/credits)
