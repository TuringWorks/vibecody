/**
 * CompanyAgentDetailPanel — Full agent profile view.
 *
 * Shows agent info: title, role, skills, budget, adapter config,
 * and recent heartbeat runs. Supports firing the agent.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyAgentDetailPanelProps {
  workspacePath?: string | null;
}

export function CompanyAgentDetailPanel({ workspacePath: _wp }: CompanyAgentDetailPanelProps) {
  const [agentId, setAgentId] = useState("");
  const [agentInfo, setAgentInfo] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [allAgents, setAllAgents] = useState<string>("");

  useEffect(() => {
    invoke<string>("company_agent_list")
      .then(setAllAgents)
      .catch(() => {});
  }, []);

  const loadAgent = async () => {
    if (!agentId.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const info = await invoke<string>("company_agent_info", { id: agentId.trim() });
      setAgentInfo(info);
    } catch (e) {
      setError(String(e));
      setAgentInfo(null);
    } finally {
      setLoading(false);
    }
  };

  const fireAgent = async () => {
    if (!agentId.trim()) return;
    if (!confirm(`Terminate agent ${agentId}?`)) return;
    try {
      const out = await invoke<string>("company_agent_fire", { id: agentId.trim() });
      setAgentInfo(out);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="panel-container">
      <div className="panel-header">Agent Detail</div>
      <div className="panel-body" style={{ fontSize: 13 }}>

      {/* Agent list */}
      <div className="panel-card" style={{ marginBottom: 16, maxHeight: 200, overflowY: "auto" }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>All Agents</div>
        <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
          {allAgents || "No agents. Use /company agent hire <name>"}
        </pre>
      </div>

      {/* Lookup form */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          value={agentId}
          onChange={(e) => setAgentId(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && loadAgent()}
          placeholder="Agent ID (first 8 chars ok)"
          style={{
            flex: 1,
            fontSize: 12,
            padding: "4px 8px",
            background: "var(--bg-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            color: "var(--text-primary)",
          }}
        />
        <button onClick={loadAgent} disabled={loading} className="panel-btn panel-btn-secondary">
          {loading ? "…" : "Lookup"}
        </button>
        <button onClick={fireAgent} className="panel-btn panel-btn-danger">
          Fire
        </button>
      </div>

      {error && <div className="panel-error">{error}</div>}

      {agentInfo && (
        <div className="panel-card">
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
            {agentInfo}
          </pre>
        </div>
      )}
      </div>
    </div>
  );
}
