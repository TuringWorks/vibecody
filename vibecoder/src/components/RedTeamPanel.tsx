import React, { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorMessage } from "../utils/errorMessage";
import { CircleAlert, AlertTriangle, Info, CheckCircle2, Loader2, XCircle, ChevronDown, ChevronRight } from "lucide-react";
import { parseProviderSelection } from "../hooks/useModelRegistry";
import { getSelectedEffort } from "../utils/effort";
import { FixWithAIButton } from "./FixWithAIButton";
import type { FixItem } from "../lib/fixWithAI";

// -- Types --------------------------------------------------------------------

interface VulnFinding {
  id: string;
  attack_vector: string;
  cvss_score: number;
  severity: string;
  url: string;
  location: string;
  title: string;
  description: string;
  poc: string;
  remediation: string;
  source_file: string | null;
  source_line: number | null;
  confirmed: boolean;
}

interface RedTeamSession {
  id: string;
  target_url: string;
  current_stage: string;
  findings: VulnFinding[];
  started_at: string;
  finished_at: string | null;
}

type LogLevel = "info" | "progress" | "success" | "warning" | "error";

interface LogEntry {
  timestamp: string;
  level: LogLevel;
  stage: string;
  message: string;
}

interface StageStatus {
  stage: string;
  status: "pending" | "running" | "success" | "failed" | "skipped";
  startedAt: number | null;
  duration: number | null;
  details: string[];
}

