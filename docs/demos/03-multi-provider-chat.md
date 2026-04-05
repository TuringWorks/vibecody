---
layout: page
title: "Demo 3: Multi-Provider AI Chat"
permalink: /demos/multi-provider-chat/
nav_order: 3
parent: Demos
---


## Overview

VibeCody supports 23 AI providers out of the box, from cloud APIs like Claude and OpenAI to fully local models via Ollama. This demo shows you how to switch between providers, configure BYOK (Bring Your Own Key), set up failover chains, compare costs, and leverage provider-specific features like vision and tool use.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI installed and configured (see [Demo 1: First Run](../01-first-run/))
- API keys for at least two providers (to demonstrate switching)
- Ollama installed locally for offline demos (optional)

## Supported Providers

| # | Provider | Key Env Var | Local/Cloud | Notable Features |
|---|----------|-------------|-------------|------------------|
| 1 | Ollama | (none) | Local | Fully offline, 1000+ models |
| 2 | Claude | `ANTHROPIC_API_KEY` | Cloud | Tool use, 1M context (Opus), vision |
| 3 | OpenAI | `OPENAI_API_KEY` | Cloud | GPT-4o, vision, function calling |
| 4 | Gemini | `GEMINI_API_KEY` | Cloud | 2M context, multimodal |
| 5 | Grok | `GROK_API_KEY` | Cloud | Real-time knowledge |
| 6 | Groq | `GROQ_API_KEY` | Cloud | Ultra-fast LPU inference |
| 7 | OpenRouter | `OPENROUTER_API_KEY` | Cloud | 300+ models, unified API |
| 8 | Azure OpenAI | `AZURE_OPENAI_API_KEY` | Cloud | Enterprise compliance |
| 9 | AWS Bedrock | `AWS_ACCESS_KEY_ID` | Cloud | AWS-native, IAM auth |
| 10 | GitHub Copilot | `GITHUB_TOKEN` | Cloud | Uses existing Copilot subscription |
| 11 | Mistral | `MISTRAL_API_KEY` | Cloud | Codestral, code-specialized |
| 12 | Cerebras | `CEREBRAS_API_KEY` | Cloud | Wafer-scale inference |
| 13 | DeepSeek | `DEEPSEEK_API_KEY` | Cloud | Budget-friendly coding (V3/R1) |
| 14 | Zhipu | `ZHIPU_API_KEY` | Cloud | GLM-4 series |
| 15 | Vercel AI | `VERCEL_AI_API_KEY` | Cloud | Gateway proxy |
| 16 | MiniMax | `MINIMAX_API_KEY` | Cloud | MiniMax-Text-01 |
| 17 | Perplexity | `PERPLEXITY_API_KEY` | Cloud | Search-augmented Sonar models |
| 18 | Together AI | `TOGETHER_API_KEY` | Inference | Open model hosting (Llama, Qwen) |
| 19 | Fireworks AI | `FIREWORKS_API_KEY` | Inference | Fast open model inference |
| 20 | SambaNova | `SAMBANOVA_API_KEY` | Inference | Hardware-accelerated inference |
| 21 | LocalEdit | (none) | Local | Local FIM code completion |
| 22 | Failover | (configured) | Mixed | Auto-failover chain |

## Step-by-Step Walkthrough

### Step 1: Check your current provider

```bash
vibecli --provider claude "What provider are you?"
```

Or in the REPL (just run `vibecli` with no arguments):

```bash
vibecli
> What provider are you?
```

### Step 2: Switch providers on the fly

**From the command line:**

```bash
# Use OpenAI
vibecli --provider openai --model gpt-4o "Explain monads"

# Use Ollama locally
vibecli --provider ollama --model llama3 "Explain monads"

# Use Gemini
vibecli --provider gemini --model gemini-2.5-flash "Explain monads"

# Use Groq for ultra-fast responses
vibecli --provider groq --model llama-3.3-70b-versatile "Explain monads"
```

**From the REPL:**

```bash
vibecli
> /model gpt-4o
Switched to model: gpt-4o

> /model codellama
Switched to model: codellama

> /model claude-sonnet-4-6
Switched to model: claude-sonnet-4-6
```

<!-- Screenshot placeholder: REPL showing provider switching -->

### Step 3: Streaming responses

All providers support streaming by default. Tokens appear as they are generated.

```bash
vibecli --provider claude "Write a haiku about Rust programming"
```

### Step 4: Provider-specific features

**Vision (Claude, OpenAI, Gemini):**

```bash
# Analyze an image
vibecli --provider claude "What's in this image?" --image ./screenshot.png

# In the REPL
> [./diagram.png] What does this architecture diagram show?
```

