---
layout: page
title: Model Comparison
permalink: /model-comparison/
---

# Model Comparison

> A practical guide to picking the right model for the job across every provider VibeCody supports.
> Last updated: **2026-05-01**.
>
> **Caveat**: model leaderboards shift weekly. Treat the strength/weakness blurbs as a *shape* of each model's bias (what it was trained for), not a final benchmark verdict. When in doubt, run the same prompt through two candidates side-by-side in VibeUI's MultiModel panel.

## Notation

- **Ctx** — maximum context window (input tokens).
- **Tools** — native function/tool calling support: ✅ first-class, ⚠️ supported but quirky, ❌ none.
- **Vision** — accepts image input.
- **Reasoning** — model does explicit chain-of-thought / "thinking" tokens before answering.
- **Open** — open-weights (you can self-host).

---

## Pick by task

The "right" pick depends on what you're doing. Use this matrix as a starting point, then verify with the MultiModel panel in VibeUI.

### Coding agent (multi-step file edits, run-and-fix loops)

| Tier | Cloud-hosted | Open-weights (Ollama Cloud) | Local pull |
|---|---|---|---|
| **Flagship** | Claude Sonnet 4.6 | devstral-2 (123B) | devstral-small-2 |
| **Strong** | GPT-4.1, gpt-5 (when avail.) | qwen3-coder | qwen2.5-coder:7b |
| **Cheap/fast** | Claude Haiku 4.5, gpt-4.1-mini | ministral-3, devstral-small-2 | qwen2.5-coder:1.5b |

### One-shot reasoning, math, hard algorithms

| Tier | Cloud-hosted | Open-weights | Local pull |
|---|---|---|---|
| **Flagship** | Claude Opus 4.6, o3 | nemotron-3-super, deepseek-v4-pro | deepseek-r1:14b |
| **Strong** | gpt-4.1, Gemini 2.5 Pro | glm-5.1, magistral | qwq:32b |
| **Cheap** | o4-mini, gpt-4.1-mini | nemotron-3-nano | phi4-reasoning |

### Long context (≥200k tokens)

| Tier | Provider · Model |
|---|---|
| **Flagship** | Gemini 2.5 Pro (1M+), Claude Sonnet 4.6 (200k) |
| **Strong** | gpt-4.1 (1M), Grok-3 (256k) |
| **Open** | qwen3-next, llama4 (variable) |

### Vision (image input)

| Tier | Provider · Model |
|---|---|
| **Flagship** | Claude Sonnet 4.6, GPT-4o, Gemini 2.5 Pro |
| **Strong** | Grok-3, gpt-4.1 |
| **Open** | qwen3-coder (vision variant), llama4 vision |
| **Local** | llama3.2-vision, gemma3 |

### Cheap & fast tool-calling agents

| Tier | Provider · Model |
|---|---|
| **Cloud** | Claude Haiku 4.5, gpt-4.1-mini, Gemini 2.5 Flash, Grok-3-mini |
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
| **With tools** | gpt-4.1 + browser tool, Claude Sonnet 4.6 + web tool |

---

## Providers and models

Below: every provider VibeCody ships, the models we expose in the picker, and what each one is actually good at. Flagships get deeper dives; secondary models get one-liners.

### Anthropic Claude (`claude`)

Three-tier family — Opus (deepest reasoning), Sonnet (balanced workhorse), Haiku (fast/cheap). All three support tool calling, vision, and extended thinking. Default in VibeCody is `claude-opus-4-6`.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| claude-opus-4-6 | 200k | ✅ | ✅ | ✅ | Flagship reasoning + agent default |
| claude-sonnet-4-6 | 200k | ✅ | ✅ | ✅ | Best-bang-for-buck coding agent |
| claude-haiku-4-5 | 200k | ✅ | ✅ | ✅ | Cheap/fast tool calls + classification |
| claude-sonnet-4-5 | 200k | ✅ | ✅ | ✅ | Previous-gen Sonnet |
| claude-3-5-sonnet-20241022 | 200k | ✅ | ✅ | ❌ | Legacy 3.5 — kept for reproducibility |

