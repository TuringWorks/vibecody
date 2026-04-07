/**
 * RLRewardDecomposition — Per-component reward visualization.
 *
 * Stacked bar chart showing reward components (e.g., Sharpe + drawdown + turnover),
 * component weight sliders, and a time series per component.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RewardDecomposition {
  policyId: string;
  components: RewardComponent[];
  timeSeries: TimeSeriesPoint[];
  totalReward: number;
}

interface RewardComponent {
  name: string;
  value: number;
  weight: number;
  color: string;
}

interface TimeSeriesPoint {
  step: number;
  values: Record<string, number>;
}

const DEFAULT_COLORS = ["var(--info-color)", "var(--success-color)", "var(--warning-color)", "var(--error-color)", "#9c27b0", "#00bcd4", "var(--error-color)", "#795548"];

export function RLRewardDecomposition() {
  const [policyId, setPolicyId] = useState("");
  const [data, setData] = useState<RewardDecomposition | null>(null);
  const [weights, setWeights] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(false);

  const fetchData = useCallback(async () => {
    if (!policyId) return;
    setLoading(true);
    try {
      const res = await invoke<RewardDecomposition>("rl_get_reward_decomposition", { policyId });
      setData(res);
      const w: Record<string, number> = {};
      res.components.forEach(c => { w[c.name] = c.weight; });
      setWeights(w);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [policyId]);

  const totalPositive = data?.components.reduce((s, c) => s + Math.max(c.value * (weights[c.name] ?? c.weight), 0), 0) ?? 1;

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>Reward Decomposition</h2>

      <div className="panel-card" style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <label className="panel-label">Policy ID:</label>
        <input value={policyId} onChange={e => setPolicyId(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12 }} />
        <button className="panel-btn panel-btn-primary" onClick={fetchData} disabled={loading}>{loading ? "..." : "Load"}</button>
      </div>

      {data && (
        <>
          <div className="panel-card">
            <div className="panel-label">Total Reward: <strong>{data.totalReward.toFixed(4)}</strong></div>
            <div style={{ display: "flex", height: 32, borderRadius: 4, overflow: "hidden", marginTop: 6 }}>
              {data.components.filter(c => c.value * (weights[c.name] ?? c.weight) > 0).map((c, i) => {
                const wVal = c.value * (weights[c.name] ?? c.weight);
                return <div key={c.name} style={{ width: `${(wVal / totalPositive) * 100}%`, background: c.color || DEFAULT_COLORS[i % DEFAULT_COLORS.length], display: "flex", alignItems: "center", justifyContent: "center", fontSize: 10, color: "#fff", overflow: "hidden" }}>{c.name}</div>;
              })}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Component Weights</div>
            {data.components.map((c, i) => (
              <div key={c.name} style={{ marginBottom: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 2 }}>
                  <span><span style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: c.color || DEFAULT_COLORS[i % DEFAULT_COLORS.length], marginRight: 6 }} />{c.name}</span>
                  <span style={{ fontWeight: 600 }}>{c.value.toFixed(4)} x {(weights[c.name] ?? c.weight).toFixed(2)}</span>
                </div>
                <input type="range" min="0" max="2" step="0.05" value={weights[c.name] ?? c.weight} onChange={e => setWeights(w => ({ ...w, [c.name]: parseFloat(e.target.value) }))} style={{ width: "100%" }} />
              </div>
            ))}
          </div>

          <div className="panel-card">
            <div className="panel-label">Time Series (last {Math.min(data.timeSeries.length, 50)} steps)</div>
            {data.components.map((c, ci) => {
              const pts = data.timeSeries.slice(-50);
              const vals = pts.map(p => p.values[c.name] ?? 0);
              const max = Math.max(...vals.map(Math.abs), 0.01);
              return (
                <div key={c.name} style={{ marginBottom: 8 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 2, color: c.color || DEFAULT_COLORS[ci % DEFAULT_COLORS.length] }}>{c.name}</div>
                  <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 30 }}>
                    {vals.map((v, i) => (
                      <div key={i} style={{ flex: 1, height: `${(Math.abs(v) / max) * 100}%`, background: c.color || DEFAULT_COLORS[ci % DEFAULT_COLORS.length], opacity: 0.7, borderRadius: "2px 2px 0 0" }} />
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        </>
      )}

      {!data && !loading && <div className="panel-empty">Enter a Policy ID and click Load.</div>}
    </div>
  );
}
