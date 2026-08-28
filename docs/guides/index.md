---
layout: page
title: "Deployment Guides"
permalink: /guides/
nav_order: 4
has_children: true
---

These guides are about running the **VibeCLI daemon** somewhere — a cloud VM, a
spare Linux box, a Raspberry Pi on your desk. The daemon is the single process
every VibeCody client talks to: it holds the providers, the agent loop, the
skills, and the workspace state, and the fourteen clients are windows onto it.

## You may not need any of this

**If you are one person on one machine, install a desktop app and stop reading.**
VibeCoder, VibeDesk and VibeAIChat each start the daemon for you on launch, on
`127.0.0.1:7878`, and reuse one that is already running. There is nothing to
deploy and nothing to configure. See [Quickstart]({{ site.baseurl }}/quickstart/).

Deploy a daemon when you want one of these instead:

| You want | Read |
|---|---|
| It running when your laptop is closed — for the phone, the watch, or a schedule | [Cloud](#cloud-platforms) or [Edge / homelab](#edge--homelab) |
| One daemon your team shares, with its own credentials | [Cloud](#cloud-platforms) |
| Everything to stay on hardware you own | [Edge / homelab](#edge--homelab) or [Desktop as a service](#desktop-as-an-always-on-service) |
| A GPU the daemon can use for local models | [Desktop](#desktop-as-an-always-on-service) (CUDA / ROCm / Metal) |

Whatever you pick, the daemon is the same build with the same surface: 25
selectable AI providers, 1,144 skill files across 155 categories, the autonomous
agent loop, and the HTTP + WebSocket API in
[api-reference]({{ site.baseurl }}/api-reference/).

## Before you start

Two things decide whether a fresh daemon is useful, and neither is
platform-specific:

1. **A model to talk to.** Either a cloud provider key — stored encrypted in the
   ProfileStore, never in a config file — or [Ollama]({{ site.baseurl }}/providers/ollama/)
   alongside the daemon for a fully local setup. A daemon with neither starts
   fine and can answer nothing.
2. **A way for your clients to reach it.** On a LAN that is mDNS and nothing
   else. Off it, the mobile and watch clients race Tailscale, ngrok and a phone
   relay — see [Connectivity]({{ site.baseurl }}/connectivity/). Do not expose
   port 7878 to the internet directly; nearly every route is behind a bearer
   token that **rotates on every daemon start**, which is a poor fit for a
   public address.

## Cloud platforms

Always-on and team-reachable. Costs below are **the provider's list prices as we
last read them (August 2026)** for a small always-on instance — they are not
measured by us and not a quote. Check the vendor's calculator before committing;
egress and storage are what usually move the number.

| Platform | Service | Free tier | Est. monthly | Notes |
|----------|---------|-----------|-------------:|-------|
| [Oracle Cloud](./oracle-cloud/) | Container Instances | **Always-free ARM** | **$0** | 4 ARM cores + 24 GB |
| [DigitalOcean](./digitalocean/) | Droplet + Docker | $200 credit | $24–96 | Simplest VM path |
| [Linode / Akamai](./linode/) | Linode + Docker | — | $12–48 | Simplest VM path |
| [Google Cloud](./gcp/) | Cloud Run | $300 credit | $10–50 | Scales to zero |
| [AWS](./aws/) | ECS Fargate + ALB | 12-month | $15–60 | Most IAM control |
| [Azure](./azure/) | Container Apps | $200 credit | $15–55 | Entra ID integration |

> **Start with Oracle Cloud if cost is the deciding factor.** Its always-free
> tier is 4 ARM cores and 24 GB RAM, permanently — enough to run the daemon and
> a 7B model locally beside it, at nothing per month. The catch is capacity:
> free ARM instances are frequently unavailable in busy regions, and you may
> have to retry or pick another region.

> **Cloud Run scales to zero**, which is cheap and wrong for this workload if
> you want the phone to reach it — a cold daemon has been measured at **~16 s**
> to first answer `/health`. Pin a minimum instance if you care about that.

## Desktop, as an always-on service

For a workstation that is already yours and already has the GPU. This installs
the daemon as a system service so it survives logout and reboot; it is not
required just to use the desktop apps.

| Platform | Service manager | Local-model acceleration | Notes |
|----------|-----------------|--------------------------|-------|
| [macOS](./macos/) | launchd | Metal (Apple Silicon) | MacBook & Mac Mini |
| [Linux](./linux/) | systemd | CUDA / ROCm | Ubuntu, Fedora, Arch |
| [Windows](./windows/) | Scheduled Task | CUDA | PowerShell installer |

## Edge / homelab

Private by construction: nothing leaves the house unless you point it at a cloud
provider. The constraint is RAM, and it decides which local model you can run.

| Board | RAM | Local model that fits | Verdict |
|-------|-----|----------------------|-------|
| [Raspberry Pi 5](./raspberry-pi/) | 8 GB | Mistral 7B (quantised) | Recommended |
| [Raspberry Pi 4](./raspberry-pi/) | 4–8 GB | Phi 2.7B – 7B | Workable |
| [Raspberry Pi 3](./raspberry-pi/) | 1 GB | TinyLlama 1.1B | Tight |

A Pi can also run the daemon against a **cloud** provider, in which case RAM
stops mattering — it is then a small always-on relay, and even a Pi 3 is fine.

## Choosing

| | Cloud | Desktop service | Raspberry Pi |
|---|---|---|---|
| **Cost** | $0–60/mo | Free (hardware you own) | Free (~$35–80 board) |
| **Always-on** | Yes, by default | Only while the machine is | Yes |
| **Reached from outside** | Public URL or Tailscale | Tailscale / tunnel | Tailscale / tunnel |
| **Local models** | Ollama sidecar; no GPU on the cheap tiers | Full GPU | RAM-bound, CPU only |
| **Who sees your code** | Your cloud provider | You | You |
| **Upkeep** | Managed runtime, you patch the image | You patch the host | You patch the host |

## Setting one up

Every guide starts from the same wizard, which detects the platform, picks a
tier, configures a provider and offers to install the service:

```bash
vibecli --setup
```

The per-platform pages cover what the wizard cannot: the provider's own
networking, its firewall rules, and the deploy scripts under `deploy/<platform>/`.

## Then point something at it

A deployed daemon is not useful until a client reaches it. The bearer token is
written to `~/.vibecli/daemon.token` on the host and **changes every time the
daemon restarts**, so copy it after the final start, not before.

- **Desktop shells** — set the daemon URL in Settings; they otherwise assume
  `127.0.0.1:7878` and will start a second local daemon.
- **Mobile and watch** — [pair]({{ site.baseurl }}/pairing/) with a URL, or a URL
  plus the token. QR is a convenience, never the only path.
- **VS Code, JetBrains, Neovim, the Agent SDK** — daemon URL plus token in each
  client's settings.
- **Anything else** — [api-reference]({{ site.baseurl }}/api-reference/).
