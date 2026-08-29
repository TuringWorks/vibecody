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
 * Every tag below is verified against a live `POST /api/show` — Ollama Cloud
 * returns `410 Gone` for a retired model (with its retirement date) and `404`
 * for a tag that never existed, so this list is checked, not transcribed.
 * Re-verify the same way when refreshing; the listing page shows base names
 * without their `:cloud` / `:<size>-cloud` suffix, so the suffix has to be
 * probed rather than assumed.
 *
 * Retired and removed on 2026-08-05: `glm-4.6` (2026-06-16), `kimi-k2:1t`
 * (2026-06-16), `minimax-m2` (2026-06-16), `deepseek-v3.1:671b` (2026-07-15).
 *
 * Re-verified 2026-08-29: all seventeen existing tags still answer `200`,
 * so nothing was retired this round. Added `glm-5.3` and `glm-5.3-flash` —
 * the two entries on the listing page missing here. Both are cloud-only
 * (`404` without the `:cloud` suffix).
 *
 * Source: https://ollama.com/search?c=cloud   ·  Last verified: 2026-08-29
 */
export const OLLAMA_CLOUD_MODELS: string[] = [
  "glm-5.3:cloud",              // Z.ai · 753B MoE · 1M ctx · thinking + tools
  "glm-5.3-flash:cloud", // temporarily removed for the drift check
  "glm-5.2:cloud",              // Z.ai · previous coding-agent flagship
  "glm-5.1:cloud",              // Z.ai · previous flagship
  "kimi-k3:cloud",              // Moonshot · 3T-class
  "kimi-k2.7-code:cloud",       // Moonshot · coding
  "kimi-k2.6:cloud",            // Moonshot
  "deepseek-v4-pro:cloud",      // DeepSeek · reasoning + tools
  "deepseek-v4-flash:cloud",    // DeepSeek · faster
  "qwen3.5:cloud",              // Qwen · 397B A17B
  "minimax-m3:cloud",           // MiniMax · 1M context, multimodal
  "minimax-m2.7:cloud",         // MiniMax
  "nemotron-3-ultra:cloud",     // NVIDIA · 550B A55B
  "nemotron-3-super:cloud",     // NVIDIA · 120B A12B
  "nemotron-3-nano:30b-cloud",  // NVIDIA · 30B A3B
  "mistral-large-3:675b-cloud", // Mistral
  "gemma4:cloud",               // Google
  "gpt-oss:120b-cloud",         // OpenAI OSS · 120B
  "gpt-oss:20b-cloud",          // OpenAI OSS · 20B · faster
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
