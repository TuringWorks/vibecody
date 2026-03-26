/**
 * SpecPipelinePanel — Spec-driven development pipeline with requirements, design, and tasks.
 *
 * Tabs: Requirements, Design, Tasks
 */
import React, { useState } from "react";

type Tab = "Requirements" | "Design" | "Tasks";
const TABS: Tab[] = ["Requirements", "Design", "Tasks"];

const STATUS_COLORS: Record<string, string> = {
  Verified: "var(--success-color)", Implemented: "var(--info-color)",
  Pending: "var(--warning-color)", Draft: "var(--text-secondary)",
  "In Progress": "var(--info-color)", Done: "var(--success-color)",
  Blocked: "var(--error-color)", Accepted: "var(--success-color)",
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
const barBg: React.CSSProperties = {
  height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden", flex: 1,
};
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12, flexShrink: 0, gap: 12,
};

const REQS = [
  { id: "EARS-001", text: "When a user logs in, the system shall create a session token", status: "Verified", priority: "P0" },
  { id: "EARS-002", text: "The system shall respond within 200ms for all API calls", status: "Implemented", priority: "P1" },
  { id: "EARS-003", text: "While processing a batch, the system shall display progress", status: "Pending", priority: "P1" },
  { id: "EARS-004", text: "If the network is unavailable, the system shall queue requests", status: "Draft", priority: "P2" },
];
const DESIGNS = [
  { id: "DES-01", title: "JWT session tokens", reqs: ["EARS-001"], status: "Accepted", rationale: "Stateless, scalable" },
  { id: "DES-02", title: "Redis response caching", reqs: ["EARS-002"], status: "Accepted", rationale: "Sub-ms cache hits" },
  { id: "DES-03", title: "SSE progress streaming", reqs: ["EARS-003"], status: "Draft", rationale: "Real-time updates" },
];
const TASKS = [
  { id: "T-01", title: "Implement JWT middleware", status: "Done", deps: [], progress: 100 },
  { id: "T-02", title: "Add Redis caching layer", status: "In Progress", deps: ["T-01"], progress: 60 },
  { id: "T-03", title: "SSE endpoint for progress", status: "Pending", deps: ["T-02"], progress: 0 },
  { id: "T-04", title: "Offline request queue", status: "Blocked", deps: ["T-01"], progress: 0 },
];

const coverage = Math.round((REQS.filter(r => r.status === "Verified" || r.status === "Implemented").length / REQS.length) * 100);

const SpecPipelinePanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Requirements");
  return (
    <div style={containerStyle} role="region" aria-label="Spec Pipeline Panel">
      <div style={statusBarStyle}>
        <span>Coverage: <strong>{coverage}%</strong></span>
        <div style={barBg}><div style={{ height: "100%", borderRadius: 4, background: "var(--success-color)", width: `${coverage}%` }} /></div>
        <span>{REQS.filter(r => r.status === "Verified").length} verified / {REQS.length} total</span>
      </div>
      <div style={tabBarStyle} role="tablist" aria-label="Spec Pipeline tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Requirements" && REQS.map((r, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <span><strong>{r.id}</strong> <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>[{r.priority}]</span></span>
              <span style={badgeStyle(STATUS_COLORS[r.status] || "var(--text-secondary)")}>{r.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{r.text}</div>
          </div>
        ))}
        {tab === "Design" && DESIGNS.map((d, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{d.id}: {d.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[d.status] || "var(--text-secondary)")}>{d.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Rationale: {d.rationale}</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>Links: {d.reqs.join(", ")}</div>
          </div>
        ))}
        {tab === "Tasks" && TASKS.map((t, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{t.id}: {t.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[t.status] || "var(--text-secondary)")}>{t.status}</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
              <div style={barBg}><div style={{ height: "100%", borderRadius: 4, background: "var(--accent-color)", width: `${t.progress}%` }} /></div>
              <span style={{ fontSize: 11 }}>{t.progress}%</span>
            </div>
            {t.deps.length > 0 && <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>Depends on: {t.deps.join(", ")}</div>}
          </div>
        ))}
      </div>
    </div>
  );
};

export default SpecPipelinePanel;
