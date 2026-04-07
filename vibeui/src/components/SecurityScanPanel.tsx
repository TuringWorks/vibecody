import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

type Severity = "Critical" | "High" | "Medium" | "Low" | "Info";
type TabName = "Findings" | "Summary" | "Patterns" | "History";
type GroupMode = "none" | "cwe" | "file" | "severity";

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

// -- CWE descriptions for group headers ---------------------------------------

const CWE_NAMES: Record<string, string> = {
  "CWE-78": "Command Injection",
  "CWE-79": "Cross-Site Scripting (XSS)",
  "CWE-89": "SQL Injection",
  "CWE-22": "Path Traversal",
  "CWE-327": "Weak Cryptographic Algorithm",
  "CWE-319": "Insecure HTTP Connection",
  "CWE-798": "Hardcoded Secret or API Key",
  "CWE-916": "Weak Password Hashing",
};

// -- Helpers ------------------------------------------------------------------

const severityColor = (s: Severity): string => {
  switch (s) {
    case "Critical": return "var(--accent-rose)";
    case "High": return "var(--accent-gold)";
    case "Medium": return "var(--accent-gold)";
    case "Low": return "var(--info-color)";
    case "Info": return "var(--text-secondary)";
  }
};

const severityOrder: Record<Severity, number> = { Critical: 0, High: 1, Medium: 2, Low: 3, Info: 4 };

