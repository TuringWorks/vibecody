---
layout: page
title: "Provider: Claude"
permalink: /providers/claude/
---

# Claude Provider

[Claude](https://www.anthropic.com/claude) by Anthropic is one of the most capable AI models for code generation, reasoning, and long-context tasks.

## Get an API Key

1. Go to [console.anthropic.com](https://console.anthropic.com)
2. Create an account or sign in
3. Navigate to **API Keys** in the left sidebar
4. Click **Create Key** and copy it

Your key will look like: `sk-ant-api03-...`

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
vibecli --provider claude
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[claude]
enabled = true
api_key = "sk-ant-api03-..."
model = "claude-sonnet-4-6"
```

**Option 3: API key helper** (for key rotation / vault integration)

```toml
[claude]
enabled = true
api_key_helper = "~/.vibecli/get-key.sh claude"
model = "claude-sonnet-4-6"
```

The helper script must print the key to stdout and exit 0.

## Model Selection

| Model | Strengths | Context | Best for |
|-------|-----------|---------|----------|
| `claude-opus-4-6` | Highest quality reasoning, complex tasks | 1M tokens | Architecture decisions, hard bugs, nuanced refactoring |
| `claude-sonnet-4-6` | Strong balance of quality and speed | 200K tokens | Daily coding, code review, generation |
| `claude-haiku-4-5` | Fast and affordable | 200K tokens | Quick questions, completions, simple edits |

**Default:** `claude-sonnet-4-6`

Override from the CLI:

```bash
vibecli --provider claude --model claude-opus-4-6
```

## Extended Thinking

Enable Claude's extended thinking mode for complex reasoning tasks. The model will "think" internally before responding, producing higher-quality outputs for difficult problems.

```toml
[claude]
enabled = true
model = "claude-sonnet-4-6"
thinking_budget_tokens = 10000
```

Extended thinking uses additional tokens (and cost), but significantly improves performance on:

- Multi-step debugging
- Architecture design
- Complex refactoring
- Security analysis

## Pricing

Pricing as of early 2026 (check [anthropic.com/pricing](https://www.anthropic.com/pricing) for current rates):

| Model | Input (per 1M tokens) | Output (per 1M tokens) |
|-------|----------------------|------------------------|
| Opus 4.6 | $15.00 | $75.00 |
| Sonnet 4.6 | $3.00 | $15.00 |
| Haiku 4.5 | $0.25 | $1.25 |

**Tip:** Use Haiku for quick questions and Sonnet for coding tasks to manage costs. Reserve Opus for complex problems.

## Custom API URL

For proxies or enterprise endpoints:

```toml
[claude]
enabled = true
api_url = "https://my-proxy.example.com/v1"
model = "claude-sonnet-4-6"
```

## Verify Connection

```bash
vibecli --provider claude -c "Say hello and identify yourself"
```

Expected output should mention Claude by name.

## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Verify the key starts with `sk-ant-`
- Check that the key has not been revoked in [console.anthropic.com](https://console.anthropic.com)
- If using an env var, confirm it is exported: `echo $ANTHROPIC_API_KEY`

### Rate limited

```
Error: 429 Too Many Requests
```

- Anthropic applies per-key rate limits based on your usage tier
- New accounts start at a lower tier; limits increase with usage
- Add a billing method to increase your tier

### Context length exceeded

```
Error: 400 - max tokens exceeded
```

- Use a model with a larger context window (Opus 4.6 supports 1M tokens)
- Reduce the conversation history or input size
- VibeCody automatically prunes old context when the window fills up

### Timeout

```
Error: request timed out
```

- Long prompts with extended thinking may take 30-60 seconds
- Check your network connection
- If behind a corporate proxy, configure `api_url` appropriately
