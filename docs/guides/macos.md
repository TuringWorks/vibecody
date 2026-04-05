---
layout: page
title: "macOS"
permalink: /guides/macos/
parent: Deployment Guides
---

# Deploy VibeCody on macOS

Works on MacBook (Air/Pro) and Mac Mini/Studio. Apple Silicon gets Metal GPU acceleration for fast local inference.

**Setup time:** 2 minutes | **Cost:** Free | **GPU:** Metal (Apple Silicon)

## Quick Start

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
vibecli
```

Or use the guided setup:

```bash
cd vibecody/deploy/macos
./setup.sh --always-on
```

## Step-by-Step

### 1. Install VibeCLI

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

### 2. Install Ollama (Optional)

```bash
brew install ollama
ollama serve &
ollama pull codellama:7b
```

Apple Silicon Macs run Ollama with Metal GPU acceleration — a MacBook Pro M3 can run 13B models at ~30 tokens/sec.

### 3. Start Chatting

```bash
vibecli
```

### 4. Enable Always-On (Optional)

Install VibeCody as a launchd service that starts at login:

```bash
./setup.sh --always-on
# Or manually:
vibecli --service install
vibecli --service start
```

Access at: `http://localhost:7878`

### Mac Mini as a Server

A Mac Mini with Apple Silicon is the best desktop "server" — silent, efficient, and Metal GPU runs 13B models at full speed.

```bash
# Headless setup with remote access
./setup.sh --always-on
vibecli --serve --tailscale  # Expose via Tailscale
```

## Model Recommendations

| Mac | RAM | Recommended Model | Performance |
|-----|-----|------------------|-------------|
| MacBook Air M1 | 8 GB | codellama:7b | ~15 tok/s |
| MacBook Pro M3 | 18 GB | codellama:13b | ~30 tok/s |
| Mac Mini M2 Pro | 32 GB | codellama:34b | ~20 tok/s |
| Mac Studio M2 Ultra | 64 GB+ | llama3:70b | ~15 tok/s |

## Uninstall

```bash
./uninstall.sh
# Or: vibecli --service stop && rm ~/.local/bin/vibecli
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "command not found" | Add `~/.local/bin` to PATH: `export PATH="$HOME/.local/bin:$PATH"` |
| Ollama slow | Ensure Ollama is using Metal — check with `ollama ps` |
| Service won't start | Check logs: `cat /tmp/vibecody-stderr.log` |
| Intel Mac | Works fine but no GPU acceleration — use cloud providers for large models |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
