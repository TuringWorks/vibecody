/**
 * DataAnalysisPanel — Data analysis with datasets, charts, dashboards, and natural language queries.
 *
 * Tabs: Datasets, Charts, Dashboard, Query
 */
import React, { useState } from "react";

type Tab = "Datasets" | "Charts" | "Dashboard" | "Query";
const TABS: Tab[] = ["Datasets", "Charts", "Dashboard", "Query"];

const STATUS_COLORS: Record<string, string> = {
  Loaded: "var(--success-color)", Loading: "var(--info-color)",
  Error: "var(--error-color)", Stale: "var(--warning-color)",
};

const containerStyle: React.CSSProperties = {
  display: "flex", flexDirection: "column", height: "100%",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  fontFamily: "inherit", overflow: "hidden",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex", gap: 2, padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)",
  overflowX: "auto", flexShrink: 0,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const btnStyle: React.CSSProperties = {
  padding: "6px 14px", background: "var(--accent-color)", color: "var(--bg-primary)",
  border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontFamily: "inherit",
};
const inputStyle: React.CSSProperties = {
  width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit",
  boxSizing: "border-box",
};

const DATASETS = [
  { name: "sales_2026.csv", rows: 14520, cols: 12, status: "Loaded", size: "2.4 MB", source: "CSV" },
  { name: "user_events", rows: 89400, cols: 8, status: "Loaded", size: "18 MB", source: "PostgreSQL" },
  { name: "api_metrics.json", rows: 5200, cols: 15, status: "Loading", size: "1.1 MB", source: "JSON" },
  { name: "inventory_snapshot", rows: 3400, cols: 22, status: "Stale", size: "4.8 MB", source: "MySQL" },
];
const CHARTS = [
  { title: "Monthly Revenue", type: "Line Chart", dataset: "sales_2026.csv", created: "2026-03-19" },
  { title: "User Signups by Region", type: "Bar Chart", dataset: "user_events", created: "2026-03-18" },
  { title: "API Latency Distribution", type: "Histogram", dataset: "api_metrics.json", created: "2026-03-17" },
  { title: "Top Products by Sales", type: "Pie Chart", dataset: "sales_2026.csv", created: "2026-03-16" },
];
const WIDGETS = [
  { title: "Revenue KPI", type: "Metric", value: "$1.2M" },
  { title: "Active Users", type: "Metric", value: "8,420" },
  { title: "P95 Latency", type: "Metric", value: "142ms" },
  { title: "Monthly Revenue", type: "Chart" },
  { title: "User Signups by Region", type: "Chart" },
];
const QUERIES = [
  { query: "Show monthly revenue trend for 2026", result: "Line chart generated with 3 data points" },
  { query: "What are the top 5 products by sales volume?", result: "Bar chart with product names and volumes" },
  { query: "Average API latency by endpoint", result: "Table with 12 endpoints and avg latency" },
];

const DataAnalysisPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Datasets");
  const [query, setQuery] = useState("");

  return (
    <div style={containerStyle} role="region" aria-label="Data Analysis Panel">
      <div style={tabBarStyle} role="tablist" aria-label="Data Analysis tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Datasets" && DATASETS.map((d, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{d.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[d.status] || "var(--text-secondary)")}>{d.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {d.rows.toLocaleString()} rows x {d.cols} cols &middot; {d.size} &middot; Source: {d.source}
            </div>
          </div>
        ))}
        {tab === "Charts" && CHARTS.map((c, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{c.title}</strong>
              <span style={badgeStyle("var(--info-color)")}>{c.type}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Dataset: {c.dataset} &middot; {c.created}</div>
            <div style={{ height: 40, background: "var(--bg-tertiary)", borderRadius: 4, marginTop: 8, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 11, color: "var(--text-secondary)" }}>
              [Chart visualization]
            </div>
          </div>
        ))}
        {tab === "Dashboard" && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
            {WIDGETS.map((w, i) => (
              <div key={i} style={cardStyle}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>{w.type}</div>
                <strong>{w.title}</strong>
                {w.value && <div style={{ fontSize: 20, fontWeight: 700, marginTop: 4, color: "var(--accent-color)" }}>{w.value}</div>}
                {w.type === "Chart" && <div style={{ height: 30, background: "var(--bg-tertiary)", borderRadius: 4, marginTop: 8, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 10, color: "var(--text-secondary)" }}>[chart]</div>}
              </div>
            ))}
          </div>
        )}
        {tab === "Query" && (
          <div>
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <input style={{ ...inputStyle, flex: 1 }} placeholder="Ask a question about your data..." value={query} onChange={e => setQuery(e.target.value)} aria-label="Natural language query" />
              <button style={btnStyle} aria-label="Run query">Ask</button>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12 }}>Recent queries:</div>
            {QUERIES.map((q, i) => (
              <div key={i} style={cardStyle}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>{q.query}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{q.result}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default DataAnalysisPanel;
