---
layout: page
title: "Demo 1: First Run & Setup"
permalink: /demos/first-run/
nav_order: 1
parent: Demos
---


## Overview

This demo walks you through installing VibeCody, configuring your first AI provider, and sending your first chat message. By the end, you will have a working VibeCLI installation connected to an AI provider of your choice.

**Time to complete:** ~5 minutes

## Prerequisites

- macOS, Linux, or Windows (WSL2)
- Rust toolchain (1.75+) if building from source, or Docker
- An API key for at least one AI provider (Ollama works offline without a key)

## Step-by-Step Walkthrough

### Step 1: Install VibeCody

Choose one of three installation methods.

**Option A: One-liner installer (recommended)**

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | bash
```

The installer downloads a pre-built binary for your platform and verifies the SHA-256 checksum before placing it in your PATH.

**Option B: Build from source with Cargo**

```bash
cargo install --git https://github.com/TuringWorks/vibecody.git vibecli
```

Or clone and build locally:

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
cargo build --release -p vibecli
# Binary is at target/release/vibecli
```

**Option C: Docker**

```bash
docker pull ghcr.io/turingworks/vibecody:latest
docker run -it --rm \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/turingworks/vibecody:latest "Hello"
```

For air-gapped environments with a local Ollama sidecar:

```bash
docker compose up -d
```

### Step 2: Verify the installation

```bash
vibecli --version
```

Expected output:

```
vibecli 0.5.1
```

### Step 3: Set up API keys

You can provide API keys via environment variables or the config file.

**Environment variables (quick start):**

```bash
# Pick one (or more) provider
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="AIza..."
export GROK_API_KEY="xai-..."
```

**Config file (persistent):**

VibeCLI stores its configuration at `~/.vibecli/config.toml`. Create or edit it:

```bash
mkdir -p ~/.vibecli
cat > ~/.vibecli/config.toml << 'EOF'
[claude]
enabled = true
api_key = "sk-ant-..."
model = "claude-sonnet-4-6"

[openai]
enabled = true
api_key = "sk-..."
model = "gpt-4o"
EOF
```

**Using Ollama (fully offline, no key needed):**

```bash
# Install Ollama from https://ollama.com
ollama pull llama3
```

Then set Ollama as the default:

```bash
cat > ~/.vibecli/config.toml << 'EOF'
[ollama]
enabled = true
model = "llama3"
EOF
```

### Step 4: Send your first message

```bash
vibecli chat "Hello! What can you help me with?"
```

You should see a streaming response from your configured AI provider.

<!-- Screenshot placeholder: terminal showing vibecli chat response -->

### Step 5: Configure the default provider

Switch your default provider at any time:

```bash
# Via CLI flag
vibecli --provider openai "Explain quicksort"

# Via REPL (just run vibecli with no arguments to enter the REPL)
vibecli
> /model codellama
```

### Step 6: Explore help

View all available commands:

```bash
vibecli --help
```

```
AI-powered coding assistant for the terminal

Usage: vibecli [OPTIONS] [MESSAGE]...

Arguments:
  [MESSAGE]...  One-shot chat message

Options:
  -p, --provider <PROVIDER>  AI provider [default: ollama]
  -m, --model <MODEL>        Model name
      --tui                  Launch terminal UI
      --agent <TASK>         Run agent on a task
      --exec <TASK>          CI/non-interactive mode
      --serve                Start HTTP daemon
      --review               Run code review
      --doctor               Health check
      --help                 Print help
      --version              Print version
```

Inside the REPL, type `/help` for interactive commands:

```bash
vibecli
> /help
```

```
Available REPL Commands:
  /help           Show this help message
  /model          Switch model
  /clear          Clear conversation
  /history        Show conversation history
  /save           Save conversation
  /load           Load conversation
  /tools          List available tools
  /hooks          Manage agent hooks
  /orchestrate    Workflow orchestration
  /sessions       Session management
  /quit           Exit REPL
```

### Step 7: Start the HTTP daemon (optional)

Run VibeCLI as a persistent background service:

```bash
vibecli --serve --port 7878 --provider ollama
```

Test the health endpoint:

```bash
curl http://localhost:7878/health
```

```json
{
  "status": "ok",
  "version": "0.5.1",
  "provider": "ollama"
}
```

## VibeUI Setup

If you want the desktop IDE experience:

```bash
cd vibeui
npm install
npm run tauri dev
```

VibeUI shares the same `~/.vibecli/config.toml` configuration, so any provider setup you did above carries over.

<!-- Screenshot placeholder: VibeUI main window after first launch -->

## Demo Recording

The following JSON recording can be used for automated demo playback with VibeCody's built-in demo runner.

```json
{
  "meta": {
    "title": "First Run & Setup",
    "description": "Install VibeCody, configure an AI provider, and send your first chat message.",
    "duration_seconds": 120,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --version",
      "description": "Verify VibeCLI is installed",
      "expected_output_contains": "vibecli",
      "delay_ms": 1000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "mkdir -p ~/.vibecli",
      "description": "Create config directory",
      "delay_ms": 500
    },
    {
      "id": 3,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[ollama]\nenabled = true\nmodel = \"llama3\"\n",
      "description": "Write default provider config",
      "delay_ms": 500
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli chat \"Hello! What can you help me with?\"",
      "description": "Send first chat message",
      "expected_output_contains": "help",
      "delay_ms": 5000,
      "typing_speed_ms": 50
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli --help",
      "description": "Explore available commands",
      "expected_output_contains": "Options",
      "delay_ms": 2000
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/help", "delay_ms": 2000 },
        { "input": "What is the capital of France?", "delay_ms": 4000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Interactive REPL session exploring help and sending a message"
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli --serve --port 7878 --provider ollama &",
      "description": "Start HTTP daemon in background",
      "delay_ms": 2000
    },
    {
      "id": 8,
      "action": "shell",
      "command": "curl -s http://localhost:7878/health | python3 -m json.tool",
      "description": "Verify daemon health endpoint",
      "expected_output_contains": "\"status\": \"ok\"",
      "delay_ms": 1500
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `vibecli: command not found` | Add the install directory to your PATH: `export PATH="$HOME/.cargo/bin:$PATH"` |
| `Provider error: missing API key` | Set the appropriate environment variable or add it to `~/.vibecli/config.toml` |
| `Connection refused` on Ollama | Make sure Ollama is running: `ollama serve` |
| Docker permission denied | Run with `sudo` or add your user to the `docker` group |

## What's Next

- [Demo 2: TUI Interface](../02-tui-interface/) -- Learn to navigate the terminal UI
- [Demo 3: Multi-Provider Chat](../03-multi-provider-chat/) -- Switch between 23 AI providers
- [Demo 4: Agent Loop](../04-agent-loop/) -- Let the AI edit files and run commands
