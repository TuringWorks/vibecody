import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ChangedFile {
  path: string;
  status: string;
}

export interface GitStatus {
  is_git_repo: boolean;
  branch: string;
  changed_count: number;
  changed: ChangedFile[];
}

/**
 * Live git status for the Environment inspector (VX-109). Polls the daemon's
 * `/api/vibedesk/git/status`. The daemon is the source of truth; this is a thin
 * client. `refreshKey` lets callers force a refetch (e.g. after a run finishes).
 */
export function useEnvironment(daemonUrl: string, daemonOnline: boolean, refreshKey = 0) {
  const [status, setStatus] = useState<GitStatus | null>(null);

  const refresh = useCallback(async () => {
    if (!daemonOnline) return;
    try {
      const s = await invoke<GitStatus>("git_status", { url: daemonUrl });
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, [daemonUrl, daemonOnline]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 8_000);
    return () => clearInterval(id);
  }, [refresh, refreshKey]);

  return { status, refresh };
}
