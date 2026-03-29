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

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: 8,
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: 13,
};

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
    <div style={panelStyle}>
      <h2 style={headingStyle}>Web Search Grounding</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
        <button style={tabStyle(tab === "cache")} onClick={() => setTab("cache")}>Cache</button>
        <button style={tabStyle(tab === "citations")} onClick={() => setTab("citations")}>Citations</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "search" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="Search the web..." value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleSearch()} />
            <button style={btnStyle} onClick={handleSearch} disabled={searching}>
              {searching ? "Searching..." : "Web Search"}
            </button>
            <button style={{ ...btnStyle, background: "var(--bg-secondary)" }} onClick={handleSemanticSearch} disabled={searching}>
              Semantic
            </button>
          </div>
          {error && <div style={{ color: "var(--error-color)", fontSize: 12, marginBottom: 8 }}>{error}</div>}
          {results.length === 0 && !searching && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Enter a query and press Search.</div>}
          {results.map((r) => (
            <div key={r.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: 13 }}>{r.title}</strong>
                <span style={{ fontSize: 11, color: "var(--text-secondary)", background: "var(--bg-primary)", padding: "2px 6px", borderRadius: 4 }}>{(r.relevance * 100).toFixed(0)}%</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--accent-color)", marginTop: 2 }}>{r.url}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>{r.snippet}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "cache" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <div><strong>{cacheEntries.length}</strong> cached entries | Hit rate: <strong>{cacheHitRate}%</strong></div>
            <button style={{ ...btnStyle, background: "var(--error-color)" }}>Clear Cache</button>
          </div>
          {cacheEntries.map((c, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <strong>{c.query}</strong>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{c.hitCount} hits</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Cached: {c.cachedAt}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "citations" && (
        <div>
          {citations.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No citations recorded yet.</div>}
          {citations.map((c) => (
            <div key={c.id} style={cardStyle}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <strong style={{ fontSize: 14, color: "var(--accent-color)" }}>{c.label}</strong>
                <a href={c.url} style={{ fontSize: 13, color: "var(--accent-color)", textDecoration: "none" }}>{c.url}</a>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Used in: {c.usedIn}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Search Provider</div>
            <select value={provider} onChange={(e) => setProvider(e.target.value)} style={{ ...inputStyle, width: "auto" }}>
              <option value="tavily">Tavily</option>
              <option value="serp">SerpAPI</option>
              <option value="brave">Brave Search</option>
              <option value="bing">Bing</option>
            </select>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>API Key</div>
            <input type="password" style={inputStyle} value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder="Enter API key" />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>Rate Limit</div>
            <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{rateLimit} requests/minute</div>
          </div>
        </div>
      )}
    </div>
  );
}
