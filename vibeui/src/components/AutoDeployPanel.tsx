import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PipelineStage {
  id: string;
  name: string;
  kind: string;
  status: "pending" | "running" | "passed" | "failed" | "skipped";
  pre_conditions: string[];
  post_conditions: string[];
  duration_secs: number | null;
  order: number;
}

interface HealthGate {
  id: string;
  name: string;
  metric: string;
  threshold: string;
  current_value: string | null;
  passed: boolean | null;
}

interface DeployPlan {
  id: string;
  created_at: string;
  target_env: string;
  triggered_by: string;
  status: string;
  stages_total: number;
  stages_passed: number;
}

const STAGE_STATUS_COLORS: Record<string, string> = {
  pending: "var(--text-muted)",
  running: "var(--accent-color)",
  passed: "var(--success-color)",
  failed: "var(--error-color)",
  skipped: "var(--warning-color)",
};

const KIND_COLORS: Record<string, string> = {
  build: "#4a9eff",
  test: "#9c6fe0",
  deploy: "#4caf7d",
  validate: "#f0a050",
  rollback: "#e85d8a",
  notify: "#50c8e8",
};

export function AutoDeployPanel() {
  const [tab, setTab] = useState("pipeline");
  const [stages, setStages] = useState<PipelineStage[]>([]);
  const [gates, setGates] = useState<HealthGate[]>([]);
  const [history, setHistory] = useState<DeployPlan[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [planRes, gatesRes, histRes] = await Promise.all([
          invoke<PipelineStage[]>("auto_deploy_plan"),
          invoke<HealthGate[]>("auto_deploy_stage_status"),
          invoke<DeployPlan[]>("auto_deploy_history"),
        ]);
        setStages(Array.isArray(planRes) ? planRes.sort((a, b) => a.order - b.order) : []);
        setGates(Array.isArray(gatesRes) ? gatesRes : []);
        setHistory(Array.isArray(histRes) ? histRes : []);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const planStatusColor = (s: string) => {
    if (s === "succeeded") return "var(--success-color)";
    if (s === "failed") return "var(--error-color)";
    if (s === "running") return "var(--accent-color)";
    return "var(--text-muted)";
  };

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 12 }}>Auto Deploy</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["pipeline", "gates", "history"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: 6, cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12 }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "pipeline" && (
        <div style={{ position: "relative" }}>
          {stages.length === 0 && <div style={{ color: "var(--text-muted)" }}>No pipeline stages defined.</div>}
          <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
            {stages.map((stage, idx) => {
              const statusColor = STAGE_STATUS_COLORS[stage.status] ?? "var(--text-muted)";
              const kindColor = KIND_COLORS[stage.kind] ?? "var(--accent-color)";
              return (
                <div key={stage.id} style={{ display: "flex", gap: 0 }}>
                  <div style={{ display: "flex", flexDirection: "column", alignItems: "center", marginRight: 14 }}>
                    <div style={{ width: 28, height: 28, borderRadius: "50%", background: statusColor + "22", border: `2px solid ${statusColor}`, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 11, fontWeight: 700, color: statusColor, flexShrink: 0, marginTop: 12 }}>
                      {idx + 1}
                    </div>
                    {idx < stages.length - 1 && (
                      <div style={{ width: 2, flex: 1, minHeight: 16, background: "var(--border-color)", margin: "4px 0" }} />
                    )}
                  </div>
                  <div style={{ flex: 1, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${statusColor}`, borderRadius: 8, padding: "12px 14px", marginBottom: 8 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                      <span style={{ fontSize: 13, fontWeight: 600 }}>{stage.name}</span>
                      <span style={{ fontSize: 11, padding: "1px 8px", borderRadius: 8, background: kindColor + "22", color: kindColor, fontWeight: 600 }}>{stage.kind}</span>
                      <span style={{ fontSize: 11, padding: "1px 8px", borderRadius: 8, background: statusColor + "22", color: statusColor, marginLeft: "auto" }}>{stage.status}</span>
                      {stage.duration_secs !== null && (
                        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{stage.duration_secs}s</span>
                      )}
                    </div>
                    <div style={{ display: "flex", gap: 16, fontSize: 11 }}>
                      {stage.pre_conditions.length > 0 && (
                        <div>
                          <span style={{ color: "var(--text-muted)" }}>Pre: </span>
                          <span style={{ color: "var(--text-primary)" }}>{stage.pre_conditions.join(", ")}</span>
                        </div>
                      )}
                      {stage.post_conditions.length > 0 && (
                        <div>
                          <span style={{ color: "var(--text-muted)" }}>Post: </span>
                          <span style={{ color: "var(--text-primary)" }}>{stage.post_conditions.join(", ")}</span>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {!loading && tab === "gates" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {gates.length === 0 && <div style={{ color: "var(--text-muted)" }}>No health gates configured.</div>}
          {gates.map(gate => {
            const gateColor = gate.passed === null ? "var(--text-muted)" : gate.passed ? "var(--success-color)" : "var(--error-color)";
            return (
              <div key={gate.id} style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", borderLeft: `3px solid ${gateColor}`, padding: "12px 14px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <span style={{ fontSize: 13, fontWeight: 600 }}>{gate.name}</span>
                  <span style={{ fontSize: 11, padding: "2px 10px", borderRadius: 10, background: gateColor + "22", color: gateColor, fontWeight: 700, marginLeft: "auto" }}>
                    {gate.passed === null ? "pending" : gate.passed ? "passed" : "failed"}
                  </span>
                </div>
                <div style={{ display: "flex", gap: 20, fontSize: 12 }}>
                  <div><span style={{ color: "var(--text-muted)" }}>Metric: </span><span>{gate.metric}</span></div>
                  <div><span style={{ color: "var(--text-muted)" }}>Threshold: </span><span>{gate.threshold}</span></div>
                  {gate.current_value !== null && (
                    <div><span style={{ color: "var(--text-muted)" }}>Current: </span><span style={{ fontWeight: 600, color: gateColor }}>{gate.current_value}</span></div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {!loading && tab === "history" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Plan ID", "Created", "Target Env", "Triggered By", "Status", "Progress"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {history.length === 0 && (
                <tr><td colSpan={6} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No deployment history.</td></tr>
              )}
              {history.map(plan => (
                <tr key={plan.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", fontFamily: "var(--font-mono)", fontSize: 11 }}>{plan.id.slice(0, 8)}…</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{plan.created_at}</td>
                  <td style={{ padding: "6px 10px" }}>{plan.target_env}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{plan.triggered_by}</td>
                  <td style={{ padding: "6px 10px" }}>
                    <span style={{ padding: "2px 8px", borderRadius: 10, fontSize: 11, background: planStatusColor(plan.status) + "22", color: planStatusColor(plan.status) }}>{plan.status}</span>
                  </td>
                  <td style={{ padding: "6px 10px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <div style={{ width: 60, height: 5, background: "var(--bg-primary)", borderRadius: 3 }}>
                        <div style={{ height: "100%", width: `${plan.stages_total > 0 ? (plan.stages_passed / plan.stages_total) * 100 : 0}%`, background: "var(--success-color)", borderRadius: 3 }} />
                      </div>
                      <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{plan.stages_passed}/{plan.stages_total}</span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
