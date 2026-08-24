#!/usr/bin/env python3
"""Third pass: provider lists and the last stale counts.

Provider inventory is taken from vibecoder/src/hooks/useModelRegistry.ts
(STATIC_MODELS / PROVIDER_DEFAULT_MODEL, 25 keys, identical sets) cross-checked
against vibecoder/crates/vibe-ai/src/providers/*.rs (26 files = 23 concrete
backends + failover meta-provider + compat.rs and openai_compat.rs helpers).
"""
import os
import sys

R = []


def fix(path, old, new):
    R.append((path, old, new))


PROVIDER_LIST = """- **Ollama** — local and Ollama Cloud models (no API key for local pulls)
- **vibecli-mistralrs** — in-process local inference, no server; weights cached under `~/.cache/huggingface/hub`
- **vLLM** — self-hosted OpenAI-compatible endpoint
- **LM Studio** — local desktop model server
- **Anthropic Claude** — Claude Opus 5, Fable 5, Sonnet 5, Opus 4.x, Haiku 4.5
- **Claude Code** — routes through the local Claude Code CLI (Free/Pro/Max/Team/Enterprise plans, no API credits)
- **OpenAI** — GPT-5.6 Sol/Terra/Luna, GPT-5.5, GPT-5.3-Codex, GPT-4.1, GPT-4o
- **Google Gemini** — Gemini 3.6 Flash, 3.5 Flash / Flash-Lite
- **xAI Grok** — Grok 4.5, 4.3, 4.20
- **Groq** — fast inference (gpt-oss, Qwen)
- **OpenRouter** — multi-provider gateway (Kimi K3, and 300+ models)
- **Azure OpenAI** — enterprise Azure-hosted models
- **AWS Bedrock** — AWS-hosted Claude, Llama, Titan
- **GitHub Copilot** — Copilot integration
- **Mistral** — Mistral Large / Medium / Small
- **Cerebras** — wafer-scale inference (gpt-oss-120b, Gemma 4, GLM 4.7)
- **DeepSeek** — DeepSeek V4 Pro / Flash
- **Zhipu** — GLM-5.2 / 5.1 / 5
- **Vercel AI** — Vercel AI SDK gateway
- **MiniMax** — MiniMax-M3 / M2.7
- **Perplexity** — search-augmented Sonar models
- **Together AI** — open model hosting (Kimi, Qwen)
- **Fireworks AI** — fast open model inference
- **SambaNova** — hardware-accelerated inference
- **Poolside** — Laguna models

Two more live in the crate but not in the model picker:

- **LocalEdit** — local code-editing model backend
- **Failover** — meta-provider that chains backends and retries the next on timeout, rate-limit, or error"""

fix("README.md",
    "Unified AI provider abstraction with agent loop, hooks, planner, multi-agent orchestration, "
    "skills, artifacts, admin policy, trace/session resume, and OpenTelemetry. Supports 22 providers "
    "(plus a shared `openai_compat` helper module):\n\n"
    "- **Ollama** — Local/private models (default)\n"
    "- **Anthropic Claude** — Claude 4 Sonnet/Opus\n"
    "- **OpenAI** — GPT-4o and variants\n"
    "- **Google Gemini** — Gemini 2.5 Pro/Flash\n"
    "- **xAI Grok** — Grok 2\n"
    "- **Groq** — Fast inference (Llama, Mixtral)\n"
    "- **OpenRouter** — Multi-provider gateway\n"
    "- **Azure OpenAI** — Enterprise Azure-hosted models\n"
    "- **AWS Bedrock** — AWS-hosted models (Claude, Llama, Titan)\n"
    "- **GitHub Copilot** — Copilot integration\n"
    "- **LocalEdit** — Local code editing model\n"
    "- **Mistral** — Mistral AI models\n"
    "- **Cerebras** — Wafer-scale inference\n"
    "- **DeepSeek** — DeepSeek V3/R1\n"
    "- **Zhipu** — GLM-4 models\n"
    "- **Vercel AI** — Vercel AI SDK\n"
    "- **MiniMax** — MiniMax-Text-01\n"
    "- **Perplexity** — Search-augmented Sonar models\n"
    "- **Together AI** — Open model hosting (Llama, Qwen)\n"
    "- **Fireworks AI** — Fast open model inference\n"
    "- **SambaNova** — Hardware-accelerated inference\n"
    "- **Failover** — Auto-failover wrapper (chains multiple providers)",

    "Unified AI provider abstraction with agent loop, hooks, planner, multi-agent orchestration, "
    "skills, artifacts, admin policy, trace/session resume, and OpenTelemetry.\n\n"
    "**25 providers are selectable in the UI.** The canonical list is "
    "`vibecoder/src/hooks/useModelRegistry.ts` — `STATIC_MODELS` and `PROVIDER_DEFAULT_MODEL` "
    "must stay in sync, and nothing else needs to change to add one. The crate underneath "
    "(`vibe-ai/src/providers/`) holds 23 concrete backends plus a failover meta-provider and two "
    "shared `*compat` helper modules.\n\n" + PROVIDER_LIST)

