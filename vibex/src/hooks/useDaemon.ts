import { useCallback, useEffect, useRef, useState } from "react";
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
/**
 * While the daemon is being autostarted on launch (lib.rs), keep showing
 * "connecting…" rather than flashing "offline" — it usually binds within a
 * second or two. After this grace window a still-unreachable daemon is "offline".
 */
const STARTUP_GRACE_MS = 8_000;

export function useDaemon(url: string = DEFAULT_DAEMON_URL) {
  const [status, setStatus] = useState<DaemonStatus>("checking");
  const [error, setError] = useState<string | null>(null);
  const startRef = useRef<number>(Date.now());
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const activeRef = useRef(true);

  const poll = useCallback(async () => {
    if (!activeRef.current) return;
    let online = false;
    try {
      await invoke<string>("check_daemon", { url });
      online = true;
    } catch (e) {
      if (activeRef.current) setError(String(e));
    }
    if (!activeRef.current) return;
    if (online) {
      setStatus("online");
      setError(null);
    } else {
      // Stay in "checking" during the autostart grace window, then "offline".
      const within = Date.now() - startRef.current < STARTUP_GRACE_MS;
      setStatus(within ? "checking" : "offline");
    }
    // Poll fast while waiting for the daemon to come up; relax once it's online.
    timerRef.current = setTimeout(poll, online ? 10_000 : 1_500);
  }, [url]);

  /** Reset the grace window and re-poll immediately (used after a manual start). */
  const recheck = useCallback(() => {
    startRef.current = Date.now();
    setStatus("checking");
    if (timerRef.current) clearTimeout(timerRef.current);
    poll();
  }, [poll]);

  useEffect(() => {
    activeRef.current = true;
    startRef.current = Date.now();
    poll();
    return () => {
      activeRef.current = false;
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [poll]);

  return { status, error, recheck, url };
}
