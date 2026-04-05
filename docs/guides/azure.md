---
layout: page
title: "Azure"
permalink: /guides/azure/
parent: Deployment Guides
---

# Deploy VibeCody on Azure

Run VibeCody on Azure Container Apps with Ollama sidecar.

**Setup time:** 10 minutes | **Cost:** $15–55/month | **Free credit:** $200 for new accounts

## Quick Start

```bash
cd vibecody/deploy/azure
./setup.sh --tier lite
```

## Prerequisites

- Azure account with subscription
- Azure CLI installed (`az login`)

## Step-by-Step

### 1. Create Resource Group

```bash
az group create --name vibecody-rg --location eastus
```

### 2. Deploy

```bash
az deployment group create \
  --resource-group vibecody-rg \
  --template-file deploy/azure/main.bicep \
  --parameters tier=pro
```

### 3. Get URL

```bash
az containerapp show --name vibecody --resource-group vibecody-rg \
  --query "properties.configuration.ingress.fqdn" -o tsv
```

## Tiers

| Tier | vCPU | RAM | Estimated Cost |
|------|------|-----|---------------|
| lite | 2 | 4 GB | ~$15/mo |
| pro | 4 | 8 GB | ~$35/mo |
| max | 8 | 16 GB | ~$55/mo |

## Teardown

```bash
./teardown.sh --rg vibecody-rg
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Deployment fails | Check `az deployment group show` for error details |
| Container restarting | Increase tier — Ollama needs RAM for model loading |
| Slow response | Container Apps may scale to zero; set minReplicas=1 |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