**Tool use (Claude, OpenAI):**

Tool use is automatic in agent mode. The provider's native function-calling protocol is used when available:

```bash
vibecli --agent "Read the file src/main.rs and add error handling" --provider claude
```

**Large context (Gemini):**

```bash
# Gemini supports up to 2M tokens of context
vibecli --provider gemini --model gemini-2.5-pro \
  "Summarize this codebase" --add-dir ./src/
```

### Step 5: OpenRouter for 300+ models

OpenRouter provides access to hundreds of models through a single API key.

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

```bash
# Use any model available on OpenRouter
vibecli --provider openrouter --model "anthropic/claude-sonnet-4-6" "Hello"
vibecli --provider openrouter --model "google/gemini-2.5-flash" "Hello"
vibecli --provider openrouter --model "meta-llama/llama-3.3-70b" "Hello"
vibecli --provider openrouter --model "deepseek/deepseek-chat" "Hello"
```

### Step 6: BYOK (Bring Your Own Key) setup

Configure multiple API keys in your config file for team or multi-account setups:

```toml
# ~/.vibecli/config.toml

[claude]
enabled = true
api_key = "sk-ant-YOUR-KEY"
model = "claude-sonnet-4-6"

[openai]
enabled = true
api_key = "sk-YOUR-KEY"
model = "gpt-4o"
api_url = "https://api.openai.com/v1"  # Customizable endpoint

[azure_openai]
enabled = true
api_key = "YOUR-AZURE-KEY"
api_url = "https://YOUR-RESOURCE.openai.azure.com"
deployment = "gpt-4o"
api_version = "2024-02-01"

[ollama]
enabled = true
api_url = "http://localhost:11434"  # Default Ollama endpoint
model = "llama3"

[openrouter]
enabled = true
api_key = "sk-or-YOUR-KEY"
model = "anthropic/claude-sonnet-4-6"
```

Each provider supports a custom `api_url` for proxied or self-hosted endpoints.

### Step 7: Failover provider chain

The Failover provider automatically tries the next provider in the chain when one fails (rate limit, outage, timeout):

```toml
# ~/.vibecli/config.toml

[failover]
chain = ["claude", "openai", "gemini", "ollama"]
max_retries = 2
retry_delay_ms = 1000
```

```bash
vibecli --provider failover "This message will be sent to the first available provider"
```

If Claude returns a rate limit error, VibeCody automatically retries with OpenAI, then Gemini, then falls back to local Ollama.

<!-- Screenshot placeholder: Failover chain in action -->

### Step 8: Cost tracking per provider

VibeCody tracks token usage and estimated costs for every interaction.

```bash
vibecli
> /cost
Session cost summary:
  claude:   $0.0342 (12,400 tokens)
  openai:   $0.0128 (5,200 tokens)
  ollama:   $0.0000 (8,100 tokens)  [local]
  Total:    $0.0470
```

See [Demo 6: Cost Observatory](../06-cost-observatory/) for the full cost dashboard.

### Step 9: Provider comparison

Send the same prompt to multiple providers and compare:

```bash
# Quick comparison from CLI
vibecli --provider claude "Write FizzBuzz in Rust" > claude_response.txt
vibecli --provider openai "Write FizzBuzz in Rust" > openai_response.txt
vibecli --provider gemini "Write FizzBuzz in Rust" > gemini_response.txt

# Or use the Arena for side-by-side comparison (see Demo 5)
```

## Real-World Provider Workflows

### Workflow 1: Privacy-First Development (Ollama)

Everything stays on your machine — no API keys, no network calls, no data sharing:

```bash
# Pull a coding model once
ollama pull qwen3-coder

# Daily coding workflow — zero cost, full privacy
vibecli --provider ollama --model qwen3-coder \
  --agent "Add input validation to the /api/register endpoint"

# Code review without sending code to the cloud
git diff HEAD~1 | vibecli --provider ollama "Review this diff for bugs"
```

### Workflow 2: Multi-Provider Cost Optimization

Use cheap/fast providers for simple tasks, premium providers for complex ones:

```bash
# Quick question → Groq (fast, free tier)
vibecli --provider groq "What does #[derive(Clone)] do in Rust?"

# Code generation → DeepSeek (budget-friendly, strong at coding)
vibecli --provider deepseek "Write a rate limiter middleware for Axum"

# Complex debugging → Claude Opus (highest reasoning quality)
vibecli --provider claude --model claude-opus-4-6 \
  "There's a race condition in src/worker.rs. The worker sometimes processes \
   the same job twice when under load. Find and fix it."
```

