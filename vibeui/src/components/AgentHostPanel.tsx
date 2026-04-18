import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const agentColors = ["var(--accent-color)", "var(--accent-purple)", "var(--error-color)", "var(--warning-color)", "var(--success-color)"];

export function AgentHostPanel() {
  const [tab, setTab] = useState("agents");
  const [agents, setAgents] = useState<HostedAgent[]>([]);
  const [output, setOutput] = useState<OutputLine[]>([]);
  const [clipboard, setClipboard] = useState<ClipboardEntry[]>([]);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [maxAgents, setMaxAgents] = useState(5);
  const [interleave, setInterleave] = useState(true);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const loadOutput = useCallback(async () => {
    try {
      const outputRes = await invoke<{ lines?: OutputLine[] }>("host_get_output", { agentId: "all", lastN: 50 });
      setOutput(Array.isArray(outputRes) ? outputRes : Array.isArray(outputRes?.lines) ? outputRes.lines : []);
    } catch {/* silent */}
  }, []);

  const loadAgents = useCallback(async () => {
    try {
      const agentList = await invoke<HostedAgent[]>("host_list_agents");
      setAgents(Array.isArray(agentList) ? agentList : []);
    } catch {/* silent */}
  }, []);

  const loadClipboard = useCallback(async () => {
    try {
      const data = await invoke<ClipboardEntry[]>("host_get_clipboard");
      setClipboard(Array.isArray(data) ? data : []);
    } catch {/* silent */}
  }, []);

  useEffect(() => {
    (async () => {
      setLoading(true);
      await Promise.all([loadAgents(), loadOutput(), loadClipboard()]);
      setLoading(false);
    })();

    const poll = setInterval(() => { loadAgents(); loadOutput(); loadClipboard(); }, 5_000);

    const unlisten = listen("host:output", () => loadOutput());
    const unlistenStatus = listen("host:status_changed", () => loadAgents());
    const unlistenClip = listen("host:clipboard_changed", () => loadClipboard());

    return () => {
      clearInterval(poll);
      unlisten.then(fn => fn());
      unlistenStatus.then(fn => fn());
      unlistenClip.then(fn => fn());
    };
  }, [loadAgents, loadOutput, loadClipboard]);

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

  const handleSetClipboard = useCallback(async () => {
    if (!newKey.trim() || !newValue.trim()) return;
    try {
      await invoke("host_set_clipboard", { key: newKey.trim(), value: newValue.trim(), agentId: "user" });
      setNewKey("");
      setNewValue("");
      await loadClipboard();
    } catch (e) {
      console.error("host_set_clipboard failed:", e);
    }
  }, [newKey, newValue, loadClipboard]);

  const handleClearClipboard = useCallback(async () => {
    try {
      await invoke("host_clear_clipboard");
      setClipboard([]);
    } catch {/* silent */}
  }, []);

  const statusColor: Record<string, string> = { running: "var(--success-color)", stopped: "var(--text-secondary)", error: "var(--error-color)" };

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Multi-Agent Terminal Host</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        <button className={`panel-tab ${tab === "agents" ? "active" : ""}`} onClick={() => setTab("agents")}>Agents</button>
        <button className={`panel-tab ${tab === "output" ? "active" : ""}`} onClick={() => setTab("output")}>Output</button>
        <button className={`panel-tab ${tab === "context" ? "active" : ""}`} onClick={() => setTab("context")}>
          Context
          {clipboard.length > 0 && (
            <span style={{ marginLeft: 4, background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", borderRadius: "var(--radius-sm-alt)", padding: "0 4px", fontSize: "var(--font-size-xs)" }}>
              {clipboard.length}
            </span>
          )}
        </button>
        <button className={`panel-tab ${tab === "config" ? "active" : ""}`} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "agents" && (
        <div>
          {loading && <div className="panel-loading">Loading agents...</div>}
          {!loading && agents.length === 0 && <div className="panel-empty">No agents configured. Start a new agent to get going.</div>}
          {agents.map((a) => (
            <div key={a.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{a.name}</strong>
                <span style={{ ...badgeStyle("var(--accent-indigo)"), marginLeft: 8 }}>{a.type}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: statusColor[a.status], display: "inline-block" }} />
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{a.status}</span>
                <button className="panel-btn panel-btn-primary" disabled={actionLoading === a.id} onClick={() => toggleAgent(a.id)}>
                  {actionLoading === a.id ? "..." : a.status === "running" ? "Stop" : "Start"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "output" && (
        <div style={{ fontFamily: "monospace", fontSize: "var(--font-size-base)" }}>
          {loading && <div className="panel-loading">Loading output...</div>}
          {!loading && output.length === 0 && <div className="panel-empty">No output yet.</div>}
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
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>Shared Clipboard</span>
            {clipboard.length > 0 && (
              <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-base)" }} onClick={handleClearClipboard}>
                Clear All
              </button>
            )}
          </div>
          {clipboard.length === 0 && <div className="panel-empty">Clipboard is empty.</div>}
          {clipboard.map((c) => (
            <div key={c.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <strong>{c.key}</strong>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>by {c.setBy}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", fontFamily: "monospace", marginTop: 4, color: "var(--text-secondary)", wordBreak: "break-all" }}>{c.value}</div>
            </div>
          ))}
          <div className="panel-card" style={{ marginTop: 8 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>Set Clipboard Entry</div>
            <input className="panel-input panel-input-full" placeholder="Key" style={{ marginBottom: 6 }}
              value={newKey} onChange={(e) => setNewKey(e.target.value)} />
            <input className="panel-input panel-input-full" placeholder="Value"
              value={newValue} onChange={(e) => setNewValue(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleSetClipboard(); }} />
            <button className="panel-btn panel-btn-primary" style={{ marginTop: 8 }}
              disabled={!newKey.trim() || !newValue.trim()} onClick={handleSetClipboard}>
              Set
            </button>
          </div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Max Agents: {maxAgents}</div>
            <input type="range" min={1} max={10} value={maxAgents} onChange={(e) => setMaxAgents(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div className="panel-card">
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
