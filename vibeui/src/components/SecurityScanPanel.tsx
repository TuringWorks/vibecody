import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

type Severity = "Critical" | "High" | "Medium" | "Low" | "Info";
type TabName = "Findings" | "Summary" | "Patterns" | "History";

interface Finding {
  id: string;
  title: string;
  severity: Severity;
  file: string;
  line: number;
  description: string;
  cwe: string;
  remediation: string;
  suppressed: boolean;
}

interface ScanPattern {
  id: string;
  name: string;
  vulnerabilityClass: string;
  languages: string[];
  enabled: boolean;
  matchCount: number;
}

interface ScanRun {
  id: string;
  timestamp: string;
  findingCount: number;
  duration: string;
}

interface SecurityScanPanelProps {
  workspacePath?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

// -- Default Patterns ---------------------------------------------------------

const DEFAULT_PATTERNS: ScanPattern[] = [
  { id: "p-001", name: "SQL Injection", vulnerabilityClass: "Injection", languages: ["Rust", "Python", "JavaScript", "Go", "Java"], enabled: true, matchCount: 0 },
  { id: "p-002", name: "XSS Detection", vulnerabilityClass: "Injection", languages: ["JavaScript", "TypeScript", "HTML"], enabled: true, matchCount: 0 },
  { id: "p-003", name: "Hardcoded Secrets", vulnerabilityClass: "Authentication", languages: ["*"], enabled: true, matchCount: 0 },
  { id: "p-004", name: "Path Traversal", vulnerabilityClass: "Access Control", languages: ["Rust", "Python", "Go", "Java"], enabled: true, matchCount: 0 },
  { id: "p-005", name: "CSRF Checks", vulnerabilityClass: "Session Management", languages: ["Rust", "Python", "JavaScript"], enabled: true, matchCount: 0 },
  { id: "p-006", name: "Crypto Weakness", vulnerabilityClass: "Cryptography", languages: ["*"], enabled: true, matchCount: 0 },
  { id: "p-007", name: "Dependency CVE Scan", vulnerabilityClass: "Supply Chain", languages: ["Rust", "JavaScript", "Python"], enabled: true, matchCount: 0 },
  { id: "p-008", name: "Insecure Deserialization", vulnerabilityClass: "Injection", languages: ["Java", "Python", "JavaScript"], enabled: true, matchCount: 0 },
  { id: "p-009", name: "Command Injection", vulnerabilityClass: "Injection", languages: ["*"], enabled: true, matchCount: 0 },
  { id: "p-010", name: "Insecure HTTP", vulnerabilityClass: "Transport", languages: ["*"], enabled: true, matchCount: 0 },
];

// -- Helpers ------------------------------------------------------------------

const severityColor = (s: Severity): string => {
  switch (s) {
    case "Critical": return "#f38ba8";
    case "High": return "#fab387";
    case "Medium": return "#f9e2af";
    case "Low": return "#89b4fa";
    case "Info": return "#6c7086";
  }
};

const severityOrder: Record<Severity, number> = { Critical: 0, High: 1, Medium: 2, Low: 3, Info: 4 };

// -- Component ----------------------------------------------------------------

const SecurityScanPanel: React.FC<SecurityScanPanelProps> = ({ workspacePath, onOpenFile }) => {
  const [tab, setTab] = useState<TabName>("Findings");
  const [findings, setFindings] = useState<Finding[]>([]);
  const [patterns, setPatterns] = useState<ScanPattern[]>(DEFAULT_PATTERNS);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanHistory, setScanHistory] = useState<ScanRun[]>([]);
  const [filterSeverity, setFilterSeverity] = useState<Severity | "All">("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [lastScanTime, setLastScanTime] = useState<string | null>(null);

  const tabs: TabName[] = ["Findings", "Summary", "Patterns", "History"];

  useEffect(() => {
    loadScanResults();
    loadScanHistory();
  }, [workspacePath]);

  async function loadScanResults() {
    if (!workspacePath) return;
    try {
      const result = await invoke<Finding[]>("get_security_scan_results", { workspacePath });
      if (result.length > 0) setFindings(result);
    } catch {
      // No previous results
    }
  }

  async function loadScanHistory() {
    if (!workspacePath) return;
    try {
      const result = await invoke<ScanRun[]>("get_security_scan_history", { workspacePath });
      setScanHistory(result);
    } catch {
      // No history yet
    }
  }

  async function runScan() {
    if (!workspacePath) {
      setError("Open a workspace folder first.");
      return;
    }
    setScanning(true);
    setError(null);
    const startTime = Date.now();
    try {
      const enabledPatterns = patterns.filter((p) => p.enabled).map((p) => p.id);
      const result = await invoke<Finding[]>("run_security_scan", {
        workspacePath,
        patternIds: enabledPatterns,
      });
      setFindings(result);
      setLastScanTime(new Date().toLocaleString());

      // Update pattern match counts
      const countMap: Record<string, number> = {};
      for (const f of result) {
        const patternId = f.cwe; // Map CWE to patterns loosely
        countMap[patternId] = (countMap[patternId] || 0) + 1;
      }
      setPatterns((prev) =>
        prev.map((p) => ({
          ...p,
          matchCount: result.filter((f) => {
            if (p.name === "SQL Injection") return f.cwe === "CWE-89";
            if (p.name === "Hardcoded Secrets") return f.cwe === "CWE-798" || f.cwe === "CWE-321";
            if (p.name === "Path Traversal") return f.cwe === "CWE-22";
            if (p.name === "XSS Detection") return f.cwe === "CWE-79";
            if (p.name === "Command Injection") return f.cwe === "CWE-78";
            if (p.name === "Crypto Weakness") return f.cwe === "CWE-916" || f.cwe === "CWE-327";
            if (p.name === "Insecure HTTP") return f.cwe === "CWE-319";
            return false;
          }).length,
        }))
      );

      // Add to history
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      setScanHistory((prev) => [
        { id: `scan-${Date.now()}`, timestamp: new Date().toLocaleString(), findingCount: result.length, duration: `${elapsed}s` },
        ...prev.slice(0, 19),
      ]);
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  const activeFindings = findings.filter((f) => !f.suppressed);
  const suppressedFindings = findings.filter((f) => f.suppressed);

  const filteredFindings = activeFindings
    .filter((f) => filterSeverity === "All" || f.severity === filterSeverity)
    .filter((f) => {
      if (!searchQuery) return true;
      const q = searchQuery.toLowerCase();
      return f.title.toLowerCase().includes(q) || f.file.toLowerCase().includes(q) || f.cwe.toLowerCase().includes(q);
    })
    .sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);

  const toggleSuppress = (id: string) => {
    setFindings((prev) => prev.map((f) => f.id === id ? { ...f, suppressed: !f.suppressed } : f));
  };

  const togglePattern = (id: string) => {
    setPatterns((prev) => prev.map((p) => p.id === id ? { ...p, enabled: !p.enabled } : p));
  };

  const countBySeverity = (sev: Severity) => activeFindings.filter((f) => f.severity === sev).length;

  const handleFileClick = (file: string, line: number) => {
    if (onOpenFile && workspacePath) {
      const fullPath = file.startsWith("/") ? file : `${workspacePath}/${file}`;
      onOpenFile(fullPath, line);
    }
  };

  return (
    <div style={{ padding: 12, fontSize: 13, height: "100%", display: "flex", flexDirection: "column", gap: 10, color: "var(--text-primary)", background: "var(--bg-primary)" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <div style={{ fontWeight: 600, fontSize: 14 }}>Security Scanner</div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
            {lastScanTime ? `Last scan: ${lastScanTime}` : "Static analysis for common vulnerabilities"}
          </div>
        </div>
        <button
          onClick={runScan}
          disabled={scanning || !workspacePath}
          style={{
            padding: "5px 14px", borderRadius: 5, border: "none",
            background: scanning ? "var(--bg-tertiary)" : "var(--accent-blue)",
            color: "#fff", cursor: scanning ? "wait" : "pointer",
            fontWeight: 600, fontSize: 12, flexShrink: 0,
          }}
        >
          {scanning ? "Scanning..." : "Run Scan"}
        </button>
      </div>

      {error && (
        <div style={{ padding: "6px 10px", background: "rgba(244,67,54,0.13)", color: "#ff4d4f", borderRadius: 5, fontSize: 12, display: "flex", justifyContent: "space-between" }}>
          <span>{error}</span>
          <button onClick={() => setError(null)} style={{ background: "none", border: "none", color: "#ff4d4f", cursor: "pointer" }}>×</button>
        </div>
      )}

      {/* Severity badges summary */}
      {activeFindings.length > 0 && (
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {(["Critical", "High", "Medium", "Low", "Info"] as Severity[]).map((sev) => {
            const count = countBySeverity(sev);
            if (count === 0) return null;
            return (
              <button
                key={sev}
                onClick={() => setFilterSeverity(filterSeverity === sev ? "All" : sev)}
                style={{
                  padding: "2px 8px", borderRadius: 4,
                  border: `1px solid ${severityColor(sev)}`,
                  background: filterSeverity === sev ? `${severityColor(sev)}33` : "transparent",
                  color: severityColor(sev), cursor: "pointer", fontSize: 11, fontWeight: 600,
                }}
              >
                {count} {sev}
              </button>
            );
          })}
          {filterSeverity !== "All" && (
            <button
              onClick={() => setFilterSeverity("All")}
              style={{ padding: "2px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "transparent", color: "var(--text-secondary)", cursor: "pointer", fontSize: 11 }}
            >
              Clear filter
            </button>
          )}
        </div>
      )}

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
        {tabs.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "6px 14px", fontSize: 12, background: "none", border: "none",
            borderBottom: tab === t ? "2px solid var(--accent-blue)" : "2px solid transparent",
            color: tab === t ? "var(--text-primary)" : "var(--text-secondary)",
            cursor: "pointer", fontWeight: tab === t ? 600 : 400,
          }}>
            {t} {t === "Findings" && activeFindings.length > 0 ? `(${filteredFindings.length})` : ""}
          </button>
        ))}
      </div>

      {/* Content area */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {/* Findings Tab */}
        {tab === "Findings" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {/* Search */}
            {activeFindings.length > 0 && (
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search findings by title, file, or CWE..."
                style={{
                  padding: "5px 8px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                  color: "var(--text-primary)", borderRadius: 4, fontSize: 12, marginBottom: 4,
                }}
              />
            )}

            {scanning && (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: 13 }}>
                Scanning workspace for vulnerabilities...<br />
                <span style={{ fontSize: 11, opacity: 0.7 }}>Checking {patterns.filter((p) => p.enabled).length} patterns</span>
              </div>
            )}

            {!scanning && findings.length === 0 && (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: 13, lineHeight: 1.7 }}>
                No scan results yet.<br />
                Click <strong>Run Scan</strong> to analyze your workspace for security issues.
              </div>
            )}

            {filteredFindings.map((f) => (
              <div
                key={f.id}
                style={{
                  borderRadius: 6, background: "var(--bg-tertiary)",
                  borderLeft: `3px solid ${severityColor(f.severity)}`,
                  border: `1px solid ${severityColor(f.severity)}44`,
                }}
              >
                <div
                  onClick={() => setExpandedId(expandedId === f.id ? null : f.id)}
                  style={{ padding: "8px 10px", cursor: "pointer", display: "flex", alignItems: "flex-start", gap: 8 }}
                >
                  <span style={{
                    fontSize: 10, padding: "2px 8px", borderRadius: 3,
                    background: `${severityColor(f.severity)}22`, color: severityColor(f.severity),
                    fontWeight: 600, flexShrink: 0, marginTop: 1,
                  }}>
                    {f.severity}
                  </span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontWeight: 600, fontSize: 12 }}>{f.title}</div>
                    <div style={{ display: "flex", gap: 8, marginTop: 3, flexWrap: "wrap", alignItems: "center" }}>
                      <span
                        onClick={(e) => { e.stopPropagation(); handleFileClick(f.file, f.line); }}
                        style={{
                          fontSize: 10, color: "var(--accent-blue)", fontFamily: "monospace",
                          cursor: onOpenFile ? "pointer" : "default",
                          textDecoration: onOpenFile ? "underline" : "none",
                        }}
                        title="Open in editor"
                      >
                        {f.file}:{f.line}
                      </span>
                      <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                        {f.cwe}
                      </span>
                    </div>
                  </div>
                  <button
                    onClick={(e) => { e.stopPropagation(); toggleSuppress(f.id); }}
                    style={{
                      padding: "2px 8px", fontSize: 10, borderRadius: 3,
                      border: "1px solid var(--border-color)", background: "none",
                      color: "var(--text-secondary)", cursor: "pointer", flexShrink: 0,
                    }}
                  >
                    Suppress
                  </button>
                </div>

                {expandedId === f.id && (
                  <div style={{ borderTop: "1px solid var(--bg-secondary)", padding: "10px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
                    <div>
                      <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>PROBLEM</div>
                      <div style={{ fontSize: 12, lineHeight: 1.6 }}>{f.description}</div>
                    </div>
                    {f.remediation && (
                      <div>
                        <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>REMEDIATION</div>
                        <div style={{ fontSize: 12, lineHeight: 1.6, color: "#a6e3a1" }}>{f.remediation}</div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}

            {suppressedFindings.length > 0 && (
              <div style={{ marginTop: 8, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 4 }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                  {suppressedFindings.length} suppressed finding(s)
                </div>
                {suppressedFindings.map((f) => (
                  <div key={f.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 0" }}>
                    <span style={{ textDecoration: "line-through", fontSize: 11, flex: 1 }}>{f.title}</span>
                    <button
                      onClick={() => toggleSuppress(f.id)}
                      style={{ padding: "2px 6px", fontSize: 10, borderRadius: 3, border: "1px solid var(--border-color)", background: "none", color: "var(--text-secondary)", cursor: "pointer" }}
                    >
                      Restore
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Summary Tab */}
        {tab === "Summary" && (
          <div>
            <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 16 }}>
              {[
                { label: "Total", value: findings.length, color: "var(--text-primary)" },
                { label: "Active", value: activeFindings.length, color: "#a6e3a1" },
                { label: "Suppressed", value: suppressedFindings.length, color: "var(--text-secondary)" },
              ].map(({ label, value, color }) => (
                <div key={label} style={{ background: "var(--bg-tertiary)", padding: "10px 16px", borderRadius: 6, textAlign: "center", minWidth: 80, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontSize: 22, fontWeight: 700, color }}>{value}</div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>{label}</div>
                </div>
              ))}
            </div>

            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Severity Breakdown</div>
            {(["Critical", "High", "Medium", "Low", "Info"] as Severity[]).map((sev) => {
              const count = countBySeverity(sev);
              const pct = activeFindings.length > 0 ? Math.round((count / activeFindings.length) * 100) : 0;
              return (
                <div key={sev} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <span style={{ minWidth: 55, fontSize: 12, color: severityColor(sev), fontWeight: 500 }}>{sev}</span>
                  <div style={{ flex: 1, background: "var(--bg-tertiary)", borderRadius: 3, height: 10, overflow: "hidden" }}>
                    <div style={{ width: `${pct}%`, height: "100%", background: severityColor(sev), borderRadius: 3, transition: "width 0.3s" }} />
                  </div>
                  <span style={{ minWidth: 25, textAlign: "right", fontSize: 11 }}>{count}</span>
                  <span style={{ minWidth: 35, textAlign: "right", fontSize: 11, color: "var(--text-secondary)" }}>{pct}%</span>
                </div>
              );
            })}

            {/* Top affected files */}
            {activeFindings.length > 0 && (
              <div style={{ marginTop: 16 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Most Affected Files</div>
                {Object.entries(
                  activeFindings.reduce<Record<string, number>>((acc, f) => { acc[f.file] = (acc[f.file] || 0) + 1; return acc; }, {})
                )
                  .sort((a, b) => b[1] - a[1])
                  .slice(0, 5)
                  .map(([file, count]) => (
                    <div key={file} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: 11 }}>
                      <span
                        style={{ flex: 1, fontFamily: "monospace", color: "var(--accent-blue)", cursor: onOpenFile ? "pointer" : "default" }}
                        onClick={() => handleFileClick(file, 1)}
                      >
                        {file}
                      </span>
                      <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600 }}>
                        {count}
                      </span>
                    </div>
                  ))}
              </div>
            )}
          </div>
        )}

        {/* Patterns Tab */}
        {tab === "Patterns" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
              Enable or disable vulnerability patterns for scanning. {patterns.filter((p) => p.enabled).length}/{patterns.length} enabled.
            </div>
            {patterns.map((p) => (
              <div key={p.id} style={{
                padding: "8px 10px", borderRadius: 6,
                background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                opacity: p.enabled ? 1 : 0.5,
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <input type="checkbox" checked={p.enabled} onChange={() => togglePattern(p.id)} style={{ cursor: "pointer" }} />
                  <span style={{ fontWeight: 600, fontSize: 12, flex: 1 }}>{p.name}</span>
                  <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                    {p.vulnerabilityClass}
                  </span>
                  {p.matchCount > 0 && (
                    <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 10, background: "#f38ba822", color: "#f38ba8", fontWeight: 600 }}>
                      {p.matchCount}
                    </span>
                  )}
                </div>
                <div style={{ display: "flex", gap: 4, marginTop: 6, flexWrap: "wrap" }}>
                  {p.languages.map((lang) => (
                    <span key={lang} style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                      {lang}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* History Tab */}
        {tab === "History" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {scanHistory.length === 0 ? (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: 13 }}>
                No scan history yet. Run a scan to start tracking.
              </div>
            ) : (
              scanHistory.map((run) => (
                <div key={run.id} style={{
                  padding: "8px 10px", borderRadius: 6, background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: 12,
                }}>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 12, fontWeight: 500 }}>{run.timestamp}</div>
                  </div>
                  <span style={{
                    fontSize: 11, padding: "2px 8px", borderRadius: 10,
                    background: run.findingCount > 0 ? "#f38ba822" : "#a6e3a122",
                    color: run.findingCount > 0 ? "#f38ba8" : "#a6e3a1",
                    fontWeight: 600,
                  }}>
                    {run.findingCount} findings
                  </span>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{run.duration}</span>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      {findings.length > 0 && (
        <div style={{ fontSize: 11, color: "var(--text-secondary)", flexShrink: 0 }}>
          {activeFindings.length} active issue{activeFindings.length !== 1 ? "s" : ""} — click to expand, file links open in editor
        </div>
      )}
    </div>
  );
};

export default SecurityScanPanel;
