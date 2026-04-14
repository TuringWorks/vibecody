/**
 * VmOrchestratorPanel — Manages VM environments, agent PRs, and branch conflicts.
 *
 * Tabs: Environments, Pull Requests, Conflicts, Config
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Environments" | "Pull Requests" | "Conflicts" | "Config";
const TABS: Tab[] = ["Environments", "Pull Requests", "Conflicts", "Config"];

interface VmEnvironment {
  name: string;
  branch: string;
  status: string;
  cpu: string;
  mem: string;
  uptime: string;
}

interface VmPullRequest {
  title: string;
  branch: string;
  status: string;
  author: string;
  checks: string;
}

interface VmConflict {
  branch1: string;
  branch2: string;
  file: string;
  status: string;
  suggestion: string;
}

interface VmConfig {
  maxConcurrentVms: number;
  activeVms: number;
  defaultCpu: string;
  defaultMem: string;
  autoCleanupMinutes: number;
  snapshotIntervalMinutes: number;
}

const STATUS_COLORS: Record<string, string> = {
  Running: "var(--success-color)", Stopped: "var(--text-secondary)",
  Provisioning: "var(--info-color)", Error: "var(--error-color)",
  Open: "var(--info-color)", Merged: "var(--success-color)", Closed: "var(--text-secondary)",
  Resolved: "var(--success-color)", Pending: "var(--warning-color)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)", background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const inputStyle: React.CSSProperties = {
  width: "100%",
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-xs-plus)",
  color: "var(--text-primary)",
  padding: "6px 8px",
  fontSize: "var(--font-size-base)",
  boxSizing: "border-box",
};

const VmOrchestratorPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Environments");
  const [envs, setEnvs] = useState<VmEnvironment[]>([]);
  const [prs, setPrs] = useState<VmPullRequest[]>([]);
  const [conflicts, setConflicts] = useState<VmConflict[]>([]);
  const [config, setConfig] = useState<VmConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function loadData() {
      setLoading(true);
      try {
        const [e, p, c, cfg] = await Promise.all([
          invoke<VmEnvironment[]>("get_vm_environments"),
          invoke<VmPullRequest[]>("get_vm_pull_requests"),
          invoke<VmConflict[]>("get_vm_conflicts"),
          invoke<VmConfig>("get_vm_config"),
        ]);
        if (!cancelled) {
          setEnvs(e);
          setPrs(p);
          setConflicts(c);
          setConfig(cfg);
        }
      } catch (err) {
        console.error("Failed to load VM orchestrator data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadData();
    return () => { cancelled = true; };
  }, []);

  const handleSaveConfig = async () => {
    if (!config) return;
    setSaving(true);
    try {
      await invoke("save_vm_config", { config });
    } catch (err) {
      console.error("Failed to save VM config:", err);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="panel-container" role="region" aria-label="VM Orchestrator Panel">
        <div className="panel-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="panel-container" role="region" aria-label="VM Orchestrator Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="VM Orchestrator tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Environments" && envs.length === 0 && (
          <div className="panel-empty">No VM environments provisioned yet.</div>
        )}
        {tab === "Environments" && envs.map((e, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{e.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[e.status] || "var(--text-secondary)")}>{e.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              Branch: <code>{e.branch}</code> &middot; {e.cpu} / {e.mem} &middot; Uptime: {e.uptime}
            </div>
          </div>
        ))}
        {tab === "Pull Requests" && prs.length === 0 && (
          <div className="panel-empty">No pull requests from agents yet.</div>
        )}
        {tab === "Pull Requests" && prs.map((pr, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{pr.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[pr.status] || "var(--text-secondary)")}>{pr.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              {pr.branch} &middot; {pr.author} &middot; Checks: {pr.checks}
            </div>
          </div>
        ))}
        {tab === "Conflicts" && conflicts.length === 0 && (
          <div className="panel-empty">No branch conflicts detected.</div>
        )}
        {tab === "Conflicts" && conflicts.map((c, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.file}</strong>
              <span style={badgeStyle(STATUS_COLORS[c.status] || "var(--text-secondary)")}>{c.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              {c.branch1} vs {c.branch2}
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--accent-color)", marginTop: 4 }}>{c.suggestion}</div>
          </div>
        ))}
        {tab === "Config" && config && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label className="panel-label">Max Concurrent VMs</label>
              <input
                type="number"
                min={1}
                max={64}
                value={config.maxConcurrentVms}
                onChange={(e) => setConfig({ ...config, maxConcurrentVms: Number(e.target.value) })}
                style={inputStyle}
              />
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>{config.activeVms} currently active</div>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <div>
                <label className="panel-label">Default CPU</label>
                <input
                  value={config.defaultCpu}
                  onChange={(e) => setConfig({ ...config, defaultCpu: e.target.value })}
                  style={inputStyle}
                />
              </div>
              <div>
                <label className="panel-label">Default Memory</label>
                <input
                  value={config.defaultMem}
                  onChange={(e) => setConfig({ ...config, defaultMem: e.target.value })}
                  style={inputStyle}
                />
              </div>
            </div>
            <div>
              <label className="panel-label">Auto-cleanup (minutes idle)</label>
              <input
                type="number"
                min={1}
                max={1440}
                value={config.autoCleanupMinutes}
                onChange={(e) => setConfig({ ...config, autoCleanupMinutes: Number(e.target.value) })}
                style={inputStyle}
              />
            </div>
            <div>
              <label className="panel-label">Snapshot Interval (minutes)</label>
              <input
                type="number"
                min={1}
                max={60}
                value={config.snapshotIntervalMinutes}
                onChange={(e) => setConfig({ ...config, snapshotIntervalMinutes: Number(e.target.value) })}
                style={inputStyle}
              />
            </div>
            <button
              onClick={handleSaveConfig}
              disabled={saving}
              className="panel-btn panel-btn-primary"
              style={{ alignSelf: "flex-start", opacity: saving ? 0.5 : 1 }}
            >
              {saving ? "Saving..." : "Save Config"}
            </button>
          </div>
        )}
        {tab === "Config" && !config && (
          <div className="panel-empty">Failed to load configuration.</div>
        )}
      </div>
    </div>
  );
};

export default VmOrchestratorPanel;
