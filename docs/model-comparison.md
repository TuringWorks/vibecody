---
layout: page
title: Model Comparison
permalink: /model-comparison/
---

> A practical guide to picking the right model for the job across every provider VibeCody supports.
> Last updated: **2026-09-02** (deprecated-model sweep across every provider table).
>
> **Caveat**: model leaderboards shift weekly. Treat the strength/weakness blurbs as a *shape* of each model's bias (what it was trained for), not a final benchmark verdict. When in doubt, run the same prompt through two candidates side-by-side in VibeCoder's MultiModel panel.

## Where models run

Three different execution shapes hide behind the model picker. Pick the row that matches your privacy / cost / capability needs:

> **`vibecli-mistralrs` — runs on your machine.**
> Weights cached at `~/.cache/huggingface/hub`, forward passes execute on your CPU / Metal / CUDA. Nothing leaves the host. Limited to ~7B-class models on a laptop. Default for the privacy path.
> **Shipped in the macOS downloads only** — on Linux and Windows the backend is not compiled in and answers `Unavailable` until you build from source ([why](#platform-support)).
>
> **`ollama` — runs locally OR on Ollama Cloud.**
> Without an `ollama` API key, only models you've `ollama pull`-ed run (locally). With an API key, large cloud-hosted models (`devstral-2:123b-cloud`, `nemotron-3-super`, etc.) route to Ollama Cloud transparently. Open-weights only, scales up to 100B+ MoE.
>
> **Cloud APIs (`claude`, `openai`, `gemini`, `grok`, `mistral`, `deepseek`, `cerebras`, `perplexity`, `together`, `fireworks`, `openrouter`, `azure_openai`, `bedrock`, `copilot`, `zhipu`, `vercel_ai`, `minimax`, `sambanova`) — runs on the provider's hardware.**
> Closed-weights flagships live here. Inputs and outputs traverse the network; check each provider's data-handling terms.

The daemon serves all three from the same HTTP surface (`:7878`), so a remote VibeCoder / VibeMobile / VibeWatch client can use any of them. The choice of provider determines *where the model itself executes*, not how the client connects.

## Notation

- **Ctx** — maximum context window (input tokens).
- **Tools** — native function/tool calling support: ✅ first-class, ⚠️ supported but quirky, ❌ none.
- **Vision** — accepts image input.
- **Reasoning** — model does explicit chain-of-thought / "thinking" tokens before answering.
- **Open** — open-weights (you can self-host).

---

## Pick by task

The "right" pick depends on what you're doing. Use this matrix as a starting point, then verify with the MultiModel panel in VibeCoder.

### Coding agent (multi-step file edits, run-and-fix loops)

| Tier | Cloud-hosted | Open-weights (Ollama Cloud) | Local pull |
|---|---|---|---|
| **Flagship** | Claude Sonnet 5, gpt-5.6-sol, gpt-5.3-codex | devstral-2 (123B) | devstral-small-2 |
| **Strong** | Claude Sonnet 4.6, gpt-5.5 | qwen3-coder | qwen2.5-coder:7b |
| **Cheap/fast** | Claude Haiku 4.5, gpt-5.6-luna | ministral-3, devstral-small-2 | qwen2.5-coder:1.5b |

### One-shot reasoning, math, hard algorithms

| Tier | Cloud-hosted | Open-weights | Local pull |
|---|---|---|---|
| **Flagship** | Claude Fable 5.1, Claude Opus 5, gpt-5.6-sol | nemotron-3-super, deepseek-v4-pro | deepseek-r1:14b |
| **Strong** | gemini-3.1-pro-preview, gpt-5.5 | glm-5.1, magistral | qwq:32b |
| **Cheap** | gpt-5.6-luna, Claude Haiku 4.5 | nemotron-3-nano | phi4-reasoning |

### Long context (≥200k tokens)

| Tier | Provider · Model |
|---|---|
| **Flagship** | gemini-3.1-pro-preview (1M+), gpt-5.6-sol (922k in), Claude Opus 5 (1M) |
| **Strong** | gemini-3.6-flash (1M+), grok-4.3 (1M) |
| **Open** | qwen3-next, llama4 (variable) |

### Vision (image input)

| Tier | Provider · Model |
|---|---|
| **Flagship** | gemini-3.1-pro-preview, gpt-5.6-sol, Claude Sonnet 5 |
| **Strong** | gemini-3.6-flash, grok-4.6, gpt-5.5 |
| **Open** | qwen3-coder (vision variant), llama4 vision |
| **Local** | llama3.2-vision, gemma3 |

### Cheap & fast tool-calling agents

| Tier | Provider · Model |
|---|---|
| **Cloud** | Claude Haiku 4.5, gpt-5.6-luna, gemini-3.5-flash-lite |
| **Open cloud** | ministral-3, devstral-small-2, gemma4 |
| **Local** | phi4-mini, llama3.2:3b, qwen2.5:1.5b |

### Privacy / fully offline

| Tier | Engine · Model |
|---|---|
| **Daemon (mistralrs)** | Qwen2.5-7B-Instruct, Qwen2.5-Coder-7B, Phi-3.5-mini |
| **Ollama local** | devstral-small-2, qwen2.5-coder:7b, llama3.2:3b |

### Web search / news-aware

| Tier | Provider · Model |
|---|---|
| **Native** | Perplexity Sonar Pro, Sonar Reasoning |
| **With tools** | gpt-5.6-sol + browser tool, Claude Sonnet 5 + web tool |

---

## Providers and models

Below: every provider VibeCody ships, the models we expose in the picker, and what each one is actually good at. Flagships get deeper dives; secondary models get one-liners.

### Anthropic Claude (`claude`)

Three-tier family — Opus (deepest reasoning), Sonnet (balanced workhorse), Haiku (fast/cheap). All three support tool calling, vision, and adaptive thinking. Default in VibeCody is `claude-opus-5`.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| claude-fable-5-1 | 1M | ✅ | ✅ | ✅ | Most capable widely released model |
| claude-opus-5 | 1M | ✅ | ✅ | ✅ | Flagship reasoning + agent default |
| claude-fable-5 | 1M | ✅ | ✅ | ✅ | Previous Fable release |
| claude-sonnet-5 | 1M | ✅ | ✅ | ✅ | Current Sonnet — near-Opus coding at Sonnet cost |
| claude-haiku-4-5 | 200k | ✅ | ✅ | ✅ | Current Haiku — cheap/fast tool calls + classification |
| claude-opus-4-8 | 1M | ✅ | ✅ | ✅ | Previous-gen Opus |
| claude-opus-4-7 | 1M | ✅ | ✅ | ✅ | Older Opus |
| claude-opus-4-6 | 1M | ✅ | ✅ | ✅ | Older Opus |
| claude-sonnet-4-6 | 1M | ✅ | ✅ | ✅ | Previous-gen Sonnet |

Every row above is marked `Active` on Anthropic's own model-status table, checked
2026-09-02. `claude-fable-5-1` was added that day; `claude-sonnet-4-5` was dropped
the same day — still Active, but its tentative retirement is "not sooner than
2026-09-29", inside a release cycle. `claude-haiku-4-5` is the next shortest
runway (not sooner than 2026-10-15) and stays only because it is Anthropic's sole
cheap tier.

`claude-3-5-sonnet-20241022` was dropped on 2026-08-05 — Anthropic retired it on
2025-10-28 and it now 404s. Retired alongside it: Claude 3 Opus (2026-01-05),
Claude 3.5 Haiku and Claude 3.7 Sonnet (both 2026-02-19).

**claude-opus-5** — Strongest at sustained agentic loops with many tools and many turns. It rarely loses the plot on long sessions and is willing to push back on bad instructions. Most expensive of the three. Use when latency doesn't matter and the work is hard.

**claude-sonnet-5** — The model most VibeCody users will actually run. Roughly Opus-level coding quality on common tasks, ~3-4× cheaper, ~2× faster. Default for the VibeCoder Code panel.

**claude-haiku-4-5** — Surprisingly capable for its tier; handles routine tool-calling, summarization, intent classification. Don't use it for novel architecture or deep debugging — it gets confidently wrong.

### `claude-code` (local Claude Code CLI passthrough)

Same Anthropic models (Fable 5.1, Opus 5, Sonnet 5, Haiku 4.5, …), but billed against the user's Claude.ai Free/Pro/Max/Team/Enterprise plan instead of API credits. Same capabilities; payment shape differs.

### OpenAI (`openai`)

As of September 2026: the **GPT-5.6 line** (`-sol`, `-terra`, `-luna`) is the current flagship family with built-in adaptive reasoning; **GPT-5.5 / 5.4** remain as cheaper previous flagships; the **codex variant** (gpt-5.3-codex) is coding-tuned for agent loops. VibeCody's default is `gpt-5.6-sol`. The o-line (o3, o3-mini, o4-mini) and gpt-4-turbo were dropped from the picker on 2026-08-05.

> **There is no `gpt-5.6-*-pro` model id.** This page and the picker both listed
> `gpt-5.6-sol-pro`, `gpt-5.6-terra-pro` and `gpt-5.6-luna-pro` until 2026-09-02.
> "Pro" on the 5.6 family is a *request parameter* — `reasoning.mode: "pro"` on
> the Responses API — not a separate model; OpenAI's own deprecation table spells
> the replacement for `gpt-5-pro-2025-10-06` as "gpt-5.6-sol (reasoning.mode:
> pro)". The three ids could only ever 404. `gpt-5.5-pro` is real: the separate
> `-pro` id was retired *with* the 5.6 generation, not before it.

> **The GPT-4 line stays in the picker.** `gpt-4o`, `gpt-4o-mini`, `gpt-4.1` and
> `gpt-4.1-mini` are two generations behind the rest of the table and OpenAI's
> model guidance routes all four to the 5.6 family — but they are still callable,
> and OpenAI lists all four Active. Leaving ChatGPT on 2026-02-13 retired them
> from the consumer product, not from the API. They were cut in the 2026-09-02
> sweep as "superseded" and restored the same day: this page drops ids that fail,
> not ids that are merely old.
>
> Their neighbours in that generation are genuinely gone. `gpt-4.1-nano`,
> `gpt-4-turbo` and `gpt-3.5-turbo` are deprecated with an API shutdown on
> 2026-10-23. `gpt-5` went with them: its only snapshot, `gpt-5-2025-08-07`, was
> deprecated 2026-06-11 and shuts down 2026-12-11.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| gpt-5.6-sol | 1.05M (922k in) | ✅ | ✅ | ✅ | Current flagship — default in VibeCody |
| gpt-5.6-terra | 1M | ✅ | ✅ | ✅ | Balanced 5.6 tier |
| gpt-5.6-luna | 1M | ✅ | ✅ | ✅ | Cheapest 5.6 tier |
| gpt-5.5-pro | 1M | ✅ | ✅ | ✅ | Previous flagship, high effort |
| gpt-5.5 | 1M | ✅ | ✅ | ✅ | Previous flagship |
| gpt-5.4 | 1M | ✅ | ✅ | ✅ | Older flagship; cheaper |
| gpt-5.4-mini | 1M | ✅ | ✅ | ⚠️ | Cheap 5.4 variant |
| gpt-5.3-codex | 200k | ✅ | ❌ | ✅ | Coding-tuned, agent-loop optimised |
| gpt-5.3-chat | 200k | ✅ | ✅ | ⚠️ | Chat-tuned 5.3 |
| gpt-4.1 | 1M | ✅ | ✅ | ❌ | Long-context GPT-4 flagship; non-reasoning baseline |
| gpt-4.1-mini | 1M | ✅ | ✅ | ❌ | Fast long-context |
| gpt-4o | 128k | ✅ | ✅ | ❌ | Workhorse multimodal, omni input/output |
| gpt-4o-mini | 128k | ✅ | ✅ | ❌ | Fast/cheap variant |

**gpt-5.6-sol** — OpenAI's current general-purpose flagship. 1M-token context with strong long-range retrieval, built-in adaptive reasoning (the model decides per-prompt how much "thinking" to spend), native vision, rock-solid tool calling. Default in VibeCody for the OpenAI provider.

**gpt-5.3-codex** — Coding-specialised GPT-5 variant; tuned for multi-step file edits, run-and-fix loops, and tool-heavy agent flows. Pick this over `gpt-5.6-sol` when the workload is overwhelmingly code-editing.

**gpt-5.6-luna** — The cheap tier of the current family ($0.20 / $1.20 per MTok at short context, list, 2026-09-02). Prefer it over `gpt-4o-mini` / `gpt-4.1-mini` for high-volume classification and routing on new work.

**gpt-4.1** — Still useful when you want a non-reasoning baseline at low cost, or an A/B against a GPT-5 output. 1M-token context retrieves well. For new work, default to a GPT-5 entry instead — and note it has no dated retirement, which is not the same as a commitment to keep it.

### Google Gemini (`gemini`)

Long context is the headline (1M+ across the line). The Gemini 3 generation (released Q1 2026) is competitive with GPT-5-class models on most general tasks and remains best-in-class for long-context retrieval. VibeCody's default is `gemini-3.6-flash`. The 2.0 line was dropped from the picker on 2026-08-05; the 2.5 line followed on 2026-09-02.

> **`gemini-3.5-pro` is not in the picker, because it has never shipped.** Google announced it at I/O on 2026-05-19 and it has slipped three times; as of August 2026 it remains a limited Vertex AI preview for selected enterprise customers, absent from the consumer Gemini app and AI Studio. This page previously listed it as the current flagship and as VibeCody's default — both were wrong, written from a projected release date. Corrected 2026-08-10.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| gemini-3.8-flash | 1M+ | ✅ | ✅ | ✅ | Newest flash tier (2026-09-02) |
| gemini-3.7-flash | 1M+ | ✅ | ✅ | ✅ | Previous flash (2026-08-13) |
| gemini-3.6-flash | 1M+ | ✅ | ✅ | ✅ | **Default in VibeCody** |
| gemini-3.5-flash | 1M+ | ✅ | ✅ | ⚠️ | Cheap workhorse |
| gemini-3.5-flash-lite | 1M+ | ✅ | ❌ | ❌ | Cheapest tier |
| gemini-3.1-flash-lite | 1M+ | ✅ | ❌ | ❌ | Cheapest tier, prior gen |
| gemini-3.1-pro-preview | 1M+ | ✅ | ✅ | ✅ | Current Pro tier — still preview |

> **The Pro tier has no GA id, and this page had the wrong one twice.** After
> `gemini-3.5-pro` was removed on 2026-08-10 for never having shipped, the
> 2026-09-02 sweep found `gemini-3.1-pro` and `gemini-3-pro` here and in the
> picker — neither is a callable model code. Google ships the current Pro as
> `gemini-3.1-pro-preview`, and `gemini-3-pro-preview` is already in the
> deprecated/shut-down table. The `-preview` suffix is load-bearing: an id
> written the way the marketing name reads is not an id.

> **The 2.5 line left the picker on 2026-09-02.** A newly created GCP project
> gets `404 … no longer available to new users` for `gemini-2.5-pro`, so for most
> callers it is already retired ahead of its published date, and the whole 2.5
> line goes no earlier than 2026-10-16.

**gemini-3.6-flash** — the current default for the Gemini provider (shipped 2026-07-21). Google's workhorse tier: roughly 17% fewer output tokens than the model it replaced, with tools, vision, and reasoning across a 1M+ window. Tool calling caught up to Claude/GPT-5 with the 3.x line; argument-shape hallucinations on complex tools have largely cleared. The default stays here rather than on 3.8/3.7 until those two have a track record.

**gemini-3.1-pro-preview** — the strongest Gemini in the picker for deep long-context work, at $2 / $12 per MTok for prompts ≤200k (list, 2026-09-02). Being a preview id, it can move without a deprecation window — pin deliberately.

### xAI Grok (`grok`)

Strong on real-time / news-aware tasks (it has live X data feed integration on the back end). Decent coding ability; tool calling is solid across the 4.x line.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| grok-4.6 | 500k | ✅ | ✅ | ✅ | Flagship (2026-08-12) — VibeCody default |
| grok-4.5 | 256k | ✅ | ✅ | ✅ | Previous flagship |
| grok-4.3 | 1M | ✅ | ✅ | ✅ | Previous gen |

The grok-3 and grok-2 entries were dropped on 2026-08-05 — superseded by the 4.x
line, and requests to them redirect to grok-4.3. Bare `grok-4.20` was dropped on
2026-09-02: docs.x.ai lists no such id — that generation is addressed as
`grok-4.20-0309-reasoning` / `-non-reasoning`.

**grok-4.6** — Useful when the task involves recent events, market data, or code where the relevant docs were published in the last few months — it tends to be more current than rivals. Tool calling works but the JSON schema adherence is fussier than Claude's.

### Mistral (`mistral`)

European cloud provider, strong on multilingual and coding (Codestral). Tool calling is native and well-specced.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| mistral-large-latest | 128k | ✅ | ❌ | ❌ | General flagship |
| mistral-medium-latest | 128k | ✅ | ❌ | ❌ | Mid-tier balanced |
| mistral-small-latest | 32k | ✅ | ❌ | ❌ | Cheap/fast |
| codestral-latest | 32k | ✅ | ❌ | ❌ | Coding-tuned |

**codestral-latest** — Mistral's coding specialist. Excellent at completion and edit tasks; smaller than Devstral but covers most languages well. Use this for inline-style completions; use `devstral-2` (via Ollama) for full agentic loops.

### DeepSeek (`deepseek`)

Chinese provider; very strong reasoning (R1) and aggressively cheap pricing. Note: data residency / outbound traffic considerations apply if your project requires non-Chinese hosting.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| deepseek-v4-pro | 128k | ✅ | ❌ | ✅ | Current flagship (MIT, 2026-04-24) |
| deepseek-v4-flash | 128k | ✅ | ❌ | ⚠️ | Cheaper/faster V4 |

`deepseek-coder` was dropped on 2026-08-05 (folded into the chat line), as was
the bare `deepseek-v4` id — the shipped pair is `-pro` / `-flash`.
`deepseek-chat` and `deepseek-reasoner` were dropped on 2026-09-02: DeepSeek
retired both legacy names on 2026-07-24 and its model-list endpoint now returns
only the v4 pair. `deepseek-chat` had also been this provider's default model, so
every unconfigured DeepSeek call was aimed at a retired id.

**deepseek-v4-pro** — Strong at math and algorithmic reasoning at a fraction of frontier pricing. Tool calling is solid on the v4 line; verify your function schemas round-trip cleanly before relying on it for long agent loops.

### Cerebras (`cerebras`)

Inference-only platform — does not train models, but runs open weights at extreme speed (often 10-20× faster than typical cloud endpoints) on their wafer-scale hardware. The Llama-class entries were dropped on 2026-08-05; Cerebras no longer serves them.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| gpt-oss-120b | 128k | ✅ | ❌ | ✅ | Best-quality option — VibeCody default |
| gemma-4-31b | 128k | ✅ | ❌ | ⚠️ | Google open weights |
| zai-glm-4.7 | 128k | ✅ | ❌ | ✅ | Z.ai GLM on Cerebras hardware |

**gpt-oss-120b** on Cerebras — Use when you want frontier open-weights quality with 1000+ tokens/sec generation. Great for streaming-heavy chat UIs and agent loops where round-trip count dominates.

### Perplexity (`perplexity`)

Web-search-augmented chat. Models include browsing as a native step in their generation pipeline; you don't add a separate tool. Citations come back inline.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| sonar-pro | 200k | ⚠️ | ❌ | ❌ | Default. Web-grounded answers + citations |
| sonar | 128k | ⚠️ | ❌ | ❌ | Cheaper variant |
| sonar-reasoning-pro | 128k | ⚠️ | ❌ | ✅ | Reasoning + web search |
| sonar-deep-research | 128k | ⚠️ | ❌ | ✅ | Multi-step research reports |

Use Perplexity for "what's the latest on X" prompts where you need fresh sources. Don't use it for code generation or long agent loops — it isn't shaped for that.

### Together.ai (`together`)

Inference-only marketplace for open-weights models. We expose a handful of current frontier open weights; Together hosts dozens more — extend STATIC_MODELS if you need them. The Llama-3.3 / Mixtral defaults were dropped on 2026-08-05.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| moonshotai/Kimi-K2.7-Code | 256k | ✅ | ❌ | ✅ | Coding flagship — VibeCody default |
| Qwen/Qwen3.8-Max | 256k | ✅ | ❌ | ✅ | Alibaba flagship |
| Qwen/Qwen3.5-397B-A17B | 256k | ✅ | ❌ | ✅ | Large MoE |
| deepseek-ai/DeepSeek-V4-Pro | 128k | ✅ | ❌ | ✅ | Reasoning leader at low cost |

### Fireworks (`fireworks`)

Same shape as Together — inference-only, open-weights focus.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| accounts/fireworks/models/gpt-oss-120b | 128k | ✅ | ❌ | ✅ | Default; what Llama 3.3 was migrated to |
| accounts/fireworks/models/minimax-m3 | 1M | ✅ | ✅ | ✅ | 1M context + multimodal |

The two previous entries were replaced on 2026-09-02. Fireworks pulled its Llama
models from serverless after 2026-05-14 — `llama-v3p3-70b-instruct` was migrated
to `gpt-oss-120b` — and the Mixtral endpoints went with them. Fireworks serves a
large rotating catalogue; `GET /v1/models` on your account is the authority and
the picker accepts a typed id.

### OpenRouter (`openrouter`)

Aggregator front-end — one API key, hundreds of models routed to the cheapest/fastest available backend. Useful for quick experimentation across models, less ideal as a production primary because pricing and latency vary by route.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| anthropic/claude-opus-5 | 1M | ✅ | ✅ | ✅ | Claude route — VibeCody default |
| anthropic/claude-sonnet-5 | 1M | ✅ | ✅ | ✅ | Cheaper Claude route |
| openai/gpt-5.6-sol | 1M | ✅ | ✅ | ✅ | OpenAI passthrough |
| google/gemini-3.6-flash | 1M+ | ✅ | ✅ | ✅ | Cheap long context |
| moonshotai/kimi-k3 | 256k | ✅ | ❌ | ✅ | 3T-class open weights |
| moonshotai/kimi-k2.7-code | 256k | ✅ | ❌ | ✅ | Coding-tuned |
| moonshotai/kimi-k2.6 | 256k | ✅ | ❌ | ✅ | Previous Kimi |
| z-ai/glm-5.2 | 200k | ✅ | ❌ | ✅ | Open-weights intelligence leader |
| qwen/qwen3.8-max | 256k | ✅ | ❌ | ✅ | Alibaba flagship |
| deepseek/deepseek-v4-pro | 128k | ✅ | ❌ | ✅ | Reasoning at low cost |
| minimax/minimax-m3 | 1M | ✅ | ✅ | ✅ | 1M context + multimodal |
| x-ai/grok-4.5 | 256k | ✅ | ✅ | ✅ | News-aware |

The previous entries (`anthropic/claude-3.5-sonnet`, `openai/gpt-4o`,
`google/gemini-2.0-flash-001`, `deepseek/deepseek-v4`) were replaced on
2026-08-05 — the first is retired and the last was never a valid slug. This list
is still deliberately small; the full catalog is at `https://openrouter.ai/api/v1/models`.

### Azure OpenAI (`azure_openai`)

Enterprise Azure-region-pinned OpenAI deployments. Same models as `openai` but billed via Azure with regional / compliance guarantees.

| Model | Notes |
|---|---|
| gpt-5.6-sol | Current flagship — VibeCody default |
| gpt-5.6-terra | Balanced tier |
| gpt-5.6-luna | Cheapest tier |
| gpt-5.5 | Previous flagship |
| gpt-5.4 | Older flagship |

Azure deployment names are chosen by whoever created the deployment, so this is a
hint list of what Foundry currently offers rather than a claim about your
resource. `gpt-4o` and `gpt-4-turbo` were dropped on 2026-09-02: gpt-4-turbo
retired long ago, gpt-4o retires on Foundry 2026-10-01, and gpt-4o-mini went
2026-03-31.

### Amazon Bedrock (`bedrock`)

AWS-region-pinned Anthropic Claude (and others). Same models, AWS billing, IAM-gated. Bedrock is partner-operated, so availability per region can trail the first-party API and the feature surface is a subset — check the model is listed in your region before pinning it.

| Model | Notes |
|---|---|
| anthropic.claude-opus-5 | Current Opus route — VibeCody default |
| anthropic.claude-sonnet-5 | Current Sonnet route |
| anthropic.claude-opus-4-8 | Previous-gen Opus |
| anthropic.claude-haiku-4-5 | Cheap/fast route |

Bedrock ids take an `anthropic.` prefix on the otherwise-identical first-party id.
The previous entries were replaced on 2026-08-05: `claude-3-5-sonnet` retired
2025-10-28 and `claude-3-haiku` retires 2026-04-19. If a model isn't yet listed in
your region, pin an older entry from the table above.

### GitHub Copilot (`copilot`)

Copilot brokers models from OpenAI, Anthropic, Google, xAI and Microsoft. We expose it as a provider for users on Copilot Business/Enterprise who want to channel chat through that quota.

| Model | Notes |
|---|---|
| gpt-5.6-sol | Current flagship — VibeCody default |
| gpt-5.6-terra | Balanced tier |
| gpt-5.6-luna | Cheapest tier |
| gpt-5.5 | Previous flagship |
| gpt-5.4 | Older flagship |
| gpt-5.4-mini | Cheap tier |
| gpt-5.3-codex | Coding-tuned |

Only the OpenAI ids are listed. Copilot also brokers Claude, Gemini and Grok, but
its slugs for those are not the vendors' own (it spells them
`claude-opus-4.1`-style) and this table ships no id it has not verified. `GET
/models` on `api.githubcopilot.com` is the authority for a given account, and the
picker accepts a typed id. `gpt-4o` was dropped on 2026-09-02 — superseded, and
it had been the provider default.

