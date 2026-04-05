---
layout: page
title: "Demo 14: Cloud Provider Integration"
permalink: /demos/14-cloud-providers/
nav_order: 14
parent: Demos
---


## Overview

VibeCody detects your cloud infrastructure usage directly from your codebase, generates least-privilege IAM policies, produces Infrastructure-as-Code templates, and estimates costs. This demo covers the full workflow using the `/cloud` REPL commands and the Cloud Provider panel in VibeUI.

**Time to complete:** ~20 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../01-first-run/))
- A project that uses at least one cloud provider (AWS, GCP, or Azure)
- (Optional) Terraform, AWS CLI, `gcloud`, or `az` CLI installed for template validation
- (Optional) VibeUI for the desktop panel experience

## Step-by-Step Walkthrough

### Step 1: Scan your codebase for cloud usage

The `/cloud scan` command analyzes your source files, dependency manifests, and configuration files to detect cloud service usage. VibeCody uses 84 detection patterns across AWS, GCP, and Azure.

```bash
vibecli
> /cloud scan
```

Expected output:

```
Cloud Provider Scan Results
============================
Scanning 347 files...

AWS (14 services detected)
  S3              src/storage.rs:23, src/upload.rs:8
  DynamoDB        src/db/dynamo.rs:1-120
  Lambda          serverless.yml, src/handlers/*.rs
  SQS             src/queue/sqs_client.rs:15
  SNS             src/notifications.rs:42
  CloudWatch      src/metrics.rs:7
  Secrets Manager src/config.rs:33
  IAM             infrastructure/iam.tf
  API Gateway     serverless.yml:18
  ECR             Dockerfile, .github/workflows/deploy.yml
  ECS             infrastructure/ecs.tf
  RDS             src/db/postgres.rs:5 (via connection string)
  CloudFront      infrastructure/cdn.tf
  Route53         infrastructure/dns.tf

GCP (3 services detected)
  Cloud Storage   src/backup.rs:12
  BigQuery        src/analytics/bq.rs:1-80
  Pub/Sub         src/events/pubsub.rs:9

Azure (0 services detected)

Dependency Detection:
  Cargo.toml      aws-sdk-s3, aws-sdk-dynamodb, aws-config
  package.json    @google-cloud/bigquery, @google-cloud/storage

Total: 17 cloud services across 2 providers
```

### Step 2: Generate IAM policies

Generate least-privilege IAM policies based on the detected service usage. VibeCody supports 30+ AWS services, GCP IAM roles, and Azure RBAC.

```bash
> /cloud iam
```

Expected output:

```
Generating least-privilege IAM policies...

AWS IAM Policy (JSON):
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "S3Access",
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:ListBucket",
        "s3:DeleteObject"
      ],
      "Resource": [
        "arn:aws:s3:::my-app-uploads",
        "arn:aws:s3:::my-app-uploads/*"
      ]
    },
    {
      "Sid": "DynamoDBAccess",
      "Effect": "Allow",
      "Action": [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:Query",
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem"
      ],
      "Resource": "arn:aws:dynamodb:*:*:table/my-app-*"
    },
    {
      "Sid": "SQSAccess",
      "Effect": "Allow",
      "Action": [
        "sqs:SendMessage",
        "sqs:ReceiveMessage",
        "sqs:DeleteMessage",
        "sqs:GetQueueAttributes"
      ],
      "Resource": "arn:aws:sqs:*:*:my-app-*"
    }
  ]
}

GCP IAM Roles:
  roles/storage.objectViewer    (Cloud Storage read)
  roles/storage.objectCreator   (Cloud Storage write)
  roles/bigquery.dataViewer     (BigQuery read)
  roles/bigquery.jobUser        (BigQuery query execution)
  roles/pubsub.publisher        (Pub/Sub publish)

Saved to: .vibecli/generated/iam-policy.json
```

You can also target a specific provider or service:

```bash
# IAM for only AWS
> /cloud iam --provider aws

# IAM for a specific service
> /cloud iam --service s3 --bucket my-app-uploads
```

### Step 3: Generate Terraform templates

Produce Infrastructure-as-Code templates for detected services.

```bash
> /cloud terraform
```

Expected output:

```
Generating Terraform templates...

Created files:
  .vibecli/generated/terraform/
    main.tf            Provider configuration, backend
    s3.tf              S3 bucket with versioning, encryption
    dynamodb.tf        DynamoDB table with PAY_PER_REQUEST billing
    sqs.tf             SQS queue with dead-letter queue
    lambda.tf          Lambda functions with IAM roles
    cloudwatch.tf      CloudWatch alarms and dashboards
    outputs.tf         Stack outputs (ARNs, URLs, endpoints)
    variables.tf       Input variables with defaults
    terraform.tfvars   Example variable values

Preview (s3.tf):
  resource "aws_s3_bucket" "uploads" {
    bucket = var.uploads_bucket_name

    tags = {
      Project   = var.project_name
      ManagedBy = "vibecody"
    }
  }

  resource "aws_s3_bucket_versioning" "uploads" {
    bucket = aws_s3_bucket.uploads.id
    versioning_configuration {
      status = "Enabled"
    }
  }

  resource "aws_s3_bucket_server_side_encryption_configuration" "uploads" {
    bucket = aws_s3_bucket.uploads.id
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "aws:kms"
      }
    }
  }

Validate: cd .vibecli/generated/terraform && terraform init && terraform plan
```

