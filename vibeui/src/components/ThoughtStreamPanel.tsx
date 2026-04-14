import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ThoughtEntry {
  id: string;
  timestamp: string;
  category: "Planning" | "Reasoning" | "Uncertainty" | "Decision" | "Observation";
  content: string;
  confidence: number | null;
}

const CATEGORY_COLORS: Record<string, string> = {
  Planning: "#4a9eff",
  Reasoning: "#8b8b9e",
  Uncertainty: "#f0a050",
  Decision: "#4caf7d",
  Observation: "#9c6fe0",
};

const CATEGORIES = ["Planning", "Reasoning", "Uncertainty", "Decision", "Observation"];

export function ThoughtStreamPanel() {
  const [tab, setTab] = useState("live");
  const [liveThoughts, setLiveThoughts] = useState<ThoughtEntry[]>([]);
  const [historyThoughts, setHistoryThoughts] = useState<ThoughtEntry[]>([]);
  const [exportMd, setExportMd] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterCategory, setFilterCategory] = useState("All");
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
    const color = CATEGORY_COLORS[thought.category] ?? "var(--text-muted)";
    return (
      <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: `1px solid ${color}44`, borderLeft: `3px solid ${color}`, padding: "10px 14px", marginBottom: 8 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
          <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: color + "22", color, fontWeight: 600 }}>{thought.category}</span>
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{thought.timestamp}</span>
          {thought.confidence !== null && (
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginLeft: "auto" }}>conf: {Math.round(thought.confidence * 100)}%</span>
          )}
        </div>
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>{thought.content}</div>
      </div>
    );
  };

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Thought Stream</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["live", "history", "export"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
        {tab === "live" && (
          <span style={{ marginLeft: "auto", fontSize: "var(--font-size-sm)", color: "var(--success-color)", display: "flex", alignItems: "center", gap: 4 }}>
            <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success-color)", display: "inline-block" }} />
            Live
          </span>
        )}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "live" && (
        <div ref={liveRef} style={{ flex: 1, overflowY: "auto" }}>
          {liveThoughts.length === 0 && <div style={{ color: "var(--text-muted)" }}>Waiting for thoughts…</div>}
          {liveThoughts.map(t => <ThoughtCard key={t.id} thought={t} />)}
        </div>
      )}

      {!loading && tab === "history" && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
          <div style={{ marginBottom: 12 }}>
            <select value={filterCategory} onChange={e => setFilterCategory(e.target.value)}
              style={{ padding: "5px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
              <option value="All">All Categories</option>
              {CATEGORIES.map(c => <option key={c} value={c}>{c}</option>)}
            </select>
            <span style={{ marginLeft: 10, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>{filteredHistory.length} entries</span>
          </div>
          <div style={{ flex: 1, overflowY: "auto" }}>
            {filteredHistory.length === 0 && <div style={{ color: "var(--text-muted)" }}>No thoughts in this category.</div>}
            {filteredHistory.map(t => <ThoughtCard key={t.id} thought={t} />)}
          </div>
        </div>
      )}

      {!loading && tab === "export" && (
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>Markdown export preview</span>
            <button onClick={copyExport}
              style={{ padding: "4px 14px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: copied ? "var(--success-color)" : "var(--bg-secondary)", color: copied ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
          <pre style={{ flex: 1, overflowY: "auto", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 14, fontSize: "var(--font-size-sm)", lineHeight: 1.6, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0 }}>
            {exportMd || "No export data available."}
          </pre>
        </div>
      )}
    </div>
  );
}
