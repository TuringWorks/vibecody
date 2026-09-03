---
layout: page
title: "Provider: AWS Bedrock"
permalink: /providers/bedrock/
---

[Amazon Bedrock](https://aws.amazon.com/bedrock/) provides managed access to foundation models from Anthropic, Meta, Mistral, and Amazon through the AWS ecosystem, with IAM-based authentication and SigV4 request signing.

## Prerequisites

1. An AWS account with Bedrock access enabled
2. Model access granted in the Bedrock console (must request access per model)
3. IAM credentials with `bedrock:InvokeModel` and `bedrock:InvokeModelWithResponseStream` permissions

## Enable Model Access

1. Go to the [Bedrock console](https://console.aws.amazon.com/bedrock/)
2. Navigate to **Model access** in the left sidebar
3. Request access to the models you want (e.g., Claude, Llama)
4. Wait for approval (usually instant for most models)

## Configure VibeCody

**Option 1: Environment variables** (recommended)

```bash
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
vibecli --provider bedrock
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[bedrock]
enabled = true
api_key = "AKIA..."          # AWS access key ID
model = "anthropic.claude-sonnet-5"
region = "us-east-1"
```

The `api_key` field holds the AWS access key ID. The secret access key is read from `AWS_SECRET_ACCESS_KEY`.

## Model Selection

| Model | Provider | Best for |
|-------|----------|----------|
| `anthropic.claude-opus-5` | Anthropic | Deepest reasoning |
| `anthropic.claude-sonnet-5` | Anthropic | Strong coding, default |
| `anthropic.claude-opus-4-8` | Anthropic | Previous-gen Opus |
| `anthropic.claude-haiku-4-5` | Anthropic | Fast, affordable |
| `meta.llama3-1-70b-instruct-v1:0` | Meta | Open model |
| `mistral.mistral-large-2402-v1:0` | Mistral | European alternative |
| `amazon.titan-text-premier-v1:0` | Amazon | AWS-native model |

**Default:** `anthropic.claude-sonnet-5`

The Claude 3 ids this page used to list are retired (3.5 Sonnet on 2025-10-28,
3 Haiku on 2026-04-20). Bedrock is partner-operated and sets its own retirement
schedule, so check the model is listed in your region before pinning it.

Override from the CLI:

```bash
vibecli --provider bedrock --model anthropic.claude-haiku-4-5
```

## Authentication

Bedrock uses AWS SigV4 request signing. VibeCody reads credentials from:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Config file (`api_key` field for access key ID)

The `AWS_REGION` environment variable or `region` config field determines the Bedrock endpoint.

## Best For

- **AWS-native infrastructure** -- integrates with IAM, CloudWatch, VPC, and CloudTrail
- **Compliance** -- SOC 2, HIPAA, FedRAMP through AWS compliance programs
- **Multi-model access** -- use Claude, Llama, Mistral through one service
- **No API key management** -- uses existing AWS IAM credentials

## Verify Connection

```bash
vibecli --provider bedrock -c "Write a TypeScript function to parse CSV files"
```

## Troubleshooting

### AccessDeniedException

- Verify model access is granted in the Bedrock console
- Check IAM permissions include `bedrock:InvokeModel`
- Confirm the model ID is correct and available in your region

### Region not available

- Not all models are available in all AWS regions
- Check [Bedrock model availability](https://docs.aws.amazon.com/bedrock/latest/userguide/models-regions.html)
- Try `us-east-1` or `us-west-2` for the broadest model selection

### Credential errors

- Verify `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are set
- Check credentials are not expired (if using temporary credentials, also set `AWS_SESSION_TOKEN`)
