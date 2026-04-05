---
layout: page
title: "Google Cloud"
permalink: /guides/gcp/
parent: Deployment Guides
---

# Deploy VibeCody on Google Cloud

Run VibeCody on Cloud Run with Ollama sidecar.

**Setup time:** 10 minutes | **Cost:** $10–50/month | **Free credit:** $300 for new accounts

## Quick Start

```bash
cd vibecody/deploy/gcp
./setup.sh --project YOUR_PROJECT_ID --tier lite
```

## Prerequisites

- Google Cloud account with billing
- `gcloud` CLI installed (`gcloud auth login`)
- Terraform installed

## Step-by-Step

### 1. Enable APIs

```bash
gcloud services enable run.googleapis.com containerregistry.googleapis.com
```

### 2. Deploy

```bash
cd deploy/gcp
terraform init
terraform apply -var="project_id=my-project" -var="tier=pro"
```

### 3. Verify

```bash
URL=$(terraform output -raw url)
curl $URL/health
```

## Tiers

| Tier | vCPU | RAM | Estimated Cost |
|------|------|-----|---------------|
| lite | 2 | 4 GB | ~$10/mo |
| pro | 4 | 8 GB | ~$30/mo |
| max | 8 | 16 GB | ~$50/mo |

## Teardown

```bash
./teardown.sh
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Permission denied | Enable required APIs and check IAM roles |
| Cold start slow | Cloud Run scales to zero — first request takes ~30s |
| Ollama OOM | Increase tier to pro or max |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
