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

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
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

  if (loading) return <div style={panelStyle}><div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading proactive intelligence...</div></div>;
  if (error) return <div style={panelStyle}><div style={{ color: "var(--error-color)", fontSize: 13 }}>Error: {error}</div></div>;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Proactive Agent Intelligence</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "suggestions")} onClick={() => setTab("suggestions")}>Suggestions</button>
        <button style={tabStyle(tab === "scan")} onClick={() => setTab("scan")}>Scan</button>
        <button style={tabStyle(tab === "learning")} onClick={() => setTab("learning")}>Learning</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "suggestions" && (
        <div>
          {suggestions.filter((s) => s.status === "pending").map((s) => (
            <div key={s.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{s.title}</strong>
                <span style={badgeStyle(priorityColor[s.priority])}>{s.priority}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{s.description}</div>
              <div>
                <button style={btnStyle} onClick={() => handleAction(s.id, "accepted")}>Accept</button>
                <button style={{ ...btnStyle, background: "var(--error-color)" }} onClick={() => handleAction(s.id, "rejected")}>Reject</button>
              </div>
            </div>
          ))}
          {suggestions.filter((s) => s.status === "pending").length === 0 && (
            <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No pending suggestions</div>
          )}
        </div>
      )}

      {tab === "scan" && (
        <div>
          <button style={btnStyle} onClick={handleScan}>Trigger Scan</button>
          <table style={{ width: "100%", fontSize: 13, marginTop: 12, borderCollapse: "collapse" }}>
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
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Acceptance Rate</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{total > 0 ? ((accepted / total) * 100).toFixed(0) : 0}%</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{accepted} accepted / {rejected} rejected / {total} total</div>
          </div>
          {digest && (
            <div style={cardStyle}>
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Digest</div>
              <pre style={{ fontSize: 12, color: "var(--text-secondary)", whiteSpace: "pre-wrap" }}>{JSON.stringify(digest, null, 2)}</pre>
            </div>
          )}
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Top Patterns by Category</div>
            {categories.map((c) => {
              const count = suggestions.filter((s) => s.category === c).length;
              return <div key={c} style={{ display: "flex", justifyContent: "space-between", padding: "4px 0", fontSize: 13 }}><span>{c}</span><strong>{count}</strong></div>;
            })}
          </div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Scan Cadence</div>
            <select value={cadence} onChange={(e) => setCadence(e.target.value)} style={{ padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
              <option value="realtime">Real-time</option>
              <option value="hourly">Hourly</option>
              <option value="daily">Daily</option>
              <option value="manual">Manual only</option>
            </select>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Min Confidence: {minConfidence}%</div>
            <input type="range" min={0} max={100} value={minConfidence} onChange={(e) => setMinConfidence(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
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
