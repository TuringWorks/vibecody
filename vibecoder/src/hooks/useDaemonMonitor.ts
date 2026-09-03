/**
 * useDaemonMonitor — periodic daemon health monitoring with change-based notifications.
 *
 * Runs at app level (in App.tsx) so it works regardless of which panel is open.
 * Only fires notifications when the daemon status *changes* (online ↔ offline),
 * and — separately — when the daemon is reachable but this client's bearer token
 * will not authenticate against it, which no amount of health polling can see.
 * Emits a custom event "vibecoder:daemon-status" so BackgroundJobsPanel can display
 * live status without running its own polling loop.
 *
 * Usage:
 *   useDaemonMonitor({ toast, addNotification, daemonUrl: "http://localhost:7878" });
 */

import { useEffect, useRef, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { daemonReadiness } from "../lib/daemonFetch";
import type { ToastApi } from "./useToast";
import type { AddNotificationOpts } from "./useNotifications";

export interface DaemonStatus {
  online: boolean;
  checkedAt: number;
}

/** How often to poll the daemon (30 seconds). */
const POLL_INTERVAL = 30_000;

/**
 * How long one `/health` probe may take before it counts as a failure.
 *
 * The daemon answers in ~5 ms when idle; the timeout exists for the case where
 * it is busy, not slow, so it is generous rather than tight.
 */
const PROBE_TIMEOUT_MS = 6000;

/**
 * Consecutive failed probes before the daemon is called offline.
 *
 * Two, not one: a single missed probe is indistinguishable from a busy daemon,
 * and treating it as an outage produced "daemon offline"/"back online" toast
 * pairs — plus a redundant `start_daemon` — for a process that never died.
 */
const OFFLINE_AFTER_FAILURES = 2;

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
export function isVibeCliHealth(body: unknown): boolean {
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
  // Consecutive failed probes. Reset by any success — see `check`.
  const failuresRef = useRef(0);
  // Whether the "your token will not authenticate" warning has been shown for
  // the current bad-credential episode. Cleared the moment readiness is good
  // again, so a later recurrence is reported rather than swallowed.
  const credentialWarnedRef = useRef(false);

  const check = useCallback(async () => {
    let probeOk: boolean;
    try {
      const res = await fetch(`${daemonUrlRef.current}${HEALTH_PATH}`, {
        signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
      });
      // `res.ok` alone is liveness, not identity: any local service that
      // answers 200 on this port would read as a healthy daemon and every
      // panel would then fail with a confusing error. `/health` reports
      // `service: "vibecli"` precisely so clients can tell the difference.
      const body: unknown = res.ok ? await res.json().catch(() => null) : null;
      probeOk = isVibeCliHealth(body);
    } catch {
      probeOk = false;
    }

    // One missed probe is not an outage. A daemon that is merely busy — a code
    // graph build pegging a core, a cold model load — can miss a 4 s deadline
    // while it is perfectly alive, and declaring it offline on that one sample
    // both cried wolf to the user and fired `start_daemon` at a daemon that was
    // already running. Require the failure to repeat before believing it.
    failuresRef.current = probeOk ? 0 : failuresRef.current + 1;
    const isOnline = probeOk || failuresRef.current < OFFLINE_AFTER_FAILURES;
    if (!probeOk && isOnline) {
      // Held back, deliberately: report nothing this tick and re-probe sooner
      // than the normal cadence, so a real outage is still caught quickly.
      setLastChecked(Date.now());
      return;
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

      // **Online is not the same as usable.** This probe proves a VibeCLI
      // daemon is on the port; it says nothing about whether we hold a token it
      // accepts. Those came apart for two and a half days: a second daemon on
      // another port overwrote the shared token file and exited, so this hook
      // reported a healthy daemon on every tick while every authenticated route
      // in the app returned 401 and each panel blamed the daemon in its own
      // words. Check the credential too, and say so once — a stale token does
      // not fix itself, so repeating it every 30 s would only be noise.
      void (async () => {
        const readiness = await daemonReadiness();
        if (!readiness || readiness.ready) {
          credentialWarnedRef.current = false;
          return;
        }
        if (credentialWarnedRef.current) return;
        credentialWarnedRef.current = true;
        toastRef.current.warn(readiness.message);
        addNotificationRef.current({
          title:
            readiness.tokenState === "stale"
              ? "Daemon token is stale"
              : "Cannot authenticate with the daemon",
          body: readiness.message,
          severity: "warn",
          category: "system",
        });
      })();

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
    // A held-back failure re-probes on the next tick; the cadence stays 30 s so
    // a busy daemon is not hammered while it is under the load that slowed it.
    return () => {
      clearTimeout(initial);
      clearInterval(interval);
    };
  }, [check]);

  return { online, lastChecked, recheck: check };
}
