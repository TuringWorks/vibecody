/**
 * Ollama chat models — static fallback list for when Ollama API is unreachable.
 *
 * Source: https://ollama.com/library?sort=newest  (and ?c=cloud for cloud-hosted)
 * Last updated: 2026-05-01
 *
 * Only chat / general-purpose models are included here.
 * Embedding models, vision-only models, and OCR models are excluded.
 * To refresh: visit the URLs above and add new chat model IDs.
 */

/**
 * Ollama Cloud / Turbo models — datacenter-hosted, addressed by the `*-cloud`
 * suffix. Selecting one routes the request to ollama.com instead of the local
 * runtime: the backend keeps the Bearer for any model whose name contains
 * "cloud", even on a loopback endpoint (see `OllamaProvider::new`). These are
 * never reported by a local `/api/tags`, so they're listed statically here.
 *
 * Requires an Ollama Cloud / Turbo token (Settings → Providers → "Ollama Cloud /
 * Turbo"); without one, selecting these will fail at request time.
 *
 * Source: https://ollama.com/library?c=cloud   ·  Last updated: 2026-06-06
 */
export const OLLAMA_CLOUD_MODELS: string[] = [
  "glm-5.2:cloud",   // Qwen · 480B · coding-agent flagship
  "deepseek-v3.1:671b-cloud", // DeepSeek · 671B · reasoning + tools
  "kimi-k2:1t-cloud",         // Moonshot · 1T MoE
  "gpt-oss:120b-cloud",       // OpenAI OSS · 120B
  "gpt-oss:20b-cloud",        // OpenAI OSS · 20B · faster
  "glm-4.6:cloud",            // Zhipu
  "minimax-m2:cloud",         // MiniMax
];

export const OLLAMA_CHAT_MODELS: string[] = [
  // ── Ollama Cloud / Turbo (datacenter-hosted, *-cloud, needs token) ──
  ...OLLAMA_CLOUD_MODELS,

  // ── Cloud-hosted flagship · non-Chinese · tool-calling ─────────────
  // These run on Ollama Cloud (no local pull needed when an API key is
  // configured). Strong on coding, agentic loops, and `tools` JSON mode.
  "devstral-2",          // Mistral · 123B · coding-agent flagship (default)
  "devstral-small-2",    // Mistral · smaller, faster
  "nemotron-3-super",    // NVIDIA · reasoning + tools
  "nemotron-3-nano",     // NVIDIA · smaller
  "cogito-2.1",          // DeepCogito · hybrid reasoning
  "gemma4",              // Google
  "ministral-3",         // Mistral · small
  "rnj-1",
  "gemini-3-flash-preview",

  // ── Latest / flagship (mixed origin) ───────────────────────────────
  "qwen3-coder",
  "qwen3.6",
  "qwen3.5",
  "qwen3",
  "qwen3-next",
  "qwen3-coder-next",
  "deepseek-v4-pro",
  "deepseek-v4-flash",
  "deepseek-v3.2",
  "deepseek-v3",
  "deepseek-r1",
  "llama4",
  "llama3.3",
  "llama3.2",
  "gemma3",
  "gemma3n",
  "phi4",
  "phi4-reasoning",
  "phi4-mini-reasoning",
  "phi4-mini",
  "mistral-large-3",
  "mistral-small3.2",
  "mistral-small3.1",

  // ── Strong reasoning / agentic ─────────────────────────────────────
  "glm-5.1",
  "glm-5",
  "glm-4.7",
  "glm-4.7-flash",
  "glm-4.6",
  "kimi-k2.6",
  "kimi-k2.5",
  "kimi-k2",
  "kimi-k2-thinking",
  "cogito",
  "magistral",
  "exaone-deep",
  "command-a",

  // ── NVIDIA Nemotron ────────────────────────────────────────────────
  "nemotron-cascade-2",
  "nemotron",
  "nemotron-mini",

  // ── MiniMax ────────────────────────────────────────────────────────
  "minimax-m2.7",
  "minimax-m2.5",
  "minimax-m2.1",
  "minimax-m2",

  // ── Coding-focused ─────────────────────────────────────────────────
  "devstral",
  "deepcoder",
  "codestral",
  "qwen2.5-coder",
  "deepseek-coder-v2",
  "deepseek-coder",
  "codellama",
  "starcoder2",

  // ── Mid-size / efficient ───────────────────────────────────────────
  "lfm2",
  "lfm2.5-thinking",
  "granite4",
  "granite3.1-dense",
  "granite3.1-moe",
  "olmo-3.1",
  "olmo-3",
  "olmo2",
  "falcon3",
  "exaone3.5",
  "smollm2",

  // ── Community / fine-tuned ─────────────────────────────────────────
  "gpt-oss-safeguard",
  "r1-1776",
  "dolphin3",
  "hermes3",
  "command-r-plus",
  "command-r",
  "command-r7b",
  "command-r7b-arabic",
  "qwq",
  "openthinker",
  "deepscaler",
  "smallthinker",
  "sailor2",

  // ── Older but widely used ──────────────────────────────────────────
  "mistral-nemo",
  "mistral-small",
  "mistral-large",
  "mistral",
  "mixtral",
  "qwen2.5",
  "deepseek-v2.5",
  "llama3.1",
  "llama3",
  "phi3.5",
  "phi3",
  "gemma2",
  "solar-pro",
  "wizardlm2",
  "nous-hermes2",
  "zephyr",
  "openchat",
  "vicuna",
  "llama2",
];
