/**
 * useApiKeyMonitor — periodic API key health monitoring with change-based notifications.
 *
 * Runs at app level (in App.tsx) so it works regardless of which panel is open.
 * Only fires notifications when a key's status *changes* (valid -> invalid, or first failure).
 * Emits a custom event "vibeui:api-key-validations" so SettingsPanel can display inline statuses
 * without running its own polling loop.
 *
 * Usage:
 *   const monitor = useApiKeyMonitor({ toast, addNotification });
 *   // monitor.validations — current validation map
 *   // monitor.lastChecked — timestamp of last check
 */

import { useEffect, useRef, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ToastApi } from "./useToast";
import type { AddNotificationOpts } from "./useNotifications";

export interface ApiKeyValidation {
  provider: string;
  valid: boolean;
  error: string | null;
  latency_ms: number;
}

/** Human-readable provider names for notification messages. */
const PROVIDER_LABELS: Record<string, string> = {
  anthropic: "Anthropic (Claude)",
  openai: "OpenAI",
  gemini: "Google Gemini",
  grok: "xAI (Grok)",
  groq: "Groq",
  openrouter: "OpenRouter",
  azure_openai: "Azure OpenAI",
  mistral: "Mistral AI",
  cerebras: "Cerebras",
  deepseek: "DeepSeek",
  zhipu: "Zhipu (GLM)",
  vercel_ai: "Vercel AI",
  minimax: "MiniMax",
  perplexity: "Perplexity",
  together: "Together AI",
  fireworks: "Fireworks AI",
  sambanova: "SambaNova",
  ollama: "Ollama",
};

/** Validation interval in ms (5 minutes). */
const VALIDATION_INTERVAL = 5 * 60 * 1000;

/** Initial delay after mount (let keys load first). */
const INITIAL_DELAY = 4000;

interface UseApiKeyMonitorOpts {
  toast: ToastApi;
  addNotification: (opts: AddNotificationOpts) => void;
  /** If true, also attempts OS-level notification for critical failures. */
  osNotifications?: boolean;
}

export function useApiKeyMonitor({ toast, addNotification, osNotifications }: UseApiKeyMonitorOpts) {
  const [validations, setValidations] = useState<Record<string, ApiKeyValidation>>({});
  const [lastChecked, setLastChecked] = useState<number | null>(null);
  const prevStatusRef = useRef<Record<string, boolean>>({});
  const isFirstRunRef = useRef(true);

  const sendOsNotification = useCallback((title: string, body: string) => {
    if (!osNotifications) return;
    if ("Notification" in window && Notification.permission === "granted") {
      new Notification(title, { body, icon: "/icons/128x128.png" });
    } else if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission().then(perm => {
        if (perm === "granted") {
          new Notification(title, { body, icon: "/icons/128x128.png" });
        }
      });
    }
  }, [osNotifications]);

  const runValidation = useCallback(async () => {
    try {
      const results = await invoke<ApiKeyValidation[]>("validate_all_api_keys");
      const map: Record<string, ApiKeyValidation> = {};
      results.forEach(r => { map[r.provider] = r; });

      setValidations(map);
      setLastChecked(Date.now());

      // Emit event for SettingsPanel to consume
      window.dispatchEvent(new CustomEvent("vibeui:api-key-validations", { detail: map }));

      const prevStatus = prevStatusRef.current;
      const newlyFailed: ApiKeyValidation[] = [];
      const recovered: ApiKeyValidation[] = [];

      for (const result of results) {
        // Skip unconfigured keys — don't notify about keys that aren't set
        if (result.error === "No key configured") continue;

        const wasValid = prevStatus[result.provider];
        const isValid = result.valid;

        if (wasValid === true && !isValid) {
          // Was working, now broken
          newlyFailed.push(result);
        } else if (wasValid === false && isValid) {
          // Was broken, now recovered
          recovered.push(result);
        } else if (wasValid === undefined && !isValid && !isFirstRunRef.current) {
          // First time seeing this provider and it's failing (not first app run)
          newlyFailed.push(result);
        }
      }

      // On first run, report all currently-failing keys as a batch
      if (isFirstRunRef.current) {
        const failingOnStartup = results.filter(
          r => !r.valid && r.error !== "No key configured"
        );
        if (failingOnStartup.length > 0) {
          const names = failingOnStartup.map(r => PROVIDER_LABELS[r.provider] || r.provider);
          if (failingOnStartup.length === 1) {
            const r = failingOnStartup[0];
            const label = PROVIDER_LABELS[r.provider] || r.provider;
            toast.warn(`${label} API key is not working: ${r.error}`);
            addNotification({
              title: `${label} API key invalid`,
              body: r.error || "Validation failed",
              severity: "warn",
              category: "api-keys",
            });
          } else {
            toast.warn(`${failingOnStartup.length} API keys need attention: ${names.join(", ")}`);
            for (const r of failingOnStartup) {
              const label = PROVIDER_LABELS[r.provider] || r.provider;
              addNotification({
                title: `${label} API key invalid`,
                body: r.error || "Validation failed",
                severity: "warn",
                category: "api-keys",
              });
            }
          }
        }
        isFirstRunRef.current = false;
      } else {
        // Subsequent runs — only notify on changes
        for (const r of newlyFailed) {
          const label = PROVIDER_LABELS[r.provider] || r.provider;
          toast.error(`${label} API key failed: ${r.error}`);
          addNotification({
            title: `${label} API key failed`,
            body: `${r.error || "Validation failed"}. Check Settings > API Keys.`,
            severity: "error",
            category: "api-keys",
          });
          sendOsNotification(
            "VibeCody: API Key Failed",
            `${label}: ${r.error || "Validation failed"}`
          );
        }

        for (const r of recovered) {
          const label = PROVIDER_LABELS[r.provider] || r.provider;
          toast.success(`${label} API key recovered (${r.latency_ms}ms)`);
          addNotification({
            title: `${label} API key recovered`,
            body: `Key is working again. Latency: ${r.latency_ms}ms`,
            severity: "success",
            category: "api-keys",
          });
        }
      }

      // Update previous status map
      const newPrevStatus: Record<string, boolean> = {};
      for (const result of results) {
        if (result.error !== "No key configured") {
          newPrevStatus[result.provider] = result.valid;
        }
      }
      prevStatusRef.current = newPrevStatus;

    } catch (err) {
      console.warn("[ApiKeyMonitor] validation failed:", err);
    }
  }, [toast, addNotification, sendOsNotification]);

  useEffect(() => {
    const initial = setTimeout(runValidation, INITIAL_DELAY);
    const interval = setInterval(runValidation, VALIDATION_INTERVAL);
    return () => {
      clearTimeout(initial);
      clearInterval(interval);
    };
  }, [runValidation]);

  // Also re-validate when keys are saved (SettingsPanel emits this event)
  useEffect(() => {
    const onKeysChanged = () => {
      // Small delay to let the backend persist
      setTimeout(runValidation, 1500);
    };
    window.addEventListener("vibeui:providers-updated", onKeysChanged);
    return () => window.removeEventListener("vibeui:providers-updated", onKeysChanged);
  }, [runValidation]);

  return { validations, lastChecked, revalidate: runValidation };
}