### Ollama (`ollama`)

The most-used provider in VibeCody. `ollama` covers both **local-pulled** models (run on your machine) and **cloud-hosted** models (run on ollama.com when an API key is configured). The full library list lives in `vibecoder/src/constants/ollamaModels.ts`.

VibeCody's default Ollama model is **`devstral-2`** — Mistral's 123B coding-agent flagship, non-Chinese origin, native tool calling.

#### Cloud-hosted flagships (non-Chinese)

| Model | Origin | Best for | Notes |
|---|---|---|---|
| **devstral-2** | Mistral / France | **Coding agents** | 123B MoE, default. Tool calling native. |
| devstral-small-2 | Mistral / France | Cheaper coding | Smaller variant of devstral-2 |
| nemotron-3-ultra | NVIDIA / US | Deepest reasoning | 550B A55B, cloud-only |
| nemotron-3-super | NVIDIA / US | Reasoning | 120B A12B, RL-tuned for math/code reasoning |
| nemotron-3-nano | NVIDIA / US | Cheap reasoning | 30B A3B (cloud tag `nemotron-3-nano:30b-cloud`) |
| mistral-large-3 | Mistral / France | General | Cloud tag `mistral-large-3:675b-cloud` |
| cogito-2.1 | DeepCogito / US | Hybrid reasoning + tools | Newer entry; promising on agent benches |
| gemma4 | Google / US | General | Open-weights Gemini-adjacent |
| ministral-3 | Mistral / France | Cheap fast | Small but capable |

