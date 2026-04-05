---
layout: page
title: "Deployment Guides"
permalink: /guides/
nav_order: 4
has_children: true
---

# Deploy VibeCody Anywhere

Pick your platform and follow the guide. Every deployment gives you the same VibeCody experience — 23 AI providers, 106+ REPL commands, 556+ skills, autonomous agent loop, and always-on server mode. All deployments include the productivity integrations (Gmail, Calendar, Todoist, Notion, Jira, Home Assistant).

## Cloud Platforms — Always-On, Team-Ready

Best for teams, CI/CD integration, and 24/7 availability.

| Platform | Setup Time | Monthly Cost | Free Tier | GPU | Guide |
|----------|-----------|-------------|-----------|-----|-------|
| [AWS](./aws/) | 10 min | $15–60 | 12-month | — | ECS Fargate + ALB |
| [Google Cloud](./gcp/) | 10 min | $10–50 | $300 credit | — | Cloud Run |
| [Azure](./azure/) | 10 min | $15–55 | $200 credit | — | Container Apps |
| [Oracle Cloud](./oracle-cloud/) | 10 min | **$0** | **Always-free ARM** | — | Container Instances |
| [DigitalOcean](./digitalocean/) | 5 min | $12–48 | $200 credit | — | Droplet + Docker |
| [Linode / Akamai](./linode/) | 5 min | $12–48 | — | — | Linode + Docker |

> **Best value:** Oracle Cloud's always-free tier gives you 4 ARM cores + 24 GB RAM — enough to run VibeCody + Mistral 7B at **$0/month**.

## Desktop — Personal Workstation

Best for individual developers.

| Platform | Setup Time | Always-On | GPU Acceleration | Guide |
|----------|-----------|-----------|-----------------|-------|
| [macOS](./macos/) | 2 min | launchd | Metal (Apple Silicon) | MacBook & Mac Mini |
| [Linux](./linux/) | 2 min | systemd | CUDA / ROCm | Ubuntu, Fedora, Arch |
| [Windows](./windows/) | 3 min | Scheduled Task | CUDA | PowerShell installer |

## Edge / Homelab — IoT & Self-Hosted

Best for privacy-first setups and always-on personal assistants.

| Platform | Setup Time | Max Local Model | Remote Access | Guide |
|----------|-----------|----------------|--------------|-------|
| [Raspberry Pi 5](./raspberry-pi/) | 5 min | Mistral 7B | Tailscale / Cloudflare | 8 GB RAM |
| [Raspberry Pi 4](./raspberry-pi/) | 5 min | Phi 2.7B–7B | Tailscale / Cloudflare | 4–8 GB RAM |
| [Raspberry Pi 3](./raspberry-pi/) | 10 min | TinyLlama 1.1B | Tailscale / Cloudflare | 1 GB RAM |

## Decision Matrix

| Factor | Cloud | Desktop | Raspberry Pi |
|--------|-------|---------|-------------|
| **Cost** | $0–60/mo | Free | Free ($35 hardware) |
| **Setup** | 5–10 min | 2–3 min | 5–10 min |
| **Always-on** | Automatic | Optional service | systemd service |
| **Team access** | Built-in | Via Tailscale | Via tunnel |
| **Local models** | Via Ollama sidecar | Full GPU access | Limited by RAM |
| **Maintenance** | Auto-managed | Manual updates | Manual updates |
| **Privacy** | Cloud provider | 100% local | 100% local |

## Quick Start

Not sure? Run the interactive setup wizard:

```bash
vibecli --setup
```

It detects your platform, recommends a tier, configures your AI provider, and optionally installs the always-on service.
