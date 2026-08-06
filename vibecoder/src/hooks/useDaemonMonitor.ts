/**
 * useDaemonMonitor — periodic daemon health monitoring with change-based notifications.
 *
 * Runs at app level (in App.tsx) so it works regardless of which panel is open.
 * Only fires notifications when the daemon status *changes* (online ↔ offline).
 * Emits a custom event "vibecoder:daemon-status" so BackgroundJobsPanel can display
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

/**
 * Value `/health` reports as `service`. Must match
 * `vibecli_cli::daemon_bootstrap::SERVICE_NAME` — the daemon and every client
 * agree on this one string to distinguish "VibeCLI is here" from "something is
 * listening on this port".
 */
const DAEMON_SERVICE_NAME = "vibecli";

/**
 * True when a `/health` body really came from a VibeCLI daemon.
 *
 * A daemon predating the `service` field is accepted via its exact legacy shape
 * (`status: "ok"` **and** a `version`), matching what the mobile/SDK/VS Code
 * clients do. Requiring `service` strictly was an upgrade regression: with an
 * older daemon already on 7878, the app reported it offline and then blamed
 * "another program" for holding the port. A body naming a *different* service
 * is still rejected — that is the case this check exists for.
 */
function isVibeCliHealth(body: unknown): boolean {
  if (typeof body !== "object" || body === null) return false;
  const b = body as { service?: unknown; status?: unknown; version?: unknown };
  if (typeof b.service === "string") return b.service === DAEMON_SERVICE_NAME;
  return b.status === "ok" && typeof b.version === "string";
}

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
    let isOnline: boolean;
    try {
      const res = await fetch(`${daemonUrlRef.current}${HEALTH_PATH}`, {
        signal: AbortSignal.timeout(4000),
      });
      // `res.ok` alone is liveness, not identity: any local service that
      // answers 200 on this port would read as a healthy daemon and every
      // panel would then fail with a confusing error. `/health` reports
      // `service: "vibecli"` precisely so clients can tell the difference.
      const body: unknown = res.ok ? await res.json().catch(() => null) : null;
      isOnline = isVibeCliHealth(body);
    } catch {
      isOnline = false;
    }

    const now = Date.now();
    setOnline(isOnline);
    setLastChecked(now);

    // Emit app-level event so BackgroundJobsPanel can sync without its own poll.
    window.dispatchEvent(
      new CustomEvent<DaemonStatus>("vibecoder:daemon-status", {
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
        } catch (e) {
          // Autostart failed. The backend returns a message that names the
          // actual cause and its fix — binary missing, port held by another
          // program, daemon exited on boot. Show *that* rather than a generic
          // "install vibecli", which is wrong advice for two of the three.
          startingRef.current = false;
          const reason = e instanceof Error ? e.message : String(e);
          const detail =
            reason.trim().length > 0
              ? reason
              : "Could not auto-start the VibeCLI daemon. Install vibecli or start it manually.";
          if (prev === null || prev) {
            toastRef.current.warn(detail);
            addNotificationRef.current({
              title: "Daemon unavailable",
              body: detail,
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
