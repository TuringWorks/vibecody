import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { applyThemeById, getPairedTheme, THEMES, type ThemeDef } from "../theme/themes";

/** Back-compat alias used by callers that haven't been migrated to the
 *  full theme-id system. */
export type ThemeMode = "dark" | "light";

/* ── Persistence keys ─────────────────────────────────────────────────── */

/** Durable theme-id setting in the shared ProfileStore. New code paths
 *  use this — preserves the full theme selection across restarts. */
const SETTING_KEY_ID = "theme_id";
/** Legacy ProfileStore key from the dark/light-only era. We read it once on
 *  boot and migrate it into `theme_id` so existing users don't lose their
 *  preference. */
const LEGACY_SETTING_KEY_MODE = "theme_mode";

/** localStorage mirror so the theme applies instantly on boot, before the
 *  daemon round-trip resolves (avoids dark→light flash). The themes.ts
 *  applier already writes `vibedesk-theme-id` after a successful apply, so
 *  this is the same key it reads back from. */
const LS_KEY_ID = "vibedesk-theme-id";

/* ── Defaults ─────────────────────────────────────────────────────────── */

const DEFAULT_DARK = "dark-default";
const DEFAULT_LIGHT = "light-default";

function isKnownThemeId(id: string | null | undefined): id is string {
  return !!id && THEMES.some((t) => t.id === id);
}

/* ── Hook ─────────────────────────────────────────────────────────────── */

/**
 * Theme state + persistence over the full VibeUI theme registry.
 *
 * Reads the instant localStorage mirror first, then reconciles with the
 * shared ProfileStore. If the durable store holds the legacy `theme_mode`
 * setting only, we promote it to a `theme_id` (e.g. `"light"` → `"light-default"`)
 * so the upgrade is invisible.
 *
 * Returns both the rich (`themeId` / `theme` / `setThemeId`) and the legacy
 * (`mode` / `setTheme`) interfaces so older call sites keep working.
 */
export function useTheme() {
  const [themeId, setThemeIdState] = useState<string>(() => {
    const fromLS = localStorage.getItem(LS_KEY_ID);
    if (isKnownThemeId(fromLS)) return fromLS;
    return DEFAULT_DARK;
  });

  // Apply on mount + whenever it changes.
  useEffect(() => {
    applyThemeById(themeId);
  }, [themeId]);

  // Reconcile with the durable store on first load.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const stored = await invoke<string | null>("setting_get", { key: SETTING_KEY_ID });
        if (cancelled) return;
        if (isKnownThemeId(stored)) {
          setThemeIdState(stored);
          return;
        }
        // No theme_id in the store yet — try to migrate from the legacy
        // theme_mode setting before falling back to whatever we already have.
        const legacy = await invoke<string | null>("setting_get", { key: LEGACY_SETTING_KEY_MODE });
        if (cancelled) return;
        if (legacy === "dark" || legacy === "light") {
          const promoted = legacy === "light" ? DEFAULT_LIGHT : DEFAULT_DARK;
          setThemeIdState(promoted);
          // Write the promoted id forward so subsequent loads skip the migration.
          invoke("setting_set", { key: SETTING_KEY_ID, value: promoted }).catch(() => {});
        }
      } catch {
        /* daemon/store not ready — localStorage default stands */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const setThemeId = useCallback((next: string) => {
    if (!isKnownThemeId(next)) return;
    setThemeIdState(next);
    // applyThemeById (called from the effect) writes the LS mirror, but write
    // here too so even an interrupted commit (closed tab before effect runs)
    // persists the choice.
    localStorage.setItem(LS_KEY_ID, next);
    invoke("setting_set", { key: SETTING_KEY_ID, value: next }).catch(() => {
      /* best-effort persistence; localStorage already holds it */
    });
  }, []);

  /** Legacy mode-only setter — picks the dark/light counterpart of the
   *  current theme in the same pair (so "Charcoal dark → light" stays
   *  on Charcoal, not Default). */
  const setTheme = useCallback(
    (mode: ThemeMode) => {
      const current = THEMES.find((t) => t.id === themeId);
      if (current?.mode === mode) return;
      const paired = current ? getPairedTheme(current.id) : undefined;
      setThemeId(paired?.id ?? (mode === "light" ? DEFAULT_LIGHT : DEFAULT_DARK));
    },
    [themeId, setThemeId],
  );

  const theme: ThemeDef | undefined = THEMES.find((t) => t.id === themeId);
  const mode: ThemeMode = theme?.mode ?? "dark";

  return { themeId, theme, setThemeId, mode, setTheme, themes: THEMES };
}
