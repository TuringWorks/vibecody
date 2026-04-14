---
layout: page
title: "Provider: Together AI"
permalink: /providers/together/
---

# Together AI Provider

[Together AI](https://www.together.ai) hosts open-source models with competitive pricing and a free tier. They offer a wide catalog of Llama, Mixtral, Qwen, and other open models.

## Get an API Key

1. Go to [api.together.ai](https://api.together.ai)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

Together AI offers a free tier with limited usage.

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export TOGETHER_API_KEY="..."
vibecli --provider together
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[together]
enabled = true
api_key = "..."
model = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo"
```

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo` | Strong general coding | Daily tasks (default) |
| `meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo` | Ultra-fast | Quick completions |
| `Qwen/Qwen2.5-Coder-32B-Instruct` | Code-specialized | Code generation |
| `mistralai/Mixtral-8x22B-Instruct-v0.1` | Large context | Longer analysis |

**Default:** `meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo`

Override from the CLI:

```bash
vibecli --provider together --model "Qwen/Qwen2.5-Coder-32B-Instruct"
```

Browse all models at [api.together.ai/models](https://api.together.ai/models).

## Pricing

Together AI offers competitive pricing for open models. Free tier includes limited tokens per day. Check [together.ai/pricing](https://www.together.ai/pricing) for current rates.

## Best For

- **Open-source models** -- access the latest Llama, Qwen, Mixtral releases
- **Cost-effective** -- lower prices than running your own GPU infrastructure
- **Free tier** -- experiment with models before committing
- **Model variety** -- large catalog of open models

## Verify Connection

```bash
vibecli --provider together -c "Write a Rust function to parse command-line arguments"
```

## Troubleshooting

### Invalid API key

- Check your key at [api.together.ai](https://api.together.ai)
- Confirm the env var is set: `echo $TOGETHER_API_KEY`

### Model not found

- Use the full model path (e.g., `meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo`)
- Check [api.together.ai/models](https://api.together.ai/models) for current model names
