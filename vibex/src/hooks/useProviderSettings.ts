import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Providers that need an API key (vs local ollama/mistralrs). Mirrors the
 *  key-requiring set from VibeUI's useModelRegistry. */
export const KEYED_PROVIDERS: { id: string; label: string; needsUrl?: boolean; optional?: boolean }[] = [
  // Ollama is local-first (no key for local models), but a token is required to
  // reach Ollama Cloud / Turbo models (e.g. `qwen3-coder:480b-cloud`). Optional:
  // local models ignore it, and a loopback Ollama drops it for local models.
  { id: "ollama", label: "Ollama Cloud / Turbo (optional — for *-cloud models)", optional: true },
  { id: "anthropic", label: "Anthropic (Claude)" },
  { id: "openai", label: "OpenAI" },
  { id: "gemini", label: "Google Gemini" },
  { id: "groq", label: "Groq" },
  { id: "grok", label: "xAI (Grok)" },
  { id: "mistral", label: "Mistral" },
  { id: "deepseek", label: "DeepSeek" },
  { id: "cerebras", label: "Cerebras" },
  { id: "perplexity", label: "Perplexity" },
  { id: "together", label: "Together" },
  { id: "fireworks", label: "Fireworks" },
  { id: "openrouter", label: "OpenRouter" },
  { id: "azure_openai", label: "Azure OpenAI", needsUrl: true },
  { id: "zhipu", label: "Zhipu (GLM)" },
  { id: "sambanova", label: "SambaNova" },
  { id: "minimax", label: "MiniMax" },
];

/** Local providers — no key, listed for the default-provider selector. */
export const LOCAL_PROVIDERS = ["ollama", "vibecli-mistralrs"];

/**
 * Provider config management against the shared ProfileStore (via Tauri
 * commands embedded from vibecli_cli). Keys are write-only from the UI's view —
 * we only ever learn *whether* a provider is configured, never read the secret
 * back. Carries over with VibeUI because it's the same encrypted store.
 */
export function useProviderSettings() {
  const [configured, setConfigured] = useState<Set<string>>(new Set());
  const [defaultProvider, setDefaultProviderState] = useState<string>("ollama");

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<string[]>("provider_key_list");
      setConfigured(new Set(list));
    } catch {
      /* store unavailable */
    }
    try {
      const dp = await invoke<string | null>("setting_get", { key: "default_provider" });
      if (typeof dp === "string" && dp) setDefaultProviderState(dp);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const setKey = useCallback(
    async (provider: string, apiKey: string) => {
      await invoke("provider_key_set", { provider, apiKey });
      await refresh();
    },
    [refresh]
  );

  const deleteKey = useCallback(
    async (provider: string) => {
      await invoke("provider_key_delete", { provider });
      await refresh();
    },
    [refresh]
  );

  const setProviderUrl = useCallback(async (provider: string, url: string) => {
    await invoke("provider_config_set", { provider, key: "api_url", value: url });
  }, []);

  const setDefaultProvider = useCallback(async (provider: string) => {
    setDefaultProviderState(provider);
    await invoke("setting_set", { key: "default_provider", value: provider });
  }, []);

  return { configured, defaultProvider, setKey, deleteKey, setProviderUrl, setDefaultProvider, refresh };
}
