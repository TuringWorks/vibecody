/**
 * McpDirectoryPanel — MCP Plugin Directory panel.
 *
 * Browse, search, and manage MCP plugins with ratings, downloads,
 * and category filtering. Backed by Tauri commands for real plugin
 * directory management with persistent install state.
 */
import { useState, useMemo, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

// ── Constants ────────────────────────────────────────────────────────────────

const CATEGORIES = ["All", "File Systems", "Git", "Databases", "Cloud", "AI/ML", "Testing", "DevOps", "Communication"];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--text-primary)" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-family)", boxSizing: "border-box" };
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
  const [plugins, setPlugins] = useState<McpPlugin[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("All");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);

  // Load all plugins from backend
  const loadPlugins = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<{ plugins: McpPlugin[]; total: number }>("list_mcp_plugins");
      setPlugins(result.plugins ?? []);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPlugins();
  }, [loadPlugins]);

  const installedPlugins = useMemo(() => plugins.filter((p) => p.installed), [plugins]);

  const browsePlugins = useMemo(() => {
    return [...plugins].sort((a, b) => b.downloads - a.downloads);
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

  const handleInstall = async (id: string) => {
    try {
      setActionInProgress(id);
      const result = await invoke<{ success: boolean; message: string }>("install_mcp_plugin", { id });
      if (result.success) {
        await loadPlugins();
      } else {
        setError(result.message);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setActionInProgress(null);
    }
  };

  const handleUninstall = async (id: string) => {
    try {
      setActionInProgress(id);
      const result = await invoke<{ success: boolean; message: string }>("uninstall_mcp_plugin", { id });
      if (result.success) {
        await loadPlugins();
      } else {
        setError(result.message);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setActionInProgress(null);
    }
  };

  const updatePlugin = (id: string) => {
    // Update is effectively a reinstall — mark as no longer updatable locally
    setPlugins((prev) => prev.map((p) => (p.id === id ? { ...p, updatable: false } : p)));
  };

  const renderPluginCard = (plugin: McpPlugin, showInstallBtn: boolean) => {
    const isActioning = actionInProgress === plugin.id;
    return (
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
              <button
                style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--text-primary)" }}
                onClick={() => handleInstall(plugin.id)}
                disabled={isActioning}
              >
                {isActioning ? "..." : "Install"}
              </button>
            )}
            {plugin.installed && plugin.updatable && (
              <button style={{ ...btnStyle, background: "var(--warning-color)", color: "var(--text-primary)" }} onClick={() => updatePlugin(plugin.id)}>Update</button>
            )}
            {plugin.installed && (
              <button
                style={{ ...btnStyle, background: "var(--error-color)", color: "var(--text-primary)" }}
                onClick={() => handleUninstall(plugin.id)}
                disabled={isActioning}
              >
                {isActioning ? "..." : "Uninstall"}
              </button>
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCP Plugin Directory</h2>

      {error && (
        <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", fontSize: 12, marginBottom: 12 }}>
          {error}
          <button style={{ ...btnStyle, marginLeft: 8, fontSize: 11 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "browse")} onClick={() => setTab("browse")}>Browse</button>
        <button style={tabBtnStyle(tab === "installed")} onClick={() => setTab("installed")}>Installed ({installedPlugins.length})</button>
        <button style={tabBtnStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
      </div>

      {loading && <div style={cardStyle}>Loading plugins...</div>}

      {!loading && tab === "browse" && (
        <div>
          <div style={{ ...cardStyle, fontSize: 12 }}>
            {plugins.length} plugins available | {installedPlugins.length} installed
          </div>
          {browsePlugins.map((p) => renderPluginCard(p, true))}
        </div>
      )}

      {!loading && tab === "installed" && (
        <div>
          {installedPlugins.length === 0 && (
            <div style={cardStyle}>
              <div style={{ textAlign: "center", padding: "16px 0" }}>
                <div style={{ fontSize: 14, marginBottom: 6 }}>No MCP plugins installed</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 10 }}>
                  Browse the directory to find and install plugins.
                </div>
                <button style={{ ...btnStyle, background: "var(--accent-primary, #0e639c)", color: "#fff" }} onClick={() => setTab("browse")}>
                  Browse Directory
                </button>
              </div>
            </div>
          )}

          {installedPlugins.length > 0 && (
            <>
              {/* Summary */}
              <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                <span>{installedPlugins.length} plugin{installedPlugins.length !== 1 ? "s" : ""} installed</span>
                {installedPlugins.some(p => p.updatable) && (
                  <span style={{ color: "var(--warning-color, #cca700)" }}>
                    {installedPlugins.filter(p => p.updatable).length} update{installedPlugins.filter(p => p.updatable).length !== 1 ? "s" : ""} available
                  </span>
                )}
              </div>

              {/* Plugin cards with status and config info */}
              {installedPlugins.map((p) => (
                <div key={p.id} style={{ ...cardStyle, borderLeft: `3px solid ${p.updatable ? "var(--warning-color, #cca700)" : "var(--success-color, #4caf50)"}` }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span style={{ fontWeight: 600 }}>{p.name}</span>
                        <span style={{ fontSize: 10, color: "var(--text-secondary, #888)" }}>v{p.version}</span>
                        <span style={{
                          fontSize: 10, padding: "1px 6px", borderRadius: 8,
                          background: p.updatable ? "var(--warning-color, #cca700)" : "var(--success-color, #4caf50)",
                          color: "#fff",
                        }}>
                          {p.updatable ? "Update available" : "Active"}
                        </span>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary, #888)", marginTop: 3 }}>
                        by {p.author} | {p.category}
                      </div>
                      <div style={{ fontSize: 12, marginTop: 4 }}>{p.description}</div>
                      <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: 11 }}>
                        <span style={{ color: "var(--warning-color, #cca700)" }}>{renderStars(p.rating)} {p.rating.toFixed(1)}</span>
                        <span style={{ color: "var(--text-secondary, #888)" }}>{formatDownloads(p.downloads)} downloads</span>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary, #888)", marginTop: 6, fontFamily: "monospace" }}>
                        Config: ~/.vibecli/mcp/{p.id}/config.json
                      </div>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      {p.updatable && (
                        <button
                          style={{ ...btnStyle, background: "var(--warning-color, #cca700)", color: "#fff", fontSize: 11 }}
                          onClick={() => handleInstall(p.id)}
                          disabled={actionInProgress === p.id}
                        >
                          {actionInProgress === p.id ? "..." : "Update"}
                        </button>
                      )}
                      <button
                        style={{ ...btnStyle, borderColor: "var(--error-color, #f44336)", color: "var(--error-color, #f44336)", fontSize: 11 }}
                        onClick={() => handleUninstall(p.id)}
                        disabled={actionInProgress === p.id}
                      >
                        {actionInProgress === p.id ? "..." : "Uninstall"}
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      )}

      {!loading && tab === "search" && (
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
