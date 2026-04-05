---
layout: page
title: Configuration Guide
permalink: /configuration/
---


VibeCody uses TOML-based configuration for VibeCLI. VibeUI provider settings are managed through environment variables or the in-app settings UI.


## VibeCLI Configuration

**Location:** `~/.vibecli/config.toml`

The file is created automatically with defaults on first run. You can also create it manually.

### Full Reference

```toml
# ── Providers ──────────────────────────────────────────────────────

[ollama]
enabled = true
api_url = "http://localhost:11434"   # Local Ollama endpoint
model = "qwen3-coder:480b-cloud"     # Any model pulled via 'ollama pull'

[claude]
enabled = false
api_key = "sk-ant-..."              # Anthropic API key (or use env ANTHROPIC_API_KEY)
model = "claude-sonnet-4-6"
# api_key_helper = "~/.vibecli/get-key.sh claude"  # Script that prints a fresh key
# thinking_budget_tokens = 10000                    # Enable extended thinking mode

[openai]
enabled = false
api_key = "sk-..."                  # OpenAI API key (or OPENAI_API_KEY)
model = "gpt-4o"
# api_key_helper = "~/.vibecli/get-key.sh openai"

[gemini]
enabled = false
api_key = "AIza..."                 # Google AI Studio key (or GEMINI_API_KEY)
model = "gemini-2.5-flash"
# api_key_helper = "~/.vibecli/get-key.sh gemini"

[grok]
enabled = false
api_key = "..."                     # xAI API key (or GROK_API_KEY)
model = "grok-3-mini"
# api_key_helper = "~/.vibecli/get-key.sh grok"

# ── UI ─────────────────────────────────────────────────────────────

[ui]
theme = "dark"   # "dark" or "light"

# ── Safety ─────────────────────────────────────────────────────────

[safety]
require_approval_for_commands = true      # Prompt before running shell commands
require_approval_for_file_changes = true  # Prompt before applying AI file edits
approval_policy = "suggest"               # "suggest" | "auto-edit" | "full-auto"

# Wildcard tool permission patterns
# denied_tool_patterns = ["bash(rm*)"]   # Block bash calls matching rm*
# denied_tools = ["bash"]               # Exact-match tool block list

# ── Memory (auto-recording) ─────────────────────────────────────────

[memory]
auto_record = false           # Append session learnings to ~/.vibecli/memory.md
min_session_steps = 3         # Minimum tool-use steps before recording triggers

# ── Embedding Index ─────────────────────────────────────────────────

[index]
enabled = true
embedding_provider = "ollama"       # "ollama" or "openai"
embedding_model = "nomic-embed-text"
rebuild_on_startup = false
max_file_size_kb = 500

# ── OpenTelemetry (optional) ─────────────────────────────────────────

[otel]
enabled = false
endpoint = "http://localhost:4318"  # OTLP/HTTP collector
service_name = "vibecli"

# ── Red Team Security Testing ─────────────────────────────────────────

[redteam]
max_depth = 3                      # Max crawl depth for endpoint recon
timeout_secs = 300                 # Per-stage timeout
parallel_agents = 3                # Concurrent exploitation agents
scope_patterns = ["*"]             # URL patterns in scope
exclude_patterns = []              # URLs to skip
auth_config = ""                   # Path to auth YAML file
auto_report = true                 # Auto-generate report on completion

# ── Additional Providers ──────────────────────────────────────────

[groq]
enabled = false
api_key = "gsk_..."                # Groq API key (or GROQ_API_KEY)
model = "llama-3.3-70b-versatile"

[openrouter]
enabled = false
api_key = "sk-or-..."             # OpenRouter key (or OPENROUTER_API_KEY)
model = "anthropic/claude-3.5-sonnet"

[azure_openai]
enabled = false
api_key = "..."                    # Azure key (or AZURE_OPENAI_API_KEY)
api_url = "https://<resource>.openai.azure.com"
model = "gpt-4o"                   # Deployment name

[bedrock]
enabled = false
# Uses AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION env vars
model = "anthropic.claude-3-5-sonnet-20241022-v2:0"

[mistral]
enabled = false
api_key = "..."                    # Mistral key (or MISTRAL_API_KEY)
model = "mistral-large-latest"

[cerebras]
enabled = false
api_key = "..."                    # Cerebras key (or CEREBRAS_API_KEY)
model = "llama3.1-70b"

[deepseek]
enabled = false
api_key = "..."                    # DeepSeek key (or DEEPSEEK_API_KEY)
model = "deepseek-chat"

[zhipu]
enabled = false
api_key = "..."                    # Zhipu key (or ZHIPU_API_KEY)
model = "glm-4"

[vercel_ai]
enabled = false
api_key = "..."                    # Vercel AI key (or VERCEL_AI_API_KEY)
api_url = "https://..."            # Your Vercel AI Gateway URL (REQUIRED)
model = "gpt-4o"

[copilot]
enabled = false
# Uses GITHUB_TOKEN or ~/.config/github-copilot/hosts.json
model = "gpt-4o"

[perplexity]
enabled = false
api_key = "pplx-..."              # Perplexity key (or PERPLEXITY_API_KEY)
model = "sonar-pro"

[together]
enabled = false
api_key = "..."                    # Together AI key (or TOGETHER_API_KEY)
model = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo"

[fireworks]
enabled = false
api_key = "..."                    # Fireworks AI key (or FIREWORKS_API_KEY)
model = "accounts/fireworks/models/llama-v3p1-70b-instruct"

[sambanova]
enabled = false
api_key = "..."                    # SambaNova key (or SAMBANOVA_API_KEY)
model = "Meta-Llama-3.1-70B-Instruct"

[minimax]
enabled = false
api_key = "..."                    # MiniMax key (or MINIMAX_API_KEY)
model = "abab6.5s-chat"

# ── Failover (multi-provider chain) ──────────────────────────────

[failover]
chain = ["claude", "openai", "gemini"]   # Try providers in order

# ── Container Sandbox ─────────────────────────────────────────────

[sandbox]
runtime = "docker"                 # "docker", "podman", or "opensandbox"
image = "ubuntu:22.04"
network = false                    # Disable network inside sandbox
memory_limit = "512m"
cpu_limit = "1.0"

# ── Gateway Messaging ─────────────────────────────────────────────

# [[gateway]]
# platform = "telegram"
# bot_token = "..."
# whitelist = ["@username"]

# ── Email (Gmail or Outlook) ──────────────────────────────────────

[email]
provider = "gmail"              # "gmail" or "outlook"
access_token = ""               # OAuth2 access token
# refresh_token = ""            # Optional: used to auto-refresh expired tokens
# default_limit = 20            # Max messages returned per /email inbox call

# ── Calendar (Google Calendar or Outlook) ─────────────────────────

[calendar]
provider = "google"             # "google" or "outlook"
access_token = ""               # OAuth2 access token
# calendar_id = "primary"       # Google Calendar ID (default: "primary")
# timezone = "America/New_York" # Display timezone (default: system timezone)
# default_reminder_minutes = 15 # Add this reminder to every created event
# calendar_readonly = false     # Set true to prevent creates/deletes

# ── Home Assistant ─────────────────────────────────────────────────

[home_assistant]
url = "http://homeassistant.local:8123"   # Base URL of your HA instance
token = ""                                 # Long-lived access token
# insecure = false              # Set true to skip TLS cert verification (self-signed)

# ── Jira ──────────────────────────────────────────────────────────

[jira]
url = "https://yourorg.atlassian.net"   # Jira Cloud or Server base URL
email = ""                               # Atlassian account email
token = ""                               # API token from id.atlassian.com

# ── Notion ────────────────────────────────────────────────────────

notion_api_key = ""             # Notion integration secret (secret_xxx)

# ── Todoist ───────────────────────────────────────────────────────

todoist_api_key = ""            # Todoist API token (Todoist → Settings → Integrations)
```


