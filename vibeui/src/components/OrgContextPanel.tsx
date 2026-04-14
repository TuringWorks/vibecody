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

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)", background: color, color: "var(--bg-primary)", fontWeight: 600,
});

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
      <div className="panel-container" role="region" aria-label="Org Context Panel">
        <div className="panel-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="panel-container" role="region" aria-label="Org Context Panel">
      <div style={{ padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "var(--font-size-base)", flexShrink: 0 }}>
        <span>Index: <strong>{indexed}/{repos.length}</strong> repos indexed</span>
        <span>Total files: {repos.reduce((s, r) => s + r.files, 0).toLocaleString()}</span>
      </div>
      <div className="panel-tab-bar" role="tablist" aria-label="Org Context tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab${tab === t ? " active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Repositories" && repos.length === 0 && (
          <div className="panel-empty">No repositories indexed yet.</div>
        )}
        {tab === "Repositories" && repos.map((r, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{r.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[r.status] || "var(--text-secondary)")}>{r.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{r.lang} &middot; {r.files} files &middot; Last indexed: {r.lastIndexed}</div>
          </div>
        ))}
        {tab === "Patterns" && patterns.length === 0 && (
          <div className="panel-empty">No patterns detected yet.</div>
        )}
        {tab === "Patterns" && patterns.map((p, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{p.type}</strong>
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{p.count} occurrences in {p.repos} repos</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{p.desc}</div>
          </div>
        ))}
        {tab === "Conventions" && conventions.length === 0 && (
          <div className="panel-empty">No conventions configured yet.</div>
        )}
        {tab === "Conventions" && conventions.map((c, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.name}</strong>
              <span style={{ fontSize: "var(--font-size-sm)", color: c.adoption >= 90 ? "var(--success-color)" : c.adoption >= 70 ? "var(--warning-color)" : "var(--error-color)" }}>{c.adoption}% adoption</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>{c.rule}</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={barBg}><div style={{ height: "100%", borderRadius: 3, background: c.adoption >= 90 ? "var(--success-color)" : "var(--warning-color)", width: `${c.adoption}%` }} /></div>
            </div>
          </div>
        ))}
        {tab === "Dependencies" && deps.length === 0 && (
          <div className="panel-empty">No cross-repo dependencies detected.</div>
        )}
        {tab === "Dependencies" && deps.map((d, i) => (
          <div key={i} className="panel-card">
            <div style={{ fontSize: "var(--font-size-md)" }}><strong>{d.from}</strong> <span style={{ color: "var(--text-secondary)" }}>&rarr;</span> <strong>{d.to}</strong></div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Type: {d.type} &middot; Version: {d.version}</div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default OrgContextPanel;
