---
layout: page
title: "Provider: Vercel AI"
permalink: /providers/vercel-ai/
---

# Vercel AI Provider

[Vercel AI](https://vercel.com/ai) is a gateway that provides unified access to multiple AI providers. Use it when you have an existing Vercel AI setup or want to route through Vercel's infrastructure.


## Prerequisites

1. A Vercel account with AI access
2. Your Vercel AI gateway URL
3. An API key for the underlying provider


## Configure VibeCody

**Config file** (`~/.vibecli/config.toml`)

```toml
[vercel_ai]
enabled = true
api_key = "your-provider-api-key"
api_url = "https://your-vercel-ai-endpoint.vercel.app/api"
model = "gpt-4o"
```

Both `api_key` and `api_url` are required. The API URL points to your Vercel AI gateway endpoint.


## Model Selection

Available models depend on what your Vercel AI gateway is configured to serve.

**Default:** `gpt-4o`

Override from the CLI:

```bash
vibecli --provider vercel_ai --model gpt-4o-mini
```


## Best For

- **Existing Vercel infrastructure** -- route AI through your Vercel deployment
- **Custom middleware** -- add logging, caching, or rate limiting via Vercel functions
- **Multi-provider gateway** -- use Vercel AI as a proxy to multiple backends


## Verify Connection

```bash
vibecli --provider vercel_ai -c "Hello, which model are you?"
```


## Troubleshooting

### Connection refused

- Verify `api_url` points to a running Vercel AI endpoint
- Check that the endpoint supports the OpenAI-compatible chat completions format

### Authentication error

- The `api_key` is passed to the underlying provider through Vercel -- ensure it is valid for that provider
