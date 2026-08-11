/**
 * useModelRegistry — Shared cached provider→model matrix.
 *
 * Caches the provider/model list in localStorage with a 2-hour TTL.
 * All panels that need model selection import this hook to get
 * consistent, fast model dropdowns without redundant API calls.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OLLAMA_CHAT_MODELS, OLLAMA_CLOUD_MODELS } from "../constants/ollamaModels";

/**
 * Versioned: bump when the static catalog / merge logic changes so a stale
 * cache (e.g. one saved before Ollama Cloud models were added) is discarded on
 * load instead of lingering until the TTL expires and hiding the newer models.
 *
 * Exported so tests seed and assert against the *same* constant. When the key
 * was duplicated as a literal in the test file, the `:v2` bump silently left
 * the tests writing to a key nothing reads — they failed without anything
 * being wrong with the hook.
 */
// v3 (2026-08-05): retired-model sweep — a v2 cache still holds
// deepseek-v3.1:671b-cloud and friends, which now 410.
export const CACHE_KEY = "vibecody:model-registry:v3";
const CACHE_TTL_MS = 2 * 60 * 60 * 1000; // 2 hours

/**
 * Single source of truth for providers and models.
 *
 * To add a provider: add an entry to STATIC_MODELS and PROVIDER_DEFAULT_MODEL.
 * To add/update models for a provider: edit the array in STATIC_MODELS.
 * All panels consume this via useModelRegistry() — no other file needs changing.
 */

