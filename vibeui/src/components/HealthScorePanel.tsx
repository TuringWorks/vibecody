/**
 * HealthScorePanel — Codebase Health Score dashboard.
 *
 * Scan a project to get health scores across 12 dimensions (test coverage,
 * complexity, security, docs, etc.), view trends, and get remediation suggestions.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DimensionResult {
  dimension: string;
  score: number;
  weight: number;
  details: string;
  remediation: string | null;
}

interface ScanResult {
  overall: number;
  dimensions: DimensionResult[];
  timestamp: number;
}

interface RemediationItem {
  dimension: string;
  priority: string;
  title: string;
  description: string;
  impact: number;
  autoFixable: boolean;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12 };

type Tab = "scan" | "remediate";

export default function HealthScorePanel() {
  const [tab, setTab] = useState<Tab>("scan");
  const [path, setPath] = useState(() => localStorage.getItem("vibeui_workspace") || ".");
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [remediations, setRemediations] = useState<RemediationItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [scanError, setScanError] = useState("");

  const doScan = useCallback(async () => {
    setLoading(true);
    setScan(null);
    setScanError("");
    try {
      const res = await invoke<ScanResult>("healthscore_scan", { path });
      setScan(res);
    } catch (e) { setScanError(String(e)); }
    setLoading(false);
  }, [path]);

  const doRemediate = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<{ remediations: RemediationItem[] }>("healthscore_remediate", { path });
      setRemediations(res.remediations);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [path]);

  const scoreColor = (s: number) => s >= 80 ? "#4caf50" : s >= 60 ? "#ff9800" : "#f44336";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Codebase Health Score</h2>
      <div style={tabRow}>
        {(["scan", "remediate"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => setTab(t)}>
            {t === "scan" ? "Scan" : "Remediate"}
          </button>
        ))}
      </div>

      <div style={{ ...cardStyle, display: "flex", gap: 8, alignItems: "center" }}>
        <label style={labelStyle}>Path:</label>
        <input value={path} onChange={e => setPath(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12 }} />
        <button style={btnStyle} onClick={tab === "scan" ? doScan : doRemediate} disabled={loading}>
          {loading ? "..." : tab === "scan" ? "Scan" : "Analyze"}
        </button>
      </div>

      {scanError && (
        <div style={{ ...cardStyle, border: "1px solid var(--error-color)", color: "var(--error-color)", fontSize: 12 }}>{scanError}</div>
      )}

      {tab === "scan" && scan && (
        <>
          <div style={{ ...cardStyle, textAlign: "center" }}>
            <div style={{ fontSize: 36, fontWeight: 700, color: scoreColor(scan.overall) }}>{scan.overall.toFixed(0)}</div>
            <div style={labelStyle}>Overall Health Score</div>
            <div style={{ ...labelStyle, marginTop: 2 }}>{scan.dimensions.length} dimensions · {path}</div>
          </div>
          {scan.dimensions.map(d => (
            <div key={d.dimension} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>{d.dimension}</span>
                <span style={{ color: scoreColor(d.score), fontWeight: 600 }}>{d.score.toFixed(0)}/100</span>
              </div>
              <div style={{ height: 6, borderRadius: 3, background: "var(--bg-tertiary)", overflow: "hidden" }}>
                <div style={{ width: `${d.score}%`, height: "100%", background: scoreColor(d.score), borderRadius: 3 }} />
              </div>
              <div style={{ ...labelStyle, marginTop: 4 }}>{d.details}</div>
            </div>
          ))}
        </>
      )}

      {tab === "remediate" && remediations.length > 0 && (
        <>
          <div style={labelStyle}>{remediations.length} suggestion(s)</div>
          {remediations.map((r, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>{r.title}</span>
                <span style={{ fontSize: 11, padding: "2px 6px", borderRadius: 3, background: r.priority === "Critical" ? "#f44336" : r.priority === "High" ? "#ff9800" : "var(--bg-tertiary)", color: r.priority === "Critical" || r.priority === "High" ? "#fff" : "var(--text-secondary)" }}>{r.priority}</span>
              </div>
              <div style={labelStyle}>{r.dimension}</div>
              <div style={{ marginTop: 4 }}>{r.description}</div>
              <div style={{ ...labelStyle, marginTop: 4 }}>Impact: +{r.impact.toFixed(0)} pts {r.autoFixable && <span style={{ color: "#4caf50" }}>(auto-fixable)</span>}</div>
            </div>
          ))}
        </>
      )}

      {tab === "scan" && !scan && !loading && <div style={labelStyle}>Click Scan to analyze your codebase health.</div>}
      {tab === "remediate" && remediations.length === 0 && !loading && <div style={labelStyle}>Click Analyze to get improvement suggestions.</div>}
    </div>
  );
}
