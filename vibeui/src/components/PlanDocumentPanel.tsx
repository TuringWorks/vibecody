/**
 * PlanDocumentPanel — Plan-as-Document with Feedback.
 *
 * Tabs: Plans (list with status/version/author), Editor (markdown preview),
 * Comments (unresolved comments with resolve button).
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

type Tab = "plans" | "editor" | "comments";
type PlanStatus = "draft" | "review" | "approved" | "archived";

interface Plan {
  id: string;
  title: string;
  status: PlanStatus;
  version: number;
  author: string;
  updatedAt: string;
  markdown: string;
}

interface Comment {
  id: string;
  planId: string;
  author: string;
  timestamp: string;
  text: string;
  resolved: boolean;
  line?: number;
}

const MOCK_PLANS: Plan[] = [
  { id: "p1", title: "Auth Service Migration", status: "review", version: 3, author: "Alice", updatedAt: "2h ago",
    markdown: "# Auth Service Migration\n\n## Goals\n- Migrate from session-based to JWT auth\n- Support OAuth2 providers (Google, GitHub)\n- Zero-downtime migration\n\n## Phases\n1. **Phase 1**: Add JWT middleware alongside sessions\n2. **Phase 2**: Migrate all endpoints to JWT\n3. **Phase 3**: Remove session support\n\n## Risks\n- Token rotation during peak hours\n- Third-party provider rate limits\n\n## Timeline\n- Week 1-2: Phase 1\n- Week 3: Phase 2\n- Week 4: Phase 3 + cleanup" },
  { id: "p2", title: "Database Sharding Strategy", status: "draft", version: 1, author: "Bob", updatedAt: "1d ago",
    markdown: "# Database Sharding Strategy\n\n## Overview\nHorizontal sharding by tenant ID for the events table.\n\n## Approach\n- Consistent hashing with virtual nodes\n- 16 initial shards, expandable to 256\n- Cross-shard queries via scatter-gather\n\n## Open Questions\n- Rebalancing strategy during growth\n- Backup and restore per-shard" },
  { id: "p3", title: "CI Pipeline Optimization", status: "approved", version: 5, author: "Carol", updatedAt: "3d ago",
    markdown: "# CI Pipeline Optimization\n\n## Summary\nReduce CI time from 18min to under 6min.\n\n## Changes\n- Parallel test execution (4 shards)\n- Cached Docker layers\n- Incremental compilation\n- Skip unchanged modules\n\n## Results\n- Average: 5m 12s (71% reduction)\n- P95: 7m 30s" },
  { id: "p4", title: "API v2 Design", status: "archived", version: 8, author: "Dave", updatedAt: "2w ago",
    markdown: "# API v2 Design\n\n(Archived — superseded by v3 plan)" },
];

const MOCK_COMMENTS: Comment[] = [
  { id: "cm1", planId: "p1", author: "Bob", timestamp: "1h ago", text: "Should we consider refresh token rotation as well?", resolved: false, line: 12 },
  { id: "cm2", planId: "p1", author: "Carol", timestamp: "2h ago", text: "Phase 2 timeline seems tight for 15 endpoints.", resolved: false },
  { id: "cm3", planId: "p1", author: "Dave", timestamp: "3h ago", text: "Add a rollback plan for each phase.", resolved: false },
  { id: "cm4", planId: "p2", author: "Alice", timestamp: "1d ago", text: "16 shards may be too few for projected growth.", resolved: false },
  { id: "cm5", planId: "p2", author: "Carol", timestamp: "1d ago", text: "Consider CockroachDB as an alternative to manual sharding.", resolved: false },
  { id: "cm6", planId: "p3", author: "Bob", timestamp: "4d ago", text: "Great results! Can we add flaky test detection?", resolved: true },
];

const statusColors: Record<PlanStatus, string> = {
  draft: "var(--text-muted)",
  review: "var(--text-warning)",
  approved: "var(--text-success)",
  archived: "var(--text-muted)",
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg)" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer",
});

export default function PlanDocumentPanel() {
  const [tab, setTab] = useState<Tab>("plans");
  const [selectedPlan, setSelectedPlan] = useState<string>("p1");
  const [comments, setComments] = useState(MOCK_COMMENTS);

  const plan = MOCK_PLANS.find(p => p.id === selectedPlan);
  const planComments = comments.filter(c => c.planId === selectedPlan && !c.resolved);
  const allUnresolved = comments.filter(c => !c.resolved);

  const resolveComment = (id: string) => {
    setComments(cs => cs.map(c => c.id === id ? { ...c, resolved: true } : c));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {(["plans", "editor", "comments"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
            {t === "comments" && allUnresolved.length > 0 && (
              <span style={{ marginLeft: 4, fontSize: 9, padding: "0 4px", borderRadius: 6, background: "var(--text-danger)", color: "#1e1e2e" }}>{allUnresolved.length}</span>
            )}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
        {tab === "plans" && MOCK_PLANS.map(p => (
          <div key={p.id} onClick={() => { setSelectedPlan(p.id); setTab("editor"); }}
            style={{ padding: 10, background: selectedPlan === p.id ? "var(--accent-bg)" : "var(--bg-secondary)", borderRadius: 6, border: `1px solid ${selectedPlan === p.id ? "var(--accent-primary)" : "var(--border-color)"}`, cursor: "pointer" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
              <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{p.title}</span>
              <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: `${statusColors[p.status]}22`, color: statusColors[p.status], fontWeight: 600 }}>{p.status}</span>
            </div>
            <div style={{ display: "flex", gap: 12, fontSize: 10, color: "var(--text-muted)" }}>
              <span>v{p.version}</span>
              <span>{p.author}</span>
              <span>{p.updatedAt}</span>
              {comments.filter(c => c.planId === p.id && !c.resolved).length > 0 && (
                <span style={{ color: "var(--text-warning)" }}>
                  {comments.filter(c => c.planId === p.id && !c.resolved).length} comments
                </span>
              )}
            </div>
          </div>
        ))}

        {tab === "editor" && plan && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>{plan.title}</span>
              <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: `${statusColors[plan.status]}22`, color: statusColors[plan.status] }}>{plan.status}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>v{plan.version} by {plan.author}</span>
            </div>
            <pre style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 12, fontFamily: "monospace", color: "var(--text-primary)", whiteSpace: "pre-wrap", margin: 0, lineHeight: 1.6 }}>
              {plan.markdown}
            </pre>
            {planComments.length > 0 && (
              <button onClick={() => setTab("comments")}
                style={{ alignSelf: "flex-start", padding: "5px 12px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--text-warning)", borderRadius: 4, color: "var(--text-warning)", cursor: "pointer" }}>
                View {planComments.length} comments
              </button>
            )}
          </div>
        )}

        {tab === "comments" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {allUnresolved.length === 0 && (
              <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, padding: 40 }}>All comments resolved</div>
            )}
            {allUnresolved.map(c => {
              const p = MOCK_PLANS.find(pl => pl.id === c.planId);
              return (
                <div key={c.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
                  <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
                    <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-primary)" }}>{c.author}</span>
                    <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{c.timestamp}</span>
                    {c.line && <span style={{ fontSize: 10, color: "var(--text-muted)" }}>L{c.line}</span>}
                    <span style={{ fontSize: 10, color: "var(--accent-primary)", marginLeft: "auto" }}>{p?.title}</span>
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-primary)", marginBottom: 8 }}>{c.text}</div>
                  <button onClick={() => resolveComment(c.id)}
                    style={{ padding: "4px 12px", fontSize: 10, borderRadius: 4, border: "none", background: "var(--text-success)", color: "#1e1e2e", cursor: "pointer", fontWeight: 600 }}>
                    Resolve
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
