import { useState } from "react";

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
  color: "#fff",
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

export function AnalyticsPanel() {
  const [tab, setTab] = useState("dashboard");
  const [exportFormat, setExportFormat] = useState("csv");
  const [dateFrom, setDateFrom] = useState("2026-03-01");
  const [dateTo, setDateTo] = useState("2026-03-26");

  const metrics = [
    { label: "Tasks Completed", value: "1,247", change: "+12%", color: "#3b82f6" },
    { label: "Total Cost", value: "$342.18", change: "-8%", color: "#22c55e" },
    { label: "Time Saved", value: "186 hrs", change: "+23%", color: "#8b5cf6" },
    { label: "ROI", value: "4.7x", change: "+0.3x", color: "#f59e0b" },
  ];

  const users = [
    { name: "Alice Chen", tasks: 312, acceptance: 89, cost: "$82.40", timeSaved: "48 hrs" },
    { name: "Bob Park", tasks: 287, acceptance: 84, cost: "$76.10", timeSaved: "41 hrs" },
    { name: "Carol Li", tasks: 256, acceptance: 91, cost: "$68.30", timeSaved: "39 hrs" },
    { name: "Dave Kim", tasks: 198, acceptance: 78, cost: "$58.20", timeSaved: "30 hrs" },
    { name: "Eve Zhao", tasks: 194, acceptance: 86, cost: "$57.18", timeSaved: "28 hrs" },
  ];

  const teams = [
    { name: "Backend", tasks: 543, trend: "up", cost: "$148.50", timeSaved: "82 hrs" },
    { name: "Frontend", tasks: 412, trend: "up", cost: "$108.20", timeSaved: "61 hrs" },
    { name: "DevOps", tasks: 178, trend: "flat", cost: "$52.30", timeSaved: "26 hrs" },
    { name: "QA", tasks: 114, trend: "down", cost: "$33.18", timeSaved: "17 hrs" },
  ];

  const trendIcon = (t: string) => t === "up" ? "^" : t === "down" ? "v" : "-";
  const trendColor = (t: string) => t === "up" ? "#22c55e" : t === "down" ? "#ef4444" : "var(--text-secondary)";

  const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", fontSize: 12, color: "var(--text-secondary)", borderBottom: "1px solid var(--border-color)" };
  const tdStyle: React.CSSProperties = { padding: "8px", fontSize: 13, borderBottom: "1px solid var(--border-color)" };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Enterprise Agent Analytics</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["dashboard", "users", "teams", "export"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "dashboard" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {metrics.map((m) => (
            <div key={m.label} style={cardStyle}>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{m.label}</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: m.color }}>{m.value}</div>
              <div style={{ fontSize: 12, color: m.change.startsWith("+") ? "#22c55e" : "#ef4444", marginTop: 4 }}>{m.change} vs last month</div>
            </div>
          ))}
        </div>
      )}

      {tab === "users" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={thStyle}>User</th>
                <th style={thStyle}>Tasks</th>
                <th style={thStyle}>Acceptance</th>
                <th style={thStyle}>Cost</th>
                <th style={thStyle}>Time Saved</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => (
                <tr key={u.name}>
                  <td style={tdStyle}>{u.name}</td>
                  <td style={tdStyle}>{u.tasks}</td>
                  <td style={tdStyle}>
                    <span style={{ color: u.acceptance > 85 ? "#22c55e" : "#eab308" }}>{u.acceptance}%</span>
                  </td>
                  <td style={tdStyle}>{u.cost}</td>
                  <td style={tdStyle}>{u.timeSaved}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {tab === "teams" && (
        <div>
          {teams.map((t) => (
            <div key={t.name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 14 }}>{t.name}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{t.tasks} tasks | {t.cost} | {t.timeSaved}</div>
              </div>
              <span style={{ fontWeight: 700, fontSize: 16, color: trendColor(t.trend) }}>{trendIcon(t.trend)}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "export" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Export Format</div>
            <div style={{ display: "flex", gap: 8 }}>
              {["csv", "json"].map((f) => (
                <button key={f} onClick={() => setExportFormat(f)}
                  style={{ ...btnStyle, background: exportFormat === f ? "var(--accent-color)" : "transparent", color: exportFormat === f ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)" }}>
                  {f.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Date Range</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)}
                style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }} />
              <span style={{ color: "var(--text-secondary)" }}>to</span>
              <input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)}
                style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }} />
            </div>
          </div>
          <button style={btnStyle}>Export {exportFormat.toUpperCase()}</button>
        </div>
      )}
    </div>
  );
}
