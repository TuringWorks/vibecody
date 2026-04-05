---
layout: page
title: "DigitalOcean"
permalink: /guides/digitalocean/
parent: Deployment Guides
---

# Deploy VibeCody on DigitalOcean

Run VibeCody on a DigitalOcean Droplet with Docker Compose.

**Setup time:** 5 minutes | **Cost:** $12–48/month | **Free credit:** $200 for new accounts

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

## Teardown

```bash
./teardown.sh
```

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
