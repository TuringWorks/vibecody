/**
 * SessionMemoryPanel — Session Memory Profiling panel.
 *
 * Monitor session health, view memory usage over time, and manage
 * memory alerts with severity tracking.
 * Wired to Tauri backend commands for live data.
 */
import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface HealthStatus {
  status: "healthy" | "warning" | "critical";
  uptimeSec: number;
  memoryUsedMb: number;
  memoryLimitMb: number;
  growthRateMbPerMin: number;
  gcCount: number;
  lastGcAt: string;
  peakMemoryMb: number;
}

interface MemorySample {
  id: string;
  timestamp: string;
  heapUsedMb: number;
  heapTotalMb: number;
  externalMb: number;
  contextTokens: number;
  activeSessions: number;
}

interface MemoryAlert {
  id: string;
  type: "high_usage" | "rapid_growth" | "gc_pressure" | "token_overflow" | "leak_suspected";
  severity: "info" | "warning" | "critical";
  message: string;
  timestamp: string;
  resolved: boolean;
}

// ── Fallback Data ─────────────────────────────────────────────────────────────

const FALLBACK_HEALTH: HealthStatus = {
  status: "healthy",
  uptimeSec: 0,
  memoryUsedMb: 0,
  memoryLimitMb: 512,
  growthRateMbPerMin: 0,
  gcCount: 0,
  lastGcAt: new Date().toISOString(),
  peakMemoryMb: 0,
};

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg)" : "var(--text-primary)", marginRight: 4 });

const barBg: React.CSSProperties = { height: 12, borderRadius: 6, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 6, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 12, fontFamily: "var(--font-mono)" };

const statusColors: Record<string, string> = { healthy: "var(--success-color)", warning: "var(--warning-color)", critical: "var(--error-color)" };
const severityColors: Record<string, string> = { info: "var(--info-color)", warning: "var(--warning-color)", critical: "var(--error-color)" };
const typeLabels: Record<string, string> = { high_usage: "High Usage", rapid_growth: "Rapid Growth", gc_pressure: "GC Pressure", token_overflow: "Token Overflow", leak_suspected: "Leak Suspected" };
const badgeStyle = (color: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--btn-primary-fg)", background: color });

const formatUptime = (sec: number): string => {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  return `${h}h ${m}m`;
};

const formatTokens = (n: number) => n >= 1000 ? `${(n / 1000).toFixed(0)}k` : String(n);

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "health" | "samples" | "alerts";

