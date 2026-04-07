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

const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const badgeStyle = (variant: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--btn-primary-fg)", background: variant === "loaded" ? "var(--success-color)" : variant === "loading" ? "var(--warning-color)" : "var(--text-secondary)" });
const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

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
      <div className="panel-container">
        <div className="panel-header">
          <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>MCP Lazy Loading</h2>
        </div>
        <div className="panel-body">
          <div className="panel-loading">Loading registry...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>MCP Lazy Loading</h2>
      </div>

      <div className="panel-body">
        {error && (
          <div className="panel-error" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button className="panel-btn panel-btn-secondary" style={{ marginLeft: 8 }} onClick={() => setError(null)}>Dismiss</button>
          </div>
        )}

        <div className="panel-tab-bar">
          <button className={`panel-tab ${tab === "registry" ? "active" : ""}`} onClick={() => setTab("registry")}>Tool Registry</button>
          <button className={`panel-tab ${tab === "search" ? "active" : ""}`} onClick={() => setTab("search")}>Search</button>
          <button className={`panel-tab ${tab === "metrics" ? "active" : ""}`} onClick={() => { setTab("metrics"); fetchMetrics(); }}>Metrics</button>
        </div>

        {tab === "registry" && (
          <div>
            <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span>{loadedCount} / {totalCount} tools loaded</span>
              <div style={{ ...barBg, minWidth: 120 }}>
                <div style={barFill(totalCount > 0 ? (loadedCount / totalCount) * 100 : 0, "var(--info-color)")} />
              </div>
            </div>
            {manifests.map((m) => (
              <div key={m.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
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
                    className="panel-btn panel-btn-secondary"
                    style={{ opacity: actionLoading === m.id ? 0.6 : 1 }}
                    disabled={actionLoading === m.id}
                    onClick={() => toggleLoad(m.id, m.status)}
                  >
                    {actionLoading === m.id ? "..." : m.status === "loaded" ? "Unload" : m.status === "unloaded" ? "Load" : "..."}
                  </button>
                </div>
              </div>
            ))}
            {manifests.length === 0 && <div className="panel-empty">No tools registered.</div>}
          </div>
        )}

        {tab === "search" && (
          <div>
            <div style={{ marginBottom: 12 }}>
              <input className="panel-input panel-input-full" placeholder="Search tools by name or description..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} />
            </div>
            {searchLoading && <div className="panel-loading">Searching...</div>}
            {searchQuery.trim() === "" && !searchLoading && <div className="panel-empty">Type a query to search across tool manifests.</div>}
            {searchResults.map((r) => (
              <div key={r.tool_id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
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
              <div className="panel-empty">No tools matching "{searchQuery}".</div>
            )}
          </div>
        )}

        {tab === "metrics" && metrics && (
          <div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
              <div className="panel-card">
                <div style={labelStyle}>Context Savings</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--success-color)" }}><span className="panel-mono">{metrics.context_savings_pct}%</span></div>
              </div>
              <div className="panel-card">
                <div style={labelStyle}>Cache Hits</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--text-primary)" }}><span className="panel-mono">{metrics.cache_hits.toLocaleString()}</span></div>
              </div>
              <div className="panel-card">
                <div style={labelStyle}>Cache Misses</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--error-color)" }}><span className="panel-mono">{metrics.cache_misses}</span></div>
              </div>
            </div>

            <div className="panel-card">
              <div style={labelStyle}>Cache Hit Rate</div>
              <div style={barBg}>
                <div style={barFill(metrics.cache_hit_rate, "var(--success-color)")} />
              </div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
                {metrics.cache_hit_rate.toFixed(1)}%
              </div>
            </div>

            <div className="panel-card">
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
          <div className="panel-loading">Loading metrics...</div>
        )}
      </div>
    </div>
  );
}