## Environment Variables

API keys can be set as environment variables instead of (or in addition to) the config file. Environment variables take precedence.

| Variable | Provider |
|----------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `GEMINI_API_KEY` | Google Gemini |
| `GROK_API_KEY` | xAI Grok |
| `GROQ_API_KEY` | Groq |
| `OPENROUTER_API_KEY` | OpenRouter |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI |
| `AZURE_OPENAI_ENDPOINT` | Azure OpenAI endpoint URL |
| `AWS_ACCESS_KEY_ID` | AWS Bedrock |
| `AWS_SECRET_ACCESS_KEY` | AWS Bedrock |
| `AWS_REGION` | AWS Bedrock region |
| `MISTRAL_API_KEY` | Mistral |
| `CEREBRAS_API_KEY` | Cerebras |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `ZHIPU_API_KEY` | Zhipu |
| `OLLAMA_HOST` | Ollama base URL (overrides `api_url`) |
| `GITHUB_TOKEN` | GitHub Copilot + `@github:` context |
| `VERCEL_AI_API_KEY` | Vercel AI |
| `VERCEL_AI_GATEWAY_URL` | Vercel AI Gateway endpoint URL |
| `PERPLEXITY_API_KEY` | Perplexity |
| `TOGETHER_API_KEY` | Together AI |
| `FIREWORKS_API_KEY` | Fireworks AI |
| `SAMBANOVA_API_KEY` | SambaNova |
| `MINIMAX_API_KEY` | MiniMax |
| `JIRA_URL` | Jira instance URL, e.g. `https://myorg.atlassian.net` |
| `JIRA_EMAIL` | Jira account email (for basic auth) |
| `JIRA_API_TOKEN` | Jira API token (for basic auth) |
| `GMAIL_ACCESS_TOKEN` | Gmail OAuth2 access token |
| `OUTLOOK_ACCESS_TOKEN` | Microsoft Graph access token (Outlook Mail) |
| `GOOGLE_CALENDAR_TOKEN` | Google Calendar OAuth2 access token |
| `OUTLOOK_CALENDAR_TOKEN` | Microsoft Graph token (Outlook Calendar) |
| `HA_URL` | Home Assistant base URL (e.g. `http://homeassistant.local:8123`) |
| `HA_TOKEN` | Home Assistant long-lived access token |
| `NOTION_API_KEY` | Notion integration secret (`secret_xxx`) |
| `TODOIST_API_KEY` | Todoist API token |

