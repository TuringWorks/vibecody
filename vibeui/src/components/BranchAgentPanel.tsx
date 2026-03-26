/**
 * BranchAgentPanel — Monitors branch-per-agent execution, PRs, and conflict resolution.
 *
 * Tabs: Active Agents, Pull Requests, Conflicts
 */
import React, { useState } from "react";

type Tab = "Active Agents" | "Pull Requests" | "Conflicts";
const TABS: Tab[] = ["Active Agents", "Pull Requests", "Conflicts"];

const STATUS_COLORS: Record<string, string> = {
  Running: "var(--success-color)", Idle: "var(--text-secondary)",
  Errored: "var(--error-color)", Open: "var(--info-color)",
  Merged: "var(--success-color)", Closed: "var(--text-secondary)",
  Critical: "var(--error-color)", Warning: "var(--warning-color)",
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
const statsBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", gap: 24, fontSize: 12, flexShrink: 0,
};
const statStyle: React.CSSProperties = { display: "flex", flexDirection: "column", alignItems: "center" };

const AGENTS = [
  { id: "agent-auth", branch: "feature/auth-v2", status: "Running", task: "Implementing OAuth2 middleware", duration: "1h 12m" },
  { id: "agent-perf", branch: "fix/perf-regression", status: "Running", task: "Profiling hot path in query builder", duration: "32m" },
  { id: "agent-docs", branch: "docs/api-reference", status: "Idle", task: "Waiting for review approval", duration: "5m" },
  { id: "agent-test", branch: "test/integration", status: "Errored", task: "Test runner crashed on assertion", duration: "0m" },
];
const PRS = [
  { title: "OAuth2 middleware", branch: "feature/auth-v2", status: "Open", agent: "agent-auth", files: 8, additions: 342, deletions: 12 },
  { title: "Fix N+1 query", branch: "fix/perf-regression", status: "Open", agent: "agent-perf", files: 3, additions: 45, deletions: 67 },
  { title: "API docs generation", branch: "docs/api-reference", status: "Merged", agent: "agent-docs", files: 14, additions: 890, deletions: 0 },
];
const CONFLICTS = [
  { branch: "feature/auth-v2", target: "main", files: ["src/routes.ts", "src/middleware.ts"], severity: "Critical", suggestion: "Rebase onto latest main; routes.ts has diverged significantly" },
  { branch: "fix/perf-regression", target: "main", files: ["src/db/query.rs"], severity: "Warning", suggestion: "Minor conflict in imports; auto-resolvable" },
];

const VmOrchestratorPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Active Agents");
  const running = AGENTS.filter(a => a.status === "Running").length;
  return (
    <div style={containerStyle} role="region" aria-label="Branch Agent Panel">
      <div style={statsBarStyle}>
        <div style={statStyle}><strong style={{ fontSize: 18 }}>{AGENTS.length}</strong><span style={{ color: "var(--text-secondary)" }}>Agents</span></div>
        <div style={statStyle}><strong style={{ fontSize: 18, color: "var(--success-color)" }}>{running}</strong><span style={{ color: "var(--text-secondary)" }}>Running</span></div>
        <div style={statStyle}><strong style={{ fontSize: 18 }}>{PRS.filter(p => p.status === "Open").length}</strong><span style={{ color: "var(--text-secondary)" }}>Open PRs</span></div>
        <div style={statStyle}><strong style={{ fontSize: 18, color: "var(--error-color)" }}>{CONFLICTS.length}</strong><span style={{ color: "var(--text-secondary)" }}>Conflicts</span></div>
      </div>
      <div style={tabBarStyle} role="tablist" aria-label="Branch Agent tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Active Agents" && AGENTS.map((a, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{a.id}</strong>
              <span style={badgeStyle(STATUS_COLORS[a.status] || "var(--text-secondary)")}>{a.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Branch: <code>{a.branch}</code> &middot; {a.duration}</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{a.task}</div>
          </div>
        ))}
        {tab === "Pull Requests" && PRS.map((pr, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{pr.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[pr.status] || "var(--text-secondary)")}>{pr.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {pr.agent} &middot; {pr.files} files &middot; <span style={{ color: "var(--success-color)" }}>+{pr.additions}</span> <span style={{ color: "var(--error-color)" }}>-{pr.deletions}</span>
            </div>
          </div>
        ))}
        {tab === "Conflicts" && CONFLICTS.map((c, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.branch} &rarr; {c.target}</strong>
              <span style={badgeStyle(STATUS_COLORS[c.severity] || "var(--text-secondary)")}>{c.severity}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Files: {c.files.join(", ")}</div>
            <div style={{ fontSize: 12, color: "var(--accent-color)", marginTop: 4 }}>{c.suggestion}</div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default VmOrchestratorPanel;
