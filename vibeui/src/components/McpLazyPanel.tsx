/**
 * McpLazyPanel — MCP Lazy Loading panel.
 *
 * Visualises MCP tool manifests with lazy-loading status, search across
 * tools with relevance scoring, and context-savings / cache metrics.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState, useMemo } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ToolManifest {
  id: string;
  name: string;
  description: string;
  version: string;
  status: "loaded" | "unloaded" | "loading";
  sizeKb: number;
  lastUsed: string | null;
}

interface SearchResult {
  toolId: string;
  name: string;
  description: string;
  relevance: number;
}

interface LazyMetrics {
  contextSavingsPct: number;
  cacheHits: number;
  cacheMisses: number;
  avgLoadTimeMs: number;
  loadTimes: { label: string; ms: number }[];
}

// ── Mock Data ─────────────────────────────────────────────────────────────────

const MOCK_MANIFESTS: ToolManifest[] = [
  { id: "t1", name: "file_read", description: "Read file contents from disk", version: "1.2.0", status: "loaded", sizeKb: 12, lastUsed: "2026-03-13T08:30:00Z" },
  { id: "t2", name: "file_write", description: "Write content to a file", version: "1.2.0", status: "loaded", sizeKb: 14, lastUsed: "2026-03-13T08:25:00Z" },
  { id: "t3", name: "grep_search", description: "Search file contents with regex", version: "1.1.0", status: "loaded", sizeKb: 18, lastUsed: "2026-03-13T07:50:00Z" },
  { id: "t4", name: "git_status", description: "Show working tree status", version: "1.0.0", status: "unloaded", sizeKb: 22, lastUsed: null },
  { id: "t5", name: "git_diff", description: "Show file differences", version: "1.0.0", status: "unloaded", sizeKb: 26, lastUsed: null },
  { id: "t6", name: "bash_exec", description: "Execute shell commands", version: "2.0.1", status: "loaded", sizeKb: 8, lastUsed: "2026-03-13T08:31:00Z" },
  { id: "t7", name: "web_fetch", description: "Fetch URL contents", version: "1.3.0", status: "unloaded", sizeKb: 30, lastUsed: null },
  { id: "t8", name: "notebook_edit", description: "Edit Jupyter notebook cells", version: "0.9.0", status: "unloaded", sizeKb: 45, lastUsed: null },
  { id: "t9", name: "image_read", description: "Read and describe image files", version: "1.0.0", status: "loading", sizeKb: 52, lastUsed: null },
  { id: "t10", name: "sql_query", description: "Execute SQL queries against databases", version: "1.1.0", status: "unloaded", sizeKb: 35, lastUsed: null },
];

const MOCK_METRICS: LazyMetrics = {
  contextSavingsPct: 68,
  cacheHits: 1247,
  cacheMisses: 83,
  avgLoadTimeMs: 42,
  loadTimes: [
    { label: "file_read", ms: 12 },
    { label: "file_write", ms: 14 },
    { label: "grep_search", ms: 18 },
    { label: "bash_exec", ms: 8 },
    { label: "image_read", ms: 52 },
    { label: "web_fetch", ms: 38 },
  ],
};

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "#fff" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box" };
const badgeStyle = (variant: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "#fff", background: variant === "loaded" ? "#22c55e" : variant === "loading" ? "#f59e0b" : "#6b7280" });
const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "registry" | "search" | "metrics";

export function McpLazyPanel() {
  const [tab, setTab] = useState<Tab>("registry");
  const [manifests, setManifests] = useState<ToolManifest[]>(MOCK_MANIFESTS);
  const [searchQuery, setSearchQuery] = useState("");

  const searchResults = useMemo<SearchResult[]>(() => {
    if (!searchQuery.trim()) return [];
    const q = searchQuery.toLowerCase();
    return manifests
      .map((m) => {
        const nameMatch = m.name.toLowerCase().includes(q) ? 0.6 : 0;
        const descMatch = m.description.toLowerCase().includes(q) ? 0.4 : 0;
        const relevance = nameMatch + descMatch;
        return { toolId: m.id, name: m.name, description: m.description, relevance };
      })
      .filter((r) => r.relevance > 0)
      .sort((a, b) => b.relevance - a.relevance);
  }, [searchQuery, manifests]);

  const toggleLoad = (id: string) => {
    setManifests((prev) =>
      prev.map((m) =>
        m.id === id
          ? { ...m, status: m.status === "loaded" ? "unloaded" : m.status === "unloaded" ? "loading" : "loaded" }
          : m
      )
    );
  };

  const loadedCount = manifests.filter((m) => m.status === "loaded").length;
  const totalCount = manifests.length;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCP Lazy Loading</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "registry")} onClick={() => setTab("registry")}>Tool Registry</button>
        <button style={tabBtnStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
        <button style={tabBtnStyle(tab === "metrics")} onClick={() => setTab("metrics")}>Metrics</button>
      </div>

      {tab === "registry" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{loadedCount} / {totalCount} tools loaded</span>
            <div style={barBg}>
              <div style={{ ...barFill((loadedCount / totalCount) * 100, "#3b82f6"), minWidth: 120 }} />
            </div>
          </div>
          {manifests.map((m) => (
            <div key={m.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 600 }}>{m.name} <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{m.version}</span></div>
                <div style={labelStyle}>{m.description}</div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{m.sizeKb} KB {m.lastUsed ? `| Last used: ${new Date(m.lastUsed).toLocaleTimeString()}` : ""}</div>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={badgeStyle(m.status)}>{m.status}</span>
                <button style={btnStyle} onClick={() => toggleLoad(m.id)}>
                  {m.status === "loaded" ? "Unload" : m.status === "unloaded" ? "Load" : "..."}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "search" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <input style={inputStyle} placeholder="Search tools by name or description..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} />
          </div>
          {searchQuery.trim() === "" && <div style={cardStyle}>Type a query to search across tool manifests.</div>}
          {searchResults.map((r) => (
            <div key={r.toolId} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600 }}>{r.name}</div>
                <div style={labelStyle}>{r.description}</div>
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Relevance</div>
                <div style={{ fontWeight: 600, color: "var(--accent-primary, #3b82f6)" }}>{(r.relevance * 100).toFixed(0)}%</div>
              </div>
            </div>
          ))}
          {searchQuery.trim() !== "" && searchResults.length === 0 && (
            <div style={cardStyle}>No tools matching "{searchQuery}".</div>
          )}
        </div>
      )}

      {tab === "metrics" && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Context Savings</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "#22c55e" }}>{MOCK_METRICS.contextSavingsPct}%</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Cache Hits</div>
              <div style={{ fontSize: 22, fontWeight: 700 }}>{MOCK_METRICS.cacheHits.toLocaleString()}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Cache Misses</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "#ef4444" }}>{MOCK_METRICS.cacheMisses}</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Cache Hit Rate</div>
            <div style={barBg}>
              <div style={barFill((MOCK_METRICS.cacheHits / (MOCK_METRICS.cacheHits + MOCK_METRICS.cacheMisses)) * 100, "#22c55e")} />
            </div>
            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
              {((MOCK_METRICS.cacheHits / (MOCK_METRICS.cacheHits + MOCK_METRICS.cacheMisses)) * 100).toFixed(1)}%
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Avg Load Time: {MOCK_METRICS.avgLoadTimeMs}ms</div>
            <div style={{ marginTop: 8 }}>
              {MOCK_METRICS.loadTimes.map((lt) => (
                <div key={lt.label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <div style={{ width: 90, fontSize: 11 }}>{lt.label}</div>
                  <div style={{ ...barBg, flex: 1 }}>
                    <div style={barFill((lt.ms / 60) * 100, "#3b82f6")} />
                  </div>
                  <div style={{ width: 40, fontSize: 10, textAlign: "right" }}>{lt.ms}ms</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
