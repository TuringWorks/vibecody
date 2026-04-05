---
layout: page
title: "Raspberry Pi"
permalink: /guides/raspberry-pi/
parent: Deployment Guides
---

# Deploy VibeCody on Raspberry Pi

Run VibeCody on a Raspberry Pi 3, 4, or 5 as a privacy-first, always-on AI coding assistant.

**Setup time:** 5–10 minutes | **Cost:** $35 hardware | **Models:** TinyLlama to Mistral 7B

## Quick Start

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/raspberry-pi/setup.sh | sh
```

Or with remote access:

```bash
cd vibecody/deploy/raspberry-pi
./setup.sh --tailscale
```

## Model Recommendations

| Pi Model | RAM | Recommended Model | Performance |
|----------|-----|------------------|-------------|
| Pi 3 | 1 GB | tinyllama:1.1b (or cloud provider) | ~2 tok/s |
| Pi 4 (4 GB) | 4 GB | phi:2.7b | ~5 tok/s |
| Pi 4 (8 GB) | 8 GB | mistral:7b | ~3 tok/s |
| Pi 5 (8 GB) | 8 GB | mistral:7b or codellama:7b | ~5 tok/s |

> **Tip:** For Pi 3, use a cloud AI provider (Claude, OpenAI, Gemini) instead of local models. The Pi runs VibeCLI itself; the AI runs in the cloud.

## Step-by-Step

### 1. Run Setup Script

```bash
cd vibecody/deploy/raspberry-pi
./setup.sh
```

The script will:
- Detect your Pi model and RAM
- Install the aarch64 VibeCLI binary
- Install Ollama (ARM build)
- Create swap space if RAM < 4 GB
- Install a systemd service
- Copy an optimized config for your Pi model

### 2. Access VibeCody

```bash
# Local
curl http://localhost:7878/health

# From the REPL
vibecli
```

### 3. Remote Access (Optional)

**Tailscale (recommended):**
```bash
./setup.sh --tailscale
# Then from any device on your Tailscale network:
curl http://raspberrypi.your-tailnet.ts.net:7878/health
```

**Cloudflare Tunnel:**
```bash
./setup.sh --cloudflare
cloudflared tunnel login
cloudflared tunnel create vibecody
cloudflared tunnel route dns vibecody vibecody.example.com
cloudflared tunnel run vibecody
```

## Optimized Configs

The setup script installs a config optimized for your Pi:

- **Pi 3** (`config-pi3.toml`): TinyLlama 1.1B, 1024 context, 10 max agent steps
- **Pi 4** (`config-pi4.toml`): Phi 2.7B, 2048 context, 25 max agent steps
- **Pi 5** (`config-pi5.toml`): Mistral 7B, 4096 context, 50 max agent steps

## Service Management

```bash
sudo systemctl status vibecody    # Check status
sudo systemctl restart vibecody   # Restart
sudo journalctl -u vibecody -f    # View logs
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Killed" during model load | Not enough RAM — use a smaller model or add swap |
| Ollama slow to start | First model load takes time; subsequent runs are faster |
| SD card fills up | Models use 1–4 GB; use a 32 GB+ SD card or USB storage |
| Can't reach from network | Check firewall: `sudo ufw allow 7878` |
| Pi 3 too slow | Use cloud AI providers — set `--provider claude` in config |

## What's Next

- [Use Cases](/vibecody/use-cases/) — Especially the IoT & Edge category
- [Configuration](/vibecody/configuration/) — Connect to cloud AI providers
- [Demo 55: Voice & Pairing](/vibecody/demos/55-voice-pairing-tailscale/) — Voice control your Pi
