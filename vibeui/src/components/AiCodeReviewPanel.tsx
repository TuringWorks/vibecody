/**
 * AiCodeReviewPanel — AI-Assisted Code Review dashboard.
 *
 * Analyze diffs and files for bugs, security issues, complexity, style, and more.
 * Supports quality gates, multi-linter aggregation, and learning from feedback.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play } from "lucide-react";

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

const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" };

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
  const [cliOutput, setCliOutput] = useState("");

  const runAireview = useCallback(async (args: string) => {
    setCliOutput("");
    try {
      const res = await invoke<string>("handle_aireview_command", { args });
      setCliOutput(res);
    } catch (e) { setCliOutput(`Error: ${e}`); }
  }, []);

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
    <div className="panel-container">
      <h2 style={headingStyle}>AI Code Review</h2>
      <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
        {(["review", "findings", "gates", "learning"] as Tab[]).map(t => (
          <button key={t} className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => setTab(t)}>
            {t === "review" ? "Review" : t === "findings" ? "Findings" : t === "gates" ? "Gates" : "Learning"}
          </button>
        ))}
      </div>

      {tab === "review" && (
        <>
          <div className="panel-card">
            <div className="panel-label">Paste unified diff or code to review</div>
            <textarea value={diff} onChange={e => setDiff(e.target.value)} rows={10} className="panel-input panel-input-full" style={{ fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder="diff --git a/src/main.rs b/src/main.rs..." />
            <button className="panel-btn panel-btn-secondary" onClick={doReview} disabled={loading || !diff}>
              {loading ? "Analyzing..." : "Review"}
            </button>
          </div>
          {analysis && (
            <div className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                <span style={{ fontWeight: 600 }}>{analysis.title}</span>
                <span style={{ padding: "2px 8px", borderRadius: 3, background: analysis.riskScore > 70 ? "var(--error-color)" : analysis.riskScore > 40 ? "var(--warning-color)" : "var(--success-color)", color: "var(--btn-primary-fg, #fff)", fontSize: "var(--font-size-sm)" }}>Risk: {analysis.riskScore.toFixed(0)}</span>
              </div>
              <div className="panel-label">{analysis.filesChanged} files | +{analysis.linesAdded} -{analysis.linesRemoved} | {analysis.findings.length} finding(s)</div>
              <div style={{ marginTop: 8 }}>{analysis.summary}</div>
              {analysis.breakingChanges.length > 0 && (
                <div style={{ marginTop: 8, padding: 8, background: "var(--error-bg)", borderRadius: "var(--radius-xs-plus)" }}>
                  <div style={{ fontWeight: 600, color: "var(--error-color)", marginBottom: 4 }}>Breaking Changes</div>
                  {analysis.breakingChanges.map((bc, i) => <div key={i} style={{ fontSize: "var(--font-size-base)" }}>{bc}</div>)}
                </div>
              )}
            </div>
          )}
        </>
      )}

      {tab === "findings" && analysis && (
        <>
          <div className="panel-label">{analysis.findings.length} finding(s)</div>
          {analysis.findings.map(f => (
            <div key={f.id} className="panel-card" style={{ borderLeft: `3px solid ${severityColor(f.severity)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{f.file}:{f.lineStart}</span>
                <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 6px", borderRadius: 3, background: severityColor(f.severity), color: "var(--btn-primary-fg, #fff)" }}>{f.severity}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>[{f.category}] confidence: {(f.confidence * 100).toFixed(0)}%</div>
              <div>{f.message}</div>
              {f.suggestion && <div style={{ marginTop: 4, fontStyle: "italic", color: "var(--text-secondary)" }}>Suggestion: {f.suggestion}</div>}
              {f.autoFixable && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--success-color)" }}>Auto-fixable</span>}
            </div>
          ))}
        </>
      )}

      {tab === "gates" && (
        <>
          <div className="panel-card">
            <button className="panel-btn panel-btn-secondary" onClick={doGates} disabled={loading}>Check Quality Gates</button>
          </div>
          {gateResults.map(g => (
            <div key={g.gateId} className="panel-card" style={{ borderLeft: `3px solid ${g.passed ? "var(--success-color)" : "var(--error-color)"}` }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600 }}>{g.gateId}</span>
                <span style={{ fontSize: "var(--font-size-sm)", color: g.passed ? "var(--success-color)" : "var(--error-color)" }}>{g.passed ? "PASSED" : "FAILED"}</span>
              </div>
              <div className="panel-label">{g.message}</div>
            </div>
          ))}
        </>
      )}

      {tab === "learning" && (
        <div className="panel-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>Learning Metrics</span>
            <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runAireview("learn")} title='vibecli --cmd "/aireview learn"' style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Play size={12} /> View Metrics</button>
          </div>
          <div className="panel-label">Precision / recall / F1 from review feedback.</div>
          {cliOutput && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: "var(--font-size-sm)", margin: 0 }}>{cliOutput}</pre>}
        </div>
      )}
    </div>
  );
}
