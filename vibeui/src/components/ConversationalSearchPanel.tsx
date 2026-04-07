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
    <div className="panel-container">
      <div className="panel-header">
        <h3 style={{ margin: 0 }}>Conversational Search</h3>
      </div>
      <div className="panel-tab-bar">
        {tabs.map((t) => (
          <button key={t} className={`panel-tab ${activeTab === t ? "active" : ""}`} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>
      <div className="panel-body">
      {activeTab === "search" && (
        <div>
          <div style={{ display: "flex", gap: "8px", marginBottom: "12px" }}>
            <input className="panel-input panel-input-full" placeholder="Ask about your codebase..." value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleSearch()} />
            <button className="panel-btn panel-btn-primary" onClick={handleSearch} disabled={loading}>
              {loading ? "Searching..." : "Search"}
            </button>
          </div>
          {error && (
            <div className="panel-error" style={{ marginBottom: "12px" }}>
              {error}
            </div>
          )}
          {results.map((r) => (
            <div key={r.id} className="panel-card">
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
            <div className="panel-empty">No results found</div>
          )}
          {followUps.length > 0 && (
            <div style={{ marginTop: "12px" }}>
              <div style={{ fontSize: "12px", opacity: 0.6, marginBottom: "6px" }}>Follow-up suggestions:</div>
              {followUps.map((f, i) => (
                <button key={i} className="panel-btn panel-btn-secondary" style={{ display: "block", marginBottom: "4px", textAlign: "left" }} onClick={() => setQuery(f)}>
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
              <button className="panel-btn panel-btn-secondary" style={{ fontSize: "11px" }} onClick={handleClearHistory}>
                Clear History
              </button>
            </div>
          )}
          {history.length === 0 && (
            <div className="panel-empty">No search history yet</div>
          )}
          {history.map((h) => (
            <div key={h.id} className="panel-card">
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
          <div className="panel-card">
            <div style={{ marginBottom: "10px" }}>
              <label className="panel-label">File Types</label>
              <input className="panel-input panel-input-full" value={filters.fileTypes} onChange={(e) => setFilters({ ...filters, fileTypes: e.target.value })} placeholder="*.rs, *.ts, *.tsx" />
            </div>
            <div style={{ marginBottom: "10px" }}>
              <label className="panel-label">Paths</label>
              <input className="panel-input panel-input-full" value={filters.paths} onChange={(e) => setFilters({ ...filters, paths: e.target.value })} placeholder="src/" />
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <div style={{ flex: 1 }}>
                <label className="panel-label">Date From</label>
                <input className="panel-input panel-input-full" type="date" value={filters.dateFrom} onChange={(e) => setFilters({ ...filters, dateFrom: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label className="panel-label">Date To</label>
                <input className="panel-input panel-input-full" type="date" value={filters.dateTo} onChange={(e) => setFilters({ ...filters, dateTo: e.target.value })} />
              </div>
            </div>
          </div>
        </div>
      )}
      </div>
    </div>
  );
};

export default ConversationalSearchPanel;