export function SessionMemoryPanel() {
  const [tab, setTab] = useState<Tab>("health");
  const [health, setHealth] = useState<HealthStatus>(FALLBACK_HEALTH);
  const [samples, setSamples] = useState<MemorySample[]>([]);
  const [alerts, setAlerts] = useState<MemoryAlert[]>([]);
  const [loading, setLoading] = useState(true);
  const [compacting, setCompacting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchHealth = useCallback(async () => {
    try {
      const data = await invoke<HealthStatus>("get_session_memory_health");
      setHealth(data);
    } catch (err) {
      console.error("Failed to fetch session memory health:", err);
      setError(String(err));
    }
  }, []);

  const fetchSamples = useCallback(async () => {
    try {
      const data = await invoke<MemorySample[]>("get_session_memory_samples");
      setSamples(data);
    } catch (err) {
      console.error("Failed to fetch session memory samples:", err);
    }
  }, []);

  const fetchAlerts = useCallback(async () => {
    try {
      const data = await invoke<MemoryAlert[]>("get_session_memory_alerts");
      setAlerts(data);
    } catch (err) {
      console.error("Failed to fetch session memory alerts:", err);
    }
  }, []);

  const loadAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    await Promise.all([fetchHealth(), fetchSamples(), fetchAlerts()]);
    setLoading(false);
  }, [fetchHealth, fetchSamples, fetchAlerts]);

  useEffect(() => {
    loadAll();
    const interval = setInterval(loadAll, 15000);
    return () => clearInterval(interval);
  }, [loadAll]);

  const activeAlerts = useMemo(() => alerts.filter((a) => !a.resolved), [alerts]);
  const memPct = health.memoryLimitMb > 0 ? (health.memoryUsedMb / health.memoryLimitMb) * 100 : 0;
  const memBarColor = memPct >= 80 ? "var(--error-color)" : memPct >= 60 ? "var(--warning-color)" : "var(--success-color)";
  const maxHeap = samples.length > 0 ? Math.max(...samples.map((s) => s.heapTotalMb)) : 1;

  const resolveAlert = async (id: string) => {
    try {
      await invoke("dismiss_session_memory_alert", { id });
      setAlerts((prev) => prev.map((a) => (a.id === id ? { ...a, resolved: true } : a)));
    } catch (err) {
      console.error("Failed to dismiss alert:", err);
    }
  };

  const runCompact = async () => {
    setCompacting(true);
    try {
      await invoke("run_session_memory_compact");
      await loadAll();
    } catch (err) {
      console.error("Failed to compact session memory:", err);
    } finally {
      setCompacting(false);
    }
  };

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <h2 style={{ ...headingStyle, margin: 0 }}>Session Memory Profiling</h2>
        <div style={{ display: "flex", gap: 6 }}>
          <button style={btnStyle} onClick={runCompact} disabled={compacting}>
            {compacting ? "Compacting..." : "Compact"}
          </button>
          <button style={btnStyle} onClick={loadAll} disabled={loading}>
            {loading ? "Loading..." : "Refresh"}
          </button>
        </div>
      </div>

      {error && (
        <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", fontSize: 12 }}>
          Error: {error}
        </div>
      )}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "health")} onClick={() => setTab("health")}>Health</button>
        <button style={tabBtnStyle(tab === "samples")} onClick={() => setTab("samples")}>Samples</button>
        <button style={tabBtnStyle(tab === "alerts")} onClick={() => setTab("alerts")}>Alerts ({activeAlerts.length})</button>
      </div>

      {tab === "health" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: 14, color: "var(--text-primary)" }}>Session Status</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2, fontFamily: "var(--font-mono)" }}>
                Uptime: {formatUptime(health.uptimeSec)} | GC runs: {health.gcCount}
              </div>
            </div>
            <span style={badgeStyle(statusColors[health.status] || "var(--info-color)")}>{health.status.toUpperCase()}</span>
          </div>

          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
              <span style={labelStyle}>Memory Usage</span>
              <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{health.memoryUsedMb} / {health.memoryLimitMb} MB</span>
            </div>
            <div style={barBg}>
              <div style={barFill(memPct, memBarColor)} />
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginTop: 4 }}>
              <span>{memPct.toFixed(1)}% used</span>
              <span>Peak: {health.peakMemoryMb} MB</span>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Growth Rate</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: health.growthRateMbPerMin > 1 ? "var(--warning-color)" : "var(--success-color)" }}>
                {health.growthRateMbPerMin} MB/min
              </div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Last GC</div>
              <div style={{ fontSize: 13, fontWeight: 600, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{new Date(health.lastGcAt).toLocaleTimeString()}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Active Alerts</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: activeAlerts.length > 0 ? "var(--warning-color)" : "var(--success-color)" }}>
                {activeAlerts.length}
              </div>
            </div>
          </div>
        </div>
      )}

      {tab === "samples" && (
        <div>
          {samples.length === 0 && <div style={cardStyle}>No memory samples recorded yet.</div>}
          {samples.length > 0 && (
            <>
              <div style={cardStyle}>
                <div style={labelStyle}>Memory Timeline (heap used)</div>
                <div style={{ marginTop: 8 }}>
                  {samples.map((s) => {
                    const pct = (s.heapUsedMb / maxHeap) * 100;
                    return (
                      <div key={s.id} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                        <div style={{ width: 50, fontSize: 10, color: "var(--text-secondary)" }}>{new Date(s.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</div>
                        <div style={{ ...barBg, flex: 1, height: 6 }}>
                          <div style={barFill(pct, pct > 75 ? "var(--error-color)" : pct > 50 ? "var(--warning-color)" : "var(--info-color)")} />
                        </div>
                        <div style={{ width: 50, fontSize: 10, textAlign: "right" }}>{s.heapUsedMb} MB</div>
                      </div>
                    );
                  })}
                </div>
              </div>

              <div style={cardStyle}>
                <div style={labelStyle}>Detailed Samples</div>
                <div style={{ overflowX: "auto" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse" }}>
                    <thead>
                      <tr>
                        <th style={thStyle}>Time</th>
                        <th style={{ ...thStyle, textAlign: "right" }}>Heap Used</th>
                        <th style={{ ...thStyle, textAlign: "right" }}>Heap Total</th>
                        <th style={{ ...thStyle, textAlign: "right" }}>External</th>
                        <th style={{ ...thStyle, textAlign: "right" }}>Ctx Tokens</th>
                        <th style={{ ...thStyle, textAlign: "right" }}>Sessions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {samples.map((s) => (
                        <tr key={s.id}>
                          <td style={tdStyle}>{new Date(s.timestamp).toLocaleTimeString()}</td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>{s.heapUsedMb} MB</td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>{s.heapTotalMb} MB</td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>{s.externalMb} MB</td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>{formatTokens(s.contextTokens)}</td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>{s.activeSessions}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </>
          )}
        </div>
      )}

      {tab === "alerts" && (
        <div>
          {alerts.length === 0 && <div style={cardStyle}>No memory alerts recorded.</div>}
          {activeAlerts.length === 0 && alerts.length > 0 && <div style={{ ...cardStyle, color: "var(--success-color)" }}>All alerts resolved.</div>}
          {alerts.map((a) => (
            <div key={a.id} style={{ ...cardStyle, opacity: a.resolved ? 0.5 : 1 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={badgeStyle(severityColors[a.severity])}>{a.severity}</span>
                  <span style={badgeStyle("var(--bg-tertiary)")}>{typeLabels[a.type]}</span>
                </div>
                {!a.resolved && (
                  <button style={{ ...btnStyle, fontSize: 10, padding: "3px 8px" }} onClick={() => resolveAlert(a.id)}>Resolve</button>
                )}
              </div>
              <div style={{ marginTop: 6, color: "var(--text-primary)" }}>{a.message}</div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginTop: 4 }}>
                {new Date(a.timestamp).toLocaleString()} {a.resolved && "— Resolved"}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
