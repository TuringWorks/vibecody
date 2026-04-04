---
layout: page
title: "Provider: Zhipu GLM"
permalink: /providers/zhipu/
---

# Zhipu GLM Provider

[Zhipu AI](https://www.zhipuai.cn) develops the GLM (General Language Model) series, a family of Chinese AI models with strong multilingual and coding abilities.


## Get an API Key

1. Go to [open.bigmodel.cn](https://open.bigmodel.cn)
2. Create an account or sign in
3. Navigate to **API Keys**
4. Create a new key and copy it

Your key will be in the format `id.secret` (two parts separated by a dot).


## Configure VibeCody

**Option 1: Environment variable** (recommended)

```bash
export ZHIPU_API_KEY="your-id.your-secret"
vibecli --provider zhipu
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[zhipu]
enabled = true
api_key = "your-id.your-secret"
model = "glm-4"
```


## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `glm-4` | Strongest reasoning | Complex tasks, coding |
| `glm-4-flash` | Fast, affordable | Quick tasks |
| `glm-4-air` | Good balance | Daily coding |

**Default:** `glm-4`

Override from the CLI:

```bash
vibecli --provider zhipu --model glm-4-flash
```


## Authentication

Zhipu uses JWT-based authentication. VibeCody generates a JWT from the secret portion of your API key (using HMAC-SHA256) with a 1-hour expiry. This is handled automatically -- just provide the full `id.secret` key.


## Best For

- **Chinese language tasks** -- native Chinese understanding and generation
- **Multilingual coding** -- handles code with Chinese comments and documentation
- **Alternative to Western models** -- independent model family


## Verify Connection

```bash
vibecli --provider zhipu -c "Write a Python function to sort a list of dictionaries by key"
```


## Troubleshooting

### Authentication error

- Verify your key is in `id.secret` format (two parts separated by a dot)
- The key is used to generate a JWT internally -- both parts are required

### Connection issues

- Zhipu servers are hosted in China
- Some regions may experience higher latency