#### Cloud-hosted flagships (Chinese-origin)

These are technically excellent but may conflict with data-residency rules. Listed for completeness.

| Model | Origin | Notes |
|---|---|---|
| qwen3-coder, qwen3-coder-next | Alibaba | Strong coding model |
| qwen3-next, qwen3.5 | Alibaba | General-purpose |
| deepseek-v4-pro, deepseek-v4-flash | DeepSeek | Reasoning leader at low cost |
| glm-5.2, glm-5.1 | Zhipu | Strong agent eval scores |
| kimi-k3, kimi-k2.7-code, kimi-k2.6 | Moonshot | 3T-class; long context |
| minimax-m3, minimax-m2.7 | MiniMax | Agentic/reasoning hybrid; m3 adds 1M context |

**Retired from Ollama Cloud** (verified by a live `POST /api/show` on 2026-08-05,
which answers `410 Gone` with the retirement date): `glm-4.6`, `kimi-k2:1t` and
`minimax-m2` (all 2026-06-16), and `deepseek-v3.1:671b` (2026-07-15). Selecting one
returns a 410 the user can do nothing about, so re-verify tags the same way when
refreshing `constants/ollamaModels.ts` — the listing page shows base names without
their `:cloud` / `:<size>-cloud` suffix.

#### Notable local-pull models

