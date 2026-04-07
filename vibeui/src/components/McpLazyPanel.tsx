/**
 * McpLazyPanel — MCP Lazy Loading panel.
 *
 * Visualises MCP tool manifests with lazy-loading status, search across
 * tools with relevance scoring, and context-savings / cache metrics.
 * Wired to real Tauri backend commands.
 */
import { useState, useEffect, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ToolManifest {
  id: string;
  name: string;
  description: string;
  version: string;
  server_name: string;
  status: "loaded" | "unloaded" | "loading";
  size_kb: number;
  last_used: string | null;
  load_time_ms: number | null;
}

interface SearchResult {
  tool_id: string;
  name: string;
  description: string;
  server_name: string;
  relevance: number;
}

interface LazyMetrics {
  context_savings_pct: number;
  cache_hits: number;
  cache_misses: number;
  cache_hit_rate: number;
  avg_load_time_ms: number;
  load_times: { label: string; ms: number }[];
  total_load_time_ms: number;
}

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg)" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-family)", boxSizing: "border-box" };
const badgeStyle = (variant: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--btn-primary-fg)", background: variant === "loaded" ? "var(--success-color)" : variant === "loading" ? "var(--warning-color)" : "var(--text-secondary)" });
const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });
const errorStyle: React.CSSProperties = { padding: 12, background: "var(--bg-secondary)", border: "1px solid var(--error-color)", borderRadius: 6, color: "var(--error-color)", marginBottom: 10 };
const spinnerStyle: React.CSSProperties = { textAlign: "center", padding: 24, color: "var(--text-secondary)" };

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "registry" | "search" | "metrics";

