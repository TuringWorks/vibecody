/**
 * WorkspaceReviewTab — one engine for red / blue / purple team review of the
 * project's own code and content.
 *
 * All three teams do the identical thing mechanically: resolve a scope to a
 * file list, then run one provider-agnostic LLM call per file, keeping partial
 * findings and letting the user stop. What differs is the *question* — attack,
 * defence, or coverage — and the words on screen. That difference is the
 * `config` prop; everything else lives here so the three panels cannot drift.
 *
 * The backend commands are named by `kind`: `${kind}_workspace_targets`,
 * `${kind}_file`, `${kind}_save_session`. The finding shape is shared
 * (`WorkspaceFinding`); each team fills its fields with its own meaning and
 * labels them through `config`.
 *
 * Honesty, inherited from the red-team pass that came first: no fabricated CVSS
 * (severity is what the model judged), nothing is marked "confirmed" from a
 * static read, and a capped or partially-failed run says so rather than reading
 * as a clean sweep. Nothing is edited — "Fix with AI" writes the request into
 * chat.
 */
import { useCallback, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CircleAlert, AlertTriangle, Info, Loader2 } from "lucide-react";
import { errorMessage } from "../utils/errorMessage";
import { parseProviderSelection } from "../hooks/useModelRegistry";
import { getSelectedEffort } from "../utils/effort";
import { FixWithAIButton } from "./FixWithAIButton";
import type { FixItem } from "../lib/fixWithAI";

/** A finding from any of the three teams. Fields carry each team's meaning. */
export interface WorkspaceFinding {
  id: string;
  /** Attack class / control area / attack — labelled per team. */
  attack_vector: string;
  /** 0 when no CVSS was computed (always, for a static review) — never shown. */
  cvss_score: number;
  severity: string;
  location: string;
  title: string;
  /** The finding body: the attack, the gap, or the coverage judgement. */
  description: string;
  /** Evidence — an attack PoC, or a detection idea. Labelled per team; hidden when empty. */
  poc: string;
  /** The fix / hardening / coverage recommendation. Hidden when empty. */
  remediation: string;
  source_file: string | null;
  source_line: number | null;
  confirmed: boolean;
  /** Present on URL findings only; workspace findings leave it empty. */
  url?: string;
}

interface WorkspaceSession {
  id: string;
  target_url: string;
  current_stage: string;
  findings: WorkspaceFinding[];
  started_at: string;
  finished_at: string | null;
}

interface Targets {
  files: string[];
  matched: number;
  limit: number;
}

interface ReviewFailure {
  file: string;
  error: string;
}

type Run =
  | { kind: "idle" }
  | { kind: "resolving" }
  | { kind: "reviewing"; done: number; total: number; current: string }
  | { kind: "finished"; reviewed: number; matched: number; limit: number; stopped: boolean };

/** The words and command names that make this tab red, blue, or purple. */
export interface WorkspaceReviewConfig {
  /** Command prefix and session directory: "redteam" | "blueteam" | "purpleteam". */
  kind: string;
  /** Verb on the run button, e.g. "Run Red Team". */
  runLabel: string;
  /** One-line description above the scope box. */
  intro: React.ReactNode;
  /** The noun for a finding, plural — "attacks", "defensive gaps", "coverage gaps". */
  findingNoun: string;
  /** Label for the evidence block (`poc`), e.g. "PoC" / "Detection". */
  evidenceLabel: string;
  /** Label for the fix block (`remediation`), e.g. "Remediation" / "Hardening" / "Close the gap". */
  fixLabel: string;
  /** Label for the category chip (`attack_vector`), e.g. "Vector" / "Control" / "Attack". */
  vectorLabel: string;
  /** `source` and `instructions` for the Fix-with-AI hand-off. */
  fixSource: string;
  fixInstructions: string[];
  /** Empty-state body. */
  emptyBody: React.ReactNode;
}

interface Props {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
  config: WorkspaceReviewConfig;
}

function severityColor(sev: string): string {
  switch (sev.toLowerCase()) {
    case "critical": return "var(--error-color)";
    case "high": return "var(--accent-gold)";
    case "medium": return "var(--warning-color)";
    case "low": return "var(--accent-blue)";
    default: return "var(--text-secondary)";
  }
}

function severityIcon(sev: string): React.ReactNode {
  switch (sev.toLowerCase()) {
    case "critical": return <CircleAlert size={14} strokeWidth={1.5} style={{ color: "var(--error-color)" }} />;
    case "high": return <CircleAlert size={14} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />;
    case "medium": return <AlertTriangle size={14} strokeWidth={1.5} style={{ color: "var(--warning-color)" }} />;
    case "low": return <Info size={14} strokeWidth={1.5} style={{ color: "var(--accent-blue)" }} />;
    default: return <Info size={14} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />;
  }
}

