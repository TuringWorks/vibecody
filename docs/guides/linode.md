---
layout: page
title: "Linode / Akamai"
permalink: /guides/linode/
parent: Deployment Guides
---

# Deploy VibeCody on Linode (Akamai)

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

## Teardown

```bash
./teardown.sh
```

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
