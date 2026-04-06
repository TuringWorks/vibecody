/**
 * CompanyBudgetPanel — Per-agent monthly budget tracking.
 *
 * Shows budget utilization per agent, cost event timeline,
 * and alerts. Supports setting monthly budget limits.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyBudgetPanelProps {
  workspacePath?: string | null;
}

export function CompanyBudgetPanel({ workspacePath: _wp }: CompanyBudgetPanelProps) {
  const [budgetOutput, setBudgetOutput] = useState<string>("");
  const [eventsOutput, setEventsOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [agentId, setAgentId] = useState("");
  const [month, setMonth] = useState(() => {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
  });
  const [limitCents, setLimitCents] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const [b, e] = await Promise.all([
        invoke<string>("company_cmd", { args: "budget status" }),
        invoke<string>("company_cmd", { args: "budget events" }),
      ]);
      setBudgetOutput(b);
      setEventsOutput(e);
    } catch (err) {
      setBudgetOutput(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const setBudget = async () => {
    if (!agentId || !limitCents) return;
    try {
      const out = await invoke<string>("company_cmd", {
        args: `budget set ${agentId} ${month} ${parseInt(limitCents) * 100}`,
      });
      setCmdResult(out);
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const btnStyle: React.CSSProperties = {
    fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
    background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
  };
  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Budget</span>
        <button onClick={load} style={btnStyle}>
          Refresh
        </button>
      </div>

      {/* Set budget form */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Set Monthly Budget</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            value={agentId}
            onChange={(e) => setAgentId(e.target.value)}
            placeholder="Agent ID"
            style={{ width: 140, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <input
            value={month}
            onChange={(e) => setMonth(e.target.value)}
            placeholder="2026-04"
            style={{ width: 90, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <input
            value={limitCents}
            onChange={(e) => setLimitCents(e.target.value)}
            placeholder="Limit $ (USD)"
            type="number"
            style={{ width: 100, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <button onClick={setBudget} style={{...btnStyle, padding: "4px 12px"}}>
            Set
          </button>
        </div>
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
        </div>
      )}

      {loading ? (
        <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
      ) : (
        <>
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>Budget Status</div>
            <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12 }}>
              <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
                {budgetOutput || "No budgets set. Use the form above."}
              </pre>
            </div>
          </div>
          <div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>Cost Events</div>
            <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, maxHeight: 200, overflowY: "auto" }}>
              <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
                {eventsOutput || "No cost events recorded."}
              </pre>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
