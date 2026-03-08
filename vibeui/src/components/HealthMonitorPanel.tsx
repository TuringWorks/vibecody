/**
 * HealthMonitorPanel — Service Health Monitor.
 *
 * Configure a list of HTTP endpoints, run one-shot or auto-refresh checks,
 * see latency, status codes, and per-service history sparklines.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HealthMonitor {
 id: string;
 label: string;
 url: string;
 expected_status: number;
 timeout_ms: number;
}

interface HealthCheckResult {
 id: string;
 url: string;
 ok: boolean;
 status_code: number | null;
 latency_ms: number;
 timestamp: number;
 error: string | null;
}

// Up to 20 historical data points per service
type History = Record<string, HealthCheckResult[]>;

const DEFAULT_MONITORS: HealthMonitor[] = [
 { id: "gh", label: "GitHub", url: "https://github.com", expected_status: 200, timeout_ms: 5000 },
 { id: "npm", label: "npm Registry", url: "https://registry.npmjs.org", expected_status: 200, timeout_ms: 5000 },
 { id: "crates", label: "crates.io", url: "https://crates.io", expected_status: 200, timeout_ms: 5000 },
];

function latencyColor(ms: number): string {
 if (ms < 300) return "var(--text-success, #a6e3a1)";
 if (ms < 1000) return "var(--text-warning, #f9e2af)";
 return "var(--text-danger, #f38ba8)";
}

function Sparkline({ history }: { history: HealthCheckResult[] }) {
 if (history.length < 2) return null;
 const maxLatency = Math.max(...history.map(h => h.latency_ms), 1);
 const w = 60, h = 22;
 const pts = history.slice(-12).map((r, i, arr) => {
 const x = (i / (arr.length - 1)) * w;
 const y = h - (r.latency_ms / maxLatency) * (h - 2) - 1;
 return `${x.toFixed(1)},${y.toFixed(1)}`;
 });
 return (
 <svg width={w} height={h} style={{ display: "block" }}>
 <polyline points={pts.join(" ")} fill="none" stroke="var(--accent-color)" strokeWidth="1.5" strokeLinejoin="round" />
 {history.slice(-12).map((r, i, arr) => {
 const x = (i / (arr.length - 1)) * w;
 const y = h - (r.latency_ms / maxLatency) * (h - 2) - 1;
 return <circle key={i} cx={x} cy={y} r={2} fill={r.ok ? "var(--success-color)" : "var(--error-color)"} />;
 })}
 </svg>
 );
}

function StatusBadge({ result }: { result: HealthCheckResult | undefined }) {
 if (!result) return <span style={{ fontSize: 10, color: "var(--text-muted)" }}>—</span>;
 const color = result.ok ? "var(--text-success, #a6e3a1)" : "var(--text-danger, #f38ba8)";
 return (
 <span style={{ padding: "2px 8px", borderRadius: 10, background: color + "22", border: `1px solid ${color}`, color, fontSize: 10, fontWeight: 700 }}>
 {result.ok ? "UP" : "DOWN"}
 </span>
 );
}

export function HealthMonitorPanel() {
 const [monitors, setMonitors] = useState<HealthMonitor[]>([]);
 const [results, setResults] = useState<Record<string, HealthCheckResult>>({});
 const [history, setHistory] = useState<History>({});
 const [checking, setChecking] = useState(false);
 const [autoRefresh, setAutoRefresh] = useState(false);
 const [intervalSec, setIntervalSec] = useState(30);
 const [showAdd, setShowAdd] = useState(false);
 const [newLabel, setNewLabel] = useState("");
 const [newUrl, setNewUrl] = useState("https://");
 const [newTimeout, setNewTimeout] = useState(5000);
 const [error, setError] = useState<string | null>(null);
 const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

 // Load saved monitors (or fall back to defaults)
 useEffect(() => {
 invoke<HealthMonitor[]>("get_health_monitors").then(m => {
 setMonitors(m.length > 0 ? m : DEFAULT_MONITORS);
 }).catch(() => setMonitors(DEFAULT_MONITORS));
 }, []);

 const applyResults = useCallback((res: HealthCheckResult[]) => {
 const map: Record<string, HealthCheckResult> = {};
 res.forEach(r => { map[r.id] = r; });
 setResults(prev => ({ ...prev, ...map }));
 setHistory(prev => {
 const next = { ...prev };
 res.forEach(r => {
 const arr = [...(next[r.id] ?? []), r];
 next[r.id] = arr.slice(-20);
 });
 return next;
 });
 }, []);

 const checkAll = useCallback(async () => {
 if (checking || monitors.length === 0) return;
 setChecking(true);
 setError(null);
 try {
 const res = await invoke<HealthCheckResult[]>("check_all_services", { monitors });
 applyResults(res);
 } catch (e) {
 setError(String(e));
 } finally {
 setChecking(false);
 }
 }, [checking, monitors, applyResults]);

 // Auto-refresh
 useEffect(() => {
 if (timerRef.current) clearInterval(timerRef.current);
 if (autoRefresh) {
 timerRef.current = setInterval(checkAll, intervalSec * 1000);
 }
 return () => { if (timerRef.current) clearInterval(timerRef.current); };
 }, [autoRefresh, intervalSec, checkAll]);

 const saveMonitors = async (list: HealthMonitor[]) => {
 setMonitors(list);
 await invoke("save_health_monitors", { monitors: list }).catch(() => {});
 };

 const addMonitor = async () => {
 if (!newUrl || !newLabel) return;
 const m: HealthMonitor = {
 id: `m-${Date.now()}`,
 label: newLabel.trim(),
 url: newUrl.trim(),
 expected_status: 200,
 timeout_ms: newTimeout,
 };
 await saveMonitors([...monitors, m]);
 setNewLabel(""); setNewUrl("https://"); setShowAdd(false);
 };

 const removeMonitor = async (id: string) => {
 await saveMonitors(monitors.filter(m => m.id !== id));
 setResults(prev => { const n = { ...prev }; delete n[id]; return n; });
 setHistory(prev => { const n = { ...prev }; delete n[id]; return n; });
 };

 const checkOne = async (monitor: HealthMonitor) => {
 try {
 const r = await invoke<HealthCheckResult>("check_service_health", { monitor });
 applyResults([r]);
 } catch (e) {
 setError(String(e));
 }
 };

 // Aggregate stats
 const upCount = monitors.filter(m => results[m.id]?.ok === true).length;
 const downCount = monitors.filter(m => results[m.id]?.ok === false).length;
 const checkedCount = monitors.filter(m => results[m.id] !== undefined).length;

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
 <span style={{ fontSize: 15 }}></span>
 <div style={{ fontSize: 13, fontWeight: 600, flex: 1 }}>Service Health Monitor</div>

 {checkedCount > 0 && (
 <div style={{ display: "flex", gap: 6 }}>
 {upCount > 0 && <span style={{ padding: "2px 8px", borderRadius: 10, background: "rgba(166,227,161,0.15)", border: "1px solid var(--text-success, #a6e3a1)", color: "var(--text-success, #a6e3a1)", fontSize: 10, fontWeight: 700 }}>↑ {upCount} UP</span>}
 {downCount > 0 && <span style={{ padding: "2px 8px", borderRadius: 10, background: "rgba(243,139,168,0.15)", border: "1px solid var(--text-danger, #f38ba8)", color: "var(--text-danger, #f38ba8)", fontSize: 10, fontWeight: 700 }}>↓ {downCount} DOWN</span>}
 </div>
 )}

 <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
 <label style={{ fontSize: 10, color: "var(--text-muted)" }}>Auto</label>
 <input type="checkbox" checked={autoRefresh} onChange={e => setAutoRefresh(e.target.checked)} />
 {autoRefresh && (
 <select
 value={intervalSec}
 onChange={e => setIntervalSec(Number(e.target.value))}
 style={{ fontSize: 10, padding: "2px 4px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-primary)" }}
 >
 {[10, 30, 60, 120, 300].map(s => <option key={s} value={s}>{s}s</option>)}
 </select>
 )}
 </div>

 <button
 onClick={checkAll}
 disabled={checking || monitors.length === 0}
 style={{ padding: "4px 14px", fontSize: 11, fontWeight: 700, background: checking ? "var(--bg-secondary)" : "var(--accent-primary, #6366f1)", color: checking ? "var(--text-muted)" : "var(--text-on-accent, #fff)", border: "none", borderRadius: 4, cursor: checking ? "not-allowed" : "pointer" }}
 >
 {checking ? "Checking…" : "Check All"}
 </button>

 <button
 onClick={() => setShowAdd(v => !v)}
 style={{ padding: "4px 10px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", cursor: "pointer" }}
 >
 + Add
 </button>
 </div>

 {/* Add monitor form */}
 {showAdd && (
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "flex-end", flexWrap: "wrap" }}>
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>Label</label>
 <input value={newLabel} onChange={e => setNewLabel(e.target.value)} placeholder="My API" style={{ padding: "4px 8px", fontSize: 12, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", width: 120 }} />
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 3, flex: 1 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>URL</label>
 <input value={newUrl} onChange={e => setNewUrl(e.target.value)} placeholder="https://api.example.com/health" style={{ padding: "4px 8px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>Timeout</label>
 <select value={newTimeout} onChange={e => setNewTimeout(Number(e.target.value))} style={{ padding: "4px 6px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}>
 {[2000, 5000, 10000, 30000].map(t => <option key={t} value={t}>{t / 1000}s</option>)}
 </select>
 </div>
 <button onClick={addMonitor} disabled={!newLabel || !newUrl} style={{ padding: "5px 14px", fontSize: 11, fontWeight: 700, background: "var(--accent-primary, #6366f1)", border: "none", borderRadius: 4, color: "var(--text-on-accent, #fff)", cursor: "pointer", height: 28 }}>Add</button>
 <button onClick={() => setShowAdd(false)} style={{ padding: "5px 10px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer", height: 28 }}>Cancel</button>
 </div>
 )}

 {error && (
 <div style={{ padding: "6px 12px", background: "rgba(243,139,168,0.1)", color: "var(--text-danger, #f38ba8)", fontSize: 11, borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}> {error}</div>
 )}

 {/* Monitor list */}
 <div style={{ flex: 1, overflowY: "auto" }}>
 {monitors.length === 0 ? (
 <div style={{ padding: 32, textAlign: "center", color: "var(--text-muted)", fontSize: 13 }}>
 No monitors configured.<br />Click <b>+ Add</b> to add a service.
 </div>
 ) : (
 monitors.map(m => {
 const r = results[m.id];
 const hist = history[m.id] ?? [];
 return (
 <div key={m.id} style={{ padding: "10px 14px", borderBottom: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: 12 }}>
 {/* Status dot */}
 <div style={{
 width: 10, height: 10, borderRadius: "50%", flexShrink: 0,
 background: r === undefined ? "var(--text-muted)" : r.ok ? "var(--text-success, #a6e3a1)" : "var(--text-danger, #f38ba8)",
 boxShadow: r?.ok ? "0 0 6px var(--text-success, #a6e3a1)" : r && !r.ok ? "0 0 6px var(--text-danger, #f38ba8)" : "none",
 }} />

 {/* Label + URL */}
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: 12, fontWeight: 600 }}>{m.label}</div>
 <div style={{ fontSize: 10, color: "var(--text-muted)", fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{m.url}</div>
 </div>

 {/* Sparkline */}
 <div style={{ flexShrink: 0 }}><Sparkline history={hist} /></div>

 {/* Latency */}
 <div style={{ width: 60, textAlign: "right", flexShrink: 0 }}>
 {r ? (
 <>
 <div style={{ fontSize: 12, fontWeight: 700, color: latencyColor(r.latency_ms) }}>{r.latency_ms}ms</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)" }}>{r.status_code ?? "ERR"}</div>
 </>
 ) : <span style={{ fontSize: 10, color: "var(--text-muted)" }}>—</span>}
 </div>

 {/* Status badge */}
 <div style={{ width: 46, flexShrink: 0, textAlign: "center" }}>
 <StatusBadge result={r} />
 </div>

 {/* Error */}
 {r?.error && (
 <div style={{ fontSize: 9, color: "var(--text-danger, #f38ba8)", maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={r.error}>{r.error}</div>
 )}

 {/* Actions */}
 <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
 <button
 onClick={() => checkOne(m)}
 style={{ padding: "3px 8px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}
 >↺</button>
 <button
 onClick={() => removeMonitor(m.id)}
 style={{ padding: "3px 8px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-danger, #f38ba8)", cursor: "pointer" }}
 >✕</button>
 </div>
 </div>
 );
 })
 )}
 </div>

 {/* Footer — last check time */}
 {checkedCount > 0 && (
 <div style={{ padding: "6px 14px", borderTop: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 10, color: "var(--text-muted)", flexShrink: 0 }}>
 Last check: {new Date(Math.max(...Object.values(results).map(r => r.timestamp)) * 1000).toLocaleTimeString()}
 {autoRefresh && <span style={{ marginLeft: 8 }}>· Auto-refresh every {intervalSec}s</span>}
 </div>
 )}
 </div>
 );
}
