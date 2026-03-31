/**
 * RLTrainingDashboard — Real-time RL training monitoring.
 *
 * Displays reward curves, loss plots, GPU utilization, episode stats,
 * and active training runs with start/stop/pause controls.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TrainingRun {
  id: string;
  name: string;
  status: string;
  algorithm: string;
  environment: string;
  startedAt: number;
  episodes: number;
  currentReward: number;
}

interface TrainingMetrics {
  runId: string;
  rewards: number[];
  losses: number[];
  gpuUtil: number[];
  episodeStats: EpisodeStat[];
}

interface EpisodeStat {
  episode: number;
  reward: number;
  length: number;
  loss: number;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

const statusColor = (s: string) => s === "running" ? "#4caf50" : s === "paused" ? "#ff9800" : "var(--text-secondary)";

export function RLTrainingDashboard() {
  const [runs, setRuns] = useState<TrainingRun[]>([]);
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<TrainingMetrics | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchRuns = useCallback(async () => {
    try {
      const res = await invoke<TrainingRun[]>("rl_list_training_runs");
      setRuns(res);
    } catch (e) { console.error(e); }
  }, []);

  useEffect(() => { fetchRuns(); }, [fetchRuns]);

  const fetchMetrics = useCallback(async (runId: string) => {
    setLoading(true);
    try {
      const res = await invoke<TrainingMetrics>("rl_get_training_metrics", { runId });
      setMetrics(res);
      setSelectedRun(runId);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  const handleAction = useCallback(async (action: "start" | "stop", runId: string) => {
    try {
      if (action === "start") await invoke("rl_start_training", { runId });
      else await invoke("rl_stop_training", { runId });
      fetchRuns();
    } catch (e) { console.error(e); }
  }, [fetchRuns]);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>RL Training Dashboard</h2>
      <button style={btnStyle} onClick={fetchRuns}>Refresh</button>

      <div style={{ ...cardStyle, marginTop: 10 }}>
        <div style={labelStyle}>Active Training Runs</div>
        <table style={tableStyle}>
          <thead><tr><th style={thStyle}>Name</th><th style={thStyle}>Algorithm</th><th style={thStyle}>Status</th><th style={thStyle}>Episodes</th><th style={thStyle}>Reward</th><th style={thStyle}>Actions</th></tr></thead>
          <tbody>
            {runs.map(r => (
              <tr key={r.id} style={{ cursor: "pointer", background: selectedRun === r.id ? "var(--bg-tertiary)" : undefined }} onClick={() => fetchMetrics(r.id)}>
                <td style={tdStyle}>{r.name}</td>
                <td style={tdStyle}>{r.algorithm}</td>
                <td style={tdStyle}><span style={{ color: statusColor(r.status) }}>{r.status}</span></td>
                <td style={tdStyle}>{r.episodes}</td>
                <td style={tdStyle}>{r.currentReward.toFixed(2)}</td>
                <td style={tdStyle}>
                  {r.status === "running" ? <button style={btnStyle} onClick={e => { e.stopPropagation(); handleAction("stop", r.id); }}>Stop</button> : <button style={btnStyle} onClick={e => { e.stopPropagation(); handleAction("start", r.id); }}>Start</button>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {runs.length === 0 && <div style={labelStyle}>No training runs found.</div>}
      </div>

      {loading && <div style={labelStyle}>Loading metrics...</div>}
      {metrics && !loading && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Reward Curve ({metrics.rewards.length} pts)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 60 }}>
              {metrics.rewards.slice(-80).map((v, i, a) => {
                const max = Math.max(...a, 1);
                return <div key={i} style={{ flex: 1, background: "var(--accent-blue)", height: `${(v / max) * 100}%`, borderRadius: "2px 2px 0 0" }} />;
              })}
            </div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>GPU Utilization</div>
            <div style={{ display: "flex", gap: 4 }}>
              {metrics.gpuUtil.map((g, i) => (
                <div key={i} style={{ flex: 1, textAlign: "center" }}>
                  <div style={{ height: 40, background: "var(--bg-tertiary)", borderRadius: 4, position: "relative", overflow: "hidden" }}>
                    <div style={{ position: "absolute", bottom: 0, width: "100%", height: `${g}%`, background: g > 80 ? "#4caf50" : "#ff9800", borderRadius: 4 }} />
                  </div>
                  <div style={{ ...labelStyle, marginTop: 2 }}>{g}%</div>
                </div>
              ))}
            </div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>Episode Stats</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Episode</th><th style={thStyle}>Reward</th><th style={thStyle}>Length</th><th style={thStyle}>Loss</th></tr></thead>
              <tbody>
                {metrics.episodeStats.slice(-10).map(s => (
                  <tr key={s.episode}><td style={tdStyle}>{s.episode}</td><td style={tdStyle}>{s.reward.toFixed(2)}</td><td style={tdStyle}>{s.length}</td><td style={tdStyle}>{s.loss.toFixed(4)}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
