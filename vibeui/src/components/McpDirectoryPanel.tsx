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
      <div key={plugin.id} className="panel-card">
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
                className="panel-btn panel-btn-primary"
                onClick={() => handleInstall(plugin.id)}
                disabled={isActioning}
              >
                {isActioning ? "..." : "Install"}
              </button>
            )}
            {plugin.installed && plugin.updatable && (
              <button className="panel-btn panel-btn-secondary" style={{ background: "var(--warning-color)", color: "var(--text-primary)" }} onClick={() => updatePlugin(plugin.id)}>Update</button>
            )}
            {plugin.installed && (
              <button
                className="panel-btn panel-btn-danger"
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
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>MCP Plugin Directory</h2>

      {error && (
        <div className="panel-error" style={{ marginBottom: 12 }}>
          {error}
          <button className="panel-btn panel-btn-secondary" style={{ marginLeft: 8, fontSize: 11 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        <button className={`panel-tab ${tab === "browse" ? "active" : ""}`} onClick={() => setTab("browse")}>Browse</button>
        <button className={`panel-tab ${tab === "installed" ? "active" : ""}`} onClick={() => setTab("installed")}>Installed ({installedPlugins.length})</button>
        <button className={`panel-tab ${tab === "search" ? "active" : ""}`} onClick={() => setTab("search")}>Search</button>
      </div>

      {loading && <div className="panel-loading">Loading plugins...</div>}

      {!loading && tab === "browse" && (
        <div>
          <div className="panel-card" style={{ fontSize: 12 }}>
            {plugins.length} plugins available | {installedPlugins.length} installed
          </div>
          {browsePlugins.map((p) => renderPluginCard(p, true))}
        </div>
      )}

      {!loading && tab === "installed" && (
        <div>
          {installedPlugins.length === 0 && (
            <div className="panel-card">
              <div style={{ textAlign: "center", padding: "16px 0" }}>
                <div style={{ fontSize: 14, marginBottom: 6 }}>No MCP plugins installed</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 10 }}>
                  Browse the directory to find and install plugins.
                </div>
                <button className="panel-btn panel-btn-primary" onClick={() => setTab("browse")}>
                  Browse Directory
                </button>
              </div>
            </div>
          )}

          {installedPlugins.length > 0 && (
            <>
              {/* Summary */}
              <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                <span>{installedPlugins.length} plugin{installedPlugins.length !== 1 ? "s" : ""} installed</span>
                {installedPlugins.some(p => p.updatable) && (
                  <span style={{ color: "var(--warning-color, #cca700)" }}>
                    {installedPlugins.filter(p => p.updatable).length} update{installedPlugins.filter(p => p.updatable).length !== 1 ? "s" : ""} available
                  </span>
                )}
              </div>

              {/* Plugin cards with status and config info */}
              {installedPlugins.map((p) => (
                <div key={p.id} className="panel-card" style={{ borderLeft: `3px solid ${p.updatable ? "var(--warning-color, #cca700)" : "var(--accent-green)"}` }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span style={{ fontWeight: 600 }}>{p.name}</span>
                        <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{p.version}</span>
                        <span style={{
                          fontSize: 10, padding: "1px 6px", borderRadius: 8,
                          background: p.updatable ? "var(--warning-color, #cca700)" : "var(--accent-green)",
                          color: "var(--btn-primary-fg)",
                        }}>
                          {p.updatable ? "Update available" : "Active"}
                        </span>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 3 }}>
                        by {p.author} | {p.category}
                      </div>
                      <div style={{ fontSize: 12, marginTop: 4 }}>{p.description}</div>
                      <div style={{ display: "flex", gap: 12, marginTop: 6, fontSize: 11 }}>
                        <span style={{ color: "var(--warning-color, #cca700)" }}>{renderStars(p.rating)} {p.rating.toFixed(1)}</span>
                        <span style={{ color: "var(--text-secondary)" }}>{formatDownloads(p.downloads)} downloads</span>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 6, fontFamily: "var(--font-mono)" }}>
                        Config: ~/.vibecli/mcp/{p.id}/config.json
                      </div>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      {p.updatable && (
                        <button
                          className="panel-btn panel-btn-secondary"
                          style={{ background: "var(--warning-color, #cca700)", color: "var(--btn-primary-fg)", fontSize: 11 }}
                          onClick={() => handleInstall(p.id)}
                          disabled={actionInProgress === p.id}
                        >
                          {actionInProgress === p.id ? "..." : "Update"}
                        </button>
                      )}
                      <button
                        className="panel-btn panel-btn-secondary"
                        style={{ borderColor: "var(--accent-rose)", color: "var(--accent-rose)", fontSize: 11 }}
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
          {searchResults.length === 0 && <div className="panel-empty">No plugins match your search.</div>}
          <div className="panel-label">{searchResults.length} result(s)</div>
          {searchResults.map((p) => renderPluginCard(p, true))}
        </div>
      )}
    </div>
  );
}
