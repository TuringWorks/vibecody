import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Workflow, Square, RefreshCw } from "lucide-react";

interface AutomationsViewProps {
  daemonUrl: string;
  onClose: () => void;
}

/**
 * Mirrors `loop_engine::LoopJob`.
 *
 * Two shapes here are easy to get wrong and were, until this was run against a
 * live daemon: `status` serialises lowercase (`"running"`, not `"Running"`),
 * and `mode` is an externally-tagged enum — the unit variant is the bare string
 * `"SelfPaced"`, but the data-carrying one is `{ Recurring: { interval_secs } }`.
 */
type LoopMode = "SelfPaced" | { Recurring: { interval_secs: number } };

interface LoopJob {
  id: string;
  spec: {
    mode: LoopMode;
    prompt: string;
    max_iter: number;
    max_duration_secs: number;
    goal_id?: string | null;
  };
  iterations_done: number;
  status: string;
  created_at_secs: number;
  hosted: boolean;
}

/** `every 5m` / `self-paced` — reads the externally-tagged mode enum. */
function describeMode(mode: LoopMode): string {
  if (mode === "SelfPaced") return "self-paced";
  const secs = mode?.Recurring?.interval_secs ?? 0;
  if (secs % 3600 === 0) return `every ${secs / 3600}h`;
  if (secs % 60 === 0) return `every ${secs / 60}m`;
  return `every ${secs}s`;
}

/** `2m ago` / `3h ago` / `just now` */
function ago(unixSecs: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unixSecs);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

/**
 * Recurring prompts the daemon runs on its own schedule.
 *
 * The daemon has always had a resident scheduler (`hosted_loop::scheduler_tick`)
 * that runs these and survives the client disconnecting — jobs are persisted to
 * loops.json, so closing VibeDesk doesn't stop them. Its only clients were the
 * CLI REPL and raw HTTP, which is why this nav item was disabled.
 *
 * The `args` string is sent to the daemon verbatim and parsed there, so the
 * accepted forms can never drift from the CLI's — `5m run the tests` for a
 * recurring loop, `auto <prompt>` for one that runs until it reports done.
 */
export function AutomationsView({ daemonUrl, onClose }: AutomationsViewProps) {
  const [loops, setLoops] = useState<LoopJob[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [createError, setCreateError] = useState<string | null>(null);
  const [args, setArgs] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const res = await invoke<{ loops: LoopJob[] }>("list_loops", { url: daemonUrl });
      setLoops(res.loops ?? []);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [daemonUrl]);

  useEffect(() => {
    refresh();
    // These advance on the daemon's own schedule, so poll for iteration counts
    // and status rather than showing a snapshot that silently goes stale.
    const id = setInterval(refresh, 10_000);
    return () => clearInterval(id);
  }, [refresh]);

  async function create() {
    const a = args.trim();
    if (!a || busy) return;
    setBusy(true);
    setCreateError(null);
    try {
      await invoke("create_loop", { url: daemonUrl, args: a });
      setArgs("");
      await refresh();
    } catch (e) {
      // The daemon returns its usage hint here — worth showing verbatim.
      setCreateError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function stop(id: string) {
    try {
      await invoke("stop_loop", { url: daemonUrl, id });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  const isRunning = (l: LoopJob) => l.status.toLowerCase() === "running";
  const running = (loops ?? []).filter(isRunning);
  const finished = (loops ?? []).filter((l) => !isRunning(l));

  const row = (l: LoopJob) => (
    <li key={l.id} className="vx-auto__row">
      <div className="vx-auto__main">
        <span className="vx-auto__prompt" title={l.spec.prompt}>
          {l.spec.goal_id ? `goal ${l.spec.goal_id.slice(0, 8)}` : l.spec.prompt}
        </span>
        <span className="vx-auto__meta">
          {describeMode(l.spec.mode)} · {l.iterations_done}/
          {l.spec.max_iter} runs · created {ago(l.created_at_secs)}
        </span>
      </div>
      <span className={`vx-auto__status vx-auto__status--${l.status.toLowerCase()}`}>
        {l.status.toLowerCase()}
      </span>
      {isRunning(l) && (
        <button className="vx-auto__stop" onClick={() => stop(l.id)} title="Stop this automation">
          <Square size={11} fill="currentColor" /> Stop
        </button>
      )}
    </li>
  );

  return (
    <div className="vx-skills">
      <div className="vx-skills__head">
        <Workflow size={14} />
        <span>Automations</span>
        {loops && <span className="vx-skills__count">{running.length} running</span>}
        <div className="vx-skills__spacer" />
        <button className="vx-icon-btn" aria-label="Refresh" onClick={refresh} title="Refresh">
          <RefreshCw size={13} />
        </button>
        <button className="vx-icon-btn" aria-label="Close automations" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-auto__new">
        <input
          className="vx-auto__input"
          placeholder="5m run the tests"
          value={args}
          onChange={(e) => setArgs(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              create();
            }
          }}
        />
        <button className="vx-trash__btn" onClick={create} disabled={!args.trim() || busy}>
          {busy ? "Creating…" : "Create"}
        </button>
      </div>
      <div className="vx-auto__hint">
        <code>5m run the tests</code> repeats on an interval · <code>auto &lt;prompt&gt;</code> runs
        until it reports done. These run in the daemon, so they keep going after you close VibeDesk.
      </div>
      {createError && <div className="vx-auto__error">{createError}</div>}

      <div className="vx-plugins__body">
        {error && <div className="vx-files__empty">Failed to load automations: {error}</div>}
        {!error && loops === null && <div className="vx-files__empty">Loading…</div>}
        {!error && loops?.length === 0 && (
          <div className="vx-files__empty">No automations yet.</div>
        )}
        {running.length > 0 && (
          <>
            <div className="vx-pill-menu__group">Running</div>
            <ul className="vx-auto__list">{running.map(row)}</ul>
          </>
        )}
        {finished.length > 0 && (
          <>
            <div className="vx-pill-menu__group">Finished</div>
            <ul className="vx-auto__list">{finished.map(row)}</ul>
          </>
        )}
      </div>
    </div>
  );
}
