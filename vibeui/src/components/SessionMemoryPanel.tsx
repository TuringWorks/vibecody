/**
 * SessionMemoryPanel — Session Memory Profiling panel.
 *
 * Monitor session health, view memory usage over time, and manage
 * memory alerts with severity tracking.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState, useMemo } from "react";

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

// ── Mock Data ─────────────────────────────────────────────────────────────────

const MOCK_HEALTH: HealthStatus = {
  status: "warning",
  uptimeSec: 14520,
  memoryUsedMb: 384,
  memoryLimitMb: 512,
  growthRateMbPerMin: 1.2,
  gcCount: 47,
  lastGcAt: "2026-03-13T08:28:00Z",
  peakMemoryMb: 412,
};

const MOCK_SAMPLES: MemorySample[] = [
  { id: "s1", timestamp: "2026-03-13T04:00:00Z", heapUsedMb: 120, heapTotalMb: 200, externalMb: 15, contextTokens: 24000, activeSessions: 2 },
  { id: "s2", timestamp: "2026-03-13T05:00:00Z", heapUsedMb: 145, heapTotalMb: 220, externalMb: 18, contextTokens: 42000, activeSessions: 3 },
  { id: "s3", timestamp: "2026-03-13T05:30:00Z", heapUsedMb: 198, heapTotalMb: 280, externalMb: 22, contextTokens: 68000, activeSessions: 4 },
  { id: "s4", timestamp: "2026-03-13T06:00:00Z", heapUsedMb: 230, heapTotalMb: 320, externalMb: 28, contextTokens: 95000, activeSessions: 5 },
  { id: "s5", timestamp: "2026-03-13T06:30:00Z", heapUsedMb: 185, heapTotalMb: 280, externalMb: 20, contextTokens: 52000, activeSessions: 3 },
  { id: "s6", timestamp: "2026-03-13T07:00:00Z", heapUsedMb: 260, heapTotalMb: 360, externalMb: 32, contextTokens: 110000, activeSessions: 6 },
  { id: "s7", timestamp: "2026-03-13T07:30:00Z", heapUsedMb: 310, heapTotalMb: 400, externalMb: 38, contextTokens: 145000, activeSessions: 7 },
  { id: "s8", timestamp: "2026-03-13T08:00:00Z", heapUsedMb: 348, heapTotalMb: 440, externalMb: 42, contextTokens: 172000, activeSessions: 8 },
  { id: "s9", timestamp: "2026-03-13T08:15:00Z", heapUsedMb: 372, heapTotalMb: 460, externalMb: 45, contextTokens: 188000, activeSessions: 8 },
  { id: "s10", timestamp: "2026-03-13T08:30:00Z", heapUsedMb: 384, heapTotalMb: 480, externalMb: 48, contextTokens: 195000, activeSessions: 9 },
];

const MOCK_ALERTS: MemoryAlert[] = [
  { id: "a1", type: "high_usage", severity: "critical", message: "Memory usage at 75% of limit (384/512 MB)", timestamp: "2026-03-13T08:30:00Z", resolved: false },
  { id: "a2", type: "rapid_growth", severity: "warning", message: "Memory growing at 1.2 MB/min — will exhaust in ~107 minutes", timestamp: "2026-03-13T08:15:00Z", resolved: false },
  { id: "a3", type: "token_overflow", severity: "warning", message: "Context tokens approaching 200k limit (195k used)", timestamp: "2026-03-13T08:30:00Z", resolved: false },
  { id: "a4", type: "gc_pressure", severity: "info", message: "GC running frequently — 12 collections in last 30 minutes", timestamp: "2026-03-13T08:00:00Z", resolved: false },
  { id: "a5", type: "leak_suspected", severity: "warning", message: "Possible memory leak: heap grew 164 MB without corresponding session increase", timestamp: "2026-03-13T07:30:00Z", resolved: true },
  { id: "a6", type: "high_usage", severity: "info", message: "Memory usage crossed 50% threshold", timestamp: "2026-03-13T06:00:00Z", resolved: true },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "#fff" : "var(--text-primary)", marginRight: 4 });

const barBg: React.CSSProperties = { height: 12, borderRadius: 6, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 6, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 12 };

const statusColors: Record<string, string> = { healthy: "#22c55e", warning: "#f59e0b", critical: "#ef4444" };
const severityColors: Record<string, string> = { info: "#3b82f6", warning: "#f59e0b", critical: "#ef4444" };
const typeLabels: Record<string, string> = { high_usage: "High Usage", rapid_growth: "Rapid Growth", gc_pressure: "GC Pressure", token_overflow: "Token Overflow", leak_suspected: "Leak Suspected" };
const badgeStyle = (color: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "#fff", background: color });

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
  const [alerts, setAlerts] = useState<MemoryAlert[]>(MOCK_ALERTS);

  const activeAlerts = useMemo(() => alerts.filter((a) => !a.resolved), [alerts]);
  const memPct = (MOCK_HEALTH.memoryUsedMb / MOCK_HEALTH.memoryLimitMb) * 100;
  const memBarColor = memPct >= 80 ? "#ef4444" : memPct >= 60 ? "#f59e0b" : "#22c55e";
  const maxHeap = Math.max(...MOCK_SAMPLES.map((s) => s.heapTotalMb));

  const resolveAlert = (id: string) => {
    setAlerts((prev) => prev.map((a) => (a.id === id ? { ...a, resolved: true } : a)));
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Session Memory Profiling</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "health")} onClick={() => setTab("health")}>Health</button>
        <button style={tabBtnStyle(tab === "samples")} onClick={() => setTab("samples")}>Samples</button>
        <button style={tabBtnStyle(tab === "alerts")} onClick={() => setTab("alerts")}>Alerts ({activeAlerts.length})</button>
      </div>

      {tab === "health" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: 14 }}>Session Status</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                Uptime: {formatUptime(MOCK_HEALTH.uptimeSec)} | GC runs: {MOCK_HEALTH.gcCount}
              </div>
            </div>
            <span style={badgeStyle(statusColors[MOCK_HEALTH.status])}>{MOCK_HEALTH.status.toUpperCase()}</span>
          </div>

          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
              <span style={labelStyle}>Memory Usage</span>
              <span style={{ fontSize: 11 }}>{MOCK_HEALTH.memoryUsedMb} / {MOCK_HEALTH.memoryLimitMb} MB</span>
            </div>
            <div style={barBg}>
              <div style={barFill(memPct, memBarColor)} />
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
              <span>{memPct.toFixed(1)}% used</span>
              <span>Peak: {MOCK_HEALTH.peakMemoryMb} MB</span>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Growth Rate</div>
              <div style={{ fontSize: 20, fontWeight: 700, color: MOCK_HEALTH.growthRateMbPerMin > 1 ? "#f59e0b" : "#22c55e" }}>
                {MOCK_HEALTH.growthRateMbPerMin} MB/min
              </div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Last GC</div>
              <div style={{ fontSize: 13, fontWeight: 600 }}>{new Date(MOCK_HEALTH.lastGcAt).toLocaleTimeString()}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Active Alerts</div>
              <div style={{ fontSize: 20, fontWeight: 700, color: activeAlerts.length > 0 ? "#f59e0b" : "#22c55e" }}>
                {activeAlerts.length}
              </div>
            </div>
          </div>
        </div>
      )}

      {tab === "samples" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>Memory Timeline (heap used)</div>
            <div style={{ marginTop: 8 }}>
              {MOCK_SAMPLES.map((s) => {
                const pct = (s.heapUsedMb / maxHeap) * 100;
                return (
                  <div key={s.id} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <div style={{ width: 50, fontSize: 10, color: "var(--text-secondary)" }}>{new Date(s.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</div>
                    <div style={{ ...barBg, flex: 1, height: 6 }}>
                      <div style={barFill(pct, pct > 75 ? "#ef4444" : pct > 50 ? "#f59e0b" : "#3b82f6")} />
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
                  {MOCK_SAMPLES.map((s) => (
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
        </div>
      )}

      {tab === "alerts" && (
        <div>
          {activeAlerts.length === 0 && <div style={cardStyle}>No active memory alerts.</div>}
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
              <div style={{ marginTop: 6 }}>{a.message}</div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
                {new Date(a.timestamp).toLocaleString()} {a.resolved && "— Resolved"}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
