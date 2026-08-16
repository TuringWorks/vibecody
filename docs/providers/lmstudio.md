---
layout: page
title: "Provider: LM Studio"
permalink: /providers/lmstudio/
---

LM Studio is a desktop app for downloading and running local models. It includes
an OpenAI-compatible server, so VibeCody talks to it the same way it talks to a
hosted provider — with a local base URL and, normally, no API key.

Use it when you want a GUI for browsing and swapping models, and you would
rather not manage a Python inference stack.

## Requirements

- [LM Studio](https://lmstudio.ai) installed
- At least one model downloaded through its Discover tab

## Start the server

In LM Studio: **Developer** tab → load a model → **Start Server**.

It listens on `http://localhost:1234/v1` by default — the URL VibeCody assumes.
The Developer tab shows the exact model id it will answer to; that string is
what you pass as the model.

To confirm it is up:

```bash
curl http://localhost:1234/v1/models
```

## Configure VibeCody

LM Studio accepts any bearer token, including none, so there is normally nothing
to configure. All three routes work:

**Nothing at all** — with the server on its default port,
`vibecli --provider lmstudio` works as-is.

**Config file** (`~/.vibecli/config.toml`):

```toml
[lmstudio]
api_url = "http://localhost:1234/v1"      # default
model = "qwen2.5-coder-7b-instruct"
```

**Environment** — `LMSTUDIO_BASE_URL` overrides the URL; `LMSTUDIO_API_KEY`
exists for completeness and is rarely needed:

```bash
export LMSTUDIO_BASE_URL=http://localhost:1234/v1
```

## Models

LM Studio serves whichever model you loaded in the app. The list below is what
the pickers offer as a starting point, not a claim about your machine — every
picker also accepts a typed-in id, and `/v1/models` on your running server is
the authority.

Model ids are LM Studio's own short names, which differ from the Hugging Face
repo paths used by vLLM: `qwen2.5-coder-7b-instruct`, not
`Qwen/Qwen2.5-Coder-7B-Instruct`.

| Model | Notes |
|---|---|
| `qwen2.5-coder-7b-instruct` | **Default.** Good coding model on modest hardware |
| `qwen2.5-coder-14b-instruct` | Stronger; wants more RAM/VRAM |
| `meta-llama-3.1-8b-instruct` | General-purpose |
| `mistral-7b-instruct-v0.3` | Small and fast |
| `phi-3.5-mini-instruct` | Smallest of the set |

## Verify

```bash
vibecli --provider lmstudio "Explain the borrow checker in two sentences"
```

## Troubleshooting

**Connection refused** — the server is not running. Loading a model in LM Studio
is not enough; the Developer tab's **Start Server** must be on.

**404 on `/chat/completions`** — the base URL is missing the `/v1` suffix.
VibeCody appends `/chat/completions`, so `api_url` must end in `/v1`.

**"Model not found"** — the id in your config does not match what is loaded. Copy
the id from the Developer tab, or from `curl http://localhost:1234/v1/models`.
The short-name convention is easy to get wrong if you are used to HF paths.

**Replies are very slow** — LM Studio falls back to CPU when a model does not fit
in VRAM. Its own UI reports the split; a smaller model or a heavier quantisation
usually fixes it.

## See also

- [Provider comparison](../) — all providers side by side
- [vLLM](../vllm/) — the other local OpenAI-compatible server, higher throughput, no GUI
- [Ollama](../ollama/) — local models from the command line
