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
  const [message, setMessage] = useState<{ text: string; type: "ok" | "err" } | null>(null);

  useEffect(() => {
    loadPlugins();
  }, []);

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
      setMessage({ text: `${name} installed successfully!`, type: "ok" });
    } catch (e) {
      setMessage({ text: `Failed: ${e}`, type: "err" });
    }
    setInstalling(null);
  };

  const filtered = plugins;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Marketplace</span>
        <div style={{ flex: 1 }} />
        <button onClick={loadPlugins} style={chipStyle}>Refresh</button>
      </div>

      {/* Search */}
      <div style={{ padding: "6px 12px", display: "flex", gap: 6 }}>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          placeholder="Search plugins..."
          style={{ ...inputStyle, flex: 1 }}
        />
        <button onClick={handleSearch} style={{ ...chipStyle, cursor: "pointer" }}>Search</button>
      </div>

      {message && (
        <div style={{
          margin: "0 12px", padding: "4px 8px", fontSize: 11, borderRadius: 4,
          color: message.type === "ok" ? "#a6e3a1" : "#f38ba8",
          background: message.type === "ok" ? "rgba(166,227,161,0.05)" : "rgba(243,139,168,0.05)",
        }}>
          {message.text}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {loading ? (
          <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
            Loading...
          </div>
        ) : filtered.length === 0 ? (
          <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
            No plugins found.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {filtered.map((p) => (
              <div key={p.name} style={{
                padding: "8px 10px", borderRadius: 6,
                border: "1px solid var(--border-color)",
                background: "var(--bg-primary)",
              }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <span style={{ fontSize: 12, fontWeight: 700 }}>{p.name}</span>
                  <span style={{ fontSize: 9, opacity: 0.5 }}>v{p.version}</span>
                  <div style={{ flex: 1 }} />
                  <button
                    onClick={() => handleInstall(p.name, p.repo_url)}
                    disabled={installing === p.name}
                    style={{
                      ...chipStyle, cursor: "pointer",
                      background: "rgba(99,102,241,0.15)", border: "1px solid #6366f1",
                      opacity: installing === p.name ? 0.5 : 1,
                    }}
                  >
                    {installing === p.name ? "Installing..." : "Install"}
                  </button>
                </div>
                <div style={{ fontSize: 11, opacity: 0.8, marginBottom: 4 }}>{p.description}</div>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={{ fontSize: 9, opacity: 0.5 }}>by {p.author}</span>
                  {p.tags.slice(0, 4).map((tag) => (
                    <span key={tag} style={{
                      fontSize: 8, padding: "1px 5px", borderRadius: 3,
                      background: "rgba(99,102,241,0.1)", color: "var(--text-info, #89b4fa)",
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

const chipStyle: React.CSSProperties = {
  padding: "3px 8px", fontSize: 10, fontWeight: 600, borderRadius: 4, cursor: "pointer",
  border: "1px solid var(--border-color)",
  background: "transparent", color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};
