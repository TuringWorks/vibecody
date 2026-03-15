import React, { useState } from "react";

interface SearchResult {
  id: string;
  file: string;
  snippet: string;
  relevance: number;
  line: number;
}

interface HistoryEntry {
  id: string;
  query: string;
  answer: string;
  timestamp: string;
  resultCount: number;
}

interface FilterConfig {
  fileTypes: string;
  paths: string;
  dateFrom: string;
  dateTo: string;
}

const ConversationalSearchPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("search");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([
    { id: "1", file: "src/agent.rs", snippet: "pub async fn execute_tool(&self, name: &str, args: &Value) -> Result<String>", relevance: 0.95, line: 142 },
    { id: "2", file: "src/provider.rs", snippet: "trait AIProvider: Send + Sync { async fn complete(&self, prompt: &str) -> Result<String>; }", relevance: 0.88, line: 23 },
    { id: "3", file: "src/tool_executor.rs", snippet: "fn validate_url_scheme(url: &str) -> bool { matches!(scheme, \"http\" | \"https\") }", relevance: 0.72, line: 67 },
  ]);
  const [followUps] = useState<string[]>([
    "Show all implementations of AIProvider",
    "Find error handling in execute_tool",
    "List callers of validate_url_scheme",
  ]);
  const [history, setHistory] = useState<HistoryEntry[]>([
    { id: "1", query: "How does tool execution work?", answer: "Tool execution routes through execute_tool() in agent.rs, which dispatches based on tool name to the appropriate handler in tool_executor.rs.", timestamp: "12:01", resultCount: 5 },
    { id: "2", query: "Where is URL validation?", answer: "URL validation occurs in tool_executor.rs via validate_url_scheme(), which restricts to http/https to prevent SSRF.", timestamp: "11:48", resultCount: 3 },
  ]);
  const [filters, setFilters] = useState<FilterConfig>({ fileTypes: "*.rs, *.ts, *.tsx", paths: "src/", dateFrom: "", dateTo: "" });

  const containerStyle: React.CSSProperties = {
    padding: "16px",
    color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit",
    fontSize: "13px",
    height: "100%",
    overflow: "auto",
  };

  const tabBarStyle: React.CSSProperties = {
    display: "flex",
    gap: "4px",
    borderBottom: "1px solid var(--border-color)",
    marginBottom: "12px",
  };

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px",
    cursor: "pointer",
    border: "none",
    background: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
    fontFamily: "inherit",
    fontSize: "inherit",
  });

  const cardStyle: React.CSSProperties = {
    padding: "10px",
    marginBottom: "8px",
    borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };

  const inputStyle: React.CSSProperties = {
    padding: "6px 10px",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "3px",
    fontFamily: "inherit",
    fontSize: "inherit",
    width: "100%",
    boxSizing: "border-box",
  };

  const btnStyle: React.CSSProperties = {
    padding: "6px 14px",
    border: "1px solid var(--accent-color)",
    background: "var(--accent-color)",
    color: "white",
    borderRadius: "3px",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "12px",
  };

  const handleSearch = () => {
    if (!query.trim()) return;
    const newResult: SearchResult = {
      id: String(Date.now()),
      file: "src/search_result.rs",
      snippet: `Matched: "${query}" in context...`,
      relevance: 0.65,
      line: 1,
    };
    setResults([newResult, ...results]);
    setHistory((prev) => [
      { id: String(Date.now()), query, answer: `Found results for "${query}"`, timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }), resultCount: results.length + 1 },
      ...prev,
    ]);
  };

  const relevanceBar = (score: number): React.CSSProperties => ({
    width: `${score * 100}%`,
    height: "4px",
    borderRadius: "2px",
    backgroundColor: score > 0.8 ? "var(--success-color)" : score > 0.6 ? "var(--warning-color)" : "var(--text-muted)",
  });

  const tabs = ["search", "history", "settings"];

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Conversational Search</h3>
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "search" && (
        <div>
          <div style={{ display: "flex", gap: "8px", marginBottom: "12px" }}>
            <input style={inputStyle} placeholder="Ask about your codebase..." value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleSearch()} />
            <button style={btnStyle} onClick={handleSearch}>Search</button>
          </div>
          {results.map((r) => (
            <div key={r.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                <strong style={{ fontSize: "13px" }}>{r.file}:{r.line}</strong>
                <span style={{ fontSize: "11px", opacity: 0.7 }}>{(r.relevance * 100).toFixed(0)}%</span>
              </div>
              <code style={{ fontSize: "12px", opacity: 0.85, display: "block", whiteSpace: "pre-wrap", marginBottom: "6px" }}>{r.snippet}</code>
              <div style={{ background: "var(--border-color)", borderRadius: "2px", height: "4px" }}>
                <div style={relevanceBar(r.relevance)} />
              </div>
            </div>
          ))}
          {followUps.length > 0 && (
            <div style={{ marginTop: "12px" }}>
              <div style={{ fontSize: "12px", opacity: 0.6, marginBottom: "6px" }}>Follow-up suggestions:</div>
              {followUps.map((f, i) => (
                <button key={i} style={{ ...btnStyle, display: "block", marginBottom: "4px", textAlign: "left", background: "transparent", color: "var(--accent-color)", border: "1px solid var(--border-color)" }} onClick={() => setQuery(f)}>
                  {f}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === "history" && (
        <div>
          {history.map((h) => (
            <div key={h.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
                <strong style={{ fontSize: "13px" }}>{h.query}</strong>
                <span style={{ fontSize: "11px", opacity: 0.6 }}>{h.timestamp}</span>
              </div>
              <p style={{ margin: "4px 0", fontSize: "12px", opacity: 0.8 }}>{h.answer}</p>
              <div style={{ fontSize: "11px", opacity: 0.5 }}>{h.resultCount} results</div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "settings" && (
        <div>
          <div style={cardStyle}>
            <div style={{ marginBottom: "10px" }}>
              <label style={{ fontSize: "12px", fontWeight: 600, display: "block", marginBottom: "4px" }}>File Types</label>
              <input style={inputStyle} value={filters.fileTypes} onChange={(e) => setFilters({ ...filters, fileTypes: e.target.value })} placeholder="*.rs, *.ts, *.tsx" />
            </div>
            <div style={{ marginBottom: "10px" }}>
              <label style={{ fontSize: "12px", fontWeight: 600, display: "block", marginBottom: "4px" }}>Paths</label>
              <input style={inputStyle} value={filters.paths} onChange={(e) => setFilters({ ...filters, paths: e.target.value })} placeholder="src/" />
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "12px", fontWeight: 600, display: "block", marginBottom: "4px" }}>Date From</label>
                <input style={inputStyle} type="date" value={filters.dateFrom} onChange={(e) => setFilters({ ...filters, dateFrom: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label style={{ fontSize: "12px", fontWeight: 600, display: "block", marginBottom: "4px" }}>Date To</label>
                <input style={inputStyle} type="date" value={filters.dateTo} onChange={(e) => setFilters({ ...filters, dateTo: e.target.value })} />
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ConversationalSearchPanel;
