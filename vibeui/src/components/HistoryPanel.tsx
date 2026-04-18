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
 <div className="panel-container">
 <div className="panel-header">
 <h3>Agent History</h3>
 <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Audit log of past agent sessions.</span>
 </div>

 <div className="panel-body">
 {selected === null ? (
 // Session list view
 <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
 {sessions.length === 0 ? (
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
 No agent sessions yet.
 <br />Run an agent task to see history here.
 </div>
 ) : (
 sessions.map((s) => (
 <div
 key={s.session_id}
 role="button"
 tabIndex={0}
 onClick={() => loadTrace(s.session_id)}
 onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); loadTrace(s.session_id); } }}
 style={{
 padding: "8px",
 marginBottom: "4px",
 borderRadius: "var(--radius-xs-plus)",
 background: "var(--bg-tertiary)",
 border: "1px solid var(--border-color)",
 cursor: "pointer",
 fontSize: "var(--font-size-base)",
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
 className="panel-btn btn-secondary"
 onClick={loadSessions}
 style={{ marginTop: "8px", width: "100%", fontSize: "var(--font-size-base)" }}
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
 style={{ fontSize: "var(--font-size-base)", alignSelf: "flex-start" }}
 >
 ← Back
 </button>

 <div
 style={{
 flex: 1,
 overflowY: "auto",
 background: "var(--bg-tertiary)",
 borderRadius: "var(--radius-sm)",
 padding: "8px",
 fontFamily: "var(--font-mono)",
 fontSize: "var(--font-size-sm)",
 display: "flex",
 flexDirection: "column",
 gap: "8px",
 }}
 >
 {loading ? (
 <div className="panel-loading" style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
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
 style={{ borderBottom: "1px solid var(--border-color)", paddingBottom: "8px" }}
 >
 <div
 style={{
 color: e.success
 ? "var(--success-color)"
 : "var(--text-danger)",
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
 fontSize: "var(--font-size-xs)",
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
 </div>
 );
}
