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
model = "gemini-2.0-flash"
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
| `GITHUB_TOKEN` | GitHub personal access token (for `@github:` context) |
| `GITHUB_COPILOT_TOKEN` | GitHub Copilot |
| `JIRA_BASE_URL` | Jira instance URL, e.g. `https://myorg.atlassian.net` |
| `JIRA_EMAIL` | Jira account email (for basic auth) |
| `JIRA_API_TOKEN` | Jira API token (for basic auth) |

**Example:**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
vibecli --tui --provider claude
```


## Provider Setup

### Ollama (Local — Recommended for Privacy)

1. Install Ollama: [ollama.ai](https://ollama.ai)

2. Pull a coding model:

   ```bash
   ollama pull qwen2.5-coder:7b        # Compact, fast
   ollama pull codellama:13b           # Classic coding model
   ollama pull deepseek-coder-v2:16b  # Strong code completion
   ```

3. Confirm it's running:

   ```bash
   curl http://localhost:11434/api/tags
   ```

4. Configure:

   ```toml
   [ollama]
   enabled = true
   api_url = "http://localhost:11434"
   model = "qwen2.5-coder:7b"
   ```

### Anthropic Claude

1. Get an API key at [console.anthropic.com](https://console.anthropic.com/)

2. Configure:

   ```toml
   [claude]
   enabled = true
   api_key = "sk-ant-..."
   model = "claude-sonnet-4-6"
   # Enable extended thinking (increases latency but improves reasoning):
   # thinking_budget_tokens = 10000
   ```

   Or via environment:

   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

### OpenAI

1. Get an API key at [platform.openai.com](https://platform.openai.com/)

2. Configure:

   ```toml
   [openai]
   enabled = true
   api_key = "sk-..."
   model = "gpt-4o"
   ```

### Google Gemini

1. Get an API key at [aistudio.google.com](https://aistudio.google.com/)

2. Configure:

   ```toml
   [gemini]
   enabled = true
   api_key = "AIza..."
   model = "gemini-2.0-flash"
   ```

### xAI Grok

1. Get an API key at [x.ai](https://x.ai/)

2. Configure:

   ```toml
   [grok]
   enabled = true
   api_key = "..."
   model = "grok-3-mini"
   ```

### Perplexity (Search-Augmented AI)

1. Get an API key at [perplexity.ai](https://www.perplexity.ai/)

2. Configure:

   ```toml
   [perplexity]
   api_key = "pplx-..."
   model = "sonar-pro"     # Also: sonar, sonar-deep-research, sonar-reasoning-pro
   ```

### Together AI (Open Model Hosting)

1. Get an API key at [together.ai](https://www.together.ai/)

2. Configure:

   ```toml
   [together]
   api_key = "..."
   model = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo"
   ```

### Fireworks AI (Fast Open Model Inference)

1. Get an API key at [fireworks.ai](https://fireworks.ai/)

2. Configure:

   ```toml
   [fireworks]
   api_key = "..."
   model = "accounts/fireworks/models/llama-v3p1-70b-instruct"
   ```

### SambaNova (Hardware-Accelerated Inference)

1. Get an API key at [sambanova.ai](https://sambanova.ai/)

2. Configure:

   ```toml
   [sambanova]
   api_key = "..."
   model = "Meta-Llama-3.1-70B-Instruct"
   ```

### MiniMax (Chinese AI Models)

1. Get an API key from [minimax.chat](https://api.minimax.chat/)

2. Configure:

   ```toml
   [minimax]
   api_key = "..."
   model = "abab6.5s-chat"    # Also: abab6.5-chat, MiniMax-Text-01
   ```

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
vibecli --tui --provider ollama     # Default
vibecli --tui --provider claude
vibecli --tui --provider openai --model gpt-4o
vibecli --tui --provider gemini
vibecli --tui --provider grok
```

**VibeUI:**

Use the provider dropdown in the top bar. The selection is persisted across sessions.


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


## Command History

VibeCLI saves REPL command history to `~/.vibecli/history.txt`. This file is created automatically and persists across sessions. History is navigable with the Up/Down arrow keys in the REPL.