/** Known models per provider (static fallback when API unavailable) */
export const STATIC_MODELS: Record<string, string[]> = {
  // claude-code uses the local Claude Code CLI — works with Free, Pro, Max, Team, and Enterprise plans
  // without consuming Anthropic API credits.
  //
  // Fable 5 restored 2026-08-10. The 2026-06-12 US export-control directive that
  // suspended it was lifted on 06-30, and Fable 5 returned *globally* on 07-01
  // after 19 days — the comment that used to sit here claiming it was "not a
  // routable production option" was 40 days stale.
  //
  // Mythos 5 stays omitted, and for a different reason than before: it was
  // restored only to approved *US organisations*, permanently. A flat string[]
  // cannot say "available to some callers", so listing it would 403 for most
  // users. It waits on per-model availability metadata.
  "claude-code": ["claude-opus-5", "claude-fable-5", "claude-sonnet-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"],
  // claude-3-5-sonnet-20241022 removed 2026-08-05 — retired 2025-10-28 (404s).
  claude: ["claude-opus-5", "claude-fable-5", "claude-sonnet-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5", "claude-sonnet-4-5"],
  openai: ["gpt-5.6-sol-pro", "gpt-5.6-sol", "gpt-5.6-terra-pro", "gpt-5.6-terra", "gpt-5.6-luna-pro", "gpt-5.6-luna", "gpt-5.5-pro", "gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.3-codex", "gpt-5.3-chat", "gpt-5", "gpt-4.1", "gpt-4.1-mini", "gpt-4o", "gpt-4o-mini"],
  // gemini-3.6-flash is the current workhorse (shipped 2026-07-21).
  //
  // gemini-3.5-pro removed 2026-08-10: it has NEVER GA'd. It was announced at
  // I/O on 2026-05-19, delayed three times, and as of August 2026 remains a
  // limited Vertex AI preview for selected enterprise customers — absent from
  // the consumer Gemini app and AI Studio. It was listed here (and defaulted
  // to) on the strength of a *projection* written during a June refresh, so
  // every user picking Gemini got a model id the API rejects on first call.
  //
  // RULE: only ship model ids that are shipped and callable today. A forecast
  // belongs in a planning note, never in the registry — the narrative can
  // absorb a wrong projection, the code cannot. Enforced by
  // `defaults are present in their provider list` in useModelRegistry.test.ts.
  gemini: ["gemini-3.6-flash", "gemini-3.5-flash", "gemini-3.5-flash-lite", "gemini-3.1-pro", "gemini-3-pro", "gemini-2.5-pro", "gemini-2.5-flash"],
  // llama-3.1-8b-instant / llama-3.3-70b-versatile were deprecated 2026-06-17 (Groq
  // points at gpt-oss-20b / gpt-oss-120b); mixtral-8x7b-32768 and gemma2-9b-it are gone.
  groq: ["openai/gpt-oss-120b", "openai/gpt-oss-20b", "qwen/qwen3.6-27b", "minimaxai/minimax-m2.7", "groq/compound", "groq/compound-mini"],
  grok: ["grok-4.5", "grok-4.3", "grok-4.20"],
  mistral: ["mistral-large-latest", "mistral-medium-latest", "mistral-small-latest", "codestral-latest"],
  // "deepseek-v4" was never an API id — the shipped pair is v4-pro / v4-flash.
  deepseek: ["deepseek-v4-pro", "deepseek-v4-flash", "deepseek-chat", "deepseek-reasoner"],
  cerebras: ["gpt-oss-120b", "gemma-4-31b", "zai-glm-4.7"],
  perplexity: ["sonar-pro", "sonar", "sonar-reasoning-pro", "sonar-deep-research"],
  together: ["moonshotai/Kimi-K2.7-Code", "Qwen/Qwen3.8-Max", "Qwen/Qwen3.5-397B-A17B", "deepseek-ai/DeepSeek-V4-Pro"],
  fireworks: ["accounts/fireworks/models/llama-v3p3-70b-instruct", "accounts/fireworks/models/mixtral-8x7b-instruct"],
  // OpenRouter doubles as the home for frontier open-weight models that have no dedicated
  // VibeCody provider key yet — notably Moonshot's Kimi K2.7 Code (2026-06-13), which cuts
  // thinking tokens ~30% vs K2.6. Add a first-class Moonshot provider (6-file dance) if usage warrants.
  // Verified against the live https://openrouter.ai/api/v1/models catalog on 2026-08-05.
  openrouter: ["moonshotai/kimi-k3", "moonshotai/kimi-k2.7-code", "moonshotai/kimi-k2.6", "z-ai/glm-5.2", "qwen/qwen3.8-max", "deepseek/deepseek-v4-pro", "minimax/minimax-m3", "x-ai/grok-4.5", "anthropic/claude-opus-5", "anthropic/claude-sonnet-5", "openai/gpt-5.6-sol", "google/gemini-3.6-flash"],
  azure_openai: ["gpt-4o", "gpt-4-turbo"],
  // Bedrock ids take an `anthropic.` prefix. The previous entries were both dead:
  // claude-3-5-sonnet retired 2025-10-28, claude-3-haiku retires 2026-04-19.
  bedrock: ["anthropic.claude-opus-5", "anthropic.claude-sonnet-5", "anthropic.claude-opus-4-8", "anthropic.claude-haiku-4-5"],
  copilot: ["gpt-4o"],
  ollama: OLLAMA_CHAT_MODELS,
  // vibecli-mistralrs talks to the local vibecli daemon (default :7878) and
  // pins the in-process mistralrs backend via X-VibeCLI-Backend. Models here
  // are repo ids that mistralrs can lazy-load from Hugging Face on first use.
  // NOTE: meta-llama/* repos are gated — first-time download requires the
  // user to accept Meta's community license on the model page and supply
  // an HF_TOKEN. Qwen and Phi repos are fully open.
  "vibecli-mistralrs": [
    "meta-llama/Llama-3.1-8B-Instruct",
    "meta-llama/Llama-3.2-3B-Instruct",
    "meta-llama/Llama-3.2-1B-Instruct",
    "Qwen/Qwen3.6-Coder-7B-Instruct",
    "Qwen/Qwen3.6-7B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "Qwen/Qwen2.5-7B-Instruct",
    "Qwen/Qwen2.5-Coder-1.5B-Instruct",
    "Qwen/Qwen2.5-3B-Instruct",
    "Qwen/Qwen2.5-1.5B-Instruct",
    "Qwen/Qwen2.5-0.5B-Instruct",
    "microsoft/Phi-3.5-mini-instruct",
  ],
  // glm-5.2 (Z.ai, 744B, 2026-06-13) leads the Artificial Analysis open-weight intelligence index.
  // glm-4-plus / glm-4-flash dropped 2026-08-05 — the whole GLM-4.x line below 4.7 is retired
  // (Ollama Cloud reports glm-4.6 retired 2026-06-16).
  zhipu: ["glm-5.2", "glm-5.1", "glm-5", "glm-4.7", "glm-4.7-flash"],
  vercel_ai: [],
  // MiniMax-M3 (2026-06-01): 1M-token context + native multimodality in one open-weight model.
  // abab6.5s-chat dropped 2026-08-05 — superseded by the M-series.
  minimax: ["MiniMax-M3", "MiniMax-M2.7"],
  sambanova: ["Meta-Llama-3.3-70B-Instruct"],
  // Poolside AI — purpose-built coding models (Poolside AI).
  poolside: ["poolside/laguna-s-2.1", "poolside/laguna-xs-2.1", "poolside/laguna-m-1"],
};

