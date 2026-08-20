/**
 * SecurityReviewPanel.
 *
 * On-demand, provider-agnostic security review. The scope is whatever the user
 * names — blank for the whole workspace, a folder, a glob, or a single file —
 * resolved by `security_review_targets`; each resolved file is then one
 * `security_review_file` call returning standard `Finding` records (the same
 * schema clippy/eslint/semgrep produce). This is the user-invoked entry point;
 * the daemon's opt-in file-watcher loop calls the same backend for the
 * always-on path.
 *
 * Acting on a finding stays an explicit user step. "Fix with AI" writes the
 * change request into the chat composer via `vibecoder:inject-context` — the
 * user still reads it and presses send. Nothing here edits a file.
 */
import { useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { parseProviderSelection } from "../hooks/useModelRegistry";
import { getSelectedEffort } from "../utils/effort";
import { FixWithAIButton } from "./FixWithAIButton";
import type { FixItem } from "../lib/fixWithAI";

interface SecurityReviewPanelProps {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

interface SecurityFinding {
  severity: string;
  message: string;
  file: string | null;
  line: number | null;
  suggestion: string | null;
}

interface SecurityReviewTargets {
  files: string[];
  matched: number;
  limit: number;
}

/** A file the sweep could not review, kept so a partial run never reads as a clean one. */
interface ReviewFailure {
  file: string;
  error: string;
}

/**
 * Run state as one value rather than parallel loading/error/data flags, so the
 * render has a single thing to switch on.
 */
type RunState =
  | { kind: "idle" }
  | { kind: "resolving" }
  | { kind: "reviewing"; done: number; total: number; current: string }
  | { kind: "finished"; reviewed: number; matched: number; limit: number; stopped: boolean };

const SEVERITY_COLOR: Record<string, string> = {
  critical: "#e5484d",
  error: "#e5734d",
  warning: "#e2a64d",
  info: "var(--text-secondary)",
};

const SEVERITY_RANK: Record<string, number> = { critical: 0, error: 1, warning: 2, info: 3 };

/** Where a finding points, as a human-readable location. */
function locationOf(f: SecurityFinding): string {
  if (!f.file) return "the reviewed file";
  return f.line != null ? `${f.file}:${f.line}` : f.file;
}

/** One finding as the shared hand-off carries it. */
function toFixItem(f: SecurityFinding): FixItem {
  return {
    file: f.file,
    line: f.line,
    severity: f.severity,
    message: f.message,
    suggestion: f.suggestion,
  };
}

export function SecurityReviewPanel({ workspacePath, provider, onOpenFile }: SecurityReviewPanelProps) {
  const [pattern, setPattern] = useState("");
  const [findings, setFindings] = useState<SecurityFinding[]>([]);
  const [failures, setFailures] = useState<ReviewFailure[]>([]);
  const [run, setRun] = useState<RunState>({ kind: "idle" });
  const [error, setError] = useState<string | null>(null);
  // Bumped by each run so the hand-off buttons stop claiming the previous
  // run's findings were sent.
  const [runId, setRunId] = useState(0);
  const stopRef = useRef(false);

  // The toolbar hands down a display name ("Ollama (devstral-2)"); the command
  // needs the id and the model separately. Looking the prop up in
  // `PROVIDER_DEFAULT_MODEL` directly missed, so this panel sent an empty model
  // and the command refused it as "no model selected".
  const selection = useMemo(() => parseProviderSelection(provider ?? ""), [provider]);

  const sortedFindings = useMemo(
    () =>
      [...findings].sort(
        (a, b) => (SEVERITY_RANK[a.severity] ?? 9) - (SEVERITY_RANK[b.severity] ?? 9)
      ),
    [findings]
  );

  const runReview = async () => {
    if (!workspacePath) {
      setError("Open a workspace folder first.");
      return;
    }
    if (!selection.provider || !selection.model) {
      setError("Select a provider in the toolbar first.");
      return;
    }
    stopRef.current = false;
    setError(null);
    setFindings([]);
    setFailures([]);
    setRunId((n) => n + 1);
    setRun({ kind: "resolving" });

    let targets: SecurityReviewTargets;
    try {
      targets = await invoke<SecurityReviewTargets>("security_review_targets", {
        workspace: workspacePath,
        pattern: pattern.trim() || null,
      });
    } catch (e) {
      setError(String(e));
      setRun({ kind: "idle" });
      return;
    }

    if (targets.files.length === 0) {
      setRun({ kind: "finished", reviewed: 0, matched: 0, limit: targets.limit, stopped: false });
      return;
    }

    const effort = getSelectedEffort();
    let reviewed = 0;
    for (const file of targets.files) {
      if (stopRef.current) break;
      setRun({ kind: "reviewing", done: reviewed, total: targets.files.length, current: file });
      try {
        const contents = await invoke<string>("read_file", { path: file });
        const result = await invoke<SecurityFinding[]>("security_review_file", {
          provider: selection.provider,
          model: selection.model,
          file,
          contents,
          effort,
        });
        // The backend fills `file` from what it was given, but a provider that
        // returns a bare finding would otherwise lose the path the fix needs.
        setFindings(prev => [...prev, ...result.map(f => ({ ...f, file: f.file ?? file }))]);
      } catch (e) {
        setFailures(prev => [...prev, { file, error: String(e) }]);
      }
      reviewed += 1;
    }

    setRun({
      kind: "finished",
      reviewed,
      matched: targets.matched,
      limit: targets.limit,
      stopped: stopRef.current,
    });
  };

  const busy = run.kind === "resolving" || run.kind === "reviewing";

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
        Opt-in security review of the workspace. Leave the box empty to review everything,
        or narrow it to a folder, a glob (<code>src/*</code>, <code>*.rs</code>), or one file.
        Findings use the standard review schema; nothing is edited for you — "Fix with AI"
        writes the change request into chat for you to send.
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <input
          type="text"
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && !busy) runReview(); }}
          placeholder={workspacePath ? "empty = whole workspace · src/ · src/* · *.rs · path/to/file.rs" : "open a workspace folder first"}
          disabled={busy}
          style={{ flex: 1, padding: 8, fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", boxSizing: "border-box" }}
        />
        <button
          onClick={busy ? () => { stopRef.current = true; } : runReview}
          disabled={!workspacePath}
          style={{ padding: "6px 14px", fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "none", background: busy ? "var(--bg-secondary)" : "var(--accent-color)", color: busy ? "var(--text-primary)" : "#fff", cursor: workspacePath ? "pointer" : "default", opacity: workspacePath ? 1 : 0.6, whiteSpace: "nowrap" }}
        >
          {busy ? "Stop" : "Review"}
        </button>
      </div>

      {error && <div style={{ fontSize: "var(--font-size-sm)", color: SEVERITY_COLOR.critical }}>{error}</div>}

      {run.kind === "resolving" && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Finding files…</div>
      )}

      {run.kind === "reviewing" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            Reviewing {run.done + 1} of {run.total} — {run.current}
          </div>
          <div style={{ height: 3, background: "var(--bg-secondary)", borderRadius: 2, overflow: "hidden" }}>
            <div style={{ height: "100%", width: `${(run.done / run.total) * 100}%`, background: "var(--accent-color)", transition: "width 0.2s" }} />
          </div>
        </div>
      )}

      {run.kind === "finished" && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {run.matched === 0
            ? "No reviewable files matched."
            : `Reviewed ${run.reviewed} file${run.reviewed === 1 ? "" : "s"}${run.stopped ? " before you stopped it" : ""}.`}
          {run.matched > run.reviewed && !run.stopped && (
            <span style={{ color: SEVERITY_COLOR.warning }}>
              {" "}{run.matched} files matched — the run is capped at {run.limit}, so{" "}
              {run.matched - run.reviewed} were not reviewed. Narrow the pattern to cover them.
            </span>
          )}
        </div>
      )}

      {failures.length > 0 && (
        <div style={{ fontSize: "var(--font-size-sm)", color: SEVERITY_COLOR.warning }}>
          {failures.length} file{failures.length === 1 ? "" : "s"} could not be reviewed:{" "}
          {failures.slice(0, 3).map(f => f.file).join(", ")}
          {failures.length > 3 ? ` and ${failures.length - 3} more` : ""} — {failures[0].error}
        </div>
      )}

      {run.kind === "finished" && findings.length === 0 && run.reviewed > 0 && failures.length === 0 && (
        <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-success, #4caf50)" }}>No security findings.</div>
      )}

      {findings.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            {findings.length} finding{findings.length === 1 ? "" : "s"}
          </span>
          <FixWithAIButton
            items={sortedFindings.map(toFixItem)}
            source="security review"
            resetKey={runId}
            label={`Fix all ${findings.length} with AI`}
            style={{ padding: "4px 10px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-sm)", borderColor: "var(--accent-color)", color: "var(--accent-color)" }}
          />
        </div>
      )}

      {sortedFindings.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {sortedFindings.map((f, i) => {
            const key = `${f.file ?? ""}:${f.line ?? ""}:${i}`;
            return (
              <div
                key={key}
                style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${SEVERITY_COLOR[f.severity] ?? "var(--text-secondary)"}` }}
              >
                <div style={{ display: "flex", gap: 8, alignItems: "baseline", flexWrap: "wrap" }}>
                  <span style={{ fontSize: "var(--font-size-xs)", textTransform: "uppercase", fontWeight: 700, color: SEVERITY_COLOR[f.severity] ?? "var(--text-secondary)" }}>{f.severity}</span>
                  {f.file && (
                    <span
                      onClick={() => onOpenFile?.(f.file as string, f.line ?? undefined)}
                      style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)", cursor: onOpenFile ? "pointer" : "default", textDecoration: onOpenFile ? "underline" : "none" }}
                    >
                      {locationOf(f)}
                    </span>
                  )}
                </div>
                <div style={{ fontSize: "var(--font-size-md)", marginTop: 4 }}>{f.message}</div>
                {f.suggestion && (
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>↳ {f.suggestion}</div>
                )}
                <div style={{ marginTop: 8 }}>
                  <FixWithAIButton
                    items={[toFixItem(f)]}
                    source="security review"
                    resetKey={runId}
                    style={{ padding: "3px 10px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-sm)", borderColor: "var(--accent-color)", color: "var(--accent-color)" }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default SecurityReviewPanel;