| Model | Best for | Notes |
|---|---|---|
| qwen2.5-coder:7b | Local coding | Best small-coder; ~5GB RAM |
| llama3.3:70b | Local general | Needs 48GB+ VRAM |
| llama3.2:3b | Mobile-class chat | Runs on a laptop CPU |
| phi4 | Reasoning on small hardware | Microsoft, 14B-class |
| phi4-mini | Edge inference | ~3B-class |
| deepseek-r1:14b | Local reasoning | R1-distilled |
| codellama, starcoder2 | Older code completion | Kept for reproducibility |
| llama3.2-vision | Local vision | If you need image input offline |

#### devstral-2 vs nemotron-3-super (most-asked)

- **devstral-2** wins for **coding agents** — file edits, run-and-fix, multi-turn tool use. Trained specifically for that loop. SWE-Bench Verified ~58–62% per Mistral's release numbers.
- **nemotron-3-super** wins for **one-shot reasoning** — math, algorithms, "think first then answer" problems. Heavy RLHF on reasoning benches.
- For VibeCody's daemon (mostly multi-step coding/agent workloads), `devstral-2` is the default. Switch to `nemotron-3-super` in `useModelRegistry.ts:PROVIDER_DEFAULT_MODEL.ollama` if your usage is reasoning-heavy.

