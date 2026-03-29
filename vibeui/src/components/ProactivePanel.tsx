import { useState, useCallback } from "react";

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
  const [suggestions, setSuggestions] = useState<Suggestion[]>([
    { id: "s1", title: "Extract duplicated validation logic", description: "Found 3 files with similar validation code", priority: "high", category: "refactor", status: "pending" },
    { id: "s2", title: "Add error boundary to Dashboard", description: "Unhandled promise rejection detected", priority: "medium", category: "reliability", status: "pending" },
    { id: "s3", title: "Update deprecated API call", description: "fetch v2 endpoint deprecated, migrate to v3", priority: "low", category: "maintenance", status: "pending" },
  ]);
  const [scans, setScans] = useState<ScanRecord[]>([
    { id: "sc1", triggeredAt: "2026-03-26 10:00", suggestionsFound: 3, duration: "2.1s" },
    { id: "sc2", triggeredAt: "2026-03-26 09:00", suggestionsFound: 1, duration: "1.8s" },
  ]);
  const [cadence, setCadence] = useState("hourly");
  const [minConfidence, setMinConfidence] = useState(70);
  const [quietMode, setQuietMode] = useState(false);

  const handleAction = useCallback((id: string, action: "accepted" | "rejected" | "snoozed") => {
    setSuggestions((prev) => prev.map((s) => s.id === id ? { ...s, status: action } : s));
  }, []);

  const handleScan = useCallback(() => {
    setScans((prev) => [{ id: `sc${Date.now()}`, triggeredAt: new Date().toISOString().slice(0, 16).replace("T", " "), suggestionsFound: 0, duration: "0.5s" }, ...prev]);
  }, []);

  const accepted = suggestions.filter((s) => s.status === "accepted").length;
  const rejected = suggestions.filter((s) => s.status === "rejected").length;
  const total = suggestions.length;
  const categories = [...new Set(suggestions.map((s) => s.category))];

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
                <button style={{ ...btnStyle, background: "var(--text-secondary)" }} onClick={() => handleAction(s.id, "snoozed")}>Snooze</button>
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
