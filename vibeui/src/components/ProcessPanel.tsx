import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play, Moon, AlertOctagon, Square, MinusCircle, X } from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────────────────

interface ProcessInfo {
 pid: number;
 name: string;
 cpu_pct: number;
 mem_kb: number;
 status: string;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

function fmtMem(kb: number): string {
 if (kb >= 1_048_576) return `${(kb / 1_048_576).toFixed(1)} GB`;
 if (kb >= 1_024) return `${(kb / 1_024).toFixed(1)} MB`;
 return `${kb} KB`;
}

function statusBadge(status: string): React.ReactNode {
 const s = status.toUpperCase();
 if (s.startsWith("S")) return <Moon size={12} strokeWidth={1.5} style={{ color: "var(--accent-gold)" }} />;
 if (s.startsWith("R")) return <Play size={12} strokeWidth={1.5} style={{ color: "var(--accent-green)" }} />;
 if (s.startsWith("Z")) return <AlertOctagon size={12} strokeWidth={1.5} style={{ color: "var(--accent-rose)" }} />;
 if (s.startsWith("T")) return <Square size={12} strokeWidth={1.5} style={{ color: "var(--accent-blue)" }} />;
 return <MinusCircle size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />;
}

// ── Main component ─────────────────────────────────────────────────────────────

const ProcessPanel: React.FC = () => {
 const [processes, setProcesses] = useState<ProcessInfo[]>([]);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [filter, setFilter] = useState("");
 const [killing, setKilling] = useState<number | null>(null);
 const [killFeedback, setKillFeedback] = useState<{ pid: number; ok: boolean; msg: string } | null>(null);
 const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

 const loadProcesses = useCallback(async () => {
 setLoading(true);
 setError(null);
 try {
 const procs = await invoke<ProcessInfo[]>("list_processes");
 setProcesses(procs);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 }, []);

 // Auto-refresh every 5 s
 useEffect(() => {
 void loadProcesses();
 intervalRef.current = setInterval(() => void loadProcesses(), 5_000);
 return () => {
 if (intervalRef.current !== null) clearInterval(intervalRef.current);
 };
 }, [loadProcesses]);

 const handleKill = async (pid: number, name: string) => {
 if (!window.confirm(`Send SIGTERM to "${name}" (PID ${pid})?`)) return;
 setKilling(pid);
 setKillFeedback(null);
 try {
 await invoke("kill_process", { pid });
 setKillFeedback({ pid, ok: true, msg: `Sent SIGTERM to PID ${pid}` });
 // Remove the process from the list optimistically
 setProcesses((prev) => prev.filter((p) => p.pid !== pid));
 } catch (e) {
 setKillFeedback({ pid, ok: false, msg: String(e) });
 } finally {
 setKilling(null);
 }
 };

 const filtered = processes.filter((p) =>
 filter.length === 0 ||
 p.name.toLowerCase().includes(filter.toLowerCase()) ||
 String(p.pid).includes(filter)
 );

 return (
 <div className="panel-container" style={{ fontSize: 13 }}>
 {/* Toolbar */}
 <div className="panel-header">
 <input
 type="search"
 placeholder="Filter by name or PID…"
 value={filter}
 onChange={(e) => setFilter(e.target.value)}
 className="panel-input"
 style={{ flex: 1 }}
 aria-label="Filter processes"
 />
 <button
 onClick={() => void loadProcesses()}
 disabled={loading}
 className="panel-btn panel-btn-secondary"
 title="Refresh process list"
 >
 {loading ? "…" : "↻ Refresh"}
 </button>
 </div>

 {/* Feedback banner */}
 {killFeedback && (
 <div
 role="status"
 aria-live="polite"
 style={{
 padding: "6px 12px",
 background: killFeedback.ok ? "var(--success-bg)" : "var(--error-bg)",
 color: killFeedback.ok ? "var(--success-fg)" : "var(--error-fg)",
 fontSize: 12,
 }}
 >
 {killFeedback.ok ? "" : ""} {killFeedback.msg}
 <button
 onClick={() => setKillFeedback(null)}
 style={{ marginLeft: 12, background: "none", border: "none", cursor: "pointer", color: "inherit", display: "flex", alignItems: "center" }}
 aria-label="Dismiss"
 >
 <X size={14} />
 </button>
 </div>
 )}

 {error && (
 <div style={{ padding: "8px 12px", color: "var(--error-fg)", fontSize: 12 }}>
 {error}
 </div>
 )}

 {/* Table */}
 <div style={{ flex: 1, overflowY: "auto" }}>
 <table style={{ width: "100%", borderCollapse: "collapse" }} aria-label="Running processes">
 <thead>
 <tr style={{ background: "var(--table-header-bg, rgba(255,255,255,0.05))", position: "sticky", top: 0 }}>
 <th style={thStyle}>PID</th>
 <th style={{ ...thStyle, textAlign: "left" }}>Name</th>
 <th style={thStyle}>CPU %</th>
 <th style={thStyle}>Memory</th>
 <th style={thStyle}>Status</th>
 <th style={thStyle}>Action</th>
 </tr>
 </thead>
 <tbody>
 {filtered.length === 0 && !loading && (
 <tr>
 <td colSpan={6} style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
 {filter ? "No matching processes" : "No processes found"}
 </td>
 </tr>
 )}
 {filtered.map((proc) => (
 <tr
 key={proc.pid}
 style={{
 borderBottom: "1px solid var(--border, rgba(255,255,255,0.06))",
 background: killing === proc.pid ? "var(--row-killing-bg, rgba(255,80,80,0.07))" : "transparent",
 }}
 >
 <td style={tdNumStyle}>{proc.pid}</td>
 <td style={{ ...tdStyle, maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={proc.name}>
 {proc.name}
 </td>
 <td style={tdNumStyle}>{proc.cpu_pct.toFixed(1)}</td>
 <td style={tdNumStyle}>{fmtMem(proc.mem_kb)}</td>
 <td style={{ ...tdNumStyle, fontSize: 16 }} title={proc.status}>
 {statusBadge(proc.status)}
 </td>
 <td style={tdNumStyle}>
 <button
 onClick={() => void handleKill(proc.pid, proc.name)}
 disabled={killing === proc.pid}
 aria-label={`Kill process ${proc.name} (PID ${proc.pid})`}
 style={{
 padding: "2px 8px",
 borderRadius: 4,
 border: "1px solid var(--error-fg)",
 background: "transparent",
 color: "var(--error-fg)",
 cursor: "pointer",
 fontSize: 11,
 }}
 >
 {killing === proc.pid ? "…" : "Kill"}
 </button>
 </td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>

 {/* Footer */}
 <div style={{ padding: "4px 12px", fontSize: 11, color: "var(--text-secondary)", borderTop: "1px solid var(--border)" }}>
 {filtered.length} / {processes.length} processes shown · auto-refreshes every 5 s
 </div>
 </div>
 );
};

// ── Table cell styles ──────────────────────────────────────────────────────────

const thStyle: React.CSSProperties = {
 padding: "6px 10px",
 fontWeight: 600,
 textAlign: "right",
 fontSize: 11,
 color: "var(--text-secondary)",
 whiteSpace: "nowrap",
};

const tdStyle: React.CSSProperties = {
 padding: "5px 10px",
};

const tdNumStyle: React.CSSProperties = {
 ...tdStyle,
 textAlign: "right",
 fontVariantNumeric: "tabular-nums",
};

export default ProcessPanel;
