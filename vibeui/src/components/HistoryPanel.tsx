import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TraceEntry {
 timestamp: number;
 session_id: string;
 step: number;
 tool: string;
 input_summary: string;
 output: string;
 success: boolean;
 duration_ms: number;
 approved_by: string;
}

interface TraceSession {
 session_id: string;
 timestamp: number;
 step_count: number;
}

export function HistoryPanel() {
 const [sessions, setSessions] = useState<TraceSession[]>([]);
 const [selected, setSelected] = useState<string | null>(null);
 const [entries, setEntries] = useState<TraceEntry[]>([]);
 const [loading, setLoading] = useState(false);

 const loadSessions = async () => {
 try {
 const result = await invoke<TraceSession[]>("list_trace_sessions");
 setSessions(result);
 } catch {
 // No traces yet or command not yet available
 setSessions([]);
 }
 };

 useEffect(() => {
 let cancelled = false;
 invoke<TraceSession[]>("list_trace_sessions")
 .then((result) => { if (!cancelled) setSessions(result); })
 .catch(() => { if (!cancelled) setSessions([]); });
 return () => { cancelled = true; };
 }, []);

 const loadTrace = async (sessionId: string) => {
 setSelected(sessionId);
 setLoading(true);
 try {
 const result = await invoke<TraceEntry[]>("load_trace_session", { sessionId });
 setEntries(result);
 } catch {
 setEntries([]);
 } finally {
 setLoading(false);
 }
 };

 const formatAge = (ts: number) => {
 const elapsed = Math.floor(Date.now() / 1000) - ts;
 if (elapsed < 3600) return `${Math.floor(elapsed / 60)}m ago`;
 if (elapsed < 86400) return `${Math.floor(elapsed / 3600)}h ago`;
 return `${Math.floor(elapsed / 86400)}d ago`;
 };

 const toolIcon = (tool: string) => {
 switch (tool) {
 case "read_file": return "";
 case "write_file": return "";
 case "apply_patch": return "";
 case "bash": return "";
 case "search_files": return "";
 case "list_directory": return "";
 case "task_complete": return "";
 default: return "";
 }
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "8px" }}>
 <div style={{ fontWeight: 600, fontSize: "14px" }}>Agent History</div>
 <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
 Audit log of past agent sessions.
 </p>

 {selected === null ? (
 // Session list view
 <div style={{ flex: 1, overflowY: "auto" }}>
 {sessions.length === 0 ? (
 <div style={{ fontSize: "12px", color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
 No agent sessions yet.
 <br />Run an agent task to see history here.
 </div>
 ) : (
 sessions.map((s) => (
 <div
 key={s.session_id}
 onClick={() => loadTrace(s.session_id)}
 style={{
 padding: "8px",
 marginBottom: "4px",
 borderRadius: "4px",
 background: "var(--bg-tertiary)",
 border: "1px solid var(--border-color)",
 cursor: "pointer",
 fontSize: "12px",
 }}
 >
 <div style={{ fontWeight: 500 }}>
 Session {s.session_id.slice(0, 8)}…
 </div>
 <div style={{ color: "var(--text-secondary)", marginTop: "2px" }}>
 {s.step_count} steps · {formatAge(s.timestamp)}
 </div>
 </div>
 ))
 )}

 <button
 className="btn-secondary"
 onClick={loadSessions}
 style={{ marginTop: "8px", width: "100%", fontSize: "12px" }}
 >
 ↺ Refresh
 </button>
 </div>
 ) : (
 // Trace detail view
 <>
 <button
 className="btn-secondary"
 onClick={() => { setSelected(null); setEntries([]); }}
 style={{ fontSize: "12px", alignSelf: "flex-start" }}
 >
 ← Back
 </button>

 <div
 style={{
 flex: 1,
 overflowY: "auto",
 background: "var(--bg-tertiary)",
 borderRadius: "6px",
 padding: "8px",
 fontFamily: "monospace",
 fontSize: "11px",
 display: "flex",
 flexDirection: "column",
 gap: "6px",
 }}
 >
 {loading ? (
 <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
 Loading…
 </div>
 ) : entries.length === 0 ? (
 <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
 No entries in this trace.
 </div>
 ) : (
 entries.map((e, i) => (
 <div
 key={i}
 style={{ borderBottom: "1px solid var(--border-color)", paddingBottom: "6px" }}
 >
 <div
 style={{
 color: e.success
 ? "var(--accent-green, #4ec9b0)"
 : "var(--text-danger, #f44)",
 fontWeight: 500,
 }}
 >
 {e.success ? "" : ""} {toolIcon(e.tool)} {e.input_summary}
 <span style={{ color: "var(--text-secondary)", fontWeight: 400, marginLeft: "8px" }}>
 {e.duration_ms}ms · {e.approved_by}
 </span>
 </div>
 {e.output && (
 <pre
 style={{
 margin: "4px 0 0 16px",
 color: "var(--text-secondary)",
 whiteSpace: "pre-wrap",
 maxHeight: "120px",
 overflowY: "auto",
 fontSize: "10px",
 }}
 >
 {e.output.length > 300
 ? e.output.slice(0, 300) + "\n…"
 : e.output}
 </pre>
 )}
 </div>
 ))
 )}
 </div>
 </>
 )}
 </div>
 );
}
