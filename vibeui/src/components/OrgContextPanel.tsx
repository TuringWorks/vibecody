/**
 * OrgContextPanel — Organization-wide context: indexed repos, detected patterns, conventions, and dependencies.
 *
 * Tabs: Repositories, Patterns, Conventions, Dependencies
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Repositories" | "Patterns" | "Conventions" | "Dependencies";
const TABS: Tab[] = ["Repositories", "Patterns", "Conventions", "Dependencies"];

interface Repo {
  name: string;
  lang: string;
  files: number;
  status: string;
  lastIndexed: string;
}

interface Pattern {
  type: string;
  count: number;
  desc: string;
  repos: number;
}

interface Convention {
  name: string;
  rule: string;
  adoption: number;
}

interface Dependency {
  from: string;
  to: string;
  type: string;
  version: string;
}

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
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12, flexShrink: 0,
};
const barBg: React.CSSProperties = {
  height: 6, borderRadius: 3, background: "var(--bg-tertiary)", overflow: "hidden", flex: 1, maxWidth: 120,
};

const OrgContextPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Repositories");
  const [repos, setRepos] = useState<Repo[]>([]);
  const [patterns, setPatterns] = useState<Pattern[]>([]);
  const [conventions, setConventions] = useState<Convention[]>([]);
  const [deps, setDeps] = useState<Dependency[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function loadData() {
      setLoading(true);
      try {
        const [r, p, c, d] = await Promise.all([
          invoke<Repo[]>("get_org_repos"),
          invoke<Pattern[]>("get_org_patterns"),
          invoke<Convention[]>("get_org_conventions"),
          invoke<Dependency[]>("get_org_dependencies"),
        ]);
        if (!cancelled) {
          setRepos(r);
          setPatterns(p);
          setConventions(c);
          setDeps(d);
        }
      } catch (err) {
        console.error("Failed to load org context data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadData();
    return () => { cancelled = true; };
  }, []);

  const indexed = repos.filter(r => r.status === "Indexed").length;

  if (loading) {
    return (
      <div style={containerStyle} role="region" aria-label="Org Context Panel">
        <div style={{ ...contentStyle, textAlign: "center", color: "var(--text-secondary)", fontSize: 12, marginTop: 32 }}>Loading...</div>
      </div>
    );
  }

  return (
    <div style={containerStyle} role="region" aria-label="Org Context Panel">
      <div style={statusBarStyle}>
        <span>Index: <strong>{indexed}/{repos.length}</strong> repos indexed</span>
        <span>Total files: {repos.reduce((s, r) => s + r.files, 0).toLocaleString()}</span>
      </div>
      <div style={tabBarStyle} role="tablist" aria-label="Org Context tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Repositories" && repos.length === 0 && (
          <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>No repositories indexed yet.</div>
        )}
        {tab === "Repositories" && repos.map((r, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{r.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[r.status] || "var(--text-secondary)")}>{r.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{r.lang} &middot; {r.files} files &middot; Last indexed: {r.lastIndexed}</div>
          </div>
        ))}
        {tab === "Patterns" && patterns.length === 0 && (
          <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>No patterns detected yet.</div>
        )}
        {tab === "Patterns" && patterns.map((p, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{p.type}</strong>
              <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{p.count} occurrences in {p.repos} repos</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{p.desc}</div>
          </div>
        ))}
        {tab === "Conventions" && conventions.length === 0 && (
          <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>No conventions configured yet.</div>
        )}
        {tab === "Conventions" && conventions.map((c, i) => (
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
        {tab === "Dependencies" && deps.length === 0 && (
          <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>No cross-repo dependencies detected.</div>
        )}
        {tab === "Dependencies" && deps.map((d, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ fontSize: 13 }}><strong>{d.from}</strong> <span style={{ color: "var(--text-secondary)" }}>&rarr;</span> <strong>{d.to}</strong></div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Type: {d.type} &middot; Version: {d.version}</div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default OrgContextPanel;
