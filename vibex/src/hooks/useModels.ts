import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FALLBACK_MODELS } from "../constants/models";

export interface DaemonModel {
  id: string;
  name?: string;
  provider: string;
  active?: boolean;
}

/**
 * Merge the daemon's live list with the static catalog. Live rows win on
 * collision (they reflect what's actually installed/configured), and every
 * static row a live list omits — notably ollama `*-cloud` models and other
 * providers' catalogs, which a local `/api/tags` never advertises — is added
 * so the picker is never missing selectable models.
 */
function mergeWithCatalog(live: DaemonModel[]): DaemonModel[] {
  const seen = new Set<string>();
  const out: DaemonModel[] = [];
  const push = (m: DaemonModel) => {
    const key = `${m.provider}/${m.name}`;
    if (m.name && !seen.has(key)) {
      seen.add(key);
      out.push(m);
    }
  };
  live.forEach(push);
  FALLBACK_MODELS.forEach(push);
  return out;
}

/**
 * Model list for the picker. Prefers the daemon's `/models` endpoint (the
 * source of truth for what's actually installed/configured), unioned with a
 * static catalog so cloud/catalog models are always selectable. When the
 * daemon is offline the static catalog stands in on its own, so the picker
 * still loads models instead of showing nothing.
 */
export function useModels(daemonUrl: string, daemonOnline: boolean) {
  const [models, setModels] = useState<DaemonModel[]>(FALLBACK_MODELS);

  useEffect(() => {
    if (!daemonOnline) {
      setModels(FALLBACK_MODELS);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const rows = await invoke<DaemonModel[]>("list_daemon_models", { url: daemonUrl });
        if (!cancelled) {
          // Keep the addressable "provider/model" rows that carry a clean name
          // (drop the synthetic "active" entry), then union with the catalog.
          setModels(mergeWithCatalog(rows.filter((m) => !!m.name)));
        }
      } catch {
        if (!cancelled) setModels(FALLBACK_MODELS);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl, daemonOnline]);

  return models;
}
