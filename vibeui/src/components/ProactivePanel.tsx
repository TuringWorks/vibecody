/* eslint-disable @typescript-eslint/no-explicit-any */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Suggestion {
  id: string;
  title: string;
  description: string;
  priority: "high" | "medium" | "low";
  category: string;
  status: "pending" | "accepted" | "rejected" | "snoozed";
}

interface ScanRecord {
  id: string;
  triggeredAt: string;
  suggestionsFound: number;
  duration: string;
}

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const priorityColor: Record<string, string> = { high: "var(--error-color)", medium: "var(--warning-color)", low: "var(--success-color)" };

export function ProactivePanel() {
  const [tab, setTab] = useState("suggestions");
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [scans, setScans] = useState<ScanRecord[]>([]);
  const [digest, setDigest] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [cadence, setCadence] = useState("hourly");
  const [minConfidence, setMinConfidence] = useState(70);
  const [quietMode, setQuietMode] = useState(false);

  const fetchSuggestions = useCallback(async () => {
    try {
      const data = await invoke<{ suggestions: Suggestion[] }>("proactive_get_suggestions");
      const list = (data as any)?.suggestions ?? (Array.isArray(data) ? data : []);
      setSuggestions(list);
    } catch (e) {
      console.error("proactive_get_suggestions failed:", e);
    }
  }, []);

  const fetchDigest = useCallback(async () => {
    try {
      const data = await invoke<Record<string, unknown>>("proactive_get_digest");
      setDigest(data);
    } catch (e) {
      console.error("proactive_get_digest failed:", e);
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setError(null);
    Promise.all([fetchSuggestions(), fetchDigest()])
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [fetchSuggestions, fetchDigest]);

  const handleAction = useCallback(async (id: string, action: "accepted" | "rejected") => {
    try {
      const cmd = action === "accepted" ? "proactive_accept" : "proactive_reject";
      await invoke(cmd, { suggestionId: id });
      setSuggestions((prev) => prev.map((s) => s.id === id ? { ...s, status: action } : s));
    } catch (e) {
      console.error(`proactive_${action} failed:`, e);
    }
  }, []);

  const handleScan = useCallback(async () => {
    try {
      const result = await invoke<Record<string, unknown>>("proactive_scan");
      const newScan: ScanRecord = {
        id: `sc${Date.now()}`,
        triggeredAt: new Date().toISOString().slice(0, 16).replace("T", " "),
        suggestionsFound: (result as any)?.new_suggestions ?? 0,
        duration: (result as any)?.duration ?? "0.5s",
      };
      setScans((prev) => [newScan, ...prev]);
      await fetchSuggestions();
    } catch (e) {
      console.error("proactive_scan failed:", e);
    }
  }, [fetchSuggestions]);

  const accepted = suggestions.filter((s) => s.status === "accepted").length;
  const rejected = suggestions.filter((s) => s.status === "rejected").length;
  const total = suggestions.length;
  const categories = [...new Set(suggestions.map((s) => s.category))];

  if (loading) return <div className="panel-container"><div className="panel-loading">Loading proactive intelligence...</div></div>;
  if (error) return <div className="panel-container"><div className="panel-error">Error: {error}</div></div>;

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Proactive Agent Intelligence</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        <button className={`panel-tab ${tab === "suggestions" ? "active" : ""}`} onClick={() => setTab("suggestions")}>Suggestions</button>
        <button className={`panel-tab ${tab === "scan" ? "active" : ""}`} onClick={() => setTab("scan")}>Scan</button>
        <button className={`panel-tab ${tab === "learning" ? "active" : ""}`} onClick={() => setTab("learning")}>Learning</button>
        <button className={`panel-tab ${tab === "config" ? "active" : ""}`} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "suggestions" && (
        <div>
          {suggestions.filter((s) => s.status === "pending").map((s) => (
            <div key={s.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{s.title}</strong>
                <span style={badgeStyle(priorityColor[s.priority])}>{s.priority}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>{s.description}</div>
              <div>
                <button className="panel-btn panel-btn-primary" onClick={() => handleAction(s.id, "accepted")}>Accept</button>
                <button className="panel-btn panel-btn-danger" onClick={() => handleAction(s.id, "rejected")}>Reject</button>
              </div>
            </div>
          ))}
          {suggestions.filter((s) => s.status === "pending").length === 0 && (
            <div className="panel-empty">No pending suggestions</div>
          )}
        </div>
      )}

      {tab === "scan" && (
        <div>
          <button className="panel-btn panel-btn-primary" onClick={handleScan}>Trigger Scan</button>
          <table style={{ width: "100%", fontSize: "var(--font-size-md)", marginTop: 12, borderCollapse: "collapse" }}>
            <thead><tr style={{ borderBottom: "1px solid var(--border-color)" }}>
              <th style={{ textAlign: "left", padding: 8 }}>Time</th>
              <th style={{ textAlign: "left", padding: 8 }}>Found</th>
              <th style={{ textAlign: "left", padding: 8 }}>Duration</th>
            </tr></thead>
            <tbody>{scans.map((sc) => (
              <tr key={sc.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                <td style={{ padding: 8 }}>{sc.triggeredAt}</td>
                <td style={{ padding: 8 }}>{sc.suggestionsFound}</td>
                <td style={{ padding: 8 }}>{sc.duration}</td>
              </tr>
            ))}</tbody>
          </table>
        </div>
      )}

      {tab === "learning" && (
        <div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Acceptance Rate</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{total > 0 ? ((accepted / total) * 100).toFixed(0) : 0}%</div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{accepted} accepted / {rejected} rejected / {total} total</div>
          </div>
          {digest && (
            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Digest</div>
              <pre style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", whiteSpace: "pre-wrap" }}>{JSON.stringify(digest, null, 2)}</pre>
            </div>
          )}
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Top Patterns by Category</div>
            {categories.map((c) => {
              const count = suggestions.filter((s) => s.category === c).length;
              return <div key={c} style={{ display: "flex", justifyContent: "space-between", padding: "4px 0", fontSize: "var(--font-size-md)" }}><span>{c}</span><strong>{count}</strong></div>;
            })}
          </div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Scan Cadence</div>
            <select value={cadence} onChange={(e) => setCadence(e.target.value)} style={{ padding: 8, borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: "var(--font-size-md)" }}>
              <option value="realtime">Real-time</option>
              <option value="hourly">Hourly</option>
              <option value="daily">Daily</option>
              <option value="manual">Manual only</option>
            </select>
          </div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Min Confidence: {minConfidence}%</div>
            <input type="range" min={0} max={100} value={minConfidence} onChange={(e) => setMinConfidence(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div className="panel-card">
            <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
              <input type="checkbox" checked={quietMode} onChange={(e) => setQuietMode(e.target.checked)} />
              <span style={{ fontWeight: 600 }}>Quiet Mode (suppress notifications)</span>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}
