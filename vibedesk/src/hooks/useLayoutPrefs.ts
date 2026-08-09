import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Settings key holding the shell's layout state as a JSON blob. */
const PREFS_KEY = "vibedesk.layout";
/** localStorage mirror, so the panel renders at its saved width on the very
 *  first paint instead of snapping after the async settings read returns. */
const LS_KEY = "vibedesk.layout";

/** Width bounds for the right rail, in px. */
export const ENV_MIN_WIDTH = 200;
export const ENV_MAX_WIDTH = 520;

export interface LayoutPrefs {
  /** Width of the right-hand Environment rail. */
  envWidth: number;
  /** Which Environment sections are expanded, by section id. */
  envOpen: Record<string, boolean>;
}

const DEFAULTS: LayoutPrefs = {
  envWidth: 260,
  // Workspace state is what the rail is for, so it opens; Sources is a
  // navigation affordance and stays folded until asked for.
  envOpen: { workspace: true, sources: false },
};

export function clampEnvWidth(px: number): number {
  return Math.min(ENV_MAX_WIDTH, Math.max(ENV_MIN_WIDTH, Math.round(px)));
}

function parse(raw: string | null): Partial<LayoutPrefs> {
  if (!raw) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? (parsed as Partial<LayoutPrefs>) : {};
  } catch {
    return {};
  }
}

function merge(base: LayoutPrefs, patch: Partial<LayoutPrefs>): LayoutPrefs {
  return {
    envWidth: clampEnvWidth(patch.envWidth ?? base.envWidth),
    envOpen: { ...base.envOpen, ...(patch.envOpen ?? {}) },
  };
}

/**
 * Shell layout state that must survive a restart: the Environment rail's width
 * and which of its sections are expanded.
 *
 * Follows `useTheme`'s two-tier pattern — read the localStorage mirror
 * synchronously so the first paint is already correct, then reconcile with the
 * encrypted settings store once it answers. Writes go to both.
 */
export function useLayoutPrefs() {
  const [prefs, setPrefs] = useState<LayoutPrefs>(() =>
    merge(DEFAULTS, parse(typeof localStorage === "undefined" ? null : localStorage.getItem(LS_KEY))),
  );
  // Skip the first persist: it would just write back what we read.
  const hydrated = useRef(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const stored = await invoke<string | null>("setting_get", { key: PREFS_KEY });
        if (alive && stored) setPrefs((prev) => merge(prev, parse(stored)));
      } catch {
        /* store not ready — the localStorage value stands */
      } finally {
        hydrated.current = true;
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (!hydrated.current) return;
    const raw = JSON.stringify(prefs);
    try {
      localStorage.setItem(LS_KEY, raw);
    } catch {
      /* private mode — the settings store below is still authoritative */
    }
    invoke("setting_set", { key: PREFS_KEY, value: raw }).catch(() => {
      /* best-effort; the mirror already holds it */
    });
  }, [prefs]);

  const setEnvWidth = useCallback((px: number) => {
    setPrefs((prev) => ({ ...prev, envWidth: clampEnvWidth(px) }));
  }, []);

  const toggleSection = useCallback((id: string) => {
    setPrefs((prev) => ({
      ...prev,
      envOpen: { ...prev.envOpen, [id]: !(prev.envOpen[id] ?? false) },
    }));
  }, []);

  const isOpen = useCallback(
    (id: string, fallback = true) => prefs.envOpen[id] ?? fallback,
    [prefs.envOpen],
  );

  return { envWidth: prefs.envWidth, setEnvWidth, isOpen, toggleSection };
}
