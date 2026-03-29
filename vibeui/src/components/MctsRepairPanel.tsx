import { useState } from "react";

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

export function MctsRepairPanel() {
  const [tab, setTab] = useState("sessions");
  const [sessions] = useState<RepairSession[]>([
    { id: "s1", file: "src/auth.rs", error: "type mismatch: expected &str, found String", status: "running", strategy: "mcts", nodesExplored: 24, depth: 5 },
    { id: "s2", file: "src/api.rs", error: "unresolved import `serde_json`", status: "success", strategy: "agentless", nodesExplored: 3, depth: 3 },
    { id: "s3", file: "src/db.rs", error: "cannot borrow as mutable", status: "failed", strategy: "linear", nodesExplored: 8, depth: 4 },
  ]);
  const [treeNodes] = useState<TreeNode[]>([
    { id: "n1", label: "Root: type mismatch", visits: 24, reward: 0.65, children: 3, isBestPath: true },
    { id: "n2", label: "Add .as_str()", visits: 12, reward: 0.82, children: 2, isBestPath: true },
    { id: "n3", label: "Change param type", visits: 8, reward: 0.45, children: 1, isBestPath: false },
    { id: "n4", label: "Clone + convert", visits: 4, reward: 0.30, children: 0, isBestPath: false },
    { id: "n5", label: "Add .to_string()", visits: 10, reward: 0.91, children: 0, isBestPath: true },
  ]);

  const phases = [
    { name: "Localize", status: "done", detail: "Found 2 candidate locations" },
    { name: "Repair", status: "done", detail: "Generated 3 patches" },
    { name: "Validate", status: "running", detail: "Testing patch #2..." },
  ];

  const comparison = [
    { strategy: "MCTS", avgNodes: 24, successRate: "78%", avgTime: "12s", quality: "High" },
    { strategy: "Agentless", avgNodes: 3, successRate: "65%", avgTime: "4s", quality: "Medium" },
    { strategy: "Linear", avgNodes: 8, successRate: "52%", avgTime: "8s", quality: "Low" },
  ];

  const phaseColor: Record<string, string> = { done: "var(--success-color)", running: "var(--accent-color)", pending: "var(--text-secondary)" };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCTS Code Repair</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "sessions")} onClick={() => setTab("sessions")}>Sessions</button>
        <button style={tabStyle(tab === "tree")} onClick={() => setTab("tree")}>Tree</button>
        <button style={tabStyle(tab === "agentless")} onClick={() => setTab("agentless")}>Agentless</button>
        <button style={tabStyle(tab === "compare")} onClick={() => setTab("compare")}>Compare</button>
      </div>

      {tab === "sessions" && (
        <div>
          {sessions.map((s) => (
            <div key={s.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{s.file}</strong>
                <div>
                  <span style={badgeStyle(stratColor[s.strategy])}>{s.strategy}</span>
                  <span style={badgeStyle(statusColor[s.status])}>{s.status}</span>
                </div>
              </div>
              <div style={{ fontSize: 12, fontFamily: "monospace", color: "var(--error-color)", marginBottom: 4 }}>{s.error}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Nodes: {s.nodesExplored} | Depth: {s.depth}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "tree" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>MCTS Tree Visualization</div>
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
                  <td style={{ padding: 8 }}><span style={badgeStyle(stratColor[c.strategy.toLowerCase()])}>{c.strategy}</span></td>
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
