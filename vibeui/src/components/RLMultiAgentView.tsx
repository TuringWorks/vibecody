/**
 * RLMultiAgentView — Multi-agent dashboard.
 *
 * Per-agent reward bars, communication pattern graph (adjacency matrix),
 * coalition groups, and ELO rankings table for league training.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MultiAgentMetrics {
  agents: AgentInfo[];
  communicationMatrix: number[][];
  coalitions: Coalition[];
  eloRankings: EloEntry[];
}

interface AgentInfo {
  id: string;
  name: string;
  reward: number;
  episodes: number;
  winRate: number;
}

interface Coalition {
  id: string;
  members: string[];
  groupReward: number;
}

interface EloEntry {
  agentId: string;
  agentName: string;
  elo: number;
  wins: number;
  losses: number;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

export function RLMultiAgentView() {
  const [metrics, setMetrics] = useState<MultiAgentMetrics | null>(null);
  const [loading, setLoading] = useState(false);

  const fetch = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<MultiAgentMetrics>("rl_get_multi_agent_metrics");
      setMetrics(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  useEffect(() => { fetch(); }, [fetch]);

  const maxReward = metrics ? Math.max(...metrics.agents.map(a => Math.abs(a.reward)), 0.01) : 1;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Multi-Agent Dashboard</h2>
      <button style={btnStyle} onClick={fetch} disabled={loading}>{loading ? "..." : "Refresh"}</button>

      {metrics && (
        <>
          <div style={{ ...cardStyle, marginTop: 10 }}>
            <div style={labelStyle}>Per-Agent Rewards</div>
            {metrics.agents.map(a => (
              <div key={a.id} style={{ marginBottom: 6 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 2 }}>
                  <span style={{ fontWeight: 600 }}>{a.name}</span>
                  <span>{a.reward.toFixed(2)} ({a.episodes} eps, {(a.winRate * 100).toFixed(0)}% win)</span>
                </div>
                <div style={{ height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" }}>
                  <div style={{ width: `${(Math.abs(a.reward) / maxReward) * 100}%`, height: "100%", background: a.reward >= 0 ? "var(--success-color)" : "var(--error-color)", borderRadius: 4 }} />
                </div>
              </div>
            ))}
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Communication Matrix</div>
            <div style={{ overflowX: "auto" }}>
              <table style={tableStyle}>
                <thead>
                  <tr>
                    <th style={thStyle}></th>
                    {metrics.agents.map(a => <th key={a.id} style={thStyle}>{(a.name || a.id).slice(0, 8)}</th>)}
                  </tr>
                </thead>
                <tbody>
                  {metrics.communicationMatrix.map((row, i) => (
                    <tr key={i}>
                      <td style={{ ...tdStyle, fontWeight: 600 }}>{(metrics.agents[i]?.name || metrics.agents[i]?.id || "?").slice(0, 8)}</td>
                      {row.map((v, j) => {
                        const intensity = Math.min(v / Math.max(...row, 1), 1);
                        return <td key={j} style={{ ...tdStyle, background: `rgba(33,150,243,${intensity * 0.5})`, textAlign: "center" }}>{v.toFixed(0)}</td>;
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Coalitions</div>
            {metrics.coalitions.map(c => (
              <div key={c.id} style={{ padding: "4px 0", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600 }}>{c.id}</span>
                <span style={labelStyle}>{c.members.join(", ")} | reward: {c.groupReward.toFixed(2)}</span>
              </div>
            ))}
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>ELO Rankings</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Rank</th><th style={thStyle}>Agent</th><th style={thStyle}>ELO</th><th style={thStyle}>W</th><th style={thStyle}>L</th></tr></thead>
              <tbody>
                {metrics.eloRankings.sort((a, b) => b.elo - a.elo).map((e, i) => (
                  <tr key={e.agentId}>
                    <td style={tdStyle}>{i + 1}</td>
                    <td style={tdStyle}>{e.agentName}</td>
                    <td style={{ ...tdStyle, fontWeight: 700 }}>{e.elo.toFixed(0)}</td>
                    <td style={tdStyle}>{e.wins}</td>
                    <td style={tdStyle}>{e.losses}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {!metrics && !loading && <div style={labelStyle}>Loading multi-agent metrics...</div>}
    </div>
  );
}