**Example:**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
vibecli --tui --provider claude
```


## Provider Setup

### 1. Ollama — Local/Private Models (Default)

Ollama runs models locally on your machine. No API key needed, no data leaves your network.

1. Install Ollama: [ollama.ai](https://ollama.ai)

2. Pull a coding model:

   ```bash
   ollama pull qwen3-coder              # Default, strong coding
   ollama pull qwen2.5-coder:7b         # Compact, fast
   ollama pull deepseek-coder-v2:16b    # Strong code completion
   ollama pull codellama:13b            # Classic coding model
   ```

3. Confirm it's running:

   ```bash
   curl http://localhost:11434/api/tags
   ```

4. Configure (`~/.vibecli/config.toml`):

   ```toml
   [ollama]
   enabled = true
   api_url = "http://localhost:11434"
   model = "qwen3-coder:480b-cloud"
   ```

   Override the base URL with the `OLLAMA_HOST` env var if Ollama is running on a remote machine.


### 2. Anthropic Claude — Claude 4 Sonnet/Opus

1. Get an API key at [console.anthropic.com](https://console.anthropic.com/)

2. Configure:

   ```toml
   [claude]
   enabled = true
   model = "claude-sonnet-4-6"
   # Enable extended thinking (increases latency but improves reasoning):
   # thinking_budget_tokens = 10000
   ```

   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

   Available models: `claude-opus-4-6` (1M context, highest quality), `claude-sonnet-4-6` (200K, balanced), `claude-haiku-4-5` (200K, fast/cheap).

   CLI aliases: `claude`, `anthropic`


### 3. OpenAI — GPT-4o and Variants

1. Get an API key at [platform.openai.com](https://platform.openai.com/)

2. Configure:

   ```toml
   [openai]
   enabled = true
   model = "gpt-4o"
   ```

   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

   Available models: `gpt-4o` (default), `gpt-4o-mini`, `gpt-4-turbo`, `o1`, `o1-mini`.

   CLI aliases: `openai`, `gpt`


### 4. Google Gemini — Gemini 2.5 Pro/Flash

1. Get an API key at [aistudio.google.com](https://aistudio.google.com/)

2. Configure:

   ```toml
   [gemini]
   enabled = true
   model = "gemini-2.5-flash"
   ```

   ```bash
   export GEMINI_API_KEY="AIza..."
   ```

   Available models: `gemini-2.5-pro` (highest quality), `gemini-2.5-flash` (default, fast), `gemini-2.0-flash-lite` (cheapest).

   CLI aliases: `gemini`, `google`


### 5. xAI Grok — Grok 2

1. Get an API key at [x.ai](https://x.ai/)

2. Configure:

   ```toml
   [grok]
   enabled = true
   model = "grok-3-mini"
   ```

   ```bash
   export GROK_API_KEY="..."
   ```

   Available models: `grok-2`, `grok-3-mini` (default).

   CLI aliases: `grok`, `xai`


### 6. Groq — Fast Inference (Llama, Mixtral)

Groq runs open-source models on custom LPU hardware with extremely low latency.

1. Get an API key at [console.groq.com](https://console.groq.com/)

2. Configure:

   ```toml
   [groq]
   enabled = true
   model = "llama-3.3-70b-versatile"
   ```

   ```bash
   export GROQ_API_KEY="gsk_..."
   ```

   Available models: `llama-3.3-70b-versatile` (default), `llama-3.1-8b-instant`, `mixtral-8x7b-32768`, `gemma2-9b-it`.

   Free tier available with rate limits.


### 7. OpenRouter — Multi-Provider Gateway

OpenRouter aggregates 300+ models from multiple providers behind a single API key.

1. Get an API key at [openrouter.ai](https://openrouter.ai/)

2. Configure:

   ```toml
   [openrouter]
   enabled = true
   model = "anthropic/claude-3.5-sonnet"
   ```

   ```bash
   export OPENROUTER_API_KEY="sk-or-..."
   ```

   Models use the `organization/model-name` format. Browse available models at [openrouter.ai/models](https://openrouter.ai/models).


### 8. Azure OpenAI — Enterprise Azure-Hosted Models

For organizations using Azure-managed OpenAI deployments with enterprise compliance.

1. Create an Azure OpenAI resource in the [Azure Portal](https://portal.azure.com/)
2. Deploy a model (e.g., `gpt-4o`) and note the deployment name

3. Configure:

   ```toml
   [azure_openai]
   enabled = true
   api_url = "https://<resource>.openai.azure.com"
   model = "gpt-4o"       # Must match your deployment name
   ```

   ```bash
   export AZURE_OPENAI_API_KEY="..."
   export AZURE_OPENAI_ENDPOINT="https://<resource>.openai.azure.com"
   ```

   The `api_url` field is **required** — it must point to your Azure resource endpoint.

   CLI aliases: `azure`, `azure_openai`


### 9. AWS Bedrock — AWS-Hosted Models (Claude, Llama, Titan)

Uses your existing AWS credentials. No separate API key needed.

1. Enable Bedrock model access in the [AWS Console](https://console.aws.amazon.com/bedrock/)

2. Configure:

   ```toml
   [bedrock]
   enabled = true
   region = "us-east-1"
   model = "anthropic.claude-3-5-sonnet-20241022-v2:0"
   # role_arn = "arn:aws:iam::123456789:role/bedrock-role"  # Optional cross-account
   ```

   ```bash
   export AWS_ACCESS_KEY_ID="AKIA..."
   export AWS_SECRET_ACCESS_KEY="..."
   export AWS_REGION="us-east-1"
   # export AWS_SESSION_TOKEN="..."  # If using temporary credentials
   ```

   Available models: `anthropic.claude-3-5-sonnet-*`, `anthropic.claude-3-haiku-*`, `meta.llama3-*`, `amazon.titan-*`.

   CLI aliases: `bedrock`, `aws`, `aws-bedrock`


### 10. GitHub Copilot — Copilot Integration

Uses your existing GitHub Copilot subscription.

1. Ensure you have an active [GitHub Copilot](https://github.com/features/copilot) subscription

2. Configure:

   ```toml
   [copilot]
   enabled = true
   model = "gpt-4o"
   ```

   Authentication is resolved automatically in this order:
   1. `GITHUB_TOKEN` env var
   2. `~/.config/github-copilot/hosts.json` (from VS Code Copilot extension)
   3. `token` field in config

   VibeCody automatically exchanges your GitHub token for a short-lived Copilot API token.

   CLI aliases: `copilot`, `github-copilot`


### 11. LocalEdit — Local Code Editing Model

A wrapper around Ollama optimized for fill-in-middle (FIM) code completion using local GGUF models. Used internally by VibeUI for inline completions.

   ```toml
   [ollama]
   enabled = true
   api_url = "http://localhost:11434"
   model = "deepseek-coder:6.7b"   # Or any FIM-capable model
   ```

   No API key required. The model must support fill-in-middle prompting.


### 12. Mistral — Mistral AI Models

1. Get an API key at [console.mistral.ai](https://console.mistral.ai/)

2. Configure:

   ```toml
   [mistral]
   enabled = true
   model = "mistral-large-latest"
   ```

   ```bash
   export MISTRAL_API_KEY="..."
   ```

   Available models: `mistral-large-latest` (default), `mistral-medium-latest`, `mistral-small-latest`, `codestral-latest`.


### 13. Cerebras — Wafer-Scale Inference

Cerebras runs models on CS-3 wafer-scale hardware for ultra-fast inference.

1. Get an API key at [cloud.cerebras.ai](https://cloud.cerebras.ai/)

2. Configure:

   ```toml
   [cerebras]
   enabled = true
   model = "llama3.1-70b"
   ```

   ```bash
   export CEREBRAS_API_KEY="..."
   ```

   Available models: `llama3.1-70b` (default), `llama3.1-8b`.

   Free tier available with rate limits.


### 14. DeepSeek — DeepSeek V3/R1

Strong coding performance at very low prices.

1. Get an API key at [platform.deepseek.com](https://platform.deepseek.com/)

2. Configure:

   ```toml
   [deepseek]
   enabled = true
   model = "deepseek-chat"
   ```

   ```bash
   export DEEPSEEK_API_KEY="..."
   ```

   Available models: `deepseek-chat` (V3, default), `deepseek-reasoner` (R1, chain-of-thought reasoning).


### 15. Zhipu — GLM-4 Models

Chinese AI models from Zhipu AI (BigModel).

1. Get an API key at [open.bigmodel.cn](https://open.bigmodel.cn/)

2. Configure:

   ```toml
   [zhipu]
   enabled = true
   model = "glm-4"
   ```

   ```bash
   export ZHIPU_API_KEY="id.secret"   # Format: API ID dot Secret
   ```

   The API key uses a `id.secret` format for JWT-based authentication.

   CLI aliases: `zhipu`, `glm`


### 16. Vercel AI — Vercel AI SDK Gateway

A unified proxy that routes to multiple AI services through your Vercel deployment.

1. Deploy a Vercel AI Gateway and get your gateway URL and API key

2. Configure:

   ```toml
   [vercel_ai]
   enabled = true
   api_url = "https://your-gateway.vercel.app/api"   # REQUIRED
   model = "gpt-4o"
   ```

   ```bash
   export VERCEL_AI_API_KEY="..."
   export VERCEL_AI_GATEWAY_URL="https://your-gateway.vercel.app/api"
   ```

   The `api_url` field is **required** — it must point to your Vercel AI Gateway instance.

   CLI aliases: `vercel_ai`, `vercel`


### 17. MiniMax — MiniMax-Text-01

Chinese AI large language models.

1. Get an API key from [minimax.chat](https://api.minimax.chat/)

2. Configure:

   ```toml
   [minimax]
   enabled = true
   model = "abab6.5s-chat"
   ```

   ```bash
   export MINIMAX_API_KEY="..."
   ```

   Available models: `abab6.5s-chat` (default), `abab6.5-chat`, `MiniMax-Text-01`.


### 18. Perplexity — Search-Augmented Sonar Models

Combines LLM reasoning with real-time web search. Excellent for research tasks.

1. Get an API key at [perplexity.ai](https://www.perplexity.ai/)

2. Configure:

   ```toml
   [perplexity]
   enabled = true
   model = "sonar-pro"
   ```

   ```bash
   export PERPLEXITY_API_KEY="pplx-..."
   ```

   Available models: `sonar-pro` (default), `sonar`, `sonar-deep-research`, `sonar-reasoning-pro`.

   CLI aliases: `perplexity`, `pplx`


### 19. Together AI — Open Model Hosting (Llama, Qwen)

Hosts open-source models with competitive pricing and a free tier.

1. Get an API key at [together.ai](https://www.together.ai/)

2. Configure:

   ```toml
   [together]
   enabled = true
   model = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo"
   ```

   ```bash
   export TOGETHER_API_KEY="..."
   ```

   Models use the `organization/model-name` format. Browse available models at [api.together.ai/models](https://api.together.ai/models).

   CLI aliases: `together`, `together_ai`


### 20. Fireworks AI — Fast Open Model Inference

Fast inference platform for open-source models.

1. Get an API key at [fireworks.ai](https://fireworks.ai/)

2. Configure:

   ```toml
   [fireworks]
   enabled = true
   model = "accounts/fireworks/models/llama-v3p1-70b-instruct"
   ```

   ```bash
   export FIREWORKS_API_KEY="..."
   ```

   Free tier available with rate limits.

   CLI aliases: `fireworks`, `fireworks_ai`


### 21. SambaNova — Hardware-Accelerated Inference

Runs models on SambaNova's custom RDU hardware for fast inference.

1. Get an API key at [sambanova.ai](https://sambanova.ai/)

2. Configure:

   ```toml
   [sambanova]
   enabled = true
   model = "Meta-Llama-3.1-70B-Instruct"
   ```

   ```bash
   export SAMBANOVA_API_KEY="..."
   ```

   Free tier available with rate limits.

### `apiKeyHelper` — Rotating Credentials

For secrets management (Vault, 1Password CLI, AWS Secrets Manager, etc.), use a helper script instead of a static key:

```toml
[claude]
# Script is run before each API call; its stdout is used as the Bearer token.
# If the script exits non-zero, the static api_key is used as fallback.
api_key_helper = "~/.vibecli/get-key.sh claude"
```

Example helper script (`~/.vibecli/get-key.sh`):

```bash
#!/bin/bash
# $1 = provider name
case "$1" in
  claude)  op read "op://Personal/Anthropic/api_key" ;;
  openai)  aws secretsmanager get-secret-value --secret-id openai-key --query SecretString --output text ;;
  *)       echo "" ;;
