---
layout: page
title: "Provider: VibeCLI mistral.rs (local)"
permalink: /providers/vibecli-mistralrs/
---

`vibecli-mistralrs` runs models **inside the VibeCLI daemon** via [mistral.rs](https://github.com/EricLBuehler/mistral.rs) — no Ollama, no sidecar, no second process. It is the other local option alongside [Ollama](/vibecody/providers/ollama/): Ollama is a separate server you install and manage, this one is the daemon you are already running.

Weights are pulled from Hugging Face on first use and cached; inference is in-process.

## Availability

The backend is compiled in, not installed at runtime, so whether it exists depends on how your `vibecli` was built:

| Build | In-process mistral.rs |
|-------|----------------------|
| macOS (any official build) | **On** — Metal acceleration is the expected configuration |
| Linux / Windows official builds | Off unless built with the feature |
| Built from source | `cargo build --release -p vibecli --features vibe-mistralrs` |

Cargo features cannot be conditioned on target OS, so `vibecli/vibecli-cli/build.rs` computes the union and emits a single `cfg(mistralrs_enabled)`. Accelerator variants (`mistralrs-cuda`, `mistralrs-metal`, `mistralrs-flash-attn`) are selected in `vibe-infer`.

Check what your binary has:

```bash
curl -s localhost:7878/health | jq '.mistralrs_recommended_default'
```

A value means the backend is present. No daemon, no answer — see [Daemon startup](/vibecody/vibecli/).

## Configure VibeCody

There is **no API key**. Authentication is the daemon's own bearer token, read from `~/.vibecli/daemon.token` on every request — so a daemon restart, which rotates that token, needs no reconfiguration.

```bash
vibecli --provider vibecli-mistralrs
```

In VibeCoder it appears in the toolbar model dropdown with no key configured. Both `vibecli-mistralrs` and `vibecli_mistralrs` are accepted as the provider id.

**Config file** (`~/.vibecli/config.toml`) — only needed to point at a non-default daemon:

```toml
[vibecli_mistralrs]
enabled = true
model = "Qwen/Qwen2.5-Coder-7B-Instruct"
api_url = "http://localhost:7878"   # optional, default shown
```

The provider speaks the Ollama HTTP wire format against the daemon but pins the backend by sending `X-VibeCLI-Backend: mistralrs`, which outranks the daemon default and any per-model pin. Backend selection precedence is header → body field → per-model pin → daemon default.

## Gated weights and `HF_TOKEN`

The Llama models are **gated on Hugging Face** — loading one without an accepted licence returns 401. VibeCody handles this rather than letting you discover it at first token:

- `/health` reports `mistralrs_recommended_default`, which is `meta-llama/Llama-3.1-8B-Instruct` when `HF_TOKEN` is set and `Qwen/Qwen2.5-Coder-7B-Instruct` (Apache-2.0, ungated) when it is not.
- VibeCoder reads that field on launch and swaps its picker default in place, so it never pre-selects a model that would 401.

Set the token the encrypted way — it is a credential, so it does not belong in `config.toml`:

```bash
vibecli set-key huggingface "hf_..."
```

The daemon hydrates `HF_TOKEN` from the ProfileStore at startup, so downstream libraries see it without a shell export.

## Model Selection

| Model | Size | Notes |
|-------|------|-------|
| `Qwen/Qwen2.5-Coder-7B-Instruct` | 7B | Ungated, code-specific — the default when `HF_TOKEN` is absent |
| `meta-llama/Llama-3.1-8B-Instruct` | 8B | Gated — the default when `HF_TOKEN` is present |
| `Qwen/Qwen3.6-Coder-7B-Instruct` | 7B | Newer Qwen coder generation |
| `Qwen/Qwen2.5-Coder-1.5B-Instruct` | 1.5B | Fits comfortably on modest hardware |
| `Qwen/Qwen2.5-0.5B-Instruct` | 0.5B | Smoke-testing the path |
| `microsoft/Phi-3.5-mini-instruct` | 3.8B | Small general model |
| `meta-llama/Llama-3.2-3B-Instruct` | 3B | Gated |
| `meta-llama/Llama-3.2-1B-Instruct` | 1B | Gated |

Override from the CLI:

```bash
vibecli --provider vibecli-mistralrs --model Qwen/Qwen2.5-Coder-1.5B-Instruct
```

Any model mistral.rs can load works — the list above is what the picker offers, not an allow-list.

## KV-cache compression

The in-process backend can store attention KV cache with TurboQuant (PolarQuant + QJL) instead of fp16, measured at ~4.6× smaller at `head_dim=128` with 100% top-1 attention-argmax agreement against fp16 on realistic data. It is opt-in:

```bash
VIBE_INFER_KV_CACHE=turboquant vibecli --serve --port 7878
```

VibeCoder's Inference panel exposes the same choice as a **KV Cache** dropdown when the Mistral.rs backend is selected.

## Best For

- **Fully offline work** — no network once weights are cached
- **One less moving part** — no Ollama install or sidecar to keep running
- **Apple Silicon** — Metal acceleration is on by default in macOS builds

## Verify Connection

```bash
vibecli --provider vibecli-mistralrs -c "Write a Python async web scraper"
```

First run downloads weights, so it will be slow before it is fast.

## Troubleshooting

### Provider missing from the picker

The binary was built without the backend. Check `/health` as above; rebuild with `--features vibe-mistralrs` on Linux/Windows.

### 401 from Hugging Face

A gated model without a token. Either accept the licence and `vibecli set-key huggingface`, or switch to `Qwen/Qwen2.5-Coder-7B-Instruct`. The daemon substitutes the ungated fallback and says so when it can.

### 401 from the daemon

`~/.vibecli/daemon.token` is missing or stale. The token rotates on every daemon start; the provider re-reads the file per request, so this usually means the daemon is not running rather than that the token is wrong.

### Out of memory on load

Drop to a smaller model — `Qwen/Qwen2.5-Coder-1.5B-Instruct` or `Qwen/Qwen2.5-0.5B-Instruct`. Model size is the dominant term; a 7B at fp16 wants roughly 14 GB before cache.
