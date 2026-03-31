/**
 * RLOptimizationReport — Optimization pipeline results panel.
 *
 * Shows pipeline stages (distill, quantize, prune, export) with before/after
 * metrics, compression ratio, latency benchmarks, and reward retention gauge.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface OptimizationReport {
  policyId: string;
  policyName: string;
  stages: PipelineStage[];
  compressionRatio: number;
  latencyBenchmarks: LatencyBenchmark[];
  rewardRetention: number;
  originalReward: number;
  optimizedReward: number;
}

interface PipelineStage {
  name: string;
  status: string;
  beforeSize: number;
  afterSize: number;
  beforeLatency: number;
  afterLatency: number;
  rewardDelta: number;
}

interface LatencyBenchmark {
  device: string;
  p50: number;
  p95: number;
  p99: number;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

const stageIcon = (s: string) => s === "done" ? "#4caf50" : s === "running" ? "#ff9800" : "var(--text-secondary)";

export function RLOptimizationReport() {
  const [policyId, setPolicyId] = useState("");
  const [report, setReport] = useState<OptimizationReport | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchReport = useCallback(async () => {
    if (!policyId) return;
    setLoading(true);
    try {
      const res = await invoke<OptimizationReport>("rl_get_optimization_report", { policyId });
      setReport(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [policyId]);

  const runOptimization = useCallback(async () => {
    if (!policyId) return;
    setLoading(true);
    try {
      await invoke("rl_run_optimization", { policyId });
      fetchReport();
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [policyId, fetchReport]);

  const retentionColor = (r: number) => r >= 0.95 ? "#4caf50" : r >= 0.85 ? "#ff9800" : "#f44336";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Optimization Report</h2>

      <div style={{ ...cardStyle, display: "flex", gap: 8, alignItems: "center" }}>
        <label style={labelStyle}>Policy ID:</label>
        <input value={policyId} onChange={e => setPolicyId(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12 }} />
        <button style={btnStyle} onClick={fetchReport} disabled={loading}>View</button>
        <button style={{ ...btnStyle, background: "var(--accent-blue)", color: "#fff" }} onClick={runOptimization} disabled={loading}>Run Optimization</button>
      </div>

      {loading && <div style={labelStyle}>Loading...</div>}
      {report && !loading && (
        <>
          <div style={{ ...cardStyle, display: "flex", gap: 16, justifyContent: "space-around", textAlign: "center" }}>
            <div>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{report.compressionRatio.toFixed(1)}x</div>
              <div style={labelStyle}>Compression</div>
            </div>
            <div>
              <div style={{ fontSize: 24, fontWeight: 700, color: retentionColor(report.rewardRetention) }}>{(report.rewardRetention * 100).toFixed(1)}%</div>
              <div style={labelStyle}>Reward Retention</div>
            </div>
            <div>
              <div style={{ fontSize: 14 }}>{report.originalReward.toFixed(2)} &rarr; {report.optimizedReward.toFixed(2)}</div>
              <div style={labelStyle}>Reward (before/after)</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Reward Retention Gauge</div>
            <div style={{ height: 10, borderRadius: 5, background: "var(--bg-tertiary)", overflow: "hidden" }}>
              <div style={{ width: `${report.rewardRetention * 100}%`, height: "100%", background: retentionColor(report.rewardRetention), borderRadius: 5 }} />
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Pipeline Stages</div>
            {report.stages.map(s => (
              <div key={s.name} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "6px 0", borderBottom: "1px solid var(--border-color)" }}>
                <span><span style={{ display: "inline-block", width: 8, height: 8, borderRadius: "50%", background: stageIcon(s.status), marginRight: 6 }} />{s.name}</span>
                <span style={labelStyle}>{s.beforeSize}MB &rarr; {s.afterSize}MB | {s.beforeLatency}ms &rarr; {s.afterLatency}ms | reward {s.rewardDelta >= 0 ? "+" : ""}{s.rewardDelta.toFixed(3)}</span>
              </div>
            ))}
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Latency Benchmarks</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Device</th><th style={thStyle}>p50 (ms)</th><th style={thStyle}>p95 (ms)</th><th style={thStyle}>p99 (ms)</th></tr></thead>
              <tbody>
                {report.latencyBenchmarks.map(b => (
                  <tr key={b.device}><td style={tdStyle}>{b.device}</td><td style={tdStyle}>{b.p50.toFixed(1)}</td><td style={tdStyle}>{b.p95.toFixed(1)}</td><td style={tdStyle}>{b.p99.toFixed(1)}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {!report && !loading && <div style={labelStyle}>Enter a Policy ID and click View or Run Optimization.</div>}
    </div>
  );
}
