import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HostedAgent {
  id: string;
  name: string;
  type: string;
  status: "running" | "stopped" | "error";
}

interface OutputLine {
  agentId: string;
  agentName: string;
  text: string;
  timestamp: string;
  color: string;
}

interface ClipboardEntry {
  id: string;
  key: string;
  value: string;
  setBy: string;
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

const agentColors = ["var(--accent-color)", "var(--accent-purple)", "#ec4899", "var(--warning-color)", "var(--success-color)"];

export function AgentHostPanel() {
  const [tab, setTab] = useState("agents");
  const [agents, setAgents] = useState<HostedAgent[]>([]);
  const [output, setOutput] = useState<OutputLine[]>([]);
  const [clipboard] = useState<ClipboardEntry[]>([]);
  const [maxAgents, setMaxAgents] = useState(5);
  const [interleave, setInterleave] = useState(true);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  useEffect(() => {
    async function loadData() {
      setLoading(true);
      try {
        const agentList = await invoke<HostedAgent[]>("host_list_agents");
        setAgents(Array.isArray(agentList) ? agentList : []);
      } catch (e) {
        console.error("Failed to load agents:", e);
      }
      try {
        const outputRes = await invoke<{ lines?: OutputLine[] }>("host_get_output", { agentId: "all", lastN: 50 });
        setOutput(Array.isArray(outputRes) ? outputRes : Array.isArray(outputRes?.lines) ? outputRes.lines : []);
      } catch (e) {
        console.error("Failed to load output:", e);
      }
      setLoading(false);
    }
    loadData();
  }, []);

  const toggleAgent = useCallback(async (id: string) => {
    const agent = agents.find((a) => a.id === id);
    if (!agent) return;
    setActionLoading(id);
    try {
      if (agent.status === "running") {
        await invoke("host_stop", { agentId: id });
        setAgents((prev) => prev.map((a) => a.id === id ? { ...a, status: "stopped" as const } : a));
      } else {
        await invoke("host_start", { agentId: id });
        setAgents((prev) => prev.map((a) => a.id === id ? { ...a, status: "running" as const } : a));
      }
    } catch (e) {
      console.error("Failed to toggle agent:", e);
    }
    setActionLoading(null);
  }, [agents]);

  const statusColor: Record<string, string> = { running: "var(--success-color)", stopped: "var(--text-secondary)", error: "var(--error-color)" };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Multi-Agent Terminal Host</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "agents")} onClick={() => setTab("agents")}>Agents</button>
        <button style={tabStyle(tab === "output")} onClick={() => setTab("output")}>Output</button>
        <button style={tabStyle(tab === "context")} onClick={() => setTab("context")}>Context</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "agents" && (
        <div>
          {loading && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading agents...</div>}
          {!loading && agents.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No agents configured. Start a new agent to get going.</div>}
          {agents.map((a) => (
            <div key={a.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{a.name}</strong>
                <span style={{ ...badgeStyle("#6366f1"), marginLeft: 8 }}>{a.type}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: statusColor[a.status], display: "inline-block" }} />
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{a.status}</span>
                <button style={btnStyle} disabled={actionLoading === a.id} onClick={() => toggleAgent(a.id)}>
                  {actionLoading === a.id ? "..." : a.status === "running" ? "Stop" : "Start"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "output" && (
        <div style={{ fontFamily: "monospace", fontSize: 12 }}>
          {loading && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading output...</div>}
          {!loading && output.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No output yet.</div>}
          {output.map((line, i) => (
            <div key={i} style={{ padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
              <span style={{ color: "var(--text-secondary)", marginRight: 8 }}>{line.timestamp}</span>
              <span style={{ color: line.color || agentColors[i % agentColors.length], fontWeight: 600, marginRight: 8 }}>[{line.agentName}]</span>
              <span>{line.text}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "context" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Shared Clipboard</div>
          {clipboard.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Clipboard is empty.</div>}
          {clipboard.map((c) => (
            <div key={c.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <strong>{c.key}</strong>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>by {c.setBy}</span>
              </div>
              <div style={{ fontSize: 12, fontFamily: "monospace", marginTop: 4, color: "var(--text-secondary)" }}>{c.value}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Max Agents: {maxAgents}</div>
            <input type="range" min={1} max={10} value={maxAgents} onChange={(e) => setMaxAgents(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
              <input type="checkbox" checked={interleave} onChange={(e) => setInterleave(e.target.checked)} />
              <span style={{ fontWeight: 600 }}>Interleave output from all agents</span>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}
