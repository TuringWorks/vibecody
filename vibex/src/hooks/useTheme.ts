import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type ThemeMode = "dark" | "light";

/** Theme persistence key in the shared ProfileStore (carries across restarts). */
const SETTING_KEY = "theme_mode";
/** localStorage mirror so the theme applies instantly on boot, before the
 *  daemon/store round-trip resolves (avoids a dark→light flash). */
const LS_KEY = "vibex-theme-mode";

/**
 * Apply a theme by toggling `data-theme` on <html> — the same mechanism VibeUI
 * uses, driving the shared design-system tokens (`[data-theme="light"]` block
 * in tokens.css). No token duplication: VibeX and VibeUI render identically.
 */
export function applyTheme(mode: ThemeMode): void {
  document.documentElement.setAttribute("data-theme", mode);
}

/**
 * Theme state + persistence. Reads the instant localStorage mirror first, then
 * reconciles with the ProfileStore (the durable, cross-app source of truth via
 * the `setting_get`/`setting_set` Tauri commands).
 */
export function useTheme() {
  const [mode, setMode] = useState<ThemeMode>(
    () => (localStorage.getItem(LS_KEY) as ThemeMode) || "dark"
  );

  // Apply on mount + whenever it changes.
  useEffect(() => {
    applyTheme(mode);
  }, [mode]);

  // Reconcile with the durable store on first load.
  useEffect(() => {
    (async () => {
      try {
        const v = await invoke<string | null>("setting_get", { key: SETTING_KEY });
        if (v === "dark" || v === "light") {
          setMode(v);
          localStorage.setItem(LS_KEY, v);
        }
      } catch {
        /* daemon/store not ready — localStorage default stands */
      }
    })();
  }, []);

  const setTheme = useCallback((next: ThemeMode) => {
    setMode(next);
    localStorage.setItem(LS_KEY, next);
    invoke("setting_set", { key: SETTING_KEY, value: next }).catch(() => {
      /* best-effort persistence; localStorage already holds it */
    });
  }, []);

  return { mode, setTheme };
}
