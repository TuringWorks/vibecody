/**
 * UsageMeteringPanel — Usage Metering panel.
 *
 * Dashboard for tracking AI usage: spend, tokens, requests, budgets,
 * provider/model breakdowns, and configurable alerts.
 * Wired to Tauri backend commands persisted at ~/.vibeui/usage-metering.json.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface Budget {
  id: string;
  name: string;
  limit: number;
  used: number;
  period: "daily" | "weekly" | "monthly";
}

interface UsageRow {
  label: string;
  tokens: number;
  requests: number;
  cost: number;
}

interface UsageAlert {
  id: string;
  severity: "info" | "warning" | "critical";
  message: string;
  timestamp: string;
  dismissed: boolean;
}

interface KpiData {
  totalSpend: number;
  tokensUsed: number;
  requests: number;
  activeBudgets: number;
  alertsTriggered: number;
}

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg)" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-family)", boxSizing: "border-box" };
const selectStyle: React.CSSProperties = { ...inputStyle, width: "auto", cursor: "pointer" };

const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 12 };

const severityColor: Record<string, string> = { info: "var(--info-color)", warning: "var(--warning-color)", critical: "var(--error-color)" };
const badgeStyle = (severity: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--btn-primary-fg)", background: severityColor[severity] || "var(--text-secondary)" });

const budgetBarColor = (pct: number) => pct >= 90 ? "var(--error-color)" : pct >= 70 ? "var(--warning-color)" : "var(--success-color)";
const formatTokens = (n: number) => n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1_000 ? `${(n / 1_000).toFixed(0)}k` : String(n);

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "dashboard" | "budgets" | "reports" | "alerts";
type ReportView = "provider" | "model" | "task";

export function UsageMeteringPanel() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [kpis, setKpis] = useState<KpiData>({ totalSpend: 0, tokensUsed: 0, requests: 0, activeBudgets: 0, alertsTriggered: 0 });
  const [budgets, setBudgets] = useState<Budget[]>([]);
  const [alerts, setAlerts] = useState<UsageAlert[]>([]);
  const [byProvider, setByProvider] = useState<UsageRow[]>([]);
  const [byModel, setByModel] = useState<UsageRow[]>([]);
  const [reportView, setReportView] = useState<ReportView>("provider");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Create budget form
  const [newBudgetName, setNewBudgetName] = useState("");
  const [newBudgetLimit, setNewBudgetLimit] = useState("");
  const [newBudgetPeriod, setNewBudgetPeriod] = useState<"daily" | "weekly" | "monthly">("monthly");

  const loadData = useCallback(async () => {
    try {
      setError(null);
      const [kpiResult, budgetResult, providerResult, modelResult, alertResult] = await Promise.all([
        invoke<KpiData>("get_usage_kpis"),
        invoke<Budget[]>("get_usage_budgets"),
        invoke<UsageRow[]>("get_usage_by_provider"),
        invoke<UsageRow[]>("get_usage_by_model"),
        invoke<UsageAlert[]>("get_usage_alerts"),
      ]);
      setKpis(kpiResult);
      setBudgets(budgetResult);
      setByProvider(providerResult);
      setByModel(modelResult);
      setAlerts(alertResult);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const createBudget = async () => {
    if (!newBudgetName.trim() || !newBudgetLimit) return;
    try {
      await invoke("create_usage_budget", {
        name: newBudgetName.trim(),
        limit: parseFloat(newBudgetLimit),
        period: newBudgetPeriod,
      });
      setNewBudgetName("");
      setNewBudgetLimit("");
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const deleteBudget = (id: string) => {
    setBudgets((prev) => prev.filter((b) => b.id !== id));
  };

  const dismissAlert = async (id: string) => {
    try {
      await invoke("dismiss_usage_alert", { id });
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const reportData: Record<ReportView, UsageRow[]> = { provider: byProvider, model: byModel, task: [] };

  const renderTable = (rows: UsageRow[]) => (
    <table style={{ width: "100%", borderCollapse: "collapse" }}>
      <thead>
        <tr>
          <th style={thStyle}>Label</th>
          <th style={{ ...thStyle, textAlign: "right" }}>Tokens</th>
          <th style={{ ...thStyle, textAlign: "right" }}>Requests</th>
          <th style={{ ...thStyle, textAlign: "right" }}>Cost</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <td style={tdStyle}>{r.label}</td>
            <td style={{ ...tdStyle, textAlign: "right" }}>{formatTokens(r.tokens)}</td>
            <td style={{ ...tdStyle, textAlign: "right" }}>{r.requests.toLocaleString()}</td>
            <td style={{ ...tdStyle, textAlign: "right" }}>${r.cost.toFixed(2)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );

  if (loading) {
    return <div style={panelStyle}><h2 style={headingStyle}>Usage Metering</h2><div style={cardStyle}>Loading...</div></div>;
  }

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Usage Metering</h2>

      {error && (
        <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", marginBottom: 12 }}>
          {error}
        </div>
      )}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "dashboard")} onClick={() => setTab("dashboard")}>Dashboard</button>
        <button style={tabBtnStyle(tab === "budgets")} onClick={() => setTab("budgets")}>Budgets</button>
        <button style={tabBtnStyle(tab === "reports")} onClick={() => setTab("reports")}>Reports</button>
        <button style={tabBtnStyle(tab === "alerts")} onClick={() => setTab("alerts")}>Alerts ({alerts.filter((a) => !a.dismissed).length})</button>
      </div>

      {tab === "dashboard" && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Total Spend</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--accent-primary)" }}>${kpis.totalSpend.toFixed(2)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Tokens Used</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{formatTokens(kpis.tokensUsed)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Requests</div>
              <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{kpis.requests.toLocaleString()}</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Spend by Provider</div>
            {byProvider.length === 0 && <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>No provider data yet.</div>}
            {byProvider.map((p) => {
              const pct = kpis.totalSpend > 0 ? (p.cost / kpis.totalSpend) * 100 : 0;
              return (
                <div key={p.label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <div style={{ width: 140, fontSize: 11 }}>{p.label}</div>
                  <div style={{ ...barBg, flex: 1 }}>
                    <div style={barFill(pct, "var(--accent-primary)")} />
                  </div>
                  <div style={{ width: 60, fontSize: 11, textAlign: "right" }}>${p.cost.toFixed(2)}</div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {tab === "budgets" && (
        <div>
          {budgets.length === 0 && <div style={cardStyle}>No budgets configured. Create one below.</div>}
          {budgets.map((b) => {
            const pct = b.limit > 0 ? (b.used / b.limit) * 100 : 0;
            return (
              <div key={b.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <div>
                    <span style={{ fontWeight: 600 }}>{b.name}</span>
                    <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: 6 }}>{b.period}</span>
                  </div>
                  <button style={{ ...btnStyle, fontSize: 10, padding: "3px 8px" }} onClick={() => deleteBudget(b.id)}>Remove</button>
                </div>
                <div style={barBg}>
                  <div style={barFill(pct, budgetBarColor(pct))} />
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
                  <span>${b.used.toFixed(2)} used</span>
                  <span>${b.limit.toFixed(2)} limit ({pct.toFixed(0)}%)</span>
                </div>
              </div>
            );
          })}

          <div style={cardStyle}>
            <div style={{ ...labelStyle, fontWeight: 600, fontSize: 12 }}>Create Budget</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginTop: 8 }}>
              <div>
                <div style={labelStyle}>Name</div>
                <input style={inputStyle} value={newBudgetName} onChange={(e) => setNewBudgetName(e.target.value)} placeholder="Budget name" />
              </div>
              <div>
                <div style={labelStyle}>Limit ($)</div>
                <input style={inputStyle} type="number" value={newBudgetLimit} onChange={(e) => setNewBudgetLimit(e.target.value)} placeholder="100.00" />
              </div>
              <div>
                <div style={labelStyle}>Period</div>
                <select style={selectStyle} value={newBudgetPeriod} onChange={(e) => setNewBudgetPeriod(e.target.value as "daily" | "weekly" | "monthly")}>
                  <option value="daily">Daily</option>
                  <option value="weekly">Weekly</option>
                  <option value="monthly">Monthly</option>
                </select>
              </div>
            </div>
            <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--btn-primary-fg)", marginTop: 8 }} onClick={createBudget}>Create</button>
          </div>
        </div>
      )}

      {tab === "reports" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <button style={tabBtnStyle(reportView === "provider")} onClick={() => setReportView("provider")}>By Provider</button>
            <button style={tabBtnStyle(reportView === "model")} onClick={() => setReportView("model")}>By Model</button>
          </div>
          <div style={cardStyle}>
            {reportData[reportView].length === 0
              ? <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>No data yet.</div>
              : renderTable(reportData[reportView])}
          </div>
        </div>
      )}

      {tab === "alerts" && (
        <div>
          {alerts.filter((a) => !a.dismissed).length === 0 && <div style={cardStyle}>No active alerts.</div>}
          {alerts.map((a) => (
            <div key={a.id} style={{ ...cardStyle, opacity: a.dismissed ? 0.5 : 1 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={badgeStyle(a.severity)}>{a.severity}</span>
                  <span>{a.message}</span>
                </div>
                {!a.dismissed && (
                  <button style={{ ...btnStyle, fontSize: 10, padding: "3px 8px" }} onClick={() => dismissAlert(a.id)}>Dismiss</button>
                )}
              </div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>{new Date(a.timestamp).toLocaleString()}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
