import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PluginEntry {
  id: string;
  name: string;
  version: string;
  visibility: "Private" | "TeamOnly" | "Org" | "Public";
  status: "Pending" | "Approved" | "Rejected" | "Deprecated";
  author: string;
}

interface AuditEntry {
  timestamp: string;
  action: string;
  user: string;
  detail: string;
}

interface GovernancePolicies {
  requireApproval: boolean;
  allowedCategories: string;
  maxSizeMb: number;
  requireShaPin: boolean;
}

const TeamGovernancePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("plugins");
  const [plugins, setPlugins] = useState<PluginEntry[]>([]);
  const [auditLog, setAuditLog] = useState<AuditEntry[]>([]);
  const [policies, setPolicies] = useState<GovernancePolicies>({
    requireApproval: true,
    allowedCategories: "linting,formatting,testing,deployment",
    maxSizeMb: 50,
    requireShaPin: true,
  });
  const [loading, setLoading] = useState(false);

  // Registration form state
  const [newName, setNewName] = useState("");
  const [newVersion, setNewVersion] = useState("");
  const [newVisibility, setNewVisibility] = useState<string>("TeamOnly");
  const [newAuthor, setNewAuthor] = useState("");
  const [showRegisterForm, setShowRegisterForm] = useState(false);

  const loadPlugins = useCallback(async () => {
    try {
      const result = await invoke<PluginEntry[]>("list_governance_plugins");
      setPlugins(result);
    } catch (e) {
      console.error("Failed to load governance plugins:", e);
    }
  }, []);

  const loadAuditLog = useCallback(async () => {
    try {
      const result = await invoke<AuditEntry[]>("get_governance_audit_log");
      setAuditLog(result);
    } catch (e) {
      console.error("Failed to load governance audit log:", e);
    }
  }, []);

  const loadPolicies = useCallback(async () => {
    try {
      const result = await invoke<GovernancePolicies>("get_governance_policies");
      setPolicies(result);
    } catch (e) {
      console.error("Failed to load governance policies:", e);
    }
  }, []);

  useEffect(() => {
    loadPlugins();
    loadAuditLog();
    loadPolicies();
  }, [loadPlugins, loadAuditLog, loadPolicies]);

  const handleSubmitPlugin = async () => {
    if (!newName || !newVersion || !newAuthor) return;
    setLoading(true);
    try {
      await invoke("submit_plugin_for_approval", {
        name: newName,
        version: newVersion,
        visibility: newVisibility,
        author: newAuthor,
      });
      setNewName("");
      setNewVersion("");
      setNewVisibility("TeamOnly");
      setNewAuthor("");
      setShowRegisterForm(false);
      await loadPlugins();
      await loadAuditLog();
    } catch (e) {
      console.error("Failed to submit plugin:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleApprove = async (id: string) => {
    setLoading(true);
    try {
      const updated = await invoke<PluginEntry[]>("approve_plugin", {
        pluginId: id,
        reviewer: "current-user",
      });
      setPlugins(updated);
      await loadAuditLog();
    } catch (e) {
      console.error("Failed to approve plugin:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleReject = async (id: string) => {
    setLoading(true);
    try {
      const updated = await invoke<PluginEntry[]>("reject_plugin", {
        pluginId: id,
        reviewer: "current-user",
      });
      setPlugins(updated);
      await loadAuditLog();
    } catch (e) {
      console.error("Failed to reject plugin:", e);
    } finally {
      setLoading(false);
    }
  };

  const badge = (color: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: "10px", fontSize: "11px", fontWeight: 600,
    backgroundColor: color, color: "var(--text-primary)", marginLeft: "6px",
  });
  const visibilityColor = (v: string) => v === "Public" ? "var(--success-color)" : v === "Org" ? "var(--info-color)" : v === "TeamOnly" ? "var(--accent-color)" : "var(--text-secondary)";
  const statusColor = (s: string) => s === "Approved" ? "var(--success-color)" : s === "Pending" ? "var(--warning-color)" : s === "Rejected" ? "var(--error-color)" : "var(--text-secondary)";

  const pendingPlugins = plugins.filter(p => p.status === "Pending");

  return (
    <div className="panel-container" style={{ padding: "16px", fontSize: "13px", overflow: "auto" }}>
      <h3 style={{ margin: "0 0 12px" }}>Team Governance</h3>
      <div className="panel-tab-bar">
        {["plugins", "approvals", "policy"].map(t => (
          <button key={t} className={`panel-tab ${activeTab === t ? "active" : ""}`} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "plugins" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
            <span style={{ fontWeight: 600 }}>{plugins.length} registered plugins</span>
            <button className="panel-btn panel-btn-primary" onClick={() => setShowRegisterForm(!showRegisterForm)}>
              {showRegisterForm ? "Cancel" : "Register Plugin"}
            </button>
          </div>
          {showRegisterForm && (
            <div className="panel-card" style={{ marginBottom: "16px" }}>
              <div style={{ marginBottom: "8px" }}>
                <label className="panel-label">Plugin Name</label>
                <input className="panel-input panel-input-full" value={newName} onChange={e => setNewName(e.target.value)} placeholder="e.g. my-plugin" />
              </div>
              <div style={{ marginBottom: "8px" }}>
                <label className="panel-label">Version</label>
                <input className="panel-input panel-input-full" value={newVersion} onChange={e => setNewVersion(e.target.value)} placeholder="e.g. 1.0.0" />
              </div>
              <div style={{ marginBottom: "8px" }}>
                <label className="panel-label">Visibility</label>
                <select className="panel-select" value={newVisibility} onChange={e => setNewVisibility(e.target.value)}>
                  <option value="Private">Private</option>
                  <option value="TeamOnly">TeamOnly</option>
                  <option value="Org">Org</option>
                  <option value="Public">Public</option>
                </select>
              </div>
              <div style={{ marginBottom: "8px" }}>
                <label className="panel-label">Author</label>
                <input className="panel-input panel-input-full" value={newAuthor} onChange={e => setNewAuthor(e.target.value)} placeholder="e.g. alice" />
              </div>
              <button className="panel-btn panel-btn-primary" disabled={loading || !newName || !newVersion || !newAuthor} onClick={handleSubmitPlugin}>
                {loading ? "Submitting..." : "Submit for Approval"}
              </button>
            </div>
          )}
          {plugins.map(p => (
            <div key={p.id} className="panel-card">
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
          <h4 style={{ margin: "0 0 12px" }}>Pending Approvals ({pendingPlugins.length})</h4>
          {pendingPlugins.length === 0 && <p style={{ opacity: 0.6 }}>No pending approvals.</p>}
          {pendingPlugins.map(p => (
            <div key={p.id} className="panel-card">
              <div style={{ marginBottom: "8px" }}>
                <strong>{p.name}</strong> <span style={{ opacity: 0.6 }}>v{p.version} by {p.author}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ opacity: 0.7 }}>Visibility: {p.visibility}</span>
                <div style={{ marginLeft: "auto", display: "flex", gap: "6px" }}>
                  <button className="panel-btn panel-btn-primary" style={{ backgroundColor: "var(--success-color)" }} disabled={loading} onClick={() => handleApprove(p.id)}>Approve</button>
                  <button className="panel-btn panel-btn-danger" disabled={loading} onClick={() => handleReject(p.id)}>Reject</button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "policy" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Governance Policy</h4>
          <div className="panel-card">
            <label style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
              <input type="checkbox" checked={policies.requireApproval} onChange={e => setPolicies({ ...policies, requireApproval: e.target.checked })} />
              Require approval for new plugins
            </label>
            <div style={{ marginBottom: "12px" }}>
              <label className="panel-label">Allowed Categories</label>
              <input className="panel-input panel-input-full" value={policies.allowedCategories} onChange={e => setPolicies({ ...policies, allowedCategories: e.target.value })} />
            </div>
            <div style={{ marginBottom: "12px" }}>
              <label className="panel-label">Max Plugin Size (MB)</label>
              <input className="panel-input" style={{ width: "120px" }} type="number" value={policies.maxSizeMb} onChange={e => setPolicies({ ...policies, maxSizeMb: Number(e.target.value) })} />
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <input type="checkbox" checked={policies.requireShaPin} onChange={e => setPolicies({ ...policies, requireShaPin: e.target.checked })} />
              Require SHA pinning for plugin versions
            </label>
          </div>
          <h4 style={{ margin: "16px 0 8px" }}>Audit Log</h4>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-color)", textAlign: "left" }}>
                <th style={{ padding: "6px 8px" }}>Timestamp</th>
                <th style={{ padding: "6px 8px" }}>Action</th>
                <th style={{ padding: "6px 8px" }}>User</th>
                <th style={{ padding: "6px 8px" }}>Detail</th>
              </tr>
            </thead>
            <tbody>
              {auditLog.map((entry, i) => (
                <tr key={i} style={{ borderBottom: "1px solid var(--border-color)" }}>
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
