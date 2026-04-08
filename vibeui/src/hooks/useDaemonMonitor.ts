/**
 * useDaemonMonitor — periodic daemon health monitoring with change-based notifications.
 *
 * Runs at app level (in App.tsx) so it works regardless of which panel is open.
 * Only fires notifications when the daemon status *changes* (online ↔ offline).
 * Emits a custom event "vibeui:daemon-status" so BackgroundJobsPanel can display
 * live status without running its own polling loop.
 *
 * Usage:
 *   useDaemonMonitor({ toast, addNotification, daemonUrl: "http://localhost:7878" });
 */

import { useEffect, useRef, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ToastApi } from "./useToast";
import type { AddNotificationOpts } from "./useNotifications";

export interface DaemonStatus {
  online: boolean;
  checkedAt: number;
}

/** How often to poll the daemon (30 seconds). */
const POLL_INTERVAL = 30_000;

/** Initial delay after mount to let the app settle. */
const INITIAL_DELAY = 3000;

/** Daemon URL used for the health check. */
const HEALTH_PATH = "/health";

interface UseDaemonMonitorOpts {
  toast: ToastApi;
  addNotification: (opts: AddNotificationOpts) => void;
  daemonUrl?: string;
}

export function useDaemonMonitor({
  toast,
  addNotification,
  daemonUrl = "http://localhost:7878",
}: UseDaemonMonitorOpts) {
  const [online, setOnline] = useState(false);
  const [lastChecked, setLastChecked] = useState<number | null>(null);

  // Keep callbacks in refs so the interval closure never goes stale.
  const toastRef = useRef(toast);
  const addNotificationRef = useRef(addNotification);
  const daemonUrlRef = useRef(daemonUrl);
  toastRef.current = toast;
  addNotificationRef.current = addNotification;
  daemonUrlRef.current = daemonUrl;

  // Track previous online state to fire notifications only on transitions.
  const prevOnlineRef = useRef<boolean | null>(null);
  // Prevent hammering start_daemon on every poll tick while it boots.
  const startingRef = useRef(false);

  const check = useCallback(async () => {
    let isOnline = false;
    try {
      const res = await fetch(`${daemonUrlRef.current}${HEALTH_PATH}`, {
        signal: AbortSignal.timeout(4000),
      });
      isOnline = res.ok;
    } catch {
      isOnline = false;
    }

    const now = Date.now();
    setOnline(isOnline);
    setLastChecked(now);

    // Emit app-level event so BackgroundJobsPanel can sync without its own poll.
    window.dispatchEvent(
      new CustomEvent<DaemonStatus>("vibeui:daemon-status", {
        detail: { online: isOnline, checkedAt: now },
      })
    );

    const prev = prevOnlineRef.current;

    if (isOnline) {
      // Reset the "starting" guard so we retry if it ever goes offline again.
      startingRef.current = false;

      if (prev === null) {
        // First check and already running — silent confirmation.
        toastRef.current.success("VibeCLI daemon is running on port 7878");
        addNotificationRef.current({
          title: "Daemon online",
          body: "VibeCLI daemon is reachable at port 7878.",
          severity: "success",
          category: "system",
        });
      } else if (!prev) {
        // Was offline, now online.
        toastRef.current.success("VibeCLI daemon is back online");
        addNotificationRef.current({
          title: "Daemon recovered",
          body: "VibeCLI daemon is reachable again on port 7878.",
          severity: "success",
          category: "system",
        });
      }
    } else {
      // Daemon is offline. Try to start it via the Tauri backend (which knows
      // where the vibecli binary lives and manages the child process lifetime).
      if (!startingRef.current) {
        startingRef.current = true;
        try {
          const result = await invoke<string>("start_daemon");
          if (result === "started" || result === "running") {
            // Daemon came up — next poll tick will pick it up as online.
            startingRef.current = false;
          }
          // If result === "starting", keep startingRef=true and wait for next tick.
        } catch {
          // vibecli not installed or spawn failed — fall through to warning.
          startingRef.current = false;
          if (prev === null || prev) {
            toastRef.current.warn(
              "VibeCLI daemon is not running and could not be auto-started. " +
              "Install or run: vibecli --serve --port 7878"
            );
            addNotificationRef.current({
              title: "Daemon unavailable",
              body: "Could not auto-start the VibeCLI daemon. Install vibecli or start it manually.",
              severity: "warn",
              category: "system",
            });
          }
        }
      }

      if (prev && !startingRef.current) {
        // Was online, went offline and we couldn't restart it.
        toastRef.current.warn("VibeCLI daemon went offline — attempting to restart…");
      }
    }

    prevOnlineRef.current = isOnline;
  }, []);

  useEffect(() => {
    const initial = setTimeout(check, INITIAL_DELAY);
    const interval = setInterval(check, POLL_INTERVAL);
    return () => {
      clearTimeout(initial);
      clearInterval(interval);
    };
  }, [check]);

  return { online, lastChecked, recheck: check };
}
