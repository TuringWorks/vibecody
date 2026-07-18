/**
 * RLPolicyComparison — Side-by-side policy comparison panel.
 *
 * Select two policies, compare metrics (reward, sharpe, drawdown, etc.),
 * view action distribution diffs, and explore the lineage tree.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PolicySummary {
  id: string;
  name: string;
  algorithm: string;
  version: string;
}

interface ComparisonResult {
  policyA: string;
  policyB: string;
  metrics: MetricPair[];
  actionDistA: Record<string, number>;
  actionDistB: Record<string, number>;
  lineage: LineageNode[];
}

interface MetricPair {
  name: string;
  valueA: number;
  valueB: number;
  unit: string;
}

interface LineageNode {
  id: string;
  label: string;
  parentId: string | null;
}

const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

const diffColor = (a: number, b: number, higherBetter = true) => {
  const better = higherBetter ? b > a : b < a;
  return better ? "var(--success-color)" : b === a ? "var(--text-primary)" : "var(--error-color)";
};

export function RLPolicyComparison() {
  const [policies, setPolicies] = useState<PolicySummary[]>([]);
  const [policyA, setPolicyA] = useState("");
  const [policyB, setPolicyB] = useState("");
  const [result, setResult] = useState<ComparisonResult | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<PolicySummary[]>("rl_list_policies").then(setPolicies).catch(console.error);
  }, []);

  const compare = useCallback(async () => {
    if (!policyA || !policyB) return;
    setLoading(true);
    try {
      const res = await invoke<ComparisonResult>("rl_compare_policies", { policyA, policyB });
      setResult(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [policyA, policyB]);

  const lowerBetter = new Set(["drawdown", "latency", "loss"]);

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>Policy Comparison</h2>

      <div className="panel-card" style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <select className="panel-select" style={{flex:1}} value={policyA} onChange={e => setPolicyA(e.target.value)}>
          <option value="">Select Policy A</option>
          {policies.map(p => <option key={p.id} value={p.id}>{p.name} (v{p.version})</option>)}
        </select>
        <span style={{ fontWeight: 600 }}>vs</span>
        <select className="panel-select" style={{flex:1}} value={policyB} onChange={e => setPolicyB(e.target.value)}>
          <option value="">Select Policy B</option>
          {policies.map(p => <option key={p.id} value={p.id}>{p.name} (v{p.version})</option>)}
        </select>
        <button className="panel-btn panel-btn-primary" onClick={compare} disabled={loading || !policyA || !policyB}>
          {loading ? "..." : "Compare"}
        </button>
      </div>

      {result && (
        <>
          <div className="panel-card">
            <div className="panel-label">Metric Comparison</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Metric</th><th style={thStyle}>Policy A</th><th style={thStyle}>Policy B</th><th style={thStyle}>Unit</th></tr></thead>
              <tbody>
                {result.metrics.map(m => (
                  <tr key={m.name}>
                    <td style={tdStyle}>{m.name}</td>
                    <td style={tdStyle}>{m.valueA.toFixed(4)}</td>
                    <td style={{ ...tdStyle, color: diffColor(m.valueA, m.valueB, !lowerBetter.has(m.name.toLowerCase())) }}>{m.valueB.toFixed(4)}</td>
                    <td style={tdStyle}>{m.unit}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="panel-card">
            <div className="panel-label">Action Distribution Diff</div>
            {Object.keys(result.actionDistA).map(k => {
              const a = result.actionDistA[k] ?? 0;
              const b = result.actionDistB[k] ?? 0;
              const max = Math.max(a, b, 0.01);
              return (
                <div key={k} style={{ marginBottom: 6 }}>
                  <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 2 }}>{k}</div>
                  <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                    <div style={{ flex: 1, height: 8, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", overflow: "hidden" }}>
                      <div style={{ width: `${(a / max) * 100}%`, height: "100%", background: "var(--info-color)", borderRadius: "var(--radius-xs-plus)" }} />
                    </div>
                    <div style={{ flex: 1, height: 8, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", overflow: "hidden" }}>
                      <div style={{ width: `${(b / max) * 100}%`, height: "100%", background: "var(--warning-color)", borderRadius: "var(--radius-xs-plus)" }} />
                    </div>
                  </div>
                </div>
              );
            })}
          </div>

          <div className="panel-card">
            <div className="panel-label">Lineage</div>
            {result.lineage.map(n => (
              <div key={n.id} style={{ padding: "3px 0", paddingLeft: n.parentId ? 16 : 0, fontSize: "var(--font-size-base)" }}>
                {n.parentId && <span style={{ color: "var(--text-secondary)" }}>|_ </span>}{n.label}
              </div>
            ))}
          </div>
        </>
      )}

      {!result && !loading && <div className="panel-empty">Select two policies and click Compare.</div>}
    </div>
  );
}
