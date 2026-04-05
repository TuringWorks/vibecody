---
layout: page
title: "Demo 57: Easy Setup & Deployment"
permalink: /demos/57-easy-setup/
nav_order: 57
parent: Demos
---

## Overview

VibeCody can be deployed on any platform — cloud, desktop, or edge device — with a single command. This demo walks through the interactive setup wizard, always-on service installation, and multi-platform deployment. Think of it as your own self-hosted AI coding assistant, like myclaw.ai but free and running on your hardware.

**Time to complete:** 5–15 minutes depending on platform

## Prerequisites

- A machine running macOS, Linux, Windows, or a Raspberry Pi 3/4/5
- (Optional) Docker installed for container-based deployment
- (Optional) Cloud CLI tools (aws, gcloud, az) for cloud deployment

## Step-by-Step Walkthrough

### Step 1: Run the Setup Wizard

The setup wizard auto-detects your platform, RAM, GPU, and recommends the optimal configuration.

```bash
vibecli --setup
```

Expected output:

```
┌─ VibeCody Setup Wizard ─────────────────────────────────┐
│                                                          │
│  ✓ Platform:  macOS (aarch64)                            │
│  ✓ Memory:    36.0 GB                                    │
│  ✓ GPU:       Apple Silicon (Metal)                      │
│  ✓ Hostname:  macbook-pro                                │
│  ✓ Tier:      max (recommended)                          │
│                                                          │
└──────────────────────────────────────────────────────────┘

? Choose your AI provider:
  ▸ ollama   Local models — free, private, no API key needed
    claude   Anthropic Claude — best for complex coding tasks
    openai   OpenAI GPT — widely used, fast
    gemini   Google Gemini — good free tier
    grok     xAI Grok — fast, generous rate limits
    groq     Groq — ultra-fast inference for open models

Enter choice (1-6, default 1): 1

  ✓ Recommended model for your hardware: codellama:13b

? Pull codellama:13b now? (this may take a few minutes) [Y/n] y
  Running: ollama pull codellama:13b

? Enable always-on mode (run VibeCody as a background service)? [y/N] y
  ✓ Created ~/Library/LaunchAgents/com.vibecody.vibecli.plist
  ✓ Service loaded — VibeCody is running at http://localhost:7878

  Checking VibeCody health... ✓ Healthy

┌─ Setup Complete ────────────────────────────────────────┐
│                                                         │
│  ✓ Platform:  macOS (aarch64)                           │
│  ✓ Provider:  ollama (codellama:13b)                    │
│  ✓ Always-on: http://localhost:7878                     │
│                                                         │
│  Next steps:                                            │
│    vibecli                    # Start chatting           │
│    vibecli --agent "fix bugs" # Run an agent            │
│    vibecli --review           # Review code              │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Step 2: Manage the Background Service

Once installed, use the `/service` REPL command or `--service` CLI flag to manage the always-on daemon.

```bash
# Check status
vibecli --service status

# Stop the service
vibecli --service stop

# Start the service
vibecli --service start
```

Or from the REPL:

```
vibecli> /service status
● vibecody.service - VibeCody AI Coding Assistant
   Active: active (running) since Fri 2026-04-04 10:30:00 PDT

vibecli> /service stop
✓ VibeCody service stopped
```

### Step 3: Deploy to a Cloud Platform

For always-on team use, deploy to any cloud provider with one command.

```bash
# Clone the repo
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody

# AWS (ECS Fargate)
./deploy/aws/setup.sh --tier pro

# Google Cloud (Cloud Run)
./deploy/gcp/setup.sh --tier pro

# Azure (Container Apps)
./deploy/azure/setup.sh --tier pro

# Oracle Cloud (FREE tier!)
./deploy/oracle-cloud/setup.sh --tier lite

# DigitalOcean
./deploy/digitalocean/setup.sh --tier lite

# Linode/Akamai
./deploy/linode-akamai/setup.sh --tier lite
```

Each script will:
1. Validate cloud CLI authentication
2. Provision infrastructure (container service, storage, networking)
3. Deploy VibeCody with Ollama sidecar
4. Print the access URL

### Step 4: Deploy on a Raspberry Pi

VibeCody runs on Raspberry Pi 3, 4, and 5 — perfect for a privacy-first homelab AI assistant.

```bash
# On the Raspberry Pi:
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/raspberry-pi/setup.sh | sh
```

The Pi setup script:
- Detects your Pi model and available RAM
- Installs the ARM64 binary
- Recommends the right model size (TinyLlama for Pi 3, Mistral 7B for Pi 5)
- Sets up a systemd service for always-on operation
- Optionally configures Cloudflare Tunnel or Tailscale for remote access

### Step 5: Access from Anywhere

Once VibeCody is running as an always-on service, connect from anywhere:

**Via direct HTTP:**
```bash
curl http://your-server:7878/health
```

**Via Tailscale (private network):**
```bash
vibecli --serve --tailscale
# Access from any device on your Tailscale network:
curl https://vibecody.your-tailnet.ts.net/health
```

**Via messaging gateway (Slack, Discord, Telegram):**
```bash
vibecli gateway enable telegram
vibecli gateway enable discord
vibecli gateway enable slack
```

Now you can chat with VibeCody from your phone via Telegram, your team via Slack, or your community via Discord.

### Step 6: Connect Multiple Platforms

Set up VibeCody on multiple devices with shared memory:

```bash
# On your Mac (main workstation)
vibecli --setup