### VibeCLI mistralrs (`vibecli-mistralrs`)

Embedded-in-daemon inference. Talks to the local VibeCLI daemon (`:7878` by default) and pins the in-process mistralrs backend via `X-VibeCLI-Backend`. Models here are HuggingFace repo IDs that lazy-load on first use.

#### Platform support

The backend is a build-time feature, not a runtime one, so which download you have decides whether it exists at all.

| Platform | In the released binary | Notes |
|---|---|---|
| macOS (arm64) | ✅ Metal-accelerated | `build.rs` emits `cfg(mistralrs_enabled)` for every `target_os = "macos"` build; no flag to set. The only configuration exercised on a GPU in CI |
| macOS (Intel) | ✅ compiled in | Same target block. Cross-compiled on an arm64 runner, and its Metal path is not executed in CI |
| Linux (x86_64 / arm64) | ❌ | Build from source: `--features vibe-mistralrs` (CPU), `vibe-mistralrs-cuda` (NVIDIA), `vibe-mistralrs-flash-attn` (Ampere+, implies CUDA) |
| Windows (x86_64) | ❌ | Same feature flags exist; no CI job builds them |
| Docker image | ❌ | Linux base — as above |

Where it is not compiled in, every call returns `BackendError::Unavailable` — *"mistralrs backend not built — recompile vibecli with `--features vibe-mistralrs`"*. The daemon and the `ollama` path are unaffected, so the local-inference route on those platforms is Ollama.

