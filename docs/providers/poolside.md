---
layout: page
title: "Provider: Poolside"
permalink: /providers/poolside/
---

[Poolside](https://poolside.ai) trains models purpose-built for software engineering rather than general chat. VibeCody talks to its OpenAI-compatible endpoint at `https://inference.poolside.ai/v1`.

## Get an API Key

1. Go to [poolside.ai](https://poolside.ai) and request access — Poolside is enterprise-first, so keys are issued per organization rather than self-serve
2. Once provisioned, create a key from the console
3. Copy it

There is no free tier.

## Configure VibeCody

**Option 1: Encrypted store** (recommended)

```bash
vibecli set-key poolside "..."
vibecli --provider poolside
```

The key is stored encrypted in `~/.vibecli/profile_settings.db`, not in any plaintext file. See [Secure settings storage](/vibecody/security/).

**Option 2: Environment variable**

```bash
export POOLSIDE_API_KEY="..."
vibecli --provider poolside
```

**Option 3: Config file** (`~/.vibecli/config.toml`)

```toml
[poolside]
enabled = true
model = "poolside/laguna-s-2.1"
api_url = "https://inference.poolside.ai/v1"   # optional, default shown
```

Prefer option 1 or 2 for the key itself — VibeCody's storage rules keep credentials out of `config.toml`.

In VibeCoder, the key field is under **Settings → Configuration → Keys**, and Poolside appears in the toolbar model dropdown once a key is set.

## Model Selection

| Model | Strengths | Best for |
|-------|-----------|----------|
| `poolside/laguna-s-2.1` | Balanced coding model | Daily tasks (default) |
| `poolside/laguna-xs-2.1` | Smaller, faster | Quick completions, high-volume calls |
| `poolside/laguna-m-1` | Larger, earlier generation | Harder reasoning |

**Default:** `poolside/laguna-s-2.1`

Override from the CLI:

```bash
vibecli --provider poolside --model poolside/laguna-xs-2.1
```

See [Poolside's supported-models list](https://docs.poolside.ai/get-started/supported-models) for the authoritative set — the table above is what VibeCody ships as its picker defaults, not a live query.

> **Known inconsistency.** The daemon's `/models` catalog lists these three
> without the `poolside/` prefix (`laguna-s-2.1`), while the CLI default, the
> VibeCoder registry and the Tauri engine all use the prefixed form. If a
> daemon-driven picker hands you a bare `laguna-*` id and the API rejects it,
> pass the prefixed name explicitly with `--model`.

## Best For

- **Code-specific models** — trained for software engineering rather than adapted from a general chat model
- **Enterprise deployments** — per-organization key issuance and a configurable `api_url` for a private proxy

## Verify Connection

```bash
vibecli --provider poolside -c "Write a Python async web scraper"
```

## Troubleshooting

### `Poolside API key not set (POOLSIDE_API_KEY)`

No key reached the provider. Check `vibecli list-keys` shows `poolside`, or that `POOLSIDE_API_KEY` is exported in the shell that launched VibeCody.

### 401 / 403

The key is present but rejected — confirm it belongs to an organization with inference access, and that it has not been rotated in the Poolside console.

### Model not found

Pass the fully-qualified id (`poolside/laguna-s-2.1`) rather than the bare model name, and check the [supported-models list](https://docs.poolside.ai/get-started/supported-models) — Poolside retires generations.

### Routing through a proxy

Set `api_url` to your gateway. VibeCody appends `/chat/completions` to whatever you give it, so supply the base including any version segment.