### Step 4: Generate CloudFormation templates

```bash
> /cloud cloudformation
```

```
Generating CloudFormation template...

Created: .vibecli/generated/cloudformation/template.yaml

Resources defined: 12
  AWS::S3::Bucket
  AWS::DynamoDB::Table
  AWS::SQS::Queue
  AWS::SQS::Queue (DLQ)
  AWS::Lambda::Function (x3)
  AWS::IAM::Role (x2)
  AWS::CloudWatch::Alarm (x2)
  AWS::SNS::Topic

Validate: aws cloudformation validate-template \
  --template-body file://.vibecli/generated/cloudformation/template.yaml
```

### Step 5: Generate Pulumi templates

```bash
> /cloud pulumi
```

```
Generating Pulumi program (TypeScript)...

Created: .vibecli/generated/pulumi/
  Pulumi.yaml         Project definition
  Pulumi.dev.yaml     Dev stack config
  index.ts            Main infrastructure code
  package.json        Dependencies (@pulumi/aws, @pulumi/gcp)
  tsconfig.json       TypeScript config

Preview (index.ts):
  import * as aws from "@pulumi/aws";

  const uploadsBucket = new aws.s3.Bucket("uploads", {
    versioning: { enabled: true },
    serverSideEncryptionConfiguration: {
      rule: {
        applyServerSideEncryptionByDefault: {
          sseAlgorithm: "aws:kms",
        },
      },
    },
  });

  export const bucketName = uploadsBucket.bucket;

Deploy: cd .vibecli/generated/pulumi && npm install && pulumi up
```

### Step 6: Estimate cloud costs

Get a monthly cost estimate based on detected usage patterns.

```bash
> /cloud cost
```

```
Cloud Cost Estimation
======================
Provider: AWS (us-east-1)

Service              Usage Estimate         Monthly Cost
-------              ---------------        ------------
S3                   50 GB storage,         $1.15
                     100K requests
DynamoDB             10 GB, 50 WCU/RCU      $35.00
Lambda               1M invocations,        $4.20
                     avg 256MB / 200ms
SQS                  500K messages           $0.20
SNS                  100K notifications      $0.05
CloudWatch           10 custom metrics,      $3.00
                     5 alarms
Secrets Manager      5 secrets               $2.00
API Gateway          500K requests           $1.75
ECR                  5 GB images             $0.50
ECS (Fargate)        2 tasks, 0.5vCPU/1GB    $29.40
RDS (db.t3.micro)    PostgreSQL, 20GB        $15.50
CloudFront           100 GB transfer          $8.50

AWS Subtotal:                                $101.25

Provider: GCP (us-central1)

Cloud Storage        10 GB, 50K ops          $0.30
BigQuery             100 GB scanned/mo       $5.00
Pub/Sub              200K messages           $0.08

GCP Subtotal:                                $5.38

Estimated Total:                             $106.63/month

Note: Estimates based on code analysis heuristics. Actual costs
depend on real traffic patterns. Free tier not applied.

Saved to: .vibecli/generated/cost-estimate.json
```

### Step 7: Use the Cloud Provider panel in VibeUI

Open VibeUI and navigate to the **Cloud** panel in the left sidebar.

```bash
cd vibeui && npm run tauri dev
```

The Cloud Provider panel has four tabs:

1. **Scan** -- Visualizes detected cloud services as a dependency graph. Click any node to see source file references. Run scans on demand and compare results across branches.

2. **IAM** -- Interactive IAM policy editor. Select services, choose permission granularity (read/write/admin), and export policies in JSON, YAML, or HCL format.

3. **IaC** -- Infrastructure-as-Code generator with a dropdown for Terraform, CloudFormation, and Pulumi. Preview generated files in a split editor, edit before saving, and run validation commands inline.

4. **Cost** -- Cost dashboard with pie charts per service, monthly projections, and "what-if" sliders to estimate costs at different usage levels.

### Step 8: Dependency-based detection

VibeCody scans dependency manifests to discover cloud SDKs even if they are not directly imported in scanned source files.

Supported manifests:

| File | Detected Packages |
|------|-------------------|
| `Cargo.toml` | `aws-sdk-*`, `google-cloud-*`, `azure_*` |
| `package.json` | `@aws-sdk/*`, `@google-cloud/*`, `@azure/*` |
| `requirements.txt` | `boto3`, `google-cloud-*`, `azure-*` |
| `go.mod` | `github.com/aws/aws-sdk-go-v2`, `cloud.google.com/go` |
| `pom.xml` | `software.amazon.awssdk`, `com.google.cloud` |
| `build.gradle` | `software.amazon.awssdk:*`, `com.google.cloud:*` |

