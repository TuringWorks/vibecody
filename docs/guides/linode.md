---
layout: page
title: "Linode / Akamai"
permalink: /guides/linode/
parent: Deployment Guides
---

Run VibeCody on a Linode instance with Docker Compose.

**Setup time:** 5 minutes | **Cost:** $12–48/month

## Quick Start

```bash
export LINODE_TOKEN="your-token"
cd vibecody/deploy/linode-akamai
./setup.sh --tier lite
```

## Prerequisites

- Linode account and API token
- Terraform installed

## Step-by-Step

### 1. Deploy

```bash
cd deploy/linode-akamai
terraform init
terraform apply -var="linode_token=$LINODE_TOKEN" -var="tier=pro"
```

### 2. Verify

```bash
IP=$(terraform output -raw ip)
curl http://$IP:7878/health
```

## Tiers

| Tier | Plan | Monthly Cost |
|------|------|-------------|
| lite | g6-standard-2 (2 CPU, 4 GB) | ~$12/mo |
| pro | g6-standard-4 (4 CPU, 8 GB) | ~$24/mo |
| max | g6-standard-8 (8 CPU, 16 GB) | ~$48/mo |

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
> `deploy/linode-akamai/main.tf` allows inbound `7878` from `0.0.0.0/0` and `::/0`.
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
