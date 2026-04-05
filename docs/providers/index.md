---
layout: page
title: AI Providers
permalink: /providers/
---


VibeCody supports 23 AI providers, covering cloud APIs, local models, inference platforms, and specialized services. This page provides a comparison and links to individual setup guides.


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
| [Gemini](gemini/) | Cloud | `GEMINI_API_KEY` | `gemini-2.5-flash` | Yes (generous) | Yes |
| [DeepSeek](deepseek/) | Cloud | `DEEPSEEK_API_KEY` | `deepseek-chat` | No | Yes |
| [Grok](grok/) | Cloud | `GROK_API_KEY` | `grok-3-mini` | No | Yes |
| [Groq](groq/) | Cloud | `GROQ_API_KEY` | `llama-3.3-70b-versatile` | Yes (rate-limited) | Yes |
| [OpenRouter](openrouter/) | Cloud | `OPENROUTER_API_KEY` | `anthropic/claude-3.5-sonnet` | No | Yes |
| [Azure OpenAI](azure-openai/) | Cloud | `AZURE_OPENAI_API_KEY` | `gpt-4o` | No | Yes |
| [AWS Bedrock](bedrock/) | Cloud | `AWS_ACCESS_KEY_ID` | `anthropic.claude-3-sonnet-*` | No | Yes |
| [GitHub Copilot](copilot/) | Cloud | `GITHUB_TOKEN` | `gpt-4o` | Yes (with subscription) | Yes |
| [Mistral](mistral/) | Cloud | `MISTRAL_API_KEY` | `mistral-large-latest` | No | Yes |
| [Cerebras](cerebras/) | Cloud | `CEREBRAS_API_KEY` | `llama3.1-70b` | Yes (limited) | Yes |
| [Zhipu GLM](zhipu/) | Cloud | `ZHIPU_API_KEY` | `glm-4` | No | Yes |
| [Vercel AI](vercel-ai/) | Cloud | Via provider key | Provider-dependent | No | Yes |
| [MiniMax](minimax/) | Cloud | `MINIMAX_API_KEY` | `abab6.5s-chat` | No | Yes |
| [Perplexity](perplexity/) | Cloud | `PERPLEXITY_API_KEY` | `sonar-pro` | No | Yes |
| [Together AI](together/) | Inference | `TOGETHER_API_KEY` | `meta-llama/Llama-3.1-70B-Instruct-Turbo` | Yes (limited) | Yes |
| [Fireworks AI](fireworks/) | Inference | `FIREWORKS_API_KEY` | `llama-v3p1-70b-instruct` | Yes (limited) | Yes |
| [SambaNova](sambanova/) | Inference | `SAMBANOVA_API_KEY` | `Meta-Llama-3.1-70B-Instruct` | Yes (limited) | Yes |
| LocalEdit | Local | None | Local model | Yes (fully free) | Yes |
| [Failover](failover/) | Wrapper | N/A | N/A | N/A | Yes |


## Choosing a Provider

**For beginners:** Start with [Ollama](ollama/) -- it is free, runs locally, and requires no API key. Pull `qwen3-coder` or `llama3.2` and you are ready.

**For best quality:** [Claude](claude/) (Opus 4.6 or Sonnet 4.6) and [OpenAI](openai/) (GPT-4o) provide the highest-quality code generation and reasoning.

**For budget-conscious use:** [DeepSeek](deepseek/) offers strong coding performance at very low prices. [Gemini](gemini/) has a generous free tier.

**For fastest inference:** [Groq](https://groq.com) runs open-source models on custom LPU hardware with extremely low latency. [Cerebras](https://cerebras.ai) and SambaNova also provide fast inference on custom hardware.

**For open models:** Together AI, Fireworks AI, and SambaNova host open-source models (Llama, Mixtral, Qwen) with competitive pricing and free tiers.

**For search-augmented AI:** Perplexity's Sonar models combine LLM reasoning with real-time web search — excellent for research tasks.

**For enterprise:** Azure OpenAI and AWS Bedrock integrate with your existing cloud infrastructure and compliance requirements.

**For reliability:** The Failover provider wraps multiple providers and automatically falls back if one fails.


## Quick Examples by Provider

Every provider works with the same CLI interface. Here are copy-paste examples:

```bash
# ── Local (free, private) ─────────────────────────────────────────
ollama pull qwen3-coder
vibecli --provider ollama "Explain the borrow checker"

# ── Cloud APIs ────────────────────────────────────────────────────
export ANTHROPIC_API_KEY="sk-ant-..."
vibecli --provider claude "Fix the bug in src/auth.rs" --agent

export OPENAI_API_KEY="sk-..."
vibecli --provider openai --model gpt-4o "Write unit tests for parser.rs"

export GEMINI_API_KEY="AIza..."
vibecli --provider gemini "Summarize this codebase" --add-dir ./src/

export GROK_API_KEY="..."
vibecli --provider grok "What does this error mean? E0308: mismatched types"

# ── Fast inference (great for quick iterations) ───────────────────
export GROQ_API_KEY="gsk_..."
vibecli --provider groq "Convert this JSON to a Rust struct"

export CEREBRAS_API_KEY="..."
vibecli --provider cerebras "Write a regex for email validation"

export SAMBANOVA_API_KEY="..."
vibecli --provider sambanova "Explain this stack trace"

# ── Budget-friendly ───────────────────────────────────────────────
export DEEPSEEK_API_KEY="..."
vibecli --provider deepseek "Write comprehensive tests for src/db.rs"

# ── Search-augmented ──────────────────────────────────────────────
export PERPLEXITY_API_KEY="pplx-..."
vibecli --provider perplexity "What breaking changes are in Tokio 1.40?"

# ── Multi-model gateways ─────────────────────────────────────────
export OPENROUTER_API_KEY="sk-or-..."
vibecli --provider openrouter --model "meta-llama/llama-3.3-70b" "Hello"

# ── Enterprise ────────────────────────────────────────────────────
export AZURE_OPENAI_API_KEY="..." AZURE_OPENAI_ENDPOINT="https://myco.openai.azure.com"
vibecli --provider azure "Audit this code for OWASP top 10"

export AWS_ACCESS_KEY_ID="AKIA..." AWS_SECRET_ACCESS_KEY="..." AWS_REGION="us-east-1"
vibecli --provider bedrock "Generate a CloudFormation template"

vibecli --provider copilot "Complete this function"   # Uses existing GitHub Copilot

# ── Failover chain ────────────────────────────────────────────────
vibecli --provider failover "Fix the build errors"
# Tries: claude → openai → gemini → ollama (configured in config.toml)
```

### Agent Mode Examples

```bash
# Interactive (approve each step)
vibecli --agent "Add input validation to all API endpoints" --provider claude

# Auto-edit (approve shell commands only)
vibecli --agent "Refactor to async/await" --provider openai --auto-edit

# Full-auto (CI/scripts — no prompts)
vibecli --exec "Run tests and fix any failures" --provider claude --full-auto

# Resume a previous session
vibecli --resume 1711234567
```

### REPL Session

```bash
vibecli
> [src/main.rs]                    # Add file to context
> What does this function do?
> /model claude-opus-4-6           # Switch mid-conversation
> Now refactor it to use async
> /cost                            # Check token usage
```


## Configuration

All providers are configured in `~/.vibecli/config.toml`. See the [Configuration Guide](/vibecody/configuration/) for the full reference with all 21 providers, usage examples, and safety settings.

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
