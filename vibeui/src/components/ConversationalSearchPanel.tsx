import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  result_count: number;
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
  const [results, setResults] = useState<SearchResult[]>([]);
  const [followUps, setFollowUps] = useState<string[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [filters, setFilters] = useState<FilterConfig>({ fileTypes: "*.rs, *.ts, *.tsx", paths: "src/", dateFrom: "", dateTo: "" });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
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
    color: "var(--btn-primary-fg)",
    borderRadius: "3px",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "12px",
  };

  const loadHistory = useCallback(async () => {
    try {
      const hist = await invoke<HistoryEntry[]>("get_search_history");
      setHistory(hist);
    } catch (_e) {
      // History loading is non-critical; silently ignore
    }
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setLoading(true);
    setError(null);

    try {
      const searchResults = await invoke<SearchResult[]>("conversational_search", {
        query,
        fileTypes: filters.fileTypes || null,
        paths: filters.paths || null,
      });
      setResults(searchResults);

      // Fetch follow-up suggestions based on results
      try {
        const suggestions = await invoke<string[]>("get_search_suggestions", {
          query,
          results: searchResults,
        });
        setFollowUps(suggestions);
      } catch (_e) {
        setFollowUps([]);
      }

      // Refresh history after search
      loadHistory();
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error)?.message || "Search failed";
      setError(msg);
      setResults([]);
      setFollowUps([]);
    } finally {
      setLoading(false);
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke("clear_search_history");
      setHistory([]);
    } catch (_e) {
      // ignore
    }
  };

  const relevanceBar = (score: number): React.CSSProperties => ({
    width: `${score * 100}%`,
    height: "4px",
    borderRadius: "2px",
    backgroundColor: score > 0.8 ? "var(--success-color)" : score > 0.6 ? "var(--warning-color)" : "var(--text-secondary)",
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
            <button style={btnStyle} onClick={handleSearch} disabled={loading}>
              {loading ? "Searching..." : "Search"}
            </button>
          </div>
          {error && (
            <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", marginBottom: "12px" }}>
              {error}
            </div>
          )}
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
          {!loading && results.length === 0 && query.trim() && !error && (
            <div style={{ opacity: 0.5, textAlign: "center", padding: "20px" }}>No results found</div>
          )}
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
          {history.length > 0 && (
            <div style={{ marginBottom: "8px", textAlign: "right" }}>
              <button style={{ ...btnStyle, background: "transparent", color: "var(--text-secondary)", border: "1px solid var(--border-color)", fontSize: "11px" }} onClick={handleClearHistory}>
                Clear History
              </button>
            </div>
          )}
          {history.length === 0 && (
            <div style={{ opacity: 0.5, textAlign: "center", padding: "20px" }}>No search history yet</div>
          )}
          {history.map((h) => (
            <div key={h.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
                <strong style={{ fontSize: "13px" }}>{h.query}</strong>
                <span style={{ fontSize: "11px", opacity: 0.6 }}>{h.timestamp}</span>
              </div>
              <p style={{ margin: "4px 0", fontSize: "12px", opacity: 0.8 }}>{h.answer}</p>
              <div style={{ fontSize: "11px", opacity: 0.5 }}>{h.result_count} results</div>
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
