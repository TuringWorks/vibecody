import React, { useState } from "react";

interface SearchResult {
  file: string;
  line: number;
  snippet: string;
  relevance: number;
  matchType: string;
}

const FastContextPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("search");
  const [query, setQuery] = useState("");
  const [matchType, setMatchType] = useState("Exact");
  const [results, setResults] = useState<SearchResult[]>([
    { file: "src/main.rs", line: 42, snippet: "fn main() { ... }", relevance: 0.98, matchType: "Exact" },
    { file: "src/config.rs", line: 15, snippet: "pub struct Config { ... }", relevance: 0.87, matchType: "Symbol" },
    { file: "src/agent.rs", line: 210, snippet: "async fn run_agent(...)", relevance: 0.74, matchType: "Fuzzy" },
  ]);
  const [indexStats, setIndexStats] = useState({ files: 1247, symbols: 18432, trigrams: 294817, lastBuilt: "2 min ago" });
  const [cacheStats, setCacheStats] = useState({ hits: 3842, misses: 291, size: "14.2 MB", maxSize: "64 MB" });

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--vscode-panel-border)",
    paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-button-background)" : "transparent",
    color: active ? "var(--vscode-button-foreground)" : "var(--vscode-foreground)",
    borderRadius: "4px", fontSize: "var(--vscode-font-size)",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--vscode-input-background)",
    color: "var(--vscode-input-foreground)",
    border: "1px solid var(--vscode-input-border)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--vscode-button-background)",
    color: "var(--vscode-button-foreground)",
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--vscode-editor-inactiveSelectionBackground)",
    border: "1px solid var(--vscode-panel-border)",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "#fff",
  });
  const statRow: React.CSSProperties = {
    display: "flex", justifyContent: "space-between", padding: "8px 0",
    borderBottom: "1px solid var(--vscode-panel-border)",
  };

  const matchTypes = ["Exact", "Fuzzy", "Semantic", "Structural", "Symbol"];
  const hitRate = cacheStats.hits + cacheStats.misses > 0
    ? ((cacheStats.hits / (cacheStats.hits + cacheStats.misses)) * 100).toFixed(1) : "0";

  const handleSearch = () => {
    if (!query.trim()) return;
    setResults(prev => [
      { file: "search/result.rs", line: 1, snippet: query, relevance: Math.random() * 0.3 + 0.7, matchType },
      ...prev,
    ]);
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
        <button style={btnStyle} onClick={handleSearch}>Search</button>
      </div>
      <div style={{ fontSize: "12px", marginBottom: "8px", opacity: 0.7 }}>
        {results.length} result{results.length !== 1 ? "s" : ""}
      </div>
      {results.map((r, i) => (
        <div key={i} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
            <span style={{ fontWeight: 600 }}>{r.file}:{r.line}</span>
            <span style={badgeStyle(r.relevance > 0.9 ? "#2e7d32" : r.relevance > 0.7 ? "#f57f17" : "#c62828")}>
              {(r.relevance * 100).toFixed(0)}%
            </span>
          </div>
          <code style={{ fontSize: "12px", opacity: 0.8 }}>{r.snippet}</code>
          <div style={{ marginTop: "4px" }}>
            <span style={badgeStyle("#5c6bc0")}>{r.matchType}</span>
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
      <div style={{ marginTop: "16px" }}>
        <button style={btnStyle} onClick={() => setIndexStats(s => ({ ...s, lastBuilt: "just now" }))}>
          Rebuild Index
        </button>
      </div>
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
        <button style={btnStyle} onClick={() => setCacheStats({ hits: 0, misses: 0, size: "0 MB", maxSize: "64 MB" })}>
          Clear Cache
        </button>
      </div>
    </div>
  );

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
