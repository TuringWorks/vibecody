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

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
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
    <div style={panelStyle}>
      <h2 style={headingStyle}>RLHF Alignment Dashboard</h2>

      <div style={{ ...cardStyle, display: "flex", gap: 8, alignItems: "center" }}>
        <label style={labelStyle}>Run ID:</label>
        <input value={runId} onChange={e => setRunId(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12 }} />
        <button style={btnStyle} onClick={fetchData} disabled={loading}>{loading ? "..." : "Load"}</button>
      </div>

      {data && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Pipeline Stages</div>
            <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
              {data.stages.map((s, i) => (
                <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                  <div style={{ padding: "8px 12px", borderRadius: 6, background: stageStatusColor(s.status), color: "#fff", fontWeight: 600, fontSize: 12, textAlign: "center", minWidth: 60 }}>
                    <div>{s.name}</div>
                    <div style={{ fontSize: 10, fontWeight: 400 }}>{s.progress}%</div>
                  </div>
                  {i < data.stages.length - 1 && <span style={{ color: "var(--text-secondary)" }}>&rarr;</span>}
                </div>
              ))}
            </div>
            {data.stages.map(s => s.status !== "pending" && (
              <div key={s.name} style={{ marginTop: 8 }}>
                <div style={{ fontSize: 11, fontWeight: 600 }}>{s.name} Metrics</div>
                {Object.entries(s.metrics).map(([k, v]) => (
                  <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: 12, padding: "1px 0" }}><span>{k}</span><span>{v.toFixed(4)}</span></div>
                ))}
              </div>
            ))}
          </div>

          <div style={{ ...cardStyle, display: "flex", gap: 16, justifyContent: "space-around", textAlign: "center" }}>
            <div>
              <div style={{ fontSize: 24, fontWeight: 700, color: data.alignmentTax > 0.1 ? "var(--error-color)" : "var(--success-color)" }}>{(data.alignmentTax * 100).toFixed(1)}%</div>
              <div style={labelStyle}>Alignment Tax</div>
            </div>
            <div>
              <div style={{ fontSize: 14 }}>{data.basePerformance.toFixed(2)} &rarr; {data.alignedPerformance.toFixed(2)}</div>
              <div style={labelStyle}>Performance (base / aligned)</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Reward Model Accuracy ({data.rewardModelAccuracy.length} pts)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 50 }}>
              {data.rewardModelAccuracy.slice(-60).map((v, i) => (
                <div key={i} style={{ flex: 1, height: `${v * 100}%`, background: "var(--info-color)", borderRadius: "2px 2px 0 0" }} />
              ))}
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>KL Divergence ({data.klDivergence.length} pts)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 50 }}>
              {data.klDivergence.slice(-60).map((v, i, a) => {
                const max = Math.max(...a, 0.01);
                return <div key={i} style={{ flex: 1, height: `${(v / max) * 100}%`, background: v > max * 0.7 ? "var(--error-color)" : "var(--warning-color)", borderRadius: "2px 2px 0 0" }} />;
              })}
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Safety Benchmarks</div>
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

      {!data && !loading && <div style={labelStyle}>Enter a Run ID and click Load.</div>}
    </div>
  );
}
