import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ThoughtEntry {
  id: string;
  timestamp: string;
  category: "Planning" | "Reasoning" | "Uncertainty" | "Decision" | "Observation";
  content: string;
  confidence: number | null;
}

type TagIntent = "info" | "success" | "warning" | "danger" | "neutral";

const CATEGORY_INTENT: Record<ThoughtEntry["category"], TagIntent> = {
  Planning: "info",
  Reasoning: "neutral",
  Uncertainty: "warning",
  Decision: "success",
  Observation: "info",
};

const CATEGORIES: ThoughtEntry["category"][] = [
  "Planning", "Reasoning", "Uncertainty", "Decision", "Observation",
];

export function ThoughtStreamPanel() {
  const [tab, setTab] = useState<"live" | "history" | "export">("live");
  const [liveThoughts, setLiveThoughts] = useState<ThoughtEntry[]>([]);
  const [historyThoughts, setHistoryThoughts] = useState<ThoughtEntry[]>([]);
  const [exportMd, setExportMd] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterCategory, setFilterCategory] = useState<"All" | ThoughtEntry["category"]>("All");
  const [copied, setCopied] = useState(false);
  const liveRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [liveRes, historyRes, exportRes] = await Promise.all([
          invoke<ThoughtEntry[]>("thought_stream_live"),
          invoke<ThoughtEntry[]>("thought_stream_history"),
          invoke<string>("thought_stream_export"),
        ]);
        setLiveThoughts(Array.isArray(liveRes) ? liveRes : []);
        setHistoryThoughts(Array.isArray(historyRes) ? historyRes : []);
        setExportMd(typeof exportRes === "string" ? exportRes : "");
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
    const interval = setInterval(async () => {
      try {
        const res = await invoke<ThoughtEntry[]>("thought_stream_live");
        setLiveThoughts(Array.isArray(res) ? res : []);
      } catch { /* silent */ }
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (tab === "live" && liveRef.current) {
      liveRef.current.scrollTop = liveRef.current.scrollHeight;
    }
  }, [liveThoughts, tab]);

  const filteredHistory = filterCategory === "All"
    ? historyThoughts
    : historyThoughts.filter(t => t.category === filterCategory);

  function copyExport() {
    navigator.clipboard?.writeText(exportMd);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  const ThoughtCard = ({ thought }: { thought: ThoughtEntry }) => {
    const intent = CATEGORY_INTENT[thought.category];
    return (
      <div className="panel-card" style={{ marginBottom: 8 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
          <span className={`panel-tag panel-tag-${intent}`}>{thought.category}</span>
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{thought.timestamp}</span>
          {thought.confidence !== null && (
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginLeft: "auto" }}>
              conf: {Math.round(thought.confidence * 100)}%
            </span>
          )}
        </div>
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>
          {thought.content}
        </div>
      </div>
    );
  };

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Thought Stream</h3>
        {tab === "live" && (
          <span style={{ marginLeft: "auto", fontSize: "var(--font-size-sm)", color: "var(--success-color)", display: "flex", alignItems: "center", gap: 4 }}>
            <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success-color)", display: "inline-block" }} />
            Live
          </span>
        )}
      </div>

      <div className="panel-body">
        <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
          {(["live", "history", "export"] as const).map(t => (
            <button
              key={t}
              className={`panel-tab${tab === t ? " active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        {loading && <div className="panel-loading">Loading…</div>}
        {error && (
          <div className="panel-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="dismiss">✕</button>
          </div>
        )}

        {!loading && tab === "live" && (
          <div ref={liveRef} style={{ overflowY: "auto" }}>
            {liveThoughts.length === 0
              ? <div className="panel-empty">Waiting for thoughts…</div>
              : liveThoughts.map(t => <ThoughtCard key={t.id} thought={t} />)}
          </div>
        )}

        {!loading && tab === "history" && (
          <>
            <div style={{ marginBottom: 12, display: "flex", alignItems: "center", gap: 10 }}>
              <select
                className="panel-select"
                value={filterCategory}
                onChange={e => setFilterCategory(e.target.value as typeof filterCategory)}
              >
                <option value="All">All Categories</option>
                {CATEGORIES.map(c => <option key={c} value={c}>{c}</option>)}
              </select>
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
                {filteredHistory.length} entries
              </span>
            </div>
            <div>
              {filteredHistory.length === 0
                ? <div className="panel-empty">No thoughts in this category.</div>
                : filteredHistory.map(t => <ThoughtCard key={t.id} thought={t} />)}
            </div>
          </>
        )}

        {!loading && tab === "export" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>Markdown export preview</span>
              <button
                className={`panel-btn panel-btn-sm ${copied ? "panel-btn-primary" : "panel-btn-secondary"}`}
                onClick={copyExport}
              >
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
            <pre style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", padding: 14, fontSize: "var(--font-size-sm)", lineHeight: 1.6, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0 }}>
              {exportMd || "No export data available."}
            </pre>
          </>
        )}
      </div>
    </div>
  );
}
