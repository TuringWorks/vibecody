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

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const selectStyle: React.CSSProperties = { padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, flex: 1 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
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
    <div style={panelStyle}>
      <h2 style={headingStyle}>Policy Comparison</h2>

      <div style={{ ...cardStyle, display: "flex", gap: 8, alignItems: "center" }}>
        <select style={selectStyle} value={policyA} onChange={e => setPolicyA(e.target.value)}>
          <option value="">Select Policy A</option>
          {policies.map(p => <option key={p.id} value={p.id}>{p.name} (v{p.version})</option>)}
        </select>
        <span style={{ fontWeight: 600 }}>vs</span>
        <select style={selectStyle} value={policyB} onChange={e => setPolicyB(e.target.value)}>
          <option value="">Select Policy B</option>
          {policies.map(p => <option key={p.id} value={p.id}>{p.name} (v{p.version})</option>)}
        </select>
        <button style={btnStyle} onClick={compare} disabled={loading || !policyA || !policyB}>
          {loading ? "..." : "Compare"}
        </button>
      </div>

      {result && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Metric Comparison</div>
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

          <div style={cardStyle}>
            <div style={labelStyle}>Action Distribution Diff</div>
            {Object.keys(result.actionDistA).map(k => {
              const a = result.actionDistA[k] ?? 0;
              const b = result.actionDistB[k] ?? 0;
              const max = Math.max(a, b, 0.01);
              return (
                <div key={k} style={{ marginBottom: 6 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 2 }}>{k}</div>
                  <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                    <div style={{ flex: 1, height: 8, background: "var(--bg-tertiary)", borderRadius: 4, overflow: "hidden" }}>
                      <div style={{ width: `${(a / max) * 100}%`, height: "100%", background: "var(--info-color)", borderRadius: 4 }} />
                    </div>
                    <div style={{ flex: 1, height: 8, background: "var(--bg-tertiary)", borderRadius: 4, overflow: "hidden" }}>
                      <div style={{ width: `${(b / max) * 100}%`, height: "100%", background: "var(--warning-color)", borderRadius: 4 }} />
                    </div>
                  </div>
                </div>
              );
            })}
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Lineage</div>
            {result.lineage.map(n => (
              <div key={n.id} style={{ padding: "3px 0", paddingLeft: n.parentId ? 16 : 0, fontSize: 12 }}>
                {n.parentId && <span style={{ color: "var(--text-secondary)" }}>|_ </span>}{n.label}
              </div>
            ))}
          </div>
        </>
      )}

      {!result && !loading && <div style={labelStyle}>Select two policies and click Compare.</div>}
    </div>
  );
}
