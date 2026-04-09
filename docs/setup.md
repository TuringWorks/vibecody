---
layout: page
title: "Easy Setup"
permalink: /setup/
nav_order: 2
---

# Deploy VibeCody Anywhere

**Your own always-on AI coding assistant — zero DevOps required.**

VibeCody runs on anything from a $35 Raspberry Pi to a full cloud instance. Pick your platform, run one command, and you're live in under 5 minutes.

---

## Choose Your Platform

### Cloud Platforms — Always-On, Team-Ready

Best for teams, CI/CD integration, and 24/7 availability. Estimated cost: $5–60/month.

| Platform | Setup Time | Monthly Cost | Free Tier | Guide |
|----------|-----------|-------------|-----------|-------|
| **AWS** | 10 min | $15–60 | 12-month free tier | [Deploy →](/vibecody/guides/aws/) |
| **Google Cloud** | 10 min | $10–50 | $300 credit | [Deploy →](/vibecody/guides/gcp/) |
| **Azure** | 10 min | $15–55 | $200 credit | [Deploy →](/vibecody/guides/azure/) |
| **Oracle Cloud** | 10 min | **$0** | Always-free ARM | [Deploy →](/vibecody/guides/oracle-cloud/) |
| **DigitalOcean** | 5 min | $12–48 | $200 credit | [Deploy →](/vibecody/guides/digitalocean/) |
| **Linode / Akamai** | 5 min | $12–48 | — | [Deploy →](/vibecody/guides/linode/) |

> **Best value:** Oracle Cloud's always-free tier gives you 4 ARM cores + 24 GB RAM — enough for VibeCody + a 7B local model at **$0/month**.

### Desktop — Personal Workstation

Best for individual developers who want VibeCody integrated into their daily workflow.

| Platform | Setup Time | Always-On | GPU Acceleration | Guide |
|----------|-----------|-----------|-----------------|-------|
| **macOS** (MacBook / Mac Mini) | 2 min | ✅ launchd | ✅ Metal | [Install →](/vibecody/guides/macos/) |
| **Linux** (Ubuntu, Fedora, Arch) | 2 min | ✅ systemd | ✅ CUDA/ROCm | [Install →](/vibecody/guides/linux/) |
| **Windows** | 3 min | ✅ Service | ✅ CUDA | [Install →](/vibecody/guides/windows/) |

### Edge / Homelab — IoT & Self-Hosted

Best for privacy-first setups, smart home hubs, and always-on personal assistants.

| Platform | Setup Time | Local Models | Remote Access | Guide |
|----------|-----------|-------------|--------------|-------|
| **Raspberry Pi 5** (8 GB) | 5 min | Up to 7B | Tailscale / Cloudflare | [Install →](/vibecody/guides/raspberry-pi/) |
| **Raspberry Pi 4** (4–8 GB) | 5 min | Up to 3B–7B | Tailscale / Cloudflare | [Install →](/vibecody/guides/raspberry-pi/) |
| **Raspberry Pi 3** (1 GB) | 10 min | TinyLlama (cloud recommended) | Tailscale / Cloudflare | [Install →](/vibecody/guides/raspberry-pi/) |

> **Homelab tip:** A Mac Mini with Apple Silicon is the best desktop "server" — 18 GPU cores run 13B models at full speed with zero fan noise.

---

## One-Command Setup

Every platform has a one-liner. Here's the fastest path for each:

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
vibecli
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/windows/setup.ps1 | iex
```

### Docker (Any Platform)

```bash
docker run -p 7878:7878 ghcr.io/turingworks/vibecody:latest
```

### Cloud (AWS Example)

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody/deploy/aws
./setup.sh
```

### Raspberry Pi

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/raspberry-pi/setup.sh | sh
```

---

## Always-On Mode

VibeCody can run as a persistent background service, just like myclaw.ai — but self-hosted and free.

| Platform | Service Manager | Command |
|----------|----------------|---------|
| macOS | launchd | `vibecli --serve --port 7878 --provider ollama` |
| Linux | systemd | `vibecli --serve --port 7878 --provider ollama` |
| Windows | Windows Service | `vibecli --serve --port 7878 --provider ollama` |
| Docker | docker-compose | `docker compose up -d` |
| Cloud | Managed (auto) | Always running after deploy |

Once running, access VibeCody from anywhere:

```bash
# Local access
curl http://localhost:7878/health

# Remote access (via Tailscale)
curl http://vibecody.tailnet:7878/health

# Gateway integrations (Slack, Discord, Telegram, etc.)
vibecli gateway enable slack
vibecli gateway enable discord
vibecli gateway enable telegram
```

---

---

## Tiers

Like myclaw.ai's Lite/Pro/Max plans, VibeCody offers tier presets for resource allocation — but you control the hardware and pay only infrastructure costs.

| Tier | vCPU | RAM | Storage | Best For | Cloud Cost |
|------|------|-----|---------|----------|------------|
| **Lite** | 2 | 4 GB | 40 GB | Chat + small agent tasks | $5–16/mo |
| **Pro** | 4 | 8 GB | 80 GB | Agent loops + local 7B models | $20–33/mo |
| **Max** | 8 | 16 GB | 160 GB | Multi-agent + local 13B models | $40–66/mo |

Resource allocation is configured through your infrastructure provider's settings (instance size, container resource limits, etc.) rather than through VibeCody CLI flags.

---

## How It Compares

| Feature | **VibeCody** | myclaw.ai (OpenClaw) |
|---------|-------------|---------------------|
| **Cost** | Free (self-hosted) + infra costs | $16–66/month |
| **AI Providers** | 23 (local + cloud) | Limited to what OpenClaw supports |
| **Code Intelligence** | 7 review detectors, 550+ skills, LSP | General-purpose |
| **Deployment Targets** | 12 platforms | Managed cloud only |
| **Raspberry Pi** | ✅ Full support | ❌ |
| **Desktop App** | ✅ VibeUI (196+ panels) | ❌ |
| **Open Source** | ✅ MIT License | Wraps open-source, hosted service |
| **Always-On** | ✅ All platforms | ✅ Cloud only |
| **MCP Integration** | ✅ Full | ✅ Via OpenClaw |
| **Agent Autonomy** | Agent loop + multi-agent teams | Single agent |
| **Gateway** | 18 messaging platforms | ~10 integrations |
| **Privacy** | Your hardware, your data | Managed servers |
| **Customization** | Full config + plugins + rules | Limited |
| **CI/CD Integration** | GitHub Actions, K8s, Docker | Limited |
| **GPU Acceleration** | Metal, CUDA, ROCm | Depends on plan |

---

## What's Next

- **[Use Cases](/vibecody/use-cases/)** — See 80+ things you can do with VibeCody
- **[Quickstart](/vibecody/quickstart/)** — Your first chat in 60 seconds
- **[Configuration](/vibecody/configuration/)** — Deep-dive on all settings
- **[Tutorials](/vibecody/tutorials/)** — Step-by-step walkthroughs