/** Group an array by a key function. */
function groupBy<T>(items: T[], keyFn: (item: T) => string): Record<string, T[]> {
  const result: Record<string, T[]> = {};
  for (const item of items) {
    const key = keyFn(item);
    (result[key] ??= []).push(item);
  }
  return result;
}

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
  const [groupMode, setGroupMode] = useState<GroupMode>("cwe");
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());

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

  // Persist suppress/unsuppress to backend
  async function toggleSuppress(f: Finding) {
    const newSuppressed = !f.suppressed;
    setFindings((prev) => prev.map((x) => x.id === f.id ? { ...x, suppressed: newSuppressed } : x));
    try {
      await invoke("suppress_security_finding", {
        cwe: f.cwe,
        file: f.file,
        line: f.line,
        reason: "Suppressed via Security Scanner panel",
      });
    } catch {
      // Revert on failure
      setFindings((prev) => prev.map((x) => x.id === f.id ? { ...x, suppressed: !newSuppressed } : x));
    }
  }

  // Suppress all findings for a CWE project-wide
  async function suppressCwe(cwe: string) {
    setFindings((prev) => prev.map((f) => f.cwe === cwe ? { ...f, suppressed: true } : f));
    try {
      await invoke("suppress_security_cwe", {
        cwe,
        reason: `All ${cwe} findings suppressed via Security Scanner panel`,
      });
    } catch {
      // Revert on failure
      setFindings((prev) => prev.map((f) => f.cwe === cwe ? { ...f, suppressed: false } : f));
    }
  }

  const togglePattern = (id: string) => {
    setPatterns((prev) => prev.map((p) => p.id === id ? { ...p, enabled: !p.enabled } : p));
  };

  const toggleGroup = (key: string) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  };

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

  const countBySeverity = (sev: Severity) => activeFindings.filter((f) => f.severity === sev).length;

  const handleFileClick = (file: string, line: number) => {
    if (onOpenFile && workspacePath) {
      const fullPath = file.startsWith("/") ? file : `${workspacePath}/${file}`;
      onOpenFile(fullPath, line);
    }
  };

  // Group findings for display
  const groupedFindings: [string, Finding[]][] = groupMode === "none"
    ? [["", filteredFindings]]
    : Object.entries(groupBy(filteredFindings, (f) => {
        if (groupMode === "cwe") return f.cwe;
        if (groupMode === "file") return f.file;
        return f.severity;
      })).sort((a, b) => {
        // Sort groups: by severity of worst finding, then alphabetically
        if (groupMode === "severity") return severityOrder[a[0] as Severity] - severityOrder[b[0] as Severity];
        const aWorst = Math.min(...a[1].map((f) => severityOrder[f.severity]));
        const bWorst = Math.min(...b[1].map((f) => severityOrder[f.severity]));
        return aWorst !== bWorst ? aWorst - bWorst : a[0].localeCompare(b[0]);
      });

  // Group suppressed findings by CWE
  const suppressedByCwe = groupBy(suppressedFindings, (f) => f.cwe);

  // Render a single finding row
  const renderFinding = (f: Finding) => (
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
                fontSize: 10, color: "var(--accent-blue)", fontFamily: "var(--font-mono)",
                cursor: onOpenFile ? "pointer" : "default",
                textDecoration: onOpenFile ? "underline" : "none",
              }}
              title="Open in editor"
            >
              {f.file}:{f.line}
            </span>
            {groupMode !== "cwe" && (
              <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                {f.cwe}
              </span>
            )}
          </div>
        </div>
        <button
          onClick={(e) => { e.stopPropagation(); toggleSuppress(f); }}
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
              <div style={{ fontSize: 12, lineHeight: 1.6, color: "var(--success-color)" }}>{f.remediation}</div>
            </div>
          )}
        </div>
      )}
    </div>
  );

  return (
    <div style={{ padding: 12, fontSize: 13, flex: 1, minHeight: 0, display: "flex", flexDirection: "column", gap: 10, color: "var(--text-primary)", background: "var(--bg-primary)" }}>
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
            color: "var(--text-primary)", cursor: scanning ? "wait" : "pointer",
            fontWeight: 600, fontSize: 12, flexShrink: 0,
          }}
        >
          {scanning ? "Scanning..." : "Run Scan"}
        </button>
      </div>

      {error && (
        <div style={{ padding: "6px 10px", background: "color-mix(in srgb, var(--accent-rose) 13%, transparent)", color: "var(--error-color)", borderRadius: 5, fontSize: 12, display: "flex", justifyContent: "space-between" }}>
          <span>{error}</span>
          <button aria-label="Dismiss error" onClick={() => setError(null)} style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer" }}>x</button>
        </div>
      )}

      {/* Severity badges + group toggle */}
      {activeFindings.length > 0 && (
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
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
          <span style={{ flex: 1 }} />
          <select
            value={groupMode}
            onChange={(e) => { setGroupMode(e.target.value as GroupMode); setCollapsedGroups(new Set()); }}
            style={{
              padding: "2px 6px", fontSize: 10, borderRadius: 3,
              background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
              color: "var(--text-secondary)", cursor: "pointer",
            }}
          >
            <option value="cwe">Group by CWE</option>
            <option value="severity">Group by Severity</option>
            <option value="file">Group by File</option>
            <option value="none">No grouping</option>
          </select>
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

            {/* Grouped findings */}
            {groupedFindings.map(([groupKey, groupFindings]) => (
              <div key={groupKey || "__ungrouped"}>
                {groupKey && (
                  <div
                    onClick={() => toggleGroup(groupKey)}
                    style={{
                      display: "flex", alignItems: "center", gap: 8, padding: "6px 8px",
                      cursor: "pointer", userSelect: "none", marginTop: 4, marginBottom: 2,
                      background: "var(--bg-secondary)", borderRadius: 5,
                    }}
                  >
                    <span style={{ fontSize: 10, opacity: 0.6 }}>
                      {collapsedGroups.has(groupKey) ? "\u25B6" : "\u25BC"}
                    </span>
                    <span style={{ fontWeight: 600, fontSize: 12, flex: 1 }}>
                      {groupMode === "cwe" ? `${groupKey} — ${CWE_NAMES[groupKey] || "Unknown"}` : groupKey}
                    </span>
                    <span style={{
                      fontSize: 10, padding: "2px 6px", borderRadius: 10,
                      background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600,
                    }}>
                      {groupFindings.length}
                    </span>
                    {groupMode === "cwe" && (
                      <button
                        onClick={(e) => { e.stopPropagation(); suppressCwe(groupKey); }}
                        style={{
                          padding: "2px 8px", fontSize: 10, borderRadius: 3,
                          border: "1px solid var(--border-color)", background: "none",
                          color: "var(--text-secondary)", cursor: "pointer",
                        }}
                        title={`Suppress all ${groupKey} findings`}
                      >
                        Suppress All
                      </button>
                    )}
                  </div>
                )}
                {!collapsedGroups.has(groupKey) && groupFindings.map(renderFinding)}
              </div>
            ))}

            {/* Suppressed findings section — grouped by CWE */}
            {suppressedFindings.length > 0 && (
              <div style={{ marginTop: 12, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 6 }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 6 }}>
                  {suppressedFindings.length} suppressed finding(s)
                </div>
                {Object.entries(suppressedByCwe)
                  .sort((a, b) => a[0].localeCompare(b[0]))
                  .map(([cwe, cweFindngs]) => (
                  <div key={cwe} style={{ marginBottom: 6 }}>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", padding: "4px 0 2px" }}>
                      {cwe} — {CWE_NAMES[cwe] || "Unknown"} ({cweFindngs.length})
                    </div>
                    {cweFindngs.slice(0, 5).map((f) => (
                      <div key={f.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "2px 0" }}>
                        <span style={{ textDecoration: "line-through", fontSize: 11, flex: 1, opacity: 0.6 }}>
                          {f.file}:{f.line}
                        </span>
                        <button
                          onClick={() => toggleSuppress(f)}
                          style={{ padding: "2px 6px", fontSize: 10, borderRadius: 3, border: "1px solid var(--border-color)", background: "none", color: "var(--text-secondary)", cursor: "pointer" }}
                        >
                          Restore
                        </button>
                      </div>
                    ))}
                    {cweFindngs.length > 5 && (
                      <div style={{ fontSize: 10, color: "var(--text-secondary)", padding: "2px 0" }}>
                        ...and {cweFindngs.length - 5} more
                      </div>
                    )}
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
                { label: "Active", value: activeFindings.length, color: "var(--success-color)" },
                { label: "Suppressed", value: suppressedFindings.length, color: "var(--text-secondary)" },
              ].map(({ label, value, color }) => (
                <div key={label} style={{ background: "var(--bg-tertiary)", padding: "10px 16px", borderRadius: 6, textAlign: "center", minWidth: 80, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color }}>{value}</div>
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

            {/* CWE Breakdown */}
            {activeFindings.length > 0 && (
              <div style={{ marginTop: 16 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>By CWE Category</div>
                {Object.entries(groupBy(activeFindings, (f) => f.cwe))
                  .sort((a, b) => b[1].length - a[1].length)
                  .map(([cwe, items]) => (
                    <div key={cwe} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: 11 }}>
                      <span style={{ fontWeight: 600, minWidth: 60 }}>{cwe}</span>
                      <span style={{ flex: 1, color: "var(--text-secondary)" }}>{CWE_NAMES[cwe] || "Unknown"}</span>
                      <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600 }}>
                        {items.length}
                      </span>
                    </div>
                  ))}
              </div>
            )}

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
                        style={{ flex: 1, fontFamily: "var(--font-mono)", color: "var(--accent-blue)", cursor: onOpenFile ? "pointer" : "default" }}
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
                    <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 10, background: "var(--error-bg)", color: "var(--error-color)", fontWeight: 600 }}>
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
                    color: run.findingCount > 0 ? "var(--accent-rose)" : "var(--accent-green)",
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
