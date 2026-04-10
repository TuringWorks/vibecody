import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown } from "lucide-react";

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
  const [sessions, setSessions] = useState<RepairSession[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newFile, setNewFile] = useState("");
  const [newError, setNewError] = useState("");
  const [newStrategy, setNewStrategy] = useState("mcts");
  const [treeNodes, setTreeNodes] = useState<TreeNode[]>([]);
  const [treeLoading, setTreeLoading] = useState(false);

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

  // Auto-select first session
  useEffect(() => {
    if (sessions.length > 0 && !selectedId) {
      setSelectedId(sessions[0].id);
    }
  }, [sessions, selectedId]);

  // Fetch tree nodes whenever tree tab is active with a selected session
  useEffect(() => {
    if (tab !== "tree" || !selectedId) return;
    setTreeLoading(true);
    invoke<TreeNode[]>("mcts_get_tree", { sessionId: selectedId })
      .then((data) => setTreeNodes(Array.isArray(data) ? data : []))
      .catch(() => setTreeNodes([]))
      .finally(() => setTreeLoading(false));
  }, [tab, selectedId]);

  const handleCreate = useCallback(async () => {
    if (!newFile.trim() || !newError.trim()) return;
    try {
      const created = await invoke<any>("mcts_create_session", { file: newFile, errorMsg: newError, strategy: newStrategy });
      setNewFile("");
      setNewError("");
      await fetchSessions();
      if (created?.id) setSelectedId(String(created.id));
      setTab("sessions");
    } catch (e) {
      console.error("mcts_create_session failed:", e);
    }
  }, [newFile, newError, newStrategy, fetchSessions]);

  const selectedSession = sessions.find((s) => s.id === selectedId);

  // Derive agentless phases from selected session status
  const phases = selectedSession ? [
    {
      name: "Localize",
      status: "done",
      detail: `Found candidate locations in ${selectedSession.file || "target file"}`,
    },
    {
      name: "Repair",
      status: selectedSession.status === "running" ? "running" : "done",
      detail: selectedSession.status === "running" ? "Generating patches…" : "Generated patches",
    },
    {
      name: "Validate",
      status: selectedSession.status === "success" ? "done" : selectedSession.status === "failed" ? "failed" : selectedSession.status === "running" ? "pending" : "done",
      detail: selectedSession.status === "success" ? "All patches validated" : selectedSession.status === "failed" ? "Validation failed" : "Waiting for repair",
    },
  ] : [
    { name: "Localize", status: "pending", detail: "Select a session to see progress" },
    { name: "Repair",   status: "pending", detail: "" },
    { name: "Validate", status: "pending", detail: "" },
  ];

  const comparison = [
    { strategy: "MCTS",      avgNodes: 24, successRate: "78%", avgTime: "12s", quality: "High" },
    { strategy: "Agentless", avgNodes: 3,  successRate: "65%", avgTime: "4s",  quality: "Medium" },
    { strategy: "Linear",    avgNodes: 8,  successRate: "52%", avgTime: "8s",  quality: "Low" },
  ];

  const phaseColor: Record<string, string> = {
    done: "var(--success-color)", running: "var(--accent-color)",
    pending: "var(--text-secondary)", failed: "var(--error-color)",
  };

  if (loading) return <div className="panel-container"><div className="panel-loading">Loading repair sessions...</div></div>;
  if (error) return <div className="panel-container"><div className="panel-error">Error: {error}</div></div>;

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>MCTS Code Repair</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        <button className={`panel-tab ${tab === "sessions" ? "active" : ""}`} onClick={() => setTab("sessions")}>Sessions</button>
        <button className={`panel-tab ${tab === "new" ? "active" : ""}`} onClick={() => setTab("new")}>New</button>
        <button className={`panel-tab ${tab === "tree" ? "active" : ""}`} onClick={() => setTab("tree")}>Tree</button>
        <button className={`panel-tab ${tab === "agentless" ? "active" : ""}`} onClick={() => setTab("agentless")}>Agentless</button>
        <button className={`panel-tab ${tab === "compare" ? "active" : ""}`} onClick={() => setTab("compare")}>Compare</button>
      </div>

      {tab === "sessions" && (
        <div>
          {sessions.length === 0 && <div className="panel-empty">No repair sessions yet. Create one from the New tab.</div>}
          {sessions.map((s) => (
            <div key={s.id} className="panel-card"
              style={{ cursor: "pointer", outline: s.id === selectedId ? "2px solid var(--accent-color)" : "none" }}
              onClick={() => setSelectedId(s.id)}>
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
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Create Repair Session</div>
          <input placeholder="File path (e.g. src/auth.rs)" className="panel-input panel-input-full" style={{ marginBottom: 8 }} value={newFile} onChange={(e) => setNewFile(e.target.value)} />
          <input placeholder="Error message" className="panel-input panel-input-full" style={{ marginBottom: 8 }} value={newError} onChange={(e) => setNewError(e.target.value)} />
          <select value={newStrategy} onChange={(e) => setNewStrategy(e.target.value)} className="panel-select" style={{ marginBottom: 8 }}>
            <option value="mcts">MCTS</option>
            <option value="agentless">Agentless</option>
            <option value="linear">Linear</option>
          </select>
          <div style={{ marginTop: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={handleCreate} disabled={!newFile.trim() || !newError.trim()}>Create Session</button>
          </div>
        </div>
      )}

      {tab === "tree" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>MCTS Tree Visualization</span>
            {selectedSession && (
              <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                {selectedSession.file} · {selectedSession.nodesExplored} nodes
              </span>
            )}
          </div>
          {!selectedId && <div className="panel-empty">Select a session from the Sessions tab first.</div>}
          {selectedId && treeLoading && <div className="panel-loading">Loading tree…</div>}
          {selectedId && !treeLoading && treeNodes.length === 0 && (
            <div className="panel-empty">No tree data yet. Session needs nodes explored.</div>
          )}
          {treeNodes.map((n, i) => (
            <div key={n.id} className="panel-card" style={{ marginLeft: i * 8, borderLeft: n.isBestPath ? "3px solid var(--success-color)" : "3px solid var(--border-color)" }}>
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
          {!selectedSession && (
            <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: 13 }}>
              Select a session from the Sessions tab to see its phase progress.
            </div>
          )}
          {phases.map((p, i) => (
            <div key={i} className="panel-card" style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span style={{ width: 10, height: 10, borderRadius: "50%", background: phaseColor[p.status] || "var(--text-secondary)", flexShrink: 0 }} />
              <div style={{ flex: 1 }}>
                <strong>{p.name}</strong>
                {p.detail && <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{p.detail}</div>}
              </div>
              <span style={{ fontSize: 11, color: phaseColor[p.status] || "var(--text-secondary)" }}>{p.status}</span>
              {i < phases.length - 1 && <ChevronDown size={14} strokeWidth={1.5} style={{ color: "var(--text-secondary)", flexShrink: 0 }} />}
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
