/**
 * ScriptPanel — Script Runner & Task Manager.
 *
 * Auto-detects runnable scripts from package.json, Cargo.toml, Makefile,
 * pyproject.toml, go.mod, and justfile. Runs any script with live output
 * streaming via `script:log` Tauri events.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ProjectScript {
 category: string;
 name: string;
 command: string;
 description: string | null;
}

interface ScriptCategories {
 scripts: ProjectScript[];
 detected_tools: string[];
}

interface ScriptRunResult {
 command: string;
 exit_code: number;
 duration_ms: number;
 output: string;
 success: boolean;
}

interface ScriptPanelProps {
 workspacePath: string | null;
}

const CATEGORY_ICONS: Record<string, string> = {
 npm: "",
 cargo: "",
 make: "",
 python: "",
 go: "",
 just: "",
};

const CATEGORY_COLORS: Record<string, string> = {
 npm: "#f7df1e",
 cargo: "#dea584",
 make: "#6cb6ff",
 python: "#4584b6",
 go: "#00add8",
 just: "#a6e3a1",
};

export function ScriptPanel({ workspacePath }: ScriptPanelProps) {
 const [data, setData] = useState<ScriptCategories | null>(null);
 const [loading, setLoading] = useState(false);
 const [running, setRunning] = useState<string | null>(null);
 const [result, setResult] = useState<ScriptRunResult | null>(null);
 const [logs, setLogs] = useState<string[]>([]);
 const [error, setError] = useState<string | null>(null);
 const [filter, setFilter] = useState<string>("all");
 const [search, setSearch] = useState("");
 const [customCmd, setCustomCmd] = useState("");
 const logRef = useRef<HTMLDivElement>(null);
 const unlistenRef = useRef<(() => void) | null>(null);

 const load = async () => {
 if (!workspacePath) return;
 setLoading(true);
 setError(null);
 try {
 const result = await invoke<ScriptCategories>("detect_project_scripts", {
 workspace: workspacePath,
 });
 setData(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 useEffect(() => {
 load();
 return () => {
 unlistenRef.current?.();
 };
 }, [workspacePath]);

 useEffect(() => {
 if (logRef.current) {
 logRef.current.scrollTop = logRef.current.scrollHeight;
 }
 }, [logs]);

 const runScript = async (command: string) => {
 if (!workspacePath || running) return;
 setRunning(command);
 setResult(null);
 setLogs([]);
 setError(null);

 // Subscribe to live log events
 unlistenRef.current?.();
 const unlisten = await listen<string>("script:log", (event) => {
 setLogs((prev) => [...prev, event.payload]);
 });
 unlistenRef.current = unlisten;

 try {
 const res = await invoke<ScriptRunResult>("run_project_script", {
 workspace: workspacePath,
 command,
 });
 setResult(res);
 } catch (e) {
 setError(String(e));
 } finally {
 setRunning(null);
 unlistenRef.current?.();
 unlistenRef.current = null;
 }
 };

 if (!workspacePath) {
 return (
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-muted)", fontSize: 12 }}>
 Open a workspace to view available scripts.
 </div>
 );
 }

 const categories = data
 ? [...new Set(data.scripts.map((s) => s.category))]
 : [];

 const filtered = (data?.scripts ?? []).filter((s) => {
 const matchCat = filter === "all" || s.category === filter;
 const matchSearch = !search ||
 s.name.toLowerCase().includes(search.toLowerCase()) ||
 s.command.toLowerCase().includes(search.toLowerCase()) ||
 (s.description ?? "").toLowerCase().includes(search.toLowerCase());
 return matchCat && matchSearch;
 });

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{
 padding: "10px 12px", borderBottom: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", flexShrink: 0,
 display: "flex", alignItems: "center", gap: 8,
 }}>
 <span style={{ fontSize: 16 }}></span>
 <div style={{ flex: 1 }}>
 <div style={{ fontSize: 13, fontWeight: 600 }}>Script Runner</div>
 {data && (
 <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
 {data.scripts.length} scripts · {data.detected_tools.join(", ")}
 </div>
 )}
 </div>
 <button
 onClick={load}
 disabled={loading}
 style={{
 padding: "4px 10px", fontSize: 11, cursor: "pointer",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-secondary)",
 }}
 >
 {loading ? "" : "↻ Refresh"}
 </button>
 </div>

 <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column", gap: 0 }}>
 {/* Filter + Search bar */}
 {data && (
 <div style={{
 padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
 display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center",
 }}>
 {(["all", ...categories] as string[]).map((cat) => (
 <button
 key={cat}
 onClick={() => setFilter(cat)}
 style={{
 padding: "3px 10px", fontSize: 11, borderRadius: 12,
 background: filter === cat ? (CATEGORY_COLORS[cat] ?? "var(--accent-color)") : "var(--bg-secondary)",
 border: `1px solid ${filter === cat ? (CATEGORY_COLORS[cat] ?? "var(--accent-color)") : "var(--border-color)"}`,
 color: filter === cat ? "var(--bg-primary)" : "var(--text-secondary)",
 cursor: "pointer", fontWeight: filter === cat ? 600 : 400,
 }}
 >
 {cat === "all" ? "All" : `${CATEGORY_ICONS[cat] ?? "•"} ${cat}`}
 </button>
 ))}
 <input
 value={search}
 onChange={(e) => setSearch(e.target.value)}
 placeholder="Filter scripts…"
 style={{
 marginLeft: "auto", padding: "3px 8px", fontSize: 11,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 12, color: "var(--text-primary)", outline: "none", width: 140,
 }}
 />
 </div>
 )}

 {error && (
 <div style={{ margin: "8px 12px", padding: "6px 10px", background: "var(--error-bg)", color: "var(--error-color)", borderRadius: 4, fontSize: 12 }}>
 {error}
 </div>
 )}

 {/* Script list */}
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
 {filtered.length === 0 && !loading && (
 <div style={{ textAlign: "center", padding: "30px 0", color: "var(--text-muted)", fontSize: 12 }}>
 {data ? "No scripts match your filter." : "No scripts detected in this workspace."}
 </div>
 )}

 {filtered.map((script) => {
 const isRunning = running === script.command;
 return (
 <div
 key={`${script.category}:${script.name}`}
 style={{
 display: "flex", alignItems: "center", gap: 10,
 padding: "8px 10px", borderRadius: 6,
 background: isRunning ? "rgba(99,102,241,0.12)" : "var(--bg-secondary)",
 border: `1px solid ${isRunning ? "var(--accent-color)" : "var(--border-color)"}`,
 transition: "border-color 0.15s",
 }}
 >
 {/* Category badge */}
 <span
 title={script.category}
 style={{
 fontSize: 13, flexShrink: 0, width: 22, textAlign: "center",
 }}
 >
 {CATEGORY_ICONS[script.category] ?? ""}
 </span>

 {/* Script info */}
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: 12, fontWeight: 600, fontFamily: "monospace" }}>
 {script.name}
 </div>
 {script.description && (
 <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {script.description}
 </div>
 )}
 <div style={{ fontSize: 10, color: "var(--text-muted)", fontFamily: "monospace", opacity: 0.7, marginTop: 1 }}>
 {script.command}
 </div>
 </div>

 {/* Run button */}
 <button
 onClick={() => runScript(script.command)}
 disabled={!!running}
 style={{
 padding: "5px 14px", fontSize: 11, fontWeight: 600,
 background: isRunning ? "rgba(99,102,241,0.3)" : "var(--accent-color)",
 color: "var(--text-primary)", border: "none", borderRadius: 4,
 cursor: running ? "not-allowed" : "pointer",
 opacity: running && !isRunning ? 0.4 : 1,
 flexShrink: 0,
 }}
 >
 {isRunning ? "Running…" : "Run"}
 </button>
 </div>
 );
 })}
 </div>

 {/* Custom command */}
 <div style={{
 padding: "8px 12px", borderTop: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", display: "flex", gap: 6,
 }}>
 <input
 value={customCmd}
 onChange={(e) => setCustomCmd(e.target.value)}
 onKeyDown={(e) => {
 if (e.key === "Enter" && customCmd.trim()) {
 runScript(customCmd.trim());
 setCustomCmd("");
 }
 }}
 placeholder="Run custom command…"
 disabled={!!running}
 style={{
 flex: 1, padding: "5px 8px", fontSize: 12,
 background: "var(--bg-primary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 />
 <button
 onClick={() => {
 if (customCmd.trim()) {
 runScript(customCmd.trim());
 setCustomCmd("");
 }
 }}
 disabled={!!running || !customCmd.trim()}
 style={{
 padding: "5px 12px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer",
 }}
 >
 {running ? "" : "Run"}
 </button>
 </div>

 {/* Live output */}
 {(logs.length > 0 || result) && (
 <div style={{ borderTop: "1px solid var(--border-color)", flexShrink: 0 }}>
 {/* Result header */}
 {result && (
 <div style={{
 padding: "6px 12px", fontSize: 11, fontWeight: 600,
 background: result.success ? "rgba(166,227,161,0.1)" : "rgba(243,139,168,0.1)",
 color: result.success ? "var(--success-color)" : "var(--error-color)",
 borderBottom: "1px solid var(--border-color)",
 display: "flex", justifyContent: "space-between",
 }}>
 <span>
 {result.success ? "Success" : ` Exit code ${result.exit_code}`}
 </span>
 <span style={{ opacity: 0.8 }}>
 {(result.duration_ms / 1000).toFixed(2)}s
 </span>
 </div>
 )}
 {/* Log output */}
 <div
 ref={logRef}
 style={{
 maxHeight: 220, overflow: "auto", padding: "8px 12px",
 background: "var(--bg-primary)", fontFamily: "monospace", fontSize: 11,
 lineHeight: 1.5, whiteSpace: "pre-wrap", wordBreak: "break-all",
 }}
 >
 {logs.map((line, i) => (
 <div
 key={i}
 style={{
 color: line.startsWith("$") ? "var(--accent-color)"
 : line.includes("error") || line.includes("Error") || line.includes("FAILED") ? "var(--error-color)"
 : line.includes("warn") || line.includes("Warn") || line.includes("WARNING") ? "var(--warning-color)"
 : line.startsWith("[Exited") ? (line.includes("code 0") ? "var(--success-color)" : "var(--error-color)")
 : "var(--text-primary)",
 }}
 >
 {line}
 </div>
 ))}
 {running && (
 <div style={{ color: "var(--accent-color)", marginTop: 4 }}>
 <span className="blink">▌</span>
 </div>
 )}
 </div>
 </div>
 )}
 </div>
 </div>
 );
}
