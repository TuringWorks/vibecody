---
layout: page
title: "Provider: DeepSeek"
permalink: /providers/deepseek/
---

# DeepSeek Provider

[DeepSeek](https://www.deepseek.com) is a Chinese AI lab producing high-quality open-weight models with strong coding performance at very affordable prices. DeepSeek V3 and R1 consistently rank among the top models on coding benchmarks.


## Get an API Key

1. Go to [platform.deepseek.com](https://platform.deepseek.com)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export DEEPSEEK_API_KEY="sk-..."
vibecli --provider deepseek
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[deepseek]
enabled = true
api_key = "sk-..."
model = "deepseek-chat"
```


## Model Selection

| Model | API Name | Strengths | Best for |
|-------|----------|-----------|----------|
| DeepSeek V3 | `deepseek-chat` | General-purpose, strong coding | Code generation, debugging, refactoring |
| DeepSeek R1 | `deepseek-reasoner` | Chain-of-thought reasoning | Complex logic, architecture decisions, hard bugs |

**Default:** `deepseek-chat` (V3)

Override from the CLI:

```bash
vibecli --provider deepseek --model deepseek-reasoner
```

### When to use R1 (Reasoner)

DeepSeek R1 uses chain-of-thought reasoning similar to OpenAI's o-series. Use it when you need:
- Multi-step debugging of complex issues
- Algorithm design and optimization
- System architecture planning
- Mathematical or logical reasoning in code

For everyday coding tasks, `deepseek-chat` (V3) is faster and more cost-effective.


## Pricing

DeepSeek is one of the most affordable providers available:

| Model | Input (per 1M tokens) | Output (per 1M tokens) |
|-------|----------------------|------------------------|
| DeepSeek V3 (chat) | $0.27 | $1.10 |
| DeepSeek R1 (reasoner) | $0.55 | $2.19 |

This makes DeepSeek roughly 5-10x cheaper than GPT-4o and 10-50x cheaper than Claude Opus for equivalent tasks.

**Cache hits** are even cheaper -- DeepSeek caches common prefixes automatically.


## Best For

DeepSeek excels at:

- **Code generation** -- strong performance across Python, JavaScript, Rust, Go, and more
- **Debugging** -- identifies bugs and suggests fixes with high accuracy
- **Code review** -- catches issues that other models miss, at a fraction of the cost
- **Refactoring** -- understands complex codebases and suggests clean improvements
- **Batch workloads** -- very affordable for high-volume agent tasks


## Verify Connection

```bash
vibecli --provider deepseek -c "Write a Rust function to merge two sorted arrays"
```


## Running DeepSeek Locally

DeepSeek models are open-weight. You can run them locally via Ollama:

```bash
ollama pull deepseek-coder-v2:16b
vibecli --provider ollama --model deepseek-coder-v2:16b
```

This gives you the DeepSeek model quality with zero API costs and full privacy.


## Troubleshooting

### Invalid API key

```
Error: 401 Unauthorized
```

- Check the key at [platform.deepseek.com](https://platform.deepseek.com)
- Confirm the env var is set: `echo $DEEPSEEK_API_KEY`

### Rate limited

```
Error: 429 Too Many Requests
```

- DeepSeek has rate limits based on your account tier
- Wait briefly and retry
- Contact DeepSeek support to increase limits for production use

### Slow responses with R1

The reasoner model (`deepseek-reasoner`) performs chain-of-thought reasoning internally, which can take 15-60 seconds for complex prompts. This is expected behavior -- the model is "thinking" before responding.

### Connection issues

```
Error: Connection timed out
```

- DeepSeek servers are primarily hosted in China
- Some regions may experience higher latency
- Consider using a proxy if you have consistent connectivity issues
- Alternatively, run DeepSeek locally via Ollama for zero-latency inference
