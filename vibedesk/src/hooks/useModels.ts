import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface DaemonModel {
  id: string;
  name?: string;
  provider: string;
  active?: boolean;
}

// The daemon's /models endpoint is the single source of truth for the catalog
// (see vibe-ai/src/catalog.rs). We cache its last successful response so the
// picker still lists models while the daemon is briefly unreachable — the cache
// is the daemon's own data, not a hardcoded fallback list.
const CACHE_KEY = "vibedesk:models:v1";

function readCache(): DaemonModel[] {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    return Array.isArray(parsed) ? (parsed as DaemonModel[]) : [];
  } catch {
    return [];
  }
}

/**
 * Model list for the picker, sourced entirely from the daemon's `/models`
 * endpoint. Rows are cached to localStorage on each success and served from
 * cache when the daemon is offline, so the list survives a disconnect without
 * VibeDesk carrying its own catalog.
 */
export function useModels(daemonUrl: string, daemonOnline: boolean) {
  const [models, setModels] = useState<DaemonModel[]>(readCache);

  useEffect(() => {
    if (!daemonOnline) {
      setModels(readCache());
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const rows = await invoke<DaemonModel[]>("list_daemon_models", { url: daemonUrl });
        // Keep addressable "provider/model" rows (drop the synthetic active
        // entry, which carries no name).
        const named = rows.filter((m) => !!m.name);
        if (!cancelled) {
          setModels(named);
          localStorage.setItem(CACHE_KEY, JSON.stringify(named));
        }
      } catch {
        if (!cancelled) setModels(readCache());
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl, daemonOnline]);

  return models;
}
