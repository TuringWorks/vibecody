---
layout: page
title: "Provider: Failover"
permalink: /providers/failover/
---

# Failover Provider

The Failover provider wraps multiple providers and automatically falls back to the next one in the chain if a request fails. Use it for high-reliability setups where uptime is critical.

## Configure VibeCody

**Config file** (`~/.vibecli/config.toml`)

```toml
[failover]
chain = ["claude", "openai", "ollama"]
```

When a request fails (timeout, rate limit, server error), VibeCody automatically retries with the next provider in the chain. Each provider in the chain must be configured separately.

### Full example

```toml
# Primary provider
[claude]
enabled = true
model = "claude-sonnet-4-6"

# Secondary provider
[openai]
enabled = true
model = "gpt-4o"

# Local fallback (always available)
[ollama]
enabled = true
model = "qwen3-coder"

# Failover chain
[failover]
chain = ["claude", "openai", "ollama"]
```

Use from the CLI:

```bash
vibecli --provider failover
```

## How It Works

1. VibeCody sends the request to the first provider in the chain
2. If it fails (network error, 429, 500, timeout), the next provider is tried
3. This continues until a provider succeeds or the chain is exhausted
4. If all providers fail, the error from the last provider is returned

## Best For

- **Production reliability** -- ensure AI is always available even if one provider has an outage
- **Rate limit mitigation** -- overflow to another provider when rate-limited
- **Cost optimization** -- try a cheaper provider first, fall back to a more expensive one
- **Air-gapped fallback** -- chain a cloud provider with a local Ollama instance

## Troubleshooting

### All providers failed

- Check that each provider in the chain is properly configured with valid credentials
- Run `vibecli --provider <name> -c "test"` for each provider individually to identify which one works
