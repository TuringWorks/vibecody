import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ChangedFile {
  path: string;
  status: string;
}

/** HEAD summary, absent in a repo with no commits yet. */
export interface HeadCommit {
  hash: string;
  message: string;
  author: string;
  timestamp: number;
}

export interface GitStatus {
  is_git_repo: boolean;
  branch: string;
  changed_count: number;
  changed: ChangedFile[];
  head?: HeadCommit | null;
}

/**
 * Live git status for the Environment inspector (VX-109). Polls the daemon's
 * `/api/vibedesk/git/status` for `path` — the active project or the selected
 * task's worktree. Without that scope the inspector reported the daemon's own
 * repo no matter which project was open, so "Changes" and the branch name were
 * about a repo the user wasn't looking at. `undefined` → the daemon's root.
 * The daemon is the source of truth; this is a thin client. `refreshKey` lets
 * callers force a refetch (e.g. after a run finishes).
 */
export function useEnvironment(
  daemonUrl: string,
  daemonOnline: boolean,
  path?: string,
  refreshKey = 0
) {
  const [status, setStatus] = useState<GitStatus | null>(null);

  const refresh = useCallback(async () => {
    if (!daemonOnline) return;
    try {
      const s = await invoke<GitStatus>("git_status", { url: daemonUrl, path });
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, [daemonUrl, daemonOnline, path]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 8_000);
    return () => clearInterval(id);
  }, [refresh, refreshKey]);

  return { status, refresh };
}
