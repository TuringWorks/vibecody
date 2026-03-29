import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RepairSession {
  id: string;
  file: string;
  error: string;
  status: "running" | "success" | "failed";
  strategy: "mcts" | "agentless" | "linear";
  nodesExplored: number;
  depth: number;
}

interface TreeNode {
  id: string;
  label: string;
  visits: number;
  reward: number;
  children: number;
  isBestPath: boolean;
}

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const statusColor: Record<string, string> = { running: "var(--accent-color)", success: "var(--success-color)", failed: "var(--error-color)" };
const stratColor: Record<string, string> = { mcts: "var(--accent-purple)", agentless: "var(--warning-color)", linear: "var(--text-secondary)" };

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: 8,
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: 13,
  marginBottom: 8,
};

export function MctsRepairPanel() {
  const [tab, setTab] = useState("sessions");
  const [sessions, setSessions] = useState<RepairSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newFile, setNewFile] = useState("");
  const [newError, setNewError] = useState("");
  const [newStrategy, setNewStrategy] = useState("mcts");
  const [treeNodes] = useState<TreeNode[]>([]);

  const fetchSessions = useCallback(async () => {
    try {
      const data = await invoke<unknown>("mcts_list_sessions");
      const list = Array.isArray(data) ? data : [];
      setSessions(list.map((s: any) => ({
        id: String(s.id),
        file: s.file || "",
        error: s.error || "",
        status: s.status || "running",
        strategy: s.strategy || "mcts",
        nodesExplored: s.nodesExplored ?? s.nodes_explored ?? 0,
        depth: s.depth ?? 0,
      })));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchSessions().finally(() => setLoading(false));
  }, [fetchSessions]);

  const handleCreate = useCallback(async () => {
    if (!newFile.trim() || !newError.trim()) return;
    try {
      await invoke("mcts_create_session", { file: newFile, errorMsg: newError, strategy: newStrategy });
      setNewFile("");
      setNewError("");
      await fetchSessions();
    } catch (e) {
      console.error("mcts_create_session failed:", e);
    }
  }, [newFile, newError, newStrategy, fetchSessions]);

  const phases = [
    { name: "Localize", status: "done", detail: "Found candidate locations" },
    { name: "Repair", status: "done", detail: "Generated patches" },
    { name: "Validate", status: "running", detail: "Testing patches..." },
  ];

  const comparison = [
    { strategy: "MCTS", avgNodes: 24, successRate: "78%", avgTime: "12s", quality: "High" },
    { strategy: "Agentless", avgNodes: 3, successRate: "65%", avgTime: "4s", quality: "Medium" },
    { strategy: "Linear", avgNodes: 8, successRate: "52%", avgTime: "8s", quality: "Low" },
  ];

  const phaseColor: Record<string, string> = { done: "var(--success-color)", running: "var(--accent-color)", pending: "var(--text-secondary)" };

  if (loading) return <div style={panelStyle}><div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading repair sessions...</div></div>;
  if (error) return <div style={panelStyle}><div style={{ color: "var(--error-color)", fontSize: 13 }}>Error: {error}</div></div>;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCTS Code Repair</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "sessions")} onClick={() => setTab("sessions")}>Sessions</button>
        <button style={tabStyle(tab === "new")} onClick={() => setTab("new")}>New</button>
        <button style={tabStyle(tab === "tree")} onClick={() => setTab("tree")}>Tree</button>
        <button style={tabStyle(tab === "agentless")} onClick={() => setTab("agentless")}>Agentless</button>
        <button style={tabStyle(tab === "compare")} onClick={() => setTab("compare")}>Compare</button>
      </div>

      {tab === "sessions" && (
        <div>
          {sessions.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No repair sessions yet. Create one from the New tab.</div>}
          {sessions.map((s) => (
            <div key={s.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{s.file}</strong>
                <div>
                  <span style={badgeStyle(stratColor[s.strategy] || "var(--text-secondary)")}>{s.strategy}</span>
                  <span style={badgeStyle(statusColor[s.status] || "var(--text-secondary)")}>{s.status}</span>
                </div>
              </div>
              <div style={{ fontSize: 12, fontFamily: "monospace", color: "var(--error-color)", marginBottom: 4 }}>{s.error}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Nodes: {s.nodesExplored} | Depth: {s.depth}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "new" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Create Repair Session</div>
          <input placeholder="File path (e.g. src/auth.rs)" style={inputStyle} value={newFile} onChange={(e) => setNewFile(e.target.value)} />
          <input placeholder="Error message" style={inputStyle} value={newError} onChange={(e) => setNewError(e.target.value)} />
          <select value={newStrategy} onChange={(e) => setNewStrategy(e.target.value)} style={{ ...inputStyle, width: "auto" }}>
            <option value="mcts">MCTS</option>
            <option value="agentless">Agentless</option>
            <option value="linear">Linear</option>
          </select>
          <div style={{ marginTop: 8 }}>
            <button style={btnStyle} onClick={handleCreate} disabled={!newFile.trim() || !newError.trim()}>Create Session</button>
          </div>
        </div>
      )}

      {tab === "tree" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>MCTS Tree Visualization</div>
          {treeNodes.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No tree data available. Start a repair session to populate.</div>}
          {treeNodes.map((n, i) => (
            <div key={n.id} style={{ ...cardStyle, marginLeft: i * 16, borderLeft: n.isBestPath ? "3px solid #22c55e" : "3px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: 13 }}>{n.label}</strong>
                {n.isBestPath && <span style={badgeStyle("var(--success-color)")}>best path</span>}
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
                Visits: {n.visits} | Reward: {n.reward.toFixed(2)} | Children: {n.children}
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "agentless" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 12 }}>3-Phase Pipeline</div>
          {phases.map((p, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 12 }}>
              <span style={{ width: 10, height: 10, borderRadius: "50%", background: phaseColor[p.status], flexShrink: 0 }} />
              <div>
                <strong>{p.name}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{p.detail}</div>
              </div>
              {i < phases.length - 1 && <span style={{ fontSize: 18, color: "var(--text-secondary)", marginLeft: "auto" }}>&darr;</span>}
            </div>
          ))}
        </div>
      )}

      {tab === "compare" && (
        <div>
          <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
                {["Strategy", "Avg Nodes", "Success Rate", "Avg Time", "Quality"].map((h) => (
                  <th key={h} style={{ textAlign: "left", padding: 8 }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {comparison.map((c) => (
                <tr key={c.strategy} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: 8 }}><span style={badgeStyle(stratColor[c.strategy.toLowerCase()] || "var(--text-secondary)")}>{c.strategy}</span></td>
                  <td style={{ padding: 8 }}>{c.avgNodes}</td>
                  <td style={{ padding: 8 }}>{c.successRate}</td>
                  <td style={{ padding: 8 }}>{c.avgTime}</td>
                  <td style={{ padding: 8 }}>{c.quality}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
