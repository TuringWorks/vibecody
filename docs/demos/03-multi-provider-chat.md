---
layout: page
title: "Demo 3: Multi-Provider AI Chat"
permalink: /demos/multi-provider-chat/
nav_order: 3
parent: Demos
---


## Overview

VibeCody supports 17 AI providers out of the box, from cloud APIs like Claude and OpenAI to fully local models via Ollama. This demo shows you how to switch between providers, configure BYOK (Bring Your Own Key), set up failover chains, compare costs, and leverage provider-specific features like vision and tool use.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI installed and configured (see [Demo 1: First Run](../first-run/))
- API keys for at least two providers (to demonstrate switching)
- Ollama installed locally for offline demos (optional)

## Supported Providers

| # | Provider | Key Env Var | Local/Cloud | Notable Features |
|---|----------|-------------|-------------|------------------|
| 1 | Ollama | (none) | Local | Fully offline, 1000+ models |
| 2 | Claude | `ANTHROPIC_API_KEY` | Cloud | Tool use, 200K context, vision |
| 3 | OpenAI | `OPENAI_API_KEY` | Cloud | GPT-4o, vision, function calling |
| 4 | Gemini | `GEMINI_API_KEY` | Cloud | 2M context, multimodal |
| 5 | Grok | `GROK_API_KEY` | Cloud | Real-time knowledge |
| 6 | Groq | `GROQ_API_KEY` | Cloud | Ultra-fast inference |
| 7 | OpenRouter | `OPENROUTER_API_KEY` | Cloud | 300+ models, unified API |
| 8 | Azure OpenAI | `AZURE_OPENAI_API_KEY` | Cloud | Enterprise compliance |
| 9 | Bedrock | `AWS_ACCESS_KEY_ID` | Cloud | AWS-native, IAM auth |
| 10 | Copilot | `GITHUB_TOKEN` | Cloud | GitHub-optimized |
| 11 | Mistral | `MISTRAL_API_KEY` | Cloud | Code-specialized models |
| 12 | Cerebras | `CEREBRAS_API_KEY` | Cloud | Wafer-scale inference |
| 13 | DeepSeek | `DEEPSEEK_API_KEY` | Cloud | Cost-effective coding |
| 14 | Zhipu | `ZHIPU_API_KEY` | Cloud | GLM-4 series |
| 15 | Vercel AI | `VERCEL_AI_API_KEY` | Cloud | Edge-optimized |
| 16 | LocalEdit | (none) | Local | Local file editing only |
| 17 | Failover | (configured) | Mixed | Auto-failover chain |

## Step-by-Step Walkthrough

### Step 1: Check your current provider

```bash
vibecli chat --provider claude "What provider are you?"
```

Or in the REPL:

```bash
vibecli repl
> /provider
Current provider: claude (claude-sonnet-4-20250514)
```

### Step 2: Switch providers on the fly

**From the command line:**

```bash
# Use OpenAI
vibecli chat --provider openai --model gpt-4o "Explain monads"

# Use Ollama locally
vibecli chat --provider ollama --model llama3 "Explain monads"

# Use Gemini
vibecli chat --provider gemini --model gemini-2.0-flash "Explain monads"

# Use Groq for ultra-fast responses
vibecli chat --provider groq --model llama-3.3-70b-versatile "Explain monads"
```

**From the REPL:**

```bash
vibecli repl
> /provider openai
Switched to provider: openai (gpt-4o)

> /provider ollama --model codellama
Switched to provider: ollama (codellama)

> /provider claude --model claude-sonnet-4-20250514
Switched to provider: claude (claude-sonnet-4-20250514)
```

<!-- Screenshot placeholder: REPL showing provider switching -->

### Step 3: Streaming responses

All providers support streaming by default. Tokens appear as they are generated.

```bash
vibecli chat --provider claude "Write a haiku about Rust programming"
```

To disable streaming (wait for full response):

```bash
vibecli chat --no-stream --provider openai "Write a haiku about Rust programming"
```

### Step 4: Provider-specific features

**Vision (Claude, OpenAI, Gemini):**

```bash
# Analyze an image
vibecli chat --provider claude "What's in this image?" --image ./screenshot.png

# In the REPL
> /image ./diagram.png
> What does this architecture diagram show?
```

**Tool use (Claude, OpenAI):**

Tool use is automatic in agent mode. The provider's native function-calling protocol is used when available:

```bash
vibecli agent --provider claude "Read the file src/main.rs and add error handling"
```

**Large context (Gemini):**

```bash
# Gemini supports up to 2M tokens of context
vibecli chat --provider gemini --model gemini-2.0-pro \
  "Summarize this codebase" --context-dir ./src/
```

### Step 5: OpenRouter for 300+ models

OpenRouter provides access to hundreds of models through a single API key.

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

```bash
# Use any model available on OpenRouter
vibecli chat --provider openrouter --model "anthropic/claude-sonnet-4-20250514" "Hello"
vibecli chat --provider openrouter --model "google/gemini-2.0-flash" "Hello"
vibecli chat --provider openrouter --model "meta-llama/llama-3.3-70b" "Hello"
vibecli chat --provider openrouter --model "deepseek/deepseek-chat" "Hello"
```

