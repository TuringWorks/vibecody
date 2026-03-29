import { useState } from "react";

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

export function CostRouterPanel() {
  const [tab, setTab] = useState("models");
  const [models] = useState<ModelEntry[]>([
    { id: "m1", name: "claude-opus-4", provider: "Anthropic", costPer1k: 0.015, qualityScore: 95, latencyMs: 1200 },
    { id: "m2", name: "gpt-4o", provider: "OpenAI", costPer1k: 0.010, qualityScore: 92, latencyMs: 800 },
    { id: "m3", name: "claude-sonnet-4", provider: "Anthropic", costPer1k: 0.003, qualityScore: 88, latencyMs: 600 },
    { id: "m4", name: "llama-3-70b", provider: "Ollama", costPer1k: 0.000, qualityScore: 82, latencyMs: 2000 },
  ]);
  const [decisions] = useState<RoutingDecision[]>([
    { id: "d1", query: "Refactor auth module", chosenModel: "claude-opus-4", reason: "High complexity task, quality priority", timestamp: "10:15" },
    { id: "d2", query: "Fix typo in README", chosenModel: "claude-sonnet-4", reason: "Simple edit, cost optimized", timestamp: "10:12" },
    { id: "d3", query: "Generate test suite", chosenModel: "gpt-4o", reason: "Balanced cost/quality, moderate complexity", timestamp: "10:08" },
  ]);
  const [budget] = useState(50.0);
  const [spent] = useState(18.42);
  const [alertThreshold, setAlertThreshold] = useState(80);
  const [abTests] = useState<AbTest[]>([
    { id: "ab1", name: "Code review quality", modelA: "claude-opus-4", modelB: "gpt-4o", samplesA: 45, samplesB: 43, winnerScore: { a: 87, b: 82 }, status: "active" },
    { id: "ab2", name: "Refactoring speed", modelA: "claude-sonnet-4", modelB: "llama-3-70b", samplesA: 30, samplesB: 30, winnerScore: { a: 91, b: 74 }, status: "concluded" },
  ]);

  const remaining = budget - spent;
  const pct = (spent / budget) * 100;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Cost-Optimized Routing</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "models")} onClick={() => setTab("models")}>Models</button>
        <button style={tabStyle(tab === "routing")} onClick={() => setTab("routing")}>Routing</button>
        <button style={tabStyle(tab === "budget")} onClick={() => setTab("budget")}>Budget</button>
        <button style={tabStyle(tab === "abtests")} onClick={() => setTab("abtests")}>A/B Tests</button>
      </div>

      {tab === "models" && (
        <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
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
          {decisions.map((d) => (
            <div key={d.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <strong style={{ fontSize: 13 }}>{d.query}</strong>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{d.timestamp}</span>
              </div>
              <div style={{ fontSize: 13 }}>Routed to: <span style={badgeStyle("#6366f1")}>{d.chosenModel}</span></div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4, fontStyle: "italic" }}>{d.reason}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "budget" && (
        <div>
          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
              <span style={{ fontWeight: 600 }}>Budget Usage</span>
              <span style={{ fontWeight: 600 }}>${spent.toFixed(2)} / ${budget.toFixed(2)}</span>
            </div>
            <div style={{ background: "var(--bg-primary)", borderRadius: 4, height: 12 }}>
              <div style={{ background: pct > alertThreshold ? "var(--error-color)" : "var(--accent-color)", borderRadius: 4, height: 12, width: `${Math.min(pct, 100)}%` }} />
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Remaining: ${remaining.toFixed(2)}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Alert Threshold: {alertThreshold}%</div>
            <input type="range" min={50} max={100} value={alertThreshold} onChange={(e) => setAlertThreshold(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        </div>
      )}

      {tab === "abtests" && (
        <div>
          {abTests.map((t) => {
            const winner = t.winnerScore.a > t.winnerScore.b ? t.modelA : t.modelB;
            return (
              <div key={t.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <strong>{t.name}</strong>
                  <span style={badgeStyle(t.status === "active" ? "var(--accent-color)" : "var(--success-color)")}>{t.status}</span>
                </div>
                <div style={{ fontSize: 13, display: "flex", gap: 16, marginBottom: 6 }}>
                  <span>{t.modelA}: <strong>{t.winnerScore.a}</strong> ({t.samplesA} samples)</span>
                  <span>vs</span>
                  <span>{t.modelB}: <strong>{t.winnerScore.b}</strong> ({t.samplesB} samples)</span>
                </div>
                <div style={{ fontSize: 12 }}>Winner: <span style={badgeStyle("var(--success-color)")}>{winner}</span></div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