export function McpLazyPanel() {
  const [tab, setTab] = useState<Tab>("registry");
  const [manifests, setManifests] = useState<ToolManifest[]>([]);
  const [metrics, setMetrics] = useState<LazyMetrics | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [searchLoading, setSearchLoading] = useState(false);

  // ── Fetch tool list ───────────────────────────────────────────────────────

  const fetchTools = useCallback(async () => {
    try {
      setError(null);
      const result = await invoke<{ tools: ToolManifest[] }>("mcp_lazy_list_tools");
      setManifests(result.tools ?? []);
    } catch (err) {
      setError(`Failed to load tools: ${err}`);
    }
  }, []);

  // ── Fetch metrics ─────────────────────────────────────────────────────────

  const fetchMetrics = useCallback(async () => {
    try {
      const result = await invoke<LazyMetrics>("mcp_lazy_metrics");
      setMetrics(result);
    } catch (err) {
      setError(`Failed to load metrics: ${err}`);
    }
  }, []);

  // ── Initial load ──────────────────────────────────────────────────────────

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      await fetchTools();
      await fetchMetrics();
      if (!cancelled) setLoading(false);
    })();
    return () => { cancelled = true; };
  }, [fetchTools, fetchMetrics]);

  // ── Search ────────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(async () => {
      setSearchLoading(true);
      try {
        const result = await invoke<{ results: SearchResult[] }>("mcp_lazy_search", { query: searchQuery });
        if (!cancelled) setSearchResults(result.results ?? []);
      } catch (err) {
        if (!cancelled) setError(`Search failed: ${err}`);
      } finally {
        if (!cancelled) setSearchLoading(false);
      }
    }, 200);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [searchQuery]);

  // ── Load / Unload ─────────────────────────────────────────────────────────

  const toggleLoad = async (id: string, currentStatus: string) => {
    setActionLoading(id);
    try {
      if (currentStatus === "loaded") {
        await invoke("mcp_lazy_unload_tool", { toolId: id });
      } else {
        // Show "loading" state immediately
        setManifests((prev) =>
          prev.map((m) => (m.id === id ? { ...m, status: "loading" as const } : m))
        );
        await invoke("mcp_lazy_load_tool", { toolId: id });
      }
      await fetchTools();
      await fetchMetrics();
    } catch (err) {
      setError(`Action failed: ${err}`);
      await fetchTools();
    } finally {
      setActionLoading(null);
    }
  };

  // ── Derived values ────────────────────────────────────────────────────────

  const loadedCount = useMemo(() => manifests.filter((m) => m.status === "loaded").length, [manifests]);
  const totalCount = manifests.length;

  // ── Render ────────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div style={panelStyle}>
        <h2 style={headingStyle}>MCP Lazy Loading</h2>
        <div style={spinnerStyle}>Loading registry...</div>
      </div>
    );
  }

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCP Lazy Loading</h2>

      {error && (
        <div style={errorStyle}>
          <span>{error}</span>
          <button style={{ ...btnStyle, marginLeft: 8 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "registry")} onClick={() => setTab("registry")}>Tool Registry</button>
        <button style={tabBtnStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
        <button style={tabBtnStyle(tab === "metrics")} onClick={() => { setTab("metrics"); fetchMetrics(); }}>Metrics</button>
      </div>

      {tab === "registry" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{loadedCount} / {totalCount} tools loaded</span>
            <div style={{ ...barBg, minWidth: 120 }}>
              <div style={barFill(totalCount > 0 ? (loadedCount / totalCount) * 100 : 0, "var(--info-color)")} />
            </div>
          </div>
          {manifests.map((m) => (
            <div key={m.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 600 }}>{m.name} <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{m.version}</span></div>
                <div style={labelStyle}>{m.description}</div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                  {m.size_kb} KB
                  {m.server_name ? ` | Server: ${m.server_name}` : ""}
                  {m.last_used ? ` | Last used: ${new Date(m.last_used).toLocaleTimeString()}` : ""}
                  {m.load_time_ms != null ? ` | Load: ${m.load_time_ms}ms` : ""}
                </div>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={badgeStyle(m.status)}>{m.status}</span>
                <button
                  style={{ ...btnStyle, opacity: actionLoading === m.id ? 0.6 : 1 }}
                  disabled={actionLoading === m.id}
                  onClick={() => toggleLoad(m.id, m.status)}
                >
                  {actionLoading === m.id ? "..." : m.status === "loaded" ? "Unload" : m.status === "unloaded" ? "Load" : "..."}
                </button>
              </div>
            </div>
          ))}
          {manifests.length === 0 && <div style={cardStyle}>No tools registered.</div>}
        </div>
      )}

      {tab === "search" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <input style={inputStyle} placeholder="Search tools by name or description..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} />
          </div>
          {searchLoading && <div style={spinnerStyle}>Searching...</div>}
          {searchQuery.trim() === "" && !searchLoading && <div style={cardStyle}>Type a query to search across tool manifests.</div>}
          {searchResults.map((r) => (
            <div key={r.tool_id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600 }}>{r.name}</div>
                <div style={labelStyle}>{r.description}</div>
                {r.server_name && <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>Server: {r.server_name}</div>}
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Relevance</div>
                <div style={{ fontWeight: 600, color: "var(--accent-primary)" }}>{(r.relevance * 100).toFixed(0)}%</div>
              </div>
            </div>
          ))}
          {searchQuery.trim() !== "" && !searchLoading && searchResults.length === 0 && (
            <div style={cardStyle}>No tools matching "{searchQuery}".</div>
          )}
        </div>
      )}

      {tab === "metrics" && metrics && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Context Savings</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--success-color)" }}>{metrics.context_savings_pct}%</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Cache Hits</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{metrics.cache_hits.toLocaleString()}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Cache Misses</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--error-color)" }}>{metrics.cache_misses}</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Cache Hit Rate</div>
            <div style={barBg}>
              <div style={barFill(metrics.cache_hit_rate, "var(--success-color)")} />
            </div>
            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
              {metrics.cache_hit_rate.toFixed(1)}%
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Avg Load Time: {metrics.avg_load_time_ms}ms</div>
            <div style={{ marginTop: 8 }}>
              {metrics.load_times.map((lt) => (
                <div key={lt.label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <div style={{ width: 90, fontSize: 11 }}>{lt.label}</div>
                  <div style={{ ...barBg, flex: 1 }}>
                    <div style={barFill((lt.ms / 60) * 100, "var(--info-color)")} />
                  </div>
                  <div style={{ width: 40, fontSize: 10, textAlign: "right" }}>{lt.ms}ms</div>
                </div>
              ))}
              {metrics.load_times.length === 0 && (
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>No load times recorded yet.</div>
              )}
            </div>
          </div>
        </div>
      )}

      {tab === "metrics" && !metrics && (
        <div style={spinnerStyle}>Loading metrics...</div>
      )}
    </div>
  );
}