**claude-opus-4-6** — Strongest at sustained agentic loops with many tools and many turns. It rarely loses the plot on long sessions and is willing to push back on bad instructions. Most expensive of the three. Use when latency doesn't matter and the work is hard.

**claude-sonnet-4-6** — The model most VibeCody users will actually run. Roughly Opus-level coding quality on common tasks, ~3-4× cheaper, ~2× faster. Default for the VibeUI Code panel.

**claude-haiku-4-5** — Surprisingly capable for its tier; handles routine tool-calling, summarization, intent classification. Don't use it for novel architecture or deep debugging — it gets confidently wrong.

### `claude-code` (local Claude Code CLI passthrough)

Same three Anthropic models, but billed against the user's Claude.ai Pro/Max/Team/Enterprise plan instead of API credits. Same capabilities; payment shape differs.

### OpenAI (`openai`)

Two parallel lines: the **GPT line** (gpt-4o, gpt-4.1) is the general-purpose chat/agent family; the **o-line** (o3, o4-mini) is the explicit-reasoning family that thinks before answering.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| gpt-4o | 128k | ✅ | ✅ | ❌ | Workhorse multimodal, omni input/output |
| gpt-4o-mini | 128k | ✅ | ✅ | ❌ | Fast/cheap variant |
| gpt-4-turbo | 128k | ✅ | ✅ | ❌ | Older; kept for reproducibility |
| gpt-4.1 | 1M | ✅ | ✅ | ❌ | Long-context flagship |
| gpt-4.1-mini | 1M | ✅ | ✅ | ❌ | Fast long-context |
| gpt-4.1-nano | 1M | ✅ | ❌ | ❌ | Very cheap classification/extract |
| o3 | 200k | ✅ | ✅ | ✅ | Hard reasoning flagship |
| o3-mini | 200k | ✅ | ❌ | ✅ | Cheaper reasoning |
| o4-mini | 200k | ✅ | ✅ | ✅ | Reasoning + vision; replaces o3-mini for most use |

**gpt-4.1** — Drop-in successor to gpt-4o for most coding/agent tasks, plus a 1M-token context window that actually retrieves well (not just claimed). Tool calling is rock-solid. Use this when the codebase is large.

**o3** — When a problem rewards "thinking longer" — algorithm design, math proofs, debugging weird race conditions — o3 is the strongest in the OpenAI lineup. It's slow and expensive; not the right pick for chat or routine edits.

**o4-mini** — A reasonable middle ground when you want some explicit reasoning but not o3 cost. Good for code review and architecture sketches.

### Google Gemini (`gemini`)

Long context is the headline (1M+ on Pro). The 2.5 generation is competitive with GPT-4-class models on most general tasks and dominates anything where you want to feed it an entire codebase or a 200-page PDF.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| gemini-2.5-pro | 1M | ✅ | ✅ | ✅ | Long-context flagship |
| gemini-2.5-flash | 1M | ✅ | ✅ | ⚠️ | Cheap workhorse, default |
| gemini-2.0-flash | 1M | ✅ | ✅ | ❌ | Previous-gen flash |
| gemini-2.0-flash-lite | 1M | ✅ | ❌ | ❌ | Cheapest tier |

**gemini-2.5-pro** — When you need a model to actually *understand* a million-token context (not just accept it), Pro 2.5 is the strongest. Tool calling has improved substantially over 2.0; still occasionally hallucinates argument shapes for complex tools.

**gemini-2.5-flash** — VibeCody's default Gemini pick. Sub-second time-to-first-token, supports tools and vision, costs roughly 10× less than Pro. Good for chat-style use; use Pro when you need depth.

### xAI Grok (`grok`)

Strong on real-time / news-aware tasks (it has live X data feed integration on the back end). Decent coding ability; tool calling is solid as of grok-3.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| grok-3 | 256k | ✅ | ✅ | ⚠️ | Flagship |
| grok-3-mini | 256k | ✅ | ❌ | ❌ | Cheap/fast — VibeCody default |
| grok-2 | 128k | ✅ | ✅ | ❌ | Previous gen |

