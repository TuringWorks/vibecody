---
layout: page
title: "AWS"
permalink: /guides/aws/
parent: Deployment Guides
---

# Deploy VibeCody on AWS

Run VibeCody as an always-on service on AWS ECS Fargate with an Application Load Balancer. Includes Ollama sidecar for local model inference.

**Setup time:** 10 minutes | **Cost:** $15–60/month | **Tier:** Lite / Pro / Max

## Prerequisites

- AWS account with billing enabled
- AWS CLI installed and configured (`aws configure`)
- (Optional) A domain name and ACM certificate for HTTPS

## Quick Start

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody/deploy/aws
./setup.sh --tier lite
```

The script will deploy a CloudFormation stack and print your VibeCody URL.

## Step-by-Step

### 1. Authenticate

```bash
aws configure
# Enter your Access Key ID, Secret Key, region (e.g., us-east-1)
aws sts get-caller-identity  # Verify
```

### 2. Choose a Tier

| Tier | vCPU | RAM | Fargate Cost (us-east-1) |
|------|------|-----|--------------------------|
| lite | 2 | 4 GB | ~$15/mo |
| pro | 4 | 8 GB | ~$35/mo |
| max | 8 | 16 GB | ~$60/mo |

### 3. Deploy

```bash
./setup.sh --tier pro --region us-west-2
```

Or deploy manually with CloudFormation:

```bash
aws cloudformation deploy \
  --stack-name vibecody \
  --template-file cloudformation.yaml \
  --parameter-overrides Tier=pro \
  --capabilities CAPABILITY_IAM
```

### 4. Verify

```bash
# Get the URL
URL=$(aws cloudformation describe-stacks --stack-name vibecody \
  --query "Stacks[0].Outputs[?OutputKey=='ServiceURL'].OutputValue" --output text)

curl $URL/health
```

### 5. Enable HTTPS (Optional)

1. Request an ACM certificate: `aws acm request-certificate --domain-name vibecody.example.com`
2. Validate the certificate (DNS or email)
3. Redeploy with the certificate ARN:

```bash
aws cloudformation deploy --stack-name vibecody --template-file cloudformation.yaml \
  --parameter-overrides Tier=pro CertArn=arn:aws:acm:us-east-1:123:certificate/abc-123
```

## Remote Access

Your ALB URL is publicly accessible. For private access, restrict the security group to your IP range in the CloudFormation template.

## Upgrading

```bash
# Update to latest image
aws ecs update-service --cluster vibecody-cluster --service vibecody --force-new-deployment
```

## Teardown

```bash
./teardown.sh --stack vibecody --region us-east-1
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Stack creation fails | Check CloudTrail for IAM permission errors |
| Health check failing | Wait 2–3 minutes for Ollama to initialize |
| 503 errors | ECS task may be starting — check ECS console for task status |
| High costs | Use Lite tier; consider Oracle Cloud free tier instead |
| Need GPU | Use EC2 with GPU instead of Fargate (modify template) |

## What's Next

- [Use Cases](/vibecody/use-cases/) — What to do with your always-on VibeCody
- [Configuration](/vibecody/configuration/) — Connect cloud AI providers
- [Demo 57: Easy Setup](/vibecody/demos/57-easy-setup/) — Full walkthrough
