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

const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };
const passBadge: React.CSSProperties = { fontSize: "var(--font-size-xs)", padding: "2px 6px", borderRadius: 3, color: "var(--btn-primary-fg, #fff)" };

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
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>Evaluation Results</h2>

      <div className="panel-card">
        <div className="panel-label">Eval Suites</div>
        {suites.map(s => (
          <div key={s.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between" }} onClick={() => loadResults(s.id)}>
            <span style={{ fontWeight: 600 }}>{s.name}</span>
            <span className="panel-label">{s.scenarioCount} scenarios</span>
          </div>
        ))}
        {suites.length === 0 && <div className="panel-empty">No eval suites found.</div>}
      </div>

      {loading && <div className="panel-loading">Loading results...</div>}
      {results && !loading && (
        <>
          <div className="panel-card">
            <div className="panel-label">Quality Gates</div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {results.qualityGates.map(g => (
                <span key={g.name} style={{ ...passBadge, background: g.passed ? "var(--success-color)" : "var(--error-color)" }}>{g.name}: {g.passed ? "PASS" : "FAIL"}</span>
              ))}
            </div>
            <div className="panel-label" style={{ marginTop: 6 }}>{passedCount}/{totalCount} scenarios passed</div>
          </div>

          {results.scenarios.map(sc => (
            <div key={sc.name} className="panel-card">
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
              <div className="panel-label" style={{ marginTop: 6 }}>Safety Constraints</div>
              {sc.safetyConstraints.map(c => (
                <div key={c.name} style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", padding: "2px 0" }}>
                  <span>{c.name}</span>
                  <span style={{ color: c.satisfied ? "var(--success-color)" : "var(--error-color)" }}>{c.value.toFixed(4)} / {c.threshold.toFixed(4)}</span>
                </div>
              ))}
            </div>
          ))}

          <div className="panel-card">
            <div className="panel-label">Off-Policy Evaluation (OPE)</div>
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