**grok-3** — Useful when the task involves recent events, market data, or code where the relevant docs were published in the last few months — it tends to be more current than rivals. Coding capability roughly between gpt-4o and gpt-4.1. Tool calling works but the JSON schema adherence is fussier than Claude's.

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
| deepseek-chat | 128k | ✅ | ❌ | ❌ | General workhorse |
| deepseek-reasoner | 128k | ✅ | ❌ | ✅ | R1-class reasoning |
| deepseek-coder | 128k | ✅ | ❌ | ❌ | Coding-tuned |

**deepseek-reasoner** — Strong at math and algorithmic reasoning; meaningfully cheaper than o3 for similar quality on bench tasks. Tool calling support is recent and a bit rough; verify your function schemas round-trip cleanly before relying on it for agent loops.

### Cerebras (`cerebras`)

Inference-only platform — does not train models, but runs Llama-class open weights at extreme speed (often 10-20× faster than typical cloud endpoints) on their wafer-scale hardware.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| llama-3.3-70b | 128k | ✅ | ❌ | ❌ | Best-quality option |
| llama-3.1-8b | 128k | ✅ | ❌ | ❌ | Tiny + extremely fast |

**llama-3.3-70b** on Cerebras — Use when you want Llama-3.3 quality with 1000+ tokens/sec generation. Great for streaming-heavy chat UIs and agent loops where round-trip count dominates. Tool calling works but the model itself is slightly weaker at strict JSON than GPT-4-class.

### Perplexity (`perplexity`)

Web-search-augmented chat. Models include browsing as a native step in their generation pipeline; you don't add a separate tool. Citations come back inline.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| sonar-pro | 200k | ⚠️ | ❌ | ❌ | Default. Web-grounded answers + citations |
| sonar | 128k | ⚠️ | ❌ | ❌ | Cheaper variant |
| sonar-reasoning | 128k | ⚠️ | ❌ | ✅ | Reasoning + web search |

Use Perplexity for "what's the latest on X" prompts where you need fresh sources. Don't use it for code generation or long agent loops — it isn't shaped for that.

### Together.ai (`together`)

Inference-only marketplace for open-weights models. We expose a couple of Llama / Mixtral defaults; Together hosts dozens more — extend STATIC_MODELS if you need them.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| meta-llama/Llama-3.3-70B-Instruct | 128k | ⚠️ | ❌ | ❌ | Workhorse open weights |
| mistralai/Mixtral-8x7B-Instruct-v0.1 | 32k | ⚠️ | ❌ | ❌ | Older but cheap MoE |

### Fireworks (`fireworks`)

Same shape as Together — inference-only, open-weights focus, similar Llama/Mixtral lineup.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| accounts/fireworks/models/llama-v3p3-70b-instruct | 128k | ⚠️ | ❌ | ❌ | Llama 3.3 default |
| accounts/fireworks/models/mixtral-8x7b-instruct | 32k | ⚠️ | ❌ | ❌ | Older Mixtral |

### OpenRouter (`openrouter`)

Aggregator front-end — one API key, hundreds of models routed to the cheapest/fastest available backend. Useful for quick experimentation across models, less ideal as a production primary because pricing and latency vary by route.

| Model | Ctx | Tools | Vision | Reasoning | Notes |
|---|---|---|---|---|---|
| anthropic/claude-3.5-sonnet | 200k | ✅ | ✅ | ❌ | Default in VibeCody |
| openai/gpt-4o | 128k | ✅ | ✅ | ❌ | OpenAI passthrough |
| google/gemini-2.0-flash-001 | 1M | ✅ | ✅ | ❌ | Cheap long context |

### Azure OpenAI (`azure_openai`)

Enterprise Azure-region-pinned OpenAI deployments. Same models as `openai` but billed via Azure with regional / compliance guarantees.

| Model | Notes |
|---|---|
| gpt-4o | Standard 4o on Azure |
| gpt-4-turbo | Older; kept for compliance reproducibility |

