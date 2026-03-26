/**
 * VmOrchestratorPanel — Manages VM environments, agent PRs, and branch conflicts.
 *
 * Tabs: Environments, Pull Requests, Conflicts, Config
 */
import React, { useState } from "react";

type Tab = "Environments" | "Pull Requests" | "Conflicts" | "Config";
const TABS: Tab[] = ["Environments", "Pull Requests", "Conflicts", "Config"];

const STATUS_COLORS: Record<string, string> = {
  Running: "var(--success-color)", Stopped: "var(--text-secondary)",
  Provisioning: "var(--info-color)", Error: "var(--error-color)",
  Open: "var(--info-color)", Merged: "var(--success-color)", Closed: "var(--text-secondary)",
  Resolved: "var(--success-color)", Pending: "var(--warning-color)",
};

const containerStyle: React.CSSProperties = {
  display: "flex", flexDirection: "column", height: "100%",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  fontFamily: "inherit", overflow: "hidden",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex", gap: 2, padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)",
  overflowX: "auto", flexShrink: 0,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});

const ENVS = [
  { name: "feature/auth-v2", branch: "feature/auth-v2", status: "Running", cpu: "2 vCPU", mem: "4 GB", uptime: "2h 14m" },
  { name: "fix/race-condition", branch: "fix/race-condition", status: "Running", cpu: "1 vCPU", mem: "2 GB", uptime: "45m" },
  { name: "feat/dashboard", branch: "feat/dashboard", status: "Provisioning", cpu: "2 vCPU", mem: "4 GB", uptime: "-" },
  { name: "refactor/api", branch: "refactor/api", status: "Stopped", cpu: "-", mem: "-", uptime: "-" },
];
const PRS = [
  { title: "Add OAuth2 flow", branch: "feature/auth-v2", status: "Open", author: "agent-01", checks: "3/3 passed" },
  { title: "Fix data race in worker pool", branch: "fix/race-condition", status: "Open", author: "agent-02", checks: "2/3 pending" },
  { title: "Refactor REST endpoints", branch: "refactor/api", status: "Merged", author: "agent-03", checks: "3/3 passed" },
];
const CONFLICTS = [
  { branch1: "feature/auth-v2", branch2: "feat/dashboard", file: "src/routes.ts", status: "Pending", suggestion: "Rebase feat/dashboard onto feature/auth-v2" },
  { branch1: "fix/race-condition", branch2: "refactor/api", file: "src/worker.rs", status: "Resolved", suggestion: "Merged manually" },
];

const VmOrchestratorPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Environments");
  return (
    <div style={containerStyle} role="region" aria-label="VM Orchestrator Panel">
      <div style={tabBarStyle} role="tablist" aria-label="VM Orchestrator tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Environments" && ENVS.map((e, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{e.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[e.status] || "var(--text-secondary)")}>{e.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              Branch: <code>{e.branch}</code> &middot; {e.cpu} / {e.mem} &middot; Uptime: {e.uptime}
            </div>
          </div>
        ))}
        {tab === "Pull Requests" && PRS.map((pr, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{pr.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[pr.status] || "var(--text-secondary)")}>{pr.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {pr.branch} &middot; {pr.author} &middot; Checks: {pr.checks}
            </div>
          </div>
        ))}
        {tab === "Conflicts" && CONFLICTS.map((c, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.file}</strong>
              <span style={badgeStyle(STATUS_COLORS[c.status] || "var(--text-secondary)")}>{c.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {c.branch1} vs {c.branch2}
            </div>
            <div style={{ fontSize: 12, color: "var(--accent-color)", marginTop: 4 }}>{c.suggestion}</div>
          </div>
        ))}
        {tab === "Config" && (
          <div>
            <div style={cardStyle}><strong>Max concurrent VMs</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Limit: 8 VMs (4 currently active)</div></div>
            <div style={cardStyle}><strong>Default resources</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>2 vCPU, 4 GB RAM per environment</div></div>
            <div style={cardStyle}><strong>Auto-cleanup</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Stop idle VMs after 30 minutes</div></div>
            <div style={cardStyle}><strong>Snapshot policy</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Checkpoint every 10 minutes during active sessions</div></div>
          </div>
        )}
      </div>
    </div>
  );
};

export default VmOrchestratorPanel;