### Step 6: BYOK (Bring Your Own Key) setup

Configure multiple API keys in your config file for team or multi-account setups:

```toml
# ~/.vibecli/config.toml

[provider]
default = "claude"

[provider.claude]
api_key = "sk-ant-YOUR-KEY"
model = "claude-sonnet-4-20250514"

[provider.openai]
api_key = "sk-YOUR-KEY"
model = "gpt-4o"
api_url = "https://api.openai.com/v1"  # Customizable endpoint

[provider.azure_openai]
api_key = "YOUR-AZURE-KEY"
api_url = "https://YOUR-RESOURCE.openai.azure.com"
deployment = "gpt-4o"
api_version = "2024-02-01"

[provider.ollama]
api_url = "http://localhost:11434"  # Default Ollama endpoint
model = "llama3"

[provider.openrouter]
api_key = "sk-or-YOUR-KEY"
model = "anthropic/claude-sonnet-4-20250514"
```

Each provider supports a custom `api_url` for proxied or self-hosted endpoints.

### Step 7: Failover provider chain

The Failover provider automatically tries the next provider in the chain when one fails (rate limit, outage, timeout):

```toml
# ~/.vibecli/config.toml

[provider.failover]
chain = ["claude", "openai", "gemini", "ollama"]
max_retries = 2
retry_delay_ms = 1000
```

```bash
vibecli chat --provider failover "This message will be sent to the first available provider"
```

If Claude returns a rate limit error, VibeCody automatically retries with OpenAI, then Gemini, then falls back to local Ollama.

<!-- Screenshot placeholder: Failover chain in action -->

### Step 8: Cost tracking per provider

VibeCody tracks token usage and estimated costs for every interaction.

```bash
vibecli repl
> /cost
Session cost summary:
  claude:   $0.0342 (12,400 tokens)
  openai:   $0.0128 (5,200 tokens)
  ollama:   $0.0000 (8,100 tokens)  [local]
  Total:    $0.0470
```

See [Demo 6: Cost Observatory](../cost-observatory/) for the full cost dashboard.

### Step 9: Provider comparison

Send the same prompt to multiple providers and compare:

```bash
# Quick comparison from CLI
vibecli chat --provider claude "Write FizzBuzz in Rust" > claude_response.txt
vibecli chat --provider openai "Write FizzBuzz in Rust" > openai_response.txt
vibecli chat --provider gemini "Write FizzBuzz in Rust" > gemini_response.txt

# Or use the Arena for side-by-side comparison (see Demo 5)
```

## VibeUI Provider Switching

In VibeUI, open the AI panel (`Cmd+J`) and use the provider dropdown in the top toolbar to switch providers. The Keys panel (`Cmd+J` then "Keys" tab) lets you manage API keys with a graphical interface.

<!-- Screenshot placeholder: VibeUI provider dropdown -->

## Demo Recording

```json
{
  "meta": {
    "title": "Multi-Provider AI Chat",
    "description": "Switch between 17 AI providers, set up BYOK, configure failover chains, and compare provider costs.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli chat --provider claude \"What is 2 + 2?\"",
      "description": "Chat with Claude",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli chat --provider openai --model gpt-4o \"What is 2 + 2?\"",
      "description": "Chat with OpenAI GPT-4o",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli chat --provider ollama --model llama3 \"What is 2 + 2?\"",
      "description": "Chat with local Ollama",
      "delay_ms": 4000,
      "typing_speed_ms": 40
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli chat --provider groq --model llama-3.3-70b-versatile \"What is 2 + 2?\"",
      "description": "Chat with Groq (ultra-fast)",
      "delay_ms": 3000,
      "typing_speed_ms": 40
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/provider claude", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/provider openai", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/provider gemini", "delay_ms": 1500 },
        { "input": "Write a one-liner Python function to reverse a string", "delay_ms": 5000 },
        { "input": "/cost", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Switch providers in REPL and compare responses, then check costs"
    },
    {
      "id": 6,
      "action": "shell",
      "command": "vibecli chat --provider failover \"What's the weather in Tokyo?\"",
      "description": "Demonstrate failover provider chain",
      "delay_ms": 5000
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli chat --provider openrouter --model \"meta-llama/llama-3.3-70b\" \"Hello from OpenRouter\"",
      "description": "Access 300+ models via OpenRouter",
      "delay_ms": 5000,
      "typing_speed_ms": 40
    },
    {
      "id": 8,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[provider]\ndefault = \"failover\"\n\n[provider.failover]\nchain = [\"claude\", \"openai\", \"ollama\"]\nmax_retries = 2\n\n[provider.claude]\napi_key = \"sk-ant-demo\"\n\n[provider.openai]\napi_key = \"sk-demo\"\n\n[provider.ollama]\nmodel = \"llama3\"\n",
      "description": "Write failover provider configuration",
      "delay_ms": 1000
    }
  ]
}
```

## What's Next

- [Demo 4: Agent Loop](../agent-loop/) -- Autonomous code editing with tool execution
- [Demo 5: Model Arena](../model-arena/) -- Compare models in a structured evaluation
- [Demo 6: Cost Observatory](../cost-observatory/) -- Deep dive into token costs and budgets