export function WorkspaceReviewTab({ workspacePath, provider, onOpenFile, config }: Props) {
  const [scopePattern, setScopePattern] = useState("");
  const [run, setRun] = useState<Run>({ kind: "idle" });
  const [findings, setFindings] = useState<WorkspaceFinding[]>([]);
  const [failures, setFailures] = useState<ReviewFailure[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  // Bumped each run so the Fix-with-AI button stops claiming the previous run's
  // findings were already sent.
  const [runId, setRunId] = useState(0);
  const stopRef = useRef(false);
  const taskRef = useRef(0);

  // The toolbar hands a display name ("Ollama (devstral-2)"); the command needs
  // the id and model split out.
  const selection = useMemo(() => parseProviderSelection(provider ?? ""), [provider]);

  const busy = run.kind === "resolving" || run.kind === "reviewing";

  const runReview = useCallback(async () => {
    if (!workspacePath) {
      setError("Open a workspace folder first.");
      return;
    }
    if (!selection.provider || !selection.model) {
      setError("Select a provider and model in the toolbar first.");
      return;
    }
    stopRef.current = false;
    const thisTask = ++taskRef.current;
    setError(null);
    setFindings([]);
    setFailures([]);
    setExpanded(null);
    setRunId((n) => n + 1);
    setRun({ kind: "resolving" });

    let targets: Targets;
    try {
      targets = await invoke<Targets>(`${config.kind}_workspace_targets`, {
        workspace: workspacePath,
        pattern: scopePattern.trim() || null,
      });
    } catch (e) {
      setError(errorMessage(e) || "Could not resolve targets");
      setRun({ kind: "idle" });
      return;
    }
    if (thisTask !== taskRef.current) return;

    if (targets.files.length === 0) {
      setRun({ kind: "finished", reviewed: 0, matched: 0, limit: targets.limit, stopped: false });
      return;
    }

    const effort = getSelectedEffort();
    const collected: WorkspaceFinding[] = [];
    let reviewed = 0;
    for (const file of targets.files) {
      if (stopRef.current || thisTask !== taskRef.current) break;
      setRun({ kind: "reviewing", done: reviewed, total: targets.files.length, current: file });
      try {
        const contents = await invoke<string>("read_file", { path: file });
        const result = await invoke<WorkspaceFinding[]>(`${config.kind}_file`, {
          provider: selection.provider,
          model: selection.model,
          file,
          contents,
          effort,
        });
        if (result.length > 0) {
          collected.push(...result);
          setFindings((prev) => [...prev, ...result]);
        }
      } catch (e) {
        setFailures((prev) => [...prev, { file, error: errorMessage(e) || "review failed" }]);
      }
      reviewed += 1;
    }
    if (thisTask !== taskRef.current) return;

    setRun({
      kind: "finished",
      reviewed,
      matched: targets.matched,
      limit: targets.limit,
      stopped: stopRef.current,
    });

    // Persist so the run can be revisited / exported. A save failure loses the
    // history row, not the findings on screen.
    try {
      await invoke(`${config.kind}_save_session`, {
        session: {
          id: `${config.kind}-ws-${Date.now()}`,
          target_url: scopePattern.trim() || workspacePath,
          current_stage: stopRef.current ? "Stopped" : "Report",
          findings: collected,
          started_at: new Date().toISOString(),
          finished_at: new Date().toISOString(),
        } satisfies WorkspaceSession,
      });
    } catch {
      /* history only */
    }
  }, [workspacePath, selection, scopePattern, config]);

  const fixItems = useMemo<FixItem[]>(
    () =>
      findings.map((f) => ({
        file: f.source_file,
        line: f.source_line,
        severity: f.severity,
        title: f.title,
        message: f.description || f.title,
        suggestion: f.remediation || null,
        notes: [
          f.attack_vector ? `${config.vectorLabel}: ${f.attack_vector}` : "",
          f.poc ? `${config.evidenceLabel}: ${f.poc}` : "",
        ].filter(Boolean),
      })),
    [findings, config],
  );

  const openFinding = (f: WorkspaceFinding) => {
    if (onOpenFile && workspacePath && f.source_file) {
      const full = f.source_file.startsWith("/") ? f.source_file : `${workspacePath}/${f.source_file}`;
      onOpenFile(full, f.source_line ?? undefined);
    }
  };

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
        {config.intro}
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <input
          value={scopePattern}
          onChange={(e) => setScopePattern(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && !busy) runReview(); }}
          placeholder={workspacePath ? "empty = whole workspace · src/ · src/* · *.py · path/to/file.ts" : "open a workspace folder first"}
          disabled={busy || !workspacePath}
          className="panel-input"
          style={{ flex: 1 }}
        />
        {busy ? (
          <button onClick={() => { stopRef.current = true; }} className="panel-btn panel-btn-danger">Stop</button>
        ) : (
          <button onClick={runReview} disabled={!workspacePath} className="panel-btn panel-btn-primary">{config.runLabel}</button>
        )}
      </div>

      {!workspacePath && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}>
          Open a workspace folder to review its code and content.
        </div>
      )}
      {error && <div className="panel-error">{error}</div>}

      {run.kind === "reviewing" && (
        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          <Loader2 size={14} style={{ animation: "spin 1s linear infinite" }} />
          <span>Reviewing {run.done + 1} / {run.total} — <code style={{ fontSize: "var(--font-size-xs)" }}>{run.current}</code></span>
        </div>
      )}
      {run.kind === "finished" && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {run.stopped ? "Stopped" : "Done"} — reviewed {run.reviewed} file(s)
          {run.matched > run.limit && `; scope had ${run.matched}, capped at ${run.limit}`}
          {failures.length > 0 && `; ${failures.length} could not be read`}.
        </div>
      )}

      {/* Summary + hand-off */}
      {run.kind === "finished" && findings.length > 0 && (
        <div style={{ display: "flex", gap: 12, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", alignItems: "center", fontSize: "var(--font-size-base)" }}>
          {(["critical", "high", "medium", "low"] as const).map((s) => {
            const n = findings.filter((f) => f.severity.toLowerCase() === s).length;
            return n > 0 ? (
              <span key={s} style={{ color: severityColor(s), fontWeight: 600 }}>
                {n} {s[0].toUpperCase() + s.slice(1)}
              </span>
            ) : null;
          })}
          <span style={{ flex: 1 }} />
          <FixWithAIButton
            items={fixItems}
            source={config.fixSource}
            resetKey={runId}
            instructions={config.fixInstructions}
          />
        </div>
      )}

      {/* Findings */}
      {findings.length > 0 && (
        <div>
          <h4 style={{ margin: "0 0 8px", fontSize: "var(--font-size-md)" }}>
            {findings.length} {config.findingNoun}
          </h4>
          {findings.map((f) => (
            <div
              role="button"
              tabIndex={0}
              key={f.id}
              onClick={() => setExpanded(expanded === f.id ? null : f.id)}
              style={{
                marginBottom: 8, padding: "8px 12px", borderRadius: "var(--radius-xs-plus)",
                background: "var(--bg-secondary)", borderLeft: `3px solid ${severityColor(f.severity)}`, cursor: "pointer",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ display: "inline-flex" }}>{severityIcon(f.severity)}</span>
                <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, flex: 1 }}>{f.title}</span>
                {/* Severity, never a fabricated CVSS. */}
                <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, background: severityColor(f.severity), color: "var(--btn-primary-fg, #fff)", fontWeight: 600 }}>
                  {f.cvss_score > 0 ? `CVSS ${f.cvss_score.toFixed(1)}` : f.severity.toUpperCase()}
                </span>
              </div>

              {f.source_file && (
                <div style={{ marginTop: 4 }}>
                  <span
                    onClick={(e) => { e.stopPropagation(); openFinding(f); }}
                    style={{
                      fontSize: "var(--font-size-xs)", color: "var(--accent-blue)", fontFamily: "var(--font-mono)",
                      cursor: onOpenFile ? "pointer" : "default", textDecoration: onOpenFile ? "underline" : "none",
                    }}
                    title="Open in editor"
                  >
                    {f.source_file}{f.source_line ? `:${f.source_line}` : ""}
                  </span>
                </div>
              )}

              {expanded === f.id && (
                <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", lineHeight: 1.6 }}>
                  {f.attack_vector && <div><strong>{config.vectorLabel}:</strong> {f.attack_vector}</div>}
                  <div style={{ marginTop: 4 }}>{f.description}</div>
                  {f.poc && (
                    <div style={{ marginTop: 4 }}>
                      <strong>{config.evidenceLabel}:</strong>
                      <pre style={{ margin: "4px 0", padding: 8, background: "var(--bg-primary)", borderRadius: 3, fontSize: "var(--font-size-sm)", overflow: "auto", whiteSpace: "pre-wrap" }}>
                        {f.poc}
                      </pre>
                    </div>
                  )}
                  {f.remediation && (
                    <div style={{ marginTop: 4, color: "var(--success-color)" }}>
                      <strong>{config.fixLabel}:</strong> {f.remediation}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Empty state */}
      {run.kind === "finished" && findings.length === 0 && (
        <div style={{ textAlign: "center", padding: "24px 20px", color: "var(--text-secondary)" }}>
          Reviewed {run.reviewed} file(s) — nothing flagged.
        </div>
      )}
      {run.kind === "idle" && !error && (
        <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-secondary)" }}>
          <CircleAlert size={32} strokeWidth={1.5} style={{ color: "var(--text-secondary)", marginBottom: 12 }} />
          <p style={{ fontSize: "var(--font-size-base)" }}>{config.emptyBody}</p>
          <p style={{ fontSize: "var(--font-size-sm)", marginTop: 12, fontStyle: "italic" }}>
            Only review code and applications you own and control.
          </p>
        </div>
      )}

      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