esac
```


## Selecting a Provider

**VibeCLI — at launch:**

```bash
vibecli --tui --provider ollama          # Default (local)
vibecli --tui --provider claude          # Anthropic Claude
vibecli --tui --provider openai          # OpenAI GPT-4o
vibecli --tui --provider gemini          # Google Gemini
vibecli --tui --provider grok            # xAI Grok
vibecli --tui --provider groq            # Groq (fast Llama/Mixtral)
vibecli --tui --provider openrouter      # OpenRouter gateway
vibecli --tui --provider azure           # Azure OpenAI
vibecli --tui --provider bedrock         # AWS Bedrock
vibecli --tui --provider copilot         # GitHub Copilot
vibecli --tui --provider mistral         # Mistral AI
vibecli --tui --provider cerebras        # Cerebras
vibecli --tui --provider deepseek        # DeepSeek
vibecli --tui --provider zhipu           # Zhipu GLM
vibecli --tui --provider vercel          # Vercel AI Gateway
vibecli --tui --provider minimax         # MiniMax
vibecli --tui --provider perplexity      # Perplexity Sonar
vibecli --tui --provider together        # Together AI
vibecli --tui --provider fireworks       # Fireworks AI
vibecli --tui --provider sambanova       # SambaNova
vibecli --tui --provider failover        # Failover chain
```

Override the model for any provider:

```bash
vibecli --tui --provider claude --model claude-opus-4-6
vibecli --tui --provider openai --model gpt-4o-mini
vibecli --tui --provider groq --model mixtral-8x7b-32768
```

**VibeUI:**

Use the provider dropdown in the top bar. The selection is persisted across sessions.


## Provider Usage Examples

These examples show real-world workflows across different providers. Each example can be run from the command line or the REPL.

### One-Shot Chat

Ask a quick question without entering the REPL:

```bash
# Local (free, private)
vibecli --provider ollama "Explain the borrow checker in Rust"

