import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types matching Tauri command return shapes ─────────────────────── */

interface CostEntry {
  session_id: string;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  cost_usd: number;
  timestamp_ms: number;
  task_hint: string | null;
}

interface ProviderCostSummary {
  provider: string;
  total_cost_usd: number;
  total_tokens: number;
  call_count: number;
}

interface CostMetrics {
  entries: CostEntry[];
  by_provider: ProviderCostSummary[];
  total_cost_usd: number;
  total_tokens: number;
  budget_limit_usd: number | null;
  budget_remaining_usd: number | null;
}

interface TraceSessionInfo {
  session_id: string;
  timestamp: number;
  step_count: number;
}

/* ── Helpers ────────────────────────────────────────────────────────── */

/** Assumed developer hourly rate for ROI calculation */
const HOURLY_RATE_USD = 75;
/** Estimated minutes saved per completed AI task (conservative) */
const MINS_SAVED_PER_TASK = 8;

function monthRange(date: Date): { start: number; end: number } {
  const start = new Date(date.getFullYear(), date.getMonth(), 1).getTime();
  const end = new Date(date.getFullYear(), date.getMonth() + 1, 0, 23, 59, 59, 999).getTime();
  return { start, end };
}

function pctChange(current: number, previous: number): string {
  if (previous === 0) return current > 0 ? "+100%" : "0%";
  const pct = ((current - previous) / previous) * 100;
  const sign = pct >= 0 ? "+" : "";
  return `${sign}${pct.toFixed(0)}%`;
}

function fmtCost(n: number): string {
  return `$${n.toFixed(2)}`;
}

function fmtHours(mins: number): string {
  const hrs = mins / 60;
  return hrs < 1 ? `${mins.toFixed(0)} min` : `${hrs.toFixed(1)} hrs`;
}

/* ── Styles ─────────────────────────────────────────────────────────── */

const panelStyle: React.CSSProperties = {
  padding: 16, height: "100%", overflow: "auto",
  color: "var(--text-primary)", background: "var(--bg-primary)",
};
const headingStyle: React.CSSProperties = { fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 8, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const btnStyle: React.CSSProperties = {
  padding: "6px 14px", borderRadius: 6, border: "1px solid var(--border-color)",
  background: "var(--accent-color)", color: "#fff", cursor: "pointer", fontSize: 13, marginRight: 8,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px", cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent", border: "none", fontSize: 13, fontWeight: active ? 600 : 400,
});
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", fontSize: 12, color: "var(--text-secondary)", borderBottom: "1px solid var(--border-color)" };
const tdStyle: React.CSSProperties = { padding: "8px", fontSize: 13, borderBottom: "1px solid var(--border-color)" };

/* ── Component ──────────────────────────────────────────────────────── */

