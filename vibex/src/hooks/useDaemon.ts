import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Default VibeCLI daemon address. The daemon is the source of truth (AGENTS.md).
 * Overridable via the `VITE_DAEMON_URL` env var (e.g. to point at a daemon on a
 * non-default port during development) without touching code.
 */
export const DEFAULT_DAEMON_URL =
  import.meta.env.VITE_DAEMON_URL ?? "http://127.0.0.1:7878";

export type DaemonStatus = "checking" | "online" | "offline";

/**
 * Tracks reachability of the VibeCLI daemon. VibeX never re-implements daemon
 * logic — it talks to it over HTTP via the Tauri command layer (commands.rs),
 * the same pattern as vibeapp.
 */
export function useDaemon(url: string = DEFAULT_DAEMON_URL) {
  const [status, setStatus] = useState<DaemonStatus>("checking");
  const [error, setError] = useState<string | null>(null);

  const check = useCallback(async () => {
    setStatus("checking");
    try {
      await invoke<string>("check_daemon", { url });
      setStatus("online");
      setError(null);
    } catch (e) {
      setStatus("offline");
      setError(String(e));
    }
  }, [url]);

  useEffect(() => {
    check();
    const id = setInterval(check, 10_000);
    return () => clearInterval(id);
  }, [check]);

  return { status, error, recheck: check, url };
}
