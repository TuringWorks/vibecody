/**
 * UsageMeteringPanel — Usage Metering panel.
 *
 * Dashboard for tracking AI usage: spend, tokens, requests, budgets,
 * provider/model breakdowns, and configurable alerts.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState } from "react";

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

// ── Mock Data ─────────────────────────────────────────────────────────────────

const MOCK_KPI = { totalSpend: 142.87, tokensUsed: 8_450_000, requests: 3_241 };

const MOCK_BUDGETS: Budget[] = [
  { id: "bg1", name: "Daily Claude", limit: 10.00, used: 7.23, period: "daily" },
  { id: "bg2", name: "Weekly GPT-4o", limit: 50.00, used: 32.10, period: "weekly" },
  { id: "bg3", name: "Monthly Total", limit: 200.00, used: 142.87, period: "monthly" },
  { id: "bg4", name: "Dev Team Budget", limit: 500.00, used: 289.50, period: "monthly" },
];

const MOCK_BY_PROVIDER: UsageRow[] = [
  { label: "Anthropic (Claude)", tokens: 4_200_000, requests: 1_520, cost: 78.40 },
  { label: "OpenAI (GPT)", tokens: 2_800_000, requests: 1_100, cost: 42.30 },
  { label: "Google (Gemini)", tokens: 950_000, requests: 420, cost: 14.20 },
  { label: "Ollama (Local)", tokens: 500_000, requests: 201, cost: 0.00 },
];

const MOCK_BY_MODEL: UsageRow[] = [
  { label: "claude-opus-4-20250514", tokens: 2_100_000, requests: 620, cost: 52.50 },
  { label: "claude-sonnet-4-20250514", tokens: 2_100_000, requests: 900, cost: 25.90 },
  { label: "gpt-4o", tokens: 1_800_000, requests: 700, cost: 31.50 },
  { label: "gpt-4o-mini", tokens: 1_000_000, requests: 400, cost: 10.80 },
  { label: "gemini-2.0-pro", tokens: 950_000, requests: 420, cost: 14.20 },
  { label: "llama3:70b", tokens: 500_000, requests: 201, cost: 0.00 },
];

const MOCK_BY_TASK: UsageRow[] = [
  { label: "Code Generation", tokens: 3_200_000, requests: 1_100, cost: 58.20 },
  { label: "Code Review", tokens: 1_800_000, requests: 650, cost: 32.10 },
  { label: "Debugging", tokens: 1_500_000, requests: 520, cost: 24.80 },
  { label: "Documentation", tokens: 950_000, requests: 480, cost: 15.40 },
  { label: "Refactoring", tokens: 600_000, requests: 291, cost: 8.20 },
  { label: "Testing", tokens: 400_000, requests: 200, cost: 4.17 },
];

const MOCK_ALERTS: UsageAlert[] = [
  { id: "a1", severity: "critical", message: "Daily Claude budget at 72% — approaching limit", timestamp: "2026-03-13T08:30:00Z", dismissed: false },
  { id: "a2", severity: "warning", message: "Monthly total spend exceeded $100", timestamp: "2026-03-12T16:00:00Z", dismissed: false },
  { id: "a3", severity: "info", message: "Weekly GPT-4o usage is 64% of budget", timestamp: "2026-03-12T10:00:00Z", dismissed: false },
  { id: "a4", severity: "warning", message: "Token usage spike detected: 2x normal rate", timestamp: "2026-03-11T14:30:00Z", dismissed: true },
  { id: "a5", severity: "info", message: "New billing period started for Monthly Total", timestamp: "2026-03-01T00:00:00Z", dismissed: true },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "white" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box" };
const selectStyle: React.CSSProperties = { ...inputStyle, width: "auto", cursor: "pointer" };

const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 12 };

const severityColor: Record<string, string> = { info: "var(--info-color)", warning: "var(--warning-color)", critical: "var(--error-color)" };
const badgeStyle = (severity: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "white", background: severityColor[severity] || "var(--text-muted)" });

const budgetBarColor = (pct: number) => pct >= 90 ? "var(--error-color)" : pct >= 70 ? "var(--warning-color)" : "var(--success-color)";
const formatTokens = (n: number) => n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1_000 ? `${(n / 1_000).toFixed(0)}k` : String(n);

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "dashboard" | "budgets" | "reports" | "alerts";
type ReportView = "provider" | "model" | "task";

export function UsageMeteringPanel() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [budgets, setBudgets] = useState<Budget[]>(MOCK_BUDGETS);
  const [alerts, setAlerts] = useState<UsageAlert[]>(MOCK_ALERTS);
  const [reportView, setReportView] = useState<ReportView>("provider");

  // Create budget form
  const [newBudgetName, setNewBudgetName] = useState("");
  const [newBudgetLimit, setNewBudgetLimit] = useState("");
  const [newBudgetPeriod, setNewBudgetPeriod] = useState<"daily" | "weekly" | "monthly">("monthly");

  const createBudget = () => {
    if (!newBudgetName.trim() || !newBudgetLimit) return;
    const budget: Budget = {
      id: `bg${Date.now()}`,
      name: newBudgetName.trim(),
      limit: parseFloat(newBudgetLimit),
      used: 0,
      period: newBudgetPeriod,
    };
    setBudgets((prev) => [...prev, budget]);
    setNewBudgetName("");
    setNewBudgetLimit("");
  };

  const deleteBudget = (id: string) => {
    setBudgets((prev) => prev.filter((b) => b.id !== id));
  };

  const dismissAlert = (id: string) => {
    setAlerts((prev) => prev.map((a) => (a.id === id ? { ...a, dismissed: true } : a)));
  };

  const reportData: Record<ReportView, UsageRow[]> = { provider: MOCK_BY_PROVIDER, model: MOCK_BY_MODEL, task: MOCK_BY_TASK };

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

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Usage Metering</h2>

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
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent-primary)" }}>${MOCK_KPI.totalSpend.toFixed(2)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Tokens Used</div>
              <div style={{ fontSize: 22, fontWeight: 700 }}>{formatTokens(MOCK_KPI.tokensUsed)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Requests</div>
              <div style={{ fontSize: 22, fontWeight: 700 }}>{MOCK_KPI.requests.toLocaleString()}</div>
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Spend by Provider</div>
            {MOCK_BY_PROVIDER.map((p) => {
              const pct = (p.cost / MOCK_KPI.totalSpend) * 100;
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
          {budgets.map((b) => {
            const pct = (b.used / b.limit) * 100;
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
            <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "white", marginTop: 8 }} onClick={createBudget}>Create</button>
          </div>
        </div>
      )}

      {tab === "reports" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <button style={tabBtnStyle(reportView === "provider")} onClick={() => setReportView("provider")}>By Provider</button>
            <button style={tabBtnStyle(reportView === "model")} onClick={() => setReportView("model")}>By Model</button>
            <button style={tabBtnStyle(reportView === "task")} onClick={() => setReportView("task")}>By Task</button>
          </div>
          <div style={cardStyle}>{renderTable(reportData[reportView])}</div>
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