VibeCody's default mistralrs model is **`meta-llama/Llama-3.1-8B-Instruct`** — Meta's most recent ~8B open-weights model with a 128k context window and tool-calling support.

| Model | Ctx | Best for | Notes |
|---|---|---|---|
| **meta-llama/Llama-3.1-8B-Instruct** | 128k | **Privacy-default — general + tools** | Default. Gated (see below) |
| meta-llama/Llama-3.2-3B-Instruct | 128k | Mid-tier general | Gated |
| meta-llama/Llama-3.2-1B-Instruct | 128k | Tiniest Llama | Gated |
| Qwen/Qwen2.5-Coder-7B-Instruct | 32k | Privacy-default coding | Apache-2.0, ungated |
| Qwen/Qwen2.5-7B-Instruct | 32k | General ~7B alternative | Apache-2.0, ungated |
| Qwen/Qwen2.5-Coder-1.5B-Instruct | 32k | Edge / fast coding | Apache-2.0 |
| Qwen/Qwen2.5-3B-Instruct | 32k | Mobile-class chat | Apache-2.0 |
| Qwen/Qwen2.5-1.5B-Instruct | 32k | Edge / fast general | Apache-2.0 |
| Qwen/Qwen2.5-0.5B-Instruct | 32k | Tiniest viable | Apache-2.0 |
| microsoft/Phi-3.5-mini-instruct | 128k | Smart-but-small reasoning | MIT, ungated |

