/**
 * RLEvalResults — Evaluation results dashboard.
 *
 * Scenario list with pass/fail badges, metric tables per scenario,
 * safety constraint status, quality gate summary, and OPE results
 * with confidence intervals.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EvalSuite {
  id: string;
  name: string;
  scenarioCount: number;
  lastRun: number;
}

interface EvalResults {
  suiteId: string;
  scenarios: ScenarioResult[];
  qualityGates: QualityGate[];
  opeResults: OPEResult[];
}

interface ScenarioResult {
  name: string;
  passed: boolean;
  metrics: Record<string, number>;
  safetyConstraints: SafetyConstraint[];
}

interface SafetyConstraint {
  name: string;
  satisfied: boolean;
  value: number;
  threshold: number;
}

interface QualityGate {
  name: string;
  passed: boolean;
  detail: string;
}

interface OPEResult {
  estimator: string;
  value: number;
  ciLow: number;
  ciHigh: number;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };
const passBadge: React.CSSProperties = { fontSize: 10, padding: "2px 6px", borderRadius: 3, color: "#fff" };

export function RLEvalResults() {
  const [suites, setSuites] = useState<EvalSuite[]>([]);
  const [results, setResults] = useState<EvalResults | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<EvalSuite[]>("rl_list_eval_suites").then(setSuites).catch(console.error);
  }, []);

  const loadResults = useCallback(async (suiteId: string) => {
    setLoading(true);
    try {
      const res = await invoke<EvalResults>("rl_get_eval_results", { suiteId });
      setResults(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  const passedCount = results?.scenarios.filter(s => s.passed).length ?? 0;
  const totalCount = results?.scenarios.length ?? 0;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Evaluation Results</h2>

      <div style={cardStyle}>
        <div style={labelStyle}>Eval Suites</div>
        {suites.map(s => (
          <div key={s.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between" }} onClick={() => loadResults(s.id)}>
            <span style={{ fontWeight: 600 }}>{s.name}</span>
            <span style={labelStyle}>{s.scenarioCount} scenarios</span>
          </div>
        ))}
        {suites.length === 0 && <div style={labelStyle}>No eval suites found.</div>}
      </div>

      {loading && <div style={labelStyle}>Loading results...</div>}
      {results && !loading && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Quality Gates</div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {results.qualityGates.map(g => (
                <span key={g.name} style={{ ...passBadge, background: g.passed ? "var(--success-color)" : "var(--error-color)" }}>{g.name}: {g.passed ? "PASS" : "FAIL"}</span>
              ))}
            </div>
            <div style={{ ...labelStyle, marginTop: 6 }}>{passedCount}/{totalCount} scenarios passed</div>
          </div>

          {results.scenarios.map(sc => (
            <div key={sc.name} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <span style={{ fontWeight: 600 }}>{sc.name}</span>
                <span style={{ ...passBadge, background: sc.passed ? "var(--success-color)" : "var(--error-color)" }}>{sc.passed ? "PASS" : "FAIL"}</span>
              </div>
              <table style={tableStyle}>
                <thead><tr><th style={thStyle}>Metric</th><th style={thStyle}>Value</th></tr></thead>
                <tbody>
                  {Object.entries(sc.metrics).map(([k, v]) => (
                    <tr key={k}><td style={tdStyle}>{k}</td><td style={tdStyle}>{v.toFixed(4)}</td></tr>
                  ))}
                </tbody>
              </table>
              <div style={{ ...labelStyle, marginTop: 6 }}>Safety Constraints</div>
              {sc.safetyConstraints.map(c => (
                <div key={c.name} style={{ display: "flex", justifyContent: "space-between", fontSize: 12, padding: "2px 0" }}>
                  <span>{c.name}</span>
                  <span style={{ color: c.satisfied ? "var(--success-color)" : "var(--error-color)" }}>{c.value.toFixed(4)} / {c.threshold.toFixed(4)}</span>
                </div>
              ))}
            </div>
          ))}

          <div style={cardStyle}>
            <div style={labelStyle}>Off-Policy Evaluation (OPE)</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Estimator</th><th style={thStyle}>Value</th><th style={thStyle}>95% CI</th></tr></thead>
              <tbody>
                {results.opeResults.map(o => (
                  <tr key={o.estimator}><td style={tdStyle}>{o.estimator}</td><td style={tdStyle}>{o.value.toFixed(4)}</td><td style={tdStyle}>[{o.ciLow.toFixed(4)}, {o.ciHigh.toFixed(4)}]</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