fix("docs/faq.md",
    "VibeCody supports 25 AI providers: Ollama, Claude, OpenAI, Gemini, Grok, Groq, OpenRouter, "
    "Azure OpenAI, Bedrock, Copilot, LocalEdit, Mistral, Cerebras, DeepSeek, Zhipu, Vercel AI, "
    "MiniMax, Perplexity, Together AI, Fireworks AI, SambaNova, plus the FailoverProvider meta-provider.",

    "VibeCody offers 25 selectable AI providers: Ollama, vibecli-mistralrs, vLLM, LM Studio, "
    "Claude, Claude Code, OpenAI, Gemini, Grok, Groq, OpenRouter, Azure OpenAI, Bedrock, Copilot, "
    "Mistral, Cerebras, DeepSeek, Zhipu, Vercel AI, MiniMax, Perplexity, Together AI, "
    "Fireworks AI, SambaNova, and Poolside. Two more exist in the crate but are not in the model "
    "picker: LocalEdit, and the FailoverProvider meta-provider that chains backends and retries "
    "the next one on error.")

fix("docs/quickstart.md",
    "**VibeCoder** (desktop editor, 293+ panels)",
    "**VibeCoder** (desktop editor, 246 panels)")

fix("docs/quickstart.md",
    "supporting 22 AI providers, an autonomous agent loop",
    "supporting 25 AI providers, an autonomous agent loop")

fix("docs/llms.txt", "Unit Tests: 10,535 (0 failures)", "Test functions: 16,102")
fix("docs/llms-full.txt", "Unit Tests: 9,570 (0 failures)", "Test functions: 16,102")

# The 17-provider list in the root llms.txt
fix("llms.txt",
    "25 providers: Ollama, Claude, OpenAI, Gemini, Grok, Groq, OpenRouter, Azure OpenAI, Bedrock, "
    "Copilot, Mistral, Cerebras, DeepSeek, Zhipu, Vercel AI, L",
    "25 providers: Ollama, vibecli-mistralrs, vLLM, LM Studio, Claude, Claude Code, OpenAI, "
    "Gemini, Grok, Groq, OpenRouter, Azure OpenAI, Bedrock, Copilot, Mistral, Cerebras, DeepSeek, "
    "Zhipu, Vercel AI, L")

# Source doc-comment that seeded the wrong Retry description in agent-panel.md
fix("vibecoder/src/components/AgentPanel.tsx",
    "/** Retry after error — preserves completed steps and work. */",
    "/** Retry after error — re-runs the task from the start (no checkpoint); the step feed stays on screen. */")


def main():
    root = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else ".")
    os.chdir(root)
    cache, applied, missed = {}, 0, []
    for path, old, new in R:
        if path not in cache:
            cache[path] = open(path, encoding="utf-8", errors="ignore").read() if os.path.exists(path) else None
        s = cache[path]
        if s is None:
            missed.append((path, "FILE MISSING"))
            continue
        if old not in s:
            missed.append((path, old.split("\n")[0][:90]))
            continue
        cache[path] = s.replace(old, new, 1)
        applied += 1
    written = 0
    for p, s in cache.items():
        if s is None:
            continue
        if open(p, encoding="utf-8", errors="ignore").read() != s:
            open(p, "w", encoding="utf-8").write(s)
            written += 1
    print(f"fixes applied : {applied}/{len(R)}")
    print(f"files written : {written}")
    for m in missed:
        print("  MISS", m[0], "|", m[1])


if __name__ == "__main__":
    main()
