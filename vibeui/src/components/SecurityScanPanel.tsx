import React, { useState } from "react";

// -- Types --------------------------------------------------------------------

type Severity = "Critical" | "High" | "Medium" | "Low" | "Info";
type TabName = "Findings" | "Summary" | "Patterns";

interface Finding {
  id: string;
  title: string;
  severity: Severity;
  file: string;
  line: number;
  description: string;
  cwe: string;
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

// -- Mock Data ----------------------------------------------------------------

const MOCK_FINDINGS: Finding[] = [
  { id: "f-001", title: "SQL Injection via string concatenation", severity: "Critical", file: "src/db/queries.rs", line: 42, description: "User input is directly concatenated into SQL query without parameterization.", cwe: "CWE-89", suppressed: false },
  { id: "f-002", title: "Hardcoded API key in source", severity: "High", file: "src/config.rs", line: 18, description: "API key is hardcoded as a string literal. Use environment variables instead.", cwe: "CWE-798", suppressed: false },
  { id: "f-003", title: "Path traversal in file read", severity: "High", file: "src/handlers/files.rs", line: 97, description: "User-controlled path passed to fs::read without sanitization.", cwe: "CWE-22", suppressed: false },
  { id: "f-004", title: "Missing CSRF token validation", severity: "Medium", file: "src/middleware/auth.rs", line: 55, description: "POST endpoints do not validate CSRF tokens.", cwe: "CWE-352", suppressed: true },
  { id: "f-005", title: "Verbose error messages in production", severity: "Medium", file: "src/handlers/errors.rs", line: 12, description: "Stack traces and internal details exposed in error responses.", cwe: "CWE-209", suppressed: false },
  { id: "f-006", title: "Weak password hashing algorithm", severity: "Medium", file: "src/auth/password.rs", line: 30, description: "Using SHA-256 for password hashing instead of bcrypt/argon2.", cwe: "CWE-916", suppressed: false },
  { id: "f-007", title: "Unrestricted file upload type", severity: "Low", file: "src/handlers/upload.rs", line: 23, description: "No MIME type validation on uploaded files.", cwe: "CWE-434", suppressed: false },
  { id: "f-008", title: "Missing Content-Security-Policy header", severity: "Info", file: "src/middleware/headers.rs", line: 8, description: "Response headers do not include Content-Security-Policy.", cwe: "CWE-1021", suppressed: false },
];

const MOCK_PATTERNS: ScanPattern[] = [
  { id: "p-001", name: "SQL Injection", vulnerabilityClass: "Injection", languages: ["Rust", "Python", "JavaScript", "Go"], enabled: true, matchCount: 1 },
  { id: "p-002", name: "XSS Detection", vulnerabilityClass: "Injection", languages: ["JavaScript", "TypeScript", "HTML"], enabled: true, matchCount: 0 },
  { id: "p-003", name: "Hardcoded Secrets", vulnerabilityClass: "Authentication", languages: ["*"], enabled: true, matchCount: 1 },
  { id: "p-004", name: "Path Traversal", vulnerabilityClass: "Access Control", languages: ["Rust", "Python", "Go", "Java"], enabled: true, matchCount: 1 },
  { id: "p-005", name: "CSRF Checks", vulnerabilityClass: "Session Management", languages: ["Rust", "Python", "JavaScript"], enabled: true, matchCount: 1 },
  { id: "p-006", name: "Crypto Weakness", vulnerabilityClass: "Cryptography", languages: ["*"], enabled: true, matchCount: 1 },
  { id: "p-007", name: "Dependency CVE Scan", vulnerabilityClass: "Supply Chain", languages: ["Rust", "JavaScript", "Python"], enabled: false, matchCount: 0 },
];

// -- Helpers ------------------------------------------------------------------

const severityColor = (s: Severity): string => {
  switch (s) {
    case "Critical": return "var(--vscode-errorForeground, #ff4444)";
    case "High": return "var(--vscode-charts-orange, #ff8800)";
    case "Medium": return "var(--vscode-charts-yellow, #ffcc00)";
    case "Low": return "var(--vscode-charts-blue, #4488ff)";
    case "Info": return "var(--vscode-disabledForeground, #888)";
  }
};

const severityOrder: Record<Severity, number> = { Critical: 0, High: 1, Medium: 2, Low: 3, Info: 4 };

// -- Component ----------------------------------------------------------------

const SecurityScanPanel: React.FC = () => {
  const [tab, setTab] = useState<TabName>("Findings");
  const [findings, setFindings] = useState<Finding[]>(MOCK_FINDINGS);
  const [patterns, setPatterns] = useState<ScanPattern[]>(MOCK_PATTERNS);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const tabs: TabName[] = ["Findings", "Summary", "Patterns"];

  const activeFindings = findings.filter((f) => !f.suppressed);
  const suppressedFindings = findings.filter((f) => f.suppressed);

  const toggleSuppress = (id: string) => {
    setFindings((prev) => prev.map((f) => f.id === id ? { ...f, suppressed: !f.suppressed } : f));
  };

  const togglePattern = (id: string) => {
    setPatterns((prev) => prev.map((p) => p.id === id ? { ...p, enabled: !p.enabled } : p));
  };

  const countBySeverity = (sev: Severity) => findings.filter((f) => f.severity === sev && !f.suppressed).length;

  return (
    <div style={{ padding: 12, fontFamily: "var(--vscode-font-family, sans-serif)", fontSize: 13, height: "100%", overflowY: "auto", color: "var(--vscode-foreground, #ccc)", background: "var(--vscode-editor-background, #1e1e1e)" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Security Scanner</div>

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, marginBottom: 12, borderBottom: "1px solid var(--vscode-panel-border, #444)" }}>
        {tabs.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "6px 16px", fontSize: 12, background: "none", border: "none", borderBottom: tab === t ? "2px solid var(--vscode-focusBorder, #007acc)" : "2px solid transparent", color: tab === t ? "var(--vscode-foreground, #fff)" : "var(--vscode-disabledForeground, #888)", cursor: "pointer", fontWeight: tab === t ? 600 : 400 }}>
            {t}
          </button>
        ))}
      </div>

      {/* Findings Tab */}
      {tab === "Findings" && (
        <div>
          {activeFindings
            .sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity])
            .map((f) => (
              <div key={f.id} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderLeft: `3px solid ${severityColor(f.severity)}`, cursor: "pointer" }} onClick={() => setExpandedId(expandedId === f.id ? null : f.id)}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, background: severityColor(f.severity), color: "#fff", fontWeight: 600, minWidth: 50, textAlign: "center" }}>{f.severity}</span>
                  <span style={{ fontWeight: 600, fontSize: 12, flex: 1 }}>{f.title}</span>
                  <button onClick={(e) => { e.stopPropagation(); toggleSuppress(f.id); }} style={{ padding: "2px 8px", fontSize: 10, borderRadius: 3, border: "1px solid var(--vscode-panel-border, #555)", background: "none", color: "var(--vscode-disabledForeground, #888)", cursor: "pointer" }}>Suppress</button>
                </div>
                <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginTop: 4 }}>
                  {f.file}:{f.line} | {f.cwe}
                </div>
                {expandedId === f.id && (
                  <div style={{ marginTop: 8, padding: "8px 10px", background: "var(--vscode-editor-background, #1e1e1e)", borderRadius: 4, fontSize: 12, lineHeight: 1.6 }}>
                    {f.description}
                  </div>
                )}
              </div>
            ))}
          {suppressedFindings.length > 0 && (
            <div style={{ marginTop: 12, fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>
              {suppressedFindings.length} suppressed finding(s)
              {suppressedFindings.map((f) => (
                <div key={f.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", marginTop: 4 }}>
                  <span style={{ textDecoration: "line-through" }}>{f.title}</span>
                  <button onClick={() => toggleSuppress(f.id)} style={{ padding: "2px 6px", fontSize: 10, borderRadius: 3, border: "1px solid var(--vscode-panel-border, #555)", background: "none", color: "var(--vscode-disabledForeground, #888)", cursor: "pointer" }}>Restore</button>
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
              { label: "Total", value: findings.length, color: "var(--vscode-foreground, #ccc)" },
              { label: "Active", value: activeFindings.length, color: "var(--vscode-charts-green, #4caf50)" },
              { label: "Suppressed", value: suppressedFindings.length, color: "var(--vscode-disabledForeground, #888)" },
            ].map(({ label, value, color }) => (
              <div key={label} style={{ background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", padding: "10px 16px", borderRadius: 6, textAlign: "center", minWidth: 80 }}>
                <div style={{ fontSize: 22, fontWeight: 700, color }}>{value}</div>
                <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginTop: 2 }}>{label}</div>
              </div>
            ))}
          </div>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Severity Breakdown</div>
          {(["Critical", "High", "Medium", "Low", "Info"] as Severity[]).map((sev) => {
            const count = countBySeverity(sev);
            const pct = activeFindings.length > 0 ? Math.round((count / activeFindings.length) * 100) : 0;
            return (
              <div key={sev} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ minWidth: 55, fontSize: 12, color: severityColor(sev) }}>{sev}</span>
                <div style={{ flex: 1, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderRadius: 3, height: 10, overflow: "hidden" }}>
                  <div style={{ width: `${pct}%`, height: "100%", background: severityColor(sev), borderRadius: 3 }} />
                </div>
                <span style={{ minWidth: 25, textAlign: "right", fontSize: 11 }}>{count}</span>
                <span style={{ minWidth: 35, textAlign: "right", fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>{pct}%</span>
              </div>
            );
          })}
        </div>
      )}

      {/* Patterns Tab */}
      {tab === "Patterns" && (
        <div>
          {patterns.map((p) => (
            <div key={p.id} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", opacity: p.enabled ? 1 : 0.5 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <input type="checkbox" checked={p.enabled} onChange={() => togglePattern(p.id)} style={{ cursor: "pointer" }} />
                <span style={{ fontWeight: 600, fontSize: 12, flex: 1 }}>{p.name}</span>
                <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--vscode-badge-background, #444)", color: "var(--vscode-badge-foreground, #fff)" }}>{p.vulnerabilityClass}</span>
                {p.matchCount > 0 && (
                  <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 10, background: "var(--vscode-errorForeground, #f44336)", color: "#fff", fontWeight: 600 }}>{p.matchCount}</span>
                )}
              </div>
              <div style={{ display: "flex", gap: 4, marginTop: 6, flexWrap: "wrap" }}>
                {p.languages.map((lang) => (
                  <span key={lang} style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--vscode-editor-background, #1e1e1e)", color: "var(--vscode-disabledForeground, #888)" }}>{lang}</span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SecurityScanPanel;
