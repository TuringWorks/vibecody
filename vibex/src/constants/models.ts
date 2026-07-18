import type { DaemonModel } from "../hooks/useModels";

/**
 * Static model catalog — the fallback VibeX shows when the daemon is offline
 * (or returns nothing), and the source of the ollama Cloud + full-catalog rows
 * that a local `/api/tags` never reports. Mirrors the desktop app's registry
 * (`vibeui/src/hooks/useModelRegistry.ts` + `constants/ollamaModels.ts`).
 *
 * Provider ids match VibeX's `KEYED_PROVIDERS` (see `hooks/useProviderSettings.ts`)
 * so the grouped picker lines up with the provider selector.
 *
 * When the daemon IS online we UNION its live list with this so the real,
 * installed models (e.g. ollama's pulled models) still take precedence while
 * cloud/catalog entries — which a local runtime never advertises — remain
 * selectable.
 */

/** Ollama Cloud / Turbo — datacenter-hosted, addressed by the `*-cloud` suffix. */
export const OLLAMA_CLOUD_MODELS: string[] = [
  "glm-5.2:cloud",
  "deepseek-v3.1:671b-cloud",
  "kimi-k2:1t-cloud",
  "gpt-oss:120b-cloud",
  "gpt-oss:20b-cloud",
  "glm-4.6:cloud",
  "minimax-m2:cloud",
];

/** Ollama chat catalog (cloud rows first, then pull-able local models). */
export const OLLAMA_CHAT_MODELS: string[] = [
  ...OLLAMA_CLOUD_MODELS,
  // Cloud-hosted flagship coding/agentic
  "devstral-2",
  "devstral-small-2",
  "nemotron-3-super",
  "nemotron-3-nano",
  "cogito-2.1",
  "gemma4",
  "ministral-3",
  // Latest / flagship
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
  "phi4-mini",
  "mistral-large-3",
  "mistral-small3.2",
  // Reasoning / agentic
  "glm-5.1",
  "glm-5",
  "glm-4.7",
  "codellama",
  "codegemma",
  "starcoder2",
];

/** Known models per provider — provider ids match `KEYED_PROVIDERS`. */
export const STATIC_MODELS: Record<string, string[]> = {
  ollama: OLLAMA_CHAT_MODELS,
  anthropic: [
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
    "claude-sonnet-4-5",
  ],
  openai: [
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.3-codex",
    "gpt-5",
    "gpt-4o",
    "gpt-4o-mini",
    "o4-mini",
    "o3",
    "gpt-4.1",
    "gpt-4.1-mini",
  ],
  gemini: [
    "gemini-3.5-pro",
    "gemini-3.5-flash",
    "gemini-3.1-pro",
    "gemini-3-pro",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
  ],
  groq: ["llama-3.3-70b-versatile", "llama-3.1-8b-instant", "mixtral-8x7b-32768", "gemma2-9b-it"],
  grok: ["grok-3", "grok-3-mini", "grok-2"],
  mistral: ["mistral-large-latest", "mistral-medium-latest", "mistral-small-latest", "codestral-latest"],
  deepseek: ["deepseek-v4", "deepseek-v4-flash", "deepseek-chat", "deepseek-reasoner", "deepseek-coder"],
  cerebras: ["llama-3.3-70b", "llama-3.1-8b"],
  perplexity: ["sonar-pro", "sonar", "sonar-reasoning"],
  together: ["meta-llama/Llama-3.3-70B-Instruct", "mistralai/Mixtral-8x7B-Instruct-v0.1"],
  fireworks: [
    "accounts/fireworks/models/llama-v3p3-70b-instruct",
    "accounts/fireworks/models/mixtral-8x7b-instruct",
  ],
  openrouter: [
    "moonshotai/kimi-k2.7-code",
    "z-ai/glm-5.2",
    "qwen/qwen3.6-coder",
    "deepseek/deepseek-v4",
    "anthropic/claude-3.5-sonnet",
    "openai/gpt-4o",
  ],
  azure_openai: ["gpt-4o", "gpt-4-turbo"],
  zhipu: ["glm-5.2", "glm-5.1", "glm-4-plus", "glm-4-flash"],
  minimax: ["MiniMax-M3", "abab6.5s-chat"],
  sambanova: ["Meta-Llama-3.3-70B-Instruct"],
};

/** Flattened `DaemonModel[]` for the picker (same shape the daemon returns). */
export const FALLBACK_MODELS: DaemonModel[] = Object.entries(STATIC_MODELS).flatMap(
  ([provider, names]) =>
    names.map((name) => ({ id: `${provider}/${name}`, name, provider }) satisfies DaemonModel),
);
