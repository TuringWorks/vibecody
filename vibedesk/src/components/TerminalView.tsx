import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, TerminalSquare, CornerDownLeft } from "lucide-react";

interface TerminalViewProps {
  daemonUrl: string;
  /** Directory commands run in — the active project or task worktree. */
  path?: string;
  onClose: () => void;
}

/** One completed command, as returned by `/api/vibedesk/exec`. */
interface ExecResult {
  command: string;
  cwd: string;
  stdout: string;
  stderr: string;
  exit_code: number | null;
  timed_out: boolean;
  truncated: boolean;
  duration_ms: number;
  /** Timeout the daemon actually applied. Optional: older daemons omit it. */
  timeout_ms?: number;
  /** True when the daemon's hard cap reduced the timeout that was asked for. */
  timeout_clamped?: boolean;
}

/** `842ms` / `3.2s` */
function formatMs(ms: number): string {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
}

/**
 * The Terminal quick-action: run a command in the project and read its output.
 *
 * One-shot by design — the daemon runs each command to completion with a
 * timeout and returns the result. There is **no PTY**, so interactive programs
 * (`vim`, `top`, `ssh`, anything with a prompt) will not work; the empty state
 * says so, because the failure mode otherwise is a command that appears to hang
 * until it's killed. It covers what a terminal is actually wanted for next to a
 * coding agent: run the tests, check the build, look at git log.
 *
 * Commands run where the agent runs, so what you see here matches what the
 * agent sees — which is the point of having it in the same scope.
 */
export function TerminalView({ daemonUrl, path, onClose }: TerminalViewProps) {
  const [history, setHistory] = useState<ExecResult[]>([]);
  const [command, setCommand] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  // Previously-run commands, recalled with ↑/↓ like any shell.
  const recall = useRef<string[]>([]);
  const recallPos = useRef<number | null>(null);

  useLayoutEffect(() => {
    const el = bodyRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [history, busy]);

  const run = useCallback(async () => {
    const cmd = command.trim();
    if (!cmd || busy) return;
    setBusy(true);
    setError(null);
    setCommand("");
    recall.current = [...recall.current, cmd];
    recallPos.current = null;
    try {
      const res = await invoke<ExecResult>("run_command", { url: daemonUrl, command: cmd, path });
      setHistory((prev) => [...prev, res]);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [command, busy, daemonUrl, path]);

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      run();
      return;
    }
    const items = recall.current;
    if (e.key === "ArrowUp" && items.length > 0) {
      e.preventDefault();
      const next = recallPos.current === null ? items.length - 1 : Math.max(0, recallPos.current - 1);
      recallPos.current = next;
      setCommand(items[next]);
    } else if (e.key === "ArrowDown" && recallPos.current !== null) {
      e.preventDefault();
      const next = recallPos.current + 1;
      if (next >= items.length) {
        recallPos.current = null;
        setCommand("");
      } else {
        recallPos.current = next;
        setCommand(items[next]);
      }
    }
  }

  return (
    <div className="vx-term">
      <div className="vx-skills__head">
        <TerminalSquare size={14} />
        <span>Terminal</span>
        <span className="vx-term__cwd" title={path ?? "Daemon workspace root"}>
          {path ? path.split("/").filter(Boolean).slice(-2).join("/") : "workspace"}
        </span>
        <div className="vx-skills__spacer" />
        {history.length > 0 && (
          <button className="vx-trash__btn" onClick={() => setHistory([])}>
            Clear
          </button>
        )}
        <button className="vx-icon-btn" aria-label="Close terminal" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-term__body" ref={bodyRef}>
        {history.length === 0 && !busy && (
          <div className="vx-term__empty">
            Runs one command at a time in the project the agent is using.
            <div className="vx-term__note">
              No interactive shell — <code>vim</code>, <code>top</code> and anything expecting a
              terminal won&rsquo;t work. Commands are killed after 30s.
            </div>
          </div>
        )}
        {history.map((r, i) => (
          <div key={i} className="vx-term__entry">
            <div className="vx-term__cmd">
              <span className="vx-term__prompt">$</span> {r.command}
              <span className="vx-term__meta">
                {formatMs(r.duration_ms)}
                {r.exit_code !== null && r.exit_code !== 0 && (
                  <span className="vx-term__exit"> exit {r.exit_code}</span>
                )}
                {r.timed_out && (
                  <span className="vx-term__exit">
                    {r.timeout_ms ? ` timed out at ${formatMs(r.timeout_ms)}` : " timed out"}
                  </span>
                )}
              </span>
            </div>
            {r.stdout && <pre className="vx-term__out">{r.stdout}</pre>}
            {r.stderr && <pre className="vx-term__out vx-term__out--err">{r.stderr}</pre>}
            {!r.stdout && !r.stderr && !r.timed_out && (
              <pre className="vx-term__out vx-term__out--quiet">(no output)</pre>
            )}
            {r.truncated && <div className="vx-term__note">Output was clipped at 256KB.</div>}
            {r.timeout_clamped && r.timeout_ms && (
              // A bound that binds says so: otherwise a shortened run reads as a
              // slow command rather than a timeout the daemon refused to grant.
              <div className="vx-term__note">
                Requested timeout exceeded the daemon's cap — capped at {formatMs(r.timeout_ms)}.
              </div>
            )}
          </div>
        ))}
        {busy && <div className="vx-stream__typing">●●●</div>}
        {error && <div className="vx-msg__error">{error}</div>}
      </div>

      <div className="vx-term__composer">
        <span className="vx-term__prompt">$</span>
        <input
          className="vx-term__input"
          value={command}
          placeholder="npm test"
          autoFocus
          spellCheck={false}
          disabled={busy}
          onChange={(e) => {
            recallPos.current = null;
            setCommand(e.target.value);
          }}
          onKeyDown={onKeyDown}
        />
        <button
          className="vx-term__run"
          onClick={run}
          disabled={!command.trim() || busy}
          aria-label="Run command"
          title="Run (Enter)"
        >
          <CornerDownLeft size={13} />
        </button>
      </div>
    </div>
  );
}
