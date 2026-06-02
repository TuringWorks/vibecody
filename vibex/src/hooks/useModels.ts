import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface DaemonModel {
  id: string;
  name?: string;
  provider: string;
  active?: boolean;
}

/**
 * Live model list from the daemon's `/models` endpoint. The daemon is the
 * source of truth for what's actually installed/configured, so VibeX shows the
 * real set (e.g. ollama's pulled models) instead of a hardcoded list.
 */
export function useModels(daemonUrl: string, daemonOnline: boolean) {
  const [models, setModels] = useState<DaemonModel[]>([]);

  useEffect(() => {
    if (!daemonOnline) return;
    let cancelled = false;
    (async () => {
      try {
        const rows = await invoke<DaemonModel[]>("list_daemon_models", { url: daemonUrl });
        if (!cancelled) {
          // Drop the synthetic "active" entry (id like "Ollama (model)") — keep
          // the addressable "provider/model" rows that carry a clean name.
          setModels(rows.filter((m) => !!m.name));
        }
      } catch {
        if (!cancelled) setModels([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl, daemonOnline]);

  return models;
}
