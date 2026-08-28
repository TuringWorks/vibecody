---
layout: page
title: "DigitalOcean"
permalink: /guides/digitalocean/
parent: Deployment Guides
---

Run VibeCody on a DigitalOcean Droplet with Docker Compose.

**Setup time:** 5 minutes | **Cost:** $24–96/month (provider list price, read August 2026) | **Free credit:** $200 for new accounts

## Quick Start

```bash
export DIGITALOCEAN_TOKEN="your-token"
cd vibecody/deploy/digitalocean
./setup.sh --tier lite
```

## Prerequisites

- DigitalOcean account and API token
- Terraform installed

## Step-by-Step

### 1. Get API Token

Create a token at: https://cloud.digitalocean.com/account/api/tokens

```bash
export DIGITALOCEAN_TOKEN="dop_v1_..."
```

### 2. Deploy

```bash
cd deploy/digitalocean
terraform init
terraform apply -var="do_token=$DIGITALOCEAN_TOKEN" -var="tier=pro"
```

### 3. Verify

```bash
IP=$(terraform output -raw ip)
curl http://$IP:7878/health
```

## Tiers

| Tier | Droplet | Monthly Cost |
|------|---------|-------------|
| lite | s-2vcpu-4gb | $24/mo |
| pro | s-4vcpu-8gb | $48/mo |
| max | s-8vcpu-16gb | $96/mo |

## Troubleshooting

| Problem | What is actually happening | Fix |
|---|---|---|
| `curl http://$IP:7878/health` hangs or refuses right after `terraform apply` | The daemon is still starting. A cold daemon has been **measured at ~16 s** to first answer `/health`, and cloud-init has to pull the image before that | Poll to a deadline rather than checking once: `until curl -sf http://$IP:7878/health; do sleep 3; done` |
| `/health` answers but every other route returns `401` | Working as designed. Almost every route needs the bearer token, and the daemon mints a **fresh one on every restart** | `ssh` in and read `~/.vibecli/daemon.token`. Re-read it after any restart — a cached token dies with the process |
| Something answers on 7878 but is not VibeCLI | Another process took the port | `GET /health` returns `service: "vibecli"`. If it does not, the responder is not the daemon |
| The assistant replies "no provider configured" | The droplet has no key and no local model | `vibecli set-key <provider> <key>` on the host, or install Ollama beside it. Keys go to the encrypted ProfileStore, never a config file |
| Terraform: `Error: POST .../droplets: 401` | The API token is missing, expired, or lacks write scope | Regenerate at the provider console and re-export it. `--tier` is unrelated |
| Terraform succeeds, `terraform output -raw ip` is empty | The apply targeted a different state or workspace | Run it from the same `deploy/<platform>/` directory you applied in |
| Local models are unusably slow | These are CPU instances — no GPU on any tier | Point at a cloud provider, or size for it: see [Sizing]({{ site.baseurl }}/sizing/) |
| Out of memory building or running a model | The tier's RAM is the binding constraint | `lite` is 4 GB, which is a cloud-provider relay, not a local-inference host |

> ### The firewall opens 7878 to the whole internet
>
> `deploy/digitalocean/main.tf` allows inbound `7878` from `0.0.0.0/0` and `::/0`.
> The daemon is not unauthenticated — it mints a random 128-bit token at
> startup and nearly every route requires it — but the port is world-reachable
> and the token sits on the box.
>
> For anything but a throwaway test, restrict the source range to your own
> address, or drop the rule entirely and reach the daemon over
> [Tailscale]({{ site.baseurl }}/connectivity/). A token that rotates on every
> restart is a poor fit for a public address.

## Teardown

```bash
./teardown.sh
```

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
