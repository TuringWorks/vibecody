/* eslint-disable @typescript-eslint/no-explicit-any */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ModelEntry {
  id: string;
  name: string;
  provider: string;
  costPer1k: number;
  qualityScore: number;
  latencyMs: number;
}

interface RoutingDecision {
  id: string;
  query: string;
  chosenModel: string;
  reason: string;
  timestamp: string;
}

interface AbTest {
  id: string;
  name: string;
  modelA: string;
  modelB: string;
  samplesA: number;
  samplesB: number;
  winnerScore: { a: number; b: number };
  status: "active" | "concluded";
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

export function CostRouterPanel() {
  const [tab, setTab] = useState("models");
  const [models] = useState<ModelEntry[]>([
    { id: "m1", name: "claude-opus-4", provider: "Anthropic", costPer1k: 0.015, qualityScore: 95, latencyMs: 1200 },
    { id: "m2", name: "gpt-4o", provider: "OpenAI", costPer1k: 0.010, qualityScore: 92, latencyMs: 800 },
    { id: "m3", name: "claude-sonnet-4", provider: "Anthropic", costPer1k: 0.003, qualityScore: 88, latencyMs: 600 },
    { id: "m4", name: "llama-3-70b", provider: "Ollama", costPer1k: 0.000, qualityScore: 82, latencyMs: 2000 },
  ]);
  const [decisions, setDecisions] = useState<RoutingDecision[]>([]);
  const [budget, setBudget] = useState<{ total: number; spent: number; remaining: number }>({ total: 100, spent: 0, remaining: 100 });
  const [alertThreshold, setAlertThreshold] = useState(80);
  const [abTests] = useState<AbTest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [routerData, budgetData] = await Promise.all([
        invoke<unknown>("cost_router_list_models"),
        invoke<unknown>("cost_router_get_budget"),
      ]);
      const rd = routerData as any;
      if (rd?.decisions && Array.isArray(rd.decisions)) {
        setDecisions(rd.decisions);
      }
      const bd = (budgetData ?? rd?.budget) as any;
      if (bd) {
        setBudget({
          total: bd.total ?? 100,
          spent: bd.spent ?? 0,
          remaining: bd.remaining ?? (bd.total ?? 100) - (bd.spent ?? 0),
        });
      }
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchData().finally(() => setLoading(false));
  }, [fetchData]);

  const pct = budget.total > 0 ? (budget.spent / budget.total) * 100 : 0;

  if (loading) return <div className="panel-container"><div className="panel-loading">Loading cost router data...</div></div>;
  if (error) return <div className="panel-container"><div className="panel-error">Error: {error}</div></div>;

  return (
    <div className="panel-container">
      <div className="panel-tab-bar">
        <button className={`panel-tab ${tab === "models" ? "active" : ""}`} onClick={() => setTab("models")}>Models</button>
        <button className={`panel-tab ${tab === "routing" ? "active" : ""}`} onClick={() => setTab("routing")}>Routing</button>
        <button className={`panel-tab ${tab === "budget" ? "active" : ""}`} onClick={() => setTab("budget")}>Budget</button>
        <button className={`panel-tab ${tab === "abtests" ? "active" : ""}`} onClick={() => setTab("abtests")}>A/B Tests</button>
      </div>

      <div className="panel-body">

      {tab === "models" && (
        <table style={{ width: "100%", fontSize: "var(--font-size-md)", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
              {["Model", "Provider", "Cost/1k", "Quality", "Latency"].map((h) => (
                <th key={h} style={{ textAlign: "left", padding: 8 }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                <td style={{ padding: 8, fontWeight: 600 }}>{m.name}</td>
                <td style={{ padding: 8 }}>{m.provider}</td>
                <td style={{ padding: 8 }}>${m.costPer1k.toFixed(3)}</td>
                <td style={{ padding: 8 }}><span style={badgeStyle(m.qualityScore >= 90 ? "var(--success-color)" : m.qualityScore >= 80 ? "var(--warning-color)" : "var(--text-secondary)")}>{m.qualityScore}</span></td>
                <td style={{ padding: 8 }}>{m.latencyMs}ms</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {tab === "routing" && (
        <div>
          {decisions.length === 0 && <div className="panel-empty">No routing decisions recorded yet.</div>}
          {decisions.map((d: any, idx: number) => (
            <div key={d.id || idx} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <strong style={{ fontSize: "var(--font-size-md)" }}>{d.query}</strong>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{d.timestamp}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-md)" }}>Routed to: <span style={badgeStyle("var(--accent-indigo)")}>{d.chosenModel || d.chosen_model}</span></div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4, fontStyle: "italic" }}>{d.reason}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "budget" && (
        <div>
          <div className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
              <span style={{ fontWeight: 600 }}>Budget Usage</span>
              <span style={{ fontWeight: 600 }}>${budget.spent.toFixed(2)} / ${budget.total.toFixed(2)}</span>
            </div>
            <div style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", height: 12 }}>
              <div style={{ background: pct > alertThreshold ? "var(--error-color)" : "var(--accent-color)", borderRadius: "var(--radius-xs-plus)", height: 12, width: `${Math.min(pct, 100)}%` }} />
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Remaining: ${budget.remaining.toFixed(2)}</div>
          </div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Alert Threshold: {alertThreshold}%</div>
            <input type="range" min={50} max={100} value={alertThreshold} onChange={(e) => setAlertThreshold(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        </div>
      )}

      {tab === "abtests" && (
        <div>
          {abTests.length === 0 && <div className="panel-empty">No A/B tests configured yet.</div>}
          {abTests.map((t) => {
            const winner = t.winnerScore.a > t.winnerScore.b ? t.modelA : t.modelB;
            return (
              <div key={t.id} className="panel-card">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <strong>{t.name}</strong>
                  <span style={badgeStyle(t.status === "active" ? "var(--accent-color)" : "var(--success-color)")}>{t.status}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-md)", display: "flex", gap: 16, marginBottom: 6 }}>
                  <span>{t.modelA}: <strong>{t.winnerScore.a}</strong> ({t.samplesA} samples)</span>
                  <span>vs</span>
                  <span>{t.modelB}: <strong>{t.winnerScore.b}</strong> ({t.samplesB} samples)</span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)" }}>Winner: <span style={badgeStyle("var(--success-color)")}>{winner}</span></div>
              </div>
            );
          })}
        </div>
      )}
      </div>
    </div>
  );
}
