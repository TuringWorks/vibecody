---
layout: page
title: "Provider: Perplexity"
permalink: /providers/perplexity/
---

# Perplexity Provider

[Perplexity](https://www.perplexity.ai) combines LLM reasoning with real-time web search, producing responses grounded in current information. Their Sonar models are uniquely suited for research-heavy coding tasks.

## Get an API Key

1. Go to [perplexity.ai/settings/api](https://www.perplexity.ai/settings/api)
2. Create an account or sign in
3. Generate an API key and copy it

## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export PERPLEXITY_API_KEY="pplx-..."
vibecli --provider perplexity
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[perplexity]
enabled = true
api_key = "pplx-..."
model = "sonar-pro"
```

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `sonar-pro` | Deep research, citations | Complex research tasks |
| `sonar` | Fast search-augmented | Quick lookups |

**Default:** `sonar-pro`

Override from the CLI:

```bash
vibecli --provider perplexity --model sonar
```

## Best For

- **Research-heavy tasks** -- find current documentation, API references, library versions
- **Debugging with context** -- search for error messages and known issues in real-time
- **Technology comparison** -- evaluate libraries and tools with up-to-date information
- **Learning new APIs** -- get current examples and best practices

## Verify Connection

```bash
vibecli --provider perplexity -c "What are the latest changes in React 19?"
```

## Troubleshooting

### Invalid API key

- Verify the key starts with `pplx-`
- Check your key at [perplexity.ai/settings/api](https://www.perplexity.ai/settings/api)

### Slow responses

- Search-augmented models perform web searches before responding
- Expect 3-10 seconds for research-heavy queries
