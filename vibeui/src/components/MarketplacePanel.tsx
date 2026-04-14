/**
 * MarketplacePanel — Browse, search, and install community plugins.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MarketplacePlugin {
  name: string;
  description: string;
  version: string;
  author: string;
  repo_url: string;
  tags: string[];
  downloads: number;
  updated_at: string;
}

export function MarketplacePanel() {
  const [plugins, setPlugins] = useState<MarketplacePlugin[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [message, setMessage] = useState<{ text: string; type: "ok" | "err" } | null>(null);

  useEffect(() => {
    loadPlugins();
    loadInstalled();
  }, []);

  const loadInstalled = async () => {
    try {
      const names = await invoke<string[]>("list_installed_plugins");
      setInstalled(new Set(names));
    } catch { /* ignore */ }
  };

  const loadPlugins = async () => {
    setLoading(true);
    try {
      const list = await invoke<MarketplacePlugin[]>("get_marketplace_plugins");
      setPlugins(list);
    } catch {
      setPlugins([]);
    }
    setLoading(false);
  };

  const handleSearch = async () => {
    if (!search.trim()) { loadPlugins(); return; }
    setLoading(true);
    try {
      const results = await invoke<MarketplacePlugin[]>("search_marketplace", { query: search.trim() });
      setPlugins(results);
    } catch {
      setPlugins([]);
    }
    setLoading(false);
  };

  const handleInstall = async (name: string, repoUrl: string) => {
    setInstalling(name);
    setMessage(null);
    try {
      await invoke("install_marketplace_plugin", { name, repoUrl });
      setInstalled(prev => new Set(prev).add(name));
      setMessage({ text: `${name} installed successfully!`, type: "ok" });
    } catch (e) {
      setMessage({ text: `Failed: ${e}`, type: "err" });
    }
    setInstalling(null);
  };

  const filtered = plugins;

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <h3>Marketplace</h3>
        <div style={{ marginLeft: "auto" }} />
        <button onClick={loadPlugins} className="panel-btn panel-btn-secondary">Refresh</button>
      </div>

      {/* Search */}
      <div style={{ padding: "6px 12px", display: "flex", gap: 6 }}>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          placeholder="Search plugins..."
          className="panel-input"
          style={{ flex: 1 }}
        />
        <button onClick={handleSearch} className="panel-btn panel-btn-secondary">Search</button>
      </div>

      {message && (
        <div style={{
          margin: "0 12px", padding: "4px 8px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)",
          color: message.type === "ok" ? "var(--success-color)" : "var(--error-color)",
          background: message.type === "ok" ? "rgba(166,227,161,0.05)" : "color-mix(in srgb, var(--accent-rose) 5%, transparent)",
        }}>
          {message.text}
        </div>
      )}

      <div className="panel-body">
        {loading ? (
          <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: "var(--font-size-sm)" }}>
            Loading...
          </div>
        ) : filtered.length === 0 ? (
          <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: "var(--font-size-sm)" }}>
            No plugins found.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {filtered.map((p) => (
              <div key={p.name} style={{
                padding: "8px 10px", borderRadius: "var(--radius-sm)",
                border: "1px solid var(--border-color)",
                background: "var(--bg-primary)",
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <span style={{ fontSize: "var(--font-size-base)", fontWeight: 700 }}>{p.name}</span>
                  <span style={{ fontSize: 9, opacity: 0.5 }}>v{p.version}</span>
                  <div style={{ flex: 1 }} />
                  {installed.has(p.name) ? (
                    <span className="panel-tag panel-tag-success" style={{ cursor: "default" }}>
                      ✓ Installed
                    </span>
                  ) : (
                    <button
                      onClick={() => handleInstall(p.name, p.repo_url)}
                      disabled={installing === p.name}
                      className="panel-btn panel-btn-primary"
                      style={{ opacity: installing === p.name ? 0.5 : 1 }}
                    >
                      {installing === p.name ? "Installing..." : "Install"}
                    </button>
                  )}
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.8, marginBottom: 4 }}>{p.description}</div>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={{ fontSize: 9, opacity: 0.5 }}>by {p.author}</span>
                  {p.tags.slice(0, 4).map((tag) => (
                    <span key={tag} style={{
                      fontSize: 8, padding: "1px 5px", borderRadius: 3,
                      background: "color-mix(in srgb, var(--accent-blue) 10%, transparent)", color: "var(--text-info)",
                    }}>
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

