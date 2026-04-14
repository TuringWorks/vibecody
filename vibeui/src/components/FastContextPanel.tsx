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

  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
    fontSize: "var(--font-size-sm)", fontWeight: 600, backgroundColor: color, color: "var(--btn-primary-fg)",
  });
  const statRow: React.CSSProperties = {
    display: "flex", justifyContent: "space-between", padding: "8px 0",
    borderBottom: "1px solid var(--border-color)",
    color: "var(--text-secondary)",
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
        <button className="panel-btn panel-btn-primary" onClick={handleSearch} disabled={searching} style={{ opacity: searching ? 0.6 : 1 }}>
          {searching ? "Searching..." : "Search"}
        </button>
      </div>
      {error && (
        <div className="panel-error" style={{ marginBottom: "8px" }}>{error}</div>
      )}
      <div style={{ fontSize: "var(--font-size-base)", marginBottom: "8px", color: "var(--text-secondary)" }}>
        {results.length} result{results.length !== 1 ? "s" : ""}
      </div>
      {results.map((r, i) => (
        <div key={i} className="panel-card" style={{ marginBottom: "8px" }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
            <span style={{ fontWeight: 600 }}>{r.file}:{r.line}</span>
            <span style={badgeStyle(r.relevance > 0.9 ? "var(--success-color)" : r.relevance > 0.7 ? "var(--warning-color)" : "var(--error-color)")}>
              {(r.relevance * 100).toFixed(0)}%
            </span>
          </div>
          <code style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{r.snippet}</code>
          <div style={{ marginTop: "4px" }}>
            <span style={badgeStyle("var(--accent-color)")}>{r.matchType}</span>
          </div>
        </div>
      ))}
    </div>
  );

  const renderIndex = () => (
    <div>
      <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Index Statistics</h3>
      <div style={statRow}><span>Indexed Files</span><strong style={{ color: "var(--text-primary)" }}>{indexStats.files.toLocaleString()}</strong></div>
      <div style={statRow}><span>Symbols</span><strong style={{ color: "var(--text-primary)" }}>{indexStats.symbols.toLocaleString()}</strong></div>
      <div style={statRow}><span>Trigrams</span><strong style={{ color: "var(--text-primary)" }}>{indexStats.trigrams.toLocaleString()}</strong></div>
      <div style={statRow}><span>Last Built</span><strong style={{ color: "var(--text-primary)" }}>{indexStats.lastBuilt}</strong></div>
      {indexStats.languages.length > 0 && (
        <div style={statRow}>
          <span>Languages</span>
          <strong style={{ color: "var(--text-primary)" }}>{indexStats.languages.join(", ")}</strong>
        </div>
      )}
      <div style={{ marginTop: "16px" }}>
        <button className="panel-btn panel-btn-primary" onClick={handleReindex} disabled={reindexing} style={{ opacity: reindexing ? 0.6 : 1 }}>
          {reindexing ? "Rebuilding..." : "Rebuild Index"}
        </button>
      </div>
      {error && (
        <div className="panel-error" style={{ marginTop: "8px" }}>{error}</div>
      )}
    </div>
  );

  const renderCache = () => (
    <div>
      <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Cache Statistics</h3>
      <div className="panel-card" style={{ textAlign: "center", marginBottom: "16px" }}>
        <div style={{ fontSize: "28px", fontWeight: 700, color: "var(--text-primary)" }}>{hitRate}%</div>
        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Hit Rate</div>
      </div>
      <div style={statRow}><span>Cache Hits</span><strong style={{ color: "var(--text-primary)" }}>{cacheStats.hits.toLocaleString()}</strong></div>
      <div style={statRow}><span>Cache Misses</span><strong style={{ color: "var(--text-primary)" }}>{cacheStats.misses.toLocaleString()}</strong></div>
      <div style={statRow}><span>Cache Size</span><strong style={{ color: "var(--text-primary)" }}>{cacheStats.size}</strong></div>
      <div style={statRow}><span>Max Size</span><strong style={{ color: "var(--text-primary)" }}>{cacheStats.maxSize}</strong></div>
      <div style={{ marginTop: "16px" }}>
        <button className="panel-btn panel-btn-secondary" onClick={handleClearCache}>
          Clear Cache
        </button>
      </div>
    </div>
  );

  if (!workspace) {
    return (
      <div className="panel-empty" style={{ padding: 24, textAlign: "center", fontSize: "var(--font-size-md)" }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 8, color: "var(--text-primary)" }}>Fast Context</div>
        <p>Open a folder to use fast context indexing.</p>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-header">Fast Context</div>
      <div className="panel-tab-bar">
        {[["search", "Search"], ["index", "Index"], ["cache", "Cache"]].map(([id, label]) => (
          <button key={id} className={`panel-tab ${activeTab === id ? "active" : ""}`} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      <div className="panel-body">
        {activeTab === "search" && renderSearch()}
        {activeTab === "index" && renderIndex()}
        {activeTab === "cache" && renderCache()}
      </div>
    </div>
  );
};

export default FastContextPanel;
