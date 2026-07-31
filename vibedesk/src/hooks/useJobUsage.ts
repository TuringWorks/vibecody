import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** The subset of the daemon's JobRecord the usage readout needs. */
export interface JobUsage {
  steps_completed: number;
  tokens_used: number;
  cost_cents: number;
}

/**
 * Per-run usage (steps · tokens · cost), polled from the daemon's `/jobs/:id`.
 *
 * A run was previously opaque about what it was spending — a long tool loop and
 * a runaway one looked identical. Claude Desktop and Codex both surface this.
 * Polls only while the run is live, then fetches once more on settle so the
 * final totals stick; a daemon that reports nothing yields `null` and the
 * readout is simply absent rather than showing zeroes.
 */
export function useJobUsage(daemonUrl: string, sessionId: string | null, live: boolean) {
  const [usage, setUsage] = useState<JobUsage | null>(null);

  useEffect(() => {
    if (!sessionId) {
      setUsage(null);
      return;
    }
    let alive = true;
    const fetchOnce = async () => {
      try {
        const job = await invoke<JobUsage>("get_job", { url: daemonUrl, sessionId });
        if (alive) setUsage(job);
      } catch {
        // A job row that isn't there yet (or a daemon blip) just means no
        // readout this tick — never surface it as an error in the header.
      }
    };
    fetchOnce();
    if (!live) return () => {
      alive = false;
    };
    const id = setInterval(fetchOnce, 4_000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [daemonUrl, sessionId, live]);

  return usage;
}

/** `1.2k` / `847` — compact token counts for a header chip. */
export function formatTokens(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}

/** Cents → `$0.04`. Sub-cent spend shows as `<$0.01`, never as `$0.00`. */
export function formatCost(cents: number): string {
  if (cents === 0) return "$0.00";
  if (cents < 1) return "<$0.01";
  return `$${(cents / 100).toFixed(2)}`;
}