# On your Raspberry Pi (always-on home server)
./deploy/raspberry-pi/setup.sh --always-on

# On AWS (team CI/CD integration)
./deploy/aws/setup.sh --tier pro
```

## Deployment Matrix

| Platform | Setup Time | Cost | Always-On | GPU | Local Models | Remote Access |
|----------|-----------|------|-----------|-----|-------------|--------------|
| macOS | 2 min | Free | launchd | Metal | Up to 70B | Tailscale |
| Linux | 2 min | Free | systemd | CUDA/ROCm | Up to 70B | Tailscale |
| Windows | 3 min | Free | Service | CUDA | Up to 70B | Tailscale |
| AWS | 10 min | $15-60/mo | Auto | — | Via Ollama | Public URL |
| GCP | 10 min | $10-50/mo | Auto | — | Via Ollama | Public URL |
| Azure | 10 min | $15-55/mo | Auto | — | Via Ollama | Public URL |
| Oracle Cloud | 10 min | **$0** | Auto | — | Via Ollama | Public URL |
| DigitalOcean | 5 min | $12-48/mo | Auto | — | Via Ollama | Public URL |
| Linode | 5 min | $12-48/mo | Auto | — | Via Ollama | Public URL |
| Raspberry Pi 5 | 5 min | Free | systemd | — | Up to 7B | Tunnel |
| Raspberry Pi 4 | 5 min | Free | systemd | — | Up to 3-7B | Tunnel |
| Raspberry Pi 3 | 10 min | Free | systemd | — | TinyLlama | Tunnel |

## Demo Recording

```json
{
  "meta": {
    "title": "Easy Setup & Deployment",
    "description": "Deploy VibeCody anywhere with one command — cloud, desktop, or Raspberry Pi.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --setup",
      "description": "Run the interactive setup wizard",
      "delay_ms": 15000
    },
    {
      "id": 2,
      "action": "Narrate",
      "value": "The setup wizard detected macOS with Apple Silicon, 36 GB RAM, and Metal GPU. It recommended the max tier and codellama:13b model."
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli --service status",
      "description": "Check that the always-on service is running",
      "delay_ms": 2000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "curl -s http://localhost:7878/health | jq .",
      "description": "Verify the health endpoint responds",
      "delay_ms": 2000
    },
    {
      "id": 5,
      "action": "Narrate",
      "value": "VibeCody is running as an always-on service. Now let's deploy to a cloud platform for team access."
    },
    {
      "id": 6,
      "action": "shell",
      "command": "cd deploy/digitalocean && ./setup.sh --tier lite --dry-run",
      "description": "Preview a DigitalOcean deployment",
      "delay_ms": 5000
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/service status", "delay_ms": 2000 }
      ],
      "description": "Check service status from the REPL"
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/gateway enable telegram", "delay_ms": 3000 }
      ],
      "description": "Enable Telegram gateway for mobile access"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Setup wizard shows 0 GB RAM | Ensure VibeCLI has permission to read system info (`sysctl` on macOS, `/proc/meminfo` on Linux) |
| Ollama install fails | Install manually from https://ollama.com/download |
| Service won't start | Check logs: `journalctl --user -u vibecody` (Linux) or `cat ~/.vibecli/vibecli-stderr.log` (macOS) |
| Cloud deploy fails auth | Run `aws configure` / `gcloud auth login` / `az login` first |
| Pi runs out of memory | Use a smaller model or add swap: `sudo fallocate -l 2G /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile` |
| Health check fails | Verify port 7878 is not in use: `lsof -i :7878` |

## What's Next

- [Easy Setup Guide](/vibecody/setup/) — Full platform comparison and one-command setup
- [Use Cases](/vibecody/use-cases/) — 80+ things to do with VibeCody
- [Demo 55: Voice & Pairing](/vibecody/demos/55-voice-pairing-tailscale/) — Voice control and pair programming
- [Demo 14: Cloud Providers](/vibecody/demos/14-cloud-providers/) — Cloud IaC generation
