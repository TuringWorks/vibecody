/**
 * SpecPipelinePanel — Spec-driven development pipeline with requirements, design, and tasks.
 *
 * Tabs: Requirements, Design, Tasks
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Requirements" | "Design" | "Tasks";
const TABS: Tab[] = ["Requirements", "Design", "Tasks"];

interface Requirement {
  id: string;
  text: string;
  status: string;
  priority: string;
}

interface Design {
  id: string;
  title: string;
  reqs: string[];
  status: string;
  rationale: string;
}

interface Task {
  id: string;
  title: string;
  status: string;
  deps: string[];
  progress: number;
}

const STATUS_COLORS: Record<string, string> = {
  Verified: "var(--success-color)", Implemented: "var(--info-color)",
  Pending: "var(--warning-color)", Draft: "var(--text-secondary)",
  "In Progress": "var(--info-color)", Done: "var(--success-color)",
  Blocked: "var(--error-color)", Accepted: "var(--success-color)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)", background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const barBg: React.CSSProperties = {
  height: 8, borderRadius: "var(--radius-xs-plus)", background: "var(--bg-tertiary)", overflow: "hidden", flex: 1,
};
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "var(--font-size-base)", flexShrink: 0, gap: 12,
};

const SpecPipelinePanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Requirements");
  const [reqs, setReqs] = useState<Requirement[]>([]);
  const [designs, setDesigns] = useState<Design[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function loadData() {
      setLoading(true);
      try {
        const [r, d, t] = await Promise.all([
          invoke<Requirement[]>("get_spec_requirements"),
          invoke<Design[]>("get_spec_designs"),
          invoke<Task[]>("get_spec_tasks"),
        ]);
        if (!cancelled) {
          setReqs(r);
          setDesigns(d);
          setTasks(t);
        }
      } catch (err) {
        console.error("Failed to load spec pipeline data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadData();
    return () => { cancelled = true; };
  }, []);

  const coverage = reqs.length > 0
    ? Math.round((reqs.filter(r => r.status === "Verified" || r.status === "Implemented").length / reqs.length) * 100)
    : 0;

  if (loading) {
    return (
      <div className="panel-container" role="region" aria-label="Spec Pipeline Panel">
        <div className="panel-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="panel-container" role="region" aria-label="Spec Pipeline Panel">
      <div style={statusBarStyle}>
        <span>Coverage: <strong>{coverage}%</strong></span>
        <div style={barBg}><div style={{ height: "100%", borderRadius: "var(--radius-xs-plus)", background: "var(--success-color)", width: `${coverage}%` }} /></div>
        <span>{reqs.filter(r => r.status === "Verified").length} verified / {reqs.length} total</span>
      </div>
      <div className="panel-tab-bar" role="tablist" aria-label="Spec Pipeline tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Requirements" && reqs.length === 0 && (
          <div className="panel-empty">No requirements defined yet.</div>
        )}
        {tab === "Requirements" && reqs.map((r, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <span><strong>{r.id}</strong> <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>[{r.priority}]</span></span>
              <span style={badgeStyle(STATUS_COLORS[r.status] || "var(--text-secondary)")}>{r.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{r.text}</div>
          </div>
        ))}
        {tab === "Design" && designs.length === 0 && (
          <div className="panel-empty">No design documents yet.</div>
        )}
        {tab === "Design" && designs.map((d, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{d.id}: {d.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[d.status] || "var(--text-secondary)")}>{d.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Rationale: {d.rationale}</div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>Links: {d.reqs.join(", ")}</div>
          </div>
        ))}
        {tab === "Tasks" && tasks.length === 0 && (
          <div className="panel-empty">No tasks created yet.</div>
        )}
        {tab === "Tasks" && tasks.map((t, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{t.id}: {t.title}</strong>
              <span style={badgeStyle(STATUS_COLORS[t.status] || "var(--text-secondary)")}>{t.status}</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
              <div style={barBg}><div style={{ height: "100%", borderRadius: "var(--radius-xs-plus)", background: "var(--accent-color)", width: `${t.progress}%` }} /></div>
              <span style={{ fontSize: "var(--font-size-sm)" }}>{t.progress}%</span>
            </div>
            {t.deps.length > 0 && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>Depends on: {t.deps.join(", ")}</div>}
          </div>
        ))}
      </div>
    </div>
  );
};

export default SpecPipelinePanel;
