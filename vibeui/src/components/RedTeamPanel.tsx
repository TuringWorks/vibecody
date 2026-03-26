import React, { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CircleAlert, AlertTriangle, Info } from "lucide-react";

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

interface Props {
  workspacePath: string | null;
  provider: string;
}

// -- Constants ----------------------------------------------------------------

const STAGES = ["Recon", "Analysis", "Exploitation", "Validation", "Report"];

function severityColor(sev: string): string {
  switch (sev.toLowerCase()) {
    case "critical": return "var(--error-color)";
    case "high": return "var(--warning-color)";
    case "medium": return "var(--warning-color)";
    case "low": return "var(--info-color)";
    default: return "var(--text-secondary)";
  }
}

function severityIcon(sev: string): React.ReactNode {
  switch (sev.toLowerCase()) {
    case "critical": return <CircleAlert size={14} strokeWidth={1.5} style={{ color: "var(--accent-rose)" }} />;
    case "high": return <CircleAlert size={14} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />;
    case "medium": return <AlertTriangle size={14} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />;
    case "low": return <Info size={14} strokeWidth={1.5} style={{ color: "var(--accent-blue)" }} />;
    default: return <Info size={14} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />;
  }
}

// -- Component ----------------------------------------------------------------

export function RedTeamPanel({ workspacePath, provider: _provider }: Props) {
  const [targetUrl, setTargetUrl] = useState("http://localhost:3000");
  const [scanning, setScanning] = useState(false);
  const [currentStage, setCurrentStage] = useState<string | null>(null);
  const [sessions, setSessions] = useState<RedTeamSession[]>([]);
  const [activeSession, setActiveSession] = useState<RedTeamSession | null>(null);
  const [expandedFinding, setExpandedFinding] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Track mount status so polling loop stops on unmount
  const mountedRef = useRef(true);
  const cancelRef = useRef(false);
  const taskIdRef = useRef(0);
  useEffect(() => { return () => { mountedRef.current = false; }; }, []);

  // Load sessions list.
  const loadSessions = useCallback(async () => {
    try {
      const list = await invoke<RedTeamSession[]>("get_redteam_sessions");
      if (mountedRef.current) setSessions(list);
    } catch (e) {
      // Command may not exist yet -- tolerate.
      if (mountedRef.current) setSessions([]);
    }
  }, []);

  // Suspend a running scan.
  const handleSuspend = useCallback(() => {
    cancelRef.current = true;
    setScanning(false);
    setCurrentStage(null);
    setError("Scan suspended by user.");
  }, []);

  // Start scan.
  const startScan = useCallback(async () => {
    if (!targetUrl.trim()) return;
    cancelRef.current = false;
    taskIdRef.current += 1;
    const thisId = taskIdRef.current;
    setError(null);
    setScanning(true);
    setCurrentStage("Recon");
    const scanStartTime = new Date().toISOString();

    try {
      const sessionId = await invoke<string>("start_redteam_scan", {
        url: targetUrl,
        config: { source_path: workspacePath },
      });

      if (cancelRef.current || taskIdRef.current !== thisId) return;

      // Poll for completion (simplified -- in production would use SSE).
      let done = false;
      let attempts = 0;
      const maxAttempts = 150; // 5 minute timeout at 2s intervals
      while (!done && attempts < maxAttempts && mountedRef.current) {
        await new Promise((r) => setTimeout(r, 2000));
        if (!mountedRef.current || cancelRef.current || taskIdRef.current !== thisId) break;
        attempts++;
        try {
          const findings = await invoke<VulnFinding[]>("get_redteam_findings", { sessionId });
          if (cancelRef.current || taskIdRef.current !== thisId) return;
          const sess: RedTeamSession = {
            id: sessionId,
            target_url: targetUrl,
            current_stage: "Report",
            findings,
            started_at: scanStartTime,
            finished_at: new Date().toISOString(),
          };
          if (mountedRef.current) setActiveSession(sess);
          done = true;
        } catch {
          if (cancelRef.current || taskIdRef.current !== thisId) return;
          // Still running -- advance stage every 3 polls (~6s) rather than every poll
          if (mountedRef.current && attempts % 3 === 0) {
            setCurrentStage((prev) => {
              const idx = STAGES.indexOf(prev || "Recon");
              return STAGES[Math.min(idx + 1, STAGES.length - 1)];
            });
          }
        }
      }
      if (!done && mountedRef.current && !cancelRef.current && taskIdRef.current === thisId) {
        setError("Scan timed out after 5 minutes");
      }
    } catch (e: any) {
      if (cancelRef.current || taskIdRef.current !== thisId) return;
      if (mountedRef.current) setError(e?.toString() || "Scan failed");
    } finally {
      if (mountedRef.current && !cancelRef.current && taskIdRef.current === thisId) {
        setScanning(false);
        setCurrentStage(null);
        loadSessions();
      }
    }
  }, [targetUrl, workspacePath, loadSessions]);

  // Generate report.
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
    } catch (e: any) {
      setError(e?.toString() || "Failed to generate report");
    }
  }, []);

  // Load sessions on mount.
  React.useEffect(() => { loadSessions(); }, [loadSessions]);

  const findings = activeSession?.findings || [];
  const critical = findings.filter((f) => f.severity.toLowerCase() === "critical").length;
  const high = findings.filter((f) => f.severity.toLowerCase() === "high").length;
  const medium = findings.filter((f) => f.severity.toLowerCase() === "medium").length;
  const low = findings.filter((f) => f.severity.toLowerCase() === "low").length;

  return (
    <div style={{ height: "100%", overflow: "auto", padding: "12px", fontFamily: "var(--font-family)" }}>
      {/* Header */}
      <div style={{ marginBottom: 16 }}>
        <h3 style={{ margin: 0, fontSize: 14 }}>Red Team Security Scanner</h3>
        <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--text-secondary)" }}>
          Autonomous penetration testing for your applications
        </p>
      </div>

      {/* Target input */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          value={targetUrl}
          onChange={(e) => setTargetUrl(e.target.value)}
          placeholder="http://localhost:3000"
          disabled={scanning}
          style={{
            flex: 1, padding: "6px 10px", fontSize: 13,
            background: "var(--bg-tertiary)", color: "var(--text-primary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
          }}
        />
        {scanning ? (
          <button
            onClick={handleSuspend}
            style={{
              padding: "6px 16px", fontSize: 13, borderRadius: 4, border: "none",
              background: "var(--error-color)", color: "var(--btn-primary-fg)", cursor: "pointer",
              fontWeight: 600,
            }}
          >
            Suspend
          </button>
        ) : (
          <button
            onClick={startScan}
            disabled={!targetUrl.trim()}
            style={{
              padding: "6px 16px", fontSize: 13, borderRadius: 4, border: "none",
              background: "var(--accent-color)", color: "var(--btn-primary-fg)",
              cursor: !targetUrl.trim() ? "not-allowed" : "pointer",
              fontWeight: 600,
            }}
          >
            Start Scan
          </button>
        )}
      </div>

      {/* Pipeline stages */}
      <div style={{ display: "flex", gap: 4, marginBottom: 16, alignItems: "center" }}>
        {STAGES.map((stage, i) => {
          const isActive = scanning && currentStage === stage;
          const isDone = scanning
            ? STAGES.indexOf(currentStage || "") > i
            : activeSession != null;

          return (
            <React.Fragment key={stage}>
              {i > 0 && (
                <div style={{ width: 20, height: 2, background: isDone ? "var(--success-color)" : "var(--border-color)" }} />
              )}
              <div
                style={{
                  width: 28, height: 28, borderRadius: "50%",
                  display: "flex", alignItems: "center", justifyContent: "center",
                  fontSize: 11, fontWeight: 600,
                  background: isActive ? "var(--error-color)" : isDone ? "var(--success-color)" : "var(--bg-secondary)",
                  color: isActive || isDone ? "white" : "var(--text-secondary)",
                  border: `2px solid ${isActive ? "var(--error-color)" : isDone ? "var(--success-color)" : "var(--border-color)"}`,
                  animation: isActive ? "pulse 1.5s infinite" : "none",
                }}
                title={stage}
              >
                {i + 1}
              </div>
            </React.Fragment>
          );
        })}
        <span style={{ marginLeft: 8, fontSize: 11, color: "var(--text-secondary)" }}>
          {STAGES.map((s) => s.slice(0, 3)).join(" > ")}
        </span>
      </div>

      {error && (
        <div style={{ padding: 8, marginBottom: 12, background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", color: "var(--error-color)", borderRadius: 4, fontSize: 12 }}>
          {error}
        </div>
      )}

      {/* Summary bar */}
      {activeSession && (
        <div style={{
          display: "flex", gap: 12, marginBottom: 16, padding: "8px 12px",
          background: "var(--bg-secondary)", borderRadius: 4, fontSize: 12, alignItems: "center",
        }}>
          <span style={{ color: "var(--error-color)", fontWeight: 600 }}>{critical} Critical</span>
          <span style={{ color: "var(--warning-color)", fontWeight: 600 }}>{high} High</span>
          <span style={{ color: "var(--warning-color)", fontWeight: 600 }}>{medium} Medium</span>
          <span style={{ color: "var(--info-color)", fontWeight: 600 }}>{low} Low</span>
          <span style={{ flex: 1 }} />
          <button
            onClick={() => downloadReport(activeSession.id)}
            style={{
              padding: "4px 12px", fontSize: 11, borderRadius: 3, border: "1px solid var(--border-color)",
              background: "none", color: "var(--text-primary)", cursor: "pointer",
            }}
          >
            Export Report
          </button>
        </div>
      )}

      {/* Findings list */}
      {findings.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "0 0 8px", fontSize: 13 }}>Findings ({findings.length})</h4>
          {findings
            .sort((a, b) => b.cvss_score - a.cvss_score)
            .map((f) => (
              <div
                key={f.id}
                style={{
                  marginBottom: 8, padding: "8px 12px", borderRadius: 4,
                  background: "var(--bg-secondary)",
                  borderLeft: `3px solid ${severityColor(f.severity)}`,
                  cursor: "pointer",
                }}
                onClick={() => setExpandedFinding(expandedFinding === f.id ? null : f.id)}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ display: "inline-flex", alignItems: "center" }}>{severityIcon(f.severity)}</span>
                  <span style={{ fontSize: 12, fontWeight: 600, flex: 1 }}>{f.title}</span>
                  <span style={{
                    fontSize: 10, padding: "2px 6px", borderRadius: 3,
                    background: severityColor(f.severity), color: "var(--btn-primary-fg)", fontWeight: 600,
                  }}>
                    CVSS {f.cvss_score.toFixed(1)}
                  </span>
                  {f.confirmed && (
                    <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--error-color)", color: "var(--btn-primary-fg)" }}>
                      CONFIRMED
                    </span>
                  )}
                </div>

                {expandedFinding === f.id && (
                  <div style={{ marginTop: 8, fontSize: 12, lineHeight: 1.6 }}>
                    <div><strong>URL:</strong> <code>{f.url}</code></div>
                    <div><strong>Parameter:</strong> <code>{f.location}</code></div>
                    <div><strong>Vector:</strong> {f.attack_vector}</div>
                    {f.source_file && (
                      <div><strong>Source:</strong> <code>{f.source_file}{f.source_line ? `:${f.source_line}` : ""}</code></div>
                    )}
                    <div style={{ marginTop: 4 }}><strong>Description:</strong> {f.description}</div>
                    <div style={{ marginTop: 4 }}>
                      <strong>PoC:</strong>
                      <pre style={{
                        margin: "4px 0", padding: 8, background: "var(--bg-primary)", borderRadius: 3,
                        fontSize: 11, overflow: "auto", whiteSpace: "pre-wrap",
                      }}>
                        {f.poc}
                      </pre>
                    </div>
                    <div style={{ marginTop: 4 }}><strong>Remediation:</strong> {f.remediation}</div>
                  </div>
                )}
              </div>
            ))}
        </div>
      )}

      {/* Previous sessions */}
      {sessions.length > 0 && (
        <div>
          <h4 style={{ margin: "0 0 8px", fontSize: 13 }}>Previous Sessions</h4>
          {sessions.map((s) => (
            <div
              key={s.id}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "6px 10px", marginBottom: 4, borderRadius: 4,
                background: "var(--bg-secondary)", fontSize: 12, cursor: "pointer",
              }}
              onClick={async () => {
                try {
                  const findings = await invoke<VulnFinding[]>("get_redteam_findings", { sessionId: s.id });
                  setActiveSession({ ...s, findings });
                } catch {
                  setActiveSession(s);
                }
              }}
            >
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>{s.id}</span>
              <span style={{ color: "var(--text-secondary)" }}>{s.target_url}</span>
              <span style={{ flex: 1 }} />
              <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                {s.findings.length} findings
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Empty state */}
      {!scanning && !activeSession && findings.length === 0 && sessions.length === 0 && (
        <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-secondary)" }}>
          <div style={{ fontSize: 32, marginBottom: 12, display: "flex", justifyContent: "center" }}><CircleAlert size={32} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} /></div>
          <p style={{ fontSize: 13, margin: "0 0 8px" }}>No security scans yet</p>
          <p style={{ fontSize: 12 }}>
            Enter a target URL above and click <strong>Start Scan</strong> to run
            an autonomous security assessment.
          </p>
          <p style={{ fontSize: 11, marginTop: 12, fontStyle: "italic" }}>
            Only test applications you own and control.
          </p>
        </div>
      )}
    </div>
  );
}
