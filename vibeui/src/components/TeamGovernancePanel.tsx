import React, { useState } from "react";

interface PluginEntry {
  id: string;
  name: string;
  version: string;
  visibility: "Private" | "TeamOnly" | "Org" | "Public";
  status: "Pending" | "Approved" | "Rejected" | "Deprecated";
  author: string;
}

interface ApprovalRequest {
  id: string;
  pluginName: string;
  requestedBy: string;
  reviewer: string;
  reason: string;
  date: string;
}

const TeamGovernancePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("plugins");
  const [plugins] = useState<PluginEntry[]>([
    { id: "1", name: "code-formatter", version: "1.2.0", visibility: "Org", status: "Approved", author: "alice" },
    { id: "2", name: "lint-rules", version: "0.9.1", visibility: "TeamOnly", status: "Pending", author: "bob" },
    { id: "3", name: "deploy-helper", version: "2.0.0", visibility: "Public", status: "Approved", author: "carol" },
    { id: "4", name: "legacy-bridge", version: "0.3.0", visibility: "Private", status: "Deprecated", author: "dave" },
  ]);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([
    { id: "a1", pluginName: "lint-rules", requestedBy: "bob", reviewer: "alice", reason: "Team standardization", date: "2026-03-08" },
    { id: "a2", pluginName: "metrics-exporter", requestedBy: "eve", reviewer: "", reason: "Observability integration", date: "2026-03-09" },
  ]);
  const [requireApproval, setRequireApproval] = useState(true);
  const [allowedCategories, setAllowedCategories] = useState("linting,formatting,testing,deployment");
  const [maxSizeMb, setMaxSizeMb] = useState(50);
  const [requireShaPin, setRequireShaPin] = useState(true);
  const [auditLog] = useState([
    { timestamp: "2026-03-08 14:22", action: "Plugin approved", user: "alice", detail: "code-formatter v1.2.0" },
    { timestamp: "2026-03-07 09:15", action: "Policy updated", user: "admin", detail: "Enabled SHA pinning" },
    { timestamp: "2026-03-06 16:40", action: "Plugin rejected", user: "alice", detail: "unsafe-exec v0.1.0" },
  ]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBar: React.CSSProperties = { display: "flex", gap: "4px", marginBottom: "16px", borderBottom: "1px solid var(--vscode-panel-border)" };
  const tab = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-tab-activeBackground)" : "transparent",
    color: active ? "var(--vscode-tab-activeForeground)" : "var(--vscode-tab-inactiveForeground)",
    borderBottom: active ? "2px solid var(--vscode-focusBorder)" : "2px solid transparent",
  });
  const badge = (color: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: "10px", fontSize: "11px", fontWeight: 600,
    backgroundColor: color, color: "#fff", marginLeft: "6px",
  });
  const btn: React.CSSProperties = {
    padding: "6px 14px", border: "none", borderRadius: "4px", cursor: "pointer",
    backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)",
  };
  const input: React.CSSProperties = {
    padding: "6px 10px", borderRadius: "4px", border: "1px solid var(--vscode-input-border)",
    backgroundColor: "var(--vscode-input-background)", color: "var(--vscode-input-foreground)", width: "100%",
  };
  const card: React.CSSProperties = {
    padding: "12px", marginBottom: "8px", borderRadius: "6px",
    backgroundColor: "var(--vscode-editorWidget-background)", border: "1px solid var(--vscode-panel-border)",
  };

  const visibilityColor = (v: string) => v === "Public" ? "#2ea043" : v === "Org" ? "#1f6feb" : v === "TeamOnly" ? "#8957e5" : "#6e7681";
  const statusColor = (s: string) => s === "Approved" ? "#2ea043" : s === "Pending" ? "#d29922" : s === "Rejected" ? "#f85149" : "#6e7681";

  const handleApprove = (id: string) => setApprovals(prev => prev.filter(a => a.id !== id));
  const handleReject = (id: string) => setApprovals(prev => prev.filter(a => a.id !== id));

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Team Governance</h3>
      <div style={tabBar}>
        {["plugins", "approvals", "policy"].map(t => (
          <button key={t} style={tab(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "plugins" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
            <span style={{ fontWeight: 600 }}>{plugins.length} registered plugins</span>
            <button style={btn}>Register Plugin</button>
          </div>
          {plugins.map(p => (
            <div key={p.id} style={card}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <strong>{p.name}</strong> <span style={{ opacity: 0.7 }}>v{p.version}</span>
                  <span style={badge(visibilityColor(p.visibility))}>{p.visibility}</span>
                  <span style={badge(statusColor(p.status))}>{p.status}</span>
                </div>
                <span style={{ opacity: 0.6 }}>by {p.author}</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "approvals" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Pending Approvals ({approvals.length})</h4>
          {approvals.length === 0 && <p style={{ opacity: 0.6 }}>No pending approvals.</p>}
          {approvals.map(a => (
            <div key={a.id} style={card}>
              <div style={{ marginBottom: "8px" }}>
                <strong>{a.pluginName}</strong> <span style={{ opacity: 0.6 }}>requested by {a.requestedBy} on {a.date}</span>
              </div>
              <div style={{ marginBottom: "8px", opacity: 0.8 }}>Reason: {a.reason}</div>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ opacity: 0.7 }}>Reviewer: {a.reviewer || "Unassigned"}</span>
                <div style={{ marginLeft: "auto", display: "flex", gap: "6px" }}>
                  <button style={{ ...btn, backgroundColor: "#2ea043" }} onClick={() => handleApprove(a.id)}>Approve</button>
                  <button style={{ ...btn, backgroundColor: "#f85149" }} onClick={() => handleReject(a.id)}>Reject</button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "policy" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Governance Policy</h4>
          <div style={card}>
            <label style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
              <input type="checkbox" checked={requireApproval} onChange={e => setRequireApproval(e.target.checked)} />
              Require approval for new plugins
            </label>
            <div style={{ marginBottom: "12px" }}>
              <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Allowed Categories</label>
              <input style={input} value={allowedCategories} onChange={e => setAllowedCategories(e.target.value)} />
            </div>
            <div style={{ marginBottom: "12px" }}>
              <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Max Plugin Size (MB)</label>
              <input style={{ ...input, width: "120px" }} type="number" value={maxSizeMb} onChange={e => setMaxSizeMb(Number(e.target.value))} />
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <input type="checkbox" checked={requireShaPin} onChange={e => setRequireShaPin(e.target.checked)} />
              Require SHA pinning for plugin versions
            </label>
          </div>
          <h4 style={{ margin: "16px 0 8px" }}>Audit Log</h4>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--vscode-panel-border)", textAlign: "left" }}>
                <th style={{ padding: "6px 8px" }}>Timestamp</th>
                <th style={{ padding: "6px 8px" }}>Action</th>
                <th style={{ padding: "6px 8px" }}>User</th>
                <th style={{ padding: "6px 8px" }}>Detail</th>
              </tr>
            </thead>
            <tbody>
              {auditLog.map((entry, i) => (
                <tr key={i} style={{ borderBottom: "1px solid var(--vscode-panel-border)" }}>
                  <td style={{ padding: "6px 8px", opacity: 0.7 }}>{entry.timestamp}</td>
                  <td style={{ padding: "6px 8px" }}>{entry.action}</td>
                  <td style={{ padding: "6px 8px" }}>{entry.user}</td>
                  <td style={{ padding: "6px 8px", opacity: 0.8 }}>{entry.detail}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default TeamGovernancePanel;