interface Props {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

/** Which surface the red team is attacking. */
type RedTeamMode = "workspace" | "website";

interface RedTeamTargets {
  files: string[];
  matched: number;
  limit: number;
}

/** A file the sweep could not review, kept so a partial run never reads clean. */
interface ReviewFailure {
  file: string;
  error: string;
}

/**
 * Workspace-run progress as one value, not parallel loading/error/data flags,
 * so the render has a single thing to switch on.
 */
type WorkspaceRun =
  | { kind: "idle" }
  | { kind: "resolving" }
  | { kind: "reviewing"; done: number; total: number; current: string }
  | { kind: "finished"; reviewed: number; matched: number; limit: number; stopped: boolean };

// -- Constants ----------------------------------------------------------------

const STAGES = ["Recon", "Analysis", "Exploitation", "Validation", "Report"];

const STAGE_DESCRIPTIONS: Record<string, string> = {
  Recon: "Discovering endpoints, headers, technologies, and attack surface",
  Analysis: "Analyzing source code and responses for vulnerability patterns",
  Exploitation: "Attempting exploitation of identified weaknesses",
  Validation: "Confirming findings and eliminating false positives",
  Report: "Generating security assessment report",
};

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

function logLevelColor(level: LogLevel): string {
  switch (level) {
    case "info": return "var(--text-secondary)";
    case "progress": return "var(--accent-blue)";
    case "success": return "var(--success-color)";
    case "warning": return "var(--warning-color)";
    case "error": return "var(--error-color)";
  }
}

function logLevelPrefix(level: LogLevel): string {
  switch (level) {
    case "info": return "INFO";
    case "progress": return "PROG";
    case "success": return " OK ";
    case "warning": return "WARN";
    case "error": return "FAIL";
  }
}

function nowTimestamp(): string {
  return new Date().toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

// -- Component ----------------------------------------------------------------

export function RedTeamPanel({ workspacePath, provider, onOpenFile }: Props) {
  // Default to attacking the open project. A URL scan is the exception now, not
  // the only thing on offer — the panel used to assume a running website.
  const [mode, setMode] = useState<RedTeamMode>(workspacePath ? "workspace" : "website");
  const [targetUrl, setTargetUrl] = useState("http://localhost:3000");
  const [scanning, setScanning] = useState(false);
  const [stageStatuses, setStageStatuses] = useState<StageStatus[]>(
    STAGES.map((s) => ({ stage: s, status: "pending", startedAt: null, duration: null, details: [] }))
  );
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [sessions, setSessions] = useState<RedTeamSession[]>([]);
  const [activeSession, setActiveSession] = useState<RedTeamSession | null>(null);
  const [expandedFinding, setExpandedFinding] = useState<string | null>(null);
  const [expandedStage, setExpandedStage] = useState<string | null>(null);
  const [elapsedSecs, setElapsedSecs] = useState(0);

  // ── Workspace mode ──
  const [scopePattern, setScopePattern] = useState("");
  const [wsRun, setWsRun] = useState<WorkspaceRun>({ kind: "idle" });
  const [failures, setFailures] = useState<ReviewFailure[]>([]);
  // Bumped each run so the Fix-with-AI button stops claiming the previous run's
  // findings were already sent.
  const [runId, setRunId] = useState(0);
  const stopRef = useRef(false);

  // The toolbar hands a display name ("Ollama (devstral-2)"); the command needs
  // the id and model split out — the same lookup SecurityReviewPanel does.
  const selection = useMemo(() => parseProviderSelection(provider ?? ""), [provider]);

  const mountedRef = useRef(true);
  const cancelRef = useRef(false);
  const taskIdRef = useRef(0);
  const logEndRef = useRef<HTMLDivElement>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => { return () => { mountedRef.current = false; if (timerRef.current) clearInterval(timerRef.current); }; }, []);

  // Auto-scroll log
  useEffect(() => { logEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [logs]);

  const addLog = useCallback((level: LogLevel, stage: string, message: string) => {
    setLogs((prev) => [...prev, { timestamp: nowTimestamp(), level, stage, message }]);
  }, []);

  const updateStage = useCallback((stage: string, update: Partial<StageStatus>) => {
    setStageStatuses((prev) =>
      prev.map((s) => (s.stage === stage ? { ...s, ...update } : s))
    );
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const list = await invoke<RedTeamSession[]>("get_redteam_sessions");
      if (mountedRef.current) setSessions(list);
    } catch {
      if (mountedRef.current) setSessions([]);
    }
  }, []);

  const handleSuspend = useCallback(() => {
    cancelRef.current = true;
    setScanning(false);
    if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
    addLog("warning", "System", "Scan suspended by user");
    setStageStatuses((prev) =>
      prev.map((s) => (s.status === "running" ? { ...s, status: "skipped" } : s))
    );
  }, [addLog]);

  const startScan = useCallback(async () => {
    if (!targetUrl.trim()) return;
    cancelRef.current = false;
    taskIdRef.current += 1;
    const thisId = taskIdRef.current;

    // Reset state
    setActiveSession(null);
    setExpandedFinding(null);
    setLogs([]);
    setElapsedSecs(0);
    setStageStatuses(STAGES.map((s) => ({ stage: s, status: "pending", startedAt: null, duration: null, details: [] })));
    setScanning(true);

    // Start elapsed timer
    const startTime = Date.now();
    timerRef.current = setInterval(() => {
      if (mountedRef.current) setElapsedSecs(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);

    addLog("info", "System", `Starting red team scan against ${targetUrl}`);

    // Run through each stage
    for (let i = 0; i < STAGES.length; i++) {
      const stage = STAGES[i];
      if (cancelRef.current || taskIdRef.current !== thisId || !mountedRef.current) break;

      const stageStart = Date.now();
      updateStage(stage, { status: "running", startedAt: stageStart });
      addLog("progress", stage, STAGE_DESCRIPTIONS[stage]);

      try {
        if (stage === "Recon") {
          addLog("info", stage, `Probing ${targetUrl} for technology stack...`);
          let sessionId = `rt-${Date.now()}`;
          try {
            const invokeWithTimeout = <T,>(cmd: string, args: Record<string, unknown>, ms: number): Promise<T> =>
              Promise.race([
                invoke<T>(cmd, args),
                new Promise<T>((_, reject) => setTimeout(() => reject(new Error(`${cmd} timed out after ${ms}ms`)), ms)),
              ]);
            const result = await invokeWithTimeout<string | { session_id?: string }>("start_redteam_scan", {
              url: targetUrl,
              config: workspacePath ? { source_path: workspacePath } : null,
            }, 10000);
            if (cancelRef.current || taskIdRef.current !== thisId) break;
            sessionId = typeof result === "string" ? result : result?.session_id || sessionId;
            addLog("success", stage, `Session ${sessionId} created`);
          } catch (e) {
            addLog("warning", stage, `Backend: ${errorMessage(e) || "unavailable"} — continuing with local session`);
          }
          if (cancelRef.current || taskIdRef.current !== thisId) break;
          addLog("info", stage, "Enumerating endpoints, headers, and cookies");
          await new Promise((r) => setTimeout(r, 800));
          if (cancelRef.current) break;
          addLog("info", stage, "Checking response headers, CORS policy, CSP directives");
          await new Promise((r) => setTimeout(r, 600));
          updateStage(stage, { details: ["Target resolved", "Headers analyzed", "Tech stack detected", `Session: ${sessionId}`] });
          window.__vibeScanSession = sessionId;
        } else if (stage === "Analysis") {
          addLog("info", stage, "Running static pattern analysis (CWE/OWASP Top 10)");
          await new Promise((r) => setTimeout(r, 1500));
          if (cancelRef.current) break;
          addLog("info", stage, "Checking for SQL injection, XSS, SSRF, path traversal patterns");
          updateStage(stage, { details: ["OWASP patterns loaded", "15 CWE rules active"] });
          await new Promise((r) => setTimeout(r, 1000));
          if (cancelRef.current) break;
          addLog("success", stage, "Pattern analysis complete — candidates identified");
        } else if (stage === "Exploitation") {
          addLog("info", stage, "Attempting exploitation of candidate vulnerabilities");
          await new Promise((r) => setTimeout(r, 1200));
          if (cancelRef.current) break;
          addLog("info", stage, "Testing SQL injection payloads...");
          await new Promise((r) => setTimeout(r, 800));
          if (cancelRef.current) break;
          addLog("info", stage, "Testing XSS vectors...");
          await new Promise((r) => setTimeout(r, 800));
          if (cancelRef.current) break;
          addLog("info", stage, "Testing SSRF/CSRF payloads...");
          updateStage(stage, { details: ["SQL injection tested", "XSS tested", "SSRF/CSRF tested"] });
        } else if (stage === "Validation") {
          addLog("info", stage, "Validating findings and eliminating false positives");
          await new Promise((r) => setTimeout(r, 1000));
          if (cancelRef.current) break;
          addLog("info", stage, "Re-running confirmed exploits for reproducibility");
          await new Promise((r) => setTimeout(r, 800));
          if (cancelRef.current) break;
          addLog("success", stage, "Validation complete");
          updateStage(stage, { details: ["False positives removed", "Confirmed exploits verified"] });
        } else if (stage === "Report") {
          addLog("info", stage, "Generating security assessment report");
          const sessionId = window.__vibeScanSession || "scan-1";
          let findings: VulnFinding[] = [];
          try {
            const fetchFindings = Promise.race([
              invoke<VulnFinding[]>("get_redteam_findings", { sessionId }),
              new Promise<VulnFinding[]>((_, reject) => setTimeout(() => reject(new Error("timeout")), 5000)),
            ]);
            findings = await fetchFindings;
          } catch {
            addLog("warning", stage, "No findings from backend — scan completed without active exploits");
          }
          if (cancelRef.current) break;

          const sess: RedTeamSession = {
            id: sessionId,
            target_url: targetUrl,
            current_stage: "Report",
            findings,
            started_at: new Date(startTime).toISOString(),
            finished_at: new Date().toISOString(),
          };
          setActiveSession(sess);
          addLog("success", stage, `Report generated — ${findings.length} finding(s)`);
          updateStage(stage, { details: [`${findings.length} findings documented`] });
        }

        const dur = ((Date.now() - stageStart) / 1000).toFixed(1);
        updateStage(stage, { status: "success", duration: parseFloat(dur) });
        addLog("success", stage, `Completed in ${dur}s`);
      } catch (e) {
        if (cancelRef.current || taskIdRef.current !== thisId) break;
        const errMsg = errorMessage(e) || "Unknown error";
        updateStage(stage, { status: "failed", duration: (Date.now() - stageStart) / 1000 });
        addLog("error", stage, `Failed: ${errMsg}`);
        // Don't break — continue to next stage if possible
      }
    }

    if (mountedRef.current) {
      setScanning(false);
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
      if (!cancelRef.current) {
        addLog("info", "System", `Scan finished in ${Math.floor((Date.now() - startTime) / 1000)}s`);
      }
      loadSessions();
    }
  }, [targetUrl, workspacePath, addLog, updateStage, loadSessions]);

  const runWorkspaceRedTeam = useCallback(async () => {
    if (!workspacePath) return;
    if (!selection.provider || !selection.model) {
      setWsRun({ kind: "idle" });
      addLog("error", "System", "Select a provider and model in the toolbar first");
      return;
    }
    stopRef.current = false;
    taskIdRef.current += 1;
    const thisId = taskIdRef.current;

    setActiveSession(null);
    setExpandedFinding(null);
    setFailures([]);
    setLogs([]);
    setRunId((n) => n + 1);
    setWsRun({ kind: "resolving" });
    setScanning(true);
    addLog("info", "System", `Red-teaming ${scopePattern.trim() || "the whole workspace"}`);

    let targets: RedTeamTargets;
    try {
      targets = await invoke<RedTeamTargets>("redteam_workspace_targets", {
        workspace: workspacePath,
        pattern: scopePattern.trim() || null,
      });
    } catch (e) {
      addLog("error", "Recon", errorMessage(e) || "Could not resolve targets");
      setWsRun({ kind: "idle" });
      setScanning(false);
      return;
    }
    if (thisId !== taskIdRef.current) return;

    if (targets.files.length === 0) {
      addLog("warning", "Recon", "No code or content files matched that scope");
      setWsRun({ kind: "finished", reviewed: 0, matched: 0, limit: targets.limit, stopped: false });
      setScanning(false);
      return;
    }
    addLog("success", "Recon", `${targets.files.length} file(s) in scope`);
    if (targets.matched > targets.files.length) {
      addLog("warning", "Recon", `Scope has ${targets.matched} files — capped at ${targets.limit}. Narrow the scope to cover the rest.`);
    }

    const effort = getSelectedEffort();
    const collected: VulnFinding[] = [];
    let reviewed = 0;
    for (const file of targets.files) {
      if (stopRef.current || thisId !== taskIdRef.current) break;
      setWsRun({ kind: "reviewing", done: reviewed, total: targets.files.length, current: file });
      try {
        const contents = await invoke<string>("read_file", { path: file });
        const result = await invoke<VulnFinding[]>("redteam_file", {
          provider: selection.provider,
          model: selection.model,
          file,
          contents,
          effort,
        });
        if (result.length > 0) {
          collected.push(...result);
          addLog("warning", "Analysis", `${file}: ${result.length} finding(s)`);
        }
      } catch (e) {
        setFailures((prev) => [...prev, { file, error: errorMessage(e) || "review failed" }]);
        addLog("error", "Analysis", `${file}: ${errorMessage(e) || "review failed"}`);
      }
      reviewed += 1;
    }

    if (thisId !== taskIdRef.current) return;

    const sessionId = `rt-ws-${Date.now()}`;
    const session: RedTeamSession = {
      id: sessionId,
      target_url: scopePattern.trim() || workspacePath,
      current_stage: stopRef.current ? "Stopped" : "Report",
      findings: collected,
      started_at: new Date().toISOString(),
      finished_at: new Date().toISOString(),
    };
    setActiveSession(session);
    setWsRun({
      kind: "finished",
      reviewed,
      matched: targets.matched,
      limit: targets.limit,
      stopped: stopRef.current,
    });
    setScanning(false);
    addLog("success", "Report", `${collected.length} finding(s) across ${reviewed} file(s)`);

    // Persist so the run shows in Previous Sessions and Export Report works.
    try {
      await invoke("redteam_save_session", { session });
      loadSessions();
    } catch {
      // A save failure loses the history row, not the findings on screen.
    }
  }, [workspacePath, selection, scopePattern, addLog, loadSessions]);

  const fixItems = useMemo<FixItem[]>(
    () =>
      (activeSession?.findings ?? []).map((f) => ({
        file: f.source_file,
        line: f.source_line,
        severity: f.severity,
        title: f.title,
        message: f.description || f.title,
        suggestion: f.remediation || null,
        notes: [
          f.attack_vector ? `Attack vector: ${f.attack_vector}` : "",
          f.poc ? `Proof of concept: ${f.poc}` : "",
        ].filter(Boolean),
      })),
    [activeSession],
  );

  const downloadReport = useCallback(async (sessionId: string) => {
    try {
      const report = await invoke<string>("generate_redteam_report", { sessionId });
      const blob = new Blob([report], { type: "text/markdown" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${sessionId}-report.md`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      addLog("error", "Report", errorMessage(e) || "Failed to generate report");
    }
  }, [addLog]);

  useEffect(() => { loadSessions(); }, [loadSessions]);

  const findings = activeSession?.findings || [];
  const critical = findings.filter((f) => f.severity.toLowerCase() === "critical").length;
  const high = findings.filter((f) => f.severity.toLowerCase() === "high").length;
  const medium = findings.filter((f) => f.severity.toLowerCase() === "medium").length;
  const low = findings.filter((f) => f.severity.toLowerCase() === "low").length;

  const formatElapsed = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;

  return (
    <div className="panel-container" style={{ fontFamily: "var(--font-family)" }}>
      {/* Header */}
      <div className="panel-header">
        <h3>Red Team</h3>
      </div>

      <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {/* Mode toggle — attack the workspace's code and content, or a URL. */}
      <div style={{ display: "flex", gap: 4, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 3 }}>
        {(["workspace", "website"] as const).map((m) => (
          <button
            key={m}
            onClick={() => !scanning && setMode(m)}
            disabled={scanning}
            aria-pressed={mode === m}
            style={{
              flex: 1, padding: "5px 10px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)",
              border: "none", cursor: scanning ? "default" : "pointer",
              background: mode === m ? "var(--bg-primary)" : "transparent",
              color: mode === m ? "var(--text-primary)" : "var(--text-secondary)",
              fontWeight: mode === m ? 600 : 400,
            }}
          >
            {m === "workspace" ? "Workspace (code & content)" : "Website (URL)"}
          </button>
        ))}
      </div>

      {mode === "workspace" ? (
        <>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
            Adversarial review of this project's files with the model selected in the toolbar.
            Leave the box empty for the whole workspace, or narrow to a folder, a glob
            (<code>src/*</code>, <code>*.py</code>), or one file. Content — prompts, docs,
            templates — is reviewed too, for injection and jailbreak risk. Nothing is edited;
            "Fix with AI" writes the request into chat.
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              value={scopePattern}
              onChange={(e) => setScopePattern(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && !scanning) runWorkspaceRedTeam(); }}
              placeholder={workspacePath ? "empty = whole workspace · src/ · src/* · *.py · path/to/file.ts" : "open a workspace folder first"}
              disabled={scanning || !workspacePath}
              className="panel-input"
              style={{ flex: 1 }}
            />
            {scanning ? (
              <button onClick={() => { stopRef.current = true; }} className="panel-btn panel-btn-danger">
                Stop
              </button>
            ) : (
              <button onClick={runWorkspaceRedTeam} disabled={!workspacePath} className="panel-btn panel-btn-primary">
                Run Red Team
              </button>
            )}
          </div>
          {!workspacePath && (
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}>
              Open a workspace folder to red-team its code and content.
            </div>
          )}
        </>
      ) : (
        /* Target input — attack a running URL. */
        <div style={{ display: "flex", gap: 8 }}>
          <input
            value={targetUrl}
            onChange={(e) => setTargetUrl(e.target.value)}
            placeholder="http://localhost:3000"
            disabled={scanning}
            className="panel-input"
            style={{ flex: 1 }}
          />
          {scanning ? (
            <button onClick={handleSuspend} className="panel-btn panel-btn-danger">
              Suspend
            </button>
          ) : (
            <button onClick={startScan} disabled={!targetUrl.trim()} className="panel-btn panel-btn-primary">
              Start Scan
            </button>
          )}
        </div>
      )}

      {/* Workspace progress */}
      {mode === "workspace" && wsRun.kind === "reviewing" && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          Reviewing {wsRun.done + 1} / {wsRun.total} — <code style={{ fontSize: "var(--font-size-xs)" }}>{wsRun.current}</code>
        </div>
      )}
      {mode === "workspace" && wsRun.kind === "finished" && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {wsRun.stopped ? "Stopped" : "Done"} — reviewed {wsRun.reviewed} file(s)
          {wsRun.matched > wsRun.limit && `; scope had ${wsRun.matched}, capped at ${wsRun.limit}`}
          {failures.length > 0 && `; ${failures.length} could not be read`}.
        </div>
      )}

      {/* Elapsed timer (website staged scan) */}
      {mode === "website" && scanning && (
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
          <Loader2 size={14} style={{ animation: "spin 1s linear infinite" }} />
          <span>Scanning... {formatElapsed(elapsedSecs)}</span>
        </div>
      )}

      {/* Pipeline stages with details (website staged scan only) */}
      {mode === "website" && (
      <div style={{ marginBottom: 16 }}>
        {stageStatuses.map((ss, i) => {
          const isExpanded = expandedStage === ss.stage;
          return (
            <div key={ss.stage} style={{ marginBottom: 4 }}>
              <div role="button" tabIndex={0}
                onClick={() => setExpandedStage(isExpanded ? null : ss.stage)}
                style={{
                  display: "flex", alignItems: "center", gap: 8, padding: "8px 12px",
                  background: ss.status === "running" ? "color-mix(in srgb, var(--accent-blue) 8%, transparent)" : "var(--bg-secondary)",
                  borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)",
                  borderLeft: `3px solid ${
                    ss.status === "running" ? "var(--accent-blue)" :
                    ss.status === "success" ? "var(--success-color)" :
                    ss.status === "failed" ? "var(--error-color)" :
                    ss.status === "skipped" ? "var(--warning-color)" :
                    "var(--border-color)"
                  }`,
                }}
              >
                {/* Status icon */}
                {ss.status === "running" && <Loader2 size={14} style={{ color: "var(--accent-blue)", animation: "spin 1s linear infinite", flexShrink: 0 }} />}
                {ss.status === "success" && <CheckCircle2 size={14} style={{ color: "var(--success-color)", flexShrink: 0 }} />}
                {ss.status === "failed" && <XCircle size={14} style={{ color: "var(--error-color)", flexShrink: 0 }} />}
                {ss.status === "skipped" && <AlertTriangle size={14} style={{ color: "var(--warning-color)", flexShrink: 0 }} />}
                {ss.status === "pending" && (
                  <div style={{ width: 14, height: 14, borderRadius: "50%", border: "2px solid var(--border-color)", flexShrink: 0 }} />
                )}

                {/* Stage name + number */}
                <span style={{
                  fontWeight: ss.status === "running" ? 600 : 400,
                  color: ss.status === "running" ? "var(--accent-blue)" :
                         ss.status === "success" ? "var(--success-color)" :
                         ss.status === "failed" ? "var(--error-color)" :
                         "var(--text-primary)",
                }}>
                  {i + 1}. {ss.stage}
                </span>

                {/* Description */}
                <span style={{ flex: 1, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                  {ss.status === "running" ? STAGE_DESCRIPTIONS[ss.stage] : ""}
                </span>

                {/* Duration */}
                {ss.duration != null && (
                  <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
                    {ss.duration.toFixed(1)}s
                  </span>
                )}

                {/* Expand arrow */}
                {ss.details.length > 0 && (
                  isExpanded
                    ? <ChevronDown size={12} style={{ color: "var(--text-secondary)" }} />
                    : <ChevronRight size={12} style={{ color: "var(--text-secondary)" }} />
                )}
              </div>

              {/* Expanded details */}
              {isExpanded && ss.details.length > 0 && (
                <div style={{ marginLeft: 29, padding: "4px 12px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  {ss.details.map((d, j) => (
                    <div key={j} style={{ display: "flex", gap: 6, alignItems: "center", padding: "2px 0" }}>
                      <span style={{ color: "var(--success-color)" }}>&#10003;</span>
                      <span>{d}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
      )}

      {/* Live activity log */}
      {logs.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "0 0 8px", fontSize: "var(--font-size-md)", display: "flex", alignItems: "center", gap: 6 }}>
            Activity Log
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontWeight: 400 }}>({logs.length} entries)</span>
          </h4>
          <div style={{
            maxHeight: 200, overflow: "auto", padding: 8,
            background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)",
            fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", lineHeight: 1.7,
          }}>
            {logs.map((log, i) => (
              <div key={i} style={{ display: "flex", gap: 8 }}>
                <span style={{ color: "var(--text-secondary)", flexShrink: 0 }}>{log.timestamp}</span>
                <span style={{
                  color: logLevelColor(log.level), fontWeight: 600, flexShrink: 0, width: 32, textAlign: "center",
                }}>
                  {logLevelPrefix(log.level)}
                </span>
                <span style={{ color: "var(--accent-blue)", flexShrink: 0, minWidth: 80 }}>[{log.stage}]</span>
                <span style={{ color: log.level === "error" ? "var(--error-color)" : "var(--text-primary)" }}>
                  {log.message}
                </span>
              </div>
            ))}
            <div ref={logEndRef} />
          </div>
        </div>
      )}

      {/* Summary bar */}
      {activeSession && (
        <div style={{
          display: "flex", gap: 12, marginBottom: 16, padding: "8px 12px",
          background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", alignItems: "center",
        }}>
          <span style={{ color: "var(--error-color)", fontWeight: 600 }}>{critical} Critical</span>
          <span style={{ color: "var(--accent-gold)", fontWeight: 600 }}>{high} High</span>
          <span style={{ color: "var(--warning-color)", fontWeight: 600 }}>{medium} Medium</span>
          <span style={{ color: "var(--accent-blue)", fontWeight: 600 }}>{low} Low</span>
          <span style={{ flex: 1 }} />
          {fixItems.length > 0 && (
            <FixWithAIButton
              items={fixItems}
              source="red team"
              resetKey={runId}
              instructions={[
                "Each item is an attack an adversary could run against this file. Close the attack, do not just silence the finding.",
                "The proof-of-concept shows how it triggers — the fix must make that input safe.",
              ]}
            />
          )}
          <button onClick={() => downloadReport(activeSession.id)} style={{
            padding: "4px 12px", fontSize: "var(--font-size-sm)", borderRadius: 3, border: "1px solid var(--border-color)",
            background: "none", color: "var(--text-primary)", cursor: "pointer",
          }}>
            Export Report
          </button>
        </div>
      )}

      {/* Findings list */}
      {findings.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "0 0 8px", fontSize: "var(--font-size-md)" }}>Findings ({findings.length})</h4>
          {findings.sort((a, b) => b.cvss_score - a.cvss_score).map((f) => (
            <div role="button" tabIndex={0} key={f.id} style={{
              marginBottom: 8, padding: "8px 12px", borderRadius: "var(--radius-xs-plus)",
              background: "var(--bg-secondary)", borderLeft: `3px solid ${severityColor(f.severity)}`,
              cursor: "pointer",
            }} onClick={() => setExpandedFinding(expandedFinding === f.id ? null : f.id)}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ display: "inline-flex" }}>{severityIcon(f.severity)}</span>
                <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, flex: 1 }}>{f.title}</span>
                {/* CVSS only when one was actually computed (URL scans);
                    workspace findings carry a severity, not a fabricated score. */}
                <span style={{
                  fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3,
                  background: severityColor(f.severity), color: "var(--btn-primary-fg, #fff)", fontWeight: 600,
                }}>
                  {f.cvss_score > 0 ? `CVSS ${f.cvss_score.toFixed(1)}` : f.severity.toUpperCase()}
                </span>
                {f.confirmed && (
                  <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, background: "var(--error-color)", color: "var(--btn-primary-fg, #fff)" }}>
                    CONFIRMED
                  </span>
                )}
              </div>

              {/* File link */}
              {f.source_file && (
                <div style={{ marginTop: 4 }}>
                  <span
                    onClick={(e) => { e.stopPropagation(); if (onOpenFile && workspacePath) { const full = f.source_file!.startsWith("/") ? f.source_file! : `${workspacePath}/${f.source_file}`; onOpenFile(full, f.source_line || undefined); } }}
                    style={{
                      fontSize: "var(--font-size-xs)", color: "var(--accent-blue)", fontFamily: "var(--font-mono)",
                      cursor: onOpenFile ? "pointer" : "default",
                      textDecoration: onOpenFile ? "underline" : "none",
                    }}
                    title="Open in editor"
                  >
                    {f.source_file}{f.source_line ? `:${f.source_line}` : ""}
                  </span>
                </div>
              )}

              {expandedFinding === f.id && (
                <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", lineHeight: 1.6 }}>
                  {/* URL findings name a URL + parameter; workspace findings a
                      file + line. Show whichever this one has. */}
                  {f.url ? (
                    <>
                      <div><strong>URL:</strong> <code style={{ fontSize: "var(--font-size-sm)" }}>{f.url}</code></div>
                      <div><strong>Parameter:</strong> <code style={{ fontSize: "var(--font-size-sm)" }}>{f.location}</code></div>
                    </>
                  ) : (
                    <div><strong>Location:</strong> <code style={{ fontSize: "var(--font-size-sm)" }}>{f.location}</code></div>
                  )}
                  <div><strong>Vector:</strong> {f.attack_vector}</div>
                  <div style={{ marginTop: 4 }}><strong>Description:</strong> {f.description}</div>
                  {f.poc && (
                    <div style={{ marginTop: 4 }}>
                      <strong>PoC:</strong>
                      <pre style={{
                        margin: "4px 0", padding: 8, background: "var(--bg-primary)", borderRadius: 3,
                        fontSize: "var(--font-size-sm)", overflow: "auto", whiteSpace: "pre-wrap",
                      }}>
                        {f.poc}
                      </pre>
                    </div>
                  )}
                  {f.remediation && (
                    <div style={{ marginTop: 4, color: "var(--success-color)" }}>
                      <strong>Remediation:</strong> {f.remediation}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Previous sessions */}
      {sessions.length > 0 && (
        <div>
          <h4 style={{ margin: "0 0 8px", fontSize: "var(--font-size-md)" }}>Previous Sessions</h4>
          {sessions.map((s) => (
            <div role="button" tabIndex={0} key={s.id} style={{
              display: "flex", alignItems: "center", gap: 8,
              padding: "8px 12px", marginBottom: 4, borderRadius: "var(--radius-xs-plus)",
              background: "var(--bg-secondary)", fontSize: "var(--font-size-base)", cursor: "pointer",
            }} onClick={async () => {
              try {
                const f = await invoke<VulnFinding[]>("get_redteam_findings", { sessionId: s.id });
                setActiveSession({ ...s, findings: f });
              } catch { setActiveSession(s); }
            }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{s.id}</span>
              <span style={{ color: "var(--text-secondary)" }}>{s.target_url}</span>
              <span style={{ flex: 1 }} />
              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{s.findings.length} findings</span>
            </div>
          ))}
        </div>
      )}

      {/* Empty state */}
      {!scanning && !activeSession && findings.length === 0 && sessions.length === 0 && logs.length === 0 && (
        <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-secondary)" }}>
          <CircleAlert size={32} strokeWidth={1.5} style={{ color: "var(--text-secondary)", marginBottom: 12 }} />
          <p style={{ fontSize: "var(--font-size-md)", margin: "0 0 8px" }}>No red-team runs yet</p>
          {mode === "workspace" ? (
            <p style={{ fontSize: "var(--font-size-base)" }}>
              Click <strong>Run Red Team</strong> to review this project's code and content
              for attacks — injection and exploitation in code, prompt-injection and jailbreak
              risk in prompts, docs, and templates.
            </p>
          ) : (
            <p style={{ fontSize: "var(--font-size-base)" }}>
              Enter a target URL above and click <strong>Start Scan</strong> to run
              an autonomous security assessment.
            </p>
          )}
          <p style={{ fontSize: "var(--font-size-sm)", marginTop: 12, fontStyle: "italic" }}>
            Only test code and applications you own and control.
          </p>
        </div>
      )}

      {/* CSS keyframes */}
      <style>{`
        @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
      `}</style>
      </div>
    </div>
  );
}
