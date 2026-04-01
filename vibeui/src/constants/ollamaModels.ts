/**
 * Ollama chat models — static fallback list for when Ollama API is unreachable.
 *
 * Source: https://ollama.com/library?sort=newest
 * Last updated: 2026-03-31
 *
 * Only chat / general-purpose models are included here.
 * Embedding models, vision-only models, and OCR models are excluded.
 * To refresh: visit the URL above and add new chat model IDs.
 */
export const OLLAMA_CHAT_MODELS: string[] = [
  // ── Latest / flagship ──────────────────────────────────────────────
  "qwen3-coder",
  "qwen3.5",
  "qwen3",
  "qwen3-next",
  "qwen3-coder-next",
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
  "gemini-3-flash-preview",

  // ── Strong reasoning / agentic ─────────────────────────────────────
  "glm-5",
  "glm-4.7",
  "glm-4.7-flash",
  "glm-4.6",
  "kimi-k2.5",
  "kimi-k2",
  "kimi-k2-thinking",
  "cogito-2.1",
  "cogito",
  "magistral",
  "exaone-deep",
  "command-a",

  // ── NVIDIA Nemotron ────────────────────────────────────────────────
  "nemotron-cascade-2",
  "nemotron-3-super",
  "nemotron-3-nano",
  "nemotron",
  "nemotron-mini",

  // ── MiniMax ────────────────────────────────────────────────────────
  "minimax-m2.7",
  "minimax-m2.5",
  "minimax-m2.1",
  "minimax-m2",

  // ── Coding-focused ─────────────────────────────────────────────────
  "devstral-2",
  "devstral-small-2",
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
  "rnj-1",
  "ministral-3",
  "falcon3",
  "exaone3.5",
  "smollm2",

  // ── Community / fine-tuned ─────────────────────────────────────────
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
