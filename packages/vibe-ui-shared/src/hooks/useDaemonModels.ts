/**
 * useDaemonModels — the model picker's list, for every desktop shell.
 *
 * The daemon's `/models` is the single source of truth for the catalog
 * (`vibe-ai/src/catalog.rs`). This hook is the one place that reads it.
 *
 * It exists because the same twenty lines had been written three times.
 * VibeDesk and VibeAIChat each had their own fetch-and-cache with its own
 * localStorage key, and VibeCoder maintained a *separate hand-written copy of
 * the catalog in TypeScript* that had to be edited alongside the Rust one. The
 * cost was not only the duplication: VibeAIChat's row type omitted
 * `may_not_load`, so it never showed the GPU-budget warning VibeDesk grew after
 * auto-picking a 19.8 GB model on a 24 GB machine — a bug that existed only
 * because the type was retyped instead of shared.
 *
 * ## Three tiers, in order
 *
 * 1. **Live `/models`** — authoritative, and cached on every success.
 * 2. **The cache** — the daemon's *own* previous answer, so a brief disconnect
 *    does not empty the picker. Not a hardcoded list: it is data the daemon
 *    produced.
 * 3. **`fallback`** — only when there has never been a successful fetch, which
 *    in practice means a first run before the daemon has started. Callers pass
 *    a static list for this and nothing else.
 *
 * A tier is only consulted when the ones above it have nothing. The list a user
 * sees is therefore the freshest thing available, and never a stale hardcoded
 * one once the daemon has answered even once.
 */
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * One row of the daemon's `/models`.
 *
 * Shared rather than retyped per app. Every field the daemon can send is here,
 * so a shell cannot silently miss one — which is exactly how VibeAIChat lost
 * the `may_not_load` guard.
 */
export interface DaemonModel {
  /** Unique key, namespaced by provider. Not necessarily what you send. */
  id: string;
  /**
   * The string to send as the model.
   *
   * Absent on the synthetic "active provider" row, which is why every consumer
   * filters on it — see `isAddressable`.
   */
  name?: string;
  provider: string;
  /** The daemon's currently-selected provider. Carries no `name`. */
  active?: boolean;
  /** Stored weights, for locally-installed models. Absent when unknown. */
  size_bytes?: number;
  /**
   * The daemon can state this machine's GPU budget and this model exceeds it.
   *
   * Absent everywhere the budget is unknowable — never a guess. Ollama answers
   * an over-budget model with HTTP 500, which is not retryable, so a picker
   * that ignores this can default to a model that fails every single turn.
   */
  may_not_load?: boolean;
  /** The two numbers behind `may_not_load`, in words. */
  load_note?: string;
}

/**
 * Rows a user can actually select.
 *
 * The synthetic active-provider row has no `name`, so it is not addressable;
 * every shell filtered it out separately before this.
 */
export function isAddressable(model: DaemonModel): boolean {
  return typeof model.name === "string" && model.name.length > 0;
}

function readCache(key: string): DaemonModel[] {
  try {
    const raw = localStorage.getItem(key);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    // Narrowed, not cast: this is data from disk, and a shape written by an
    // older build must not crash the picker.
    return Array.isArray(parsed)
      ? (parsed as unknown[]).filter(
          (m): m is DaemonModel =>
            typeof m === "object" &&
            m !== null &&
            typeof (m as DaemonModel).provider === "string"
        )
      : [];
  } catch {
    // A private window, cleared site data, or storage the browser refuses.
    return [];
  }
}

function writeCache(key: string, models: DaemonModel[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(models));
  } catch {
    // Quota or a blocked accessor. The list still works this session.
  }
}

export interface UseDaemonModelsOptions {
  /** Daemon base URL, e.g. `http://127.0.0.1:7878`. */
  daemonUrl: string;
  /**
   * Skip the fetch and serve cache/fallback. Pass `false` while a health check
   * says the daemon is down, so the picker does not stall on a dead socket.
   */
  online?: boolean;
  /** localStorage key. Distinct per shell so two apps cannot fight over one. */
  cacheKey: string;
  /**
   * Static list for a first run that has never reached a daemon — tier 3, and
   * nothing else. Once the daemon answers once, this is never consulted again.
   */
  fallback?: DaemonModel[];
  /** Re-fetch every N ms. Omit for fetch-once-per-url. */
  pollMs?: number;
}

export interface DaemonModelsState {
  /** Addressable rows, freshest tier available. */
  models: DaemonModel[];
  /** Which tier produced `models` — for empty states and diagnostics. */
  source: "live" | "cache" | "fallback";
  /** True until the first fetch settles. */
  loading: boolean;
  /** Force a re-read. */
  refresh: () => void;
}

export function useDaemonModels({
  daemonUrl,
  online = true,
  cacheKey,
  fallback = [],
  pollMs,
}: UseDaemonModelsOptions): DaemonModelsState {
  const [state, setState] = useState<{
    models: DaemonModel[];
    source: DaemonModelsState["source"];
  }>(() => {
    const cached = readCache(cacheKey).filter(isAddressable);
    return cached.length > 0
      ? { models: cached, source: "cache" }
      : { models: fallback.filter(isAddressable), source: "fallback" };
  });
  const [loading, setLoading] = useState(true);
  const [nonce, setNonce] = useState(0);

  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;

    /** Serve the best tier below "live". */
    const degrade = () => {
      if (cancelled) return;
      const cached = readCache(cacheKey).filter(isAddressable);
      setState(
        cached.length > 0
          ? { models: cached, source: "cache" }
          : { models: fallback.filter(isAddressable), source: "fallback" }
      );
    };

    const load = async () => {
      if (!online) {
        degrade();
        setLoading(false);
        return;
      }
      try {
        const rows = await invoke<DaemonModel[]>("list_daemon_models", {
          url: daemonUrl,
        });
        const named = (Array.isArray(rows) ? rows : []).filter(isAddressable);
        if (cancelled) return;
        // An empty success is not an answer worth caching over a good one —
        // it would blank the picker and persist the blank.
        if (named.length > 0) {
          setState({ models: named, source: "live" });
          writeCache(cacheKey, named);
        } else {
          degrade();
        }
      } catch {
        degrade();
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void load();
    const timer = pollMs ? setInterval(() => void load(), pollMs) : undefined;
    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
    };
    // `fallback` is intentionally not a dependency: callers pass a literal, and
    // depending on it would refetch on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [daemonUrl, online, cacheKey, pollMs, nonce]);

  return { ...state, loading, refresh };
}

/** Every provider id present in a row set, in first-seen order. */
export function providersOf(models: DaemonModel[]): string[] {
  const seen = new Set<string>();
  return models.flatMap((m) =>
    m.provider && !seen.has(m.provider) ? (seen.add(m.provider), [m.provider]) : []
  );
}

/** The addressable model names a provider offers, in catalog order. */
export function modelsOf(models: DaemonModel[], provider: string): string[] {
  return models
    .filter((m) => m.provider === provider && isAddressable(m))
    .map((m) => m.name as string);
}
