/**
 * RLDeploymentMonitor — Live deployment health panel.
 *
 * Deployment list with status badges, reward drift chart, distributional
 * shift indicator, auto-rollback status, A/B test results, and latency percentiles.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Deployment {
  id: string;
  name: string;
  policyVersion: string;
  status: string;
  autoRollback: boolean;
}

interface DeploymentHealth {
  deploymentId: string;
  rewardDrift: number[];
  distributionalShift: number;
  shiftThreshold: number;
  rollbackTriggered: boolean;
  abTest: ABTestResult | null;
  latencyP50: number;
  latencyP95: number;
  latencyP99: number;
}

interface ABTestResult {
  variantA: string;
  variantB: string;
  rewardA: number;
  rewardB: number;
  pValue: number;
  significant: boolean;
}

const badgeStyle: React.CSSProperties = { fontSize: 10, padding: "2px 6px", borderRadius: 3, color: "#fff", marginLeft: 6 };

const statusColor = (s: string) => s === "healthy" ? "var(--success-color)" : s === "degraded" ? "var(--warning-color)" : s === "rollback" ? "var(--error-color)" : "var(--text-secondary)";

export function RLDeploymentMonitor() {
  const [deployments, setDeployments] = useState<Deployment[]>([]);
  const [health, setHealth] = useState<DeploymentHealth | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<Deployment[]>("rl_list_deployments").then(setDeployments).catch(console.error);
  }, []);

  const loadHealth = useCallback(async (id: string) => {
    setSelectedId(id);
    setLoading(true);
    try {
      const res = await invoke<DeploymentHealth>("rl_get_deployment_health", { deploymentId: id });
      setHealth(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>Deployment Monitor</h2>

      <div className="panel-card">
        <div className="panel-label">Deployments</div>
        {deployments.map(d => (
          <div key={d.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center", background: selectedId === d.id ? "var(--bg-tertiary)" : undefined }} onClick={() => loadHealth(d.id)}>
            <span>
              <span style={{ fontWeight: 600 }}>{d.name}</span>
              <span style={{ ...badgeStyle, background: statusColor(d.status) }}>{d.status}</span>
            </span>
            <span className="panel-label">v{d.policyVersion} {d.autoRollback && <span style={{ ...badgeStyle, background: "var(--info-color)" }}>auto-rollback</span>}</span>
          </div>
        ))}
        {deployments.length === 0 && <div className="panel-empty">No deployments found.</div>}
      </div>

      {loading && <div className="panel-loading">Loading health data...</div>}
      {health && !loading && (
        <>
          <div className="panel-card">
            <div className="panel-label">Reward Drift (recent {health.rewardDrift.length} ticks)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 50 }}>
              {health.rewardDrift.map((v, i, a) => {
                const max = Math.max(...a.map(Math.abs), 0.01);
                const h = Math.abs(v) / max * 50;
                return <div key={i} style={{ flex: 1, height: h, background: v >= 0 ? "var(--success-color)" : "var(--error-color)", borderRadius: "2px 2px 0 0" }} />;
              })}
            </div>
          </div>

          <div className="panel-card" style={{ display: "flex", gap: 16, justifyContent: "space-around", textAlign: "center" }}>
            <div>
              <div style={{ fontSize: 18, fontWeight: 700, color: health.distributionalShift > health.shiftThreshold ? "var(--error-color)" : "var(--success-color)" }}>{health.distributionalShift.toFixed(4)}</div>
              <div className="panel-label">Dist. Shift (threshold: {health.shiftThreshold})</div>
            </div>
            <div>
              <div style={{ fontSize: 18, fontWeight: 700, color: health.rollbackTriggered ? "var(--error-color)" : "var(--success-color)" }}>{health.rollbackTriggered ? "TRIGGERED" : "OK"}</div>
              <div className="panel-label">Auto-Rollback</div>
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Action Latency</div>
            <div style={{ display: "flex", gap: 16, justifyContent: "space-around", textAlign: "center" }}>
              <div><div style={{ fontWeight: 700 }}>{health.latencyP50.toFixed(1)}ms</div><div className="panel-label">p50</div></div>
              <div><div style={{ fontWeight: 700 }}>{health.latencyP95.toFixed(1)}ms</div><div className="panel-label">p95</div></div>
              <div><div style={{ fontWeight: 700, color: health.latencyP99 > 100 ? "var(--error-color)" : "var(--text-primary)" }}>{health.latencyP99.toFixed(1)}ms</div><div className="panel-label">p99</div></div>
            </div>
          </div>

          {health.abTest && (
            <div className="panel-card">
              <div className="panel-label">A/B Test</div>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span>{health.abTest.variantA}: <strong>{health.abTest.rewardA.toFixed(4)}</strong></span>
                <span>vs</span>
                <span>{health.abTest.variantB}: <strong>{health.abTest.rewardB.toFixed(4)}</strong></span>
              </div>
              <div className="panel-label">
                p-value: {health.abTest.pValue.toFixed(4)} — <span style={{ color: health.abTest.significant ? "var(--success-color)" : "var(--warning-color)" }}>{health.abTest.significant ? "Significant" : "Not significant"}</span>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
