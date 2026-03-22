---
layout: page
title: AI Providers
permalink: /providers/
---


VibeCody supports 22 AI providers, covering cloud APIs, local models, inference platforms, and specialized services. This page provides a comparison and links to individual setup guides.


## Quick Start

1. Pick a provider from the table below
2. Follow its setup guide to get an API key (or install locally)
3. Set the environment variable or edit `~/.vibecli/config.toml`
4. Run `vibecli --provider <name>` or enable it in VibeUI settings


## Provider Comparison

| Provider | Type | API Key Env Var | Default Model | Free Tier | Streaming |
|----------|------|-----------------|---------------|-----------|-----------|
| [Ollama](ollama/) | Local | None (no key needed) | `qwen3-coder:480b-cloud` | Yes (fully free) | Yes |
| [Claude](claude/) | Cloud | `ANTHROPIC_API_KEY` | `claude-sonnet-4-6` | No | Yes |
| [OpenAI](openai/) | Cloud | `OPENAI_API_KEY` | `gpt-4o` | No | Yes |
| [Gemini](gemini/) | Cloud | `GEMINI_API_KEY` | `gemini-2.0-flash` | Yes (generous) | Yes |
| [DeepSeek](deepseek/) | Cloud | `DEEPSEEK_API_KEY` | `deepseek-chat` | No | Yes |
| Grok | Cloud | `GROK_API_KEY` | `grok-3-mini` | No | Yes |
| Groq | Cloud | `GROQ_API_KEY` | `llama-3.3-70b-versatile` | Yes (rate-limited) | Yes |
| OpenRouter | Cloud | `OPENROUTER_API_KEY` | `anthropic/claude-3.5-sonnet` | No | Yes |
| Azure OpenAI | Cloud | `AZURE_OPENAI_API_KEY` | `gpt-4o` | No | Yes |
| AWS Bedrock | Cloud | `AWS_ACCESS_KEY_ID` | `anthropic.claude-3-5-sonnet-*` | No | Yes |
| GitHub Copilot | Cloud | `GITHUB_COPILOT_TOKEN` | Copilot default | Yes (with subscription) | Yes |
| Mistral | Cloud | `MISTRAL_API_KEY` | `mistral-large-latest` | No | Yes |
| Cerebras | Cloud | `CEREBRAS_API_KEY` | `llama3.1-70b` | Yes (limited) | Yes |
| Zhipu | Cloud | `ZHIPU_API_KEY` | `glm-4` | No | Yes |
| Vercel AI | Cloud | Via provider key | Provider-dependent | No | Yes |
| MiniMax | Cloud | `MINIMAX_API_KEY` | `abab6.5s-chat` | No | Yes |
| Perplexity | Cloud | `PERPLEXITY_API_KEY` | `sonar-pro` | No | Yes |
| Together AI | Inference | `TOGETHER_API_KEY` | `meta-llama/Llama-3.1-70B-Instruct-Turbo` | Yes (limited) | Yes |
| Fireworks AI | Inference | `FIREWORKS_API_KEY` | `llama-v3p1-70b-instruct` | Yes (limited) | Yes |
| SambaNova | Inference | `SAMBANOVA_API_KEY` | `Meta-Llama-3.1-70B-Instruct` | Yes (limited) | Yes |
| LocalEdit | Local | None | Local model | Yes (fully free) | Yes |
| Failover | Wrapper | N/A | N/A | N/A | Yes |


## Choosing a Provider

**For beginners:** Start with [Ollama](ollama/) -- it is free, runs locally, and requires no API key. Pull `qwen3-coder` or `llama3.1` and you are ready.

**For best quality:** [Claude](claude/) (Opus 4.6 or Sonnet 4.6) and [OpenAI](openai/) (GPT-4o) provide the highest-quality code generation and reasoning.

**For budget-conscious use:** [DeepSeek](deepseek/) offers strong coding performance at very low prices. [Gemini](gemini/) has a generous free tier.

**For fastest inference:** [Groq](https://groq.com) runs open-source models on custom LPU hardware with extremely low latency. [Cerebras](https://cerebras.ai) and SambaNova also provide fast inference on custom hardware.

**For open models:** Together AI, Fireworks AI, and SambaNova host open-source models (Llama, Mixtral, Qwen) with competitive pricing and free tiers.

**For search-augmented AI:** Perplexity's Sonar models combine LLM reasoning with real-time web search — excellent for research tasks.

**For enterprise:** Azure OpenAI and AWS Bedrock integrate with your existing cloud infrastructure and compliance requirements.

**For reliability:** The Failover provider wraps multiple providers and automatically falls back if one fails.


## Configuration

All providers are configured in `~/.vibecli/config.toml`. See the [Configuration Guide](/vibecody/configuration/) for the full reference.

Environment variables take precedence over config file values.

```toml
# Example: enable Claude and Ollama
[claude]
enabled = true
model = "claude-sonnet-4-6"

[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen3-coder:480b-cloud"
```


## API Key Helpers

For teams that rotate keys or use vaults, every provider supports an `api_key_helper` field that runs a script to fetch the current key:

```toml
[claude]
enabled = true
api_key_helper = "~/.vibecli/get-key.sh claude"
```

The script must print the API key to stdout and exit with code 0.
