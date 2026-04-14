import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, X, RefreshCw } from "lucide-react";

const scoreColor = (score: number) => score > 80 ? "var(--success-color)" : score >= 50 ? "var(--warning-color)" : "var(--error-color)";

interface DocSyncStatus { total_sections: number; avg_freshness: number; stale_count: number; alerts: number }
interface DocAlert { id: string; type: string; severity: string; message: string }
interface DocLink { spec: string; code: string; type: string; doc_exists: boolean; code_exists: boolean }
interface DocSection { name: string; score: number; age_days: number; path: string }

interface Props { workspacePath?: string }

export function DocSyncPanel({ workspacePath }: Props) {
  const [tab, setTab] = useState("status");
  const [threshold, setThreshold] = useState(70);
  const [autoReconcile, setAutoReconcile] = useState(false);
  const [watchPatterns, setWatchPatterns] = useState(["docs/**/*.md", "README.md", "CHANGELOG.md"]);
  const [newPattern, setNewPattern] = useState("");
  const [status, setStatus] = useState<DocSyncStatus>({ total_sections: 0, avg_freshness: 100, stale_count: 0, alerts: 0 });
  const [alerts, setAlerts] = useState<DocAlert[]>([]);
  const [links, setLinks] = useState<DocLink[]>([]);
  const [sections, setSections] = useState<DocSection[]>([]);

  const loadData = useCallback(async () => {
    invoke<DocSyncStatus>("docsync_status").then(setStatus).catch(() => {});
    invoke<DocAlert[]>("docsync_get_alerts").then((data) => {
      setAlerts(Array.isArray(data) ? data : []);
    }).catch(() => {});
    if (workspacePath) {
      invoke<DocLink[]>("docsync_get_links", { workspacePath }).then((data) => {
        setLinks(Array.isArray(data) ? data : []);
      }).catch(() => {});
      invoke<DocSection[]>("docsync_get_sections", { workspacePath }).then((data) => {
        setSections(Array.isArray(data) ? data : []);
      }).catch(() => {});
    }
  }, [workspacePath]);

  useEffect(() => { loadData(); }, [loadData]);

  const handleReconcile = useCallback(async () => {
    try {
      await invoke("docsync_reconcile");
      const s = await invoke<DocSyncStatus>("docsync_status");
      setStatus(s);
      setAlerts([]);
      if (workspacePath) {
        const sec = await invoke<DocSection[]>("docsync_get_sections", { workspacePath });
        setSections(Array.isArray(sec) ? sec : []);
      }
    } catch (_) { /* ignore */ }
  }, [workspacePath]);

  const resolveAlert = useCallback((id: string) => {
    setAlerts((prev) => prev.filter((a) => a.id !== id));
  }, []);

  const badgeStyle = (type: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)", fontWeight: 600,
    background: type === "Implementation" ? "#3b82f620" : type === "Reference" ? "#8b5cf620" : "#f59e0b20",
    color: type === "Implementation" ? "var(--accent-color)" : type === "Reference" ? "var(--accent-purple)" : "var(--warning-color)",
  });

  const existsBadge = (exists: boolean) => exists
    ? <Check size={10} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginLeft: 4, color: "var(--success-color)" }} />
    : <X size={10} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginLeft: 4, color: "var(--error-color)" }} />;

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Living Documentation Sync</h2>
      <div className="panel-tab-bar">
        {["status", "links", "alerts", "config"].map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
            {t === "alerts" && alerts.length > 0 && (
              <span style={{ marginLeft: 4, background: "var(--error-color)", color: "var(--btn-primary-fg, #fff)", borderRadius: "var(--radius-sm-alt)", padding: "0 5px", fontSize: "var(--font-size-xs)" }}>
                {alerts.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {tab === "status" && (
        <div>
          <div className="panel-card" style={{ fontWeight: 600, marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Freshness Report</span>
            <button className="panel-btn panel-btn-primary" onClick={handleReconcile}>
              <RefreshCw size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Reconcile
            </button>
          </div>
          {!workspacePath && (
            <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
              Open a workspace to see per-file freshness.
            </div>
          )}
          {workspacePath && sections.length === 0 && (
            <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
              No tracked documentation files found in workspace.
            </div>
          )}
          {sections.map((s) => (
            <div key={s.path} className="panel-card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <span style={{ fontSize: "var(--font-size-md)" }}>{s.name}</span>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{s.path} · {s.age_days}d ago</div>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ width: 120, height: 8, borderRadius: "var(--radius-xs-plus)", background: "var(--border-color)" }}>
                  <div style={{ width: `${s.score}%`, height: 8, borderRadius: "var(--radius-xs-plus)", background: scoreColor(s.score) }} />
                </div>
                <span style={{ color: scoreColor(s.score), fontWeight: 600, fontSize: "var(--font-size-md)", minWidth: 36 }}>{s.score}%</span>
              </div>
            </div>
          ))}
          <div className="panel-card" style={{ display: "flex", gap: 16, fontSize: "var(--font-size-md)", color: "var(--text-secondary)" }}>
            <span>Stale: <strong style={{ color: "var(--error-color)" }}>{status.stale_count}</strong></span>
            <span>Avg freshness: <strong style={{ color: scoreColor(status.avg_freshness) }}>{Math.round(status.avg_freshness)}%</strong></span>
            <span>Alerts: <strong style={{ color: "var(--warning-color)" }}>{status.alerts}</strong></span>
          </div>
        </div>
      )}

      {tab === "links" && (
        <div>
          {!workspacePath && (
            <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
              Open a workspace to see doc-code links.
            </div>
          )}
          {workspacePath && links.length === 0 && (
            <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
              No documentation links found. Add docs/ markdown files to your workspace.
            </div>
          )}
          {links.map((l, i) => (
            <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: "var(--font-size-md)", marginBottom: 4 }}>
                  {l.spec}{existsBadge(l.doc_exists)}
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
                  {l.code}{existsBadge(l.code_exists)}
                </div>
              </div>
              <span style={badgeStyle(l.type)}>{l.type}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "alerts" && (
        <div>
          {alerts.length === 0 && <div className="panel-card">No active drift alerts.</div>}
          {alerts.map((a) => (
            <div key={a.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{a.type}</div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 2 }}>{a.message}</div>
                <span style={{ fontSize: "var(--font-size-sm)", color: a.severity === "critical" ? "var(--error-color)" : a.severity === "high" ? "var(--warning-color)" : "var(--accent-color)" }}>
                  {a.severity.toUpperCase()}
                </span>
              </div>
              <button className="panel-btn panel-btn-primary" onClick={() => resolveAlert(a.id)}>Resolve</button>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div className="panel-card">
            <label className="panel-label">Drift Threshold: {threshold}%</label>
            <input type="range" min={0} max={100} value={threshold} onChange={(e) => setThreshold(Number(e.target.value))}
              style={{ width: "100%" }} />
          </div>
          <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Auto-Reconcile</span>
            <button className={`panel-btn ${autoReconcile ? "panel-btn-primary" : "panel-btn-secondary"}`}
              onClick={() => setAutoReconcile(!autoReconcile)}>{autoReconcile ? "ON" : "OFF"}</button>
          </div>
          <div className="panel-card">
            <label className="panel-label">Watch Patterns</label>
            {watchPatterns.map((p, i) => (
              <div key={i} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "2px 0" }}>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{p}</span>
                <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "1px 6px" }}
                  onClick={() => setWatchPatterns((prev) => prev.filter((_, j) => j !== i))}><X size={11} strokeWidth={1.5} style={{ display: "block" }} /></button>
              </div>
            ))}
            <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
              <input className="panel-input" style={{ flex: 1, fontSize: "var(--font-size-base)" }} placeholder="Add pattern…"
                value={newPattern} onChange={(e) => setNewPattern(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter" && newPattern.trim()) { setWatchPatterns((p) => [...p, newPattern.trim()]); setNewPattern(""); } }} />
              <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-base)" }}
                disabled={!newPattern.trim()}
                onClick={() => { setWatchPatterns((p) => [...p, newPattern.trim()]); setNewPattern(""); }}>Add</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
