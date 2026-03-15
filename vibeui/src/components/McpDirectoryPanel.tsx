/**
 * McpDirectoryPanel — MCP Plugin Directory panel.
 *
 * Browse, search, and manage MCP plugins with ratings, downloads,
 * and category filtering.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState, useMemo } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface McpPlugin {
  id: string;
  name: string;
  author: string;
  description: string;
  category: string;
  rating: number;
  downloads: number;
  version: string;
  installed: boolean;
  updatable: boolean;
}

// ── Mock Data ─────────────────────────────────────────────────────────────────

const CATEGORIES = ["All", "File Systems", "Git", "Databases", "Cloud", "AI/ML", "Testing", "DevOps", "Communication"];

const INITIAL_PLUGINS: McpPlugin[] = [
  { id: "p1", name: "filesystem-extended", author: "mcp-org", description: "Extended file system operations: watch, glob, symlinks, permissions", category: "File Systems", rating: 4.8, downloads: 125400, version: "2.1.0", installed: true, updatable: false },
  { id: "p2", name: "git-advanced", author: "devtools-co", description: "Advanced git operations: interactive rebase, cherry-pick, bisect", category: "Git", rating: 4.6, downloads: 89200, version: "1.5.2", installed: true, updatable: true },
  { id: "p3", name: "postgres-manager", author: "db-tools", description: "PostgreSQL management: queries, migrations, schema inspection", category: "Databases", rating: 4.7, downloads: 67800, version: "3.0.1", installed: false, updatable: false },
  { id: "p4", name: "aws-toolkit", author: "cloud-devs", description: "AWS service integration: S3, Lambda, DynamoDB, CloudFormation", category: "Cloud", rating: 4.5, downloads: 54300, version: "2.3.0", installed: false, updatable: false },
  { id: "p5", name: "docker-compose", author: "container-tools", description: "Docker Compose management: up, down, logs, build, exec", category: "DevOps", rating: 4.4, downloads: 43200, version: "1.2.1", installed: true, updatable: false },
  { id: "p6", name: "slack-integration", author: "comm-tools", description: "Slack messaging: send, read channels, search messages, upload files", category: "Communication", rating: 4.3, downloads: 38100, version: "1.0.3", installed: false, updatable: false },
  { id: "p7", name: "jest-runner", author: "test-tools", description: "Jest test runner: run, watch, coverage, snapshot management", category: "Testing", rating: 4.5, downloads: 31200, version: "1.1.0", installed: false, updatable: false },
  { id: "p8", name: "huggingface-models", author: "ml-community", description: "HuggingFace model browser: search, download, inference, fine-tune", category: "AI/ML", rating: 4.2, downloads: 28700, version: "0.8.0", installed: false, updatable: false },
  { id: "p9", name: "redis-client", author: "db-tools", description: "Redis client: get, set, pub/sub, streams, cluster management", category: "Databases", rating: 4.4, downloads: 25400, version: "1.3.0", installed: false, updatable: false },
  { id: "p10", name: "kubernetes-ops", author: "cloud-devs", description: "Kubernetes operations: pods, deployments, services, logs, exec", category: "DevOps", rating: 4.6, downloads: 47800, version: "2.0.0", installed: false, updatable: false },
  { id: "p11", name: "github-actions", author: "mcp-org", description: "GitHub Actions: trigger workflows, view runs, download artifacts", category: "DevOps", rating: 4.7, downloads: 62300, version: "1.4.0", installed: true, updatable: true },
  { id: "p12", name: "mongodb-tools", author: "db-tools", description: "MongoDB operations: CRUD, aggregation, indexes, atlas management", category: "Databases", rating: 4.3, downloads: 19800, version: "1.0.1", installed: false, updatable: false },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--text-primary)" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box" };
const selectStyle: React.CSSProperties = { ...inputStyle, width: "auto", cursor: "pointer" };

const renderStars = (rating: number): string => {
  const full = Math.floor(rating);
  const half = rating - full >= 0.5 ? 1 : 0;
  const empty = 5 - full - half;
  return "★".repeat(full) + (half ? "½" : "") + "☆".repeat(empty);
};

const formatDownloads = (n: number): string => {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
};

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "browse" | "installed" | "search";

export function McpDirectoryPanel() {
  const [tab, setTab] = useState<Tab>("browse");
  const [plugins, setPlugins] = useState<McpPlugin[]>(INITIAL_PLUGINS);
  const [searchQuery, setSearchQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("All");

  const installedPlugins = useMemo(() => plugins.filter((p) => p.installed), [plugins]);

  const browsePlugins = useMemo(() => {
    return plugins.sort((a, b) => b.downloads - a.downloads);
  }, [plugins]);

  const searchResults = useMemo(() => {
    let filtered = plugins;
    if (categoryFilter !== "All") {
      filtered = filtered.filter((p) => p.category === categoryFilter);
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (p) => p.name.toLowerCase().includes(q) || p.description.toLowerCase().includes(q) || p.author.toLowerCase().includes(q)
      );
    }
    return filtered;
  }, [plugins, searchQuery, categoryFilter]);

  const toggleInstall = (id: string) => {
    setPlugins((prev) => prev.map((p) => (p.id === id ? { ...p, installed: !p.installed, updatable: false } : p)));
  };

  const updatePlugin = (id: string) => {
    setPlugins((prev) => prev.map((p) => (p.id === id ? { ...p, updatable: false } : p)));
  };

  const renderPluginCard = (plugin: McpPlugin, showInstallBtn: boolean) => (
    <div key={plugin.id} style={cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600 }}>{plugin.name} <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{plugin.version}</span></div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>by {plugin.author} | {plugin.category}</div>
          <div style={{ fontSize: 12, marginTop: 4 }}>{plugin.description}</div>
          <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: 11 }}>
            <span style={{ color: "var(--warning-color)" }}>{renderStars(plugin.rating)} {plugin.rating.toFixed(1)}</span>
            <span style={{ color: "var(--text-secondary)" }}>{formatDownloads(plugin.downloads)} downloads</span>
          </div>
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          {showInstallBtn && !plugin.installed && (
            <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--text-primary)" }} onClick={() => toggleInstall(plugin.id)}>Install</button>
          )}
          {plugin.installed && plugin.updatable && (
            <button style={{ ...btnStyle, background: "var(--warning-color)", color: "var(--text-primary)" }} onClick={() => updatePlugin(plugin.id)}>Update</button>
          )}
          {plugin.installed && (
            <button style={{ ...btnStyle, background: "var(--error-color)", color: "var(--text-primary)" }} onClick={() => toggleInstall(plugin.id)}>Uninstall</button>
          )}
        </div>
      </div>
    </div>
  );

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCP Plugin Directory</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "browse")} onClick={() => setTab("browse")}>Browse</button>
        <button style={tabBtnStyle(tab === "installed")} onClick={() => setTab("installed")}>Installed ({installedPlugins.length})</button>
        <button style={tabBtnStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
      </div>

      {tab === "browse" && (
        <div>
          <div style={{ ...cardStyle, fontSize: 12 }}>
            {plugins.length} plugins available | {installedPlugins.length} installed
          </div>
          {browsePlugins.map((p) => renderPluginCard(p, true))}
        </div>
      )}

      {tab === "installed" && (
        <div>
          {installedPlugins.length === 0 && <div style={cardStyle}>No plugins installed.</div>}
          {installedPlugins.map((p) => renderPluginCard(p, false))}
        </div>
      )}

      {tab === "search" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input style={{ ...inputStyle, flex: 1 }} value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} placeholder="Search plugins..." />
            <select style={selectStyle} value={categoryFilter} onChange={(e) => setCategoryFilter(e.target.value)}>
              {CATEGORIES.map((c) => <option key={c} value={c}>{c}</option>)}
            </select>
          </div>
          {searchResults.length === 0 && <div style={cardStyle}>No plugins match your search.</div>}
          <div style={labelStyle}>{searchResults.length} result(s)</div>
          {searchResults.map((p) => renderPluginCard(p, true))}
        </div>
      )}
    </div>
  );
}