export const ALL_PROVIDERS = Object.keys(STATIC_MODELS);

/**
 * Preferred default provider for panels that need an initial selection.
 * Points at the embedded mistralrs backend served by the local VibeCLI daemon —
 * privacy-preserving, no API key required, and the strategic direction for
 * VibeCody inference. Panels that need vision or cloud-only capabilities
 * should pick a different default explicitly.
 */
export const DEFAULT_PROVIDER = "vibecli-mistralrs";

const DEFAULT_PROVIDER_CACHE_KEY = "vibecody:default-provider";
const DAEMON_HEALTH_URL = "http://localhost:7878/health";
/** Fallback when the embedded daemon is not reachable. */
const EMBEDDED_UNREACHABLE_FALLBACK = "ollama";

/**
 * Synchronous default-provider read. Returns the value cached by the last
 * `probeAndCacheDefaultProvider()` call, or `DEFAULT_PROVIDER` if nothing is
 * cached yet. Safe to call in `useState(...)` initializers and function
 * parameter defaults.
 */
export function getDefaultProvider(): string {
  try {
    return localStorage.getItem(DEFAULT_PROVIDER_CACHE_KEY) || DEFAULT_PROVIDER;
  } catch {
    return DEFAULT_PROVIDER;
  }
}

/**
 * Pings the VibeCLI daemon's `/health` endpoint and caches the result. The
 * cached value is only read on the NEXT app launch (by `getDefaultProvider`),
 * so this never races with user selections in the current session.
 *
 * Also reads `mistralrs_recommended_default` from the response and mutates
 * `PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"]` in place — the daemon swaps
 * the recommendation to an ungated Apache-2.0 model when `HF_TOKEN` is
 * absent, so the picker doesn't pre-select a model that would 401 on load.
 *
 * Call once on app mount.
 */
export async function probeAndCacheDefaultProvider(): Promise<void> {
  let resolved = EMBEDDED_UNREACHABLE_FALLBACK;
  try {
    const res = await fetch(DAEMON_HEALTH_URL, { signal: AbortSignal.timeout(800) });
    if (res.ok) {
      resolved = DEFAULT_PROVIDER;
      try {
        const body = await res.json() as { mistralrs_recommended_default?: string };
        const rec = body?.mistralrs_recommended_default;
        if (typeof rec === "string" && rec.length > 0 && rec !== PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"]) {
          PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"] = rec;
        }
      } catch {
        // body wasn't JSON or didn't include the field — keep the static default
      }
    }
  } catch {
    // daemon unreachable — keep fallback
  }
  try {
    localStorage.setItem(DEFAULT_PROVIDER_CACHE_KEY, resolved);
  } catch {
    // localStorage unavailable — silently skip
  }
}

