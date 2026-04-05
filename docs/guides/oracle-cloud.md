---
layout: page
title: "Oracle Cloud"
permalink: /guides/oracle-cloud/
parent: Deployment Guides
---

# Deploy VibeCody on Oracle Cloud (FREE)

Oracle Cloud's always-free tier provides **4 ARM OCPU + 24 GB RAM** — enough to run VibeCody with Mistral 7B at **$0/month forever**.

**Setup time:** 10 minutes | **Cost:** $0 (always-free tier) | **Best value of any cloud platform**

## Why Oracle Cloud?

- **4 Arm-based Ampere A1 cores** (always free)
- **24 GB RAM** (always free) — enough for 7B parameter models
- **200 GB block storage** (always free)
- No credit card charges after free trial (free tier is permanent)

## Quick Start

```bash
cd vibecody/deploy/oracle-cloud
./setup.sh --tier lite
```

## Prerequisites

- Oracle Cloud account (sign up at cloud.oracle.com)
- OCI CLI installed (`oci setup config`)
- Terraform installed

## Step-by-Step

### 1. Get Your IDs

```bash
# Compartment ID
oci iam compartment list --query "data[0].id" --raw-output

# Availability domain
oci iam availability-domain list --query "data[0].name" --raw-output

# Subnet ID (create a VCN first if needed)
oci network subnet list --compartment-id YOUR_COMPARTMENT_ID --query "data[0].id" --raw-output
```

### 2. Deploy

```bash
cd deploy/oracle-cloud
terraform init
terraform apply \
  -var="compartment_id=ocid1.compartment..." \
  -var="availability_domain=AD-1" \
  -var="subnet_id=ocid1.subnet..." \
  -var="tier=max"  # max tier fits in free tier!
```

### 3. Verify

```bash
IP=$(terraform output -raw public_ip)
curl http://$IP:7878/health
```

## Tiers (All Fit in Free Tier!)

| Tier | OCPU | RAM | Free Tier? |
|------|------|-----|-----------|
| lite | 2 | 4 GB | Yes |
| pro | 4 | 8 GB | Yes |
| max | 4 | 24 GB | **Yes — uses full free allocation** |

## Always-On

Oracle Cloud Container Instances run 24/7 by default. No additional configuration needed.

## Remote Access

The container instance gets a public IP. For HTTPS, add a load balancer or use Cloudflare Tunnel.

## Teardown

```bash
./teardown.sh
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Out of capacity" | Try a different availability domain or region |
| Container won't start | ARM image required — VibeCody aarch64 build is used automatically |
| Free tier limits | 4 OCPU + 24 GB total across all free instances |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
