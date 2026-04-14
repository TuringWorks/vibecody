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
import { Loader2, Package, Box, Wrench, Code, Zap, Terminal, RefreshCw, LucideIcon } from "lucide-react";

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

const CATEGORY_ICONS: Record<string, LucideIcon> = {
 npm: Package,
 cargo: Box,
 make: Wrench,
 python: Code,
 go: Zap,
 just: Terminal,
};

const CATEGORY_COLORS: Record<string, string> = {
 npm: "#f7df1e",
 cargo: "#dea584",
 make: "#6cb6ff",
 python: "#4584b6",
 go: "#00add8",
 just: "var(--accent-green)",
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
 // eslint-disable-next-line react-hooks/exhaustive-deps
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
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
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
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <Terminal size={16} style={{ color: "var(--text-secondary)" }} />
 <div style={{ flex: 1 }}>
 <h3>Script Runner</h3>
 {data && (
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 {data.scripts.length} scripts · {data.detected_tools.join(", ")}
 </div>
 )}
 </div>
 <button
 onClick={load}
 disabled={loading}
 className="panel-btn panel-btn-secondary panel-btn-sm"
 >
 {loading ? <Loader2 size={13} className="spin" /> : <><RefreshCw size={13} /> Refresh</>}
 </button>
 </div>

 <div className="panel-body" style={{ overflow: "auto", display: "flex", flexDirection: "column", gap: 0 }}>
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
 padding: "3px 10px", fontSize: "var(--font-size-sm)", borderRadius: 12,
 background: filter === cat ? (CATEGORY_COLORS[cat] ?? "var(--accent-color)") : "var(--bg-secondary)",
 border: `1px solid ${filter === cat ? (CATEGORY_COLORS[cat] ?? "var(--accent-color)") : "var(--border-color)"}`,
 color: filter === cat ? "var(--bg-primary)" : "var(--text-secondary)",
 cursor: "pointer", fontWeight: filter === cat ? 600 : 400,
 }}
 >
 {cat === "all" ? "All" : (() => { const CatIcon = CATEGORY_ICONS[cat]; return <>{CatIcon ? <CatIcon size={11} /> : null} {cat}</>; })()}
 </button>
 ))}
 <input
 value={search}
 onChange={(e) => setSearch(e.target.value)}
 placeholder="Filter scripts…"
 style={{
 marginLeft: "auto", padding: "3px 8px", fontSize: "var(--font-size-sm)",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 12, color: "var(--text-primary)", outline: "none", width: 140,
 }}
 />
 </div>
 )}

 {error && (
 <div className="panel-error" style={{ margin: "8px 12px" }}>
 {error}
 </div>
 )}

 {/* Script list */}
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
 {filtered.length === 0 && !loading && (
 <div className="panel-empty">
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
 padding: "8px 10px", borderRadius: "var(--radius-sm)",
 background: isRunning ? "color-mix(in srgb, var(--accent-blue) 12%, transparent)" : "var(--bg-secondary)",
 border: `1px solid ${isRunning ? "var(--accent-color)" : "var(--border-color)"}`,
 transition: "border-color 0.15s",
 }}
 >
 {/* Category badge */}
 <span
 title={script.category}
 style={{
 flexShrink: 0, width: 22, display: "flex", justifyContent: "center", alignItems: "center",
 }}
 >
 {(() => { const Icon = CATEGORY_ICONS[script.category]; return Icon ? <Icon size={13} /> : null; })()}
 </span>

 {/* Script info */}
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
 {script.name}
 </div>
 {script.description && (
 <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 {script.description}
 </div>
 )}
 <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)", opacity: 0.7, marginTop: 1 }}>
 {script.command}
 </div>
 </div>

 {/* Run button */}
 <button
 onClick={() => runScript(script.command)}
 disabled={!!running}
 className="panel-btn panel-btn-primary panel-btn-sm"
 style={{ flexShrink: 0 }}
 >
 {isRunning ? <><Loader2 size={13} className="spin" /> Running…</> : "Run"}
 </button>
 </div>
 );
 })}
 </div>

 {/* Custom command */}
 <div className="panel-footer">
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
 className="panel-input panel-input-full"
 style={{ flex: 1 }}
 />
 <button
 onClick={() => {
 if (customCmd.trim()) {
 runScript(customCmd.trim());
 setCustomCmd("");
 }
 }}
 disabled={!!running || !customCmd.trim()}
 className="panel-btn panel-btn-secondary panel-btn-sm"
 >
 {running ? <Loader2 size={13} className="spin" /> : "Run"}
 </button>
 </div>

 {/* Live output */}
 {(logs.length > 0 || result) && (
 <div style={{ borderTop: "1px solid var(--border-color)", flexShrink: 0 }}>
 {/* Result header */}
 {result && (
 <div style={{
 padding: "6px 12px", fontSize: "var(--font-size-sm)", fontWeight: 600,
 background: result.success ? "color-mix(in srgb, var(--accent-green) 10%, transparent)" : "color-mix(in srgb, var(--accent-rose) 10%, transparent)",
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
 background: "var(--bg-primary)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)",
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
