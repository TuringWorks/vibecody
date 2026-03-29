import { useState, useCallback } from "react";

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
  const [results, setResults] = useState<SearchResult[]>([
    { id: "r1", title: "Rust async/await patterns", url: "https://doc.rust-lang.org/book/ch17-00-async-await.html", snippet: "Learn how to use async functions and the await keyword in Rust...", relevance: 0.95 },
    { id: "r2", title: "Tokio runtime guide", url: "https://tokio.rs/tokio/tutorial", snippet: "Tokio is an asynchronous runtime for the Rust programming language...", relevance: 0.88 },
  ]);
  const [cacheEntries] = useState<CacheEntry[]>([
    { query: "rust async patterns", hitCount: 5, cachedAt: "2026-03-26 09:00" },
    { query: "tauri v2 commands", hitCount: 3, cachedAt: "2026-03-26 08:30" },
  ]);
  const [citations] = useState<Citation[]>([
    { id: "c1", label: "[1]", url: "https://doc.rust-lang.org/book/ch17-00-async-await.html", usedIn: "Agent response #42" },
    { id: "c2", label: "[2]", url: "https://tokio.rs/tokio/tutorial", usedIn: "Agent response #42" },
  ]);
  const [provider, setProvider] = useState("tavily");
  const [apiKey, setApiKey] = useState("");
  const [rateLimit] = useState(60);

  const handleSearch = useCallback(() => {
    if (!query.trim()) return;
    setResults((prev) => [
      { id: `r${Date.now()}`, title: `Result for: ${query}`, url: "https://example.com", snippet: `Search result for "${query}"...`, relevance: 0.75 },
      ...prev,
    ]);
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
            <button style={btnStyle} onClick={handleSearch}>Search</button>
          </div>
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
