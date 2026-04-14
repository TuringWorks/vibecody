/* eslint-disable @typescript-eslint/no-explicit-any */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SearchResult {
  id: string;
  title: string;
  url: string;
  snippet: string;
  relevance: number;
}

interface CacheEntry {
  query: string;
  hitCount: number;
  cachedAt: string;
}

interface Citation {
  id: string;
  label: string;
  url: string;
  usedIn: string;
}

export function WebGroundingPanel() {
  const [tab, setTab] = useState("search");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [cacheEntries] = useState<CacheEntry[]>([]);
  const [citations] = useState<Citation[]>([]);
  const [provider, setProvider] = useState("tavily");
  const [apiKey, setApiKey] = useState("");
  const [rateLimit] = useState(60);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;
    setSearching(true);
    setError(null);
    try {
      const data = await invoke<unknown>("web_search", { query });
      const resultList = Array.isArray(data) ? data : (data as any)?.results ?? [];
      const mapped: SearchResult[] = resultList.map((r: any, i: number) => ({
        id: r.id || `r${Date.now()}_${i}`,
        title: r.title || r.name || query,
        url: r.url || r.link || "",
        snippet: r.snippet || r.description || r.content || "",
        relevance: r.relevance ?? r.score ?? 0.5,
      }));
      setResults(mapped);
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }, [query]);

  const handleSemanticSearch = useCallback(async () => {
    if (!query.trim()) return;
    setSearching(true);
    setError(null);
    try {
      const data = await invoke<unknown>("semindex_search", { query });
      const resultList = Array.isArray(data) ? data : (data as any)?.results ?? [];
      const mapped: SearchResult[] = resultList.map((r: any, i: number) => ({
        id: r.id || `s${Date.now()}_${i}`,
        title: r.title || r.symbol || r.name || query,
        url: r.url || r.file || "",
        snippet: r.snippet || r.description || "",
        relevance: r.relevance ?? r.score ?? 0.5,
      }));
      setResults(mapped);
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }, [query]);

  const cacheHitRate = cacheEntries.length > 0 ? ((cacheEntries.reduce((s, c) => s + c.hitCount, 0) / (cacheEntries.length * 10)) * 100).toFixed(0) : "0";

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Web Search Grounding</h2>
      <div className="panel-tab-bar">
        <button className={`panel-tab ${tab === "search" ? "active" : ""}`} onClick={() => setTab("search")}>Search</button>
        <button className={`panel-tab ${tab === "cache" ? "active" : ""}`} onClick={() => setTab("cache")}>Cache</button>
        <button className={`panel-tab ${tab === "citations" ? "active" : ""}`} onClick={() => setTab("citations")}>Citations</button>
        <button className={`panel-tab ${tab === "config" ? "active" : ""}`} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "search" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input className="panel-input" style={{ flex: 1 }} placeholder="Search the web..." value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleSearch()} />
            <button className="panel-btn panel-btn-primary" onClick={handleSearch} disabled={searching}>
              {searching ? "Searching..." : "Web Search"}
            </button>
            <button className="panel-btn panel-btn-secondary" onClick={handleSemanticSearch} disabled={searching}>
              Semantic
            </button>
          </div>
          {error && <div className="panel-error">{error}</div>}
          {results.length === 0 && !searching && <div className="panel-empty">Enter a query and press Search.</div>}
          {results.map((r) => (
            <div key={r.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: "var(--font-size-md)" }}>{r.title}</strong>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", background: "var(--bg-primary)", padding: "2px 6px", borderRadius: "var(--radius-xs-plus)" }}>{(r.relevance * 100).toFixed(0)}%</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--accent-color)", marginTop: 2 }}>{r.url}</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>{r.snippet}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "cache" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <div><strong>{cacheEntries.length}</strong> cached entries | Hit rate: <strong>{cacheHitRate}%</strong></div>
            <button className="panel-btn panel-btn-danger">Clear Cache</button>
          </div>
          {cacheEntries.map((c, i) => (
            <div key={i} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <strong>{c.query}</strong>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{c.hitCount} hits</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Cached: {c.cachedAt}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "citations" && (
        <div>
          {citations.length === 0 && <div className="panel-empty">No citations recorded yet.</div>}
          {citations.map((c) => (
            <div key={c.id} className="panel-card">
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <strong style={{ fontSize: "var(--font-size-lg)", color: "var(--accent-color)" }}>{c.label}</strong>
                <a href={c.url} style={{ fontSize: "var(--font-size-md)", color: "var(--accent-color)", textDecoration: "none" }}>{c.url}</a>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Used in: {c.usedIn}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div className="panel-card">
            <label className="panel-label">Search Provider</label>
            <select value={provider} onChange={(e) => setProvider(e.target.value)} className="panel-input" style={{ width: "auto" }}>
              <option value="tavily">Tavily</option>
              <option value="serp">SerpAPI</option>
              <option value="brave">Brave Search</option>
              <option value="bing">Bing</option>
            </select>
          </div>
          <div className="panel-card">
            <label className="panel-label">API Key</label>
            <input type="password" className="panel-input" value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder="Enter API key" />
          </div>
          <div className="panel-card">
            <label className="panel-label">Rate Limit</label>
            <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)" }}>{rateLimit} requests/minute</div>
          </div>
        </div>
      )}
    </div>
  );
}