```bash
# Scan only dependency files (faster)
> /cloud scan --deps-only

# Scan a specific directory
> /cloud scan --path ./services/payment
```

## Demo Recording

```json
{
  "meta": {
    "title": "Cloud Provider Integration",
    "description": "Scan your codebase for cloud usage, generate IAM policies, produce IaC templates, and estimate costs.",
    "duration_seconds": 420,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/cloud scan", "delay_ms": 5000 }
      ],
      "description": "Scan the project for AWS, GCP, and Azure service usage across 84 patterns"
    },
    {
      "id": 2,
      "action": "Narrate",
      "value": "VibeCody detected 17 cloud services across 2 providers by analyzing source code, config files, and dependency manifests."
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/cloud iam", "delay_ms": 4000 }
      ],
      "description": "Generate least-privilege IAM policies for all detected services"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/cloud iam --provider aws --service s3 --bucket my-app-uploads", "delay_ms": 3000 }
      ],
      "description": "Generate a scoped IAM policy for a specific S3 bucket"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/cloud terraform", "delay_ms": 5000 }
      ],
      "description": "Generate Terraform templates for all detected AWS and GCP services"
    },
    {
      "id": 6,
      "action": "shell",
      "command": "ls -la .vibecli/generated/terraform/",
      "description": "List the generated Terraform files",
      "delay_ms": 1000
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/cloud cloudformation", "delay_ms": 4000 }
      ],
      "description": "Generate a CloudFormation template as an alternative to Terraform"
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/cloud pulumi", "delay_ms": 4000 }
      ],
      "description": "Generate a Pulumi TypeScript program for infrastructure"
    },
    {
      "id": 9,
      "action": "repl",
      "commands": [
        { "input": "/cloud cost", "delay_ms": 3000 }
      ],
      "description": "Estimate monthly cloud costs based on detected usage patterns"
    },
    {
      "id": 10,
      "action": "Narrate",
      "value": "The estimated monthly cost is $106.63 across AWS and GCP. Now let's see this in VibeUI."
    },
    {
      "id": 11,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to use the Cloud Provider panel",
      "delay_ms": 8000
    },
    {
      "id": 12,
      "action": "Navigate",
      "target": "panel://cloud",
      "description": "Open the Cloud Provider panel"
    },
    {
      "id": 13,
      "action": "Click",
      "target": ".tab-scan",
      "description": "View the Scan tab with the cloud service dependency graph"
    },
    {
      "id": 14,
      "action": "Screenshot",
      "label": "cloud-scan-graph",
      "description": "Capture the cloud service dependency visualization"
    },
    {
      "id": 15,
      "action": "Click",
      "target": ".tab-iam",
      "description": "Switch to the IAM tab"
    },
    {
      "id": 16,
      "action": "Screenshot",
      "label": "cloud-iam-editor",
      "description": "Capture the interactive IAM policy editor"
    },
    {
      "id": 17,
      "action": "Click",
      "target": ".tab-iac",
      "description": "Switch to the IaC tab"
    },
    {
      "id": 18,
      "action": "Click",
      "target": ".iac-format-select option[value='terraform']",
      "description": "Select Terraform as the output format"
    },
    {
      "id": 19,
      "action": "Screenshot",
      "label": "cloud-iac-terraform",
      "description": "Capture the Terraform template preview"
    },
    {
      "id": 20,
      "action": "Click",
      "target": ".tab-cost",
      "description": "Switch to the Cost tab"
    },
    {
      "id": 21,
      "action": "Screenshot",
      "label": "cloud-cost-dashboard",
      "description": "Capture the cost estimation dashboard with pie charts and projections"
    },
    {
      "id": 22,
      "action": "repl",
      "commands": [
        { "input": "/cloud scan --deps-only", "delay_ms": 2000 }
      ],
      "description": "Run a fast scan using only dependency manifests"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Scan finds 0 services | Ensure you are running the scan from the project root with source files present |
| Missing dependency detection | Check that your manifest file (Cargo.toml, package.json, etc.) is in the scanned directory |
| IAM policy too broad | Use `--service` flag to scope policies per service, then combine manually |
| Terraform validation fails | Run `terraform init` first to download provider plugins |
| Cost estimates seem high | Estimates use on-demand pricing without free tier -- adjust with `--region` flag |

## What's Next

- [Demo 15: Deploy & Database](../15-deploy-database/) -- Deployment workflows and database management
- [Demo 13: CI/CD Pipeline](../13-cicd/) -- Monitor and debug GitHub Actions pipelines
- [Demo 12: Kubernetes Operations](../12-kubernetes/) -- Deploy and manage K8s workloads
