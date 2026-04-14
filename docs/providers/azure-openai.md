---
layout: page
title: "Provider: Azure OpenAI"
permalink: /providers/azure-openai/
---

# Azure OpenAI Provider

[Azure OpenAI Service](https://azure.microsoft.com/en-us/products/ai-services/openai-service) provides OpenAI models through Microsoft Azure, with enterprise security, compliance, and regional data residency.

## Prerequisites

1. An Azure subscription
2. Access granted to Azure OpenAI Service (request via [Azure portal](https://portal.azure.com))
3. A deployed model in your Azure OpenAI resource

## Set Up a Deployment

1. Go to [Azure AI Studio](https://ai.azure.com) or the Azure portal
2. Create an **Azure OpenAI** resource
3. Navigate to **Deployments** and deploy a model (e.g., `gpt-4o`)
4. Note your:
   - **Resource name** (e.g., `my-company-openai`)
   - **Deployment name** (e.g., `gpt-4o`)
   - **API key** (found under **Keys and Endpoint**)

## Configure VibeCody

**Option 1: Environment variable**

```bash
export AZURE_OPENAI_API_KEY="your-key-here"
vibecli --provider azure_openai
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[azure_openai]
enabled = true
api_key = "your-key-here"
api_url = "https://my-company-openai.openai.azure.com"
model = "gpt-4o"
```

The `model` field should match your Azure deployment name. The `api_url` is your resource endpoint.

## Model Selection

Available models depend on what you have deployed in your Azure resource:

| Model | Strengths | Best for |
|-------|-----------|----------|
| `gpt-4o` | Strongest reasoning | Complex coding, architecture |
| `gpt-4o-mini` | Fast and affordable | Daily coding tasks |
| `o3-mini` | Chain-of-thought reasoning | Hard debugging, logic |

**Default:** `gpt-4o`

## Best For

- **Enterprise compliance** -- data stays within your Azure tenant and chosen region
- **Private networking** -- use with VNETs and private endpoints
- **Existing Azure infrastructure** -- integrates with Azure AD, RBAC, and monitoring
- **Data residency** -- choose specific Azure regions for data locality requirements

## Verify Connection

```bash
vibecli --provider azure_openai -c "List 3 advantages of using Azure for AI workloads"
```

## Troubleshooting

### 401 Unauthorized

- Verify your API key under **Keys and Endpoint** in the Azure portal
- Confirm the env var is set: `echo $AZURE_OPENAI_API_KEY`

### 404 Resource Not Found

- Check that `api_url` matches your resource endpoint exactly
- Verify the deployment name in `model` matches an active deployment

### 403 Forbidden

- Your Azure subscription may not have Azure OpenAI access approved
- Check region availability -- not all models are available in all regions
