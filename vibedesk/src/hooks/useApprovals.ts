import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** One pending approval, mirroring the daemon's `PendingPromptEvent`. */
export interface PendingApproval {
  request_id: string;
  audit_id: string;
  /** `kind=… audit_id=… origin={…}` — the tainted value's provenance. */
  summary: string;
  /** Which gate fired: `ToolCallArgument`, `McpArgument`, … */
  sink: string;
  issued_at: number;
}

/**
 * Pending approval prompts from the daemon's tainted-content gate.
 *
 * The composer's approval pill only ever selected a *policy*. When the daemon
 * actually gated an action (running with `--tainted-http-prompt`), the run
 * blocked with nothing on screen to answer it — indistinguishable from a hang.
 * This subscribes to the daemon's prompt queue and lets the UI resolve them.
 *
 * The daemon re-emits a full snapshot on every change, so events are merged by
 * `request_id` rather than appended. When the daemon isn't running with the
 * gate enabled the stream never emits and this stays empty — no configuration
 * needed either way.
 */
export function useApprovals(daemonUrl: string, daemonOnline: boolean) {
  const [pending, setPending] = useState<PendingApproval[]>([]);
  const unlisten = useRef<UnlistenFn | null>(null);
  // Prompts answered locally, so a snapshot emitted before the daemon has
  // processed our response doesn't resurrect a prompt the user just dismissed.
  const answered = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!daemonOnline) return;
    let alive = true;
    (async () => {
      const off = await listen<PendingApproval>("approval://pending", (e) => {
        const ev = e.payload;
        if (!ev?.request_id || answered.current.has(ev.request_id)) return;
        setPending((prev) =>
          prev.some((p) => p.request_id === ev.request_id) ? prev : [...prev, ev]
        );
      });
      if (!alive) {
        off();
        return;
      }
      unlisten.current = off;
      try {
        await invoke("stream_approvals", { url: daemonUrl });
      } catch (e) {
        // The gate is opt-in daemon-side; a daemon without it refuses the
        // stream. That's the common case, not an error worth surfacing.
        console.debug("approval stream unavailable", e);
      }
    })();
    return () => {
      alive = false;
      unlisten.current?.();
      unlisten.current = null;
    };
  }, [daemonUrl, daemonOnline]);

  const respond = useCallback(
    async (requestId: string, approve: boolean) => {
      answered.current.add(requestId);
      setPending((prev) => prev.filter((p) => p.request_id !== requestId));
      try {
        await invoke("respond_approval", { url: daemonUrl, requestId, approve });
      } catch (e) {
        console.error("approval response failed", e);
      }
    },
    [daemonUrl]
  );

  return { pending, respond };
}
