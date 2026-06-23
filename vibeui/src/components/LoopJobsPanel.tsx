/**
 * LoopJobsPanel (gap C1) — viewer/manager for `/loop` jobs.
 *
 * `/loop` jobs (recurring or self-paced loop-until-done) run in the CLI REPL
 * process and persist to ~/.vibecli/loops.json. This panel reads that store
 * (`list_loop_jobs`) and can request a stop (`stop_loop_job`, honored by the
 * REPL before the loop's next iteration). It is a viewer/manager, not a
 * launcher — start loops from the REPL with `/loop <interval|auto> <prompt>`.
 */
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LoopSpec {
  mode: "SelfPaced" | { Recurring: { interval_secs: number } };
  prompt: string;
  max_iter: number;
  max_duration_secs: number;
}
interface LoopJob {
  id: string;
  spec: LoopSpec;
  iterations_done: number;
  status: string;
  created_at_secs: number;
}

function modeLabel(spec: LoopSpec): string {
  if (spec.mode === "SelfPaced") return "self-paced";
  const secs = spec.mode.Recurring.interval_secs;
  return secs % 3600 === 0 ? `every ${secs / 3600}h`
    : secs % 60 === 0 ? `every ${secs / 60}m`
    : `every ${secs}s`;
}

const STATUS_COLOR: Record<string, string> = {
  running: "#4caf50",
  done: "var(--text-secondary)",
  stopped: "#e2a64d",
  expired: "#e2a64d",
  maxiter: "#e2a64d",
  failed: "#e5484d",
};

export function LoopJobsPanel() {
  const [jobs, setJobs] = useState<LoopJob[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setJobs(await invoke<LoopJob[]>("list_loop_jobs"));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const stop = async (id: string) => {
    try {
      await invoke<boolean>("stop_loop_job", { id });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flex: 1, lineHeight: 1.5 }}>
          <code>/loop</code> jobs run in the CLI REPL. Start one with <code>/loop 5m &lt;prompt&gt;</code> or
          <code> /loop auto &lt;prompt&gt;</code>; manage them here.
        </div>
        <button onClick={refresh} disabled={loading}
          style={{ padding: "4px 12px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", cursor: "pointer" }}>
          {loading ? "…" : "Refresh"}
        </button>
      </div>

      {error && <div style={{ fontSize: "var(--font-size-sm)", color: "#e5484d" }}>{error}</div>}

      {jobs.length === 0 && !loading && (
        <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)" }}>No loop jobs.</div>
      )}

      {jobs.map((j) => {
        const terminal = j.status !== "running";
        return (
          <div key={j.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${STATUS_COLOR[j.status] ?? "var(--text-secondary)"}` }}>
            <div style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{j.id}</span>
              <span style={{ fontSize: "var(--font-size-xs)", textTransform: "uppercase", fontWeight: 700, color: STATUS_COLOR[j.status] ?? "var(--text-secondary)" }}>{j.status}</span>
              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{modeLabel(j.spec)} · {j.iterations_done}/{j.spec.max_iter}</span>
              <span style={{ flex: 1 }} />
              {!terminal && (
                <button onClick={() => stop(j.id)}
                  style={{ padding: "2px 10px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", cursor: "pointer" }}>
                  Stop
                </button>
              )}
            </div>
            <div style={{ fontSize: "var(--font-size-md)", marginTop: 4 }}>{j.spec.prompt}</div>
          </div>
        );
      })}
    </div>
  );
}

export default LoopJobsPanel;
