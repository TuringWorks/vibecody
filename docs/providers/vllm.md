---
layout: page
title: "Provider: vLLM"
permalink: /providers/vllm/
---

vLLM is a high-throughput inference server you run yourself. It exposes an
OpenAI-compatible API, so VibeCody talks to it exactly as it talks to a hosted
provider — the only differences are the base URL and that there is usually no
API key at all.

Use it when you want to serve a large open-weight model on your own hardware
with batching and paged attention, rather than one request at a time.

## Requirements

- A machine with a supported GPU (vLLM's CPU backend exists but is slow enough
  that the desktop experience suffers)
- vLLM installed: `pip install vllm`

## Start the server

```bash
vllm serve meta-llama/Llama-3.1-8B-Instruct
```

That listens on `http://localhost:8000/v1` — the default VibeCody assumes. The
server prints the model id it is serving; that string is what you pass as the
model, and it is the same id you gave `vllm serve`.

To confirm it is up and see exactly what it will answer to:

```bash
curl http://localhost:8000/v1/models
```

## Configure VibeCody

No key is needed unless you started vLLM with `--api-key`. All three routes
work:

**Nothing at all** — if vLLM is on the default port, `vibecli --provider vllm`
works with no configuration.

**Config file** (`~/.vibecli/config.toml`) — for a non-default host or port:

```toml
[vllm]
api_url = "http://gpu-box.local:8000/v1"
model = "meta-llama/Llama-3.3-70B-Instruct"
```

**Environment** — `VLLM_BASE_URL` overrides the URL, `VLLM_API_KEY` supplies a
key if you started the server with one:

```bash
export VLLM_BASE_URL=http://gpu-box.local:8000/v1
```

If you did start it with `--api-key`, store the key encrypted rather than in a
file:

```bash
vibecli set-key vllm <key>
```

## Models

vLLM serves whatever you passed to `vllm serve` — one model per server process.
The list below is what the pickers offer as a starting point, not a claim about
your machine; every picker also accepts a typed-in id, and `/v1/models` on your
running server is the authority.

| Model | Notes |
|---|---|
| `meta-llama/Llama-3.1-8B-Instruct` | **Default.** Fits on a single 24 GB card |
| `meta-llama/Llama-3.3-70B-Instruct` | Needs multiple GPUs or heavy quantisation |
| `Qwen/Qwen2.5-Coder-32B-Instruct` | Strongest coding model of this set |
| `Qwen/Qwen2.5-Coder-7B-Instruct` | Coding model for a single card |
| `mistralai/Mistral-7B-Instruct-v0.3` | Small and fast |
| `microsoft/Phi-3.5-mini-instruct` | Smallest; useful on constrained hardware |

## Verify

```bash
vibecli --provider vllm "Explain the borrow checker in two sentences"
```

## Troubleshooting

**"vLLM request failed" / connection refused** — nothing is listening. Check the
server is running and on the port you configured: `curl http://localhost:8000/v1/models`.

**404 on `/chat/completions`** — the base URL is missing the `/v1` suffix.
VibeCody appends `/chat/completions` to whatever you set, so `api_url` must end
in `/v1`.

**The model picker shows a model your server does not have** — the listed models
are suggestions. vLLM serves exactly one model, the one named in `vllm serve`.
Type that id into the picker, or set it in `config.toml`.

**401 Unauthorized** — the server was started with `--api-key` but VibeCody has
no key for it. Set it with `vibecli set-key vllm <key>`.

## See also

- [Provider comparison](../) — all providers side by side
- [LM Studio](../lmstudio/) — the other local OpenAI-compatible server, with a GUI
- [Ollama](../ollama/) — simpler local option, no GPU tuning required
