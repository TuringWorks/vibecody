import { useState, useCallback } from "react";

interface A2aAgent {
  id: string;
  name: string;
  url: string;
  capabilities: string[];
  status: "online" | "offline";
}

interface A2aTask {
  id: string;
  agentId: string;
  description: string;
  status: "Submitted" | "Working" | "Completed" | "Failed";
  createdAt: string;
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
  color: "#fff",
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
  color: "#fff",
  marginRight: 4,
});

const statusColors: Record<string, string> = {
  Submitted: "#6366f1",
  Working: "#f59e0b",
  Completed: "#22c55e",
  Failed: "#ef4444",
};

export function A2aPanel() {
  const [tab, setTab] = useState("agents");
  const [agents, setAgents] = useState<A2aAgent[]>([
    { id: "a1", name: "CodeReview Agent", url: "http://localhost:9100", capabilities: ["review", "lint"], status: "online" },
    { id: "a2", name: "Test Agent", url: "http://localhost:9101", capabilities: ["test", "coverage"], status: "offline" },
  ]);
  const [tasks] = useState<A2aTask[]>([
    { id: "t1", agentId: "a1", description: "Review PR #42", status: "Working", createdAt: "2026-03-26T10:00:00Z" },
    { id: "t2", agentId: "a2", description: "Run integration tests", status: "Completed", createdAt: "2026-03-26T09:30:00Z" },
  ]);
  const [registryUrl, setRegistryUrl] = useState("http://localhost:9000/.well-known/agent.json");
  const [cardName, setCardName] = useState("");
  const [cardUrl, setCardUrl] = useState("");

  const handleDiscover = useCallback(() => {
    setAgents((prev) => [...prev, { id: `a${Date.now()}`, name: "Discovered Agent", url: registryUrl, capabilities: ["general"], status: "online" }]);
  }, [registryUrl]);

  const handleAddCard = useCallback(() => {
    if (!cardName || !cardUrl) return;
    setAgents((prev) => [...prev, { id: `a${Date.now()}`, name: cardName, url: cardUrl, capabilities: [], status: "online" }]);
    setCardName("");
    setCardUrl("");
  }, [cardName, cardUrl]);

  const completed = tasks.filter((t) => t.status === "Completed").length;
  const failed = tasks.filter((t) => t.status === "Failed").length;
  const total = tasks.length;
  const successRate = total > 0 ? ((completed / total) * 100).toFixed(1) : "0.0";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>A2A Protocol</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "agents")} onClick={() => setTab("agents")}>Agents</button>
        <button style={tabStyle(tab === "tasks")} onClick={() => setTab("tasks")}>Tasks</button>
        <button style={tabStyle(tab === "discovery")} onClick={() => setTab("discovery")}>Discovery</button>
        <button style={tabStyle(tab === "metrics")} onClick={() => setTab("metrics")}>Metrics</button>
      </div>

      {tab === "agents" && (
        <div>
          <button style={btnStyle} onClick={handleDiscover}>Discover Agents</button>
          <div style={{ marginTop: 12 }}>
            {agents.map((a) => (
              <div key={a.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <strong>{a.name}</strong>
                  <span style={badgeStyle(a.status === "online" ? "#22c55e" : "#6b7280")}>{a.status}</span>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>{a.url}</div>
                <div style={{ marginTop: 6 }}>{a.capabilities.map((c) => <span key={c} style={badgeStyle("#6366f1")}>{c}</span>)}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {tab === "tasks" && (
        <div>
          {tasks.map((t) => (
            <div key={t.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong>{t.description}</strong>
                <span style={badgeStyle(statusColors[t.status])}>{t.status}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Agent: {t.agentId} | Created: {t.createdAt}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "discovery" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Registry URL</div>
            <input style={{ width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }} value={registryUrl} onChange={(e) => setRegistryUrl(e.target.value)} />
            <button style={{ ...btnStyle, marginTop: 8 }} onClick={handleDiscover}>Discover</button>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Register Agent Card</div>
            <input placeholder="Agent name" style={{ width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13, marginBottom: 8 }} value={cardName} onChange={(e) => setCardName(e.target.value)} />
            <input placeholder="Agent URL" style={{ width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13, marginBottom: 8 }} value={cardUrl} onChange={(e) => setCardUrl(e.target.value)} />
            <button style={btnStyle} onClick={handleAddCard}>Register</button>
          </div>
        </div>
      )}

      {tab === "metrics" && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Tasks Created</div><div style={{ fontSize: 24, fontWeight: 700 }}>{total}</div></div>
            <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Completed</div><div style={{ fontSize: 24, fontWeight: 700, color: "#22c55e" }}>{completed}</div></div>
            <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Failed</div><div style={{ fontSize: 24, fontWeight: 700, color: "#ef4444" }}>{failed}</div></div>
            <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Success Rate</div><div style={{ fontSize: 24, fontWeight: 700 }}>{successRate}%</div></div>
          </div>
        </div>
      )}
    </div>
  );
}
