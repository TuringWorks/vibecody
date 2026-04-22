/**
 * useModelRegistry — Shared cached provider→model matrix.
 *
 * Caches the provider/model list in localStorage with a 2-hour TTL.
 * All panels that need model selection import this hook to get
 * consistent, fast model dropdowns without redundant API calls.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OLLAMA_CHAT_MODELS } from "../constants/ollamaModels";

const CACHE_KEY = "vibecody:model-registry";
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
  "claude-code": ["claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"],
  claude: ["claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5", "claude-sonnet-4-5", "claude-3-5-sonnet-20241022"],
  openai: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o4-mini", "o3", "o3-mini", "gpt-4.1", "gpt-4.1-mini", "gpt-4.1-nano"],
  gemini: ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash", "gemini-2.0-flash-lite"],
  groq: ["llama-3.3-70b-versatile", "llama-3.1-8b-instant", "mixtral-8x7b-32768", "gemma2-9b-it"],
  grok: ["grok-3", "grok-3-mini", "grok-2"],
  mistral: ["mistral-large-latest", "mistral-medium-latest", "mistral-small-latest", "codestral-latest"],
  deepseek: ["deepseek-chat", "deepseek-reasoner", "deepseek-coder"],
  cerebras: ["llama-3.3-70b", "llama-3.1-8b"],
  perplexity: ["sonar-pro", "sonar", "sonar-reasoning"],
  together: ["meta-llama/Llama-3.3-70B-Instruct", "mistralai/Mixtral-8x7B-Instruct-v0.1"],
  fireworks: ["accounts/fireworks/models/llama-v3p3-70b-instruct", "accounts/fireworks/models/mixtral-8x7b-instruct"],
  openrouter: ["anthropic/claude-3.5-sonnet", "openai/gpt-4o", "google/gemini-2.0-flash-001"],
  azure_openai: ["gpt-4o", "gpt-4-turbo"],
  bedrock: ["anthropic.claude-3-5-sonnet-20241022-v2:0", "anthropic.claude-3-haiku-20240307-v1:0"],
  copilot: ["gpt-4o"],
  ollama: OLLAMA_CHAT_MODELS,
  // vibecli-mistralrs talks to the local vibecli daemon (default :7878) and
  // pins the in-process mistralrs backend via X-VibeCLI-Backend. Models here
  // are repo ids that mistralrs can lazy-load from Hugging Face on first use.
  "vibecli-mistralrs": [
    "Qwen/Qwen2.5-0.5B-Instruct",
    "Qwen/Qwen2.5-1.5B-Instruct",
    "Qwen/Qwen2.5-3B-Instruct",
    "Qwen/Qwen2.5-7B-Instruct",
    "Qwen/Qwen2.5-Coder-1.5B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "meta-llama/Llama-3.2-1B-Instruct",
    "meta-llama/Llama-3.2-3B-Instruct",
    "microsoft/Phi-3.5-mini-instruct",
  ],
  zhipu: ["glm-4-plus", "glm-4-flash"],
  vercel_ai: [],
  minimax: ["abab6.5s-chat"],
  sambanova: ["Meta-Llama-3.3-70B-Instruct"],
};

export const ALL_PROVIDERS = Object.keys(STATIC_MODELS);

/** Default model to pre-select when a provider is chosen in a dropdown. */
export const PROVIDER_DEFAULT_MODEL: Record<string, string> = {
  "claude-code": "claude-sonnet-4-6",
  claude:       "claude-sonnet-4-6",
  openai:       "gpt-4o",
  gemini:       "gemini-2.5-flash",
  groq:         "llama-3.3-70b-versatile",
  grok:         "grok-3-mini",
  mistral:      "mistral-large-latest",
  deepseek:     "deepseek-chat",
  cerebras:     "llama-3.3-70b",
  perplexity:   "sonar-pro",
  together:     "meta-llama/Llama-3.3-70B-Instruct",
  fireworks:    "accounts/fireworks/models/llama-v3p3-70b-instruct",
  openrouter:   "anthropic/claude-3.5-sonnet",
  azure_openai: "gpt-4o",
  bedrock:      "anthropic.claude-3-5-sonnet-20241022-v2:0",
  copilot:      "gpt-4o",
  ollama:       "qwen3",
  "vibecli-mistralrs": "Qwen/Qwen2.5-1.5B-Instruct",
  zhipu:        "glm-4-plus",
  vercel_ai:    "",
  minimax:      "abab6.5s-chat",
  sambanova:    "Meta-Llama-3.3-70B-Instruct",
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
      try {
        const result = await invoke<string[]>("ollama_list_models");
        if (result && result.length > 0) ollamaModels = result;
      } catch {
        // Ollama not running — keep static list
      }

      // Merge with static models
      const models = { ...STATIC_MODELS };
      if (ollamaModels.length > 0) {
        models.ollama = ollamaModels;
      }

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
