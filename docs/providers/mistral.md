---
layout: page
title: "Provider: Mistral"
permalink: /providers/mistral/
---

# Mistral Provider

[Mistral AI](https://mistral.ai) is a European AI company producing high-performance models with strong coding abilities. Their models offer a good balance of quality and speed.

## Get an API Key

1. Go to [console.mistral.ai](https://console.mistral.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export MISTRAL_API_KEY="..."
vibecli --provider mistral
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[mistral]
enabled = true
api_key = "..."
model = "mistral-large-latest"
```

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `mistral-large-latest` | Strongest reasoning | Complex coding tasks |
| `mistral-medium-latest` | Good balance | General-purpose coding |
| `mistral-small-latest` | Fast, affordable | Quick completions |
| `codestral-latest` | Code-specialized | Code generation, completion |

**Default:** `mistral-large-latest`

Override from the CLI:

```bash
vibecli --provider mistral --model codestral-latest
```

### Codestral

Codestral is Mistral's code-specialized model, trained specifically for code generation, completion, and explanation. Use it for pure coding tasks where you want maximum code quality.

## Pricing

Check [mistral.ai/pricing](https://docs.mistral.ai/getting-started/pricing/) for current rates. Mistral is competitively priced compared to other frontier providers.

## Best For

- **European data sovereignty** -- Mistral is based in the EU
- **Code-specialized models** -- Codestral for pure coding tasks
- **Competitive pricing** -- strong quality at lower cost than GPT-4o or Claude

## Verify Connection

```bash
vibecli --provider mistral -c "Write a Rust struct with serde serialization"
```

## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Check your key at [console.mistral.ai](https://console.mistral.ai)
- Confirm the env var is set: `echo $MISTRAL_API_KEY`

### Model not found

- Use `mistral-large-latest` (not just `mistral-large`)
- Check [docs.mistral.ai](https://docs.mistral.ai) for current model names