### Amazon Bedrock (`bedrock`)

AWS-region-pinned Anthropic Claude (and others). Same models, AWS billing, IAM-gated.

| Model | Notes |
|---|---|
| anthropic.claude-3-5-sonnet-20241022-v2:0 | Sonnet 3.5 on Bedrock |
| anthropic.claude-3-haiku-20240307-v1:0 | Haiku 3 on Bedrock |

### GitHub Copilot (`copilot`)

Copilot's chat back-end uses gpt-4o-class models. We expose it as a provider for users on Copilot Business/Enterprise who want to channel chat through that quota.

| Model | Notes |
|---|---|
| gpt-4o | Routed via the Copilot endpoint |

### Ollama (`ollama`)

The most-used provider in VibeCody. `ollama` covers both **local-pulled** models (run on your machine) and **cloud-hosted** models (run on ollama.com when an API key is configured). The full library list lives in `vibeui/src/constants/ollamaModels.ts`.

VibeCody's default Ollama model is **`devstral-2`** — Mistral's 123B coding-agent flagship, non-Chinese origin, native tool calling.

#### Cloud-hosted flagships (non-Chinese)

| Model | Origin | Best for | Notes |
|---|---|---|---|
| **devstral-2** | Mistral / France | **Coding agents** | 123B MoE, default. Tool calling native. |
| devstral-small-2 | Mistral / France | Cheaper coding | Smaller variant of devstral-2 |
| nemotron-3-super | NVIDIA / US | Reasoning | Llama-derived, RL-tuned for math/code reasoning |
| nemotron-3-nano | NVIDIA / US | Cheap reasoning | Smaller nemotron |
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
| glm-5, glm-5.1 | Zhipu | Strong agent eval scores |
| kimi-k2.5, kimi-k2.6 | Moonshot | 1T MoE; long context |
| minimax-m2.5, minimax-m2.7 | MiniMax | Agentic/reasoning hybrid |

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

| Model | Ctx | Best for | Notes |
|---|---|---|---|
| Qwen/Qwen2.5-7B-Instruct | 32k | Privacy-default chat | Good general model, ~7B |
| Qwen/Qwen2.5-Coder-7B-Instruct | 32k | Privacy-default coding | Coding-tuned |
| Qwen/Qwen2.5-Coder-1.5B-Instruct | 32k | Edge / fast | Tiny but viable for completion |
| Qwen/Qwen2.5-3B-Instruct | 32k | Mobile-class chat | |
| meta-llama/Llama-3.2-3B-Instruct | 128k | General small | |
| meta-llama/Llama-3.2-1B-Instruct | 128k | Tiniest viable | |
| microsoft/Phi-3.5-mini-instruct | 128k | Smart-but-small | Strong reasoning per parameter |

This is the **default provider** for VibeCody's privacy-preserving / no-API-key path. It's ~5× slower than Cerebras but every byte stays on your machine.

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

Inference-only, similar shape to Cerebras (fast Llama runs).

| Model | Notes |
|---|---|
| Meta-Llama-3.3-70B-Instruct | Default |

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

## How to set a different default

Per-provider default lives in `vibeui/src/hooks/useModelRegistry.ts`:

```ts
export const PROVIDER_DEFAULT_MODEL: Record<string, string> = {
  claude:       "claude-opus-4-6",
  openai:       "gpt-4o",
  // ...
  ollama:       "devstral-2",     // ← change here
  // ...
};
```

To **add a new model** to a provider's picker, append to the array in `STATIC_MODELS` in the same file. (For Ollama, the array is sourced from `vibeui/src/constants/ollamaModels.ts`.)

Per [CLAUDE.md](https://github.com/anthropics/claude-code), the model list is the only file you need to touch for a frontend-only change.

---

## See also

- [Providers overview](/providers/) — per-provider setup and API key configuration.
- [Configuration](/configuration/) — daemon and UI settings.
- [Failover](/providers/failover/) — chain providers so one going down doesn't kill your session.