### Workflow 3: Research with Web Grounding (Perplexity)

Get answers backed by real-time web search:

```bash
# Library decisions grounded in current benchmarks
vibecli --provider perplexity "Compare serde vs simd-json performance for large payloads in 2026"

# Debugging with current docs
vibecli --provider perplexity "How to fix 'lifetime may not live long enough' in async Rust with tokio 1.40?"

# Security advisories
vibecli --provider perplexity "Are there any recent CVEs affecting jsonwebtoken crate?"
```

### Workflow 4: Enterprise Compliance (Azure / Bedrock)

Route all AI traffic through your corporate cloud account:

```bash
# Azure — data stays in your Azure tenant
vibecli --provider azure --agent "Migrate the auth module from JWT to OIDC"

# Bedrock — uses IAM roles, no API keys in code
vibecli --provider bedrock --agent "Add CloudWatch metrics to all Lambda handlers"

# GitHub Copilot — uses existing subscription
vibecli --provider copilot "Complete the integration test for the payment service"
```

### Workflow 5: Exploring New Models (OpenRouter)

Try any of 300+ models without separate accounts:

```bash
# Compare a coding task across model families
vibecli --provider openrouter --model "anthropic/claude-sonnet-4-6" "Write binary search" > /tmp/claude.txt
vibecli --provider openrouter --model "google/gemini-2.5-flash"     "Write binary search" > /tmp/gemini.txt
vibecli --provider openrouter --model "meta-llama/llama-3.3-70b"    "Write binary search" > /tmp/llama.txt
vibecli --provider openrouter --model "deepseek/deepseek-chat"      "Write binary search" > /tmp/deepseek.txt

# Side-by-side review
diff /tmp/claude.txt /tmp/gemini.txt
```

### Workflow 6: Resilient CI Pipeline (Failover)

```toml
# ~/.vibecli/config.toml
[failover]
chain = ["claude", "openai", "gemini", "ollama"]
```

```bash
# CI job that never fails due to a single provider outage
vibecli --provider failover --full-auto \
  --exec "Review the diff in this PR for bugs and security issues" < pr.diff
```


## VibeUI Provider Switching

In VibeUI, open the AI panel (`Cmd+J`) and use the provider dropdown in the top toolbar to switch providers. The Keys panel (`Cmd+J` then "Keys" tab) lets you manage API keys with a graphical interface.

<!-- Screenshot placeholder: VibeUI provider dropdown -->

## Demo Recording

```json
{
  "meta": {
    "title": "Multi-Provider AI Chat",
    "description": "Switch between 23 AI providers, set up BYOK, configure failover chains, and compare provider costs.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --provider claude \"What is 2 + 2?\"",
      "description": "Chat with Claude",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli --provider openai --model gpt-4o \"What is 2 + 2?\"",
      "description": "Chat with OpenAI GPT-4o",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli --provider ollama --model llama3 \"What is 2 + 2?\"",
      "description": "Chat with local Ollama",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli --provider groq --model llama-3.3-70b-versatile \"What is 2 + 2?\"",
      "description": "Chat with Groq (ultra-fast)",
      "delay_ms": 3000,
      "typing_speed_ms": 40
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/model claude-sonnet-4-6", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/model gpt-4o", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/model gemini-2.5-flash", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/cost", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Switch providers in REPL and compare responses, then check costs"
    },
    {
      "id": 6,
      "action": "shell",
      "command": "vibecli --provider failover \"What's the weather in Tokyo?\"",
      "description": "Demonstrate failover provider chain",
      "delay_ms": 5000
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli --provider openrouter --model \"meta-llama/llama-3.3-70b\" \"Hello from OpenRouter\"",
      "description": "Access 300+ models via OpenRouter",
      "delay_ms": 5000,
      "typing_speed_ms": 40
    },
    {
      "id": 8,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[failover]\nchain = [\"claude\", \"openai\", \"ollama\"]\nmax_retries = 2\n\n[claude]\nenabled = true\napi_key = \"sk-ant-demo\"\n\n[openai]\nenabled = true\napi_key = \"sk-demo\"\n\n[ollama]\nenabled = true\nmodel = \"llama3\"\n",
      "description": "Write failover provider configuration",
      "delay_ms": 1000
    }
  ]
}
```

## What's Next

- [Demo 4: Agent Loop](../04-agent-loop/) -- Autonomous code editing with tool execution
- [Demo 5: Model Arena](../05-model-arena/) -- Compare models in a structured evaluation
- [Demo 6: Cost Observatory](../06-cost-observatory/) -- Deep dive into token costs and budgets
