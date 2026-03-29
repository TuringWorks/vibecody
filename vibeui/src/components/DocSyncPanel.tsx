import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
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

const scoreColor = (score: number) => score > 80 ? "var(--success-color)" : score >= 50 ? "var(--warning-color)" : "var(--error-color)";

interface DocSyncStatus { total_sections: number; avg_freshness: number; stale_count: number; alerts: number }
interface DocAlert { id: string; type: string; severity: string; message: string }

export function DocSyncPanel() {
  const [tab, setTab] = useState("status");
  const [threshold, setThreshold] = useState(70);
  const [autoReconcile, setAutoReconcile] = useState(false);
  const [watchPatterns] = useState(["docs/**/*.md", "README.md", "CHANGELOG.md"]);
  const [status, setStatus] = useState<DocSyncStatus>({ total_sections: 0, avg_freshness: 100, stale_count: 0, alerts: 0 });
  const [alerts, setAlerts] = useState<DocAlert[]>([]);
  const [links] = useState([
    { spec: "docs/api.md#auth", code: "src/auth/handler.rs", type: "Implementation" },
    { spec: "docs/api.md#users", code: "src/users/mod.rs", type: "Implementation" },
    { spec: "docs/arch.md#caching", code: "src/cache/redis.rs", type: "Reference" },
    { spec: "CHANGELOG.md", code: "src/version.rs", type: "Version" },
  ]);

  useEffect(() => {
    invoke<DocSyncStatus>("docsync_status").then(setStatus).catch(() => {});
    invoke<DocAlert[]>("docsync_get_alerts").then(setAlerts).catch(() => {});
  }, []);

  const handleReconcile = useCallback(async () => {
    try {
      await invoke("docsync_reconcile");
      const s = await invoke<DocSyncStatus>("docsync_status");
      setStatus(s);
      setAlerts([]);
    } catch (_) { /* ignore */ }
  }, []);

  const resolveAlert = useCallback((id: string) => {
    setAlerts((prev) => prev.filter((a) => a.id !== id));
  }, []);

  const badgeStyle = (type: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
    background: type === "Implementation" ? "#3b82f620" : type === "Reference" ? "#8b5cf620" : "#f59e0b20",
    color: type === "Implementation" ? "var(--accent-color)" : type === "Reference" ? "var(--accent-purple)" : "var(--warning-color)",
  });

  // Build sections from status for display
  const sections = status.total_sections > 0 ? [
    { name: "API Reference", score: Math.min(100, status.avg_freshness + 5) },
    { name: "Architecture Guide", score: Math.max(0, status.avg_freshness - 15) },
    { name: "Getting Started", score: Math.max(0, status.avg_freshness - 30) },
    { name: "Configuration", score: Math.min(100, status.avg_freshness + 2) },
    { name: "Deployment Guide", score: Math.max(0, status.avg_freshness - 40) },
  ] : [];

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Living Documentation Sync</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["status", "links", "alerts", "config"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "status" && (
        <div>
          <div style={{ ...cardStyle, fontWeight: 600, marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Freshness Report</span>
            <button style={btnStyle} onClick={handleReconcile}>Reconcile</button>
          </div>
          {sections.length === 0 && <div style={cardStyle}>No sections tracked yet.</div>}
          {sections.map((s) => (
            <div key={s.name} style={{ ...cardStyle, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span>{s.name}</span>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ width: 120, height: 8, borderRadius: 4, background: "var(--border-color)" }}>
                  <div style={{ width: `${s.score}%`, height: 8, borderRadius: 4, background: scoreColor(s.score) }} />
                </div>
                <span style={{ color: scoreColor(s.score), fontWeight: 600, fontSize: 13, minWidth: 36 }}>{s.score}%</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "links" && (
        <div>
          {links.map((l, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 13, marginBottom: 4 }}>{l.spec}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{l.code}</div>
              </div>
              <span style={badgeStyle(l.type)}>{l.type}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "alerts" && (
        <div>
          {alerts.length === 0 && <div style={cardStyle}>No active drift alerts.</div>}
          {alerts.map((a) => (
            <div key={a.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 13 }}>{a.type}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{a.message}</div>
                <span style={{ fontSize: 11, color: a.severity === "critical" ? "var(--error-color)" : a.severity === "high" ? "var(--warning-color)" : "var(--accent-color)" }}>
                  {a.severity.toUpperCase()}
                </span>
              </div>
              <button style={btnStyle} onClick={() => resolveAlert(a.id)}>Resolve</button>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ marginBottom: 8, fontWeight: 600, fontSize: 13 }}>Drift Threshold: {threshold}%</div>
            <input type="range" min={0} max={100} value={threshold} onChange={(e) => setThreshold(Number(e.target.value))}
              style={{ width: "100%" }} />
          </div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600, fontSize: 13 }}>Auto-Reconcile</span>
            <button style={{ ...btnStyle, background: autoReconcile ? "var(--success-color)" : "var(--border-color)" }}
              onClick={() => setAutoReconcile(!autoReconcile)}>{autoReconcile ? "ON" : "OFF"}</button>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Watch Patterns</div>
            {watchPatterns.map((p, i) => (
              <div key={i} style={{ fontSize: 12, color: "var(--text-secondary)", padding: "2px 0" }}>{p}</div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