/** Default model to pre-select when a provider is chosen in a dropdown. */
export const PROVIDER_DEFAULT_MODEL: Record<string, string> = {
  "claude-code": "claude-opus-5",
  claude:       "claude-opus-5",
  openai:       "gpt-5.6-sol",
  gemini:       "gemini-3.6-flash",
  groq:         "openai/gpt-oss-120b",
  grok:         "grok-4.5",
  mistral:      "mistral-large-latest",
  deepseek:     "deepseek-chat",
  cerebras:     "gpt-oss-120b",
  perplexity:   "sonar-pro",
  together:     "moonshotai/Kimi-K2.7-Code",
  fireworks:    "accounts/fireworks/models/llama-v3p3-70b-instruct",
  openrouter:   "anthropic/claude-opus-5",
  azure_openai: "gpt-4o",
  bedrock:      "anthropic.claude-opus-5",
  copilot:      "gpt-4o",
  ollama:       "devstral-2",
  "vibecli-mistralrs": "meta-llama/Llama-3.1-8B-Instruct",
  zhipu:        "glm-5.2",
  vercel_ai:    "",
  minimax:      "MiniMax-M3",
  sambanova:    "Meta-Llama-3.3-70B-Instruct",
  poolside:     "poolside/laguna-s-2.1",
};

export interface ModelInfo {
  id: string;
  name: string;
  provider: string;
}

export interface ModelRegistryData {
  providers: string[];
  models: Record<string, string[]>;
  updatedAt: number;
}

function loadCache(): ModelRegistryData | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const data: ModelRegistryData = JSON.parse(raw);
    if (Date.now() - data.updatedAt > CACHE_TTL_MS) return null;
    return data;
  } catch {
    return null;
  }
}

function saveCache(data: ModelRegistryData) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(data));
  } catch {
    // localStorage full — ignore
  }
}

/**
 * Hook that provides the cached provider→model matrix.
 *
 * Returns:
 * - `providers`: List of all provider names
 * - `modelsForProvider(provider)`: Models available for a given provider
 * - `loading`: Whether a refresh is in progress
 * - `refresh()`: Force refresh from backend
 * - `lastUpdated`: Timestamp of last cache update
 */
export function useModelRegistry() {
  const [data, setData] = useState<ModelRegistryData>(() => {
    const cached = loadCache();
    if (cached) return cached;
    return {
      providers: ALL_PROVIDERS,
      models: { ...STATIC_MODELS },
      updatedAt: 0,
    };
  });
  const [loading, setLoading] = useState(false);
  const refreshedRef = useRef(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      // Fetch Ollama models dynamically
      let ollamaModels: string[] = [];
      let ollamaReachable = false;
      try {
        const result = await invoke<string[]>("ollama_list_models");
        if (result && result.length > 0) {
          ollamaModels = result;
          ollamaReachable = true;
        }
      } catch {
        // Ollama not running — will fall back to OLLAMA_CHAT_MODELS
      }

      // Merge with static models.
      const models = { ...STATIC_MODELS };
      // When Ollama is reachable, show only cloud models + locally-pulled models.
      // The full static catalog (OLLAMA_CHAT_MODELS) includes many older/superseded
      // models that confuse the dropdown — only fall back to it when the daemon
      // is unreachable so users can still pick a model to pull.
      if (ollamaReachable) {
        const seen = new Set<string>();
        models.ollama = [
          ...OLLAMA_CLOUD_MODELS,
          ...ollamaModels,
        ].filter((m) => (seen.has(m) ? false : (seen.add(m), true)));
      }
      // If Ollama is unreachable, static fallback is already set via { ...STATIC_MODELS }.

      const newData: ModelRegistryData = {
        providers: ALL_PROVIDERS,
        models,
        updatedAt: Date.now(),
      };
      setData(newData);
      saveCache(newData);
    } catch {
      // Keep existing data on error
    }
    setLoading(false);
  }, []);

  // Auto-refresh on mount if cache is stale
  useEffect(() => {
    if (!refreshedRef.current) {
      refreshedRef.current = true;
      if (data.updatedAt === 0 || Date.now() - data.updatedAt > CACHE_TTL_MS) {
        refresh();
      }
    }
  }, [data.updatedAt, refresh]);

  const modelsForProvider = useCallback(
    (provider: string): string[] => {
      return data.models[provider] || [];
    },
    [data.models]
  );

  return {
    providers: data.providers,
    modelsForProvider,
    loading,
    refresh,
    lastUpdated: data.updatedAt,
  };
}