# Cloud (higher quality)
vibecli --provider claude "Explain the borrow checker in Rust"
```

### Agent Mode — Fix a Bug

The agent reads files, edits code, and runs tests autonomously:

```bash
# Interactive mode — approve each step
vibecli --agent "Fix the login bug in src/auth.rs" --provider claude

# Example session output:
#   Agent   Fix the login bug in src/auth.rs
#     Policy: suggest (ask before every action)  |  Press Ctrl+C to stop
#
#    ✓ Reading src/auth.rs
#    ✓ Searching: "validate_token"
#
#      bash  Running: cargo check
#       Approve? (y/n/a=approve-all): y
#
#    ✓ Running: cargo check
#    ✓ Patching src/auth.rs (2 hunks)
#    ✓ Running: cargo test auth::tests
#
#   Agent complete: Fixed RS256/HS256 mismatch and hardcoded secret.
#      Files modified: src/auth.rs
#      Commands run: 2
#      Steps: 5/5 succeeded
#      Trace saved: ~/.vibecli/traces/1711234567.jsonl
```

```bash
# Non-interactive — auto-approve everything (CI/scripts)
vibecli --exec "Add error handling to all unwrap() calls in src/" --provider claude --full-auto

# Auto-edit — auto-approve file changes, prompt for shell commands
vibecli --agent "Refactor database module" --provider openai --auto-edit
```

### Agent Mode — Scaffold a Feature

```bash
vibecli --agent "Add a REST endpoint POST /api/users that validates email, \
  hashes the password with bcrypt, stores in SQLite, and returns 201" \
  --provider claude --model claude-opus-4-6