**About gating** — Meta's Llama models are *gated repos* on HuggingFace: first-time download requires you to (a) accept Meta's community license at the model page on huggingface.co and (b) export an `HF_TOKEN` environment variable with read scope. Qwen (Apache-2.0) and Phi (MIT) repos are fully open and need no token. If `HF_TOKEN` isn't set, the daemon's first lazy-load of a Llama model fails with a 401/403 — switch to a Qwen or Phi model in the picker, or set up the token (see [Hugging Face access token docs](https://huggingface.co/docs/hub/security-tokens)).

This is the **default provider** for VibeCody's privacy-preserving / no-API-key path. Inference is ~5× slower than Cerebras but every byte stays on your machine.

### Zhipu (`zhipu`)

Chinese provider; GLM family.

| Model | Notes |
|---|---|
| glm-4-plus | Flagship |
| glm-4-flash | Cheap/fast |

### Vercel AI Gateway (`vercel_ai`)

Gateway with no preset list — you point it at any backend Vercel AI supports. Empty model list in the registry; user supplies the model string.

### MiniMax (`minimax`)

Chinese provider.

| Model | Notes |
|---|---|
| abab6.5s-chat | General chat |

### SambaNova (`sambanova`)

Inference-only, similar shape to Cerebras — open weights on custom RDU hardware.

| Model | Notes |
|---|---|
| DeepSeek-V3.2 | Default |
| MiniMax-M3 | 1M context + multimodal |
| MiniMax-M2.7 | Previous MiniMax |
| gpt-oss-120b | OpenAI open-weights line |
| gemma-4-31B-it | Google open weights |
| Meta-Llama-3.3-70B-Instruct | SambaNova's most battle-tested model |

Expanded on 2026-09-02. The Llama row is not deprecated — SambaNova still calls
it its most battle-tested model — but it had been the only entry, which made the
picker look like a one-model provider.

---

## Open vs closed weights

| Closed weights only | Open weights (you can self-host) |
|---|---|
| Claude (Anthropic) | Llama family (Meta) |
| GPT (OpenAI) | Mistral family (incl. Devstral, Codestral, Ministral) |
| Gemini (Google) | Gemma (Google) |
| Grok (xAI) | Qwen (Alibaba) |
| Sonar (Perplexity) | DeepSeek (R1, V3, V4 family) |
|  | Phi (Microsoft) |
|  | Nemotron (NVIDIA) |
|  | GLM (Zhipu) |
|  | Kimi (Moonshot) |
|  | gpt-oss (OpenAI's open-weights line) |

If your project needs to **run inference offline** or **prove no data left the machine**, only the open-weights column is viable — through Ollama (cloud or local) or the in-daemon mistralrs backend.

---

## Model lifecycle policy

Models in this picker are not equally durable. Open-weights models on HuggingFace, closed flagships behind a paid API, and inference-only marketplaces all age differently. Plan for it.

### Two clocks: supply vs quality

Every model has two deprecation timelines:

- **Supply clock** — *will the model still be available?* For open weights from Meta, Microsoft, Mistral, Alibaba, Google, etc., the answer is essentially "forever." First-party releases from major labs are not yanked from HuggingFace. Closed APIs (`gpt-3.5-turbo`, older Claude versions) *do* get sunset on published timelines — typically 6-18 months notice.
- **Quality clock** — *will the model still be the right pick?* This runs much faster. Small-model tier sees a new generation every 6-12 months: Llama-3.2 → 3.3 → 4, Phi-3.5 → 4 → 4-mini, Qwen-2.5 → 3 → 3.5. The previous version still works; it's just no longer competitive.

In practice: **expect every model in this doc to be obsolete within 18 months, but expect open-weights models to keep working for as long as you have local copies.**

### Cached-weights floor

When mistralrs first uses a model, weights download once into `~/.cache/huggingface/hub`. From that point forward, the model keeps working *even if HuggingFace removed the upstream tomorrow*. Same applies to Ollama's local pulls (`~/.ollama/models/`). Cloud APIs have no equivalent floor — when Anthropic retired `claude-3-5-sonnet-20241022` on 2025-10-28, every client lost access on the same day. Ollama Cloud does the same: four of the models this doc previously listed went `410 Gone` between 2026-06-16 and 2026-07-15.

Practical implication: if reproducibility matters (audit trail, regulated environment), cache open-weights models on disk and avoid relying on closed APIs for the part of the pipeline that must reproduce identically.

### Risk table

| Risk | Likelihood | What breaks | Mitigation |
|---|---|---|---|
| Cloud API sunsets a model | High (planned, ~yearly) | Cloud-API jobs using that model | Track provider deprecation pages; fail over to a sibling model |
| Open-weights repo renamed on HF | Low | First-time pulls; cached copies fine | Update the model id in `STATIC_MODELS` |
| Open-weights repo removed | Very low for first-party | First-time pulls; cached copies fine | Same as above; preserve cache backups |
| New generation released, old becomes "legacy" | Near-certain (6-12 mo) | Nothing breaks; competitive position erodes | Periodic registry refresh |
| HF gating policy tightens | Low-Med | New downloads of gated models fail | Switch to ungated alternative (Qwen/Phi) |
| License terms change | Low | Theoretical — already-released weights stay under their original license | Monitor license pages |
| mistralrs drops architecture support | Low (Llama, Phi, Qwen are tier-1) | Models can't load with the latest mistralrs | Pin mistralrs version; upgrade selectively |

### Hardening options for VibeCody

If you ship VibeCody to users who need reproducibility (enterprise, regulated, research), there are three knobs you can turn beyond the defaults:

1. **Pin commit SHAs in the registry.** mistralrs accepts HuggingFace revision specs — change `"meta-llama/Llama-3.1-8B-Instruct"` to `"meta-llama/Llama-3.1-8B-Instruct@<commit-sha>"` in `STATIC_MODELS`. This immunizes against silent re-uploads under the same tag. Cost: you have to manually bump the SHA when you want a newer revision.
2. **Add a `MODEL_REPLACEMENT_MAP`.** When a model 404s on pull, the daemon can log "this model has been retired; suggested replacement: X" and either fail fast or auto-substitute. Not implemented today; ~30 lines if you want it.
3. **Ship a snapshot mirror.** For closed environments without HuggingFace access, mirror the open-weights models you depend on into an internal artifact store (S3, Artifactory) and point `HF_ENDPOINT` at it. The daemon will pull from there.

None of these are urgent. They become useful when you start *depending* on a specific model staying frozen.

### What we update and when

The lists in this doc and in `vibecoder/src/hooks/useModelRegistry.ts` are refreshed on a roughly **quarterly** cadence — when a new flagship lands at one of the major providers, or when an existing model gets formally sunset. The "Last updated" date at the top of this page is authoritative; if it's more than 6 months old when you read this, treat the picks as historical and verify against the providers' current docs.

## How to set a different default

Per-provider default lives in `vibecoder/src/hooks/useModelRegistry.ts`:

```ts
export const PROVIDER_DEFAULT_MODEL: Record<string, string> = {
  claude:       "claude-opus-4-7",
  openai:       "gpt-5.5",
  gemini:       "gemini-3.1-pro",
  // ...
  ollama:       "devstral-2",     // ← change here
  // ...
};
```

To **add a new model** to a provider's picker, append to the array in `STATIC_MODELS` in the same file. (For Ollama, the array is sourced from `vibecoder/src/constants/ollamaModels.ts`.)

Per [CLAUDE.md](https://github.com/anthropics/claude-code), the model list is the only file you need to touch for a frontend-only change.

---

## See also

- [Providers overview]({{ site.baseurl }}/providers/) — per-provider setup and API key configuration.
- [Configuration]({{ site.baseurl }}/configuration/) — daemon and UI settings.
- [Failover]({{ site.baseurl }}/providers/failover/) — chain providers so one going down doesn't kill your session.
