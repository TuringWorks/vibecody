/**
 * AiCodeReviewPanel — AI-Assisted Code Review dashboard.
 *
 * Analyze diffs and files for bugs, security issues, complexity, style, and more.
 * Supports quality gates, multi-linter aggregation, and learning from feedback.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ReviewFinding {
  id: string;
  file: string;
  lineStart: number;
  lineEnd: number;
  severity: string;
  category: string;
  message: string;
  suggestion: string;
  autoFixable: boolean;
  confidence: number;
}

interface PrAnalysisResult {
  title: string;
  summary: string;
  filesChanged: number;
  linesAdded: number;
  linesRemoved: number;
  findings: ReviewFinding[];
  riskScore: number;
  breakingChanges: string[];
}

interface QualityGateResult {
  gateId: string;
  passed: boolean;
  message: string;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12 };

const severityColor = (s: string) => {
  switch (s) { case "Critical": case "Security": return "var(--error-color)"; case "Error": return "#ff5722"; case "Warning": return "var(--warning-color)"; default: return "var(--info-color)"; }
};

type Tab = "review" | "gates" | "findings" | "learning";

export default function AiCodeReviewPanel() {
  const [tab, setTab] = useState<Tab>("review");
  const [diff, setDiff] = useState("");
  const [analysis, setAnalysis] = useState<PrAnalysisResult | null>(null);
  const [gateResults, setGateResults] = useState<QualityGateResult[]>([]);
  const [loading, setLoading] = useState(false);

  const doReview = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<PrAnalysisResult>("aireview_analyze_diff", { diff });
      setAnalysis(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [diff]);

  const doGates = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<{ results: QualityGateResult[] }>("aireview_check_gates", { diff });
      setGateResults(res.results);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [diff]);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>AI Code Review</h2>
      <div style={tabRow}>
        {(["review", "findings", "gates", "learning"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => setTab(t)}>
            {t === "review" ? "Review" : t === "findings" ? "Findings" : t === "gates" ? "Gates" : "Learning"}
          </button>
        ))}
      </div>

      {tab === "review" && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Paste unified diff or code to review</div>
            <textarea value={diff} onChange={e => setDiff(e.target.value)} rows={10} style={{ ...inputStyle, fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder="diff --git a/src/main.rs b/src/main.rs..." />
            <button style={btnStyle} onClick={doReview} disabled={loading || !diff}>
              {loading ? "Analyzing..." : "Review"}
            </button>
          </div>
          {analysis && (
            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                <span style={{ fontWeight: 600 }}>{analysis.title}</span>
                <span style={{ padding: "2px 8px", borderRadius: 3, background: analysis.riskScore > 70 ? "var(--error-color)" : analysis.riskScore > 40 ? "var(--warning-color)" : "var(--success-color)", color: "#fff", fontSize: 11 }}>Risk: {analysis.riskScore.toFixed(0)}</span>
              </div>
              <div style={labelStyle}>{analysis.filesChanged} files | +{analysis.linesAdded} -{analysis.linesRemoved} | {analysis.findings.length} finding(s)</div>
              <div style={{ marginTop: 8 }}>{analysis.summary}</div>
              {analysis.breakingChanges.length > 0 && (
                <div style={{ marginTop: 8, padding: 8, background: "#f4433620", borderRadius: 4 }}>
                  <div style={{ fontWeight: 600, color: "var(--error-color)", marginBottom: 4 }}>Breaking Changes</div>
                  {analysis.breakingChanges.map((bc, i) => <div key={i} style={{ fontSize: 12 }}>{bc}</div>)}
                </div>
              )}
            </div>
          )}
        </>
      )}

      {tab === "findings" && analysis && (
        <>
          <div style={labelStyle}>{analysis.findings.length} finding(s)</div>
          {analysis.findings.map(f => (
            <div key={f.id} style={{ ...cardStyle, borderLeft: `3px solid ${severityColor(f.severity)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontWeight: 600, fontSize: 12 }}>{f.file}:{f.lineStart}</span>
                <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: severityColor(f.severity), color: "#fff" }}>{f.severity}</span>
              </div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>[{f.category}] confidence: {(f.confidence * 100).toFixed(0)}%</div>
              <div>{f.message}</div>
              {f.suggestion && <div style={{ marginTop: 4, fontStyle: "italic", color: "var(--text-secondary)" }}>Suggestion: {f.suggestion}</div>}
              {f.autoFixable && <span style={{ fontSize: 10, color: "var(--success-color)" }}>Auto-fixable</span>}
            </div>
          ))}
        </>
      )}

      {tab === "gates" && (
        <>
          <div style={cardStyle}>
            <button style={btnStyle} onClick={doGates} disabled={loading}>Check Quality Gates</button>
          </div>
          {gateResults.map(g => (
            <div key={g.gateId} style={{ ...cardStyle, borderLeft: `3px solid ${g.passed ? "var(--success-color)" : "var(--error-color)"}` }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600 }}>{g.gateId}</span>
                <span style={{ fontSize: 11, color: g.passed ? "var(--success-color)" : "var(--error-color)" }}>{g.passed ? "PASSED" : "FAILED"}</span>
              </div>
              <div style={labelStyle}>{g.message}</div>
            </div>
          ))}
        </>
      )}

      {tab === "learning" && (
        <div style={cardStyle}>
          <div style={labelStyle}>Learning statistics will appear after review feedback is recorded.</div>
          <div style={{ marginTop: 8, fontSize: 12 }}>
            Use <code>/aireview learn</code> in the terminal to view precision/recall/F1 metrics.
          </div>
        </div>
      )}
    </div>
  );
}