```

### Code Review

```bash
# Review a file
vibecli --provider claude "Review src/handler.rs for security issues and performance"

# Review a git diff
git diff HEAD~3 | vibecli --provider claude "Review this diff for bugs and suggest improvements"

# Review a GitHub PR
vibecli --provider claude "/review-pr 42"
```

### Multi-Provider Comparison

Send the same prompt to different providers and compare:

```bash
# Side-by-side comparison
vibecli --provider claude   "Write FizzBuzz in Rust" > /tmp/claude.txt
vibecli --provider openai   "Write FizzBuzz in Rust" > /tmp/openai.txt
vibecli --provider deepseek "Write FizzBuzz in Rust" > /tmp/deepseek.txt
diff /tmp/claude.txt /tmp/openai.txt

# Or use the Model Arena in the REPL
vibecli
> /arena "Write a binary search in Python" --providers claude,openai,gemini
```

### REPL — Interactive Session

```bash
vibecli
> What does this function do?
> [src/auth.rs]             # Attach a file for context
> Now add rate limiting to the validate_token function
> /model claude-opus-4-6    # Switch to a more capable model mid-session
> /cost                     # Check session token costs
> /quit
```

### Provider-Specific Strengths

**Ollama — Fully offline, unlimited use:**

```bash
# Pull a model and chat immediately (no API key, no cost)
ollama pull qwen3-coder
vibecli --provider ollama --model qwen3-coder "Optimize this SQL query: SELECT * FROM users WHERE ..."

# Remote Ollama server
OLLAMA_HOST=http://gpu-server:11434 vibecli --provider ollama "Explain this stack trace"
```

**Claude — Extended thinking for hard problems:**

```toml
# ~/.vibecli/config.toml
[claude]
enabled = true
model = "claude-sonnet-4-6"
thinking_budget_tokens = 16000   # AI "thinks" internally before answering
```

```bash
# Architecture decisions, complex debugging, security analysis
vibecli --provider claude --model claude-opus-4-6 \
  "Design a migration strategy from our monolith to microservices. \
   Here is the current architecture:" --add-dir ./src/
```

**Gemini — Massive context window:**

```bash
# Feed an entire codebase (Gemini supports up to 2M tokens)
vibecli --provider gemini --model gemini-2.5-pro \
  "Summarize the architecture of this project and identify dead code" \
  --add-dir ./src/
