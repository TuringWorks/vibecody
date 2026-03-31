import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SearchResult {
  file: string;
  line: number;
  snippet: string;
  relevance: number;
  matchType: string;
}

interface IndexStats {
  files: number;
  symbols: number;
  trigrams: number;
  lastBuilt: string;
  languages: string[];
}

interface CacheStats {
  hits: number;
  misses: number;
  size: string;
  maxSize: string;
}

const FastContextPanel: React.FC<{ workspacePath?: string | null }> = ({ workspacePath }) => {
  const [activeTab, setActiveTab] = useState<string>("search");
  const [query, setQuery] = useState("");
  const [matchType, setMatchType] = useState("Exact");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexStats, setIndexStats] = useState<IndexStats>({ files: 0, symbols: 0, trigrams: 0, lastBuilt: "-", languages: [] });
  const [cacheStats, setCacheStats] = useState<CacheStats>({ hits: 0, misses: 0, size: "0 MB", maxSize: "64 MB" });
  const [reindexing, setReindexing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const workspace = workspacePath || "";

  const loadIndexStats = useCallback(async () => {
    if (!workspace) return;
    try {
      const stats = await invoke<IndexStats>("fast_context_index_stats", { workspace });
      setIndexStats(stats);
    } catch (err) {
      console.error("Failed to load index stats:", err);
    }
  }, [workspace]);

  const loadCacheStats = useCallback(async () => {
    if (!workspace) return;
    try {
      const stats = await invoke<CacheStats>("fast_context_cache_stats", { workspace });
      setCacheStats(stats);
    } catch (err) {
      console.error("Failed to load cache stats:", err);
    }
  }, [workspace]);

  useEffect(() => {
    loadIndexStats();
    loadCacheStats();
  }, [loadIndexStats, loadCacheStats]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "var(--font-mono, monospace)", fontSize: "13px",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--border-color)",
    paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--accent-color)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-primary)",
    borderRadius: "4px", fontSize: "13px",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--accent-color)",
    color: "var(--text-primary)",
  };
  const btnDisabledStyle: React.CSSProperties = {
    ...btnStyle,
    opacity: 0.6,
    cursor: "not-allowed",
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "var(--btn-primary-fg)",
  });
  const statRow: React.CSSProperties = {
    display: "flex", justifyContent: "space-between", padding: "8px 0",
    borderBottom: "1px solid var(--border-color)",
  };

  const matchTypes = ["Exact", "Fuzzy", "Semantic", "Structural", "Symbol"];
  const hitRate = cacheStats.hits + cacheStats.misses > 0
    ? ((cacheStats.hits / (cacheStats.hits + cacheStats.misses)) * 100).toFixed(1) : "0";

  const handleSearch = async () => {
    if (!query.trim() || !workspace) return;
    setSearching(true);
    setError(null);
    try {
      const res = await invoke<SearchResult[]>("fast_context_search", {
        workspace,
        query,
        matchType,
      });
      setResults(res);
    } catch (err) {
      setError(String(err));
      console.error("Search failed:", err);
    } finally {
      setSearching(false);
    }
  };

  const handleReindex = async () => {
    if (!workspace) return;
    setReindexing(true);
    setError(null);
    try {
      const stats = await invoke<IndexStats>("fast_context_reindex", { workspace });
      setIndexStats(stats);
      await loadCacheStats();
    } catch (err) {
      setError(String(err));
      console.error("Reindex failed:", err);
    } finally {
      setReindexing(false);
    }
  };

  const handleClearCache = async () => {
    setCacheStats({ hits: 0, misses: 0, size: "0 MB", maxSize: "64 MB" });
  };

  const renderSearch = () => (
    <div>
      <div style={{ display: "flex", gap: "8px", marginBottom: "12px" }}>
        <input style={{ ...inputStyle, flex: 1 }} placeholder="Search query..." value={query}
          onChange={e => setQuery(e.target.value)} onKeyDown={e => e.key === "Enter" && handleSearch()} />
        <select style={{ ...inputStyle, width: "140px" }} value={matchType}
          onChange={e => setMatchType(e.target.value)}>
          {matchTypes.map(t => <option key={t} value={t}>{t}</option>)}
        </select>
        <button style={searching ? btnDisabledStyle : btnStyle} onClick={handleSearch} disabled={searching}>
          {searching ? "Searching..." : "Search"}
        </button>
      </div>
      {error && (
        <div style={{ color: "var(--error-color)", fontSize: "12px", marginBottom: "8px" }}>{error}</div>
      )}
      <div style={{ fontSize: "12px", marginBottom: "8px", opacity: 0.7 }}>
        {results.length} result{results.length !== 1 ? "s" : ""}
      </div>
      {results.map((r, i) => (
        <div key={i} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
            <span style={{ fontWeight: 600 }}>{r.file}:{r.line}</span>
            <span style={badgeStyle(r.relevance > 0.9 ? "var(--success-color)" : r.relevance > 0.7 ? "var(--warning-color)" : "var(--error-color)")}>
              {(r.relevance * 100).toFixed(0)}%
            </span>
          </div>
          <code style={{ fontSize: "12px", opacity: 0.8 }}>{r.snippet}</code>
          <div style={{ marginTop: "4px" }}>
            <span style={badgeStyle("var(--accent-color)")}>{r.matchType}</span>
          </div>
        </div>
      ))}
    </div>
  );

  const renderIndex = () => (
    <div>
      <h3 style={{ margin: "0 0 12px" }}>Index Statistics</h3>
      <div style={statRow}><span>Indexed Files</span><strong>{indexStats.files.toLocaleString()}</strong></div>
      <div style={statRow}><span>Symbols</span><strong>{indexStats.symbols.toLocaleString()}</strong></div>
      <div style={statRow}><span>Trigrams</span><strong>{indexStats.trigrams.toLocaleString()}</strong></div>
      <div style={statRow}><span>Last Built</span><strong>{indexStats.lastBuilt}</strong></div>
      {indexStats.languages.length > 0 && (
        <div style={statRow}>
          <span>Languages</span>
          <strong>{indexStats.languages.join(", ")}</strong>
        </div>
      )}
      <div style={{ marginTop: "16px" }}>
        <button style={reindexing ? btnDisabledStyle : btnStyle} onClick={handleReindex} disabled={reindexing}>
          {reindexing ? "Rebuilding..." : "Rebuild Index"}
        </button>
      </div>
      {error && (
        <div style={{ color: "var(--error-color)", fontSize: "12px", marginTop: "8px" }}>{error}</div>
      )}
    </div>
  );

  const renderCache = () => (
    <div>
      <h3 style={{ margin: "0 0 12px" }}>Cache Statistics</h3>
      <div style={{ ...cardStyle, textAlign: "center", marginBottom: "16px" }}>
        <div style={{ fontSize: "28px", fontWeight: 700 }}>{hitRate}%</div>
        <div style={{ opacity: 0.7, fontSize: "12px" }}>Hit Rate</div>
      </div>
      <div style={statRow}><span>Cache Hits</span><strong>{cacheStats.hits.toLocaleString()}</strong></div>
      <div style={statRow}><span>Cache Misses</span><strong>{cacheStats.misses.toLocaleString()}</strong></div>
      <div style={statRow}><span>Cache Size</span><strong>{cacheStats.size}</strong></div>
      <div style={statRow}><span>Max Size</span><strong>{cacheStats.maxSize}</strong></div>
      <div style={{ marginTop: "16px" }}>
        <button style={btnStyle} onClick={handleClearCache}>
          Clear Cache
        </button>
      </div>
    </div>
  );

  if (!workspace) {
    return (
      <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>
        <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 8, color: "var(--text-primary)" }}>Fast Context</div>
        <p>Open a folder to use fast context indexing.</p>
      </div>
    );
  }

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Fast Context</h2>
      <div style={tabBarStyle}>
        {[["search", "Search"], ["index", "Index"], ["cache", "Cache"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "search" && renderSearch()}
      {activeTab === "index" && renderIndex()}
      {activeTab === "cache" && renderCache()}
    </div>
  );
};

export default FastContextPanel;