export function AnalyticsPanel() {
  const [tab, setTab] = useState("dashboard");
  const [exportFormat, setExportFormat] = useState("csv");
  const [dateFrom, setDateFrom] = useState(() => {
    const d = new Date(); d.setDate(1);
    return d.toISOString().slice(0, 10);
  });
  const [dateTo, setDateTo] = useState(() => new Date().toISOString().slice(0, 10));
  const [loading, setLoading] = useState(true);

  // Real data
  const [costMetrics, setCostMetrics] = useState<CostMetrics | null>(null);
  const [sessions, setSessions] = useState<TraceSessionInfo[]>([]);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [costs, traces] = await Promise.all([
        invoke<CostMetrics>("get_cost_metrics"),
        invoke<TraceSessionInfo[]>("list_trace_sessions"),
      ]);
      setCostMetrics(costs);
      setSessions(traces);
    } catch (e) {
      console.error("Analytics load error:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  // ── Compute metrics from real data ──────────────────────────────────

  const now = new Date();
  const thisMonth = monthRange(now);
  const lastMonth = monthRange(new Date(now.getFullYear(), now.getMonth() - 1, 1));

  // Tasks: count sessions by month
  const thisMonthSessions = sessions.filter(s => {
    const t = s.timestamp * 1000; // unix seconds → ms
    return t >= thisMonth.start && t <= thisMonth.end;
  });
  const lastMonthSessions = sessions.filter(s => {
    const t = s.timestamp * 1000;
    return t >= lastMonth.start && t <= lastMonth.end;
  });
  const tasksThisMonth = thisMonthSessions.length;
  const tasksLastMonth = lastMonthSessions.length;

  // Cost: filter entries by month
  const costThisMonth = costMetrics?.entries
    .filter(e => e.timestamp_ms >= thisMonth.start && e.timestamp_ms <= thisMonth.end)
    .reduce((sum, e) => sum + e.cost_usd, 0) ?? 0;
  const costLastMonth = costMetrics?.entries
    .filter(e => e.timestamp_ms >= lastMonth.start && e.timestamp_ms <= lastMonth.end)
    .reduce((sum, e) => sum + e.cost_usd, 0) ?? 0;

  // Time saved: estimated from task count
  const timeSavedMinsThisMonth = tasksThisMonth * MINS_SAVED_PER_TASK;
  const timeSavedMinsLastMonth = tasksLastMonth * MINS_SAVED_PER_TASK;

  // ROI: (time_saved_value - cost) / cost
  const timeSavedValueThisMonth = (timeSavedMinsThisMonth / 60) * HOURLY_RATE_USD;
  const roiThisMonth = costThisMonth > 0 ? timeSavedValueThisMonth / costThisMonth : 0;
  const timeSavedValueLastMonth = (timeSavedMinsLastMonth / 60) * HOURLY_RATE_USD;
  const roiLastMonth = costLastMonth > 0 ? timeSavedValueLastMonth / costLastMonth : 0;

  const metrics = [
    { label: "Tasks Completed", value: tasksThisMonth.toLocaleString(), change: pctChange(tasksThisMonth, tasksLastMonth), color: "#3b82f6" },
    { label: "Total Cost", value: fmtCost(costThisMonth), change: pctChange(costThisMonth, costLastMonth), color: "#22c55e" },
    { label: "Time Saved", value: fmtHours(timeSavedMinsThisMonth), change: pctChange(timeSavedMinsThisMonth, timeSavedMinsLastMonth), color: "#8b5cf6" },
    { label: "ROI", value: `${roiThisMonth.toFixed(1)}x`, change: roiLastMonth > 0 ? `${(roiThisMonth - roiLastMonth) >= 0 ? "+" : ""}${(roiThisMonth - roiLastMonth).toFixed(1)}x` : "—", color: "#f59e0b" },
  ];

  // ── Per-provider breakdown (for Users tab — repurposed as Provider breakdown) ──
  const providerRows = (costMetrics?.by_provider ?? []).map(p => {
    const providerEntries = costMetrics?.entries.filter(e => e.provider === p.provider) ?? [];
    const providerSessions = new Set(providerEntries.map(e => e.session_id));
    const taskCount = providerSessions.size;
    const timeSaved = taskCount * MINS_SAVED_PER_TASK;
    return {
      name: p.provider,
      tasks: taskCount,
      calls: p.call_count,
      cost: fmtCost(p.total_cost_usd),
      timeSaved: fmtHours(timeSaved),
      tokens: p.total_tokens.toLocaleString(),
    };
  });

  // ── Per-model breakdown (for Teams tab — repurposed as Model breakdown) ──
  const modelMap = new Map<string, { cost: number; calls: number; sessions: Set<string> }>();
  for (const e of costMetrics?.entries ?? []) {
    const key = `${e.provider}/${e.model}`;
    const cur = modelMap.get(key) ?? { cost: 0, calls: 0, sessions: new Set<string>() };
    cur.cost += e.cost_usd;
    cur.calls += 1;
    cur.sessions.add(e.session_id);
    modelMap.set(key, cur);
  }
  const modelRows = [...modelMap.entries()]
    .map(([name, d]) => {
      // Trend: compare this month vs last month calls
      const thisMonthCalls = (costMetrics?.entries ?? []).filter(e => `${e.provider}/${e.model}` === name && e.timestamp_ms >= thisMonth.start && e.timestamp_ms <= thisMonth.end).length;
      const lastMonthCalls = (costMetrics?.entries ?? []).filter(e => `${e.provider}/${e.model}` === name && e.timestamp_ms >= lastMonth.start && e.timestamp_ms <= lastMonth.end).length;
      const trend = thisMonthCalls > lastMonthCalls ? "up" : thisMonthCalls < lastMonthCalls ? "down" : "flat";
      return { name, tasks: d.sessions.size, calls: d.calls, cost: fmtCost(d.cost), timeSaved: fmtHours(d.sessions.size * MINS_SAVED_PER_TASK), trend };
    })
    .sort((a, b) => b.calls - a.calls);

  const trendIcon = (t: string) => t === "up" ? "\u2191" : t === "down" ? "\u2193" : "\u2192";
  const trendColor = (t: string) => t === "up" ? "#22c55e" : t === "down" ? "#ef4444" : "var(--text-secondary)";

  // ── Export ──────────────────────────────────────────────────────────

  const handleExport = () => {
    const fromMs = new Date(dateFrom).getTime();
    const toMs = new Date(dateTo + "T23:59:59").getTime();
    const filtered = (costMetrics?.entries ?? []).filter(e => e.timestamp_ms >= fromMs && e.timestamp_ms <= toMs);
    const filteredSessions = sessions.filter(s => s.timestamp * 1000 >= fromMs && s.timestamp * 1000 <= toMs);

    let content: string;
    const filename = `vibecody-analytics-${dateFrom}-to-${dateTo}`;

    if (exportFormat === "csv") {
      const lines = ["session_id,provider,model,prompt_tokens,completion_tokens,cost_usd,timestamp,task_hint"];
      for (const e of filtered) {
        lines.push(`${e.session_id},${e.provider},${e.model},${e.prompt_tokens},${e.completion_tokens},${e.cost_usd.toFixed(6)},${new Date(e.timestamp_ms).toISOString()},${e.task_hint ?? ""}`);
      }
      content = lines.join("\n");
      downloadFile(`${filename}.csv`, content, "text/csv");
    } else {
      const data = {
        period: { from: dateFrom, to: dateTo },
        cost_entries: filtered,
        sessions: filteredSessions,
        summary: {
          total_cost_usd: filtered.reduce((s, e) => s + e.cost_usd, 0),
          total_tasks: filteredSessions.length,
          total_tokens: filtered.reduce((s, e) => s + e.prompt_tokens + e.completion_tokens, 0),
        },
      };
      content = JSON.stringify(data, null, 2);
      downloadFile(`${filename}.json`, content, "application/json");
    }
  };

  const downloadFile = (name: string, content: string, mime: string) => {
    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = name; a.click();
    URL.revokeObjectURL(url);
  };

  if (loading) {
    return <div style={panelStyle}><h2 style={headingStyle}>Enterprise Agent Analytics</h2><div style={{ color: "var(--text-secondary)", padding: 20 }}>Loading analytics...</div></div>;
  }

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
        <h2 style={headingStyle}>Enterprise Agent Analytics</h2>
        <button onClick={loadData} style={{ ...btnStyle, background: "transparent", color: "var(--text-secondary)", fontSize: 11, padding: "4px 10px" }}>Refresh</button>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 12 }}>
        {sessions.length} total sessions | {(costMetrics?.entries.length ?? 0)} AI calls tracked | Est. {MINS_SAVED_PER_TASK} min saved/task @ ${HOURLY_RATE_USD}/hr
      </div>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["dashboard", "providers", "models", "export"].map((t) => (
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
              <div style={{ fontSize: 12, color: m.change.startsWith("+") ? "#22c55e" : m.change.startsWith("-") ? "#ef4444" : "var(--text-secondary)", marginTop: 4 }}>
                {m.change} vs last month
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "providers" && (
        <div style={{ overflowX: "auto" }}>
          {providerRows.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>No cost data recorded yet</div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr>
                  <th style={thStyle}>Provider</th>
                  <th style={thStyle}>Sessions</th>
                  <th style={thStyle}>API Calls</th>
                  <th style={thStyle}>Tokens</th>
                  <th style={thStyle}>Cost</th>
                  <th style={thStyle}>Est. Time Saved</th>
                </tr>
              </thead>
              <tbody>
                {providerRows.map((u) => (
                  <tr key={u.name}>
                    <td style={tdStyle}><span style={{ fontWeight: 600 }}>{u.name}</span></td>
                    <td style={tdStyle}>{u.tasks}</td>
                    <td style={tdStyle}>{u.calls}</td>
                    <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 12 }}>{u.tokens}</td>
                    <td style={tdStyle}>{u.cost}</td>
                    <td style={tdStyle}>{u.timeSaved}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {tab === "models" && (
        <div>
          {modelRows.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>No cost data recorded yet</div>
          ) : modelRows.map((t) => (
            <div key={t.name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 14 }}>{t.name}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>
                  {t.tasks} sessions | {t.calls} calls | {t.cost} | {t.timeSaved}
                </div>
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
          <button style={btnStyle} onClick={handleExport}>Export {exportFormat.toUpperCase()}</button>
        </div>
      )}
    </div>
  );
}