```

**Groq / Cerebras / SambaNova — Ultra-fast inference:**

```bash
# Near-instant responses for quick iterations
vibecli --provider groq "Convert this JSON to a Rust struct: {\"name\": \"Alice\", \"age\": 30}"
vibecli --provider cerebras "Write a regex to match email addresses"
vibecli --provider sambanova "Explain this error: cannot borrow as mutable"
```

**DeepSeek — Budget-friendly coding:**

```bash
# Strong coding quality at ~1/10th the price of Claude/GPT-4o
vibecli --provider deepseek "Write comprehensive unit tests for src/auth.rs"
vibecli --provider deepseek --model deepseek-reasoner "Debug this race condition" # R1 reasoning
```

**Perplexity — Search-augmented answers:**

```bash
# Answers grounded in real-time web search
vibecli --provider perplexity "What are the latest breaking changes in Tokio 1.40?"
vibecli --provider perplexity "Compare axum vs actix-web performance benchmarks 2026"
```

**OpenRouter — Access any model:**

```bash
# Try niche or new models via unified API
vibecli --provider openrouter --model "meta-llama/llama-3.3-70b" "Explain monads"
vibecli --provider openrouter --model "google/gemini-2.5-pro" "Review this code"
vibecli --provider openrouter --model "deepseek/deepseek-r1" "Solve this optimization"
```

**Failover — Automatic resilience:**

```toml
# ~/.vibecli/config.toml — tries each provider in order
[failover]
chain = ["claude", "openai", "gemini", "ollama"]
```

```bash
# If Claude is rate-limited, automatically falls back to OpenAI, then Gemini, then local Ollama
vibecli --provider failover --agent "Fix the build errors in src/"
```

### Enterprise Examples

**Azure OpenAI — Corporate proxy with compliance:**

```bash
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_ENDPOINT="https://mycompany.openai.azure.com"
vibecli --provider azure --model gpt-4o "Audit this code for OWASP top 10 vulnerabilities"
```

**AWS Bedrock — IAM-authenticated, no API keys in code:**

```bash
# Uses your AWS CLI credentials or instance role
aws sso login --profile production
vibecli --provider bedrock "Generate a CloudFormation template for an ECS Fargate service"
```

**GitHub Copilot — Use your existing subscription:**

```bash
# No extra API key needed if you have VS Code Copilot configured
vibecli --provider copilot "Complete this function" --file src/parser.rs
```

### Batch Processing & CI/CD

```bash
# One-shot in CI pipelines
vibecli --exec "Check this diff for security issues" --provider claude --full-auto < pr.diff

# Process multiple files
for f in src/*.rs; do
  vibecli --provider groq "Summarize what this file does" --file "$f" >> summary.md
done

# JSON output for scripts
vibecli --provider claude --json "List all TODO comments in src/" | jq '.items[]'
```

### Vision (Image Analysis)

```bash
# Analyze screenshots, diagrams, or UI mockups (Claude, OpenAI, Gemini)
vibecli --provider claude "What's wrong with this UI?" --image ./screenshot.png
vibecli --provider gemini "Convert this wireframe to React components" --image ./mockup.png
```

### Cost Tracking

```bash
vibecli
> /cost
Session cost summary:
  claude:   $0.0342 (12,400 tokens)
  openai:   $0.0128 (5,200 tokens)
  ollama:   $0.0000 (8,100 tokens)  [local]
  Total:    $0.0470
```


## Safety Settings

VibeCLI has a built-in approval gate for potentially destructive actions.

| Setting | Default | Description |
|---------|---------|-------------|
| `require_approval_for_commands` | `true` | Prompt `y/N` before any shell execution |
| `require_approval_for_file_changes` | `true` | Prompt `y/N` before applying AI file edits |
| `approval_policy` | `"suggest"` | `"suggest"` \| `"auto-edit"` \| `"full-auto"` |
| `denied_tools` | `[]` | Exact tool names to always block |
| `denied_tool_patterns` | `[]` | Patterns like `"bash(rm*)"` to block tool+argument combos |

### Approval Policies

| Policy | Behavior |
|--------|----------|
| `suggest` | Prompts before every tool call (default) |
| `auto-edit` | Auto-approves file writes; prompts for shell commands |
| `full-auto` | Approves all tool calls without prompting (use with caution) |

Override at launch: `vibecli --full-auto`, `vibecli --auto-edit`, `vibecli --suggest`

### Wildcard Tool Patterns

Block granular tool+argument combinations without blocking the entire tool:

```toml
[safety]
# Block bash calls matching rm* but allow everything else
denied_tool_patterns = ["bash(rm*)", "bash(sudo*)"]

# Still block specific tools entirely
denied_tools = ["execute_python"]
```

**Disable approvals for trusted environments:**

```toml
[safety]
require_approval_for_commands = false
require_approval_for_file_changes = false
approval_policy = "full-auto"
```

> **Warning:** Disabling command approval allows VibeCLI to execute AI-suggested commands without confirmation. Only disable in trusted, sandboxed environments.


## Auto Memory Recording

VibeCLI can automatically summarize completed agent sessions and append key learnings to `~/.vibecli/memory.md`.

```toml
[memory]
auto_record = true         # Enable auto-recording (default: false)
min_session_steps = 3      # Only record sessions with ≥ N tool calls
```

After a session completes, the LLM generates 1–3 concise bullet points and appends them:

```text
<!-- auto-recorded 2026-02-24 -->
- Always check for existing tests before adding new ones
- Use `cargo check` before `cargo build` to catch compile errors faster
```

The memory file is automatically injected into future agent system prompts.


## Rules Directory

Place `.md` files in `.vibecli/rules/` (project-level) or `~/.vibecli/rules/` (global) to inject persistent instructions into the agent system prompt.

Rules support optional YAML-style front-matter to restrict injection to specific file types:

```text
name: rust-safety
path_pattern: "**/*.rs"
When editing Rust files, prefer `?` over `unwrap()`.
Always add `#[must_use]` to functions returning Result or Option.
```

Rules without a `path_pattern` always inject. Rules with a pattern only inject when a matching file is open in the session's workspace.


## Productivity Integrations

VibeCLI connects to external services for email, calendar, tasks, knowledge, project tracking, and smart home control. Configure each under the corresponding section in `config.toml` or via environment variables.

### Email (Gmail / Outlook)

```toml
[email]
provider     = "gmail"    # "gmail" | "outlook"
access_token = "ya29.xxxx"
```

**Getting a Gmail token**: Use the Google OAuth2 Playground (`developers.google.com/oauthplayground`) with scope `https://www.googleapis.com/auth/gmail.modify`, or run `vibecli --setup --email`.

