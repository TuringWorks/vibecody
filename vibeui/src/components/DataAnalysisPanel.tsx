/**
 * DataAnalysisPanel — Data analysis with datasets, charts, dashboards, and natural language queries.
 *
 * Tabs: Datasets, Charts, Dashboard, Query
 * Wired to Tauri backend commands: da_list_datasets, da_add_dataset, da_remove_dataset,
 * da_list_charts, da_add_chart, da_list_widgets, da_add_widget, da_execute_query, da_list_queries.
 */
import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Datasets" | "Charts" | "Dashboard" | "Query";
const TABS: Tab[] = ["Datasets", "Charts", "Dashboard", "Query"];

const STATUS_COLORS: Record<string, string> = {
  Loaded: "var(--success-color)", Loading: "var(--info-color)",
  Error: "var(--error-color)", Stale: "var(--warning-color)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const inputStyle: React.CSSProperties = {
  width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit",
  boxSizing: "border-box",
};
const formRowStyle: React.CSSProperties = {
  display: "flex", gap: 8, marginBottom: 12, alignItems: "flex-end", flexWrap: "wrap",
};
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 2 };

interface Dataset {
  id: string;
  name: string;
  source: string;
  rows: number;
  cols: number;
  size: string;
  status: string;
  created_at: string;
}

interface Chart {
  id: string;
  title: string;
  type: string;
  dataset: string;
  created: string;
}

interface Widget {
  id: string;
  title: string;
  type: string;
  value?: string;
}

interface QueryEntry {
  id: string;
  query: string;
  result: string;
  rows_scanned: number;
  datasets_matched: number;
  executed_at: string;
}

const DataAnalysisPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Datasets");
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [charts, setCharts] = useState<Chart[]>([]);
  const [widgets, setWidgets] = useState<Widget[]>([]);
  const [queries, setQueries] = useState<QueryEntry[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Add-dataset form state
  const [dsName, setDsName] = useState("");
  const [dsSource, setDsSource] = useState("CSV");
  const [dsRows, setDsRows] = useState("");
  const [dsCols, setDsCols] = useState("");
  const [dsSize, setDsSize] = useState("");

  // Add-chart form state
  const [chartTitle, setChartTitle] = useState("");
  const [chartType, setChartType] = useState("Line Chart");
  const [chartDataset, setChartDataset] = useState("");

  // Add-widget form state
  const [widgetTitle, setWidgetTitle] = useState("");
  const [widgetType, setWidgetType] = useState("Metric");
  const [widgetValue, setWidgetValue] = useState("");

  const fetchDatasets = useCallback(async () => {
    try {
      const result = await invoke<Dataset[]>("da_list_datasets");
      setDatasets(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const fetchCharts = useCallback(async () => {
    try {
      const result = await invoke<Chart[]>("da_list_charts");
      setCharts(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const fetchWidgets = useCallback(async () => {
    try {
      const result = await invoke<Widget[]>("da_list_widgets");
      setWidgets(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const fetchQueries = useCallback(async () => {
    try {
      const result = await invoke<QueryEntry[]>("da_list_queries");
      setQueries(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    fetchDatasets();
    fetchCharts();
    fetchWidgets();
    fetchQueries();
  }, [fetchDatasets, fetchCharts, fetchWidgets, fetchQueries]);

  const handleAddDataset = async () => {
    if (!dsName.trim()) return;
    setError(null);
    try {
      await invoke("da_add_dataset", {
        name: dsName.trim(),
        source: dsSource,
        rows: parseInt(dsRows) || 0,
        cols: parseInt(dsCols) || 0,
        size: dsSize.trim() || "0 B",
      });
      setDsName(""); setDsRows(""); setDsCols(""); setDsSize("");
      await fetchDatasets();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemoveDataset = async (id: string) => {
    setError(null);
    try {
      await invoke("da_remove_dataset", { id });
      await fetchDatasets();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleAddChart = async () => {
    if (!chartTitle.trim()) return;
    setError(null);
    try {
      await invoke("da_add_chart", {
        title: chartTitle.trim(),
        chartType,
        dataset: chartDataset.trim(),
      });
      setChartTitle(""); setChartDataset("");
      await fetchCharts();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleAddWidget = async () => {
    if (!widgetTitle.trim()) return;
    setError(null);
    try {
      await invoke("da_add_widget", {
        title: widgetTitle.trim(),
        widgetType,
        value: widgetValue.trim() || null,
      });
      setWidgetTitle(""); setWidgetValue("");
      await fetchWidgets();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRunQuery = async () => {
    if (!query.trim()) return;
    setError(null);
    setLoading(true);
    try {
      await invoke("da_execute_query", { query: query.trim() });
      setQuery("");
      await fetchQueries();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="panel-container" role="region" aria-label="Data Analysis Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="Data Analysis tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      {error && (
        <div className="panel-error" style={{ padding: "8px 16px", fontSize: 12 }}>
          {error}
          <button style={{ marginLeft: 12, background: "transparent", border: "none", color: "inherit", cursor: "pointer", fontSize: 12 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Datasets" && (
          <div>
            <div className="panel-card" style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Add Dataset</div>
              <div style={formRowStyle}>
                <div style={{ flex: 2 }}>
                  <div style={labelStyle}>Name</div>
                  <input style={inputStyle} placeholder="e.g. sales_2026.csv" value={dsName} onChange={e => setDsName(e.target.value)} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Source</div>
                  <select style={inputStyle} value={dsSource} onChange={e => setDsSource(e.target.value)}>
                    <option>CSV</option><option>JSON</option><option>PostgreSQL</option><option>MySQL</option><option>SQLite</option><option>API</option>
                  </select>
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Rows</div>
                  <input style={inputStyle} type="number" placeholder="0" value={dsRows} onChange={e => setDsRows(e.target.value)} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Cols</div>
                  <input style={inputStyle} type="number" placeholder="0" value={dsCols} onChange={e => setDsCols(e.target.value)} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Size</div>
                  <input style={inputStyle} placeholder="e.g. 2.4 MB" value={dsSize} onChange={e => setDsSize(e.target.value)} />
                </div>
                <button className="panel-btn panel-btn-primary" onClick={handleAddDataset}>Add</button>
              </div>
            </div>
            {datasets.length === 0 && <div className="panel-empty">No datasets loaded. Add one above.</div>}
            {datasets.map((d) => (
              <div key={d.id} className="panel-card">
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                  <strong>{d.name}</strong>
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <span style={badgeStyle(STATUS_COLORS[d.status] || "var(--text-secondary)")}>{d.status}</span>
                    <button className="panel-btn panel-btn-danger" onClick={() => handleRemoveDataset(d.id)} title="Remove dataset">X</button>
                  </div>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                  {d.rows.toLocaleString()} rows x {d.cols} cols &middot; {d.size} &middot; Source: {d.source}
                </div>
              </div>
            ))}
          </div>
        )}
        {tab === "Charts" && (
          <div>
            <div className="panel-card" style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Add Chart</div>
              <div style={formRowStyle}>
                <div style={{ flex: 2 }}>
                  <div style={labelStyle}>Title</div>
                  <input style={inputStyle} placeholder="e.g. Monthly Revenue" value={chartTitle} onChange={e => setChartTitle(e.target.value)} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Type</div>
                  <select style={inputStyle} value={chartType} onChange={e => setChartType(e.target.value)}>
                    <option>Line Chart</option><option>Bar Chart</option><option>Pie Chart</option><option>Histogram</option><option>Scatter Plot</option><option>Area Chart</option>
                  </select>
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Dataset</div>
                  <input style={inputStyle} placeholder="dataset name" value={chartDataset} onChange={e => setChartDataset(e.target.value)} />
                </div>
                <button className="panel-btn panel-btn-primary" onClick={handleAddChart}>Add</button>
              </div>
            </div>
            {charts.length === 0 && <div className="panel-empty">No charts created yet. Add one above.</div>}
            {charts.map((c) => (
              <div key={c.id} className="panel-card">
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
          </div>
        )}
        {tab === "Dashboard" && (
          <div>
            <div className="panel-card" style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Add Widget</div>
              <div style={formRowStyle}>
                <div style={{ flex: 2 }}>
                  <div style={labelStyle}>Title</div>
                  <input style={inputStyle} placeholder="e.g. Revenue KPI" value={widgetTitle} onChange={e => setWidgetTitle(e.target.value)} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Type</div>
                  <select style={inputStyle} value={widgetType} onChange={e => setWidgetType(e.target.value)}>
                    <option>Metric</option><option>Chart</option><option>Table</option><option>Counter</option>
                  </select>
                </div>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Value (optional)</div>
                  <input style={inputStyle} placeholder="e.g. $1.2M" value={widgetValue} onChange={e => setWidgetValue(e.target.value)} />
                </div>
                <button className="panel-btn panel-btn-primary" onClick={handleAddWidget}>Add</button>
              </div>
            </div>
            {widgets.length === 0 && <div className="panel-empty">No dashboard widgets. Add one above.</div>}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
              {widgets.map((w) => (
                <div key={w.id} className="panel-card">
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>{w.type}</div>
                  <strong>{w.title}</strong>
                  {w.value && <div style={{ fontSize: 20, fontWeight: 700, marginTop: 4, color: "var(--accent-color)" }}>{w.value}</div>}
                  {w.type === "Chart" && <div style={{ height: 30, background: "var(--bg-tertiary)", borderRadius: 4, marginTop: 8, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 10, color: "var(--text-secondary)" }}>[chart]</div>}
                </div>
              ))}
            </div>
          </div>
        )}
        {tab === "Query" && (
          <div>
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <input
                style={{ ...inputStyle, flex: 1 }}
                placeholder="Ask a question about your data..."
                value={query}
                onChange={e => setQuery(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") handleRunQuery(); }}
                aria-label="Natural language query"
              />
              <button className="panel-btn panel-btn-primary" onClick={handleRunQuery} disabled={loading} aria-label="Run query">
                {loading ? "Running..." : "Ask"}
              </button>
            </div>
            {queries.length === 0 && <div className="panel-empty">No queries yet. Ask a question above.</div>}
            {queries.length > 0 && <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12 }}>Recent queries:</div>}
            {queries.map((q) => (
              <div key={q.id} className="panel-card">
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>{q.query}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{q.result}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
                  {q.rows_scanned.toLocaleString()} rows scanned &middot; {q.datasets_matched} dataset(s)
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default DataAnalysisPanel;
