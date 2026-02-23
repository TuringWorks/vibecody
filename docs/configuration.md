---
layout: page
title: Configuration Guide
permalink: /configuration/
---

# Configuration Guide

VibeCody uses TOML-based configuration for VibeCLI. VibeUI provider settings are managed through environment variables or the in-app settings UI.

---

## VibeCLI Configuration

**Location:** `~/.vibecli/config.toml`

The file is created automatically with defaults on first run. You can also create it manually.

### Full Reference

```toml
# ── Providers ──────────────────────────────────────────────────────

[ollama]
enabled = true
api_url = "http://localhost:11434"   # Local Ollama endpoint
model = "qwen2.5-coder:7b"          # Any model pulled via 'ollama pull'

[claude]
enabled = false
api_key = "sk-ant-..."              # Anthropic API key
model = "claude-3-5-sonnet-20241022"
# model = "claude-3-opus-20240229"

[openai]
enabled = false
api_key = "sk-..."                  # OpenAI API key
model = "gpt-4o"
# model = "gpt-4-turbo"
# model = "gpt-3.5-turbo"

[gemini]
enabled = false
api_key = "AIza..."                 # Google AI Studio API key
model = "gemini-1.5-pro"
# model = "gemini-1.5-flash"

[grok]
enabled = false
api_key = "..."                     # xAI API key
model = "grok-beta"

# ── UI ─────────────────────────────────────────────────────────────

[ui]
theme = "dark"   # "dark" or "light"

# ── Safety ─────────────────────────────────────────────────────────

[safety]
require_approval_for_commands = true      # Prompt before running shell commands
require_approval_for_file_changes = true  # Prompt before applying AI file edits
```

---

## Environment Variables

API keys can be set as environment variables instead of (or in addition to) the config file. Environment variables take precedence.

| Variable | Provider |
|----------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `GEMINI_API_KEY` | Google Gemini |
| `GROK_API_KEY` | xAI Grok |
| `OLLAMA_HOST` | Ollama base URL (overrides `api_url`) |

**Example:**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
vibecli --tui --provider claude
```

---

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
   model = "claude-3-5-sonnet-20241022"
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
   model = "gemini-1.5-pro"
   ```

### xAI Grok

1. Get an API key at [x.ai](https://x.ai/)

2. Configure:
   ```toml
   [grok]
   enabled = true
   api_key = "..."
   model = "grok-beta"
   ```

---

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

---

## Safety Settings

VibeCLI has a built-in approval gate for potentially destructive actions.

| Setting | Default | Behavior when `true` |
|---------|---------|---------------------|
| `require_approval_for_commands` | `true` | Prompts `y/N` before any shell execution (via `!`, `/exec`, or AI-suggested `execute` blocks) |
| `require_approval_for_file_changes` | `true` | Prompts `y/N` before writing AI-generated content to disk (via `/apply`) |

**Disable approvals for trusted environments:**

```toml
[safety]
require_approval_for_commands = false
require_approval_for_file_changes = false
```

> **Warning:** Disabling command approval allows VibeCLI to execute AI-suggested commands without confirmation. Only disable in trusted, sandboxed environments.

---

## Command History

VibeCLI saves REPL command history to `~/.vibecli/history.txt`. This file is created automatically and persists across sessions. History is navigable with the Up/Down arrow keys in the REPL.
