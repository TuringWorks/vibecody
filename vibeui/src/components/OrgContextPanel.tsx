/**
 * OrgContextPanel — Organization-wide context: indexed repos, detected patterns, conventions, and dependencies.
 *
 * Tabs: Repositories, Patterns, Conventions, Dependencies
 */
import React, { useState } from "react";

type Tab = "Repositories" | "Patterns" | "Conventions" | "Dependencies";
const TABS: Tab[] = ["Repositories", "Patterns", "Conventions", "Dependencies"];

const STATUS_COLORS: Record<string, string> = {
  Indexed: "var(--success-color)", Indexing: "var(--info-color)",
  Stale: "var(--warning-color)", Error: "var(--error-color)",
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
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
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
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12, flexShrink: 0,
};
const barBg: React.CSSProperties = {
  height: 6, borderRadius: 3, background: "var(--bg-tertiary)", overflow: "hidden", flex: 1, maxWidth: 120,
};

const REPOS = [
  { name: "org/backend-api", lang: "Rust", files: 1240, status: "Indexed", lastIndexed: "2 min ago" },
  { name: "org/frontend-app", lang: "TypeScript", files: 890, status: "Indexed", lastIndexed: "5 min ago" },
  { name: "org/ml-pipeline", lang: "Python", files: 420, status: "Indexing", lastIndexed: "-" },
  { name: "org/infra-configs", lang: "HCL", files: 180, status: "Stale", lastIndexed: "3 days ago" },
  { name: "org/mobile-app", lang: "Kotlin", files: 650, status: "Error", lastIndexed: "Failed" },
];
const PATTERNS = [
  { type: "Error Handling", count: 42, desc: "Result/Option pattern with custom error types", repos: 3 },
  { type: "API Design", count: 28, desc: "RESTful with OpenAPI specs, versioned endpoints", repos: 2 },
  { type: "Testing", count: 35, desc: "Unit + integration with fixture factories", repos: 4 },
  { type: "Auth", count: 12, desc: "JWT bearer tokens with refresh rotation", repos: 2 },
  { type: "Logging", count: 24, desc: "Structured JSON logging with trace IDs", repos: 3 },
];
const CONVENTIONS = [
  { name: "Branch naming", rule: "type/description (e.g., feat/auth-v2)", adoption: 94 },
  { name: "Commit messages", rule: "Conventional Commits (feat:, fix:, chore:)", adoption: 87 },
  { name: "Code review", rule: "Min 2 approvals, 1 from CODEOWNERS", adoption: 100 },
  { name: "Test coverage", rule: ">80% line coverage required", adoption: 72 },
  { name: "Documentation", rule: "JSDoc/rustdoc on all public APIs", adoption: 65 },
];
const DEPS = [
  { from: "frontend-app", to: "backend-api", type: "HTTP API", version: "v2" },
  { from: "ml-pipeline", to: "backend-api", type: "gRPC", version: "v1" },
  { from: "backend-api", to: "infra-configs", type: "Config", version: "-" },
  { from: "mobile-app", to: "backend-api", type: "HTTP API", version: "v2" },
];

const indexed = REPOS.filter(r => r.status === "Indexed").length;

const OrgContextPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Repositories");
  return (
    <div style={containerStyle} role="region" aria-label="Org Context Panel">
      <div style={statusBarStyle}>
        <span>Index: <strong>{indexed}/{REPOS.length}</strong> repos indexed</span>
        <span>Total files: {REPOS.reduce((s, r) => s + r.files, 0).toLocaleString()}</span>
      </div>
      <div style={tabBarStyle} role="tablist" aria-label="Org Context tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Repositories" && REPOS.map((r, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{r.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[r.status] || "var(--text-muted)")}>{r.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{r.lang} &middot; {r.files} files &middot; Last indexed: {r.lastIndexed}</div>
          </div>
        ))}
        {tab === "Patterns" && PATTERNS.map((p, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{p.type}</strong>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{p.count} occurrences in {p.repos} repos</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{p.desc}</div>
          </div>
        ))}
        {tab === "Conventions" && CONVENTIONS.map((c, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.name}</strong>
              <span style={{ fontSize: 11, color: c.adoption >= 90 ? "var(--success-color)" : c.adoption >= 70 ? "var(--warning-color)" : "var(--error-color)" }}>{c.adoption}% adoption</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{c.rule}</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={barBg}><div style={{ height: "100%", borderRadius: 3, background: c.adoption >= 90 ? "var(--success-color)" : "var(--warning-color)", width: `${c.adoption}%` }} /></div>
            </div>
          </div>
        ))}
        {tab === "Dependencies" && DEPS.map((d, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ fontSize: 13 }}><strong>{d.from}</strong> <span style={{ color: "var(--text-muted)" }}>&rarr;</span> <strong>{d.to}</strong></div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Type: {d.type} &middot; Version: {d.version}</div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default OrgContextPanel;
