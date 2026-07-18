/**
 * RLHFAlignmentDashboard — RLHF training progress panel.
 *
 * Stage pipeline (SFT -> RM -> PPO/DPO), reward model accuracy chart,
 * KL divergence tracking, alignment tax display, and safety benchmark results.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AlignmentMetrics {
  stages: AlignmentStage[];
  rewardModelAccuracy: number[];
  klDivergence: number[];
  alignmentTax: number;
  basePerformance: number;
  alignedPerformance: number;
  safetyBenchmarks: SafetyBenchmark[];
}

interface AlignmentStage {
  name: string;
  status: string;
  progress: number;
  metrics: Record<string, number>;
}

interface SafetyBenchmark {
  name: string;
  score: number;
  threshold: number;
  passed: boolean;
}

const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

const stageStatusColor = (s: string) => s === "complete" ? "var(--success-color)" : s === "running" ? "var(--warning-color)" : "var(--text-secondary)";

export function RLHFAlignmentDashboard() {
  const [runId, setRunId] = useState("");
  const [data, setData] = useState<AlignmentMetrics | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchData = useCallback(async () => {
    if (!runId) return;
    setLoading(true);
    try {
      const res = await invoke<AlignmentMetrics>("rl_get_alignment_metrics", { runId });
      setData(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [runId]);

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>RLHF Alignment Dashboard</h2>

      <div className="panel-card" style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <label className="panel-label">Run ID:</label>
        <input value={runId} onChange={e => setRunId(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "var(--font-size-base)" }} />
        <button className="panel-btn panel-btn-primary" onClick={fetchData} disabled={loading}>{loading ? "..." : "Load"}</button>
      </div>

      {data && (
        <>
          <div className="panel-card">
            <div className="panel-label">Pipeline Stages</div>
            <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
              {data.stages.map((s, i) => (
                <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                  <div style={{ padding: "8px 12px", borderRadius: "var(--radius-sm)", background: stageStatusColor(s.status), color: "var(--btn-primary-fg, #fff)", fontWeight: 600, fontSize: "var(--font-size-base)", textAlign: "center", minWidth: 60 }}>
                    <div>{s.name}</div>
                    <div style={{ fontSize: "var(--font-size-xs)", fontWeight: 400 }}>{s.progress}%</div>
                  </div>
                  {i < data.stages.length - 1 && <span style={{ color: "var(--text-secondary)" }}>&rarr;</span>}
                </div>
              ))}
            </div>
            {data.stages.map(s => s.status !== "pending" && (
              <div key={s.name} style={{ marginTop: 8 }}>
                <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600 }}>{s.name} Metrics</div>
                {Object.entries(s.metrics).map(([k, v]) => (
                  <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", padding: "1px 0" }}><span>{k}</span><span>{v.toFixed(4)}</span></div>
                ))}
              </div>
            ))}
          </div>

          <div className="panel-card" style={{ display: "flex", gap: 16, justifyContent: "space-around", textAlign: "center" }}>
            <div>
              <div style={{ fontSize: 24, fontWeight: 700, color: data.alignmentTax > 0.1 ? "var(--error-color)" : "var(--success-color)" }}>{(data.alignmentTax * 100).toFixed(1)}%</div>
              <div className="panel-label">Alignment Tax</div>
            </div>
            <div>
              <div style={{ fontSize: "var(--font-size-lg)" }}>{data.basePerformance.toFixed(2)} &rarr; {data.alignedPerformance.toFixed(2)}</div>
              <div className="panel-label">Performance (base / aligned)</div>
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Reward Model Accuracy ({data.rewardModelAccuracy.length} pts)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 50 }}>
              {data.rewardModelAccuracy.slice(-60).map((v, i) => (
                <div key={i} style={{ flex: 1, height: `${v * 100}%`, background: "var(--info-color)", borderRadius: "2px 2px 0 0" }} />
              ))}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">KL Divergence ({data.klDivergence.length} pts)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 50 }}>
              {data.klDivergence.slice(-60).map((v, i, a) => {
                const max = Math.max(...a, 0.01);
                return <div key={i} style={{ flex: 1, height: `${(v / max) * 100}%`, background: v > max * 0.7 ? "var(--error-color)" : "var(--warning-color)", borderRadius: "2px 2px 0 0" }} />;
              })}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Safety Benchmarks</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Benchmark</th><th style={thStyle}>Score</th><th style={thStyle}>Threshold</th><th style={thStyle}>Status</th></tr></thead>
              <tbody>
                {data.safetyBenchmarks.map(b => (
                  <tr key={b.name}>
                    <td style={tdStyle}>{b.name}</td>
                    <td style={tdStyle}>{b.score.toFixed(4)}</td>
                    <td style={tdStyle}>{b.threshold.toFixed(4)}</td>
                    <td style={{ ...tdStyle, color: b.passed ? "var(--success-color)" : "var(--error-color)", fontWeight: 600 }}>{b.passed ? "PASS" : "FAIL"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {!data && !loading && <div className="panel-empty">Enter a Run ID and click Load.</div>}
    </div>
  );
}