**Getting an Outlook token**: Use the Microsoft OAuth2 flow with scope `https://graph.microsoft.com/Mail.ReadWrite`.

| REPL Command | Description |
|---|---|
| `/email unread` | List unread messages |
| `/email inbox` | Last 20 inbox messages |
| `/email read <id>` | Read full message body |
| `/email send <to> <subject> <body>` | Send an email |
| `/email search <query>` | Search messages |
| `/email triage` | AI-assisted triage |
| `/email archive <id>` | Archive a message |

### Calendar (Google / Outlook)

```toml
[calendar]
provider     = "google"   # "google" | "outlook"
access_token = "ya29.xxxx"
timezone     = "America/New_York"
```

| REPL Command | Description |
|---|---|
| `/cal today` | Today's events |
| `/cal week` | This week's events |
| `/cal create <title> <start> <end>` | Create event (natural language time) |
| `/cal free [date]` | Find free slots |
| `/cal next` | Next upcoming event |
| `/cal move <id> <start>` | Reschedule event |

### Home Assistant

```toml
[home_assistant]
url   = "http://homeassistant.local:8123"
token = "eyJ0..."  # Settings → Profile → Long-Lived Access Tokens
```

| REPL Command | Description |
|---|---|
| `/ha status` | All entity states |
| `/ha on <entity>` / `/ha off <entity>` | Turn entity on/off |
| `/ha scene <name>` | Activate a scene |
| `/ha climate <entity> <temp>` | Set thermostat |
| `/ha history <entity> [hours]` | State history |

### Todoist

```toml
todoist_api_key = "xxxx"   # Todoist → Settings → Integrations → API token
```

| REPL Command | Description |
|---|---|
| `/todo today` | Tasks due today + overdue |
| `/todo list` | All active tasks |
| `/todo add <task> due:<date> p:<1-4>` | Add a task |
| `/todo close <id>` | Complete a task |

### Notion

```toml
notion_api_key = "secret_xxxx"   # notion.so/my-integrations
```

Share pages/databases with the integration in Notion before they appear in search results.

| REPL Command | Description |
|---|---|
| `/notion search <query>` | Search workspace |
| `/notion get <page-id>` | Read a page |
| `/notion append <page-id> <text>` | Append to a page |
| `/notion databases` | List accessible databases |

### Jira

```toml
[jira]
url   = "https://yourorg.atlassian.net"
email = "you@yourorg.com"
token = "ATATT3xxx"   # id.atlassian.com → Security → API tokens
```

| REPL Command | Description |
|---|---|
| `/jira mine` | My open issues |
| `/jira list [project]` | List open issues |
| `/jira create <project> <summary>` | Create issue |
| `/jira transition <key> <status>` | Move to status |
| `/jira search <jql>` | JQL query |

---

## Command History

VibeCLI saves REPL command history to `~/.vibecli/history.txt`. This file is created automatically and persists across sessions. History is navigable with the Up/Down arrow keys in the REPL.
